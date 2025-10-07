use winapi::um::wingdi::AddFontMemResourceEx;
use winapi::um::winnt::HANDLE;

use crate::debug;

include_flate::flate!(
    static CUSTOM_FONT: [u8] from "assets\\custom_font.ttf"
);

/// 获取内嵌的字体数据
pub fn get_font_data() -> &'static [u8] {
    CUSTOM_FONT.as_slice()
}

/// 将内嵌字体添加到系统中，返回字体句柄（u32）。
/// 如果已经添加过，则返回之前的句柄。
/// 如果添加失败，返回`None`
pub fn add_font() -> Option<u32> {
    static mut FONT_HANDLE: Option<HANDLE> = None;

    unsafe {
        if let Some(handle) = FONT_HANDLE {
            return Some(handle as u32);
        }

        let font_data = get_font_data();
        let handle = AddFontMemResourceEx(
            font_data.as_ptr() as *mut _,
            font_data.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );

        if handle.is_null() {
            debug!("AddFontMemResourceEx failed");
            None
        } else {
            FONT_HANDLE = Some(handle);
            Some(handle as u32)
        }
    }
}
