use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub features: Vec<String>,
    pub run_x64: bool,
}

pub fn build_scenarios() -> Vec<Scenario> {
    let mut scenarios = vec![
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
            name: "default_impl/bind_vfs/on_without_find_file".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_vfs"],
                &["enable_vfs_find_file"],
            ),
            run_x64: true,
        },
        Scenario {
            name: "default_impl/bind_vfs/on_with_find_file".to_string(),
            features: feature_set(
                all_functional_impl_base(),
                &["default_impl", "bind_vfs", "enable_vfs_find_file"],
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

pub fn all_functional_impl_base() -> &'static [&'static str] {
    &[
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

pub fn all_functional_impl_base_test_bin() -> &'static [&'static str] {
    &[
        "bind_vfs",
        "enable_vfs_find_file",
        "bind_window_title_overrider",
        "bind_font_manager",
        "default_impl",
    ]
}

pub fn build_scenarios_test_bin() -> Vec<Scenario> {
    let base = all_functional_impl_base_test_bin();

    let param_features = [
        ("disable_forced_font", "off"),
        ("disable_forced_font", "on"),
        ("enable_window_title_override", "off"),
        ("enable_window_title_override", "on"),
        ("enable_iat_hook", "on"),
        ("enable_iat_hook", "off"),
    ];

    param_features
        .iter()
        .map(|(feature, state)| {
            let name = format!("test_bin/{feature}/{state}");
            let features = match *state {
                "on" => feature_set(base, &[feature], &[]),
                "off" => feature_set(base, &[], &[feature]),
                _ => unreachable!(),
            };
            Scenario {
                name,
                features,
                run_x64: true,
            }
        })
        .collect()
}

pub fn feature_set(base: &[&str], add: &[&str], remove: &[&str]) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();

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
