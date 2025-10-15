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

    /// 行为模式：check（仅检查），fix（自动修复），aggressive-fix（激进修复）
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
    /// 激进修复模式，即使修复失败也应用修复结果
    AggressiveFix,
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

/// 宏：应用一个修改操作，如果长度达标则返回
macro_rules! apply_and_check {
    ($modified:expr, $orig_len:expr, $method:expr, $action:block) => {
        $action
        if $method.count(&$modified) <= $orig_len {
            return ($modified, true);
        }
    };
}

fn try_fix_message(
    trans_msg: &str,
    orig_len: usize,
    method: &Method,
    aggressive: bool,
) -> (String, bool) {
    let mut modified = trans_msg.to_string();

    // 初始检查，如果原文就符合长度要求，直接返回
    if method.count(&modified) <= orig_len {
        return (modified, true);
    }

    // --- 第1阶段：标准化处理 (基本无损) ---
    // 1. 全角字母数字转半角 (包括全角空格)
    apply_and_check!(modified, orig_len, method, {
        modified = modified
            .chars()
            .map(|c| full_width_to_half_width(c).unwrap_or(c))
            .collect();
    });

    // 2. 合并重复标点符号
    const PUNCT_REPLACEMENTS: [(&str, &str); 5] = [
        ("……", "…"),
        ("――", "―"),
        ("——", "—"),
        ("‥‥", "‥"),
        ("──", "─"),
    ];
    for (from, to) in PUNCT_REPLACEMENTS {
        if modified.contains(from) {
            apply_and_check!(modified, orig_len, method, {
                while modified.contains(from) {
                    modified = modified.replace(from, to);
                }
            });
        }
    }

    // --- 第2阶段：轻度缩减 (可能轻微影响语义) ---
    // 3. 同义词替换（使用更短的表达）
    const SYNONYM_REPLACEMENTS: [(&str, &str); 21] = [
        ("真是", "真"),
        ("什么", "啥"),
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
        ("一个人", "一人"),
        ("两个人", "两人"),
        ("三个人", "三人"),
        ("的时候", "时"),
        ("之前", "前"),
        ("之后", "后"),
        ("之时", "时"),
        ("如果", "若"),
    ];
    for (from, to) in SYNONYM_REPLACEMENTS {
        if modified.contains(from) {
            apply_and_check!(modified, orig_len, method, {
                modified = modified.replace(from, to);
            });
        }
    }

    // 4. 删除末尾的特定标点
    let ends_with_puncts = ['」', '。', '！', '？', '!', '?'];
    for p in ends_with_puncts {
        if modified.ends_with(p) {
            apply_and_check!(modified, orig_len, method, {
                modified.pop();
            });
        }
    }

    // 如果长度已达标，直接返回成功
    if method.count(&modified) <= orig_len {
        return (modified, true);
    }

    // --- 第3阶段：激进修复 ---
    if aggressive {
        let (aggressively_modified, fixed) = try_aggressive_fix(&modified, orig_len, method);
        return (aggressively_modified, fixed);
    }

    (modified, false)
}

/// 激进修复措施（仅在激进修复模式下使用），会不惜一切代价缩短文本
/// 返回值：(修复后的字符串, 是否成功修复到目标长度以内)
fn try_aggressive_fix(trans_msg: &str, orig_len: usize, method: &Method) -> (String, bool) {
    let mut modified = trans_msg.to_string();

    // 在激进模式下，我们会应用所有规则，而不是成功一次就返回
    // 这样可以最大程度地缩短文本

    // 1. 激进同义词替换
    apply_and_check!(modified, orig_len, method, {
        const AGGRESSIVE_SYNONYM_REPLACEMENTS: [(&str, &str); 14] = [
            ("但是", "但"),
            ("可是", "可"),
            ("因为", "因"),
            ("所以", "故"),
            ("然后", "后"),
            ("已经", "已"),
            ("知道", "知"),
            ("觉得", "觉"),
            ("可以", "可"),
            ("不要", "别"),
            ("非常", "很"),
            ("表示", "称"),
            ("自己", "自"),
            ("我们", "我等"), // "我等"是古称，比"我们"短
        ];
        for (from, to) in AGGRESSIVE_SYNONYM_REPLACEMENTS {
            modified = modified.replace(from, to);
        }
    });

    // 2. 激进修复：删除常见的"的"字所有格
    apply_and_check!(modified, orig_len, method, {
        const DE_REPLACEMENTS: [(&str, &str); 8] = [
            ("我的", "我"),
            ("你的", "你"),
            ("他的", "他"),
            ("她的", "她"),
            ("它的", "它"),
            ("我们的", "我们"),
            ("你们的", "你们"),
            ("他们的", "他们"),
        ];
        for (from, to) in DE_REPLACEMENTS {
            modified = modified.replace(from, to);
        }
    });

    // 3. 激进修复：删除所有"的"字 (这是一个非常强力的操作)
    apply_and_check!(modified, orig_len, method, {
        modified = modified.replace('的', "");
    });

    // 4. 激进修复：删除所有空白字符
    apply_and_check!(modified, orig_len, method, {
        modified.retain(|c| !c.is_whitespace());
    });

    // 5. 激进修复：删除结尾语气词
    apply_and_check!(modified, orig_len, method, {
        const MODAL_PARTICLES: [&str; 9] = ["呢", "吗", "吧", "啊", "呀", "啦", "哦", "哟", "呦"];
        for particle in MODAL_PARTICLES {
            if modified.ends_with(particle) {
                modified = modified.trim_end_matches(particle).to_string();
            }
        }
    });

    // 6. 激进修复：完全删除特定标点符号
    apply_and_check!(modified, orig_len, method, {
        const AGGRESSIVE_PUNCT_REMOVAL: [char; 12] = [
            '…', '―', '—', '‥', '~', '～', '·', '・', '，', ',', '、', ' ',
        ];
        modified.retain(|c| !AGGRESSIVE_PUNCT_REMOVAL.contains(&c));
    });

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
    let mut aggressive_fixed_count = 0;

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
                    // 尝试自动修复（非激进模式）
                    let (fixed_msg, fixed) =
                        try_fix_message(&trans_msg, orig_len, &cli.method, false);
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
                Behavior::AggressiveFix => {
                    // 激进修复模式
                    let (fixed_msg, fixed) =
                        try_fix_message(&trans_msg, orig_len, &cli.method, true);

                    // 无论是否修复成功，都更新消息
                    let map = trans_item.as_object_mut().unwrap();
                    map.insert("message".to_string(), Value::String(fixed_msg.clone()));

                    let new_len = cli.method.count(&fixed_msg);

                    if new_len <= orig_len {
                        // 修复成功，移除错误标记
                        map.remove("error");
                        fixed_count += 1;
                        aggressive_fixed_count += 1;
                        eprintln!("第 {i} 项: 激进修复成功（原:{orig_len} 修后:{new_len}）");
                    } else {
                        // 修复失败，但仍更新消息并标记错误
                        let err_text =
                            format!("原文 {orig_len} < 译文 {new_len}（激进修复后仍超长）");
                        map.insert("error".to_string(), Value::String(err_text));
                        error_count += 1;
                        aggressive_fixed_count += 1;
                        eprintln!("第 {i} 项: 激进修复后仍超长（原:{orig_len} 修后:{new_len}）");
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
        Behavior::Fix | Behavior::AggressiveFix => {
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
        Behavior::AggressiveFix => {
            if aggressive_fixed_count > 0 {
                println!(
                    "已激进修复 {} 项超长译文（其中 {} 项完全修复，{} 项修复后仍超长），输出到: {}",
                    aggressive_fixed_count,
                    fixed_count,
                    error_count,
                    output_path.display()
                );
            } else {
                println!("无需激进修复，文件已输出到: {}", output_path.display());
            }
        }
    }

    Ok(())
}
