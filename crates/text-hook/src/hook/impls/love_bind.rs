use translate_macros::{DefaultHook, detour_fn};
use windows_sys::Win32::Foundation::{HMODULE, HWND};
use windows_sys::Win32::UI::WindowsAndMessaging::{MESSAGEBOX_RESULT, MESSAGEBOX_STYLE};

use crate::hook::traits::CoreHook;
use crate::utils::exts::slice_ext::{ByteSliceExt, WideSliceExt};

#[derive(DefaultHook)]
pub struct LoveBindHook;

impl CoreHook for LoveBindHook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        unsafe {
            HOOK_MESSAGE_BOX_TIMEOUT_A.enable().unwrap();
        };
    }

    fn on_process_detach(_hinst_dll: HMODULE) {
        unsafe {
            HOOK_MESSAGE_BOX_TIMEOUT_A.disable().unwrap();
        };
    }
}

#[detour_fn(dll = "user32.dll", symbol = "MessageBoxTimeoutA", fallback = "1")]
unsafe extern "system" fn message_box_timeout_a(
    h_wnd: HWND,
    lp_text: *const u8,
    lp_caption: *const u8,
    u_type: MESSAGEBOX_STYLE,
    w_language_id: u16,
    dw_milliseconds: u32,
) -> MESSAGEBOX_RESULT {
    unsafe {
        let cap_slice = crate::utils::mem::slice_until_null(lp_caption, 1024);
        let s = cap_slice.to_wide_ansi().to_string_lossy();

        crate::debug!("Get message box caption: {s}");
        if s == "日本語版Windows判定" {
            return 2;
        }

        crate::call!(
            HOOK_MESSAGE_BOX_TIMEOUT_A,
            h_wnd,
            lp_text,
            lp_caption,
            u_type,
            w_language_id,
            dw_milliseconds
        )
    }
}
