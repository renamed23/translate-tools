use translate_macros::detour_trait;
use windows_sys::core::{BOOL, PCSTR, PCWSTR, PSTR, PWSTR};

#[detour_trait]
pub trait CodeCvtHook: Send + Sync + 'static {
    #[detour(dll = "kernel32.dll", symbol = "MultiByteToWideChar", fallback = "0")]
    unsafe fn multi_byte_to_wide_char(
        _code_page: u32,
        _dw_flags: u32,
        _lp_multi_byte_str: PCSTR,
        _cb_multi_byte: i32,
        _lp_wide_char_str: PWSTR,
        _cch_wide_char: i32,
    ) -> i32 {
        unimplemented!();
    }

    #[detour(dll = "kernel32.dll", symbol = "WideCharToMultiByte", fallback = "0")]
    unsafe fn wide_char_to_multi_byte(
        _code_page: u32,
        _dw_flags: u32,
        _lp_wide_char_str: PCWSTR,
        _cch_wide_char: i32,
        _lp_multi_byte_str: PSTR,
        _cb_multi_byte: i32,
        _lp_default_char: PCSTR,
        _lp_used_default_char: *mut BOOL,
    ) -> i32 {
        unimplemented!();
    }
}
