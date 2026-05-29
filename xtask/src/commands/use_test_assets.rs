use std::path::Path;

use anyhow::bail;
use fs_extra::dir::remove as remove_dir;

use crate::assets::{TARGET_ASSETS_DIR, TEST_ASSETS_DIR};

pub fn run_use_test_assets() -> anyhow::Result<()> {
    let source = Path::new(TEST_ASSETS_DIR);
    let target = Path::new(TARGET_ASSETS_DIR);

    if !source.exists() {
        bail!("未找到测试资产目录: {}", source.display());
    }

    println!(
        "正在覆盖 assets: {} -> {}",
        source.display(),
        target.display()
    );
    remove_dir_if_exists(target)?;
    copy_dir_contents(source, target)?;

    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if !path.is_dir() {
        bail!("目标路径不是目录，无法删除: {}", path.display());
    }

    remove_dir(path).unwrap_or_else(|e| panic!("删除目录失败: {path}: {e}", path = path.display()));

    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if !src.is_dir() {
        bail!("源路径不是目录: {}", src.display());
    }

    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;

    fs_extra::dir::copy(src, dst, &options).unwrap_or_else(|e| {
        panic!(
            "复制目录内容失败: {src} -> {dst}: {e}",
            src = src.display(),
            dst = dst.display()
        )
    });

    Ok(())
}
