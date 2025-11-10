use anyhow::Result;
use encoding_rs::Encoding;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 扫描指定路径列表，返回所有文件路径（可选择按后缀过滤）
pub fn collect_files(paths: Vec<String>, suffix: Option<&str>) -> Result<Vec<String>> {
    let mut results = Vec::new();

    for path_str in paths {
        let path = PathBuf::from(&path_str);

        if !path.exists() {
            anyhow::bail!("路径不存在: {path_str}");
        }

        if path.is_file() {
            if let Some(suffix) = suffix {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if !ext.eq_ignore_ascii_case(suffix) {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            results.push(path.to_string_lossy().to_string());
            continue;
        }

        for entry in WalkDir::new(&path)
            .min_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();

            // 只处理文件
            if !entry_path.is_file() {
                continue;
            }

            // 后缀过滤
            if let Some(suffix) = suffix {
                if let Some(ext) = entry_path.extension().and_then(|s| s.to_str()) {
                    if !ext.eq_ignore_ascii_case(suffix) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            results.push(entry_path.to_string_lossy().to_string());
        }
    }

    Ok(results)
}

/// 将文本中的 ASCII 字符转换为全角版本。
pub fn to_full_width(text: &str) -> Result<String> {
    let mut res = String::with_capacity(text.len());

    for ch in text.chars() {
        let code = ch as u32;
        if (0x21..=0x7E).contains(&code) {
            // 可见 ASCII 字符
            res.push(std::char::from_u32(code + 0xFEE0).unwrap());
        } else if code == 0x20 {
            // 空格
            res.push('　'); // U+3000 全角空格
        } else {
            // 其他字符原样保留
            res.push(ch);
        }
    }

    Ok(res)
}

/// 尝试用指定编码解码字节序列
/// 如果有不可映射的字节，则返回 Err
pub fn decode_strict(encoding: &'static Encoding, bytes: &[u8]) -> Result<String> {
    let mut decoder = encoding.new_decoder();
    let mut output = String::with_capacity(bytes.len() * 4); // 大致预分配
    let (result, _read) = decoder.decode_to_string_without_replacement(bytes, &mut output, true);

    match result {
        encoding_rs::DecoderResult::InputEmpty => {
            if output
                .chars()
                .all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t')
            {
                Ok(output)
            } else {
                Err(anyhow::anyhow!("解码存在控制字符: {output:?}"))
            }
        }
        encoding_rs::DecoderResult::Malformed(_, _) => Err(anyhow::anyhow!("存在无法解码的字节")),
        encoding_rs::DecoderResult::OutputFull => unreachable!(),
    }
}

/// 模拟的字节长度，通常用于替身字符
pub fn pseudo_byte_len(s: &str) -> usize {
    s.chars()
        .map(|c| if (c as u32) <= 0x7F { 1 } else { 2 })
        .sum()
}

/// 字符长度，通常用于替身字符，并且ascii码会被转换为全角
pub fn chars_len(s: &str) -> usize {
    s.chars().count()
}

/// 提取`file_path`中的文件名，然后拼接到`dir`后面
pub fn to_dir(dir: impl AsRef<str>, file_path: impl AsRef<str>) -> Option<String> {
    let dir = dir.as_ref();
    let file_path = file_path.as_ref();
    let file_name = Path::new(file_path).file_name()?;
    let file_name_str = file_name.to_str()?;

    Some(format!("{dir}/{file_name_str}"))
}

/// 提取`file_path`相对于`base_dir`的相对路径，然后拼接到`dir`后面
pub fn to_dir_with_base(
    dir: impl AsRef<str>,
    base_dir: impl AsRef<str>,
    file_path: impl AsRef<str>,
) -> Option<String> {
    let dir = Path::new(dir.as_ref());
    let base_dir = Path::new(base_dir.as_ref());
    let file_path = Path::new(file_path.as_ref());

    // 确保file_path在base_dir内
    if !file_path.starts_with(base_dir) {
        return None;
    }

    // 获取相对路径
    let relative_path = file_path.strip_prefix(base_dir).ok()?;

    // 拼接路径
    dir.join(relative_path).to_str().map(|s| s.to_string())
}

/// 改变`file_path`中的文件名的拓展名
pub fn with_ext(file_path: impl AsRef<str>, ext: impl AsRef<str>) -> Option<String> {
    let file_path = file_path.as_ref();
    let ext = ext.as_ref();
    let new_file_path = Path::new(file_path).with_extension(ext);
    Some(new_file_path.to_string_lossy().into_owned())
}

/// 将 Rust 字符串转换为 UTF-16LE 字节序
///
/// `include_bom` 决定是否在开头加上 UTF-16LE 的 BOM（0xFF, 0xFE）
/// 返回值：UTF-16LE 编码的 u8 向量
pub fn encode_utf16le(s: &str, include_bom: bool) -> Vec<u8> {
    let mut buffer = Vec::new();

    if include_bom {
        // UTF-16LE 的 BOM：FF FE
        buffer.extend_from_slice(&[0xFF, 0xFE]);
    }

    for code_unit in s.encode_utf16() {
        buffer.extend_from_slice(&code_unit.to_le_bytes());
    }

    buffer
}

/// 类似于std::fs::write，但是如果目标路径不存在，那么会尝试创建目录，然后再写入文件
pub fn write_with_dir_create<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(std::fs::write(path, contents)?)
}
