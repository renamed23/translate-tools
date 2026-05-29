mod assets;
mod commands;
mod scenarios;

use anyhow::bail;
use xshell::Shell;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let shell = Shell::new()?;

    match args.next().as_deref() {
        Some("test") => commands::run_test(&shell),
        Some("test-text-hook") => commands::run_test_text_hook(&shell),
        Some("fix") => commands::run_fix(&shell),
        Some("use-test-assets") => commands::run_use_test_assets(),
        Some(cmd_name) => bail!("未知的 xtask 命令: {cmd_name}"),
        None => {
            println!("用法: cargo xtask <命令>");
            println!("可用命令:");
            println!("  test            执行所有场景的 check + unit-test + test-text-hook");
            println!("  test-text-hook  构建 text-hook + test_bin 并运行测试");
            println!("  fix             执行所有场景的 clippy --fix (自动处理 dirty 状态)");
            println!("  use-test-assets 用测试资产覆盖正式资产");
            Ok(())
        }
    }
}
