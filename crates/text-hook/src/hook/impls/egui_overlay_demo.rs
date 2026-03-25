use std::sync::{Arc, LazyLock, Mutex, Once};

use translate_macros::DefaultHook;

use crate::hook::internal_hooks::OverlayRender;

static DEMO_STATE: LazyLock<Mutex<EguiOverlayDemoState>> =
    LazyLock::new(|| Mutex::new(EguiOverlayDemoState::default()));

#[derive(Default)]
struct EguiOverlayDemoState {
    text_input: String,
    multiline_text: String,
    slider_value: f32,
    drag_value: i32,
    checkbox_value: bool,
    radio_value: DemoRadioValue,
    combo_value: DemoComboValue,
    progress: f32,
    button_clicks: u32,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum DemoRadioValue {
    #[default]
    Alpha,
    Beta,
    Gamma,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum DemoComboValue {
    #[default]
    Scroll,
    Input,
    Focus,
}

#[derive(DefaultHook)]
#[exclude(OverlayRender)]
pub struct EguiOverlayDemoHook;

pub fn my_smart_font_setup(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 不要直接用它那个可能报错的函数，自己读一个确定的路径
    if let Ok(font_data) = std::fs::read(r"C:\Windows\Fonts\simsun.ttc") {
        fonts.font_data.insert(
            "msyh".to_owned(),
            Arc::new(egui::FontData::from_owned(font_data)),
        );

        // 重点：把中文放第一位，但保留默认字体作为回退
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "msyh".to_owned());
    }

    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    // 设置面板背景颜色，最后的 100 是 Alpha 通道 (0-255)
    visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 130);
    ctx.set_visuals(visuals);
}

static CALL_ONCE: Once = Once::new();

impl OverlayRender for EguiOverlayDemoHook {
    fn on_overlay_render(context: &mut crate::overlay::OverlayContext) -> crate::Result<()> {
        CALL_ONCE.call_once(|| my_smart_font_setup(context.egui.egui_ctx()));

        context.egui.clear([0.0, 0.0, 0.0, 0.0]);

        let mut state = DEMO_STATE
            .lock()
            .map_err(|e| crate::anyhow!("lock egui overlay demo state failed: {e}"))?;

        context.run_egui(|egui_ctx| {
            egui::Window::new("text-hook egui demo")
                .default_pos([24.0, 24.0])
                .default_size([720.0, 560.0])
                .resizable(true)
                .vscroll(true)
                .show(egui_ctx, |ui| {
                    ui.heading("Overlay / egui 输入测试面板");
                    ui.label("这个 demo 专门拿来测 overlay 输入桥接，不是摆设。");

                    ui.separator();

                    egui_ctx.settings_ui(ui);

                    ui.separator();

                    ui.collapsing("当前输入状态", |ui| {
                        let input = egui_ctx.input(|i| {
                            (
                                i.pointer.latest_pos(),
                                i.pointer.hover_pos(),
                                i.pointer.any_down(),
                                i.raw_scroll_delta,
                                i.modifiers,
                                i.focused,
                                i.time,
                            )
                        });

                        ui.monospace(format!("latest pointer pos: {:?}", input.0));
                        ui.monospace(format!("hover pointer pos:  {:?}", input.1));
                        ui.monospace(format!("pointer any down:   {}", input.2));
                        ui.monospace(format!("raw scroll delta:   {:?}", input.3));
                        ui.monospace(format!("modifiers:          {:?}", input.4));
                        ui.monospace(format!("focused:            {}", input.5));
                        ui.monospace(format!("time:               {:?}", input.6));
                    });

                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label("测试说明：");
                        ui.label("1) 鼠标滚轮/横向滚轮测 ScrollArea");
                        ui.label("2) 点击输入框测键盘输入");
                        ui.label("3) 拖 slider / drag value 测拖拽");
                        ui.label("4) 切走焦点再切回来，观察状态是否正常");
                    });

                    ui.separator();
                    ui.columns(2, |columns| {
                        columns[0].group(|ui| {
                            ui.heading("输入控件");
                            ui.label("单行输入：");
                            ui.text_edit_singleline(&mut state.text_input);

                            ui.add_space(8.0);
                            ui.label("多行输入：");
                            ui.add(
                                egui::TextEdit::multiline(&mut state.multiline_text)
                                    .desired_rows(6)
                                    .desired_width(f32::INFINITY),
                            );

                            ui.add_space(8.0);
                            ui.checkbox(&mut state.checkbox_value, "checkbox / 点击测试");
                            ui.radio_value(
                                &mut state.radio_value,
                                DemoRadioValue::Alpha,
                                "radio alpha",
                            );
                            ui.radio_value(
                                &mut state.radio_value,
                                DemoRadioValue::Beta,
                                "radio beta",
                            );
                            ui.radio_value(
                                &mut state.radio_value,
                                DemoRadioValue::Gamma,
                                "radio gamma",
                            );

                            ui.add_space(8.0);
                            egui::ComboBox::from_label("combo / 弹出层测试")
                                .selected_text(match state.combo_value {
                                    DemoComboValue::Scroll => "Scroll",
                                    DemoComboValue::Input => "Input",
                                    DemoComboValue::Focus => "Focus",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.combo_value,
                                        DemoComboValue::Scroll,
                                        "Scroll",
                                    );
                                    ui.selectable_value(
                                        &mut state.combo_value,
                                        DemoComboValue::Input,
                                        "Input",
                                    );
                                    ui.selectable_value(
                                        &mut state.combo_value,
                                        DemoComboValue::Focus,
                                        "Focus",
                                    );
                                });
                        });

                        columns[1].group(|ui| {
                            ui.heading("交互控件");
                            ui.add(
                                egui::Slider::new(&mut state.slider_value, 0.0..=100.0)
                                    .text("slider / 拖拽测试"),
                            );
                            ui.add(egui::DragValue::new(&mut state.drag_value).speed(1.0));

                            if ui.button("button / 点击测试").clicked() {
                                state.button_clicks = state.button_clicks.saturating_add(1);
                                state.progress = (state.progress + 0.1).min(1.0);
                            }

                            ui.monospace(format!("button clicks: {}", state.button_clicks));
                            ui.add(
                                egui::ProgressBar::new(state.progress)
                                    .show_percentage()
                                    .text("progress / 重绘测试"),
                            );

                            ui.add_space(12.0);
                            if ui.button("reset demo state").clicked() {
                                *state = EguiOverlayDemoState::default();
                            }
                        });
                    });

                    ui.separator();
                    ui.heading("滚动区域测试");
                    ui.label(
                        "把鼠标停在下面区域里，试纵向滚轮、横向滚轮、拖滚动条。\
                         这里故意放很多内容。 ",
                    );

                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .max_height(240.0)
                        .show(ui, |ui| {
                            ui.set_min_width(1100.0);

                            egui::Grid::new("overlay_demo_scroll_grid")
                                .striped(true)
                                .min_col_width(120.0)
                                .show(ui, |ui| {
                                    ui.strong("row");
                                    ui.strong("description");
                                    ui.strong("value");
                                    ui.strong("notes");
                                    ui.end_row();

                                    for index in 0..48 {
                                        ui.label(format!("#{index:02}"));
                                        ui.label(format!(
                                            "scroll test item {index}, move wheel here and verify \
                                             pointer target"
                                        ));
                                        ui.label(format!(
                                            "slider={:.1}, drag={}, clicks={}",
                                            state.slider_value,
                                            state.drag_value,
                                            state.button_clicks
                                        ));
                                        ui.label(if index % 2 == 0 {
                                            "even row"
                                        } else {
                                            "odd row"
                                        });
                                        ui.end_row();
                                    }
                                });
                        });

                    ui.separator();
                    ui.collapsing("egui inspection_ui", |ui| {
                        egui_ctx.inspection_ui(ui);
                    });
                });
        })?;

        context.gl_ctx.swap_buffers()
    }
}
