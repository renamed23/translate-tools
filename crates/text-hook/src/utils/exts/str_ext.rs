pub trait StrExt {
    /// 在已加载的文本补丁数据库中查找对应的翻译或映射文本。
    /// 仅在开启 `text_patch` 且未开启 `text_extracting` 时可用。
    #[cfg(all(feature = "text_patch", not(feature = "text_extracting")))]
    fn lookup(&self) -> crate::Result<&'static str>;

    /// 查找对应的文本补丁，如果数据库中不存在该项，则将其添加到待处理列表中。
    /// 通常用于开发阶段的文本自动提取。
    #[cfg(feature = "text_patch")]
    fn lookup_or_add_item(&self) -> crate::Result<&'static str>;
}

impl StrExt for str {
    #[cfg(all(feature = "text_patch", not(feature = "text_extracting")))]
    fn lookup(&self) -> crate::Result<&'static str> {
        crate::text_patch::lookup(self)
    }

    #[cfg(feature = "text_patch")]
    fn lookup_or_add_item(&self) -> crate::Result<&'static str> {
        crate::text_patch::lookup_or_add_item(self)
    }
}
