use cfg_if::cfg_if;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{CREATESTRUCTA, CREATESTRUCTW, GetParent, WM_NCCREATE, WM_SETTEXT},
};

use crate::{
    hook::api_hooks::windowing::{DefWindowProc, HOOK_DEF_WINDOW_PROC_A, HOOK_DEF_WINDOW_PROC_W},
    utils::exts::{
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[allow(dead_code)]
pub struct WindowTitleOverrider;

cfg_if! {
    if #[cfg(feature = "bind_window_title_overrider")] {
        type WindowTitleOverriderSlot = crate::hook::impls::HookImplType;
    } else {
        type WindowTitleOverriderSlot = WindowTitleOverrider;
    }
}

impl DefWindowProc for WindowTitleOverriderSlot {
    unsafe fn def_window_proc_a(
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match u_msg {
            WM_NCCREATE => unsafe {
                let params_a = l_param as *const CREATESTRUCTA;
                if params_a.is_null() || !(*params_a).hwndParent.is_null() {
                    return crate::call!(HOOK_DEF_WINDOW_PROC_A, h_wnd, u_msg, w_param, l_param);
                }

                let mut params_w: CREATESTRUCTW = core::mem::zeroed();

                debug_assert_eq!(
                    core::mem::size_of::<CREATESTRUCTA>(),
                    core::mem::size_of::<CREATESTRUCTW>()
                );

                core::ptr::copy_nonoverlapping(
                    params_a.cast::<u8>(),
                    (&raw mut params_w).cast::<u8>(),
                    core::mem::size_of::<CREATESTRUCTW>(),
                );

                let class_bytes = (*params_a).lpszClass.to_slice_until_null_scan();
                let class_name = class_bytes.to_wide_null_ansi();

                let _title_slice = (*params_a).lpszName.to_slice_until_null_scan();

                cfg_if!(
                    if #[cfg(feature = "enable_window_title_override")] {
                       let window_title = crate::constant::WINDOW_TITLE.with_null();
                    } else {
                       let window_title = _title_slice.to_wide_ansi().mapping_null();
                    }
                );

                params_w.lpszClass = class_name.as_ptr();
                params_w.lpszName = window_title.as_ptr();

                #[cfg(feature = "enable_debug_output")]
                {
                    let raw_class = class_name.to_string_lossy();
                    let raw_title = _title_slice.to_wide_ansi().to_string_lossy();
                    crate::debug!("Get raw class: {raw_class}, raw window title: {raw_title}");
                }

                crate::call!(
                    HOOK_DEF_WINDOW_PROC_W,
                    h_wnd,
                    u_msg,
                    w_param,
                    &raw const params_w as LPARAM
                )
            },
            WM_SETTEXT => unsafe {
                let text_ptr = l_param as *const u8;
                if text_ptr.is_null() || !GetParent(h_wnd).is_null() {
                    return crate::call!(HOOK_DEF_WINDOW_PROC_A, h_wnd, u_msg, w_param, l_param);
                }

                let _text_slice = text_ptr.to_slice_until_null_scan();

                cfg_if!(
                    if #[cfg(feature = "enable_window_title_override")] {
                        let text = crate::constant::WINDOW_TITLE.with_null();
                    } else {
                        let text = _text_slice.to_wide_ansi().mapping_null();
                    }
                );

                #[cfg(feature = "enable_debug_output")]
                {
                    let raw_text = _text_slice.to_wide_ansi().to_string_lossy();
                    crate::debug!("Get raw window text: {raw_text}");
                }

                crate::call!(
                    HOOK_DEF_WINDOW_PROC_W,
                    h_wnd,
                    u_msg,
                    w_param,
                    text.as_ptr() as LPARAM
                )
            },
            _ => unsafe { crate::call!(HOOK_DEF_WINDOW_PROC_A, h_wnd, u_msg, w_param, l_param) },
        }
    }

    unsafe fn def_window_proc_w(
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match u_msg {
            WM_NCCREATE => unsafe {
                let params_w = l_param as *const CREATESTRUCTW;
                if params_w.is_null() || !(*params_w).hwndParent.is_null() {
                    return crate::call!(HOOK_DEF_WINDOW_PROC_W, h_wnd, u_msg, w_param, l_param);
                }

                let _title_slice = (*params_w).lpszName.to_slice_until_null_scan();

                #[cfg(feature = "enable_debug_output")]
                {
                    let raw_class = (*params_w)
                        .lpszClass
                        .to_slice_until_null_scan()
                        .to_string_lossy();

                    let raw_title = _title_slice.to_string_lossy();

                    crate::debug!("Get raw class: {raw_class}, raw window title: {raw_title}");
                }

                let mut modified_params: CREATESTRUCTW = core::ptr::read(params_w);

                cfg_if!(
                    if #[cfg(feature = "enable_window_title_override")] {
                        let window_title = crate::constant::WINDOW_TITLE.with_null();
                    } else {
                        let window_title = _title_slice.mapping_null();
                    }
                );

                modified_params.lpszName = window_title.as_ptr();

                crate::call!(
                    HOOK_DEF_WINDOW_PROC_W,
                    h_wnd,
                    u_msg,
                    w_param,
                    &raw const modified_params as LPARAM
                )
            },
            WM_SETTEXT => unsafe {
                let text_ptr = l_param as *const u16;
                if text_ptr.is_null() || !GetParent(h_wnd).is_null() {
                    return crate::call!(HOOK_DEF_WINDOW_PROC_W, h_wnd, u_msg, w_param, l_param);
                }

                let _text_slice = text_ptr.to_slice_until_null_scan();

                #[cfg(feature = "enable_debug_output")]
                {
                    let raw_text = _text_slice.to_string_lossy();
                    crate::debug!("Get raw window text: {raw_text}");
                }

                cfg_if!(
                    if #[cfg(feature = "enable_window_title_override")] {
                        let text = crate::constant::WINDOW_TITLE.with_null();
                    } else {
                        let text = _text_slice.mapping_null();
                    }
                );

                crate::call!(
                    HOOK_DEF_WINDOW_PROC_W,
                    h_wnd,
                    u_msg,
                    w_param,
                    text.as_ptr() as LPARAM
                )
            },
            _ => unsafe { crate::call!(HOOK_DEF_WINDOW_PROC_W, h_wnd, u_msg, w_param, l_param) },
        }
    }
}
