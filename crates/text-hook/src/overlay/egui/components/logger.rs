use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
};

use std::sync::atomic::{AtomicBool, Ordering};

static IS_VISIBLE: AtomicBool = AtomicBool::new(true);
static LOGGER_STATE: OnceLock<Mutex<LoggerState>> = OnceLock::new();

const DEFAULT_MAX_LINES: usize = 500;

struct LoggerState {
    lines: VecDeque<String>,
    max_lines: usize,
    auto_scroll: bool,
}

impl Default for LoggerState {
    fn default() -> Self {
        Self {
            lines: VecDeque::with_capacity(DEFAULT_MAX_LINES),
            max_lines: DEFAULT_MAX_LINES,
            auto_scroll: true,
        }
    }
}

impl LoggerState {
    fn push_line(&mut self, line: String) {
        if line.is_empty() {
            return;
        }

        self.lines.push_back(line);
        self.trim_to_max_lines();
    }

    fn trim_to_max_lines(&mut self) {
        while self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
    }
}

fn logger_state() -> &'static Mutex<LoggerState> {
    LOGGER_STATE.get_or_init(|| Mutex::new(LoggerState::default()))
}

pub fn render(egui_ctx: &egui::Context) {
    if !is_visible() {
        return;
    }

    egui::Window::new("text-hook logger")
        .default_pos([24.0, 220.0])
        .default_size([620.0, 320.0])
        .resizable(true)
        .vscroll(false)
        .show(egui_ctx, |ui| {
            let Ok(mut state) = logger_state().lock() else {
                ui.label("logger state lock failed");
                return;
            };

            ui.horizontal(|ui| {
                if ui.button("Clear").clicked() {
                    state.lines.clear();
                }

                ui.checkbox(&mut state.auto_scroll, "Auto Scroll");

                let mut max_lines = state.max_lines as u32;
                if ui
                    .add(
                        egui::DragValue::new(&mut max_lines)
                            .range(50..=10_000)
                            .prefix("Max: "),
                    )
                    .changed()
                {
                    state.max_lines = max_lines as usize;
                    state.trim_to_max_lines();
                }

                ui.separator();
                ui.label(format!("Lines: {}", state.lines.len()));
            });

            ui.separator();

            egui::ScrollArea::vertical()
                .stick_to_bottom(state.auto_scroll)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for line in &state.lines {
                        ui.label(line);
                    }
                });
        });
}

/// 向 logger 组件追加一行日志。
pub fn push_log_line(line: impl Into<String>) {
    let line: String = line.into();

    let Ok(mut state) = logger_state().lock() else {
        return;
    };

    for part in line.lines() {
        state.push_line(part.to_owned());
    }
}

pub fn init() -> crate::Result<()> {
    let _ = logger_state();
    crate::debug!("logger component initialized");
    Ok(())
}

#[cfg(feature = "enable_attach_cleanup")]
pub fn attach_cleanup() -> crate::Result<()> {
    crate::debug!("logger component cleanup");
    Ok(())
}

pub fn is_visible() -> bool {
    IS_VISIBLE.load(Ordering::Acquire)
}

pub fn set_visible(is_visible: bool) {
    IS_VISIBLE.store(is_visible, Ordering::Release);
}
