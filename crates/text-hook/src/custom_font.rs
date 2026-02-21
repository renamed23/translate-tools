use windows_sys::Win32::{
    Foundation::HANDLE,
    Graphics::Gdi::{AddFontMemResourceEx, RemoveFontMemResourceEx},
};

use crate::print_last_error_message;

translate_macros::embed!(
    static CUSTOM_FONT: [u8] from "assets\\font"
);

/// 获取内嵌的字体数据
pub fn get_font_data() -> &'static [u8] {
    CUSTOM_FONT.as_slice()
}

static mut FONT_HANDLE: Option<HANDLE> = None;

/// 将内嵌字体添加到系统中
///
/// # Safety
/// - 仅应在初始化阶段调用，且由调用者保证不会并发调用。
/// - 调用者需保证本函数与 `remove_font` 的调用时序正确（先 add 后 remove）。
pub unsafe fn add_font() -> crate::Result<()> {
    unsafe {
        #[allow(clippy::redundant_pattern_matching)]
        if matches!(FONT_HANDLE, Some(_)) {
            return Ok(());
        }

        let font_data = get_font_data();

        let mut c_fonts: u32 = 0;

        let handle = AddFontMemResourceEx(
            font_data.as_ptr() as *const _,
            font_data.len() as u32,
            core::ptr::null_mut(),
            &mut c_fonts as *mut u32,
        );

        if handle.is_null() {
            print_last_error_message!();
            crate::bail!("AddFontMemResourceEx failed");
        } else {
            FONT_HANDLE = Some(handle);
            Ok(())
        }
    }
}

/// 从系统中移除已添加的内嵌字体。
/// 移除成功会清空内部句柄缓存。
///
///
/// # Safety
/// - 仅应在清理阶段调用，且由调用者保证不会并发调用。
/// - 调用前必须保证字体已通过 `add_font` 成功添加。
pub unsafe fn remove_font() -> crate::Result<()> {
    unsafe {
        if let Some(handle) = FONT_HANDLE {
            FONT_HANDLE = None;
            if RemoveFontMemResourceEx(handle) != 0 {
                Ok(())
            } else {
                crate::bail!("RemoveFontMemResourceEx failed");
            }
        } else {
            crate::bail!("remove_font called but font is not added");
        }
    }
}
