use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

use crate::{hook::internal_hooks::OverlayWndProc, overlay::with_overlay_context_mut};

#[allow(dead_code)]
pub struct EguiIO;

type EguiIOSlot = cfg_select! {
    feature = "bind_egui_io" => crate::hook::impls::HookImplType,
    _ => EguiIO
};

impl OverlayWndProc for EguiIOSlot {
    fn on_overlay_wnd_proc(
        hwnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> crate::Result<Option<LRESULT>> {
        with_overlay_context_mut(|context| {
            if *context.overlay != hwnd {
                return Ok(None);
            }

            crate::overlay::egui::integration::handle_egui_wnd_proc(
                &mut context.egui,
                hwnd,
                msg,
                w_param,
                l_param,
            )
        })
    }
}
