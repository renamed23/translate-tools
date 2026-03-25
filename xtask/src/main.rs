use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, anyhow, bail};
use fs_extra::dir::{CopyOptions, copy as copy_dir, remove as remove_dir};
use xshell::{Shell, cmd};

const TEST_ASSETS_DIR: &str = "xtask/test_assets";
const TARGET_ASSETS_DIR: &str = "crates/text-hook/assets";

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("check") => run_check_command(),
        Some(cmd_name) => bail!("未知的 xtask 命令: {cmd_name}"),
        None => {
            println!("用法: cargo xtask <命令>");
            println!("可用命令:");
            println!("  check    执行 text-hook feature 组合检查");
            Ok(())
        }
    }
}

fn run_check_command() -> anyhow::Result<()> {
    let shell = Shell::new()?;

    let backup = backup_and_replace_assets()?;
    let run_result = run_all_check_scenarios(&shell);
    let restore_result = restore_assets(backup);

    match (run_result, restore_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(run_err), Ok(())) => Err(run_err),
        (Ok(()), Err(restore_err)) => Err(restore_err),
        (Err(run_err), Err(restore_err)) => Err(anyhow!(
            "检查失败且资产恢复失败。\n检查错误: {run_err:#}\n恢复错误: {restore_err:#}"
        )),
    }
}

fn run_all_check_scenarios(shell: &Shell) -> anyhow::Result<()> {
    let scenarios = build_scenarios();

    for scenario in scenarios {
        println!("\n=== 检查场景: {} ===", scenario.name);
        println!("features = {}", scenario.features.join(","));

        run_check_for_target(shell, "check-text-hook", &scenario.features)
            .with_context(|| format!("x86 检查失败，场景: {}", scenario.name))?;

        if scenario.run_x64 {
            run_check_for_target(shell, "check-text-hook64", &scenario.features)
                .with_context(|| format!("x64 检查失败，场景: {}", scenario.name))?;
        }
    }

    Ok(())
}

fn run_check_for_target(
    shell: &Shell,
    cargo_alias: &str,
    features: &[String],
) -> anyhow::Result<()> {
    let joined = features.join(",");
    // 仅关注是否有错误，抑制 warning 输出
    cmd!(shell, "cargo {cargo_alias} --quiet --features {joined}")
        .env("RUSTFLAGS", "-Awarnings")
        .run()?;
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
            name: "default_impl/bind_asset_virtualizer/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["bind_asset_virtualizer"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_asset_virtualizer/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_asset_virtualizer"],
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
                &["disable_forced_font"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_font_manager/on_with_arg".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_font_manager", "disable_forced_font"],
                &[],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_path_redirector/off".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl"],
                &["bind_path_redirector"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_path_redirector/on".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_path_redirector"],
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
    for imp in ["c4"] {
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

fn all_functional_impl_base<'a>() -> &'a [&'a str] {
    &[
        // 功能类 feature
        "enable_text_mapping_debug",
        "enable_debug_output",
        "enable_worker_thread",
        "enable_veh",
        "enable_resource_pack",
        "enable_x64dbg_1337_patch",
        "enable_text_patch",
        "enable_patch",
        "enable_custom_font",
        "export_default_dll_main",
        "enable_locale_emulator",
        "export_hook_symbols",
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

#[derive(Debug)]
struct AssetsBackup {
    backup_dir: Option<PathBuf>,
}

fn backup_and_replace_assets() -> anyhow::Result<AssetsBackup> {
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

    Ok(AssetsBackup { backup_dir })
}

fn restore_assets(backup: AssetsBackup) -> anyhow::Result<()> {
    let target = Path::new(TARGET_ASSETS_DIR);

    match backup.backup_dir {
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
