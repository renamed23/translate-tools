// 声明所有核心生命周期/运行时回调 trait 模块
translate_macros::expand_by_files!("src/hook/core_hook" => {
    #[allow(dead_code, unused_variables)]
    pub mod __file__;

    pub use __file__::*;
});
