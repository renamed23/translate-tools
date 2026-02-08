use translate_macros::detour_trait;
use windows_sys::{
    Win32::{
        Foundation::{LPARAM, RECT, SIZE},
        Graphics::Gdi::{
            FONTENUMPROCA, FONTENUMPROCW, GLYPHMETRICS, HDC, HFONT, LF_FACESIZE, LOGFONTA,
            LOGFONTW, MAT2,
        },
    },
    core::{BOOL, PCSTR, PCWSTR},
};

use crate::debug;
use crate::{
    code_cvt::TextVec,
    constant::{CHAR_SET, FONT_FACE, FONT_FILTER},
};

#[cfg(feature = "enum_font_families")]
use crate::hook::trait_impls::enum_font_proc::{
    EnumFontInfo, enum_fonts_proc_a, enum_fonts_proc_w,
};

#[detour_trait]
pub trait TextHook: Send + Sync + 'static {
    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutA",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn text_out_a(hdc: HDC, x: i32, y: i32, lp_string: PCSTR, c: i32) -> BOOL {
        unsafe {
            #[cfg(not(feature = "text_out_arg_c_is_bytes"))]
            let byte_len = crate::code_cvt::ansi_byte_len(lp_string, c as usize);
            #[cfg(feature = "text_out_arg_c_is_bytes")]
            let byte_len = c as usize;

            let input_slice = crate::utils::mem::slice_from_raw_parts(lp_string, byte_len);

            // 进行映射处理
            let buf = crate::mapping::map_chars(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            debug!(
                "draw text '{}' at ({x}, {y}), input: {input_slice:?}",
                String::from_utf16_lossy(&buf)
            );

            crate::call!(HOOK_TEXT_OUT_W, hdc, x, y, buf.as_ptr(), buf.len() as i32)
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutW",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn text_out_w(hdc: HDC, x: i32, y: i32, lp_string: PCWSTR, c: i32) -> BOOL {
        unsafe {
            // 假定文本均在BMP
            let input_slice = crate::utils::mem::slice_from_raw_parts(lp_string, c as usize);

            let buf = crate::mapping::map_wide_chars(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            debug!(
                "draw text '{}' at ({x}, {y})",
                String::from_utf16_lossy(&buf)
            );

            crate::call!(HOOK_TEXT_OUT_W, hdc, x, y, buf.as_ptr(), buf.len() as i32)
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "ExtTextOutA",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn ext_text_out_a(
        hdc: HDC,
        x: i32,
        y: i32,
        options: u32,
        lprect: *const RECT,
        lp_string: PCSTR,
        c: u32,
        _lp_dx: *const i32,
    ) -> BOOL {
        unsafe {
            #[cfg(not(feature = "text_out_arg_c_is_bytes"))]
            let byte_len = crate::code_cvt::ansi_byte_len(lp_string, c as usize);
            #[cfg(feature = "text_out_arg_c_is_bytes")]
            let byte_len = c as usize;

            let input_slice = crate::utils::mem::slice_from_raw_parts(lp_string, byte_len);

            let buf = crate::mapping::map_chars(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            debug!(
                "ExtTextOutA '{}' at ({x}, {y}), opt={options:#x}",
                String::from_utf16_lossy(&buf)
            );

            crate::call!(
                HOOK_EXT_TEXT_OUT_W,
                hdc,
                x,
                y,
                options,
                lprect,
                buf.as_ptr(),
                buf.len() as u32,
                core::ptr::null()
            )
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "ExtTextOutW",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn ext_text_out_w(
        hdc: HDC,
        x: i32,
        y: i32,
        options: u32,
        lprect: *const RECT,
        lp_string: PCWSTR,
        c: u32,
        _lp_dx: *const i32,
    ) -> BOOL {
        unsafe {
            let input_slice = crate::utils::mem::slice_from_raw_parts(lp_string, c as usize);

            let buf = crate::mapping::map_wide_chars(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            debug!(
                "ExtTextOutW '{}' at ({x}, {y}), opt={options:#x}",
                String::from_utf16_lossy(&buf)
            );

            crate::call!(
                HOOK_EXT_TEXT_OUT_W,
                hdc,
                x,
                y,
                options,
                lprect,
                buf.as_ptr(),
                buf.len() as u32,
                core::ptr::null()
            )
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextExtentPoint32A",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn get_text_extent_point_32_a(
        hdc: HDC,
        lp_string: PCSTR,
        c: i32,
        lp_size: *mut SIZE,
    ) -> BOOL {
        unsafe {
            #[cfg(not(feature = "text_out_arg_c_is_bytes"))]
            let byte_len = crate::code_cvt::ansi_byte_len(lp_string, c as usize);
            #[cfg(feature = "text_out_arg_c_is_bytes")]
            let byte_len = c as usize;

            let input_slice = crate::utils::mem::slice_from_raw_parts(lp_string, byte_len);

            let buf = crate::mapping::map_chars(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            debug!(
                "result: {}, input: {input_slice:?}",
                String::from_utf16_lossy(&buf)
            );

            crate::call!(
                HOOK_GET_TEXT_EXTENT_POINT_32_W,
                hdc,
                buf.as_ptr(),
                buf.len() as i32,
                lp_size
            )
        }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextExtentPoint32W",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn get_text_extent_point_32_w(
        hdc: HDC,
        lp_string: PCWSTR,
        c: i32,
        lp_size: *mut SIZE,
    ) -> BOOL {
        unsafe {
            let input_slice = crate::utils::mem::slice_from_raw_parts(lp_string, c as usize);

            let buf = crate::mapping::map_wide_chars(input_slice);

            #[cfg(feature = "debug_text_mapping")]
            debug!("result: {}", String::from_utf16_lossy(&buf));

            crate::call!(
                HOOK_GET_TEXT_EXTENT_POINT_32_W,
                hdc,
                buf.as_ptr(),
                buf.len() as i32,
                lp_size
            )
        }
    }

    #[detour(dll = "gdi32.dll", symbol = "GetGlyphOutlineA", fallback = "0")]
    unsafe fn get_glyph_outline_a(
        hdc: HDC,
        u_char: u32,
        format: u32,
        lpgm: *mut GLYPHMETRICS,
        cb_buffer: u32,
        lpv_buffer: *mut core::ffi::c_void,
        lpmat2: *const MAT2,
    ) -> u32 {
        let b1 = ((u_char >> 8) & 0xFF) as u8;
        let b2 = (u_char & 0xFF) as u8;

        let input_slice = if u_char >> 8 == 0 {
            &[b2][..]
        } else {
            &[b1, b2][..]
        };

        let buf = crate::mapping::map_chars(input_slice);

        #[cfg(feature = "debug_text_mapping")]
        debug!(
            "result: {}, input: {input_slice:?}",
            String::from_utf16_lossy(&buf)
        );

        // 直接使用第一个UTF-16字符（假设都在BMP内，不需要代理对）
        if let Some(&wchar) = buf.first() {
            return unsafe {
                crate::call!(
                    HOOK_GET_GLYPH_OUTLINE_W,
                    hdc,
                    wchar as u32,
                    format,
                    lpgm,
                    cb_buffer,
                    lpv_buffer,
                    lpmat2
                )
            };
        }

        0
    }

    #[detour(dll = "gdi32.dll", symbol = "GetGlyphOutlineW", fallback = "0")]
    unsafe fn get_glyph_outline_w(
        hdc: HDC,
        u_char: u32,
        format: u32,
        lpgm: *mut GLYPHMETRICS,
        cb_buffer: u32,
        lpv_buffer: *mut core::ffi::c_void,
        lpmat2: *const MAT2,
    ) -> u32 {
        // 假设都在BMP内，所以直接`u_char as u16`
        let buf = crate::mapping::map_wide_chars(&[u_char as u16]);

        #[cfg(feature = "debug_text_mapping")]
        debug!("result: {}", String::from_utf16_lossy(&buf));

        // 直接使用第一个UTF-16字符（假设都在BMP内，不需要代理对）
        if let Some(&wchar) = buf.first() {
            return unsafe {
                crate::call!(
                    HOOK_GET_GLYPH_OUTLINE_W,
                    hdc,
                    wchar as u32,
                    format,
                    lpgm,
                    cb_buffer,
                    lpv_buffer,
                    lpmat2
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
        c_height: i32,
        c_width: i32,
        c_escapement: i32,
        c_orientation: i32,
        c_weight: i32,
        b_italic: u32,
        b_underline: u32,
        b_strike_out: u32,
        i_char_set: u32,
        i_out_precision: u32,
        i_clip_precision: u32,
        i_quality: u32,
        i_pitch_and_family: u32,
        psz_face_name: PCSTR,
    ) -> HFONT {
        let face_u16 = {
            let bytes = unsafe {
                crate::utils::mem::slice_until_null(psz_face_name, (LF_FACESIZE - 1) as usize)
            };

            crate::code_cvt::multi_byte_to_wide_char_with_null(bytes, 0)
        };

        unsafe {
            Self::create_font_w(
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
        c_height: i32,
        c_width: i32,
        c_escapement: i32,
        c_orientation: i32,
        c_weight: i32,
        b_italic: u32,
        b_underline: u32,
        b_strike_out: u32,
        i_char_set: u32,
        i_out_precision: u32,
        i_clip_precision: u32,
        i_quality: u32,
        i_pitch_and_family: u32,
        psz_face_name: PCWSTR,
    ) -> HFONT {
        let mut u16_slice: &[u16] = unsafe {
            crate::utils::mem::slice_until_null(psz_face_name, (LF_FACESIZE - 1) as usize)
        };

        debug!(
            "Requested font name: {}",
            String::from_utf16_lossy(u16_slice)
        );

        let mut buf: Option<TextVec<u16>>;

        #[cfg(not(feature = "enum_font_families"))]
        if !FONT_FILTER.contains(&u16_slice) {
            buf = Some(crate::code_cvt::u16_with_null(FONT_FACE));
            u16_slice = buf.as_ref().unwrap().as_slice();
        }

        #[cfg(feature = "enum_font_families")]
        if FONT_FILTER.contains(&u16_slice) {
            buf = Some(crate::code_cvt::u16_with_null(FONT_FACE));
            u16_slice = buf.as_ref().unwrap().as_slice();
        }

        unsafe {
            crate::call!(
                HOOK_CREATE_FONT_W,
                c_height,
                c_width,
                c_escapement,
                c_orientation,
                c_weight,
                b_italic,
                b_underline,
                b_strike_out,
                CHAR_SET as u32,
                i_out_precision,
                i_clip_precision,
                i_quality,
                i_pitch_and_family,
                u16_slice.as_ptr()
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontIndirectA",
        fallback = "core::ptr::null_mut()"
    )]
    unsafe fn create_font_indirect_a(lplf: *const LOGFONTA) -> HFONT {
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
            let bytes = unsafe {
                crate::utils::mem::slice_until_null(
                    logfona.lfFaceName.as_ptr() as *const u8,
                    logfona.lfFaceName.len() - 1, // 最后一个字节必须为null
                )
            };

            crate::code_cvt::multi_byte_to_wide_char_with_null(bytes, 0)
        };

        logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());

        let ptr = &logfontw as *const LOGFONTW;
        unsafe { Self::create_font_indirect_w(ptr) }
    }

    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontIndirectW",
        fallback = "core::ptr::null_mut()"
    )]
    unsafe fn create_font_indirect_w(lplf: *const LOGFONTW) -> HFONT {
        let mut logfontw = unsafe { *lplf };
        logfontw.lfCharSet = CHAR_SET;

        let u16_slice = unsafe {
            crate::utils::mem::slice_until_null(
                logfontw.lfFaceName.as_ptr(),
                logfontw.lfFaceName.len() - 1,
            )
        };

        debug!(
            "Requested font name: {}",
            String::from_utf16_lossy(u16_slice)
        );

        // `FONT_FACE` 长度确保不超过 LF_FACESIZE - 1，可以直接复制
        #[cfg(not(feature = "enum_font_families"))]
        if !FONT_FILTER.contains(&u16_slice) {
            let face_u16 = crate::code_cvt::u16_with_null(FONT_FACE);
            logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());
        }

        #[cfg(feature = "enum_font_families")]
        if FONT_FILTER.contains(&u16_slice) {
            let face_u16 = crate::code_cvt::u16_with_null(FONT_FACE);
            logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());
        }

        let ptr = &logfontw as *const LOGFONTW;
        unsafe { crate::call!(HOOK_CREATE_FONT_INDIRECT_W, ptr) }
    }

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesExA", fallback = "0")]
    unsafe fn enum_font_families_ex_a(
        hdc: HDC,
        lp_logfont: *mut LOGFONTA,
        lp_enum_font_fam_proc: FONTENUMPROCA,
        l_param: LPARAM,
        dw_flags: u32,
    ) -> i32 {
        #[cfg(not(feature = "enum_font_families"))]
        unimplemented!();

        #[cfg(feature = "enum_font_families")]
        unsafe {
            let info = EnumFontInfo::from_ansi(l_param, lp_enum_font_fam_proc);

            if let Some(font) = lp_logfont.as_mut() {
                font.lfCharSet = CHAR_SET;
            }

            crate::call!(
                HOOK_ENUM_FONT_FAMILIES_EX_A,
                hdc,
                lp_logfont,
                Some(enum_fonts_proc_a),
                &info as *const _ as LPARAM,
                dw_flags
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesExW", fallback = "0")]
    unsafe fn enum_font_families_ex_w(
        hdc: HDC,
        lp_logfont: *mut LOGFONTW,
        lp_enum_font_fam_proc: FONTENUMPROCW,
        l_param: LPARAM,
        dw_flags: u32,
    ) -> i32 {
        #[cfg(not(feature = "enum_font_families"))]
        unimplemented!();

        #[cfg(feature = "enum_font_families")]
        unsafe {
            let info = EnumFontInfo::from_wide(l_param, lp_enum_font_fam_proc);

            if let Some(font) = lp_logfont.as_mut() {
                font.lfCharSet = CHAR_SET;
            }
            crate::call!(
                HOOK_ENUM_FONT_FAMILIES_EX_W,
                hdc,
                lp_logfont,
                Some(enum_fonts_proc_w),
                &info as *const _ as LPARAM,
                dw_flags
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesA", fallback = "0")]
    unsafe fn enum_font_families_a(
        hdc: HDC,
        lpsz_family: PCSTR,
        lp_enum_font_fam_proc: FONTENUMPROCA,
        l_param: LPARAM,
    ) -> i32 {
        #[cfg(not(feature = "enum_font_families"))]
        unimplemented!();

        #[cfg(feature = "enum_font_families")]
        unsafe {
            let info = EnumFontInfo::from_ansi(l_param, lp_enum_font_fam_proc);

            crate::call!(
                HOOK_ENUM_FONT_FAMILIES_A,
                hdc,
                lpsz_family,
                Some(enum_fonts_proc_a),
                &info as *const _ as LPARAM
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesW", fallback = "0")]
    unsafe fn enum_font_families_w(
        hdc: HDC,
        lpsz_family: PCWSTR,
        lp_enum_font_fam_proc: FONTENUMPROCW,
        l_param: LPARAM,
    ) -> i32 {
        #[cfg(not(feature = "enum_font_families"))]
        unimplemented!();

        #[cfg(feature = "enum_font_families")]
        unsafe {
            let info = EnumFontInfo::from_wide(l_param, lp_enum_font_fam_proc);

            crate::call!(
                HOOK_ENUM_FONT_FAMILIES_W,
                hdc,
                lpsz_family,
                Some(enum_fonts_proc_w),
                &info as *const _ as LPARAM
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontsA", fallback = "0")]
    unsafe fn enum_fonts_a(
        hdc: HDC,
        lpsz_face: PCSTR,
        lp_enum_font_proc: FONTENUMPROCA,
        l_param: LPARAM,
    ) -> i32 {
        #[cfg(not(feature = "enum_font_families"))]
        unimplemented!();

        #[cfg(feature = "enum_font_families")]
        unsafe {
            let info = EnumFontInfo::from_ansi(l_param, lp_enum_font_proc);

            crate::call!(
                HOOK_ENUM_FONTS_A,
                hdc,
                lpsz_face,
                Some(enum_fonts_proc_a),
                &info as *const _ as LPARAM
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(dll = "gdi32.dll", symbol = "EnumFontsW", fallback = "0")]
    unsafe fn enum_fonts_w(
        hdc: HDC,
        lpsz_face: PCWSTR,
        lp_enum_font_proc: FONTENUMPROCW,
        l_param: LPARAM,
    ) -> i32 {
        #[cfg(not(feature = "enum_font_families"))]
        unimplemented!();

        #[cfg(feature = "enum_font_families")]
        unsafe {
            let info = EnumFontInfo::from_wide(l_param, lp_enum_font_proc);

            crate::call!(
                HOOK_ENUM_FONTS_W,
                hdc,
                lpsz_face,
                Some(enum_fonts_proc_w),
                &info as *const _ as LPARAM
            )
        }
    }
}
