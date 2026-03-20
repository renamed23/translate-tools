use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use syn::{
    GenericArgument, LitStr, PathArguments, ReturnType, Token, Type,
    parse::{Parse, ParseStream},
};

/// 传入相对于`CARGO_MANIFEST_DIR`路径，然后返回完整的路径
pub fn get_full_path_by_manifest(rel_path: impl AsRef<Path>) -> syn::Result<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| syn_err2!("无法获取 CARGO_MANIFEST_DIR 环境变量: {e}"))?;
    Ok(PathBuf::from(&manifest_dir).join(rel_path))
}

/// 将宏输入中的字符串字面量路径解析为相对于 `CARGO_MANIFEST_DIR` 的完整路径。
pub fn resolve_manifest_path(path: &LitStr) -> syn::Result<PathBuf> {
    get_full_path_by_manifest(path.value()).map_err(|e| syn::Error::new_spanned(path, e))
}

/// 读取指定文件的 UTF-8 文本内容。
pub fn read_file_string(path: &Path) -> syn::Result<String> {
    fs::read_to_string(path).map_err(|e| syn_err2!("无法读取 {}: {}", path.display(), e))
}

/// 读取指定文件的原始字节内容。
pub fn read_file_bytes(path: &Path) -> syn::Result<Vec<u8>> {
    fs::read(path).map_err(|e| syn_err2!("无法读取 {}: {}", path.display(), e))
}

/// 读取并反序列化 JSON 文件。
///
/// 文件内容会先按字符串读取，再通过 `serde_json` 解析为目标类型 `T`。
pub fn read_json_file<T: DeserializeOwned>(path: &Path) -> syn::Result<T> {
    let content = read_file_string(path)?;
    serde_json::from_str(&content).map_err(|e| syn_err2!("解析 {} 失败: {}", path.display(), e))
}

/// 尝试读取 JSON 文件；如果文件不存在，则返回目标类型的默认值。
pub fn read_optional_json_file<T: DeserializeOwned + Default>(path: &Path) -> syn::Result<T> {
    if !path.is_file() {
        return Ok(T::default());
    }

    read_json_file(path)
}

/// 确保指定路径存在且为目录。
///
/// 当校验失败时，会基于传入的宏字面量生成带定位信息的 `syn::Error`。
pub fn ensure_dir(path: &Path, origin: &LitStr) -> syn::Result<()> {
    if !path.is_dir() {
        syn_bail!(origin, "路径不是文件夹: {}", path.display());
    }

    Ok(())
}

/// 收集目录下的所有直接子文件，并按文件名排序后返回。
///
/// 该函数只会收集当前目录层级中的普通文件，不会递归进入子目录。
pub fn collect_files_in_dir(dir: &Path) -> syn::Result<Vec<PathBuf>> {
    let mut files = fs::read_dir(dir)
        .map_err(|e| syn_err2!("无法读取目录 {}: {}", dir.display(), e))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|ft| ft.is_file())
                .map(|_| entry.path())
        })
        .collect::<Vec<_>>();

    files.sort_by_key(|path| {
        path.file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default()
    });
    Ok(files)
}

/// 仅包含单个路径字面量的宏输入。
pub struct SinglePathInput {
    /// 传入的路径字符串。
    pub path: LitStr,
}

impl Parse for SinglePathInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse()?,
        })
    }
}

/// 由两个逗号分隔的路径字面量组成的宏输入。
pub struct CommaSeparatedPaths {
    /// 逗号左侧的路径字符串。
    pub left: LitStr,
    /// 逗号右侧的路径字符串。
    pub right: LitStr,
}

impl Parse for CommaSeparatedPaths {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let left = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let right = input.parse()?;
        Ok(Self { left, right })
    }
}

/// 由 `=>` 分隔的两个路径字面量组成的宏输入。
pub struct ArrowSeparatedPaths {
    /// `=>` 左侧的路径字符串。
    pub left: LitStr,
    /// `=>` 右侧的路径字符串。
    pub right: LitStr,
}

impl Parse for ArrowSeparatedPaths {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let left = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let right = input.parse()?;
        Ok(Self { left, right })
    }
}

/// 表示函数返回类型在宏展开阶段的语义分类。
///
/// 该枚举主要用于识别函数是否返回 `Result<T, E>`，
/// 以便像 `ffi_guard`、`detour_fn`、`detour_trait` 这类宏在生成包装代码时：
/// - 能够区分普通返回值与 `Result` 返回值；
/// - 在需要时将 `Result<T, E>` 的签名“拍平”为 `T`；
/// - 为 `Err(...)` 分支注入统一的兜底返回逻辑。
pub enum ReturnKind {
    /// 非 `Result` 返回值。
    ///
    /// 包括：
    /// - 没有显式返回值（`()`）的函数；
    /// - 返回类型不是路径类型的函数；
    /// - 返回类型虽然是路径类型，但最后一段标识符不是 `Result` 的函数。
    Plain,
    /// 返回 `Result<T, E>` 形式的函数，其中仅保留成功值类型 `T`。
    ///
    /// 当前实现只关心 `Ok` 分支的内部类型，错误类型 `E` 不参与后续代码生成，
    /// 因为宏通常会在包装层统一处理错误并返回一个调用方指定的兜底值。
    Result(Box<Type>),
}

impl ReturnKind {
    /// 若返回类型是 `Result<T, E>`，则将其拍平为 `-> T`；否则保持原样。
    pub fn flatten_result(output: ReturnType) -> ReturnType {
        Self::from_return_type(&output)
            .try_flatten_result()
            .unwrap_or(output)
    }

    /// 从 `syn::ReturnType` 中识别返回值分类。
    ///
    /// 目前仅识别最后一个路径段名字为 `Result` 的返回类型，
    /// 例如 `Result<T, E>`、`crate::Result<T>`、`anyhow::Result<T>` 等。
    ///
    /// 注意：该判断只基于类型路径最后一段的标识符，不进一步校验其来源模块。
    pub fn from_return_type(output: &ReturnType) -> Self {
        let ReturnType::Type(_, ty) = output else {
            return Self::Plain;
        };

        let Type::Path(type_path) = ty.as_ref() else {
            return Self::Plain;
        };

        let Some(last_segment) = type_path.path.segments.last() else {
            return Self::Plain;
        };

        let generic_types = match &last_segment.arguments {
            PathArguments::AngleBracketed(args) => args
                .args
                .iter()
                .filter_map(|arg| match arg {
                    GenericArgument::Type(ty) => Some(ty.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };

        match last_segment.ident.to_string().as_str() {
            "Result" => generic_types
                .into_iter()
                .next()
                .map(|r| Self::Result(Box::new(r)))
                .unwrap_or(Self::Plain),
            _ => Self::Plain,
        }
    }

    /// 尝试将当前分类转换成拍平后的返回类型。
    ///
    /// - `Plain` => `None`，表示不需要修改函数签名；
    /// - `Result(T)` => `Some(-> T)`，表示可将 `Result<T, E>` 视为直接返回 `T`。
    pub fn try_flatten_result(&self) -> Option<ReturnType> {
        match self {
            Self::Plain => None,
            Self::Result(inner_ty) => Some(syn::parse_quote!(-> #inner_ty)),
        }
    }
}
