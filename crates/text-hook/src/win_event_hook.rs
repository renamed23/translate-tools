use windows_sys::Win32::{
    Foundation::HWND,
    System::Threading::GetCurrentProcessId,
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{EVENT_MAX, EVENT_MIN, WINEVENT_OUTOFCONTEXT},
    },
};

use crate::hook::{impls::HookImplType, traits::CoreHook};

static mut WIN_EVENT_HOOK: Option<HWINEVENTHOOK> = None;

/// 安装 WinEvent Hook 处理程序
///
/// # Safety
/// - 必须在 worker thread 刚开始调用，且仅调用一次
/// - 非线程安全，需由调用者保证初始化顺序
pub unsafe fn install_win_event_hook() -> crate::Result<()> {
    crate::debug!("Installing WinEvent hook");

    #[allow(static_mut_refs)]
    if unsafe { WIN_EVENT_HOOK.is_some() } {
        return Ok(());
    }

    let handle = unsafe {
        SetWinEventHook(
            EVENT_MIN,
            EVENT_MAX,
            core::ptr::null_mut(),
            Some(win_event_hook_proc),
            GetCurrentProcessId(),
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };

    if handle.is_null() {
        crate::bail!("SetWinEventHook failed");
    }

    unsafe { WIN_EVENT_HOOK = Some(handle) };

    Ok(())
}

/// 卸载 WinEvent Hook 处理程序
///
/// # Safety
/// - 必须在 worker thread 结束时调用，且仅调用一次
pub unsafe fn uninstall_win_event_hook() -> crate::Result<()> {
    crate::debug!("Uninstalling WinEvent hook");

    unsafe {
        if let Some(handle) = WIN_EVENT_HOOK {
            WIN_EVENT_HOOK = None;
            if UnhookWinEvent(handle) != 0 {
                Ok(())
            } else {
                crate::bail!("UnhookWinEvent failed");
            }
        } else {
            crate::bail!("WinEvent hook is not installed");
        }
    }
}

/// WinEvent 通用回调函数
///
/// 所有 WinEvent 事件都由该函数接收
unsafe extern "system" fn win_event_hook_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    id_event_thread: u32,
    dwms_event_time: u32,
) {
    crate::debug!(raw
        "[WinEvent] Ev: 0x{:X} | HWND: {:?} | ObjID: {} | Child: {}",
        event,
        hwnd,
        id_object,
        id_child
    );

    #[cfg(feature = "overlay")]
    crate::overlay::win_event_callback(
        event,
        hwnd,
        id_object,
        id_child,
        id_event_thread,
        dwms_event_time,
    );

    HookImplType::on_win_event_triggered(
        event,
        hwnd,
        id_object,
        id_child,
        id_event_thread,
        dwms_event_time,
    );
}
