use std::sync::{Arc, Once};

use translate_macros::expand_by_files;

expand_by_files!("src/overlay/egui/components" => {
    #[cfg(feature = __concat__("enable_egui_", __file_str__))]
    pub(crate) mod __file__;
});

static INIT_ONCE: Once = Once::new();

/// 渲染所有已启用的 egui 组件。
pub fn render_all(egui_ctx: &egui::Context) {
    INIT_ONCE.call_once(|| {
        setup_ui_style(egui_ctx);
        expand_by_files!("src/overlay/egui/components" => {
            #[cfg(feature = __concat__("enable_egui_", __file_str__))]
            if let Err(e) =  __file__::init() {
                crate::debug!("Component '{}' init failed {e:?}", __file_str__);
            }
        });
    });

    expand_by_files!("src/overlay/egui/components" => {
        #[cfg(feature = __concat__("enable_egui_", __file_str__))]
        __file__::render(egui_ctx);
    });
}

/// 设置基础的 egui 样式
pub fn setup_ui_style(egui_ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    match std::fs::read("C:/Windows/Fonts/simsun.ttc") {
        Ok(font_data) => {
            fonts.font_data.insert(
                "simsun".to_owned(),
                Arc::new(egui::FontData::from_owned(font_data)),
            );

            if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                proportional.insert(0, "simsun".to_owned());
            }

            if let Some(monospace) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                monospace.insert(0, "simsun".to_owned());
            }

            crate::debug!("logger ui font loaded: C:/Windows/Fonts/simsun.ttc");
        }
        Err(e) => {
            crate::debug!("logger ui font load failed: {e:?}");
        }
    }

    egui_ctx.set_fonts(fonts);

    let panel_fill = egui::Color32::from_rgba_unmultiplied(15, 18, 24, 220);
    let window_fill = egui::Color32::from_rgba_unmultiplied(12, 15, 22, 236);
    let accent = egui::Color32::from_rgb(92, 193, 255);
    let accent_soft = egui::Color32::from_rgba_unmultiplied(92, 193, 255, 88);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = panel_fill;
    visuals.window_fill = window_fill;
    visuals.window_highlight_topmost = false;

    visuals.hyperlink_color = accent;
    visuals.faint_bg_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    visuals.extreme_bg_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180);
    visuals.code_bg_color = egui::Color32::from_rgba_unmultiplied(14, 18, 28, 245);

    visuals.selection.bg_fill = accent_soft;
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);

    visuals.widgets.noninteractive.bg_fill =
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 24),
    );

    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_unmultiplied(92, 193, 255, 28);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(92, 193, 255, 70));

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_unmultiplied(92, 193, 255, 54);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);

    visuals.widgets.active.bg_fill = egui::Color32::from_rgba_unmultiplied(92, 193, 255, 82);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);

    visuals.widgets.open.bg_fill = egui::Color32::from_rgba_unmultiplied(92, 193, 255, 40);
    visuals.widgets.open.bg_stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(92, 193, 255, 120),
    );

    egui_ctx.set_visuals(visuals);

    egui_ctx.style_mut(|style| {
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(24.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Name("Heading2".into()),
                egui::FontId::new(20.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(16.0, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            ),
        ]
        .into();

        style.animation_time = 0.12;
    });
}

/// 渲染组件可见性控制面板。
pub fn render_visibility_panel(egui_ctx: &egui::Context) {
    egui::Window::new("text-hook components panel")
        .default_pos([24.0, 24.0])
        .default_size([260.0, 220.0])
        .resizable(true)
        .show(egui_ctx, |ui| {
            ui.label("Toggle components");
            ui.separator();

            expand_by_files!("src/overlay/egui/components" => {
                #[cfg(feature = __concat__("enable_egui_", __file_str__))]
                {
                    let mut visible = __file__::is_visible();
                    if ui.checkbox(&mut visible, __file_str__).changed() {
                        __file__::set_visible(visible);
                    }
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Show all").clicked() {
                    request_show_all();
                }

                if ui.button("Hide all").clicked() {
                    request_hide_all();
                }
            });
        });
}

/// 在 attach cleanup 阶段通知所有已启用的 egui 组件执行清理。
#[cfg(feature = "enable_attach_cleanup")]
pub fn attach_cleanup_all() {
    expand_by_files!("src/overlay/egui/components" => {
        #[cfg(feature = __concat__("enable_egui_", __file_str__))]
        if let Err(e) =  __file__::attach_cleanup() {
            crate::debug!("Component '{}' attach cleanup failed {e:?}", __file_str__);
        }
    });
}

/// 请求隐藏所有已启用的 egui 组件 UI。
pub fn request_hide_all() {
    expand_by_files!("src/overlay/egui/components" => {
        #[cfg(feature = __concat__("enable_egui_", __file_str__))]
        __file__::set_visible(false);
    });
}

/// 请求显示所有已启用的 egui 组件 UI。
pub fn request_show_all() {
    expand_by_files!("src/overlay/egui/components" => {
        #[cfg(feature = __concat__("enable_egui_", __file_str__))]
        __file__::set_visible(true);
    });
}
