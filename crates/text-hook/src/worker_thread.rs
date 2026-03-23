use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::JoinHandle,
};

use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

use crate::hook::{core_hook::WorkerMainTick, impls::HookImplType};

static STOP_FLAG: AtomicBool = AtomicBool::new(false);
static mut JOIN_HANDLE: Option<JoinHandle<()>> = None;

/// 启动工作线程。
///
/// # Safety
/// - 必须在 DLL attach 时调用，且仅调用一次。
pub unsafe fn start() -> crate::Result<()> {
    crate::debug!("Starting worker thread");

    #[allow(static_mut_refs)]
    if unsafe { JOIN_HANDLE.is_some() } {
        return Ok(());
    }

    STOP_FLAG.store(false, Ordering::Release);

    unsafe { JOIN_HANDLE = Some(std::thread::spawn(worker_main)) };
    Ok(())
}

/// 停止工作线程并等待其安全退出。
///
/// # Safety
/// - 调用者必须保证在调用此函数前，工作线程已通过 `start` 成功启动。
pub unsafe fn stop() -> crate::Result<()> {
    crate::debug!("Stopping worker thread");

    STOP_FLAG.store(true, Ordering::Release);

    #[allow(static_mut_refs)]
    if let Some(handle) = unsafe { JOIN_HANDLE.take() } {
        handle
            .join()
            .map_err(|_| crate::anyhow!("Worker thread panicked"))?;
    } else {
        crate::bail!("Worker thread is not started");
    }

    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopAction {
    /// 继续运行，立即进入下一轮循环
    #[default]
    Continue,

    /// 停止工作线程，触发清理流程并退出 worker_main
    Exit,
}

fn worker_main() {
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
    while !STOP_FLAG.load(Ordering::Acquire) {
        unsafe {
            while PeekMessageW(&mut msg, core::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    return;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            match HookImplType::on_worker_main_tick() {
                Ok(LoopAction::Continue) => {
                    #[cfg(feature = "enable_overlay")]
                    crate::overlay::render();
                }
                Ok(LoopAction::Exit) => {
                    STOP_FLAG.store(true, Ordering::Release);
                    break;
                }
                Err(e) => {
                    crate::debug!("on_worker_main_tick failed with {e:?}");
                }
            };
        }
    }
}
