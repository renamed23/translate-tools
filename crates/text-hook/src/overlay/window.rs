use windows_sys::{
    Win32::{
        Foundation::{
            ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM,
        },
        Graphics::Dwm::DwmExtendFrameIntoClientArea,
        UI::{
            Controls::MARGINS,
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, GetWindowRect, LWA_ALPHA,
                RegisterClassW, SW_SHOWNOACTIVATE, SetLayeredWindowAttributes, ShowWindow,
                WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
                WS_POPUP,
            },
        },
    },
    w,
};

use crate::{
    hook::{impls::HookImplType, traits::CoreHook},
    print_last_error_message,
    utils::raii_wrapper::OwnedHWND,
};

const TEXT_HOOK_OVERLAY_CLASS_NAME: *const u16 = w!("tt_text_hook_overlay_class_name");
const TEXT_HOOK_OVERLAY_TITLE_NAME: *const u16 = w!("tt_text_hook_overlay_title_name");

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match HookImplType::on_overlay_wnd_proc(hwnd, msg, w_param, l_param) {
        Some(ret) => ret,
        None => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}

fn ensure_window_class() -> crate::Result<()> {
    let instance = crate::utils::win32::get_module_handle(core::ptr::null())?;

    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(overlay_wnd_proc),
        hInstance: instance,
        lpszClassName: TEXT_HOOK_OVERLAY_CLASS_NAME,
        ..WNDCLASSW::default()
    };

    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 {
        let err = unsafe { GetLastError() };
        if err != ERROR_CLASS_ALREADY_EXISTS {
            print_last_error_message!(ec err);
            crate::bail!("RegisterClassW failed: {err}");
        }
    }
    Ok(())
}

/// 创建一个overlay窗口
pub(super) fn create_overlay_window(target_hwnd: HWND) -> crate::Result<OwnedHWND> {
    ensure_window_class()?;

    let mut rect = RECT::default();
    if unsafe { GetWindowRect(target_hwnd, &mut rect) } == 0 {
        print_last_error_message!();
        crate::bail!("GetWindowRect failed while create window");
    }

    let width = (rect.right - rect.left).max(1);
    let height = (rect.bottom - rect.top).max(1);

    let instance = crate::utils::win32::get_module_handle(core::ptr::null())?;
    let hwnd_raw = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED,
            TEXT_HOOK_OVERLAY_CLASS_NAME,
            TEXT_HOOK_OVERLAY_TITLE_NAME,
            WS_POPUP,
            rect.left,
            rect.top,
            width,
            height,
            target_hwnd,
            core::ptr::null_mut(),
            instance,
            core::ptr::null(),
        )
    };

    if hwnd_raw.is_null() {
        print_last_error_message!();
        crate::bail!("CreateWindowExW failed");
    }

    let hwnd = OwnedHWND(hwnd_raw);

    if unsafe { SetLayeredWindowAttributes(*hwnd, 0, 255, LWA_ALPHA) } == 0 {
        print_last_error_message!();
        crate::debug!("SetLayeredWindowAttributes failed");
    }

    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };

    let hr = unsafe { DwmExtendFrameIntoClientArea(*hwnd, &margins) };
    if hr < 0 {
        crate::debug!("DwmExtendFrameIntoClientArea failed: hr={hr:#x}");
    }

    unsafe {
        ShowWindow(*hwnd, SW_SHOWNOACTIVATE);
    }

    Ok(hwnd)
}
