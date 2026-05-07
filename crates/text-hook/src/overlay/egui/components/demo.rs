use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicBool, Ordering},
};

static IS_VISIBLE: AtomicBool = AtomicBool::new(true);
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

pub fn render(egui_ctx: &egui::Context) {
    if !is_visible() {
        return;
    }

    let Ok(mut state) = DEMO_STATE.lock() else {
        crate::debug!("demo component state lock failed");
        return;
    };

    egui::Window::new("text-hook egui 演示")
        .default_pos([24.0, 24.0])
        .default_size([720.0, 560.0])
        .resizable(true)
        .vscroll(true)
        .show(egui_ctx, |ui| {
            ui.heading("悬浮层 / egui 输入测试面板");
            ui.label("本演示用于验证悬浮层输入桥接行为。");

            ui.separator();

            egui_ctx.settings_ui(ui);

            ui.separator();

            ui.collapsing("当前输入状态", |ui| {
                let input = egui_ctx.input(|i| {
                    (
                        i.pointer.latest_pos(),
                        i.pointer.hover_pos(),
                        i.pointer.any_down(),
                        i.smooth_scroll_delta,
                        i.modifiers,
                        i.focused,
                        i.time,
                    )
                });

                ui.monospace(format!("最新指针位置: {:?}", input.0));
                ui.monospace(format!("悬停指针位置:  {:?}", input.1));
                ui.monospace(format!("指针按下状态:   {}", input.2));
                ui.monospace(format!("原始滚动增量:   {:?}", input.3));
                ui.monospace(format!("修饰键:          {:?}", input.4));
                ui.monospace(format!("焦点状态:            {}", input.5));
                ui.monospace(format!("时间:               {:?}", input.6));
            });

            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label("测试清单：");
                ui.label("1) 在滚动区域中使用垂直/水平鼠标滚轮");
                ui.label("2) 点击文本框并验证键盘输入");
                ui.label("3) 拖动滑块/数值框并验证拖拽操作");
                ui.label("4) 切换焦点离开再返回，然后验证状态");
            });

            ui.separator();
            ui.columns(2, |columns| {
                columns[0].group(|ui| {
                    ui.heading("输入控件");
                    ui.label("单行输入框：");
                    ui.text_edit_singleline(&mut state.text_input);

                    ui.add_space(8.0);
                    ui.label("多行输入框：");
                    ui.add(
                        egui::TextEdit::multiline(&mut state.multiline_text)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(&mut state.checkbox_value, "复选框/点击测试");
                    ui.radio_value(
                        &mut state.radio_value,
                        DemoRadioValue::Alpha,
                        "单选框 Alpha",
                    );
                    ui.radio_value(&mut state.radio_value, DemoRadioValue::Beta, "单选框 Beta");
                    ui.radio_value(
                        &mut state.radio_value,
                        DemoRadioValue::Gamma,
                        "单选框 Gamma",
                    );

                    ui.add_space(8.0);
                    egui::ComboBox::from_label("下拉框/弹出测试")
                        .selected_text(match state.combo_value {
                            DemoComboValue::Scroll => "滚动",
                            DemoComboValue::Input => "输入",
                            DemoComboValue::Focus => "焦点",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.combo_value,
                                DemoComboValue::Scroll,
                                "滚动",
                            );
                            ui.selectable_value(
                                &mut state.combo_value,
                                DemoComboValue::Input,
                                "输入",
                            );
                            ui.selectable_value(
                                &mut state.combo_value,
                                DemoComboValue::Focus,
                                "焦点",
                            );
                        });
                });

                columns[1].group(|ui| {
                    ui.heading("交互控件");
                    ui.add(
                        egui::Slider::new(&mut state.slider_value, 0.0..=100.0)
                            .text("滑块/拖动测试"),
                    );
                    ui.add(egui::DragValue::new(&mut state.drag_value).speed(1.0));

                    if ui.button("按钮/点击测试").clicked() {
                        state.button_clicks = state.button_clicks.saturating_add(1);
                        state.progress = (state.progress + 0.1).min(1.0);
                    }

                    ui.monospace(format!("按钮点击次数：{}", state.button_clicks));
                    ui.add(
                        egui::ProgressBar::new(state.progress)
                            .show_percentage()
                            .text("进度/重绘测试"),
                    );

                    ui.add_space(12.0);
                    if ui.button("重置演示状态").clicked() {
                        *state = EguiOverlayDemoState::default();
                    }
                });
            });

            ui.separator();
            ui.heading("滚动区域测试");
            ui.label(
                "将鼠标悬停在下方区域，测试垂直滚轮、水平滚轮以及拖动滚动条。\
                 本区域特意包含大量内容。",
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
                            ui.strong("行");
                            ui.strong("描述");
                            ui.strong("值");
                            ui.strong("备注");
                            ui.end_row();

                            for index in 0..48 {
                                ui.label(format!("#{index:02}"));
                                ui.label(format!(
                                    "滚动测试项 {index}，将鼠标悬停此处并滚动滚轮以验证指针目标"
                                ));
                                ui.label(format!(
                                    "滑块={:.1}, 数值={}, 点击次数={}",
                                    state.slider_value, state.drag_value, state.button_clicks
                                ));
                                ui.label(if index % 2 == 0 {
                                    "偶数行"
                                } else {
                                    "奇数行"
                                });
                                ui.end_row();
                            }
                        });
                });

            ui.separator();
            ui.collapsing("egui 检查界面", |ui| {
                egui_ctx.inspection_ui(ui);
            });
        });
}

pub fn init() -> crate::Result<()> {
    crate::debug!("demo component initialized");
    Ok(())
}

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
