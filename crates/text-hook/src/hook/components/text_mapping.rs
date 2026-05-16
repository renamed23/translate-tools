use windows_sys::{
    Win32::{
        Foundation::{RECT, SIZE},
        Graphics::Gdi::{GLYPHMETRICS, HDC, MAT2},
    },
    core::{BOOL, PCSTR, PCWSTR},
};

#[cfg(feature = "enable_custom_font")]
use crate::hook::api_hooks::gdi_text::{
    GetTextMetrics, HOOK_GET_TEXT_METRICS_A, HOOK_GET_TEXT_METRICS_W,
};
use crate::{
    hook::api_hooks::gdi_text::{
        ExtTextOut, GetGlyphOutline, GetTextExtentPoint32, HOOK_EXT_TEXT_OUT_W,
        HOOK_GET_GLYPH_OUTLINE_W, HOOK_GET_TEXT_EXTENT_POINT_32_W, HOOK_TEXT_OUT_W, TextOut,
    },
    utils::exts::{
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[allow(dead_code)]
pub struct TextMapping;

cfg_select! {
    feature = "bind_text_mapping" => {
        type TextMappingSlot = crate::hook::impls::HookImplType;
    }
    _ => {
        type TextMappingSlot = TextMapping;
    }
}

impl TextOut for TextMappingSlot {
    unsafe fn text_out_a(hdc: HDC, x: i32, y: i32, lp_string: PCSTR, c: i32) -> BOOL {
        unsafe {
            let byte_len = get_byte_len(lp_string, c as usize);

            let input_slice = lp_string.to_slice(byte_len);
            let buf = input_slice.to_wide_ansi().mapping();

            #[cfg(feature = "enable_text_mapping_debug")]
            crate::debug!(
                "draw text '{}' at ({x}, {y}), input: {input_slice:?}",
                buf.to_string_lossy()
            );

            with_font(hdc, || {
                crate::call!(HOOK_TEXT_OUT_W, hdc, x, y, buf.as_ptr(), buf.len() as i32)
            })
        }
    }

    unsafe fn text_out_w(hdc: HDC, x: i32, y: i32, lp_string: PCWSTR, c: i32) -> BOOL {
        unsafe {
            let input_slice = lp_string.to_slice(c as usize);

            let buf = input_slice.mapping();

            #[cfg(feature = "enable_text_mapping_debug")]
            crate::debug!("draw text '{}' at ({x}, {y})", buf.to_string_lossy());

            with_font(hdc, || {
                crate::call!(HOOK_TEXT_OUT_W, hdc, x, y, buf.as_ptr(), buf.len() as i32)
            })
        }
    }
}

impl ExtTextOut for TextMappingSlot {
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
            let byte_len = get_byte_len(lp_string, c as usize);

            let input_slice = lp_string.to_slice(byte_len);
            let buf = input_slice.to_wide_ansi().mapping();

            #[cfg(feature = "enable_text_mapping_debug")]
            crate::debug!(
                "ExtTextOutA '{}' at ({x}, {y}), opt={options:#x}",
                buf.to_string_lossy()
            );

            with_font(hdc, || {
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
            })
        }
    }

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
            let input_slice = lp_string.to_slice(c as usize);

            let buf = input_slice.mapping();

            #[cfg(feature = "enable_text_mapping_debug")]
            crate::debug!(
                "ExtTextOutW '{}' at ({x}, {y}), opt={options:#x}",
                buf.to_string_lossy()
            );

            with_font(hdc, || {
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
            })
        }
    }
}

impl GetTextExtentPoint32 for TextMappingSlot {
    unsafe fn get_text_extent_point_32_a(
        hdc: HDC,
        lp_string: PCSTR,
        c: i32,
        lp_size: *mut SIZE,
    ) -> BOOL {
        unsafe {
            let byte_len = get_byte_len(lp_string, c as usize);

            let input_slice = lp_string.to_slice(byte_len);
            let buf = input_slice.to_wide_ansi().mapping();

            #[cfg(feature = "enable_text_mapping_debug")]
            crate::debug!("result: {}, input: {input_slice:?}", buf.to_string_lossy());

            with_font(hdc, || {
                crate::call!(
                    HOOK_GET_TEXT_EXTENT_POINT_32_W,
                    hdc,
                    buf.as_ptr(),
                    buf.len() as i32,
                    lp_size
                )
            })
        }
    }

    unsafe fn get_text_extent_point_32_w(
        hdc: HDC,
        lp_string: PCWSTR,
        c: i32,
        lp_size: *mut SIZE,
    ) -> BOOL {
        unsafe {
            let input_slice = lp_string.to_slice(c as usize);

            let buf = input_slice.mapping();

            #[cfg(feature = "enable_text_mapping_debug")]
            crate::debug!("result: {}", buf.to_string_lossy());

            with_font(hdc, || {
                crate::call!(
                    HOOK_GET_TEXT_EXTENT_POINT_32_W,
                    hdc,
                    buf.as_ptr(),
                    buf.len() as i32,
                    lp_size
                )
            })
        }
    }
}

impl GetGlyphOutline for TextMappingSlot {
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

        let buf = input_slice.to_wide_ansi().mapping();

        #[cfg(feature = "enable_text_mapping_debug")]
        crate::debug!("result: {}, input: {input_slice:?}", buf.to_string_lossy());

        // 直接使用第一个UTF-16字符（假设都在BMP内，不需要代理对）
        if let Some(&wchar) = buf.first() {
            return with_font(hdc, || unsafe {
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
            });
        }

        0
    }

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
        let buf = [u_char as u16].mapping();

        #[cfg(feature = "enable_text_mapping_debug")]
        crate::debug!("result: {}", buf.to_string_lossy());

        // 直接使用第一个UTF-16字符（假设都在BMP内，不需要代理对）
        if let Some(&wchar) = buf.first() {
            return with_font(hdc, || unsafe {
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
            });
        }

        0
    }
}

#[cfg(feature = "enable_custom_font")]
impl GetTextMetrics for TextMappingSlot {
    unsafe fn get_text_metrics_a(
        hdc: HDC,
        lptm: *mut windows_sys::Win32::Graphics::Gdi::TEXTMETRICA,
    ) -> BOOL {
        with_font(hdc, || unsafe {
            crate::call!(HOOK_GET_TEXT_METRICS_A, hdc, lptm)
        })
    }

    unsafe fn get_text_metrics_w(
        hdc: HDC,
        lptm: *mut windows_sys::Win32::Graphics::Gdi::TEXTMETRICW,
    ) -> BOOL {
        with_font(hdc, || unsafe {
            crate::call!(HOOK_GET_TEXT_METRICS_W, hdc, lptm)
        })
    }
}

/// 如果开启了自定义字体，则自动select该字体然后调用闭包，否则直接调用闭包
fn with_font<F, R>(_hdc: HDC, f: F) -> R
where
    F: FnOnce() -> R,
{
    cfg_select!(
        feature = "enable_custom_font" => {
            crate::custom_font::with_font(_hdc, f)
        }
        _ => {
            f()
        }
    )
}

/// 根据字符数计算传入ANSI字符串的字节长度
fn get_byte_len(_ptr: *const u8, chars: usize) -> usize {
    cfg_select!(
        feature = "assume_text_out_arg_c_is_byte_len" => {
            chars
        }
        _ => {
            crate::code_cvt::byte_len(_ptr, chars, crate::constant::ANSI_CODE_PAGE as u16)
        }
    )
}
