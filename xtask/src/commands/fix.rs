use anyhow::{Context, bail};
use xshell::{Shell, cmd};

use crate::scenarios::build_scenarios;

pub fn run_fix(shell: &Shell) -> anyhow::Result<()> {
    ensure_git_clean(shell)?;

    let _guard = crate::assets::backup_and_replace_assets()?;

    let scenarios = build_scenarios();

    for scenario in &scenarios {
        println!("\n>>> [Fix] 正在处理场景: {}", scenario.name);
        let features = scenario.features.join(",");

        println!("  执行 Fix: fix-text-hook --features {features}");
        cmd!(
            shell,
            "cargo fix-text-hook --allow-dirty --features {features}"
        )
        .run()
        .with_context(|| format!("场景 {} 自动修复失败", scenario.name))?;
    }

    println!("\n✅ 所有场景 Fix 完成！");
    Ok(())
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
