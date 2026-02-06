use windows_sys::Win32::{
    Foundation::HANDLE,
    Graphics::Gdi::{AddFontMemResourceEx, RemoveFontMemResourceEx},
};

use crate::print_last_error_message;

translate_macros::flate!(
    static CUSTOM_FONT: [u8] from "assets\\font"
);

/// 获取内嵌的字体数据
pub fn get_font_data() -> &'static [u8] {
    CUSTOM_FONT.as_slice()
}

static mut FONT_HANDLE: Option<HANDLE> = None;

/// 将内嵌字体添加到系统中，返回字体句柄。
/// 如果已经添加过，则返回之前的句柄。
/// 如果添加失败，返回`Err`
/// 注意该函数应该只在DLL attach的时候才调用
pub unsafe fn add_font() -> crate::Result<usize> {
    unsafe {
        if let Some(handle) = FONT_HANDLE {
            return Ok(handle as usize);
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
            Ok(handle as usize)
        }
    }
}

/// 从系统中移除已添加的内嵌字体。
/// 如果尚未添加，返回`false`。
/// 移除成功会清空内部句柄缓存。
///
/// 注意该函数应该只在DLL detach时才调用
pub unsafe fn remove_font() -> crate::Result<bool> {
    unsafe {
        if let Some(handle) = FONT_HANDLE {
            FONT_HANDLE = None;
            if RemoveFontMemResourceEx(handle) != 0 {
                Ok(true)
            } else {
                crate::bail!("RemoveFontMemResourceEx failed");
            }
        } else {
            // 未添加，无需移除
            Ok(false)
        }
    }
}
