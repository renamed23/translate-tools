use anyhow::Context;
use xshell::{Shell, cmd};

use crate::scenarios::{all_functional_impl_base, build_scenarios, feature_set};

pub fn run_test(shell: &Shell) -> anyhow::Result<()> {
    let _guard = crate::assets::backup_and_replace_assets()?;
    crate::commands::test_text_hook::build_and_run_test_bin(shell)?;
    run_unit_tests(shell)?;
    check_scenarios(shell)?;
    println!("\n✅ test 完成！");
    Ok(())
}

fn run_unit_tests(shell: &Shell) -> anyhow::Result<()> {
    println!("\n>>> [Test] 正在运行单元测试...");

    let features = feature_set(all_functional_impl_base(), &["default_impl"], &[]);
    let features_str = features.join(",");
    println!("  执行 Test: cargo test -p text-hook --lib --features {features_str}",);
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

fn check_scenarios(shell: &Shell) -> anyhow::Result<()> {
    let scenarios = build_scenarios();

    for scenario in &scenarios {
        println!("\n>>> [Check] 正在处理场景: {}", scenario.name);
        let features = scenario.features.join(",");

        let mut targets = vec!["check-text-hook"];
        if scenario.run_x64 {
            targets.push("check-text-hook64");
        }

        for target in &targets {
            println!("  执行 Check: {} --features {}", target, features);
            cmd!(shell, "cargo {target} --quiet --features {features}")
                .env("RUSTFLAGS", "-Awarnings")
                .run()
                .with_context(|| format!("场景 {} 检查失败", scenario.name))?;
        }
    }

    println!("\n✅ 所有场景 Check 完成！");
    Ok(())
}
