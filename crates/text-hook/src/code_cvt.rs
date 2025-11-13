use core::ptr;
use smallvec::SmallVec;
use windows_sys::Win32::Globalization::{CP_UTF8, MultiByteToWideChar, WideCharToMultiByte};

use crate::constant::{ANSI_CODE_PAGE, TEXT_STACK_BUF_LEN};
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
