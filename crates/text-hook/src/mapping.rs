use crate::{code_cvt::TextVec, constant::CHAR_FILTER};

mod mapping_data {
    translate_macros::generate_mapping_data!("assets/mapping.json");
}

/// 重导出的`ANSI_CODE_PAGE`，请使用`constant::ANSI_CODE_PAGE`而不是这个
pub const ANSI_CODE_PAGE: u32 = mapping_data::ANSI_CODE_PAGE;

/// 将含有替身字符的多字节序列转换为u16序列，然后应用映射，将替身字符转换为正常字符
#[inline(always)]
pub fn map_chars(input_slice: &[u8]) -> TextVec<u16> {
    map_wide_chars(&crate::code_cvt::ansi_to_wide_char(input_slice))
}

/// 对含有替身字符的u16序列应用映射，将替身字符转换为正常字符
#[inline(always)]
pub fn map_wide_chars(input_slice: &[u16]) -> TextVec<u16> {
    let mut buf: TextVec<u16> = TextVec::with_capacity(input_slice.len());

    for &ch in input_slice {
        let mapped_ch = mapping_data::PHF_MAP.get(&ch).copied().unwrap_or(ch);

        if !CHAR_FILTER.contains(&mapped_ch) {
            buf.push(mapped_ch);
        }
    }

    buf
}

/// 将含有替身字符的多字节序列转换为u16序列，然后应用映射，将替身字符转换为正常字符，并以null结尾
#[inline(always)]
pub fn map_chars_with_null(input_slice: &[u8]) -> TextVec<u16> {
    let mut buf = map_chars(input_slice);
    buf.push(0);
    buf
}

/// 对含有替身字符的u16序列应用映射，将替身字符转换为正常字符，并以null结尾
#[inline(always)]
pub fn map_wide_chars_with_null(input_slice: &[u16]) -> TextVec<u16> {
    let mut buf = map_wide_chars(input_slice);
    buf.push(0);
    buf
}
