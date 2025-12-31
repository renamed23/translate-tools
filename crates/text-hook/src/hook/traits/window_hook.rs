use translate_macros::{detour, detour_trait};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CREATESTRUCTA, CREATESTRUCTW, GetParent, HMENU, MF_BITMAP, MF_OWNERDRAW, MessageBoxW,
        ModifyMenuW, SetDlgItemTextW, WM_NCCREATE, WM_SETTEXT,
    },
};
use windows_sys::core::BOOL;

use crate::{constant::WINDOW_TITLE, debug};

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
                        crate::code_cvt::u16_with_null(WINDOW_TITLE)
                    } else {
                        crate::mapping::map_chars_with_null(text_slice)
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
                        crate::code_cvt::u16_with_null(WINDOW_TITLE)
                    } else {
                        crate::mapping::map_chars_with_null(text_slice)
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

                let title_slice = crate::utils::mem::slice_until_null((*params_w).lpszName, 512);

                #[cfg(feature = "debug_output")]
                {
                    let raw_class = {
                        let class_slice =
                            crate::utils::mem::slice_until_null((*params_w).lpszClass, 512);
                        String::from_utf16_lossy(class_slice)
                    };

                    let raw_title = String::from_utf16_lossy(title_slice);

                    debug!("Get raw class: {raw_class}, raw window title: {raw_title}");
                }

                if (*params_w).hwndParent.is_null() {
                    let mut modified_params: CREATESTRUCTW = core::ptr::read(params_w);
                    let window_title = if cfg!(feature = "override_window_title") {
                        crate::code_cvt::u16_with_null(WINDOW_TITLE)
                    } else {
                        crate::mapping::map_wide_chars_with_null(title_slice)
                    };
                    modified_params.lpszName = window_title.as_ptr();
                    return HOOK_DEF_WINDOW_PROC_W.call(
                        h_wnd,
                        u_msg,
                        w_param,
                        &modified_params as *const _ as LPARAM,
                    );
                }

                HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, l_param)
            },
            WM_SETTEXT => unsafe {
                let text_ptr = l_param as *const u16;
                if text_ptr.is_null() || !GetParent(h_wnd).is_null() {
                    return HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, l_param);
                }

                let text_slice = crate::utils::mem::slice_until_null(l_param as *const u16, 512);

                #[cfg(feature = "debug_output")]
                {
                    let raw_text = { String::from_utf16_lossy(text_slice) };
                    debug!("Get raw window text: {raw_text}");
                }

                if GetParent(h_wnd).is_null() {
                    let text = if cfg!(feature = "override_window_title") {
                        crate::code_cvt::u16_with_null(WINDOW_TITLE)
                    } else {
                        crate::mapping::map_wide_chars_with_null(text_slice)
                    };
                    return HOOK_DEF_WINDOW_PROC_W.call(
                        h_wnd,
                        u_msg,
                        w_param,
                        text.as_ptr() as LPARAM,
                    );
                };

                HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, l_param)
            },
            _ => unsafe { HOOK_DEF_WINDOW_PROC_W.call(h_wnd, u_msg, w_param, l_param) },
        }
    }

    #[detour(dll = "user32.dll", symbol = "ModifyMenuA", fallback = "0")]
    unsafe fn modify_menu_a(
        &self,
        h_menu: HMENU,
        u_position: u32,
        u_flags: u32,
        u_id_new_item: usize,
        lp_new_item: *const u8,
    ) -> BOOL {
        unsafe {
            if (u_flags & (MF_BITMAP | MF_OWNERDRAW)) == 0 && !lp_new_item.is_null() {
                let text_slice = crate::utils::mem::slice_until_null(lp_new_item, 512);
                let wide_text = crate::code_cvt::ansi_to_wide_char_with_null(text_slice);

                #[cfg(feature = "debug_output")]
                {
                    let raw_text = {
                        String::from_utf16_lossy(&crate::code_cvt::ansi_to_wide_char(text_slice))
                    };
                    debug!("Get menu text: {raw_text}");
                }

                ModifyMenuW(
                    h_menu,
                    u_position,
                    u_flags,
                    u_id_new_item,
                    wide_text.as_ptr(),
                )
            } else {
                HOOK_MODIFY_MENU_A.call(h_menu, u_position, u_flags, u_id_new_item, lp_new_item)
            }
        }
    }

    #[detour(dll = "user32.dll", symbol = "MessageBoxA", fallback = "0")]
    unsafe fn message_box_a(
        &self,
        h_wnd: HWND,
        lp_text: *const u8,
        lp_caption: *const u8,
        u_type: u32,
    ) -> i32 {
        unsafe {
            if lp_text.is_null() && lp_caption.is_null() {
                return HOOK_MESSAGE_BOX_A.call(h_wnd, lp_text, lp_caption, u_type);
            }

            let wide_text_opt = if lp_text.is_null() {
                None
            } else {
                let text_slice = crate::utils::mem::slice_until_null(lp_text, 2048);
                Some(crate::code_cvt::ansi_to_wide_char_with_null(text_slice))
            };

            let wide_caption_opt = if lp_caption.is_null() {
                None
            } else {
                let cap_slice = crate::utils::mem::slice_until_null(lp_caption, 512);
                Some(crate::code_cvt::ansi_to_wide_char_with_null(cap_slice))
            };

            let wide_text_ptr = wide_text_opt
                .as_ref()
                .map_or(core::ptr::null(), |v| v.as_ptr());
            let wide_caption_ptr = wide_caption_opt
                .as_ref()
                .map_or(core::ptr::null(), |v| v.as_ptr());

            #[cfg(feature = "debug_output")]
            {
                if let Some(ref w) = wide_text_opt {
                    let s = String::from_utf16_lossy(&w[..w.len().saturating_sub(1)]);
                    debug!("Get message box text: {s}");
                }
                if let Some(ref c) = wide_caption_opt {
                    let s = String::from_utf16_lossy(&c[..c.len().saturating_sub(1)]);
                    debug!("Get message box caption: {s}");
                }
            }

            MessageBoxW(h_wnd, wide_text_ptr, wide_caption_ptr, u_type)
        }
    }

    #[detour(dll = "user32.dll", symbol = "SetDlgItemTextA", fallback = "0")]
    unsafe fn set_dlg_item_text_a(
        &self,
        h_dlg: HWND,
        n_id_dlg_item: i32,
        lp_string: *const u8,
    ) -> BOOL {
        unsafe {
            if lp_string.is_null() {
                return HOOK_SET_DLG_ITEM_TEXT_A.call(h_dlg, n_id_dlg_item, lp_string);
            }

            let text_slice = crate::utils::mem::slice_until_null(lp_string, 512);
            let wide_text = crate::code_cvt::ansi_to_wide_char_with_null(text_slice);

            #[cfg(feature = "debug_output")]
            {
                let raw_text = String::from_utf16_lossy(&wide_text);
                debug!("Get SetDlgItemTextA text: {raw_text}");
            }

            SetDlgItemTextW(h_dlg, n_id_dlg_item, wide_text.as_ptr())
        }
    }

    #[detour(dll = "user32.dll", symbol = "SetWindowTextA", fallback = "0")]
    unsafe fn set_window_text_a(&self, h_wnd: HWND, lp_string: *const u8) -> BOOL {
        unsafe {
            if lp_string.is_null() {
                return HOOK_SET_WINDOW_TEXT_A.call(h_wnd, lp_string);
            }

            // 将 ANSI 文本转换为宽字符，再转换为本地编码
            let text_slice = crate::utils::mem::slice_until_null(lp_string, 512);
            let wide_text = crate::code_cvt::ansi_to_wide_char(text_slice);
            let ansi_text = crate::code_cvt::wide_char_to_multi_byte_with_null(&wide_text, 0);

            #[cfg(feature = "debug_output")]
            {
                let raw_text = String::from_utf16_lossy(&wide_text);
                debug!("Get SetWindowTextA text: {raw_text}");
            }

            HOOK_SET_WINDOW_TEXT_A.call(h_wnd, ansi_text.as_ptr())
        }
    }

    #[detour(dll = "user32.dll", symbol = "SendMessageA", fallback = "0")]
    unsafe fn send_message_a(
        &self,
        h_wnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        #[cfg(not(feature = "text_patch"))]
        unimplemented!();

        #[cfg(feature = "text_patch")]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;

            if crate::utils::win32::needs_text_conversion(msg) && l_param != 0 {
                let text_slice = crate::utils::mem::slice_until_null(l_param as *const u8, 1024);
                let wide_text = crate::code_cvt::ansi_to_wide_char(text_slice);

                if !wide_text.contains(&0xFFFDu16) {
                    let msg_text = String::from_utf8_unchecked(
                        crate::code_cvt::wide_char_to_utf8(&wide_text).to_vec(),
                    );

                    #[cfg(feature = "debug_output")]
                    debug!("SendMessageA (msg={:#x}) text: {}", msg, msg_text);

                    let opt_trans_msg = crate::text_patch::process_message(&msg_text);

                    #[cfg(not(feature = "text_extracting"))]
                    if let Some(trans_msg) = opt_trans_msg {
                        let trans_msg_u16 =
                            crate::code_cvt::utf8_to_wide_char_with_null(trans_msg.as_bytes());
                        return SendMessageW(h_wnd, msg, w_param, trans_msg_u16.as_ptr() as LPARAM);
                    }
                }
            }
            HOOK_SEND_MESSAGE_A.call(h_wnd, msg, w_param, l_param)
        }
    }
}

/// 开启窗口过程相关的特性钩子
#[allow(dead_code)]
pub fn enable_featured_hooks() {
    unsafe {
        HOOK_DEF_WINDOW_PROC_A.enable().unwrap();
        HOOK_DEF_WINDOW_PROC_W.enable().unwrap();
        HOOK_MODIFY_MENU_A.enable().unwrap();
        HOOK_MESSAGE_BOX_A.enable().unwrap();
        HOOK_SET_DLG_ITEM_TEXT_A.enable().unwrap();
        HOOK_SET_WINDOW_TEXT_A.enable().unwrap();

        #[cfg(feature = "text_patch")]
        {
            HOOK_SEND_MESSAGE_A.enable().unwrap();
        }
    }

    debug!("Window Hooked!");
}

/// 关闭窗口过程相关的特性钩子
#[allow(dead_code)]
pub fn disable_featured_hooks() {
    unsafe {
        HOOK_DEF_WINDOW_PROC_A.disable().unwrap();
        HOOK_DEF_WINDOW_PROC_W.disable().unwrap();
        HOOK_MODIFY_MENU_A.disable().unwrap();
        HOOK_MESSAGE_BOX_A.disable().unwrap();
        HOOK_SET_DLG_ITEM_TEXT_A.disable().unwrap();
        HOOK_SET_WINDOW_TEXT_A.disable().unwrap();

        #[cfg(feature = "text_patch")]
        {
            HOOK_SEND_MESSAGE_A.disable().unwrap();
        }
    }

    debug!("Window Unhooked!");
}
