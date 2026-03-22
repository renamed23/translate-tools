pub trait StrExt {
    /// 在已加载的文本补丁数据库中查找对应的翻译或映射文本。
    /// 仅在开启 `enable_text_patch` 且未开启 `extract_text` 时可用。
    #[cfg(all(feature = "enable_text_patch", not(feature = "extract_text")))]
    fn lookup(&self) -> Option<&'static str>;

    /// 查找对应的文本补丁，如果数据库中不存在该项，则将其存储到待处理列表中。
    /// 通常用于开发阶段的文本自动提取。
    #[cfg(feature = "enable_text_patch")]
    fn lookup_or_store(&self) -> Option<&'static str>;
}

impl StrExt for str {
    #[cfg(all(feature = "enable_text_patch", not(feature = "extract_text")))]
    fn lookup(&self) -> Option<&'static str> {
        crate::text_patch::lookup(self)
    }

    #[cfg(feature = "enable_text_patch")]
    fn lookup_or_store(&self) -> Option<&'static str> {
        crate::text_patch::lookup_or_store(self)
    }
}
