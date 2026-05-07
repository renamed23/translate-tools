pub mod featured_hook;
pub mod input;
pub mod return_kind;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use syn::LitStr;

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

/// 在目录中查找唯一匹配指定拓展名的文件，找不到或找到多个时报错。
pub fn find_single_file_in_dir(dir: &Path, ext: &str, span: &LitStr) -> syn::Result<PathBuf> {
    let mut found = None;
    for entry in fs::read_dir(dir).map_err(|e| syn_err!(span, "无法读取目录: {e}"))? {
        let e = entry.map_err(|e| syn_err!(span, "读取目录项失败: {e}"))?;
        if !e
            .file_type()
            .map_err(|e| syn_err!(span, "获取文件类型失败: {e}"))?
            .is_file()
        {
            continue;
        }
        let path = e.path();
        let matches = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false);
        if matches {
            if found.is_some() {
                syn_bail!(span, "目录存在多个 .{ext} 文件");
            }
            found = Some(path);
        }
    }
    found.ok_or_else(|| syn_err!(span, "目录中未找到 .{ext} 文件"))
}

/// 扫描 `api_hooks` 目录下的所有 `.rs` 文件，提取 `#[detour]` 属性中的
/// `dll` 信息，构建 **Rust 函数名 → DLL 名** 的映射表。
///
/// 函数名使用 trait 方法标识符（snake_case），而非 `symbol` 字段值，
/// 避免不同 trait 中的同名 symbol 导致冲突。
pub fn build_dll_map_from_api_hooks(api_hooks_dir: &Path) -> syn::Result<HashMap<String, String>> {
    use crate::impls::detour::parse_detour_attr;

    let mut dll_map = HashMap::new();
    let rs_files = collect_files_in_dir(api_hooks_dir)?;

    for file_path in rs_files {
        if file_path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| syn_err2!("无法读取 {}: {}", file_path.display(), e))?;

        let file: syn::File = syn::parse_str(&content)
            .map_err(|e| syn_err2!("解析 {} 失败: {}", file_path.display(), e))?;

        for item in file.items {
            let item_trait = match item {
                syn::Item::Trait(t) => t,
                _ => continue,
            };

            for trait_item in item_trait.items {
                let fn_item = match trait_item {
                    syn::TraitItem::Fn(f) => f,
                    _ => continue,
                };

                let fn_name = fn_item.sig.ident.to_string();

                for attr in &fn_item.attrs {
                    if let Some(detour) = parse_detour_attr(attr)? {
                        dll_map.insert(fn_name, detour.dll);
                        break;
                    }
                }
            }
        }
    }

    Ok(dll_map)
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
