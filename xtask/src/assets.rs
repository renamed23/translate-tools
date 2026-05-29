use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use fs_extra::dir::{CopyOptions, copy as copy_dir, remove as remove_dir};

pub const TEST_ASSETS_DIR: &str = "xtask/test_assets";
pub const TARGET_ASSETS_DIR: &str = "crates/text-hook/assets";

pub struct AssetGuard {
    backup_dir: Option<PathBuf>,
}

impl Drop for AssetGuard {
    fn drop(&mut self) {
        if let Err(e) = restore_assets(self.backup_dir.take()) {
            println!("恢复资产出现问题: {e}");
        }
    }
}

pub fn backup_and_replace_assets() -> anyhow::Result<AssetGuard> {
    let source = Path::new(TEST_ASSETS_DIR);
    let target = Path::new(TARGET_ASSETS_DIR);

    if !source.exists() {
        bail!("未找到测试资产目录: {}", source.display());
    }
    let backup_dir = if target.exists() {
        if !target.is_dir() {
            bail!("目标路径不是目录: {}", target.display());
        }

        let backup_dir = std::env::temp_dir().join(format!(
            "translate-tools-xtask-assets-backup-{}-{}",
            std::process::id(),
            now_millis()?
        ));

        println!(
            "正在备份 assets: {} -> {}",
            target.display(),
            backup_dir.display()
        );
        copy_dir_contents(target, &backup_dir)?;
        Some(backup_dir)
    } else {
        println!(
            "目标 assets 不存在，将在检查后恢复为'不存在'状态: {}",
            target.display()
        );
        None
    };

    println!(
        "正在覆盖 assets: {} -> {}",
        source.display(),
        target.display()
    );
    remove_dir_if_exists(target)?;
    copy_dir_contents(source, target)?;

    Ok(AssetGuard { backup_dir })
}

fn restore_assets(backup_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let target = Path::new(TARGET_ASSETS_DIR);

    match backup_dir {
        Some(backup_dir) => {
            println!(
                "正在恢复 assets: {} -> {}",
                backup_dir.display(),
                target.display()
            );

            remove_dir_if_exists(target)?;
            copy_dir_contents(&backup_dir, target)?;
            remove_dir_if_exists(&backup_dir)?;
        }
        None => {
            println!("正在恢复 assets 为不存在状态: {}", target.display());
            remove_dir_if_exists(target)?;
        }
    }

    Ok(())
}

fn now_millis() -> anyhow::Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("系统时钟早于 UNIX_EPOCH")?
        .as_millis())
}

fn remove_dir_if_exists(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if !path.is_dir() {
        bail!("目标路径不是目录，无法删除: {}", path.display());
    }

    remove_dir(path).with_context(|| format!("删除目录失败: {}", path.display()))?;

    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if !src.is_dir() {
        bail!("源路径不是目录: {}", src.display());
    }

    let mut options = CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;

    copy_dir(src, dst, &options)
        .with_context(|| format!("复制目录内容失败: {} -> {}", src.display(), dst.display()))?;

    Ok(())
}
