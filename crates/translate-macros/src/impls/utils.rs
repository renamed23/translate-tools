use std::path::{Path, PathBuf};

/// 传入相对于`CARGO_MANIFEST_DIR`路径，然后返回完整的路径
pub(crate) fn get_full_path_by_manifest(rel_path: impl AsRef<Path>) -> syn::Result<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| syn_err2!("无法获取 CARGO_MANIFEST_DIR 环境变量: {e}"))?;
    Ok(PathBuf::from(&manifest_dir).join(rel_path))
}
