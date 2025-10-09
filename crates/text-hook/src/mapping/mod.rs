#[cfg(not(feature = "shift_bin"))]
mod mapping_data;

/// 默认的MAPPING实现
#[cfg(not(feature = "shift_bin"))]
mod mapping_impl {
    use crate::mapping::mapping_data;

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
}

/// VNTEXT的MAPPING实现
#[cfg(feature = "shift_bin")]
mod mapping_impl {
    use once_cell::sync::Lazy;
    use translate_utils::sjis_tunnel::SjisTunnel;

    /// 惰性静态映射表：在第一次需要时解析 SJIT_EXT_FILE 并转换为 Vec<u16>（小端）
    static TUNNEL: Lazy<SjisTunnel> = Lazy::new(|| {
        translate_macros::flate!(
            static SJIT_EXT_FILE: [u8] from "assets\\sjis_ext.bin"
        );

        SjisTunnel::from_mapping_table(SJIT_EXT_FILE.as_slice()).expect("无效的table字节")
    });

    /// Decode SJIS bytes (可能包含 tunnel 两字节)
    pub(super) fn mapping(input: &[u8]) -> Vec<u16> {
        TUNNEL.decode(input)
    }
}

/// 将指定shift-jis字节中的替身字符映射为指定的字符并转换为utf16 String
pub fn map_shift_jis_to_unicode(bytes: &[u8]) -> Vec<u16> {
    mapping_impl::mapping(bytes)
}
