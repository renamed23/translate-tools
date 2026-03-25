#[cfg(feature = "enable_overlay_egui")]
pub(crate) mod egui_integration;
pub(crate) mod window;

use std::cell::RefCell;

use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{
        EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW, GetParent,
        GetWindowRect, IsWindow, OBJID_WINDOW, SWP_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOZORDER,
        SetWindowPos,
    },
};

#[cfg(feature = "enable_overlay_gl")]
use crate::gl::GLContext;
#[cfg(feature = "enable_overlay_gl_painter")]
use crate::gl::painter::GLPainter;
#[cfg(feature = "enable_overlay_egui")]
use crate::overlay::egui_integration::EguiOverlayState;
use crate::{
    constant::{OVERLAY_TARGET_WINDOW_CLASS_NAME, OVERLAY_TARGET_WINDOW_TEXT},
    hook::{impls::HookImplType, internal_hooks::OverlayRender},
    overlay::window::create_overlay_window,
    utils::raii_wrapper::OwnedHWND,
};

/// Overlay上下文结构体
pub struct OverlayContext {
    /// egui overlay 状态
    #[cfg(feature = "enable_overlay_egui")]
    pub egui: EguiOverlayState,

    /// OpenGL 轻量级绘制器
    #[cfg(feature = "enable_overlay_gl_painter")]
    pub gl_painter: GLPainter,

    /// OpenGL 上下文
    #[cfg(feature = "enable_overlay_gl")]
    pub gl_ctx: GLContext,

    /// 目标窗口 hwnd
    pub target: HWND,

    /// Overlay窗口 hwnd
    pub overlay: OwnedHWND,
}

thread_local! {
    /// Overlay上下文
    static OVERLAY_CTX: RefCell<Option<OverlayContext>> = const { RefCell::new(None) };
}

/// 以只读方式访问当前线程上的 overlay 上下文。
///
/// 当 overlay 尚未初始化，或当前线程并不持有对应的 overlay 上下文时，
/// 返回 `Err`。
///
/// 该接口仅提供共享借用，不会把上下文从 thread-local 槽位中取出，因此适合
/// 只读查询场景。
#[allow(dead_code)]
pub fn with_overlay_context<R>(f: impl FnOnce(&OverlayContext) -> R) -> crate::Result<R> {
    OVERLAY_CTX.with_borrow(|ctx| {
        let Some(ctx) = ctx.as_ref() else {
            crate::bail!("overlay context is unavailable");
        };

        Ok(f(ctx))
    })
}

/// 以独占可变方式访问当前线程上的 overlay 上下文。
///
/// 此函数会先把上下文从 thread-local 槽位中 `take()` 出来，再把 `&mut OverlayContext`
/// 传给调用方回调；回调返回后，无论结果是 `Ok` 还是 `Err`，都会把上下文重新放回槽位。
///
/// 如果在调用开始时上下文不存在，或者同一线程上已经有外层逻辑把上下文 `take()` 走了
/// （即发生了重入访问），则直接返回 `Err`。
///
/// 这个语义是刻意的：它避免了 `RefCell` 运行时借用冲突，也避免在重入路径中拿到第二个
/// 可变借用。
pub fn with_overlay_context_mut<R>(
    f: impl FnOnce(&mut OverlayContext) -> crate::Result<R>,
) -> crate::Result<R> {
    OVERLAY_CTX.with(|ctx| {
        let Some(mut context) = ctx.take() else {
            crate::bail!("overlay context is unavailable");
        };

        let result = f(&mut context);

        ctx.replace(Some(context));

        result
    })
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
                    crate::debug!("Initialize overlay context");

                    #[cfg(feature = "enable_overlay_gl")]
                    let Ok(gl_ctx) = GLContext::new(*overlay) else {
                        return;
                    };

                    #[cfg(feature = "enable_overlay_gl_painter")]
                    let Ok(gl_painter) = GLPainter::new(gl_ctx.gl.clone()) else {
                        return;
                    };

                    #[cfg(feature = "enable_overlay_egui")]
                    let Ok(egui) = EguiOverlayState::new(gl_ctx.gl.clone()) else {
                        return;
                    };

                    crate::debug!("Initialize overlay context finished");

                    OVERLAY_CTX.set(Some(OverlayContext {
                        #[cfg(feature = "enable_overlay_egui")]
                        egui,
                        #[cfg(feature = "enable_overlay_gl_painter")]
                        gl_painter,
                        #[cfg(feature = "enable_overlay_gl")]
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

                // 因为overlay的owner被设为了目标窗口（本身是POPUP）
                // 所以不需要处理overlay的Z-Order
                if GetWindowRect(hwnd, &mut rect) != 0 {
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;

                    SetWindowPos(
                        overlay,
                        core::ptr::null_mut(),
                        rect.left,
                        rect.top,
                        width,
                        height,
                        SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS,
                    );
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

                crate::debug!("Desotry overlay context");

                OVERLAY_CTX.with(|ctx| ctx.take());
            }

            _ => {}
        }
    }
}

/// Overlay 渲染函数
pub fn render() {
    if let Err(e) = with_overlay_context_mut(|context| {
        <HookImplType as OverlayRender>::on_overlay_render(context)
    }) {
        crate::debug!("on_overlay_render failed with {e:?}");
    }
}

/// Overlay 清理函数
///
/// 清除当前线程上保存的 overlay 上下文，触发其资源释放。
pub fn cleanup() {
    OVERLAY_CTX.with(|ctx| ctx.take());
}
