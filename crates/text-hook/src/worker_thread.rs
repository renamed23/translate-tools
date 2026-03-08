use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

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

fn worker_main() {
    #[cfg(feature = "win_event_hook")]
    if let Err(e) = unsafe { crate::win_event_hook::install_win_event_hook() } {
        crate::debug!("Install WinEvent hook failed with {e:?}");
    }

    #[cfg(feature = "win_event_hook")]
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

            std::thread::yield_now();
        }
    }
}
