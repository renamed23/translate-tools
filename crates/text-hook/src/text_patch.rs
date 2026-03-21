use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "text_extracting")]  {
        use std::sync::{LazyLock, Mutex};
        static EXTRACTED_ITEMS: LazyLock<Mutex<indexmap::IndexSet<serde_json::Value>>> =
            LazyLock::new(|| Mutex::new(indexmap::IndexSet::new()));

        /// 存储一项条目
        pub fn store_item(item: serde_json::Value) {
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
        use std::sync::atomic::{AtomicUsize, Ordering};

        mod text_patch_data {
            translate_macros::generated_text_patch_data!("assets/raw_text" => "assets/translated_text");
        }

        static LAST_LOOKUP_INDEX: AtomicUsize = AtomicUsize::new(usize::MAX);

        #[allow(dead_code)]
        #[derive(Clone, Copy)]
        pub struct LookupResult {
            pub translated: &'static str,
            pub matched_index: Option<usize>,
        }

        fn get_last_lookup_index() -> Option<usize> {
            let index = LAST_LOOKUP_INDEX.load(Ordering::Relaxed);
            (index != usize::MAX).then_some(index)
        }

        fn set_last_lookup_index(index: usize) {
            LAST_LOOKUP_INDEX.store(index, Ordering::Relaxed);
        }

        /// 获取与原文对应的译文及其命中的文本索引信息。
        pub fn lookup_result(original_message: &str) -> Option<LookupResult> {
            let result = text_patch_data::lookup_result(original_message, get_last_lookup_index())?;
            if let Some(index) = result.matched_index {
                set_last_lookup_index(index);
            }
            crate::debug!(raw
                "Lookup '{original_message}', got '{}' ({:?})",
                result.translated,
                result.matched_index
            );
            Some(LookupResult {
                translated: result.translated,
                matched_index: result.matched_index,
            })
        }

        /// 获取与原文对应的译文
        #[allow(dead_code)]
        pub fn lookup(original_message: &str) -> Option<&'static str> {
            lookup_result(original_message).map(|result| result.translated)
        }
    }
}

/// 处理文本，`text_extracting` 特性开启时存储提取条目，否则返回译文（如果有）
pub fn lookup_or_store(message: &str) -> Option<&'static str> {
    cfg_if! {
        if #[cfg(feature = "text_extracting")] {
            crate::text_patch::store_item(serde_json::json!({"message": message}));
            crate::debug!("Added item for message: {message}");
            None
        } else {
            crate::text_patch::lookup(message)
        }
    }
}
