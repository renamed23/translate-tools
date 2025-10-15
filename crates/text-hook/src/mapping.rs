use crate::constant;

mod mapping_data {
    include!(concat!(env!("OUT_DIR"), "/mapping_data.rs"));
}

use winapi::um::stringapiset::MultiByteToWideChar;
pub(super) fn mapping(bytes: &[u8]) -> Vec<u16> {
    let mut out_utf16 = Vec::with_capacity(bytes.len() * 2);
    let mut i = 0;

    let mut wide_char: u16 = 0;

    while i < bytes.len() {
        let high = bytes[i];

        if high <= 0x7F {
            out_utf16.push(high as u16);
            i += 1;
            continue;
        }

        let sjis_slice: &[u8];

        if translate_utils::utils::is_sjis_high_byte(high) {
            if i + 1 >= bytes.len() {
                out_utf16.push(0xFFFD);
                break;
            }
            let low = bytes[i + 1];
            if low == 0 {
                break;
            }

            // 如果开启了`generate_full_mapping_data`特性，则mapping_data::SJIS_PHF_MAP包含了所有非ascii的映射
            // 否则仅包含替身字符的映射
            let sjis_char = ((high as u16) << 8) | (low as u16);
            if let Some(&mapped_char) = mapping_data::SJIS_PHF_MAP.get(&sjis_char) {
                out_utf16.push(mapped_char);
                i += 2;
                continue;
            }

            sjis_slice = &bytes[i..i + 2];
            i += 2;
        } else {
            sjis_slice = &bytes[i..i + 1];
            i += 1;
        }

        let chars_written = unsafe {
            MultiByteToWideChar(
                932,
                0,
                sjis_slice.as_ptr() as _,
                sjis_slice.len() as i32,
                &mut wide_char,
                1,
            )
        };

        if chars_written > 0 {
            out_utf16.push(wide_char);
        } else {
            out_utf16.push(0xFFFD);
        }
    }

    out_utf16
}

/// 将指定shift-jis字节中的替身字符映射为指定的字符并转换为utf16 String
#[allow(clippy::const_is_empty)]
pub fn map_shift_jis_to_unicode(bytes: &[u8]) -> Vec<u16> {
    if constant::CHAR_FILTER.is_empty() {
        mapping(bytes)
    } else {
        mapping(bytes)
            .into_iter()
            .filter(|c| !constant::CHAR_FILTER.contains(c))
            .collect()
    }
}
