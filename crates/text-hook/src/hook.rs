use once_cell::sync::OnceCell;
use std::os::raw::c_int;
use translate_macros::{detour, generate_detours};
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, DWORD, FARPROC, HMODULE, LPARAM, LPDWORD, LPVOID};
use winapi::shared::ntdef::HANDLE;
use winapi::shared::windef::{HDC, HFONT, LPSIZE};
use winapi::um::minwinbase::{LPOVERLAPPED, LPSECURITY_ATTRIBUTES};
use winapi::um::wingdi::{
    CreateFontIndirectW, CreateFontW, FONTENUMPROCA, GLYPHMETRICS, GetGlyphOutlineW,
    GetTextExtentPoint32W, LOGFONTA, LOGFONTW, MAT2, TextOutW,
};
use winapi::um::winnt::LPCSTR;

use crate::constant;
use crate::debug;
use crate::mapping::map_shift_jis_to_unicode;

#[generate_detours]
pub trait Hook: Send + Sync + 'static {
    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutA",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn text_out(&self, hdc: HDC, x: c_int, y: c_int, lp_string: LPCSTR, c: c_int) -> BOOL {
        if lp_string.is_null() || c <= 0 {
            return 0;
        }

        unsafe {
            let input_slice = std::slice::from_raw_parts(lp_string as *const u8, c as usize);
            let result = map_shift_jis_to_unicode(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(result.as_slice()) {
                Ok(result) => debug!("draw text '{result}' at ({x}, {y})"),
                Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
            }

            TextOutW(hdc, x, y, result.as_ptr(), result.len() as i32)
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextExtentPoint32A",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn get_text_extent_point_32(
        &self,
        hdc: HDC,
        lp_string: LPCSTR,
        c: c_int,
        lp_size: LPSIZE,
    ) -> BOOL {
        if lp_string.is_null() || lp_size.is_null() || c <= 0 {
            return 0;
        }

        unsafe {
            let input_slice = std::slice::from_raw_parts(lp_string as *const u8, c as usize);
            let result = map_shift_jis_to_unicode(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(result.as_slice()) {
                Ok(result) => debug!("result: {result}"),
                Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
            }

            GetTextExtentPoint32W(hdc, result.as_ptr(), result.len() as i32, lp_size)
        }
    }

    #[detour(dll = "gdi32.dll", symbol = "GetGlyphOutlineA", fallback = "0")]
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

        let bytes = if u_char >> 8 == 0 {
            vec![b2]
        } else {
            vec![b1, b2]
        };

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

    #[allow(dead_code)]
    #[detour(
        dll = "kernel32.dll",
        symbol = "GetProcAddress",
        fallback = "std::ptr::null_mut()"
    )]
    unsafe fn get_proc_address(&self, _hmod: HMODULE, _proc_name: LPCSTR) -> FARPROC {
        unimplemented!();
    }

    #[allow(unused_variables)]
    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontA",
        fallback = "std::ptr::null_mut()"
    )]
    unsafe fn create_font(
        &self,
        c_height: c_int,
        c_width: c_int,
        c_escapement: c_int,
        c_orientation: c_int,
        c_weight: c_int,
        b_italic: DWORD,
        b_underline: DWORD,
        b_strike_out: DWORD,
        _i_char_set: DWORD,
        i_out_precision: DWORD,
        i_clip_precision: DWORD,
        i_quality: DWORD,
        i_pitch_and_family: DWORD,
        psz_face_name: LPCSTR,
    ) -> HFONT {
        #[cfg(not(feature = "enum_font_families"))]
        let mut face_u16: Vec<u16> = constant::FONT_FACE.encode_utf16().collect();
        #[cfg(feature = "enum_font_families")]
        let mut face_u16: Vec<u16> = {
            let bytes = unsafe { std::ffi::CStr::from_ptr(psz_face_name).to_bytes() };
            crate::code_cvt::ansi_to_wide_char(bytes)
        };

        face_u16.push(0);

        unsafe {
            CreateFontW(
                c_height,
                c_width,
                c_escapement,
                c_orientation,
                c_weight,
                b_italic,
                b_underline,
                b_strike_out,
                constant::CHAR_SET,
                i_out_precision,
                i_clip_precision,
                i_quality,
                i_pitch_and_family,
                face_u16.as_ptr(),
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontIndirectA",
        fallback = "std::ptr::null_mut()"
    )]
    unsafe fn create_font_indirect(&self, lplf: *const LOGFONTA) -> HFONT {
        let logfona = unsafe { &*lplf };
        let mut logfontw = unsafe { std::mem::zeroed::<LOGFONTW>() };

        logfontw.lfHeight = logfona.lfHeight;
        logfontw.lfWidth = logfona.lfWidth;
        logfontw.lfEscapement = logfona.lfEscapement;
        logfontw.lfOrientation = logfona.lfOrientation;
        logfontw.lfWeight = logfona.lfWeight;
        logfontw.lfItalic = logfona.lfItalic;
        logfontw.lfUnderline = logfona.lfUnderline;
        logfontw.lfStrikeOut = logfona.lfStrikeOut;
        logfontw.lfCharSet = constant::CHAR_SET as u8;
        logfontw.lfOutPrecision = logfona.lfOutPrecision;
        logfontw.lfClipPrecision = logfona.lfClipPrecision;
        logfontw.lfQuality = logfona.lfQuality;
        logfontw.lfPitchAndFamily = logfona.lfPitchAndFamily;

        #[cfg(not(feature = "enum_font_families"))]
        let mut face_u16: Vec<u16> = constant::FONT_FACE.encode_utf16().collect();
        #[cfg(feature = "enum_font_families")]
        let mut face_u16: Vec<u16> = {
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    logfona.lfFaceName.as_ptr() as *const u8,
                    logfona.lfFaceName.len(),
                )
            };
            let end = bytes.iter().position(|&c| c == 0).unwrap_or(31);
            crate::code_cvt::ansi_to_wide_char(&bytes[..end])
        };

        face_u16.push(0);

        logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());

        unsafe { CreateFontIndirectW(&logfontw) }
    }

    #[allow(unused_variables, dead_code)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesExA", fallback = "0")]
    unsafe fn enum_font_families_ex(
        &self,
        hdc: HDC,
        lp_logfont: *mut LOGFONTA,
        lp_enum_font_fam_proc: FONTENUMPROCA,
        l_param: LPARAM,
        dw_flags: DWORD,
    ) -> c_int {
        #[cfg(not(feature = "enum_font_families"))]
        unimplemented!();

        #[cfg(feature = "enum_font_families")]
        unsafe {
            if let Some(font) = lp_logfont.as_mut() {
                font.lfCharSet = constant::CHAR_SET as u8;
            }
            HOOK_ENUM_FONT_FAMILIES_EX.call(
                hdc,
                lp_logfont,
                lp_enum_font_fam_proc,
                l_param,
                dw_flags,
            )
        }
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "CreateFileA",
        fallback = "winapi::um::handleapi::INVALID_HANDLE_VALUE"
    )]
    unsafe fn create_file(
        &self,
        _lp_file_name: LPCSTR,
        _dw_desired_access: DWORD,
        _dw_share_mode: DWORD,
        _lp_security_attributes: LPSECURITY_ATTRIBUTES,
        _dw_creation_disposition: DWORD,
        _dw_flags_and_attributes: DWORD,
        _h_template_file: HANDLE,
    ) -> HANDLE {
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "ReadFile",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn read_file(
        &self,
        _h_file: HANDLE,
        _lp_buffer: LPVOID,
        _n_number_of_bytes_to_read: DWORD,
        _lp_number_of_bytes_read: LPDWORD,
        _lp_overlapped: LPOVERLAPPED,
    ) -> BOOL {
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "CloseHandle",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn close_handle(&self, _h_object: HANDLE) -> BOOL {
        unimplemented!();
    }
}

/// 默认实现的钩子，应该可以应对大部分场景
#[allow(dead_code)]
pub struct DefaultHook;

impl Hook for DefaultHook {}

static HOOK_INSTANCE: OnceCell<Box<dyn Hook>> = OnceCell::new();

/// 设置全局钩子实例
#[allow(dead_code)]
pub fn set_hook_instance(h: Box<dyn Hook>) {
    if HOOK_INSTANCE.set(h).is_err() {
        debug!("Hook instance already set");
    }
}

/// 获取全局钩子实例
pub fn hook_instance() -> &'static dyn Hook {
    HOOK_INSTANCE
        .get()
        .map(|b| &**b)
        .expect("Hook not initialized")
}

/// 开启文本相关的钩子
#[allow(dead_code)]
pub fn enable_text_hooks() {
    unsafe {
        crate::hook::HOOK_CREATE_FONT.enable().unwrap();
        crate::hook::HOOK_CREATE_FONT_INDIRECT.enable().unwrap();
        crate::hook::HOOK_GET_GLYPH_OUTLINE.enable().unwrap();
        crate::hook::HOOK_TEXT_OUT.enable().unwrap();
        crate::hook::HOOK_GET_TEXT_EXTENT_POINT_32.enable().unwrap();
    }

    #[cfg(feature = "enum_font_families")]
    unsafe {
        crate::hook::HOOK_ENUM_FONT_FAMILIES_EX.enable().unwrap();
    }
    crate::debug!("Text Hooked!");
}

/// 开启文件相关的钩子
#[allow(dead_code)]
pub fn enable_file_hooks() {
    unsafe {
        crate::hook::HOOK_CREATE_FILE.enable().unwrap();
        crate::hook::HOOK_READ_FILE.enable().unwrap();
        crate::hook::HOOK_CLOSE_HANDLE.enable().unwrap();
    }

    crate::debug!("File Hooked!");
}
