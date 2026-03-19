use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "text_extracting")]  {
        use std::sync::{LazyLock, Mutex};
        static EXTRACTED_ITEMS: LazyLock<Mutex<indexmap::IndexSet<serde_json::Value>>> =
            LazyLock::new(|| Mutex::new(indexmap::IndexSet::new()));

        /// 添加一项条目
        pub fn add_item(item: serde_json::Value) {
            EXTRACTED_ITEMS.lock().unwrap().insert(item);
        }

        /// 读取raw.json（如果有），加载之前提取的数据
        pub fn load_initial_extracted_items_from_json() -> crate::Result<()> {
            let contents = std::fs::read_to_string("./raw.json")?;
            *EXTRACTED_ITEMS.lock().unwrap() = serde_json::from_str(&contents)?;
            Ok(())
        }

        /// 将提取的条目输出到json文件中
        pub fn save_extracted_items_to_json() -> crate::Result<()> {
            let text = EXTRACTED_ITEMS.lock().unwrap();
            let contents = serde_json::to_string_pretty(&*text)?;
            std::fs::write("./raw.json", contents)?;

            Ok(())
        }
    } else {
        mod text_patch_data {
            translate_macros::generated_text_patch_data!("assets/raw_text" => "assets/translated_text");
        }

        /// 获取与原文对应的译文
        #[allow(dead_code)]
        pub fn lookup(original_message: &str) -> Option<&'static str> {
            text_patch_data::lookup(original_message)
        }
    }
}

/// 处理文本，`text_extracting` 特性开启时添加提取条目，否则返回译文（如果有）
pub fn lookup_or_add_item(message: &str) -> Option<&'static str> {
    cfg_if! {
        if #[cfg(feature = "text_extracting")] {
            crate::text_patch::add_item(serde_json::json!({"message": message}));
            crate::debug!("Added item for message: {message}");
            None
        } else {
            crate::text_patch::lookup(message)
        }
    }
}
