use smallvec::SmallVec;
use std::borrow::Cow;
use translate_macros::{detour, generate_detours};
use winapi::ctypes::c_int;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM};
use winapi::shared::windef::{HDC, HFONT, LPSIZE};
use winapi::um::wingdi::GLYPHMETRICS;
use winapi::um::wingdi::LF_FACESIZE;
use winapi::um::wingdi::{FONTENUMPROCA, FONTENUMPROCW, LOGFONTA, LOGFONTW, MAT2};
use winapi::um::winnt::LPCSTR;
use winapi::um::winnt::LPCWSTR;

use crate::constant;
use crate::debug;
use crate::mapping::map_chars;
use crate::mapping::map_wide_chars;

#[generate_detours]
pub trait TextHook: Send + Sync + 'static {
    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutA",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn text_out_a(&self, hdc: HDC, x: c_int, y: c_int, lp_string: LPCSTR, c: c_int) -> BOOL {
        unsafe {
            let lp_string = lp_string as *const u8;
            // `slice_from_raw_parts`会进行简单的指针检查，若非法返回空切片
            let input_slice = crate::utils::slice_from_raw_parts(lp_string, c as usize);

            // 长度小于等于 `constant::TEXT_STACK_BUF_LEN` 的数据使用栈缓冲区，
            // 否则使用堆缓冲区
            let mut buf: SmallVec<[u16; constant::TEXT_STACK_BUF_LEN]> =
                SmallVec::with_capacity(input_slice.len());
            buf.resize(input_slice.len(), 0);

            let written_count = map_chars(input_slice, &mut buf);
            let slice = &buf[..written_count];

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(slice) {
                Ok(result) => debug!("draw text '{result}' at ({x}, {y})"),
                Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
            }

            HOOK_TEXT_OUT_W.call(hdc, x, y, slice.as_ptr(), slice.len() as i32)
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutW",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn text_out_w(
        &self,
        hdc: HDC,
        x: c_int,
        y: c_int,
        lp_string: LPCWSTR,
        c: c_int,
    ) -> BOOL {
        unsafe {
            let input_slice = crate::utils::slice_from_raw_parts(lp_string, c as usize);

            let mut buf: SmallVec<[u16; constant::TEXT_STACK_BUF_LEN]> =
                SmallVec::with_capacity(input_slice.len());
            buf.resize(input_slice.len(), 0);

            let written_count = map_wide_chars(input_slice, buf.as_mut());
            let slice = &buf[..written_count];

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(slice) {
                Ok(result) => debug!("draw text '{result}' at ({x}, {y})"),
                Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
            }

            HOOK_TEXT_OUT_W.call(hdc, x, y, slice.as_ptr(), slice.len() as i32)
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
            let lp_string = lp_string as *const u8;
            let input_slice = crate::utils::slice_from_raw_parts(lp_string, c as usize);

            let mut buf: SmallVec<[u16; constant::TEXT_STACK_BUF_LEN]> =
                SmallVec::with_capacity(input_slice.len());
            buf.resize(input_slice.len(), 0);

            let written_count = map_chars(input_slice, &mut buf);
            let slice = &buf[..written_count];

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(slice) {
                Ok(result) => debug!("result: {result}"),
                Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
            }

            HOOK_GET_TEXT_EXTENT_POINT_32_W.call(hdc, slice.as_ptr(), slice.len() as i32, lp_size)
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextExtentPoint32W",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn get_text_extent_point_32_w(
        &self,
        hdc: HDC,
        lp_string: LPCWSTR,
        c: c_int,
        lp_size: LPSIZE,
    ) -> BOOL {
        unsafe {
            let input_slice = crate::utils::slice_from_raw_parts(lp_string, c as usize);

            let mut buf: SmallVec<[u16; constant::TEXT_STACK_BUF_LEN]> =
                SmallVec::with_capacity(input_slice.len());
            buf.resize(input_slice.len(), 0);

            let written_count = map_wide_chars(input_slice, &mut buf);
            let slice = &buf[..written_count];

            #[cfg(feature = "debug_text_mapping")]
            match String::from_utf16(slice) {
                Ok(result) => debug!("result: {result}"),
                Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
            }

            HOOK_GET_TEXT_EXTENT_POINT_32_W.call(hdc, slice.as_ptr(), slice.len() as i32, lp_size)
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
            Ok(result) => debug!("result: {result}"),
            Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
        }

        // 直接使用第一个UTF-16字符（假设都在BMP内，不需要代理对）
        if let Some(&wchar) = result.first() {
            return unsafe {
                HOOK_GET_GLYPH_OUTLINE_W.call(
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

    #[detour(dll = "gdi32.dll", symbol = "GetGlyphOutlineW", fallback = "0")]
    unsafe fn get_glyph_outline_w(
        &self,
        hdc: HDC,
        u_char: u32,
        format: u32,
        lpgm: *mut GLYPHMETRICS,
        cb_buffer: u32,
        lpv_buffer: *mut c_void,
        lpmat2: *const MAT2,
    ) -> DWORD {
        // 假设都在BMP内，所以直接`u_char as u16`
        let mut buffer = [0u16; 2];
        let written_count = map_wide_chars(&[u_char as u16], &mut buffer);
        let result = &buffer[..written_count];

        #[cfg(feature = "debug_text_mapping")]
        match String::from_utf16(result) {
            Ok(result) => debug!("result: {result}"),
            Err(e) => debug!("Convert utf16 to utf8 fails with {e}"),
        }

        // 直接使用第一个UTF-16字符（假设都在BMP内，不需要代理对）
        if let Some(&wchar) = result.first() {
            return unsafe {
                HOOK_GET_GLYPH_OUTLINE_W.call(
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
        i_char_set: DWORD,
        i_out_precision: DWORD,
        i_clip_precision: DWORD,
        i_quality: DWORD,
        i_pitch_and_family: DWORD,
        psz_face_name: LPCSTR,
    ) -> HFONT {
        let face_u16 = {
            let bytes = unsafe {
                crate::utils::slice_until_null(psz_face_name as *const u8, LF_FACESIZE - 1)
            };
            crate::code_cvt::ansi_to_wide_char_with_null(bytes)
        };

        unsafe {
            self.create_font_w(
                c_height,
                c_width,
                c_escapement,
                c_orientation,
                c_weight,
                b_italic,
                b_underline,
                b_strike_out,
                i_char_set,
                i_out_precision,
                i_clip_precision,
                i_quality,
                i_pitch_and_family,
                face_u16.as_ptr(),
            )
        }
    }

    #[allow(unused_variables, unused_mut)]
    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontW",
        fallback = "core::ptr::null_mut()"
    )]
    unsafe fn create_font_w(
        &self,
        c_height: c_int,
        c_width: c_int,
        c_escapement: c_int,
        c_orientation: c_int,
        c_weight: c_int,
        b_italic: DWORD,
        b_underline: DWORD,
        b_strike_out: DWORD,
        i_char_set: DWORD,
        i_out_precision: DWORD,
        i_clip_precision: DWORD,
        i_quality: DWORD,
        i_pitch_and_family: DWORD,
        psz_face_name: LPCWSTR,
    ) -> HFONT {
        let mut u16_slice: Cow<[u16]>;
        #[cfg(not(feature = "enum_font_families"))]
        {
            u16_slice = Cow::from(crate::utils::u16_with_null(constant::FONT_FACE));
        }
        #[cfg(feature = "enum_font_families")]
        unsafe {
            u16_slice = Cow::from(crate::utils::slice_until_null(
                psz_face_name,
                LF_FACESIZE - 1,
            ));

            debug!(
                "Requested font name: {}",
                String::from_utf16_lossy(&u16_slice)
            );

            if constant::FONT_FILTER.contains(&&*u16_slice) {
                u16_slice = Cow::from(crate::utils::u16_with_null(constant::FONT_FACE));
            }
        };

        unsafe {
            HOOK_CREATE_FONT_W.call(
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
                u16_slice.as_ptr(),
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
        logfontw.lfCharSet = logfona.lfCharSet;
        logfontw.lfOutPrecision = logfona.lfOutPrecision;
        logfontw.lfClipPrecision = logfona.lfClipPrecision;
        logfontw.lfQuality = logfona.lfQuality;
        logfontw.lfPitchAndFamily = logfona.lfPitchAndFamily;

        let face_u16 = {
            let u8_slice = unsafe {
                crate::utils::slice_until_null(
                    logfona.lfFaceName.as_ptr() as *const u8,
                    logfona.lfFaceName.len() - 1, // 最后一个字节必须为null
                )
            };
            crate::code_cvt::ansi_to_wide_char_with_null(u8_slice)
        };

        logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());

        let ptr = &logfontw as *const LOGFONTW;
        unsafe { self.create_font_indirect_w(ptr) }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontIndirectW",
        fallback = "core::ptr::null_mut()"
    )]
    unsafe fn create_font_indirect_w(&self, lplf: *const LOGFONTW) -> HFONT {
        let mut logfontw = unsafe { *lplf };
        logfontw.lfCharSet = constant::CHAR_SET as u8;

        // `constant::FONT_FACE` 长度确保不超过 LF_FACESIZE - 1，可以直接复制
        #[cfg(not(feature = "enum_font_families"))]
        {
            let face_u16 = crate::utils::u16_with_null(constant::FONT_FACE);
            logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());
        }
        #[cfg(feature = "enum_font_families")]
        {
            let u16_slice = unsafe {
                crate::utils::slice_until_null(
                    logfontw.lfFaceName.as_ptr(),
                    logfontw.lfFaceName.len() - 1, // 最后一个字节必须为null
                )
            };

            debug!(
                "Requested font name: {}",
                String::from_utf16_lossy(u16_slice)
            );

            if constant::FONT_FILTER.contains(&u16_slice) {
                let face_u16 = crate::utils::u16_with_null(constant::FONT_FACE);
                logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());
            }
        };

        let ptr = &logfontw as *const LOGFONTW;
        unsafe { HOOK_CREATE_FONT_INDIRECT_W.call(ptr) }
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

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesExW", fallback = "0")]
    unsafe fn enum_font_families_ex_w(
        &self,
        hdc: HDC,
        lp_logfont: *mut LOGFONTW,
        lp_enum_font_fam_proc: FONTENUMPROCW,
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
            HOOK_ENUM_FONT_FAMILIES_EX_W.call(
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
pub fn enable_featured_hooks() {
    unsafe {
        HOOK_CREATE_FONT_A.enable().unwrap();
        HOOK_CREATE_FONT_INDIRECT_A.enable().unwrap();
        HOOK_GET_GLYPH_OUTLINE_A.enable().unwrap();
        HOOK_TEXT_OUT_A.enable().unwrap();
        HOOK_GET_TEXT_EXTENT_POINT_32_A.enable().unwrap();

        // W版本钩子
        HOOK_CREATE_FONT_W.enable().unwrap();
        HOOK_CREATE_FONT_INDIRECT_W.enable().unwrap();
        HOOK_GET_GLYPH_OUTLINE_W.enable().unwrap();
        HOOK_TEXT_OUT_W.enable().unwrap();
        HOOK_GET_TEXT_EXTENT_POINT_32_W.enable().unwrap();
    }

    #[cfg(feature = "enum_font_families")]
    unsafe {
        HOOK_ENUM_FONT_FAMILIES_EX_A.enable().unwrap();
        HOOK_ENUM_FONT_FAMILIES_EX_W.enable().unwrap();
    }
    debug!("Text Hooked!");
}

/// 关闭文本相关的钩子
pub fn disable_featured_hooks() {
    unsafe {
        HOOK_CREATE_FONT_A.disable().unwrap();
        HOOK_CREATE_FONT_INDIRECT_A.disable().unwrap();
        HOOK_GET_GLYPH_OUTLINE_A.disable().unwrap();
        HOOK_TEXT_OUT_A.disable().unwrap();
        HOOK_GET_TEXT_EXTENT_POINT_32_A.disable().unwrap();

        // W版本钩子
        HOOK_CREATE_FONT_W.disable().unwrap();
        HOOK_CREATE_FONT_INDIRECT_W.disable().unwrap();
        HOOK_GET_GLYPH_OUTLINE_W.disable().unwrap();
        HOOK_TEXT_OUT_W.disable().unwrap();
        HOOK_GET_TEXT_EXTENT_POINT_32_W.disable().unwrap();
    }

    #[cfg(feature = "enum_font_families")]
    unsafe {
        HOOK_ENUM_FONT_FAMILIES_EX_A.disable().unwrap();
        HOOK_ENUM_FONT_FAMILIES_EX_W.disable().unwrap();
    }
    debug!("Text Unhooked!");
}
