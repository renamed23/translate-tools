use core::ptr;
use smallvec::SmallVec;
use windows_sys::Win32::Globalization::{CP_UTF8, MultiByteToWideChar, WideCharToMultiByte};
use windows_sys::Win32::Graphics::Gdi::{
    ANSI_CHARSET, ARABIC_CHARSET, BALTIC_CHARSET, CHINESEBIG5_CHARSET, EASTEUROPE_CHARSET,
    GB2312_CHARSET, GREEK_CHARSET, HANGUL_CHARSET, HEBREW_CHARSET, RUSSIAN_CHARSET,
    SHIFTJIS_CHARSET, THAI_CHARSET, TURKISH_CHARSET, VIETNAMESE_CHARSET,
};
use windows_sys::Win32::UI::WindowsAndMessaging::CharNextExA;

use crate::constant::{ANSI_CODE_PAGE, CHAR_SET, TEXT_STACK_BUF_LEN};
use crate::print_system_error_message;

/// 用于处理文本缓冲区的Vec
pub type TextVec<T> = SmallVec<[T; TEXT_STACK_BUF_LEN]>;

/// 将字节切片转换为宽字符字符串
///
/// # 参数
/// - `bytes`: 输入的字节切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
///
/// # 返回
/// - 返回宽字符 TextVec（不以0结尾）
#[inline(always)]
pub fn multi_byte_to_wide_char(bytes: &[u8], code_page: u32) -> TextVec<u16> {
    if bytes.is_empty() {
        return TextVec::new();
    }

    unsafe {
        // 计算所需的宽字符缓冲区大小
        let wide_size = MultiByteToWideChar(
            code_page,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            ptr::null_mut(),
            0,
        );

        if wide_size == 0 {
            print_system_error_message!();
            return TextVec::new();
        }

        // 分配宽字符缓冲区
        let mut wide_buffer = TextVec::<u16>::with_capacity(wide_size as usize);
        let wide_ptr = wide_buffer.as_mut_ptr();

        // 执行实际转换
        let result = MultiByteToWideChar(
            code_page,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide_ptr,
            wide_size,
        );

        if result == 0 {
            print_system_error_message!();
            TextVec::new()
        } else {
            wide_buffer.set_len(wide_size as usize);
            wide_buffer
        }
    }
}

/// 将字节切片转换为宽字符字符串（以null结尾）
///
/// # 参数
/// - `bytes`: 输入的字节切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
///
/// # 返回
/// - 返回宽字符 TextVec（以0结尾）
#[inline(always)]
pub fn multi_byte_to_wide_char_with_null(bytes: &[u8], code_page: u32) -> TextVec<u16> {
    let mut result = multi_byte_to_wide_char(bytes, code_page);
    result.push(0x0);
    result
}

/// 将宽字符切片转换为指定代码页的字节向量
///
/// # 参数
/// - `wide_str`: 输入的宽字符切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
///
/// # 返回
/// - 返回字节 TextVec（不以0结尾）
#[inline(always)]
pub fn wide_char_to_multi_byte(wide_str: &[u16], code_page: u32) -> TextVec<u8> {
    if wide_str.is_empty() {
        return TextVec::new();
    }

    unsafe {
        // 计算所需的字节缓冲区大小
        let multi_byte_size = WideCharToMultiByte(
            code_page,
            0,
            wide_str.as_ptr(),
            wide_str.len() as i32,
            ptr::null_mut(),
            0,
            ptr::null(),
            ptr::null_mut(),
        );

        if multi_byte_size == 0 {
            print_system_error_message!();
            return TextVec::new();
        }

        // 分配字节缓冲区
        let mut multi_byte_buffer = TextVec::<u8>::with_capacity(multi_byte_size as usize);
        let multi_byte_ptr = multi_byte_buffer.as_mut_ptr();

        // 执行实际转换
        let result = WideCharToMultiByte(
            code_page,
            0,
            wide_str.as_ptr(),
            wide_str.len() as i32,
            multi_byte_ptr,
            multi_byte_size,
            ptr::null(),
            ptr::null_mut(),
        );

        if result == 0 {
            print_system_error_message!();
            TextVec::new()
        } else {
            // 设置 TextVec 长度
            multi_byte_buffer.set_len(multi_byte_size as usize);
            multi_byte_buffer
        }
    }
}

/// 将宽字符切片转换为指定代码页的字节向量（以null结尾）
///
/// # 参数
/// - `wide_str`: 输入的宽字符切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
///
/// # 返回
/// - 返回字节向量（以0结尾）
#[inline(always)]
pub fn wide_char_to_multi_byte_with_null(wide_str: &[u16], code_page: u32) -> TextVec<u8> {
    let mut result = wide_char_to_multi_byte(wide_str, code_page);
    result.push(0x0);
    result
}

/// 便捷函数：将UTF-8字节切片转换为宽字符字符串
#[inline(always)]
pub fn utf8_to_wide_char(bytes: &[u8]) -> TextVec<u16> {
    multi_byte_to_wide_char(bytes, CP_UTF8)
}

/// 便捷函数：将ANSI字节切片转换为宽字符字符串
#[inline(always)]
pub fn ansi_to_wide_char(bytes: &[u8]) -> TextVec<u16> {
    multi_byte_to_wide_char(bytes, ANSI_CODE_PAGE)
}

/// 便捷函数：将宽字符切片转换为UTF-8字节向量
#[inline(always)]
pub fn wide_char_to_utf8(wide_str: &[u16]) -> TextVec<u8> {
    wide_char_to_multi_byte(wide_str, CP_UTF8)
}

/// 便捷函数：将宽字符切片转换为ANSI字节向量
#[inline(always)]
pub fn wide_char_to_ansi(wide_str: &[u16]) -> TextVec<u8> {
    wide_char_to_multi_byte(wide_str, ANSI_CODE_PAGE)
}

/// 便捷函数：将UTF-8字节切片转换为宽字符字符串（以null结尾）
#[inline(always)]
pub fn utf8_to_wide_char_with_null(bytes: &[u8]) -> TextVec<u16> {
    multi_byte_to_wide_char_with_null(bytes, CP_UTF8)
}

/// 便捷函数：将ANSI字节切片转换为宽字符字符串（以null结尾）
#[inline(always)]
pub fn ansi_to_wide_char_with_null(bytes: &[u8]) -> TextVec<u16> {
    multi_byte_to_wide_char_with_null(bytes, ANSI_CODE_PAGE)
}

/// 便捷函数：将宽字符切片转换为UTF-8字节向量（以null结尾）
#[inline(always)]
pub fn wide_char_to_utf8_with_null(wide_str: &[u16]) -> TextVec<u8> {
    wide_char_to_multi_byte_with_null(wide_str, CP_UTF8)
}

/// 便捷函数：将宽字符切片转换为ANSI字节向量（以null结尾）
#[inline(always)]
pub fn wide_char_to_ansi_with_null(wide_str: &[u16]) -> TextVec<u8> {
    wide_char_to_multi_byte_with_null(wide_str, ANSI_CODE_PAGE)
}

/// 将 u16 切片转换为带有结尾 NULL 的新 TextVec<u16>
#[inline(always)]
pub fn u16_with_null(u16_slice: &[u16]) -> TextVec<u16> {
    u16_slice
        .iter()
        .copied()
        .chain(core::iter::once(0u16))
        .collect()
}

/// 根据CharSet获取对应的代码页
#[inline(always)]
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
#[inline(always)]
pub fn byte_len(ptr: *const u8, chars: usize, code_page: u16) -> usize {
    let mut cur = ptr;
    let mut byte_len = 0usize;

    unsafe {
        for _ in 0..chars {
            let next = CharNextExA(code_page, cur, 0) as *const u8;
            if next.is_null() {
                break;
            }
            byte_len += next.offset_from(cur) as usize;
            cur = next;
        }
    }

    byte_len
}

/// 根据字符数计算传入ANSI字符串的字节长度
#[inline(always)]
pub fn ansi_byte_len(ptr: *const u8, chars: usize) -> usize {
    byte_len(ptr, chars, ANSI_CODE_PAGE as u16)
}
