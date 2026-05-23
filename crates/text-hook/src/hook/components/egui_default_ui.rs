use crate::{hook::internal_hooks::OverlayRender, overlay::egui::components};

#[allow(dead_code)]
pub struct EguiDefaultUi;

type EguiDefaultUiSlot = cfg_select! {
    feature = "bind_egui_default_ui" => crate::hook::impls::HookImplType,
    _ => EguiDefaultUi
};

impl OverlayRender for EguiDefaultUiSlot {
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
