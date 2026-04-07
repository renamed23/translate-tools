use cfg_if::cfg_if;
use windows_sys::{
    Win32::Graphics::Gdi::{HFONT, LF_FACESIZE, LOGFONTA, LOGFONTW},
    core::{PCSTR, PCWSTR},
};

#[cfg(feature = "disable_forced_font")]
use windows_sys::Win32::{
    Foundation::LPARAM,
    Graphics::Gdi::{FONTENUMPROCA, FONTENUMPROCW, HDC, TEXTMETRICA, TEXTMETRICW},
};

use crate::{
    constant::{
        CHAR_SET, FONT_FACE, FONT_FILTER, CREATE_FONT_C_HEIGHT, CREATE_FONT_C_WIDTH,
        CREATE_FONT_C_ESCAPEMENT, CREATE_FONT_C_ORIENTATION, CREATE_FONT_C_WEIGHT,
        CREATE_FONT_B_ITALIC, CREATE_FONT_B_UNDERLINE, CREATE_FONT_B_STRIKE_OUT,
        CREATE_FONT_I_OUT_PRECISION, CREATE_FONT_I_CLIP_PRECISION, CREATE_FONT_I_QUALITY,
        CREATE_FONT_I_PITCH_AND_FAMILY,
    },
    debug,
    hook::api_hooks::gdi_text::{
        CreateFont, CreateFontIndirect, HOOK_CREATE_FONT_INDIRECT_A, HOOK_CREATE_FONT_INDIRECT_W,
        HOOK_CREATE_FONT_W,
    },
    utils::exts::{
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[cfg(feature = "disable_forced_font")]
use crate::{
    constant::{ENUM_FONT_PROC_CHAR_SET, ENUM_FONT_PROC_OUT_PRECISION, ENUM_FONT_PROC_PITCH},
    hook::api_hooks::gdi_text::{
        EnumFontFamilies, EnumFontFamiliesEx, EnumFonts, HOOK_ENUM_FONT_FAMILIES_A,
        HOOK_ENUM_FONT_FAMILIES_EX_A, HOOK_ENUM_FONT_FAMILIES_EX_W, HOOK_ENUM_FONT_FAMILIES_W,
        HOOK_ENUM_FONTS_A, HOOK_ENUM_FONTS_W,
    },
};

#[allow(dead_code)]
pub struct FontManager;

cfg_if! {
    if #[cfg(feature = "bind_font_manager")] {
        type FontManagerSlot = crate::hook::impls::HookImplType;
    } else {
        type FontManagerSlot = FontManager;
    }
}

impl CreateFont for FontManagerSlot {
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
        unsafe {
            let face_u16 = psz_face_name
                .to_slice_until_null((LF_FACESIZE - 1) as usize)
                .to_wide_null(0);

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
        let mut u16_slice: &[u16] =
            unsafe { psz_face_name.to_slice_until_null((LF_FACESIZE - 1) as usize) };

        debug!("Requested font name: {}", u16_slice.to_string_lossy());

        let mut buf: Option<Vec<u16>>;

        #[cfg(not(feature = "disable_forced_font"))]
        if !FONT_FILTER.contains(&u16_slice) {
            buf = Some(FONT_FACE.with_null());
            u16_slice = buf.as_ref().unwrap().as_slice();
        }

        #[cfg(feature = "disable_forced_font")]
        if FONT_FILTER.contains(&u16_slice) {
            buf = Some(FONT_FACE.with_null());
            u16_slice = buf.as_ref().unwrap().as_slice();
        }

        // 使用配置的参数覆盖传入的参数
        let c_height = CREATE_FONT_C_HEIGHT.unwrap_or(c_height);
        let c_width = CREATE_FONT_C_WIDTH.unwrap_or(c_width);
        let c_escapement = CREATE_FONT_C_ESCAPEMENT.unwrap_or(c_escapement);
        let c_orientation = CREATE_FONT_C_ORIENTATION.unwrap_or(c_orientation);
        let c_weight = CREATE_FONT_C_WEIGHT.unwrap_or(c_weight);
        let b_italic = CREATE_FONT_B_ITALIC.unwrap_or(b_italic);
        let b_underline = CREATE_FONT_B_UNDERLINE.unwrap_or(b_underline);
        let b_strike_out = CREATE_FONT_B_STRIKE_OUT.unwrap_or(b_strike_out);
        let i_out_precision = CREATE_FONT_I_OUT_PRECISION.unwrap_or(i_out_precision);
        let i_clip_precision = CREATE_FONT_I_CLIP_PRECISION.unwrap_or(i_clip_precision);
        let i_quality = CREATE_FONT_I_QUALITY.unwrap_or(i_quality);
        let i_pitch_and_family = CREATE_FONT_I_PITCH_AND_FAMILY.unwrap_or(i_pitch_and_family);

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
}

impl CreateFontIndirect for FontManagerSlot {
    unsafe fn create_font_indirect_a(lplf: *const LOGFONTA) -> HFONT {
        if lplf.is_null() {
            return unsafe { crate::call!(HOOK_CREATE_FONT_INDIRECT_A, lplf) };
        }

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
                logfona
                    .lfFaceName
                    .as_ptr()
                    .to_slice_until_null(logfona.lfFaceName.len() - 1) // 最后一个字节必须为null
            };

            bytes.to_wide_null(0)
        };

        logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());

        let ptr = &logfontw as *const LOGFONTW;
        unsafe { Self::create_font_indirect_w(ptr) }
    }

    unsafe fn create_font_indirect_w(lplf: *const LOGFONTW) -> HFONT {
        if lplf.is_null() {
            return unsafe { crate::call!(HOOK_CREATE_FONT_INDIRECT_W, lplf) };
        }

        let mut logfontw = unsafe { *lplf };
        logfontw.lfCharSet = CHAR_SET;

        let u16_slice = unsafe {
            logfontw
                .lfFaceName
                .as_ptr()
                .to_slice_until_null(logfontw.lfFaceName.len() - 1)
        };

        debug!("Requested font name: {}", u16_slice.to_string_lossy());

        // `FONT_FACE` 长度确保不超过 LF_FACESIZE - 1，可以直接复制
        #[cfg(not(feature = "disable_forced_font"))]
        if !FONT_FILTER.contains(&u16_slice) {
            let face_u16 = FONT_FACE.with_null();
            logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());
        }

        #[cfg(feature = "disable_forced_font")]
        if FONT_FILTER.contains(&u16_slice) {
            let face_u16 = FONT_FACE.with_null();
            logfontw.lfFaceName[..face_u16.len()].copy_from_slice(face_u16.as_slice());
        }

        // 使用配置的参数覆盖 LOGFONTW 结构体的字段
        if let Some(height) = CREATE_FONT_C_HEIGHT {
            logfontw.lfHeight = height;
        }
        if let Some(width) = CREATE_FONT_C_WIDTH {
            logfontw.lfWidth = width;
        }
        if let Some(escapement) = CREATE_FONT_C_ESCAPEMENT {
            logfontw.lfEscapement = escapement;
        }
        if let Some(orientation) = CREATE_FONT_C_ORIENTATION {
            logfontw.lfOrientation = orientation;
        }
        if let Some(weight) = CREATE_FONT_C_WEIGHT {
            logfontw.lfWeight = weight;
        }
        if let Some(italic) = CREATE_FONT_B_ITALIC {
            logfontw.lfItalic = italic as u8;
        }
        if let Some(underline) = CREATE_FONT_B_UNDERLINE {
            logfontw.lfUnderline = underline as u8;
        }
        if let Some(strike_out) = CREATE_FONT_B_STRIKE_OUT {
            logfontw.lfStrikeOut = strike_out as u8;
        }
        if let Some(out_precision) = CREATE_FONT_I_OUT_PRECISION {
            logfontw.lfOutPrecision = out_precision as u8;
        }
        if let Some(clip_precision) = CREATE_FONT_I_CLIP_PRECISION {
            logfontw.lfClipPrecision = clip_precision as u8;
        }
        if let Some(quality) = CREATE_FONT_I_QUALITY {
            logfontw.lfQuality = quality as u8;
        }
        if let Some(pitch_and_family) = CREATE_FONT_I_PITCH_AND_FAMILY {
            logfontw.lfPitchAndFamily = pitch_and_family as u8;
        }

        let ptr = &logfontw as *const LOGFONTW;
        unsafe { crate::call!(HOOK_CREATE_FONT_INDIRECT_W, ptr) }
    }
}

#[cfg(feature = "disable_forced_font")]
impl EnumFontFamiliesEx for FontManagerSlot {
    unsafe fn enum_font_families_ex_a(
        hdc: HDC,
        lp_logfont: *mut LOGFONTA,
        lp_enum_font_fam_proc: FONTENUMPROCA,
        l_param: LPARAM,
        dw_flags: u32,
    ) -> i32 {
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

    unsafe fn enum_font_families_ex_w(
        hdc: HDC,
        lp_logfont: *mut LOGFONTW,
        lp_enum_font_fam_proc: FONTENUMPROCW,
        l_param: LPARAM,
        dw_flags: u32,
    ) -> i32 {
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
}

#[cfg(feature = "disable_forced_font")]
impl EnumFontFamilies for FontManagerSlot {
    unsafe fn enum_font_families_a(
        hdc: HDC,
        lpsz_family: PCSTR,
        lp_enum_font_fam_proc: FONTENUMPROCA,
        l_param: LPARAM,
    ) -> i32 {
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

    unsafe fn enum_font_families_w(
        hdc: HDC,
        lpsz_family: PCWSTR,
        lp_enum_font_fam_proc: FONTENUMPROCW,
        l_param: LPARAM,
    ) -> i32 {
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
}

#[cfg(feature = "disable_forced_font")]
impl EnumFonts for FontManagerSlot {
    unsafe fn enum_fonts_a(
        hdc: HDC,
        lpsz_face: PCSTR,
        lp_enum_font_proc: FONTENUMPROCA,
        l_param: LPARAM,
    ) -> i32 {
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

    unsafe fn enum_fonts_w(
        hdc: HDC,
        lpsz_face: PCWSTR,
        lp_enum_font_proc: FONTENUMPROCW,
        l_param: LPARAM,
    ) -> i32 {
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

#[cfg(feature = "disable_forced_font")]
pub struct EnumFontInfo {
    original_proc_a: FONTENUMPROCA,
    original_proc_w: FONTENUMPROCW,
    original_lparam: LPARAM,
}

#[cfg(feature = "disable_forced_font")]
impl EnumFontInfo {
    pub fn from_ansi(lparam: LPARAM, proc_a: FONTENUMPROCA) -> Self {
        Self {
            original_lparam: lparam,
            original_proc_a: proc_a,
            original_proc_w: None,
        }
    }

    pub fn from_wide(lparam: LPARAM, proc_w: FONTENUMPROCW) -> Self {
        Self {
            original_lparam: lparam,
            original_proc_a: None,
            original_proc_w: proc_w,
        }
    }
}

#[cfg(feature = "disable_forced_font")]
pub unsafe extern "system" fn enum_fonts_proc_a(
    lplf: *const LOGFONTA,
    lptm: *const TEXTMETRICA,
    font_type: u32,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        if lplf.is_null() || lparam == 0 {
            return 0;
        }

        let info = &*(lparam as *const EnumFontInfo);

        let Some(original_proc) = info.original_proc_a else {
            debug!("original_proc_a is None");
            return 0;
        };

        let mut modified_lf = *lplf;

        if let Some(charset) = ENUM_FONT_PROC_CHAR_SET {
            modified_lf.lfCharSet = charset;
        }

        if let Some(pitch) = ENUM_FONT_PROC_PITCH {
            modified_lf.lfPitchAndFamily = (modified_lf.lfPitchAndFamily & 0b1111_1100) | pitch;
        }

        if let Some(out_precision) = ENUM_FONT_PROC_OUT_PRECISION {
            modified_lf.lfOutPrecision = out_precision;
        }

        #[cfg(feature = "enable_debug_output")]
        {
            use crate::utils::exts::ptr_ext::PtrExt;

            let facename_slice = modified_lf
                .lfFaceName
                .as_ptr()
                .to_slice_until_null(modified_lf.lfFaceName.len());

            debug!(
                "Enuming font '{}'...",
                facename_slice.to_wide(0).to_string_lossy()
            );
        }

        original_proc(&modified_lf, lptm, font_type, info.original_lparam)
    }
}

#[cfg(feature = "disable_forced_font")]
pub unsafe extern "system" fn enum_fonts_proc_w(
    lplf: *const LOGFONTW,
    lptm: *const TEXTMETRICW,
    font_type: u32,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        if lplf.is_null() || lparam == 0 {
            return 0;
        }

        let info = &*(lparam as *const EnumFontInfo);

        let Some(original_proc) = info.original_proc_w else {
            debug!("original_proc_w is None");
            return 0;
        };

        let mut modified_lf = *lplf;

        if let Some(charset) = ENUM_FONT_PROC_CHAR_SET {
            modified_lf.lfCharSet = charset;
        }

        if let Some(pitch) = ENUM_FONT_PROC_PITCH {
            modified_lf.lfPitchAndFamily = (modified_lf.lfPitchAndFamily & 0b1111_1100) | pitch;
        }

        if let Some(out_precision) = ENUM_FONT_PROC_OUT_PRECISION {
            modified_lf.lfOutPrecision = out_precision;
        }

        #[cfg(feature = "enable_debug_output")]
        {
            use crate::utils::exts::ptr_ext::PtrExt;

            let facename_slice = modified_lf
                .lfFaceName
                .as_ptr()
                .to_slice_until_null(modified_lf.lfFaceName.len());

            debug!("Enuming font '{}'...", facename_slice.to_string_lossy());
        }

        original_proc(&modified_lf, lptm, font_type, info.original_lparam)
    }
}
