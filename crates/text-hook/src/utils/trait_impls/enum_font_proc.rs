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
        let info = &*(lparam as *const EnumFontInfo);

        let mut modified_lf = *lplf;
        modified_lf.lfCharSet = constant::ENUM_FONT_PROC_CHAR_SET;

        debug!("Enuming font...");

        (info.original_proc_a.unwrap())(&modified_lf, lptm, font_type, info.original_lparam)
    }
}

pub unsafe extern "system" fn enum_fonts_proc_w(
    lplf: *const LOGFONTW,
    lptm: *const TEXTMETRICW,
    font_type: u32,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        let info = &*(lparam as *const EnumFontInfo);

        let mut modified_lf = *lplf;
        modified_lf.lfCharSet = constant::ENUM_FONT_PROC_CHAR_SET;

        debug!("Enuming font...");

        (info.original_proc_w.unwrap())(&modified_lf, lptm, font_type, info.original_lparam)
    }
}
