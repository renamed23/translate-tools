use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use translate_utils::{
    replacement_pool::{PoolBuilder, ReplacementPool},
    text::{Item, Text},
    utils::{collect_files, to_dir},
};

#[derive(Parser, Debug)]
#[command(about = "将目标json中不兼容shift-jis的字符映射为shift-jis替身字符的实用工具")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 映射文本中的字符到替身
    Map {
        #[arg(long, required = true, help = "一个或多个的json或者其目录的路径")]
        path: Vec<String>,

        #[arg(long, default_value = "./replaced/", help = "输出目录的路径")]
        output: String,

        #[arg(
            long,
            default_value = "replacement_pool.json",
            help = "包含替身字符的 JSON 文件路径"
        )]
        replacement_pool: String,
    },
    /// 生成替身池
    GeneratePool {
        #[arg(long, required = true, help = "一个或多个的json或者其目录的路径")]
        path: Vec<String>,

        #[arg(
            long,
            default_value = "replacement_pool.json",
            help = "输出JSON文件路径"
        )]
        output: String,

        #[arg(long, action = clap::ArgAction::SetTrue, help = "是否生成CP932字符(默认为JIS0208)")]
        cp932: bool,
    },
}

fn map(path: Vec<String>, output: String, replacement_pool: String) -> Result<()> {
    let files = collect_files(path, Some("json"))?;
    let mut replacement_pool = ReplacementPool::from_path(replacement_pool)?;

    std::fs::create_dir_all(&output)?;

    for file in files {
        let text = Text::from_path(&file)?;
        let new_text = text.generate_text(|name, message| {
            let mapped_name = match name {
                Some(name) => Some(replacement_pool.map_text(name)?),
                None => None,
            };
            let mapped_msg = replacement_pool.map_text(message)?;
            Ok((mapped_name, mapped_msg))
        })?;

        let out_path = to_dir(&output, &file).ok_or_else(|| anyhow!("构建输出路径失败: {file}"))?;
        new_text.write_to_path(out_path)?;
    }

    replacement_pool.write_charmap_to_path(
        to_dir(&output, "mapping.json").ok_or_else(|| anyhow!("构建输出路径失败"))?,
    )?;
    println!("处理完成！新字典已保存到: {output}");
    println!("字符映射表已保存到: mapping.json");

    Ok(())
}

fn generate_pool(path: Vec<String>, output: String, cp932: bool) -> Result<()> {
    println!("开始生成替身池...");

    let mut pool = PoolBuilder::default();
    pool.generate_shiftjis_pool(cp932)?;

    let files = collect_files(path, Some("json"))?;

    for file in files {
        let text = Text::from_path(&file)?;

        for Item { name, message } in &text.items {
            if let Some(name) = name {
                pool.exclude_used_chars(name);
            }
            pool.exclude_used_chars(message);
        }
    }

    pool.write_to_path(&output)?;
    println!("成功生成替身字符池，包含 {} 个字符", pool.len());
    println!("保存到: {output}");

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Map {
            path,
            output,
            replacement_pool,
        } => map(path, output, replacement_pool)?,
        Commands::GeneratePool {
            path,
            output,
            cp932,
        } => generate_pool(path, output, cp932)?,
    }

    Ok(())
}
