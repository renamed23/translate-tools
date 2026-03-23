// 声明所有的Hook实现的模块文件
translate_macros::expand_by_files!("src/hook/impls" => {
    #[cfg(feature = __file_str__)]
    pub mod __file__;
});

// 在`src/hook_impl`搜索可用的Hook实现类型
translate_macros::search_hook_impls!("src/hook/impls" => pub type HookImplType);
