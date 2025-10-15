use winapi::um::stringapiset::MultiByteToWideChar;

use crate::constant;

mod mapping_data {
    include!(concat!(env!("OUT_DIR"), "/mapping_data.rs"));
}

/// 将指定shift-jis字节中的替身字符映射为指定的字符并转换为utf16
///
/// # 参数
/// - `bytes`: 输入的shift-jis字节序列
/// - `buffer`: 输出缓冲区，用于存储转换后的utf16字符
///
/// # 返回值
/// 返回实际写入缓冲区的字符数量。如果缓冲区太小，会进行截断。
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
