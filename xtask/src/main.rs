use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use fs_extra::dir::{CopyOptions, copy as copy_dir, remove as remove_dir};
use xshell::{Shell, cmd};

const TEST_ASSETS_DIR: &str = "xtask/test_assets";
const TARGET_ASSETS_DIR: &str = "crates/text-hook/assets";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Check,
    Fix,
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let shell = Shell::new()?;

    match args.next().as_deref() {
        Some("check") => run_xtask_action(&shell, Action::Check),
        Some("fix") => {
            ensure_git_clean(&shell)?;
            run_xtask_action(&shell, Action::Fix)
        }
        Some("use-test-assets") => run_use_test_assets_command(),
        Some(cmd_name) => bail!("未知的 xtask 命令: {cmd_name}"),
        None => {
            println!("用法: cargo xtask <命令>");
            println!("可用命令:");
            println!("  check           执行所有场景的 cargo check");
            println!("  fix             执行所有场景的 clippy --fix (自动处理 dirty 状态)");
            println!("  use-test-assets 用测试资产覆盖正式资产");
            Ok(())
        }
    }
}

fn run_use_test_assets_command() -> anyhow::Result<()> {
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

fn run_xtask_action(shell: &Shell, action: Action) -> anyhow::Result<()> {
    let _backup_guard = backup_and_replace_assets()?;

    if action == Action::Check {
        run_unit_tests(shell)?;
    }

    let scenarios = build_scenarios();

    for scenario in scenarios {
        println!("\n>>> [{:?}] 正在处理场景: {}", action, scenario.name);
        let features = scenario.features.join(",");

        let targets = match action {
            Action::Check => {
                let mut targets = vec!["check-text-hook"];
                if scenario.run_x64 {
                    targets.push("check-text-hook64");
                }
                targets
            }
            Action::Fix => {
                vec!["fix-text-hook"]
            }
        };

        for target in targets {
            match action {
                Action::Check => {
                    println!("  执行 Check: {} --features {}", target, features);
                    cmd!(shell, "cargo {target} --quiet --features {features}")
                        .env("RUSTFLAGS", "-Awarnings")
                        .run()
                        .with_context(|| format!("场景 {} 检查失败", scenario.name))?;
                }
                Action::Fix => {
                    println!("  执行 Fix: {} --features {}", target, features);
                    cmd!(shell, "cargo {target} --allow-dirty --features {features}")
                        .run()
                        .with_context(|| format!("场景 {} 自动修复失败", scenario.name))?;
                }
            }
        }
    }

    println!("\n✅ 所有场景 {:?} 完成！", action);
    Ok(())
}

fn run_unit_tests(shell: &Shell) -> anyhow::Result<()> {
    println!("\n>>> [Test] 正在运行单元测试...");

    let features = feature_set(all_functional_impl_base(), &["default_impl"], &[]);
    let features_str = features.join(",");
    println!("  执行 Test: cargo test -p text-hook --lib --features {}", features_str);
    cmd!(
        shell,
        "cargo test --package text-hook --lib --quiet --features {features_str}"
    )
    .env("RUSTFLAGS", "-Awarnings")
    .run()
    .context("text-hook 单元测试失败")?;

    println!("  执行 Test: cargo test -p translate-macros --lib");
    cmd!(shell, "cargo test --package translate-macros --lib --quiet")
        .env("RUSTFLAGS", "-Awarnings")
        .run()
        .context("translate-macros 单元测试失败")?;

    println!("  ✅ 单元测试通过！");
    Ok(())
}

#[derive(Clone, Debug)]
struct Scenario {
    name: String,
    features: Vec<String>,
    run_x64: bool,
}

fn build_scenarios() -> Vec<Scenario> {
    let mut scenarios = vec![
        // default_impl: 覆盖类 feature 各自两种行为
        Scenario {
            name: "default_impl/bind_lifecycle_guard/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["bind_lifecycle_guard"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_lifecycle_guard/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_lifecycle_guard"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_font_manager/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["bind_font_manager"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_font_manager/on_without_arg".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_font_manager"],
                &["disable_forced_font", "enable_collect_host_font_config"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_font_manager/on_with_arg".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &[
                    "default_impl",
                    "bind_font_manager",
                    "disable_forced_font",
                    "enable_collect_host_font_config",
                ],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_vfs/off".to_string(),
            features: feature_set(all_functional_impl_base(), &["default_impl"], &["bind_vfs"]),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_vfs/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_vfs"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_text_mapping/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["bind_text_mapping"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_text_mapping/on_without_arg".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_text_mapping"],
                &["assume_text_out_arg_c_is_byte_len"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_text_mapping/on_with_arg".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &[
                    "default_impl",
                    "bind_text_mapping",
                    "assume_text_out_arg_c_is_byte_len",
                ],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_user_interface_patcher/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["bind_user_interface_patcher"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_user_interface_patcher/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_user_interface_patcher"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_window_title_overrider/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["bind_window_title_overrider"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_window_title_overrider/on_without_arg".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_window_title_overrider"],
                &["enable_window_title_override"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_window_title_overrider/on_with_arg".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &[
                    "default_impl",
                    "bind_window_title_overrider",
                    "enable_window_title_override",
                ],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/resource_pack/external".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["embed_resource_pack"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/resource_pack/embedded".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "embed_resource_pack"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/hook_backend/inline".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["enable_iat_hook"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/hook_backend/iat".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "enable_iat_hook"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/hook_backend/iat_with_strip".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "enable_iat_hook_with_strip"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/extract_text/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["extract_text"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/extract_text/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "extract_text"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/extract_patch/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["extract_patch"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/extract_patch/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "extract_patch"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/disable_forced_font/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["disable_forced_font"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/disable_forced_font/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "disable_forced_font"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/assume_text_out_arg_c_is_byte_len/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["assume_text_out_arg_c_is_byte_len"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/assume_text_out_arg_c_is_byte_len/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "assume_text_out_arg_c_is_byte_len"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/auto_apply_1337_patch/on_attach".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "auto_apply_1337_patch_on_attach"],
                &["auto_apply_1337_patch_on_hwbp_hit"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/auto_apply_1337_patch/on_hwbp_hit".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "auto_apply_1337_patch_on_hwbp_hit"],
                &["auto_apply_1337_patch_on_attach"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/window_title_override/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["enable_window_title_override"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/window_title_override/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "enable_window_title_override"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/delayed_attach/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &[
                    "enable_delayed_attach",
                    "enable_dll_hijacking",
                    "enable_hwbp_from_constants",
                ],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/delayed_attach/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &[
                    "default_impl",
                    "enable_delayed_attach",
                    "enable_dll_hijacking",
                    "enable_hwbp_from_constants",
                ],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/enable_delayed_attach_static/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &[
                    "default_impl",
                    "enable_delayed_attach_static",
                    "enable_dll_hijacking",
                    "enable_hwbp_from_constants",
                ],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/win_event_hook/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["enable_win_event_hook"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/win_event_hook/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "enable_win_event_hook"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/gl_painter/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["enable_gl_painter"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/gl_painter/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "enable_gl_painter"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/overlay/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["enable_overlay"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/overlay/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "enable_overlay"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/overlay_gl/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &[
                    "enable_overlay_gl",
                    "enable_overlay_gl_painter",
                    "enable_overlay_egui",
                    "bind_egui_io",
                ],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/overlay_gl/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &[
                    "default_impl",
                    "enable_overlay_gl",
                    "enable_overlay_gl_painter",
                    "enable_overlay_egui",
                    "bind_egui_io",
                    "bind_egui_default_ui",
                    "enable_egui_logger",
                    "enable_egui_demo",
                    "enable_egui_font_property_editor",
                    "enable_collect_host_font_config",
                ],
                &[],
            ),
            run_x64: true,
        },
    ];

    // 其余 impl: 只跑 x86
    let game_impls = [
        "c4",
        "complets",
        "natsu_natsu",
        "seraph",
        "g0win",
        "hitocos2",
        "hitocos",
        "old_minori",
        "nocturne",
        "blackbox",
    ];

    for imp in game_impls {
        scenarios.push(Scenario {
            name: format!("{imp}/all_functional"),
            features: feature_set(all_functional_impl_base(), &[imp], &[]),
            run_x64: false,
        });
    }

    // 非 default_impl 的特例补测
    #[allow(clippy::single_element_loop)]
    for imp in ["c4", "old_minori"] {
        scenarios.push(Scenario {
            name: format!("{imp}/patch_extracting"),
            features: feature_set(all_functional_impl_base(), &[imp, "extract_patch"], &[]),
            run_x64: false,
        });
    }

    // 暂时先占位
    for imp in [] {
        scenarios.push(Scenario {
            name: format!("{imp}/text_extracting"),
            features: feature_set(all_functional_impl_base(), &[imp, "extract_text"], &[]),
            run_x64: false,
        });
    }

    // 保证顺序稳定 + 去重
    dedup_scenarios(scenarios)
}

fn dedup_scenarios(scenarios: Vec<Scenario>) -> Vec<Scenario> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();

    for scenario in scenarios {
        let key = format!("{}|{}", scenario.run_x64, scenario.features.join(","));
        if seen.insert(key) {
            out.push(scenario);
        }
    }

    out
}

fn all_functional_impl_base() -> &'static [&'static str] {
    &[
        // 功能类 feature
        "enable_text_mapping_debug",
        "enable_debug_output",
        "enable_thread_manager",
        "enable_ui_thread",
        "enable_veh",
        "enable_resource_pack",
        "enable_x64dbg_1337_patch",
        "enable_text_patch",
        "enable_patch",
        "enable_embedded_font",
        "enable_custom_font",
        "export_default_dll_main",
        "enable_locale_emulator",
        "export_hook_symbols",
        "enable_vfs",
    ]
}

fn feature_set(base: &[&str], add: &[&str], remove: &[&str]) -> Vec<String> {
    let mut set = BTreeSet::new();

    for item in base {
        set.insert((*item).to_string());
    }
    for item in add {
        set.insert((*item).to_string());
    }
    for item in remove {
        set.remove(*item);
    }

    set.into_iter().collect()
}

struct AssetGuard {
    backup_dir: Option<PathBuf>,
}

impl Drop for AssetGuard {
    fn drop(&mut self) {
        if let Err(e) = restore_assets(self.backup_dir.take()) {
            println!("恢复资产出现问题: {e}");
        }
    }
}

fn ensure_git_clean(shell: &Shell) -> anyhow::Result<()> {
    let status = cmd!(shell, "git status --porcelain").read()?;
    if !status.is_empty() {
        bail!(
            "Git 工作区有未提交的改动！为了防止 clippy --fix 覆盖你的代码，请先 Commit 或 \
             Stash。\n{status}"
        );
    }
    Ok(())
}

fn backup_and_replace_assets() -> anyhow::Result<AssetGuard> {
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
            "目标 assets 不存在，将在检查后恢复为“不存在”状态: {}",
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
