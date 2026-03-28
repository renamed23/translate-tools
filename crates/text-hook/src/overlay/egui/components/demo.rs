use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

static IS_VISIBLE: AtomicBool = AtomicBool::new(true);
static DEMO_STATE: OnceLock<Mutex<EguiOverlayDemoState>> = OnceLock::new();

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

fn demo_state() -> &'static Mutex<EguiOverlayDemoState> {
    DEMO_STATE.get_or_init(|| Mutex::new(EguiOverlayDemoState::default()))
}

pub fn render(egui_ctx: &egui::Context) {
    if !is_visible() {
        return;
    }

    let Ok(mut state) = demo_state().lock() else {
        crate::debug!("demo component state lock failed");
        return;
    };

    egui::Window::new("text-hook egui demo")
        .default_pos([24.0, 24.0])
        .default_size([720.0, 560.0])
        .resizable(true)
        .vscroll(true)
        .show(egui_ctx, |ui| {
            ui.heading("Overlay / egui Input Test Panel");
            ui.label("This demo is used to validate overlay input bridging behavior.");

            ui.separator();

            egui_ctx.settings_ui(ui);

            ui.separator();

            ui.collapsing("Current Input State", |ui| {
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
                ui.label("Test checklist:");
                ui.label("1) Use vertical/horizontal mouse wheel in ScrollArea");
                ui.label("2) Click text boxes and verify keyboard input");
                ui.label("3) Drag slider / drag value and verify dragging");
                ui.label("4) Switch focus away and back, then verify state");
            });

            ui.separator();
            ui.columns(2, |columns| {
                columns[0].group(|ui| {
                    ui.heading("Input Controls");
                    ui.label("Single-line input:");
                    ui.text_edit_singleline(&mut state.text_input);

                    ui.add_space(8.0);
                    ui.label("Multi-line input:");
                    ui.add(
                        egui::TextEdit::multiline(&mut state.multiline_text)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(&mut state.checkbox_value, "checkbox / click test");
                    ui.radio_value(&mut state.radio_value, DemoRadioValue::Alpha, "radio alpha");
                    ui.radio_value(&mut state.radio_value, DemoRadioValue::Beta, "radio beta");
                    ui.radio_value(&mut state.radio_value, DemoRadioValue::Gamma, "radio gamma");

                    ui.add_space(8.0);
                    egui::ComboBox::from_label("combo / popup test")
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
                    ui.heading("Interaction Controls");
                    ui.add(
                        egui::Slider::new(&mut state.slider_value, 0.0..=100.0)
                            .text("slider / drag test"),
                    );
                    ui.add(egui::DragValue::new(&mut state.drag_value).speed(1.0));

                    if ui.button("button / click test").clicked() {
                        state.button_clicks = state.button_clicks.saturating_add(1);
                        state.progress = (state.progress + 0.1).min(1.0);
                    }

                    ui.monospace(format!("button clicks: {}", state.button_clicks));
                    ui.add(
                        egui::ProgressBar::new(state.progress)
                            .show_percentage()
                            .text("progress / repaint test"),
                    );

                    ui.add_space(12.0);
                    if ui.button("reset demo state").clicked() {
                        *state = EguiOverlayDemoState::default();
                    }
                });
            });

            ui.separator();
            ui.heading("Scroll Area Test");
            ui.label(
                "Hover the area below and test vertical wheel, horizontal wheel, and dragging the \
                 scrollbar. This area intentionally contains lots of content.",
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
                                    "scroll test item {index}, move wheel here and verify pointer \
                                     target"
                                ));
                                ui.label(format!(
                                    "slider={:.1}, drag={}, clicks={}",
                                    state.slider_value, state.drag_value, state.button_clicks
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
}

pub fn init() -> crate::Result<()> {
    let _ = demo_state();
    crate::debug!("demo component initialized");
    Ok(())
}

#[cfg(feature = "enable_attach_cleanup")]
pub fn attach_cleanup() -> crate::Result<()> {
    crate::debug!("demo component cleanup");
    Ok(())
}

pub fn is_visible() -> bool {
    IS_VISIBLE.load(Ordering::Acquire)
}

pub fn set_visible(is_visible: bool) {
    IS_VISIBLE.store(is_visible, Ordering::Release);
}
