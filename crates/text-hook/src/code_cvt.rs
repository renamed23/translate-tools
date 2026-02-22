use windows_sys::Win32::Globalization::{MultiByteToWideChar, WideCharToMultiByte};
use windows_sys::Win32::Graphics::Gdi::{
    ANSI_CHARSET, ARABIC_CHARSET, BALTIC_CHARSET, CHINESEBIG5_CHARSET, EASTEUROPE_CHARSET,
    GB2312_CHARSET, GREEK_CHARSET, HANGUL_CHARSET, HEBREW_CHARSET, RUSSIAN_CHARSET,
    SHIFTJIS_CHARSET, THAI_CHARSET, TURKISH_CHARSET, VIETNAMESE_CHARSET,
};
use windows_sys::Win32::UI::WindowsAndMessaging::CharNextExA;

use crate::constant::{CHAR_FILTER, CHAR_SET};

mod mapping_data {
    translate_macros::generate_mapping_data!("assets/mapping.json");
}

/// 重导出的`ANSI_CODE_PAGE`，请使用`constant::ANSI_CODE_PAGE`而不是这个
pub const ANSI_CODE_PAGE: u32 = mapping_data::ANSI_CODE_PAGE;

/// 对含有替身字符的u16序列应用映射，将替身字符转换为正常字符
/// 仅支持BMP序列
pub fn mapping_impl(input_slice: &[u16], add_null: bool) -> Vec<u16> {
    let capacity = if add_null {
        input_slice.len() + 1
    } else {
        input_slice.len()
    };

    let mut buf: Vec<u16> = Vec::with_capacity(capacity);

    for &ch in input_slice {
        let mapped_ch = mapping_data::PHF_MAP.get(&ch).copied().unwrap_or(ch);

        if !CHAR_FILTER.contains(&mapped_ch) {
            buf.push(mapped_ch);
        }
    }

    if add_null {
        buf.push(0);
    }

    buf
}

/// 将字节切片转换为宽字符字符串
///
/// # 参数
/// - `bytes`: 输入的字节切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
/// - `add_null`: 是否在结果末尾添加0结尾符
///
/// # 返回
/// - 返回宽字符 Vec
pub fn multi_byte_to_wide_char_impl(
    bytes: &[u8],
    code_page: u32,
    add_null: bool,
) -> crate::Result<Vec<u16>> {
    crate::utils::win32::fetch_win32_string(add_null, |ptr, size| unsafe {
        MultiByteToWideChar(
            code_page,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            ptr,
            size as i32,
        ) as u32
    })
}

/// 将宽字符切片转换为指定代码页的字节向量
///
/// # 参数
/// - `wide_str`: 输入的宽字符切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
/// - `add_null`: 是否在结果末尾添加0结尾符
///
/// # 返回
/// - 返回字节 Vec
pub fn wide_char_to_multi_byte_impl(
    wide_str: &[u16],
    code_page: u32,
    add_null: bool,
) -> crate::Result<Vec<u8>> {
    crate::utils::win32::fetch_win32_string(add_null, |ptr, size| unsafe {
        WideCharToMultiByte(
            code_page,
            0,
            wide_str.as_ptr(),
            wide_str.len() as i32,
            ptr,
            size as i32,
            core::ptr::null(),
            core::ptr::null_mut(),
        ) as u32
    })
}

/// 根据CharSet获取对应的代码页
pub const fn get_cp_by_char_set() -> u32 {
    match CHAR_SET {
        // 东亚语言（中日韩）
        GB2312_CHARSET => 936,      // 简体中文
        SHIFTJIS_CHARSET => 932,    // 日语
        HANGUL_CHARSET => 949,      // 韩语
        CHINESEBIG5_CHARSET => 950, // 繁体中文

        // 西欧及东欧
        ANSI_CHARSET => 1252,       // 西欧
        EASTEUROPE_CHARSET => 1250, // 东欧
        RUSSIAN_CHARSET => 1251,    // 西里尔文
        GREEK_CHARSET => 1253,      // 希腊语
        TURKISH_CHARSET => 1254,    // 土耳其语
        BALTIC_CHARSET => 1257,     // 波罗的海

        // 中东
        HEBREW_CHARSET => 1255, // 希伯来语
        ARABIC_CHARSET => 1256, // 阿拉伯语

        // 亚洲其他
        THAI_CHARSET => 874,        // 泰语
        VIETNAMESE_CHARSET => 1258, // 越南语

        _ => 0,
    }
}

/// 根据字符数和代码页计算传入字符串的字节长度
pub fn byte_len(ptr: *const u8, chars: usize, code_page: u16) -> usize {
    let mut cur = ptr;
    let mut byte_len = 0usize;

    unsafe {
        for _ in 0..chars {
            let next = CharNextExA(code_page, cur, 0);
            if next.is_null() {
                break;
            }
            byte_len += next.offset_from(cur) as usize;
            cur = next;
        }
    }

    byte_len
}
