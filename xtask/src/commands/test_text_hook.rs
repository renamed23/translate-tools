use std::path::Path;

use anyhow::Context;
use xshell::{Shell, cmd};

use crate::scenarios::build_scenarios_test_bin;

pub fn run_test_text_hook(shell: &Shell) -> anyhow::Result<()> {
    let _guard = crate::assets::backup_and_replace_assets()?;
    build_and_run_test_bin(shell)
}

pub fn build_and_run_test_bin(shell: &Shell) -> anyhow::Result<()> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("获取 cargo metadata 失败")?;
    let target_dir = metadata.target_directory;
    let xtask_root = metadata.workspace_root.join("xtask");

    setup_test_bin_assets(xtask_root.as_std_path())?;

    let scenarios = build_scenarios_test_bin();

    for scenario in &scenarios {
        let features = scenario.features.join(",");

        let mut targets = vec![("i686-pc-windows-msvc", "x86", "build-text-hook")];
        if scenario.run_x64 {
            targets.push(("x86_64-pc-windows-msvc", "x64", "build-text-hook64"));
        }

        for (target, label, build_cmd) in &targets {
            println!(
                "\n>>> [test-text-hook] 场景: {} | {label} ({target})",
                scenario.name
            );

            println!("  正在构建 text-hook...");
            cmd!(shell, "cargo {build_cmd} --quiet --features {features}")
                .env("RUSTFLAGS", "-Awarnings")
                .run()
                .with_context(|| {
                    format!("构建 text-hook 失败 (场景: {}, {label})", scenario.name)
                })?;

            println!("  正在构建 test_bin...");
            cmd!(
                shell,
                "cargo build --quiet --package test_bin --release --target {target}"
            )
            .env("RUSTFLAGS", "-Awarnings")
            .run()
            .with_context(|| format!("构建 test_bin 失败 (场景: {}, {label})", scenario.name))?;

            let exe_path = target_dir.join(target).join("release").join("test_bin.exe");

            println!("  运行: {}", exe_path);

            let status = std::process::Command::new(&exe_path)
                .current_dir(&xtask_root)
                .env("TEXT_HOOK_FEATURES", &features)
                .status()
                .with_context(|| {
                    format!("运行 test_bin 失败 (场景: {}, {label})", scenario.name)
                })?;

            if !status.success() {
                anyhow::bail!(
                    "test_bin 返回非0退出码 (场景: {}, {label}): {}",
                    scenario.name,
                    status
                );
            }

            println!("  ✅ 通过");
        }
    }

    Ok(())
}

fn setup_test_bin_assets(xtask_root: &Path) -> anyhow::Result<()> {
    let base = xtask_root.join("test_bin_assets");

    let files: &[(&str, &str)] = &[
        ("src_data/file_in_both.txt", "src_data/file_in_both.txt"),
        (
            "src_data/file_only_in_src.txt",
            "src_data/file_only_in_src.txt",
        ),
        ("src_data/nested/file.txt", "src_data/nested/file.txt"),
        ("dst_data/file_in_both.txt", "dst_data/file_in_both.txt"),
        (
            "dst_data/file_only_in_dst.txt",
            "dst_data/file_only_in_dst.txt",
        ),
        ("dst_data/nested/file.txt", "dst_data/nested/file.txt"),
        ("force_data/file.txt", "force_data/file.txt"),
    ];

    for (rel_path, content) in files {
        let path = base.join(rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建目录失败: {}", parent.display()))?;
        }
        std::fs::write(&path, content)
            .with_context(|| format!("写入文件失败: {}", path.display()))?;
    }

    Ok(())
}
