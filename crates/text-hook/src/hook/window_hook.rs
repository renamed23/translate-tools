use translate_macros::{detour, generate_detours};
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{CREATESTRUCTA, CREATESTRUCTW, WM_NCCREATE, WM_SETTEXT};

use crate::{constant, debug};

#[generate_detours]
pub trait WindowHook: Send + Sync + 'static {
    #[detour(dll = "user32.dll", symbol = "DefWindowProcA", fallback = "0")]
    unsafe fn def_window_proc_a(
        &self,
        h_wnd: HWND,
        u_msg: UINT,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match u_msg {
            WM_NCCREATE => unsafe {
                let params_a = l_param as *const CREATESTRUCTA;
                let mut params_w: CREATESTRUCTW = std::mem::zeroed();

                std::ptr::copy_nonoverlapping(
                    params_a as *const u8,
                    &mut params_w as *mut _ as *mut u8,
                    std::mem::size_of::<CREATESTRUCTW>(),
                );

                let class_bytes =
                    crate::utils::slice_until_null((*params_a).lpszClass as *const u8, 512);
                let class_name =
                    crate::code_cvt::ansi_to_wide_char_with_null(class_bytes).into_boxed_slice();

                let window_title =
                    crate::utils::u16_with_null(constant::WINDOW_TITLE).into_boxed_slice();

                params_w.lpszClass = class_name.as_ptr();
                params_w.lpszName = window_title.as_ptr();

                #[cfg(feature = "debug_output")]
                {
                    let raw_class = String::from_utf16_lossy(&class_name);

                    let title_bytes =
                        crate::utils::slice_until_null((*params_a).lpszName as *const u8, 512);
                    let raw_title =
                        String::from_utf16_lossy(&crate::code_cvt::ansi_to_wide_char(title_bytes));
                    debug!("Get raw class: {raw_class}, raw window title: {raw_title}");
                }

                Box::leak(class_name);
                Box::leak(window_title);

                HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, &params_w as *const _ as LPARAM)
            },
            WM_SETTEXT => {
                #[cfg(feature = "debug_output")]
                {
                    let raw_title = {
                        let bytes =
                            unsafe { crate::utils::slice_until_null(l_param as *const u8, 512) };
                        let u16_bytes = crate::code_cvt::ansi_to_wide_char(bytes);
                        String::from_utf16_lossy(&u16_bytes)
                    };
                    debug!("Get raw window title: {raw_title}");
                }

                let window_title = crate::utils::u16_with_null(constant::WINDOW_TITLE);
                unsafe {
                    HOOK_DEF_WINDOW_PROC_W.call(
                        h_wnd,
                        u_msg,
                        w_param,
                        window_title.as_ptr() as LPARAM,
                    )
                }
            }
            _ => unsafe { HOOK_DEF_WINDOW_PROC_A.call(h_wnd, u_msg, w_param, l_param) },
        }
    }

    #[detour(dll = "user32.dll", symbol = "DefWindowProcW", fallback = "0")]
    unsafe fn def_window_proc_w(
        &self,
        h_wnd: HWND,
        u_msg: UINT,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match u_msg {
            WM_NCCREATE => unsafe {
                let params_w = l_param as *const CREATESTRUCTW;
                let mut modified_params: CREATESTRUCTW = std::ptr::read(params_w);

                let window_title =
                    crate::utils::u16_with_null(constant::WINDOW_TITLE).into_boxed_slice();
                modified_params.lpszName = window_title.as_ptr();

                Box::leak(window_title);

                HOOK_DEF_WINDOW_PROC_W.call(
                    h_wnd,
                    u_msg,
                    w_param,
                    &modified_params as *const _ as LPARAM,
                )
            },
            WM_SETTEXT => {
                let window_title = crate::utils::u16_with_null(constant::WINDOW_TITLE);
                unsafe {
                    HOOK_DEF_WINDOW_PROC_W.call(
                        h_wnd,
                        u_msg,
                        w_param,
                        window_title.as_ptr() as LPARAM,
                    )
                }
            }
            _ => unsafe { HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, l_param) },
        }
    }
}

/// 开启窗口过程相关的特性钩子
#[allow(dead_code)]
pub fn enable_featured_hooks() {
    #[cfg(feature = "override_window_title")]
    unsafe {
        HOOK_DEF_WINDOW_PROC_A.enable().unwrap();
        HOOK_DEF_WINDOW_PROC_W.enable().unwrap();
    }

    debug!("Window Hooked!");
}

/// 关闭窗口过程相关的特性钩子
#[allow(dead_code)]
pub fn disable_featured_hooks() {
    #[cfg(feature = "override_window_title")]
    unsafe {
        HOOK_DEF_WINDOW_PROC_A.disable().unwrap();
        HOOK_DEF_WINDOW_PROC_W.disable().unwrap();
    }

    debug!("Window Unhooked!");
}
