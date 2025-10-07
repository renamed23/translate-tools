use std::ptr;
use winapi::um::stringapiset::{MultiByteToWideChar, WideCharToMultiByte};
use winapi::um::winnls::{CP_ACP, CP_UTF8};

use crate::print_system_error_message;

/// 将字节切片转换为宽字符字符串
///
/// # 参数
/// - `bytes`: 输入的字节切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
///
/// # 返回
/// - 成功: 返回宽字符向量（不以0结尾）
/// - 失败: 返回空向量
#[allow(dead_code)]
pub fn multi_byte_to_wide_char(bytes: &[u8], code_page: u32) -> Vec<u16> {
    if bytes.is_empty() {
        return Vec::new();
    }

    unsafe {
        // 计算所需的宽字符缓冲区大小
        let wide_size = MultiByteToWideChar(
            code_page,
            0, // flags
            bytes.as_ptr() as *const i8,
            bytes.len() as i32,
            ptr::null_mut(),
            0,
        );

        if wide_size == 0 {
            print_system_error_message!();
            return Vec::new();
        }

        // 分配宽字符缓冲区
        let mut wide_buffer = Vec::<u16>::with_capacity(wide_size as usize);
        let wide_ptr = wide_buffer.as_mut_ptr();

        // 执行实际转换
        let result = MultiByteToWideChar(
            code_page,
            0,
            bytes.as_ptr() as *const i8,
            bytes.len() as i32,
            wide_ptr,
            wide_size,
        );

        if result == 0 {
            print_system_error_message!();
            Vec::new()
        } else {
            // 设置向量长度
            wide_buffer.set_len(wide_size as usize);
            wide_buffer
        }
    }
}

/// 将宽字符切片转换为指定代码页的字节向量
///
/// # 参数
/// - `wide_str`: 输入的宽字符切片（不以0结尾）
/// - `code_page`: 代码页，如 CP_ACP、CP_UTF8 等
///
/// # 返回
/// - 成功: 返回字节向量（不以0结尾）
/// - 失败: 返回空向量
#[allow(dead_code)]
pub fn wide_char_to_multi_byte(wide_str: &[u16], code_page: u32) -> Vec<u8> {
    if wide_str.is_empty() {
        return Vec::new();
    }

    unsafe {
        // 计算所需的字节缓冲区大小
        let multi_byte_size = WideCharToMultiByte(
            code_page,
            0, // flags
            wide_str.as_ptr(),
            wide_str.len() as i32,
            ptr::null_mut(),
            0,
            ptr::null(),
            ptr::null_mut(),
        );

        if multi_byte_size == 0 {
            print_system_error_message!();
            return Vec::new();
        }

        // 分配字节缓冲区
        let mut multi_byte_buffer = Vec::<u8>::with_capacity(multi_byte_size as usize);
        let multi_byte_ptr = multi_byte_buffer.as_mut_ptr();

        // 执行实际转换
        let result = WideCharToMultiByte(
            code_page,
            0,
            wide_str.as_ptr(),
            wide_str.len() as i32,
            multi_byte_ptr as *mut i8,
            multi_byte_size,
            ptr::null(),
            ptr::null_mut(),
        );

        if result == 0 {
            print_system_error_message!();
            Vec::new()
        } else {
            // 设置向量长度
            multi_byte_buffer.set_len(multi_byte_size as usize);
            multi_byte_buffer
        }
    }
}

/// 便捷函数：将UTF-8字节切片转换为宽字符字符串
#[allow(dead_code)]
pub fn utf8_to_wide_char(bytes: &[u8]) -> Vec<u16> {
    multi_byte_to_wide_char(bytes, CP_UTF8)
}

/// 便捷函数：将ANSI字节切片转换为宽字符字符串
#[allow(dead_code)]
pub fn ansi_to_wide_char(bytes: &[u8]) -> Vec<u16> {
    multi_byte_to_wide_char(bytes, CP_ACP)
}

/// 便捷函数：将宽字符切片转换为UTF-8字节向量
#[allow(dead_code)]
pub fn wide_char_to_utf8(wide_str: &[u16]) -> Vec<u8> {
    wide_char_to_multi_byte(wide_str, CP_UTF8)
}

/// 便捷函数：将宽字符切片转换为ANSI字节向量
#[allow(dead_code)]
pub fn wide_char_to_ansi(wide_str: &[u16]) -> Vec<u8> {
    wide_char_to_multi_byte(wide_str, CP_ACP)
}
