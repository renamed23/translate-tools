/// 默认实现的钩子，应该可以应对大部分场景
#[allow(dead_code)]
#[derive(Default)]
pub struct DefaultImplHook;

impl crate::hook::CoreHook for DefaultImplHook {}

// 为 DefaultImplHook 实现所有可用的特性相关钩子接口
translate_macros::expand_by_files!("src/hook" => {
    #[cfg(feature = __file_str__)]
    impl crate::hook::__file__::__file_pascal__ for DefaultImplHook {}
});
