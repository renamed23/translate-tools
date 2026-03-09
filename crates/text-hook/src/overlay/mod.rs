mod window;

use std::cell::RefCell;

use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{
        EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW, GetParent,
        GetWindowRect, IsWindow, MoveWindow, OBJID_WINDOW,
    },
};

#[cfg(feature = "overlay_gl")]
use crate::utils::gl::GLContext;
use crate::{
    constant::{OVERLAY_TARGET_WINDOW_CLASS_NAME, OVERLAY_TARGET_WINDOW_TEXT},
    hook::{impls::HookImplType, traits::CoreHook},
    overlay::window::create_overlay_window,
    utils::raii_wrapper::OwnedHWND,
};

/// Overlay上下文结构体
pub struct OverlayContext {
    /// OpenGL 上下文
    #[cfg(feature = "overlay_gl")]
    pub gl_ctx: GLContext,

    /// 目标窗口 hwnd
    pub target: HWND,

    /// Overlay窗口 hwnd
    pub overlay: OwnedHWND,
}

thread_local! {
    /// Overlay上下文
    pub static OVERLAY_CTX: RefCell<Option<OverlayContext>> = const { RefCell::new(None) };
}

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
                if OVERLAY_CTX.with_borrow(|ctx| ctx.is_some()) {
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
                    #[cfg(feature = "overlay_gl")]
                    let Ok(gl_ctx) = GLContext::new(*overlay) else {
                        return;
                    };

                    OVERLAY_CTX.set(Some(OverlayContext {
                        #[cfg(feature = "overlay_gl")]
                        gl_ctx,
                        target: hwnd,
                        overlay,
                    }));
                }
            }

            EVENT_OBJECT_LOCATIONCHANGE => {
                let Some((target, overlay)) = OVERLAY_CTX
                    .with_borrow(|ctx| ctx.as_ref().map(|ctx| (ctx.target, *ctx.overlay)))
                else {
                    return;
                };

                if hwnd != target {
                    return;
                }

                let mut rect = RECT::default();

                if GetWindowRect(hwnd, &mut rect) != 0 {
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;

                    MoveWindow(overlay, rect.left, rect.top, width, height, 1);
                }
            }

            EVENT_OBJECT_DESTROY => {
                let Some(target) =
                    OVERLAY_CTX.with_borrow(|ctx| ctx.as_ref().map(|ctx| ctx.target))
                else {
                    return;
                };

                if hwnd != target {
                    return;
                }

                OVERLAY_CTX.with(|ctx| ctx.take());
            }

            _ => {}
        }
    }
}

/// Overlay 渲染函数
pub fn render() {
    OVERLAY_CTX.with_borrow(|ctx| {
        if let Some(context) = ctx {
            HookImplType::on_overlay_render(context);
        }
    });
}

/// Overlay 清理函数
pub fn clean_up() {
    OVERLAY_CTX.with(|ctx| ctx.take());
}
