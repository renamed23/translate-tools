mod window;

use std::sync::RwLock;

use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{
        DestroyWindow, EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW,
        GetParent, GetWindowRect, IsWindow, MoveWindow, OBJID_WINDOW,
    },
};

use crate::{
    constant::{OVERLAY_TARGET_WINDOW_CLASS_NAME, OVERLAY_TARGET_WINDOW_TEXT},
    overlay::window::create_overlay_window,
};

/// Overlay上下文结构体
#[derive(Clone, Copy)]
pub struct OverlayContext {
    /// 目标窗口 hwnd
    pub target: usize,

    /// Overlay窗口 hwnd
    pub overlay: usize,
}

/// Overlay上下文
pub static OVERLAY_CTX: RwLock<Option<OverlayContext>> = RwLock::new(None);

/// 根据窗口事件获取目标窗口的hwnd并创建overlay窗口，并根据目标窗口同步overlay窗口
///
/// 由 `win_event_hook_proc` 调用
pub fn win_event_callback(
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    unsafe {
        if id_object != OBJID_WINDOW {
            return;
        }

        match event {
            EVENT_OBJECT_SHOW => {
                if OVERLAY_CTX.read().unwrap().is_some() {
                    return;
                }

                if IsWindow(hwnd) == 0 {
                    return;
                }

                if !GetParent(hwnd).is_null() {
                    return;
                }

                if let Some(window_text) = OVERLAY_TARGET_WINDOW_TEXT
                    && let Ok(text) = crate::utils::win32::get_window_text(hwnd, false)
                    && window_text != text
                {
                    return;
                }

                if let Some(class_name) = OVERLAY_TARGET_WINDOW_CLASS_NAME
                    && let Ok(class) = crate::utils::win32::get_window_class_name(hwnd, false)
                    && class_name != class
                {
                    return;
                }

                if let Ok(overlay) = create_overlay_window(hwnd) {
                    OVERLAY_CTX.write().unwrap().insert(OverlayContext {
                        target: hwnd as usize,
                        overlay: overlay as usize,
                    });
                }
            }

            EVENT_OBJECT_LOCATIONCHANGE => {
                let Some(OverlayContext { target, overlay }) = *OVERLAY_CTX.read().unwrap() else {
                    return;
                };

                if hwnd as usize != target {
                    return;
                }

                let mut rect = RECT::default();

                if GetWindowRect(hwnd, &mut rect) != 0 {
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;

                    MoveWindow(overlay as HWND, rect.left, rect.top, width, height, 1);
                }
            }

            EVENT_OBJECT_DESTROY => {
                let Some(OverlayContext { target, overlay }) = *OVERLAY_CTX.read().unwrap() else {
                    return;
                };

                if hwnd as usize != target {
                    return;
                }

                DestroyWindow(overlay as HWND);

                OVERLAY_CTX.write().unwrap().take();
            }

            _ => {}
        }
    }
}

/// Overlay 清理函数
pub fn clean_up() {
    let Some(OverlayContext { overlay, .. }) = *OVERLAY_CTX.read().unwrap() else {
        return;
    };

    unsafe { DestroyWindow(overlay as HWND) };

    OVERLAY_CTX.write().unwrap().take();
}
