use cfg_if::cfg_if;

use crate::{hook::internal_hooks::OverlayRender, overlay::egui::components};

#[allow(dead_code)]
pub struct EguiDefaultUi;

cfg_if! {
    if #[cfg(feature = "bind_egui_default_ui")] {
        type EguiDefaultUiSlot = crate::hook::impls::HookImplType;
    } else {
        type EguiDefaultUiSlot = EguiDefaultUi;
    }
}

impl OverlayRender for EguiDefaultUiSlot {
    #[cfg(feature = "enable_overlay")]
    fn on_overlay_render(context: &mut crate::overlay::OverlayContext) -> crate::Result<()> {
        context.egui.clear([0.0, 0.0, 0.0, 0.0]);

        context.run_egui(|egui_ctx| {
            components::render_visibility_panel(egui_ctx);
            components::render_all(egui_ctx);
            Ok(())
        })?;

        context.gl_ctx.swap_buffers()
    }
}
