// FIXME: 该文件由AI生成，写的很糙，有空给它改一下

use crate::{custom_font, utils::log_font::LogFont};
use egui::{DragValue, Window};
use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicBool, Ordering},
};

static IS_VISIBLE: AtomicBool = AtomicBool::new(false);

// 核心修正：增加一个全局的编辑缓冲区，防止每帧被 get_font() 重置
static EDIT_CACHE: LazyLock<Mutex<LogFont>> = LazyLock::new(|| Mutex::new(custom_font::get_font()));

pub fn render(egui_ctx: &egui::Context) {
    if !is_visible() {
        return;
    }

    Window::new("字体属性编辑器")
        .default_pos([300.0, 220.0])
        .default_size([400.0, 550.0])
        .resizable(true)
        .show(egui_ctx, |ui| {
            render_contents(ui);
        });
}

fn render_contents(ui: &mut egui::Ui) {
    // 获取编辑缓存的锁
    let Ok(mut font) = EDIT_CACHE.lock() else {
        ui.label("无法锁定编辑缓存");
        return;
    };

    ui.horizontal(|ui| {
        ui.heading("GDI 字体配置");

        // 从全局同步最新的字体到编辑器
        if ui
            .button("重置/同步")
            .on_hover_text("从当前生效的字体重新加载")
            .clicked()
        {
            *font = custom_font::get_font();
        }

        ui.separator();

        // 将缓冲区的内容真正应用到全局管理器
        if ui.button("保存并应用").clicked()
            && let Err(e) = custom_font::set_font(font.clone())
        {
            crate::debug!("应用字体失败: {:?}", e);
        }
    });

    ui.separator();

    // --- 字体名称处理 ---
    let mut name_str = String::from_utf16_lossy(
        &font
            .face_name
            .iter()
            .take_while(|&&c| c != 0)
            .cloned()
            .collect::<Vec<_>>(),
    );

    ui.horizontal(|ui| {
        ui.label("字体名称:");
        if ui.text_edit_singleline(&mut name_str).changed() {
            let mut new_face = [0u16; 32];
            let utf16_input: Vec<u16> = name_str.encode_utf16().collect();
            let len = utf16_input.len().min(31);
            new_face[..len].copy_from_slice(&utf16_input[..len]);
            font.face_name = new_face;
        }
    });

    ui.separator();

    // --- 数值属性 ---
    egui::Grid::new("font_property_grid")
        .num_columns(2)
        .spacing([10.0, 10.0])
        .show(ui, |ui| {
            ui.label("高度 (lfHeight):");
            ui.add(DragValue::new(&mut font.height).range(-128..=128));
            ui.end_row();

            ui.label("宽度 (lfWidth):");
            ui.add(DragValue::new(&mut font.width).range(0..=128));
            ui.end_row();

            ui.label("倾斜度 (lfEscapement):");
            ui.add(DragValue::new(&mut font.escapement).range(0..=3600));
            ui.end_row();

            ui.label("粗细 (lfWeight):");
            ui.add(DragValue::new(&mut font.weight).range(0..=1000).speed(10));
            ui.end_row();

            ui.label("字符集 (lfCharSet):");
            ui.add(DragValue::new(&mut font.char_set));
            ui.end_row();
        });

    // --- 布尔/位属性 ---
    ui.horizontal(|ui| {
        let mut italic = font.italic != 0;
        if ui.checkbox(&mut italic, "斜体").changed() {
            font.italic = italic as u8;
        }

        let mut underline = font.underline != 0;
        if ui.checkbox(&mut underline, "下划线").changed() {
            font.underline = underline as u8;
        }

        let mut strike_out = font.strike_out != 0;
        if ui.checkbox(&mut strike_out, "删除线").changed() {
            font.strike_out = strike_out as u8;
        }
    });

    // --- 搜集到的原始字体列表 ---
    #[cfg(feature = "enable_collect_host_font_config")]
    {
        ui.separator();
        ui.label(egui::RichText::new("已搜集的宿主字体:").strong());

        let collected = crate::hook::components::font_manager::COLLECTED_FONTS
            .read()
            .expect("Lock failed");

        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                if collected.is_empty() {
                    ui.weak("（列表为空）");
                }

                for host_font in collected.iter() {
                    let host_name = String::from_utf16_lossy(
                        &host_font
                            .face_name
                            .iter()
                            .take_while(|&&c| c != 0)
                            .cloned()
                            .collect::<Vec<_>>(),
                    );

                    if ui
                        .button(format!("{} (H:{})", host_name, host_font.height))
                        .clicked()
                    {
                        // 直接同步到编辑器缓冲区
                        *font = host_font.clone();
                    }
                }
            });
    }
}

pub fn is_visible() -> bool {
    IS_VISIBLE.load(Ordering::Acquire)
}

pub fn set_visible(visible: bool) {
    // 当打开窗口时，自动从全局管理器同步一次状态到缓冲区
    if visible && let Ok(mut font) = EDIT_CACHE.lock() {
        *font = custom_font::get_font();
    }
    IS_VISIBLE.store(visible, Ordering::Release);
}

pub fn init() -> crate::Result<()> {
    Ok(())
}

pub fn attach_cleanup() -> crate::Result<()> {
    custom_font::save_and_cleanup()?;
    Ok(())
}
