use cfg_if::cfg_if;

use windows_sys::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::HMENU,
    },
    core::BOOL,
};

use crate::{
    hook::api_hooks::windowing::{
        AppendMenu, HOOK_APPEND_MENU_A, HOOK_MESSAGE_BOX_A, HOOK_MODIFY_MENU_A,
        HOOK_PROPERTY_SHEET_A, HOOK_SEND_MESSAGE_A, HOOK_SET_DLG_ITEM_TEXT_A,
        HOOK_SET_WINDOW_TEXT_A, MessageBox, ModifyMenu, PropertySheet, SendMessage, SetDlgItemText,
        SetWindowText,
    },
    utils::exts::{
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[allow(dead_code)]
pub struct UserInterfacePatcher;

cfg_if! {
    if #[cfg(feature = "bind_user_interface_patcher")] {
        type UserInterfacePatcherSlot = crate::hook::impls::HookImplType;
    } else {
        type UserInterfacePatcherSlot = UserInterfacePatcher;
    }
}

impl ModifyMenu for UserInterfacePatcherSlot {
    unsafe fn modify_menu_a(
        h_menu: HMENU,
        u_position: u32,
        u_flags: u32,
        u_id_new_item: usize,
        lp_new_item: *const u8,
    ) -> BOOL {
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MF_BITMAP, MF_OWNERDRAW};

            if (u_flags & (MF_BITMAP | MF_OWNERDRAW)) == 0 && !lp_new_item.is_null() {
                let text_slice = lp_new_item.to_slice_until_null_scan();

                #[cfg(feature = "enable_debug_output")]
                {
                    let raw_text = text_slice.to_wide_ansi().to_string_lossy();
                    crate::debug!("Get menu text: {raw_text}");
                }

                let _opt_trans_msg = text_slice.to_wide_ansi().lookup_or_store_null();

                #[cfg(not(feature = "extract_text"))]
                if let Ok(Some(trans_msg)) = _opt_trans_msg {
                    use windows_sys::Win32::UI::WindowsAndMessaging::ModifyMenuW;
                    return ModifyMenuW(
                        h_menu,
                        u_position,
                        u_flags,
                        u_id_new_item,
                        trans_msg.as_ptr(),
                    );
                }
            }

            crate::call!(
                HOOK_MODIFY_MENU_A,
                h_menu,
                u_position,
                u_flags,
                u_id_new_item,
                lp_new_item
            )
        }
    }
}

impl AppendMenu for UserInterfacePatcherSlot {
    unsafe fn append_menu_a(
        h_menu: HMENU,
        u_flags: u32,
        u_id_new_item: usize,
        lp_new_item: *const u8,
    ) -> BOOL {
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MF_BITMAP, MF_OWNERDRAW};

            if (u_flags & (MF_BITMAP | MF_OWNERDRAW)) == 0 && !lp_new_item.is_null() {
                let text_slice = lp_new_item.to_slice_until_null_scan();

                #[cfg(feature = "enable_debug_output")]
                {
                    let raw_text = text_slice.to_wide_ansi().to_string_lossy();
                    crate::debug!("Get AppendMenuA text: {raw_text}");
                }

                let _opt_trans_msg = text_slice.to_wide_ansi().lookup_or_store_null();

                #[cfg(not(feature = "extract_text"))]
                if let Ok(Some(trans_msg)) = _opt_trans_msg {
                    use windows_sys::Win32::UI::WindowsAndMessaging::AppendMenuW;
                    return AppendMenuW(h_menu, u_flags, u_id_new_item, trans_msg.as_ptr());
                }
            }

            crate::call!(
                HOOK_APPEND_MENU_A,
                h_menu,
                u_flags,
                u_id_new_item,
                lp_new_item
            )
        }
    }
}

impl MessageBox for UserInterfacePatcherSlot {
    unsafe fn message_box_a(
        h_wnd: HWND,
        lp_text: *const u8,
        lp_caption: *const u8,
        u_type: u32,
    ) -> i32 {
        unsafe {
            if lp_text.is_null() && lp_caption.is_null() {
                return crate::call!(HOOK_MESSAGE_BOX_A, h_wnd, lp_text, lp_caption, u_type);
            }

            let text_slice = lp_text.to_slice_until_null_scan();
            let cap_slice = lp_caption.to_slice_until_null_scan();

            #[cfg(feature = "enable_debug_output")]
            {
                if !text_slice.is_empty() {
                    let s = text_slice.to_wide_ansi().to_string_lossy();
                    crate::debug!("Get message box text: {s}");
                }
                if !cap_slice.is_empty() {
                    let s = cap_slice.to_wide_ansi().to_string_lossy();
                    crate::debug!("Get message box caption: {s}");
                }
            }

            let _opt_wide_text = text_slice.to_wide_ansi().lookup_or_store_null();
            let _opt_wide_caption = cap_slice.to_wide_ansi().lookup_or_store_null();

            #[cfg(not(feature = "extract_text"))]
            if _opt_wide_text.is_ok() || _opt_wide_caption.is_ok() {
                let wide_text = _opt_wide_text
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| text_slice.to_wide_null_ansi());
                let wide_caption = _opt_wide_caption
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| cap_slice.to_wide_null_ansi());

                let wide_text_ptr = if wide_text.len() == 1 {
                    core::ptr::null()
                } else {
                    wide_text.as_ptr()
                };

                let wide_caption_ptr = if wide_caption.len() == 1 {
                    core::ptr::null()
                } else {
                    wide_caption.as_ptr()
                };

                use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
                return MessageBoxW(h_wnd, wide_text_ptr, wide_caption_ptr, u_type);
            }
            crate::call!(HOOK_MESSAGE_BOX_A, h_wnd, lp_text, lp_caption, u_type)
        }
    }
}

impl SetDlgItemText for UserInterfacePatcherSlot {
    unsafe fn set_dlg_item_text_a(h_dlg: HWND, n_id_dlg_item: i32, lp_string: *const u8) -> BOOL {
        unsafe {
            if lp_string.is_null() {
                return crate::call!(HOOK_SET_DLG_ITEM_TEXT_A, h_dlg, n_id_dlg_item, lp_string);
            }

            let text_slice = lp_string.to_slice_until_null_scan();

            #[cfg(feature = "enable_debug_output")]
            {
                let raw_text = text_slice.to_wide_ansi().to_string_lossy();
                crate::debug!("Get SetDlgItemTextA text: {raw_text}");
            }

            let _opt_trans_msg = text_slice
                .to_wide_ansi()
                .lookup_or_store_null()
                .ok()
                .flatten();

            #[cfg(not(feature = "extract_text"))]
            if let Some(trans_msg) = _opt_trans_msg {
                use windows_sys::Win32::UI::WindowsAndMessaging::SetDlgItemTextW;
                return SetDlgItemTextW(h_dlg, n_id_dlg_item, trans_msg.as_ptr());
            }
            crate::call!(HOOK_SET_DLG_ITEM_TEXT_A, h_dlg, n_id_dlg_item, lp_string)
        }
    }
}

impl SetWindowText for UserInterfacePatcherSlot {
    unsafe fn set_window_text_a(h_wnd: HWND, lp_string: *const u8) -> BOOL {
        unsafe {
            if lp_string.is_null() {
                return crate::call!(HOOK_SET_WINDOW_TEXT_A, h_wnd, lp_string);
            }

            let text_slice = lp_string.to_slice_until_null_scan();

            #[cfg(feature = "enable_debug_output")]
            {
                let raw_text = text_slice.to_wide_ansi().to_string_lossy();
                crate::debug!("Get SetWindowTextA text: {raw_text}");
            }

            let _opt_trans_msg = text_slice
                .to_wide_ansi()
                .lookup_or_store_null()
                .ok()
                .flatten();

            #[cfg(not(feature = "extract_text"))]
            if let Some(trans_msg) = _opt_trans_msg {
                use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW;
                return SetWindowTextW(h_wnd, trans_msg.as_ptr());
            }
            crate::call!(HOOK_SET_WINDOW_TEXT_A, h_wnd, lp_string)
        }
    }
}

impl SendMessage for UserInterfacePatcherSlot {
    unsafe fn send_message_a(h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        unsafe {
            if crate::utils::win32::needs_text_conversion(msg) && l_param != 0 {
                let text_slice = (l_param as *const u8).to_slice_until_null_scan();

                #[cfg(feature = "enable_debug_output")]
                {
                    let raw_text = text_slice.to_wide_ansi().to_string_lossy();
                    crate::debug!("Get SendMessageA (msg={msg:#x}) text: {raw_text}");
                }

                let _opt_trans_msg = text_slice
                    .to_wide_ansi()
                    .lookup_or_store_null()
                    .ok()
                    .flatten();

                #[cfg(not(feature = "extract_text"))]
                if let Some(trans_msg) = _opt_trans_msg {
                    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
                    return SendMessageW(h_wnd, msg, w_param, trans_msg.as_ptr() as LPARAM);
                }
            }
            crate::call!(HOOK_SEND_MESSAGE_A, h_wnd, msg, w_param, l_param)
        }
    }
}

impl PropertySheet for UserInterfacePatcherSlot {
    unsafe fn property_sheet_a(
        ppsh: *const windows_sys::Win32::UI::Controls::PROPSHEETHEADERA_V2,
    ) -> isize {
        unsafe {
            if ppsh.is_null() {
                return crate::call!(HOOK_PROPERTY_SHEET_A, ppsh);
            }

            let header = &*ppsh;

            if header.pszCaption.is_null() {
                return crate::call!(HOOK_PROPERTY_SHEET_A, ppsh);
            }

            let caption_slice = header.pszCaption.to_slice_until_null_scan();

            #[cfg(feature = "enable_debug_output")]
            {
                let raw = caption_slice.to_wide_ansi().to_string_lossy();
                crate::debug!("PropertySheetA original caption (ANSI): {raw}");
            }

            let _opt_trans = caption_slice
                .to_wide_ansi()
                .lookup_or_store()
                .ok()
                .flatten()
                .map(|s| s.to_multi_byte_null(0));

            #[cfg(not(feature = "extract_text"))]
            if let Some(trans) = _opt_trans {
                use windows_sys::Win32::UI::Controls::PROPSHEETHEADERA_V2;

                let dw_size = header.dwSize as usize;

                let mut new_buf = Box::<[u8]>::new_uninit_slice(dw_size);
                let new_hdr_slice = new_buf
                    .write_copy_of_slice(core::slice::from_raw_parts(ppsh.cast::<u8>(), dw_size));

                let new_hdr_ptr = new_hdr_slice.as_mut_ptr() as *mut PROPSHEETHEADERA_V2;

                (*new_hdr_ptr).pszCaption = trans.as_ptr();

                return crate::call!(
                    HOOK_PROPERTY_SHEET_A,
                    new_hdr_ptr as *const PROPSHEETHEADERA_V2
                );
            }
            crate::call!(HOOK_PROPERTY_SHEET_A, ppsh)
        }
    }
}
