use std::sync::{LazyLock, RwLock};
use windows_sys::Win32::Graphics::Gdi::{DeleteObject, HDC, HFONT, SelectObject};

use crate::{
    hook::api_hooks::gdi_text::HOOK_CREATE_FONT_INDIRECT_W, print_last_error_message,
    utils::log_font::LogFont,
};

const CURRENT_FONT_KEY: &str = "custom_font_manager::current_font";

struct CustomFontManager {
    current_font: LogFont,
    current_handle: usize,
}

impl CustomFontManager {
    fn load_from_storage() -> Self {
        let mut this = Self {
            current_font: crate::storage::get_value(CURRENT_FONT_KEY).unwrap_or_default(),
            current_handle: 0,
        };

        if let Err(e) = this.apply_font() {
            crate::debug!("Create font handle failed when load from storage {e:?}");
        }

        this
    }

    fn set_font(&mut self, font: LogFont) -> crate::Result<()> {
        if font == self.current_font {
            return Ok(());
        }

        self.current_font = font;
        self.apply_font()?;

        Ok(())
    }

    fn apply_font(&mut self) -> crate::Result<()> {
        let sys_w = self.current_font.to_sys_w();

        let handle = unsafe { crate::call!(HOOK_CREATE_FONT_INDIRECT_W, &raw const sys_w) };

        if handle.is_null() {
            print_last_error_message!();
            crate::bail!("Create new font handle failed");
        }

        if let Err(e) = self.delete_handle() {
            crate::debug!("Delete failed {e:?}");
        }

        self.current_handle = handle as usize;

        Ok(())
    }

    fn delete_handle(&mut self) -> crate::Result<()> {
        if self.current_handle != 0 {
            let result = unsafe { DeleteObject(self.current_handle as HFONT) };
            self.current_handle = 0;

            if result == 0 {
                print_last_error_message!();
                crate::bail!("Delete current font handle failed");
            }
        }

        Ok(())
    }
}

static FONT_MANAGER: LazyLock<RwLock<CustomFontManager>> =
    LazyLock::new(|| RwLock::new(CustomFontManager::load_from_storage()));

/// 设置并应用字体
pub fn set_font(lf: LogFont) -> crate::Result<()> {
    let mut manager = FONT_MANAGER.write().expect("RwLock poisoned");
    manager.set_font(lf)
}

/// 获取当前字体
pub fn get_font() -> LogFont {
    FONT_MANAGER
        .read()
        .expect("RwLock poisoned")
        .current_font
        .clone()
}

/// 将当前字体信息保存到存储中，并清理字体 handle
pub fn save_and_cleanup() -> crate::Result<()> {
    crate::storage::set_value(CURRENT_FONT_KEY, &get_font())?;
    FONT_MANAGER
        .write()
        .expect("RwLock poisoned")
        .delete_handle()?;
    Ok(())
}

/// 执行绘图操作并在结束后自动还原字体，
/// 禁止在闭包中调用`set_font`，`save_and_cleanup`这样需要修改字体handle的函数
pub fn with_font<F, R>(hdc: HDC, f: F) -> R
where
    F: FnOnce() -> R,
{
    let handle = FONT_MANAGER.read().expect("RwLock poisoned").current_handle as HFONT;

    if handle.is_null() {
        crate::debug!("Try to use current font, but which is null");
    }

    unsafe {
        let old_font = SelectObject(hdc, handle);
        if old_font.is_null() {
            print_last_error_message!();
            crate::debug!("SelectObject failed");
        }

        scopeguard::defer!(if !old_font.is_null() {
            SelectObject(hdc, old_font);
        });

        f()
    }
}
