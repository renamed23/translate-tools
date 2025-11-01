use windows_sys::Win32::{
    Foundation::LPARAM,
    Graphics::Gdi::{FONTENUMPROCA, FONTENUMPROCW, LOGFONTA, LOGFONTW, TEXTMETRICA, TEXTMETRICW},
};

use crate::{constant, debug};

pub struct EnumFontInfo {
    original_proc_a: FONTENUMPROCA,
    original_proc_w: FONTENUMPROCW,
    original_lparam: LPARAM,
}

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
        modified_lf.lfCharSet = constant::ENUM_FONT_PROC_CHAR_SET;

        debug!("Enuming font...");

        original_proc(&modified_lf, lptm, font_type, info.original_lparam)
    }
}

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
        modified_lf.lfCharSet = constant::ENUM_FONT_PROC_CHAR_SET;

        debug!("Enuming font...");

        original_proc(&modified_lf, lptm, font_type, info.original_lparam)
    }
}
