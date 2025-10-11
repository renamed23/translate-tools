use translate_macros::ffi_catch_unwind;
use winapi::{
    ctypes::c_void,
    shared::{
        minwindef::{BOOL, DWORD, FALSE, HMODULE, LPVOID, TRUE},
        windef::HDC,
    },
    um::wingdi::{GLYPHMETRICS, GetGlyphOutlineW, MAT2},
};

use crate::{hook::Hook, mapping::map_shift_jis_to_unicode};

/// 默认实现的钩子，应该可以应对大部分场景
pub struct BittersweetFools;

impl Hook for BittersweetFools {
    unsafe fn get_glyph_outline(
        &self,
        hdc: HDC,
        u_char: u32,
        format: u32,
        lpgm: *mut GLYPHMETRICS,
        cb_buffer: u32,
        lpv_buffer: *mut c_void,
        lpmat2: *const MAT2,
    ) -> DWORD {
        let b1 = ((u_char >> 8) & 0xFF) as u8;
        let b2 = (u_char & 0xFF) as u8;

        let mut bytes = if u_char >> 8 == 0 {
            vec![b2]
        } else {
            vec![b1, b2]
        };

        if bytes.len() == 1 && bytes[0] == b'}' {
            bytes[0] = b' '
        }

        let result = map_shift_jis_to_unicode(&bytes);

        #[cfg(feature = "debug_text_mapping")]
        match String::from_utf16(result.as_slice()) {
            Ok(result) => debug!("result: {result}"),
            Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
        }

        // 直接使用第一个UTF-16字符（假设都在BMP内，不需要代理对）
        if let Some(&wchar) = result.first() {
            return unsafe {
                GetGlyphOutlineW(
                    hdc,
                    wchar as u32,
                    format,
                    lpgm,
                    cb_buffer,
                    lpv_buffer,
                    lpmat2,
                )
            };
        }

        0
    }
}

#[ffi_catch_unwind(FALSE)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HMODULE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
) -> BOOL {
    const PROCESS_ATTACH: DWORD = 1;
    if fdw_reason == PROCESS_ATTACH {
        crate::panic_utils::set_debug_panic_hook();
        crate::hook::set_hook_instance(BittersweetFools);
        crate::hook::enable_text_hooks();
    }

    TRUE
}
