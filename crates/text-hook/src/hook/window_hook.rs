use translate_macros::{detour, generate_detours};
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{DefWindowProcW, SetWindowTextW, WM_NCCREATE, WM_SETTEXT};

use crate::{constant, debug};

#[generate_detours]
pub trait WindowHook: Send + Sync + 'static {
    #[detour(dll = "user32.dll", symbol = "DefWindowProcA", fallback = "0")]
    unsafe fn def_window_proc(
        &self,
        h_wnd: HWND,
        u_msg: UINT,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match u_msg {
            WM_NCCREATE => unsafe {
                let result = HOOK_DEF_WINDOW_PROC.call(h_wnd, u_msg, w_param, l_param);

                if result != 0 {
                    let mut window_title = constant::WINDOW_TITLE.to_vec();
                    window_title.push(0);
                    SetWindowTextW(h_wnd, window_title.as_ptr());
                }

                result
            },
            WM_SETTEXT => {
                #[cfg(feature = "debug_output")]
                {
                    let raw_title = {
                        let bytes =
                            unsafe { core::ffi::CStr::from_ptr(l_param as *const i8).to_bytes() };
                        let u16_bytes = crate::code_cvt::ansi_to_wide_char(bytes);
                        String::from_utf16_lossy(&u16_bytes)
                    };
                    debug!("Get raw window title: {raw_title}");
                }
                let mut window_title = constant::WINDOW_TITLE.to_vec();
                window_title.push(0);

                unsafe { DefWindowProcW(h_wnd, u_msg, w_param, window_title.as_ptr() as LPARAM) }
            }
            _ => unsafe { HOOK_DEF_WINDOW_PROC.call(h_wnd, u_msg, w_param, l_param) },
        }
    }
}

/// 开启窗口过程相关的钩子
#[allow(dead_code)]
pub fn enable_hooks() {
    #[cfg(feature = "override_window_title")]
    unsafe {
        HOOK_DEF_WINDOW_PROC.enable().unwrap();
    }

    debug!("Window Hooked!");
}

/// 关闭窗口过程相关的钩子
#[allow(dead_code)]
pub fn disable_hooks() {
    #[cfg(feature = "override_window_title")]
    unsafe {
        HOOK_DEF_WINDOW_PROC.disable().unwrap();
    }

    debug!("Window Unhooked!");
}
