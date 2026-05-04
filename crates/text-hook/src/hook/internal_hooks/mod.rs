// 声明所有核心生命周期/运行时回调 trait 模块
translate_macros::expand_by_files!("src/hook/internal_hooks" => {
    #[allow(dead_code, unused_variables)]
    pub mod __file_stem_ident__;

    pub use __file_stem_ident__::*;
});
