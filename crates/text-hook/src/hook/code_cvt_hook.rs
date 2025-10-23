use translate_macros::{detour, generate_detours};
use winapi::ctypes::c_int;
use winapi::shared::minwindef::{DWORD, LPBOOL, UINT};
use winapi::shared::ntdef::{LPCSTR, LPCWSTR, LPSTR, LPWSTR};

#[cfg(feature = "emulate_locale")]
use winapi::um::winnls::CP_ACP;

use crate::debug;

#[generate_detours]
pub trait CodeCvtHook: Send + Sync + 'static {
    #[detour(dll = "kernel32.dll", symbol = "MultiByteToWideChar", fallback = "0")]
    unsafe fn multi_byte_to_wide_char(
        &self,
        code_page: UINT,
        dw_flags: DWORD,
        lp_multi_byte_str: LPCSTR,
        cb_multi_byte: c_int,
        lp_wide_char_str: LPWSTR,
        cch_wide_char: c_int,
    ) -> c_int {
        #[cfg(not(feature = "emulate_locale"))]
        unimplemented!();

        #[cfg(feature = "emulate_locale")]
        unsafe {
            let code_page = if code_page == CP_ACP { 932 } else { code_page };

            HOOK_MULTI_BYTE_TO_WIDE_CHAR.call(
                code_page,
                dw_flags,
                lp_multi_byte_str,
                cb_multi_byte,
                lp_wide_char_str,
                cch_wide_char,
            )
        }
    }

    #[detour(dll = "kernel32.dll", symbol = "WideCharToMultiByte", fallback = "0")]
    unsafe fn wide_char_to_multi_byte(
        &self,
        code_page: UINT,
        dw_flags: DWORD,
        lp_wide_char_str: LPCWSTR,
        cch_wide_char: c_int,
        lp_multi_byte_str: LPSTR,
        cb_multi_byte: c_int,
        lp_default_char: LPCSTR,
        lp_used_default_char: LPBOOL,
    ) -> c_int {
        #[cfg(not(feature = "emulate_locale"))]
        unimplemented!();

        #[cfg(feature = "emulate_locale")]
        unsafe {
            let code_page = if code_page == CP_ACP { 932 } else { code_page };

            HOOK_WIDE_CHAR_TO_MULTI_BYTE.call(
                code_page,
                dw_flags,
                lp_wide_char_str,
                cch_wide_char,
                lp_multi_byte_str,
                cb_multi_byte,
                lp_default_char,
                lp_used_default_char,
            )
        }
    }
}

/// 开启字符编码转换相关的特性钩子
#[allow(dead_code)]
pub fn enable_featured_hooks() {
    #[cfg(feature = "emulate_locale")]
    unsafe {
        HOOK_MULTI_BYTE_TO_WIDE_CHAR.enable().unwrap();
        HOOK_WIDE_CHAR_TO_MULTI_BYTE.enable().unwrap();
    }

    debug!("Code Conversion Hooked!");
}

/// 关闭字符编码转换相关的特性钩子
#[allow(dead_code)]
pub fn disable_featured_hooks() {
    #[cfg(feature = "emulate_locale")]
    unsafe {
        HOOK_MULTI_BYTE_TO_WIDE_CHAR.disable().unwrap();
        HOOK_WIDE_CHAR_TO_MULTI_BYTE.disable().unwrap();
    }

    debug!("Code Conversion Unhooked!");
}
