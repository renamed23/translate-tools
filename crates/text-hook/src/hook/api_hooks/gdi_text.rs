use translate_macros::detour_trait;
use windows_sys::{
    Win32::{
        Foundation::{FALSE, LPARAM, RECT, SIZE},
        Graphics::Gdi::{
            FONTENUMPROCA, FONTENUMPROCW, GLYPHMETRICS, HDC, HFONT, LOGFONTA, LOGFONTW, MAT2,
        },
    },
    core::{BOOL, PCSTR, PCWSTR},
};

#[detour_trait]
pub trait TextOut {
    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutA",
        fallback = FALSE
    )]
    unsafe fn text_out_a(hdc: HDC, x: i32, y: i32, lp_string: PCSTR, c: i32) -> BOOL;

    #[detour(
        dll = "gdi32.dll",
        symbol = "TextOutW",
        fallback = FALSE
    )]
    unsafe fn text_out_w(hdc: HDC, x: i32, y: i32, lp_string: PCWSTR, c: i32) -> BOOL;
}

#[detour_trait]
pub trait ExtTextOut {
    #[detour(
        dll = "gdi32.dll",
        symbol = "ExtTextOutA",
        fallback = FALSE
    )]
    unsafe fn ext_text_out_a(
        hdc: HDC,
        x: i32,
        y: i32,
        options: u32,
        lprect: *const RECT,
        lp_string: PCSTR,
        c: u32,
        lp_dx: *const i32,
    ) -> BOOL;

    #[detour(
        dll = "gdi32.dll",
        symbol = "ExtTextOutW",
        fallback = FALSE
    )]
    unsafe fn ext_text_out_w(
        hdc: HDC,
        x: i32,
        y: i32,
        options: u32,
        lprect: *const RECT,
        lp_string: PCWSTR,
        c: u32,
        lp_dx: *const i32,
    ) -> BOOL;
}

#[detour_trait]
pub trait GetTextExtentPoint32 {
    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextExtentPoint32A",
        fallback = FALSE
    )]
    unsafe fn get_text_extent_point_32_a(
        hdc: HDC,
        lp_string: PCSTR,
        c: i32,
        lp_size: *mut SIZE,
    ) -> BOOL;

    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextExtentPoint32W",
        fallback = FALSE
    )]
    unsafe fn get_text_extent_point_32_w(
        hdc: HDC,
        lp_string: PCWSTR,
        c: i32,
        lp_size: *mut SIZE,
    ) -> BOOL;
}

#[detour_trait]
pub trait GetGlyphOutline {
    #[detour(dll = "gdi32.dll", symbol = "GetGlyphOutlineA", fallback = 0)]
    unsafe fn get_glyph_outline_a(
        hdc: HDC,
        u_char: u32,
        format: u32,
        lpgm: *mut GLYPHMETRICS,
        cb_buffer: u32,
        lpv_buffer: *mut core::ffi::c_void,
        lpmat2: *const MAT2,
    ) -> u32;

    #[detour(dll = "gdi32.dll", symbol = "GetGlyphOutlineW", fallback = 0)]
    unsafe fn get_glyph_outline_w(
        hdc: HDC,
        u_char: u32,
        format: u32,
        lpgm: *mut GLYPHMETRICS,
        cb_buffer: u32,
        lpv_buffer: *mut core::ffi::c_void,
        lpmat2: *const MAT2,
    ) -> u32;
}

#[detour_trait]
pub trait CreateFont {
    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontA",
        fallback = core::ptr::null_mut()
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
    ) -> HFONT;

    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontW",
        fallback = core::ptr::null_mut()
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
    ) -> HFONT;
}

#[detour_trait]
pub trait CreateFontIndirect {
    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontIndirectA",
        fallback = core::ptr::null_mut()
    )]
    unsafe fn create_font_indirect_a(lplf: *const LOGFONTA) -> HFONT;

    #[detour(
        dll = "gdi32.dll",
        symbol = "CreateFontIndirectW",
        fallback = core::ptr::null_mut()
    )]
    unsafe fn create_font_indirect_w(lplf: *const LOGFONTW) -> HFONT;
}

#[detour_trait]
pub trait EnumFontFamilies {
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesA", fallback = 0)]
    unsafe fn enum_font_families_a(
        hdc: HDC,
        lpsz_family: PCSTR,
        lp_enum_font_fam_proc: FONTENUMPROCA,
        l_param: LPARAM,
    ) -> i32;

    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesW", fallback = 0)]
    unsafe fn enum_font_families_w(
        hdc: HDC,
        lpsz_family: PCWSTR,
        lp_enum_font_fam_proc: FONTENUMPROCW,
        l_param: LPARAM,
    ) -> i32;
}

#[detour_trait]
pub trait EnumFontFamiliesEx {
    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesExA", fallback = 0)]
    unsafe fn enum_font_families_ex_a(
        hdc: HDC,
        lp_logfont: *mut LOGFONTA,
        lp_enum_font_fam_proc: FONTENUMPROCA,
        l_param: LPARAM,
        dw_flags: u32,
    ) -> i32;

    #[detour(dll = "gdi32.dll", symbol = "EnumFontFamiliesExW", fallback = 0)]
    unsafe fn enum_font_families_ex_w(
        hdc: HDC,
        lp_logfont: *mut LOGFONTW,
        lp_enum_font_fam_proc: FONTENUMPROCW,
        l_param: LPARAM,
        dw_flags: u32,
    ) -> i32;
}

#[detour_trait]
pub trait EnumFonts {
    #[detour(dll = "gdi32.dll", symbol = "EnumFontsA", fallback = 0)]
    unsafe fn enum_fonts_a(
        hdc: HDC,
        lpsz_face: PCSTR,
        lp_enum_font_proc: FONTENUMPROCA,
        l_param: LPARAM,
    ) -> i32;

    #[detour(dll = "gdi32.dll", symbol = "EnumFontsW", fallback = 0)]
    unsafe fn enum_fonts_w(
        hdc: HDC,
        lpsz_face: PCWSTR,
        lp_enum_font_proc: FONTENUMPROCW,
        l_param: LPARAM,
    ) -> i32;
}

#[detour_trait]
pub trait GetTextMetrics {
    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextMetricsA",
        fallback = FALSE
    )]
    unsafe fn get_text_metrics_a(
        hdc: HDC,
        lptm: *mut windows_sys::Win32::Graphics::Gdi::TEXTMETRICA,
    ) -> BOOL;

    #[detour(
        dll = "gdi32.dll",
        symbol = "GetTextMetricsW",
        fallback = FALSE
    )]
    unsafe fn get_text_metrics_w(
        hdc: HDC,
        lptm: *mut windows_sys::Win32::Graphics::Gdi::TEXTMETRICW,
    ) -> BOOL;
}
