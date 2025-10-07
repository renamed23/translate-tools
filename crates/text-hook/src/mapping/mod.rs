#[cfg(not(feature = "shift_bin"))]
mod mapping_data;

/// 默认的MAPPING实现
#[cfg(not(feature = "shift_bin"))]
mod mapping_impl {
    use crate::mapping::mapping_data;
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    /// 映射数据(shift-jis码点 -> utf16码点)
    static SJIS_MAPPING: Lazy<HashMap<u16, u16>> = Lazy::new(mapping_data::new);

    pub(super) fn mapping(bytes: &[u8]) -> Vec<u16> {
        translate_utils::utils::mapping(bytes, |char: u16| SJIS_MAPPING.get(&char).copied())
    }
}

/// VNTEXT的MAPPING实现
#[cfg(feature = "shift_bin")]
mod mapping_impl {
    use once_cell::sync::Lazy;
    use translate_utils::sjis_tunnel::SjisTunnel;

    /// 惰性静态映射表：在第一次需要时解析 SJIT_EXT_FILE 并转换为 Vec<u16>（小端）
    static TUNNEL: Lazy<SjisTunnel> = Lazy::new(|| {
        include_flate::flate!(
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
