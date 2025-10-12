use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde_json::Value;
use std::{fs, path::PathBuf};

use encoding_rs::SHIFT_JIS;
use translate_utils::utils::{chars_len, pseudo_byte_len};

/// 工具：比较原文 JSON 与译文 JSON 中 message 的"长度"，并在译文中添加/移除 error 字段。
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "检查译文 message 长度并在超长时写入 error 字段（支持自动修复）"
)]
struct Cli {
    /// 原文 JSON 文件路径
    #[arg(short, long)]
    orig: PathBuf,

    /// 译文 JSON 文件路径（将被覆盖或在修复时输出到新文件）
    #[arg(short, long)]
    trans: PathBuf,

    /// 比较方法：pseudo -> pseudo_byte_len(译文) + CP932 字节(原文), chars -> chars_len
    #[arg(short, long, value_enum, default_value_t = Method::Pseudo)]
    method: Method,

    /// 行为模式：check（仅检查），fix（自动修复）
    #[arg(short, long, value_enum, default_value_t = Behavior::Check)]
    behave: Behavior,
}

#[derive(ValueEnum, Clone, Debug)]
enum Method {
    /// pseudo: 原文字节使用 CP932 (Windows-31J / MS932) 的字节长度，译文使用 pseudo_byte_len
    Pseudo,
    /// chars: 字符计数（char count）
    Chars,
}

#[derive(ValueEnum, Clone, Debug)]
enum Behavior {
    /// 仅检查，标记错误
    Check,
    /// 自动修复超长译文
    Fix,
}

impl Method {
    /// 对 *译文* 使用的计数函数（保留原先语义）
    fn count(&self, str: impl AsRef<str>) -> usize {
        match self {
            Method::Pseudo => pseudo_byte_len(str.as_ref()),
            Method::Chars => chars_len(str.as_ref()),
        }
    }
}

fn full_width_to_half_width(c: char) -> Option<char> {
    match c {
        'Ａ' => Some('A'),
        'Ｂ' => Some('B'),
        'Ｃ' => Some('C'),
        'Ｄ' => Some('D'),
        'Ｅ' => Some('E'),
        'Ｆ' => Some('F'),
        'Ｇ' => Some('G'),
        'Ｈ' => Some('H'),
        'Ｉ' => Some('I'),
        'Ｊ' => Some('J'),
        'Ｋ' => Some('K'),
        'Ｌ' => Some('L'),
        'Ｍ' => Some('M'),
        'Ｎ' => Some('N'),
        'Ｏ' => Some('O'),
        'Ｐ' => Some('P'),
        'Ｑ' => Some('Q'),
        'Ｒ' => Some('R'),
        'Ｓ' => Some('S'),
        'Ｔ' => Some('T'),
        'Ｕ' => Some('U'),
        'Ｖ' => Some('V'),
        'Ｗ' => Some('W'),
        'Ｘ' => Some('X'),
        'Ｙ' => Some('Y'),
        'Ｚ' => Some('Z'),
        'ａ' => Some('a'),
        'ｂ' => Some('b'),
        'ｃ' => Some('c'),
        'ｄ' => Some('d'),
        'ｅ' => Some('e'),
        'ｆ' => Some('f'),
        'ｇ' => Some('g'),
        'ｈ' => Some('h'),
        'ｉ' => Some('i'),
        'ｊ' => Some('j'),
        'ｋ' => Some('k'),
        'ｌ' => Some('l'),
        'ｍ' => Some('m'),
        'ｎ' => Some('n'),
        'ｏ' => Some('o'),
        'ｐ' => Some('p'),
        'ｑ' => Some('q'),
        'ｒ' => Some('r'),
        'ｓ' => Some('s'),
        'ｔ' => Some('t'),
        'ｕ' => Some('u'),
        'ｖ' => Some('v'),
        'ｗ' => Some('w'),
        'ｘ' => Some('x'),
        'ｙ' => Some('y'),
        'ｚ' => Some('z'),
        '０' => Some('0'),
        '１' => Some('1'),
        '２' => Some('2'),
        '３' => Some('3'),
        '４' => Some('4'),
        '５' => Some('5'),
        '６' => Some('6'),
        '７' => Some('7'),
        '８' => Some('8'),
        '９' => Some('9'),
        _ => None,
    }
}

fn try_fix_message(trans_msg: &str, orig_len: usize, method: &Method) -> (String, bool) {
    let mut modified = trans_msg.to_string();

    // 1. 全角字母数字转半角
    let mut temp = String::new();
    for c in modified.chars() {
        if let Some(half) = full_width_to_half_width(c) {
            temp.push(half);
        } else {
            temp.push(c);
        }
    }
    modified = temp;

    // 检查是否已解决
    if method.count(&modified) <= orig_len {
        return (modified, true);
    }

    // 2. 去除全角空格
    if modified.contains('\u{3000}') {
        modified = modified.replace('\u{3000}', "");

        if method.count(&modified) <= orig_len {
            return (modified, true);
        }
    }

    // 3. 合并重复标点符号
    const PUNCT_REPLACEMENTS: [(&str, &str); 4] = [
        ("……", "…"), // 多个省略号合并为一个
        ("――", "―"), // 多个长破折号合并为一个
        ("——", "—"), // 多个长破折号合并为一个
        ("‥‥", "‥"), // 多个双点省略号合并为一个
    ];

    for (from, to) in PUNCT_REPLACEMENTS.iter() {
        if modified.contains(from) {
            while modified.contains(from) {
                modified = modified.replace(from, to);
            }

            if method.count(&modified) <= orig_len {
                return (modified, true);
            }
        }
    }

    // 4. 删除对话边框
    if modified.ends_with('」') {
        modified.pop();

        if method.count(&modified) <= orig_len {
            return (modified, true);
        }
    }

    // 5. 删除结尾句号
    if modified.ends_with('。') {
        modified.pop();

        if method.count(&modified) <= orig_len {
            return (modified, true);
        }
    }

    // 6. 同义词替换（使用更短的表达）
    const SYNONYM_REPLACEMENTS: [(&str, &str); 21] = [
        ("真是", "真"),
        ("什么", "啥"),
        // 省略"一"字类
        ("那一个", "那个"),
        ("哪一个", "哪个"),
        ("某一个", "某个"),
        ("每一个", "每个"),
        ("是一种", "是种"),
        ("这一部分", "这部分"),
        ("是一个", "是个"),
        ("有一个", "有个"),
        ("是一名", "是名"),
        ("是一位", "是位"),
        ("是一件", "是件"),
        // 省略"个"字类
        ("一个人", "一人"),
        ("两个人", "两人"),
        ("三个人", "三人"),
        // 时间表达简化
        ("的时候", "时"),
        ("之前", "前"),
        ("之后", "后"),
        ("之时", "时"),
        // 连接词简化
        ("如果", "若"),
    ];

    for (from, to) in SYNONYM_REPLACEMENTS.iter() {
        if modified.contains(from) {
            modified = modified.replace(from, to);
            if method.count(&modified) <= orig_len {
                return (modified, true);
            }
        }
    }

    (modified, false)
}

/// 使用 CP932 (Windows-31J / MS932) 对原文进行字节编码后返回长度。
fn orig_len_cp932_bytes(s: &str) -> usize {
    let (bytes, _, _) = SHIFT_JIS.encode(s);
    bytes.len()
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 读取文件
    let orig_text = fs::read_to_string(&cli.orig)
        .with_context(|| format!("无法读取原文文件: {}", cli.orig.display()))?;
    let trans_text = fs::read_to_string(&cli.trans)
        .with_context(|| format!("无法读取译文文件: {}", cli.trans.display()))?;

    // 解析为 JSON
    let orig_value: Value = serde_json::from_str(&orig_text)
        .with_context(|| format!("无法解析原文 JSON: {}", cli.orig.display()))?;
    let mut trans_value: Value = serde_json::from_str(&trans_text)
        .with_context(|| format!("无法解析译文 JSON: {}", cli.trans.display()))?;

    let orig_array = orig_value
        .as_array()
        .context("原文 JSON 不是数组（期望顶层为 JSON 数组）")?;
    let trans_array = trans_value
        .as_array_mut()
        .context("译文 JSON 不是数组（期望顶层为 JSON 数组）")?;

    // 长度必须相等
    if orig_array.len() != trans_array.len() {
        anyhow::bail!(
            "两个 JSON 数组长度不一致：原文 {} 项，译文 {} 项",
            orig_array.len(),
            trans_array.len()
        );
    }

    let mut error_count = 0;
    let mut fixed_count = 0;

    for (i, (orig_item, trans_item)) in orig_array.iter().zip(trans_array.iter_mut()).enumerate() {
        // 提取 message 字段
        let orig_msg = orig_item
            .get("message")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        let trans_msg = trans_item
            .get("message")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();

        // 计算原文长度：Pseudo 模式下按 CP932 字节长度，Chars 模式下按字符数
        let orig_len = match cli.method {
            Method::Pseudo => orig_len_cp932_bytes(&orig_msg),
            Method::Chars => chars_len(&orig_msg),
        };

        // 译文长度：按方法定义（Pseudo 使用 pseudo_byte_len）
        let trans_len = cli.method.count(&trans_msg);

        if trans_len > orig_len {
            match cli.behave {
                Behavior::Check => {
                    // 需要加入 error 字段
                    let err_text = format!("原文 {orig_len} < 译文 {trans_len}");
                    let map = trans_item.as_object_mut().unwrap();
                    map.insert("error".to_string(), Value::String(err_text));
                    error_count += 1;
                    eprintln!("第 {i} 项: 插入 error 字段（原:{orig_len} 译:{trans_len}）");
                }
                Behavior::Fix => {
                    // 尝试自动修复
                    let (fixed_msg, fixed) = try_fix_message(&trans_msg, orig_len, &cli.method);
                    if fixed {
                        // 更新消息
                        let map = trans_item.as_object_mut().unwrap();
                        map.insert("message".to_string(), Value::String(fixed_msg.clone()));
                        map.remove("error");
                        fixed_count += 1;
                        eprintln!(
                            "第 {i} 项: 自动修复成功（原:{orig_len} 修后:{}）",
                            cli.method.count(&fixed_msg)
                        );
                    } else {
                        // 需要加入 error 字段
                        let err_text = format!("原文 {orig_len} < 译文 {trans_len}");
                        let map = trans_item.as_object_mut().unwrap();
                        map.insert("error".to_string(), Value::String(err_text));
                        error_count += 1;
                        eprintln!("第 {i} 项: 插入 error 字段（原:{orig_len} 译:{trans_len}）");
                    }
                }
            }
        } else {
            // 移除可能存在的 error 字段
            let map = trans_item.as_object_mut().unwrap();
            if map.remove("error").is_some() {
                eprintln!("第 {i} 项: 移除已有的 error 字段（原:{orig_len} 译:{trans_len}）");
            }
        }
    }

    // 确定输出路径
    let output_path = match cli.behave {
        Behavior::Check => cli.trans.clone(),
        Behavior::Fix => {
            let mut new_path = cli.trans.clone();
            let file_name = new_path.file_stem().unwrap().to_str().unwrap();
            let extension = new_path.extension().unwrap().to_str().unwrap();
            new_path.set_file_name(format!("{file_name}_modified.{extension}"));
            new_path
        }
    };

    // 将修改后的译文写入文件（漂亮格式）
    let out =
        serde_json::to_string_pretty(&trans_value).context("将修改后的译文序列化为 JSON 失败")?;
    fs::write(&output_path, out)
        .with_context(|| format!("无法将修改后的译文写回: {}", output_path.display()))?;

    match cli.behave {
        Behavior::Check => {
            if error_count > 0 {
                println!(
                    "已写回 {}（已标注 {} 项超长）。",
                    output_path.display(),
                    error_count
                );
            } else {
                println!("检查成功：未发现超长译文，文件已写回（清除可能存在的 error 字段）。");
            }
        }
        Behavior::Fix => {
            if fixed_count > 0 {
                println!(
                    "已自动修复 {} 项超长译文，输出到: {}",
                    fixed_count,
                    output_path.display()
                );
            }
            if error_count > 0 {
                println!("仍有 {error_count} 项无法自动修复，需要人工处理。");
            } else {
                println!("所有超长译文已自动修复，输出到: {}", output_path.display());
            }
        }
    }

    Ok(())
}
