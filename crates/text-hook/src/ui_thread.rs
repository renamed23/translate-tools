use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

use crate::hook::{impls::HookImplType, internal_hooks::UiMainTick};

/// 每次调用 [`UiMainTick::on_ui_main_tick`] 返回的动作。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopAction {
    /// 继续消息循环，立即进入下一次迭代。
    #[default]
    Continue,

    /// 退出 UI 线程 —— 触发 [`thread_manager::shutdown`](crate::thread_manager::shutdown)
    /// 以通知所有其他受管线程。
    Exit,
}

/// UI 线程入口点 — Windows 消息处理、Overlay 渲染以及 `WinEvent` 钩子。
pub fn run() {
    #[cfg(feature = "enable_win_event_hook")]
    if let Err(e) = unsafe { crate::win_event_hook::install_win_event_hook() } {
        crate::debug!("Install WinEvent hook failed with {e:?}");
    }

    #[cfg(feature = "enable_overlay")]
    scopeguard::defer!(
        crate::overlay::cleanup();
    );

    #[cfg(feature = "enable_win_event_hook")]
    scopeguard::defer!(
        if let Err(e) = unsafe { crate::win_event_hook::uninstall_win_event_hook() } {
            crate::debug!("Uninstall WinEvent hook failed with {e:?}");
        }
    );

    let mut msg = MSG::default();
    while !crate::thread_manager::is_shutdown() {
        unsafe {
            while PeekMessageW(&raw mut msg, core::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    crate::thread_manager::shutdown();
                    return;
                }
                TranslateMessage(&raw const msg);
                DispatchMessageW(&raw const msg);
            }

            match <HookImplType as UiMainTick>::on_ui_main_tick() {
                Ok(LoopAction::Continue) => {
                    #[cfg(feature = "enable_overlay")]
                    crate::overlay::render();
                }
                Ok(LoopAction::Exit) => {
                    crate::thread_manager::shutdown();
                    break;
                }
                Err(e) => {
                    crate::debug!("on_ui_main_tick failed with {e:?}");
                }
            }
        }
    }
}
