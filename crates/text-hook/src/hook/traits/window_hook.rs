use translate_macros::detour_trait;
use windows_sys::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::HMENU,
    },
    core::BOOL,
};

#[detour_trait]
pub trait DefWindowProc {
    #[detour(dll = "user32.dll", symbol = "DefWindowProcA", fallback = "0")]
    unsafe fn def_window_proc_a(
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT;

    #[detour(dll = "user32.dll", symbol = "DefWindowProcW", fallback = "0")]
    unsafe fn def_window_proc_w(
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT;
}

#[detour_trait]
pub trait ModifyMenu {
    #[detour(dll = "user32.dll", symbol = "ModifyMenuA", fallback = "0")]
    unsafe fn modify_menu_a(
        h_menu: HMENU,
        u_position: u32,
        u_flags: u32,
        u_id_new_item: usize,
        lp_new_item: *const u8,
    ) -> BOOL;
}

#[detour_trait]
pub trait MessageBox {
    #[detour(dll = "user32.dll", symbol = "MessageBoxA", fallback = "0")]
    unsafe fn message_box_a(
        h_wnd: HWND,
        lp_text: *const u8,
        lp_caption: *const u8,
        u_type: u32,
    ) -> i32;
}

#[detour_trait]
pub trait SetDlgItemText {
    #[detour(dll = "user32.dll", symbol = "SetDlgItemTextA", fallback = "0")]
    unsafe fn set_dlg_item_text_a(h_dlg: HWND, n_id_dlg_item: i32, lp_string: *const u8) -> BOOL;
}

#[detour_trait]
pub trait SetWindowText {
    #[detour(dll = "user32.dll", symbol = "SetWindowTextA", fallback = "0")]
    unsafe fn set_window_text_a(h_wnd: HWND, lp_string: *const u8) -> BOOL;
}

#[detour_trait]
pub trait SendMessage {
    #[detour(dll = "user32.dll", symbol = "SendMessageA", fallback = "0")]
    unsafe fn send_message_a(h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
}

#[detour_trait]
pub trait PropertySheet {
    #[detour(dll = "comctl32.dll", symbol = "PropertySheetA", fallback = "0")]
    unsafe fn property_sheet_a(
        ppsh: *const windows_sys::Win32::UI::Controls::PROPSHEETHEADERA_V2,
    ) -> isize;
}
