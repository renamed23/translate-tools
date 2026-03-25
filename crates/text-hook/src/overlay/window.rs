use windows_sys::{
    Win32::{
        Foundation::{
            ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, RECT, SetLastError,
            WPARAM,
        },
        Graphics::Dwm::DwmExtendFrameIntoClientArea,
        UI::{
            Controls::MARGINS,
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, GWL_EXSTYLE,
                GetWindowLongPtrW, GetWindowRect, LWA_ALPHA, RegisterClassW, SW_SHOW,
                SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetLayeredWindowAttributes,
                SetWindowLongPtrW, SetWindowPos, ShowWindow, WNDCLASSW, WS_EX_LAYERED,
                WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
            },
        },
    },
    w,
};

use crate::{
    hook::{impls::HookImplType, internal_hooks::OverlayWndProc},
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
    match <HookImplType as OverlayWndProc>::on_overlay_wnd_proc(hwnd, msg, w_param, l_param) {
        Ok(Some(ret)) => ret,
        Ok(None) => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
        Err(e) => {
            crate::debug!("on_overlay_wnd_proc failed with {e:?}");
            unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
        }
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
            WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_LAYERED,
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
        ShowWindow(*hwnd, SW_SHOW);
    }

    Ok(hwnd)
}

/// 更新 overlay 窗口的点击穿透状态。
///
/// 该函数会切换窗口扩展样式中的 `WS_EX_TRANSPARENT` 标志，
/// 并通过 `SetWindowPos(..., SWP_FRAMECHANGED)` 立即让样式变更生效。
pub(super) fn set_overlay_click_through(hwnd: HWND, click_through: bool) -> crate::Result<()> {
    unsafe {
        SetLastError(0);
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as isize;
        if ex_style == 0 {
            let err = GetLastError();
            if err != 0 {
                print_last_error_message!(ec err);
                crate::bail!("GetWindowLongPtrW failed while query overlay ex style: {err}");
            }
        }

        let has_click_through = (ex_style & WS_EX_TRANSPARENT as isize) != 0;
        if has_click_through == click_through {
            return Ok(());
        }

        let new_style = if click_through {
            ex_style | WS_EX_TRANSPARENT as isize
        } else {
            ex_style & !(WS_EX_TRANSPARENT as isize)
        };

        SetLastError(0);
        let prev = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style as _);
        if prev == 0 {
            let err = GetLastError();
            if err != 0 {
                print_last_error_message!(ec err);
                crate::bail!("SetWindowLongPtrW failed while update overlay ex style: {err}");
            }
        }

        if SetWindowPos(
            hwnd,
            core::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        ) == 0
        {
            print_last_error_message!();
            crate::bail!("SetWindowPos failed while update overlay ex style");
        }
    }

    Ok(())
}
