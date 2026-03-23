use translate_macros::detour_trait;
use windows_sys::core::{BOOL, PCSTR, PCWSTR, PSTR, PWSTR};

#[detour_trait]
pub trait MultiByteToWideChar {
    #[detour(dll = "kernel32.dll", symbol = "MultiByteToWideChar", fallback = "0")]
    unsafe fn multi_byte_to_wide_char(
        code_page: u32,
        dw_flags: u32,
        lp_multi_byte_str: PCSTR,
        cb_multi_byte: i32,
        lp_wide_char_str: PWSTR,
        cch_wide_char: i32,
    ) -> i32;
}

#[detour_trait]
pub trait WideCharToMultiByte {
    #[detour(dll = "kernel32.dll", symbol = "WideCharToMultiByte", fallback = "0")]
    unsafe fn wide_char_to_multi_byte(
        code_page: u32,
        dw_flags: u32,
        lp_wide_char_str: PCWSTR,
        cch_wide_char: i32,
        lp_multi_byte_str: PSTR,
        cb_multi_byte: i32,
        lp_default_char: PCSTR,
        lp_used_default_char: *mut BOOL,
    ) -> i32;
}
