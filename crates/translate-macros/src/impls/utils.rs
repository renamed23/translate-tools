use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// 传入相对于`CARGO_MANIFEST_DIR`路径，然后返回完整的路径
pub(crate) fn get_full_path_by_manifest(rel_path: impl AsRef<Path>) -> syn::Result<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| syn_err2!("无法获取 CARGO_MANIFEST_DIR 环境变量: {e}"))?;
    Ok(PathBuf::from(&manifest_dir).join(rel_path))
}

/// 读取配置 JSON 文件，返回一个字符串键到 JSON 值的映射
pub(crate) fn read_config_json(path: impl AsRef<Path>) -> syn::Result<HashMap<String, Value>> {
    let path = path.as_ref();

    let default_str = std::fs::read_to_string(path)
        .map_err(|e| syn_err2!("无法读取配置 {}: {}", path.display(), e))?;
    serde_json::from_str(&default_str)
        .map_err(|e| syn_err2!("解析默认配置 JSON 失败 ({}): {}", path.display(), e))
}
