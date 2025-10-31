use windows_sys::Win32::Globalization::MultiByteToWideChar;

use crate::constant;

mod mapping_data {
    #[cfg(not(feature = "generate_full_mapping_data"))]
    translate_macros::generate_mapping_data!("assets/mapping.json");

    #[cfg(feature = "generate_full_mapping_data")]
    translate_macros::generate_mapping_data!("assets/mapping.json", "assets/translated.json");
}

/// 将指定shift-jis字节中的替身字符映射为指定的字符并转换为utf16
///
/// # 参数
/// - `bytes`: 输入的shift-jis字节序列
/// - `buffer`: 输出缓冲区，用于存储转换后的utf16字符
///
/// # 返回值
/// 返回实际写入缓冲区的字符数量。如果缓冲区太小，会进行截断。
#[allow(dead_code)]
pub fn map_chars(bytes: &[u8], buffer: &mut [u16]) -> usize {
    #[inline(always)]
    fn cvt(bytes: &[u8]) -> u16 {
        let mut wide_char: u16 = 0;
        let chars_written = unsafe {
            MultiByteToWideChar(
                932,
                0,
                bytes.as_ptr() as _,
                bytes.len() as i32,
                &mut wide_char,
                1,
            )
        };

        if chars_written > 0 { wide_char } else { 0xFFFD }
    }

    // 需要边转换边过滤
    let mut out_pos = 0;
    let mut i = 0;

    while i < bytes.len() && out_pos < buffer.len() {
        let high = bytes[i];

        let converted_char = if high <= 0x7F {
            i += 1;
            high as u16
        } else if translate_utils::utils::is_sjis_high_byte(high) {
            if i + 1 < bytes.len() {
                let low = bytes[i + 1];
                if low == 0 {
                    break;
                }

                // 如果开启了`generate_full_mapping_data`特性，
                // 则mapping_data::SJIS_PHF_MAP包含了所有非ascii的映射
                // 否则仅包含替身字符的映射
                let sjis_char = ((high as u16) << 8) | (low as u16);
                if let Some(&mapped_char) = mapping_data::SJIS_PHF_MAP.get(&sjis_char) {
                    i += 2;
                    mapped_char
                } else {
                    let sjis_slice = &bytes[i..i + 2];
                    i += 2;
                    cvt(sjis_slice)
                }
            } else {
                i = bytes.len(); // 结束循环
                0xFFFD
            }
        } else {
            let sjis_slice = &bytes[i..i + 1];
            i += 1;
            cvt(sjis_slice)
        };

        // 应用过滤器，如果`constant::CHAR_FILTER`为空，编译器应该可以优化掉这个IF
        if !constant::CHAR_FILTER.contains(&converted_char) {
            buffer[out_pos] = converted_char;
            out_pos += 1;
        }
    }

    out_pos
}

/// 映射宽字符并过滤特定字符
///
/// # 参数
/// - `u16_slice`: 输入字符切片
/// - `buffer`: 输出缓冲区
///
/// # 返回值
/// 写入缓冲区的字符数量
///
/// # 注意
/// 此函数不处理UTF-16代理对，每个u16被独立处理
#[allow(dead_code)]
pub fn map_wide_chars(u16_slice: &[u16], buffer: &mut [u16]) -> usize {
    let mut out_pos = 0;

    for &ch in u16_slice {
        if out_pos >= buffer.len() {
            break; // 缓冲区满，提前退出
        }

        let mapped_ch = mapping_data::UTF16_PHF_MAP.get(&ch).copied().unwrap_or(ch);

        if !constant::CHAR_FILTER.contains(&mapped_ch) {
            buffer[out_pos] = mapped_ch;
            out_pos += 1;
        }
    }

    out_pos
}

/// 将指定shift-jis字节中的替身字符映射为指定的字符并转换为utf16
///
/// # 参数
/// - `bytes`: 输入的shift-jis字节序列
///
/// # 返回值
/// 返回转换后的utf16字符向量
#[allow(dead_code)]
pub fn map_chars_to_vec(bytes: &[u8]) -> Vec<u16> {
    let mut buffer = vec![0; bytes.len()];
    let len = map_chars(bytes, &mut buffer);
    buffer.truncate(len);
    buffer
}

/// 将指定shift-jis字节中的替身字符映射为指定的字符并转换为以null结尾的utf16
///
/// # 参数
/// - `bytes`: 输入的shift-jis字节序列
///
/// # 返回值
/// 返回转换后的以null结尾的utf16字符向量
#[allow(dead_code)]
pub fn map_chars_to_vec_with_null(bytes: &[u8]) -> Vec<u16> {
    let mut result = map_chars_to_vec(bytes);
    result.push(0);
    result
}

/// 映射宽字符并过滤特定字符
///
/// # 参数
/// - `u16_slice`: 输入字符切片
///
/// # 返回值
/// 返回映射和过滤后的字符向量
///
/// # 注意
/// 此函数不处理UTF-16代理对，每个u16被独立处理
#[allow(dead_code)]
pub fn map_wide_chars_to_vec(u16_slice: &[u16]) -> Vec<u16> {
    let mut buffer = vec![0; u16_slice.len()];
    let len = map_wide_chars(u16_slice, &mut buffer);
    buffer.truncate(len);
    buffer
}

/// 映射宽字符并过滤特定字符，返回以null结尾的向量
///
/// # 参数
/// - `u16_slice`: 输入字符切片
///
/// # 返回值
/// 返回映射和过滤后的以null结尾的字符向量
///
/// # 注意
/// 此函数不处理UTF-16代理对，每个u16被独立处理
#[allow(dead_code)]
pub fn map_wide_chars_to_vec_with_null(u16_slice: &[u16]) -> Vec<u16> {
    let mut result = map_wide_chars_to_vec(u16_slice);
    result.push(0);
    result
}
