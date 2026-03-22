use syn::{GenericArgument, PathArguments, ReturnType, Type};

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
