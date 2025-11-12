#[cfg(feature = "text_extracting")]
use std::sync::Mutex;

#[cfg(feature = "text_extracting")]
use once_cell::sync::Lazy;
#[cfg(feature = "text_extracting")]
use translate_utils::text::{Item, Text};

#[cfg(feature = "text_extracting")]
static EXTRACTED_ITEMS: Lazy<Mutex<Text>> = Lazy::new(|| Mutex::new(Text::new()));

/// 添加一项条目
#[cfg(feature = "text_extracting")]
pub fn add_item(item: Item) {
    EXTRACTED_ITEMS.lock().unwrap().add_item(item);
}

/// 读取raw.json（如果有），加载之前提取的数据
#[cfg(feature = "text_extracting")]
pub fn read_extracted_items_from_json() {
    match Text::from_path("./raw.json") {
        Ok(extracted_items) => *EXTRACTED_ITEMS.lock().unwrap() = extracted_items,
        Err(e) => crate::debug!("Read raw.json fails with {e}"),
    };
}

/// 将提取的条目输出到json文件中
#[cfg(feature = "text_extracting")]
pub fn write_extracted_items_to_json() {
    let mut text = EXTRACTED_ITEMS.lock().unwrap();

    text.dedup();

    if let Err(e) = text.write_to_path("./raw.json") {
        crate::debug!("Write raw.json fails with {e}");
    }
}

#[cfg(not(feature = "text_extracting"))]
mod text_patch_data {
    translate_macros::generated_text_patch_data!("assets/raw.json" => "assets/translated.json");
}

/// 获取与原名对应的译名
#[cfg(not(feature = "text_extracting"))]
pub fn lookup_name(original_name: &str) -> Option<&'static str> {
    text_patch_data::lookup_name(original_name)
}

/// 获取与原文对应的译文
#[cfg(not(feature = "text_extracting"))]
pub fn lookup_message(original_message: &str) -> Option<&'static str> {
    text_patch_data::lookup_message(original_message)
}
