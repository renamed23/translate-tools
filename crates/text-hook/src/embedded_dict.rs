use once_cell::sync::Lazy;

use crate::translated_dict::TranslatedDict;

translate_macros::flate!(
    static TRANSLATED_JSON: str from "assets\\translated.json"
);

/// 内嵌的TranslatedDict，可用于在运行期进行映射
pub static DICT: Lazy<TranslatedDict> =
    Lazy::new(|| TranslatedDict::from_string(&TRANSLATED_JSON).expect("Invalid JSON"));
