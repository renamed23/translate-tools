use translate_macros::{detour, generate_detours};
use winapi::ctypes::c_int;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM};
use winapi::shared::windef::{HDC, HFONT, LPSIZE};
use winapi::um::wingdi::{
    CreateFontIndirectW, CreateFontW, FONTENUMPROCA, GLYPHMETRICS, GetGlyphOutlineW,
    GetTextExtentPoint32W, LOGFONTA, LOGFONTW, MAT2, TextOutW,
};
use winapi::um::winnt::LPCSTR;

use crate::constant;
use crate::debug;
use crate::mapping::map_chars;

#[generate_detours]
pub trait TextHook: Send + Sync + 'static {
    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutA",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn text_out_a(&self, hdc: HDC, x: c_int, y: c_int, lp_string: LPCSTR, c: c_int) -> BOOL {
        unsafe {
            let input_slice = core::slice::from_raw_parts(lp_string as *const u8, c as usize);
            let mut buffer = [0u16; 256];
            let written_count = map_chars(input_slice, &mut buffer);
            let result = &buffer[..written_count];

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(result) {
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
    unsafe fn get_text_extent_point_32_a(
        &self,
        hdc: HDC,
        lp_string: LPCSTR,
        c: c_int,
        lp_size: LPSIZE,
    ) -> BOOL {
        unsafe {
            let input_slice = core::slice::from_raw_parts(lp_string as *const u8, c as usize);
            let mut buffer = [0u16; 256];
            let written_count = map_chars(input_slice, &mut buffer);
            let result = &buffer[..written_count];

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(result) {
                Ok(result) => debug!("get_text_extent_point_32 result: {result}"),
                Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
            }

            GetTextExtentPoint32W(hdc, result.as_ptr(), result.len() as i32, lp_size)
        }
    }

    #[detour(dll = "gdi32.dll", symbol = "GetGlyphOutlineA", fallback = "0")]
    unsafe fn get_glyph_outline_a(
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

        let input_slice = if u_char >> 8 == 0 {
            &[b2][..]
        } else {
            &[b1, b2][..]
        };

        let mut buffer = [0u16; 2];
        let written_count = map_chars(input_slice, &mut buffer);
        let result = &buffer[..written_count];

        #[cfg(feature = "debug_text_mapping")]
        match String::from_utf16(result) {
            Ok(result) => debug!("get_glyph_outline result: {result}"),
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

    #[allow(unused_variables)]
    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontA",
        fallback = "core::ptr::null_mut()"
    )]
    unsafe fn create_font_a(
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
        let mut face_u16: Vec<u16> = constant::FONT_FACE.to_vec();
        #[cfg(feature = "enum_font_families")]
        let mut face_u16: Vec<u16> = {
            let bytes = unsafe { core::ffi::CStr::from_ptr(psz_face_name).to_bytes() };
            crate::code_cvt::ansi_font_to_wide_font(bytes)
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
        fallback = "core::ptr::null_mut()"
    )]
    unsafe fn create_font_indirect_a(&self, lplf: *const LOGFONTA) -> HFONT {
        let logfona = unsafe { &*lplf };
        let mut logfontw = unsafe { core::mem::zeroed::<LOGFONTW>() };

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
        let mut face_u16: Vec<u16> = constant::FONT_FACE.to_vec();
        #[cfg(feature = "enum_font_families")]
        let mut face_u16: Vec<u16> = {
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    logfona.lfFaceName.as_ptr() as *const u8,
                    logfona.lfFaceName.len(),
                )
            };
            let end = bytes.iter().position(|&c| c == 0).unwrap_or(31);
            crate::code_cvt::ansi_font_to_wide_font(&bytes[..end])
        };

        face_u16.push(0);

        logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());

        unsafe { CreateFontIndirectW(&logfontw) }
    }

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesExA", fallback = "0")]
    unsafe fn enum_font_families_ex_a(
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
            HOOK_ENUM_FONT_FAMILIES_EX_A.call(
                hdc,
                lp_logfont,
                lp_enum_font_fam_proc,
                l_param,
                dw_flags,
            )
        }
    }
}

/// 开启文本相关的钩子
pub fn enable_hooks() {
    unsafe {
        HOOK_CREATE_FONT_A.enable().unwrap();
        HOOK_CREATE_FONT_INDIRECT_A.enable().unwrap();
        HOOK_GET_GLYPH_OUTLINE_A.enable().unwrap();
        HOOK_TEXT_OUT_A.enable().unwrap();
        HOOK_GET_TEXT_EXTENT_POINT_32_A.enable().unwrap();
    }

    #[cfg(feature = "enum_font_families")]
    unsafe {
        HOOK_ENUM_FONT_FAMILIES_EX_A.enable().unwrap();
    }
    debug!("Text Hooked!");
}

/// 关闭文本相关的钩子
pub fn disable_hooks() {
    unsafe {
        HOOK_CREATE_FONT_A.disable().unwrap();
        HOOK_CREATE_FONT_INDIRECT_A.disable().unwrap();
        HOOK_GET_GLYPH_OUTLINE_A.disable().unwrap();
        HOOK_TEXT_OUT_A.disable().unwrap();
        HOOK_GET_TEXT_EXTENT_POINT_32_A.disable().unwrap();
    }

    #[cfg(feature = "enum_font_families")]
    unsafe {
        HOOK_ENUM_FONT_FAMILIES_EX_A.disable().unwrap();
    }
    debug!("Text Unhooked!");
}
