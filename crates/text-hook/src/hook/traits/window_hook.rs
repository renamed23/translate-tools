use translate_macros::{detour, detour_trait};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{CREATESTRUCTA, CREATESTRUCTW, GetParent, WM_NCCREATE, WM_SETTEXT},
};

use crate::{constant, debug};

#[detour_trait]
pub trait WindowHook: Send + Sync + 'static {
    #[detour(dll = "user32.dll", symbol = "DefWindowProcA", fallback = "0")]
    unsafe fn def_window_proc_a(
        &self,
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match u_msg {
            WM_NCCREATE => unsafe {
                let params_a = l_param as *const CREATESTRUCTA;
                if params_a.is_null() {
                    return HOOK_DEF_WINDOW_PROC_A.call(h_wnd, u_msg, w_param, l_param);
                }

                let mut params_w: CREATESTRUCTW = core::mem::zeroed();

                core::ptr::copy_nonoverlapping(
                    params_a as *const u8,
                    &mut params_w as *mut _ as *mut u8,
                    core::mem::size_of::<CREATESTRUCTW>(),
                );

                let class_bytes = crate::utils::mem::slice_until_null((*params_a).lpszClass, 512);
                let class_name = crate::code_cvt::ansi_to_wide_char_with_null(class_bytes);

                let text_slice = crate::utils::mem::slice_until_null((*params_a).lpszName, 512);

                let window_title = if (*params_a).hwndParent.is_null() {
                    if cfg!(feature = "override_window_title") {
                        crate::code_cvt::u16_with_null(constant::WINDOW_TITLE)
                    } else {
                        crate::mapping::map_chars_to_vec_with_null(text_slice)
                    }
                } else {
                    crate::code_cvt::ansi_to_wide_char_with_null(text_slice)
                };

                params_w.lpszClass = class_name.as_ptr();
                params_w.lpszName = window_title.as_ptr();

                #[cfg(feature = "debug_output")]
                {
                    let raw_class = String::from_utf16_lossy(&class_name);
                    let raw_title =
                        String::from_utf16_lossy(&crate::code_cvt::ansi_to_wide_char(text_slice));
                    debug!("Get raw class: {raw_class}, raw window title: {raw_title}");
                }

                HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, &params_w as *const _ as LPARAM)
            },
            WM_SETTEXT => unsafe {
                let text_ptr = l_param as *const u8;
                if text_ptr.is_null() {
                    return HOOK_DEF_WINDOW_PROC_A.call(h_wnd, u_msg, w_param, l_param);
                }

                let text_slice = crate::utils::mem::slice_until_null(text_ptr, 512);

                let text = if GetParent(h_wnd).is_null() {
                    if cfg!(feature = "override_window_title") {
                        crate::code_cvt::u16_with_null(constant::WINDOW_TITLE)
                    } else {
                        crate::mapping::map_chars_to_vec_with_null(text_slice)
                    }
                } else {
                    crate::code_cvt::ansi_to_wide_char_with_null(text_slice)
                };

                #[cfg(feature = "debug_output")]
                {
                    let raw_text = {
                        let u16_bytes = crate::code_cvt::ansi_to_wide_char(text_slice);
                        String::from_utf16_lossy(&u16_bytes)
                    };
                    debug!("Get raw window text: {raw_text}");
                }

                HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, text.as_ptr() as LPARAM)
            },
            _ => unsafe { HOOK_DEF_WINDOW_PROC_A.call(h_wnd, u_msg, w_param, l_param) },
        }
    }

    #[detour(dll = "user32.dll", symbol = "DefWindowProcW", fallback = "0")]
    unsafe fn def_window_proc_w(
        &self,
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match u_msg {
            WM_NCCREATE => unsafe {
                let params_w = l_param as *const CREATESTRUCTW;
                if params_w.is_null() || !(*params_w).hwndParent.is_null() {
                    return HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, l_param);
                }

                let mut modified_params: CREATESTRUCTW = core::ptr::read(params_w);

                let window_title = crate::code_cvt::u16_with_null(constant::WINDOW_TITLE);
                modified_params.lpszName = window_title.as_ptr();

                #[cfg(feature = "debug_output")]
                {
                    let raw_class = {
                        let class_slice =
                            crate::utils::mem::slice_until_null((*params_w).lpszClass, 512);
                        String::from_utf16_lossy(class_slice)
                    };

                    let raw_title = {
                        let title_slice =
                            crate::utils::mem::slice_until_null((*params_w).lpszName, 512);
                        String::from_utf16_lossy(title_slice)
                    };

                    debug!("Get raw class: {raw_class}, raw window title: {raw_title}");
                }

                HOOK_DEF_WINDOW_PROC_W.call(
                    h_wnd,
                    u_msg,
                    w_param,
                    &modified_params as *const _ as LPARAM,
                )
            },
            WM_SETTEXT => {
                unsafe {
                    let text_ptr = l_param as *const u16;
                    if text_ptr.is_null() || !GetParent(h_wnd).is_null() {
                        return HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, l_param);
                    }
                }

                #[cfg(feature = "debug_output")]
                {
                    let raw_title = {
                        let title_slice = unsafe {
                            crate::utils::mem::slice_until_null(l_param as *const u16, 512)
                        };
                        String::from_utf16_lossy(title_slice)
                    };
                    debug!("Get raw window title: {raw_title}");
                }

                let window_title = crate::code_cvt::u16_with_null(constant::WINDOW_TITLE);
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
    unsafe {
        HOOK_DEF_WINDOW_PROC_A.enable().unwrap();
        HOOK_DEF_WINDOW_PROC_W.enable().unwrap();
    }

    debug!("Window Hooked!");
}

/// 关闭窗口过程相关的特性钩子
#[allow(dead_code)]
pub fn disable_featured_hooks() {
    unsafe {
        HOOK_DEF_WINDOW_PROC_A.disable().unwrap();
        HOOK_DEF_WINDOW_PROC_W.disable().unwrap();
    }

    debug!("Window Unhooked!");
}
