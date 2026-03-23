// 声明所有的Hook接口的模块文件
translate_macros::expand_by_files!("src/hook/api_hooks" => {
    #[allow(dead_code, unused_variables)]
    pub mod __file__;
});
