#[cfg(feature = "text_extracting")]
use std::sync::Mutex;

#[cfg(feature = "text_extracting")]
use std::sync::LazyLock;
#[cfg(feature = "text_extracting")]
use translate_utils::text::{Item, Text};

use crate::code_cvt::TextVec;

#[cfg(feature = "text_extracting")]
static EXTRACTED_ITEMS: LazyLock<Mutex<Text>> = LazyLock::new(|| Mutex::new(Text::new()));

/// 添加一项条目
#[cfg(feature = "text_extracting")]
pub fn add_item(item: Item) {
    EXTRACTED_ITEMS.lock().unwrap().add_item(item);
}

/// 读取raw.json（如果有），加载之前提取的数据
#[cfg(feature = "text_extracting")]
pub fn load_initial_extracted_items_from_json() {
    match Text::from_path("./raw.json") {
        Ok(extracted_items) => *EXTRACTED_ITEMS.lock().unwrap() = extracted_items,
        Err(e) => crate::debug!("Read raw.json failed with {e}"),
    };
}

/// 将提取的条目输出到json文件中
#[cfg(feature = "text_extracting")]
pub fn save_extracted_items_to_json() {
    let mut text = EXTRACTED_ITEMS.lock().unwrap();

    text.dedup();

    if let Err(e) = text.write_to_path("./raw.json") {
        crate::debug!("Write raw.json failed with {e}");
    }
}

#[cfg(not(feature = "text_extracting"))]
mod text_patch_data {
    translate_macros::generated_text_patch_data!("assets/raw_text" => "assets/translated_text");
}

/// 获取与原文对应的译文
#[cfg(not(feature = "text_extracting"))]
#[allow(dead_code)]
pub fn lookup(original_message: &str) -> Option<&'static str> {
    text_patch_data::lookup(original_message)
}

/// 处理文本，`text_extracting` 特性开启时添加提取条目，否则返回译文（如果有）
pub fn lookup_or_add(message: &str) -> Option<&'static str> {
    #[cfg(feature = "text_extracting")]
    {
        crate::text_patch::add_item(Item::new(message));
        None
    }

    #[cfg(not(feature = "text_extracting"))]
    crate::text_patch::lookup(message)
}

/// 接受ansi字符串，`text_extracting` 特性开启时添加提取条目，否则返回u16编码带NULL的译文（如果有）
pub fn lookup_or_add_ansi_wide(ansi_slice: &[u8]) -> Option<TextVec<u16>> {
    let wide_text = crate::code_cvt::ansi_to_wide_char(ansi_slice);
    if wide_text.contains(&0xFFFDu16) {
        return None;
    }

    // wide_char_to_utf8 保证输出合法 UTF-8
    let msg_text = unsafe {
        String::from_utf8_unchecked(crate::code_cvt::wide_char_to_utf8(&wide_text).to_vec())
    };

    crate::text_patch::lookup_or_add(&msg_text)
        .map(|trans_msg| crate::code_cvt::utf8_to_wide_char_with_null(trans_msg.as_bytes()))
}
