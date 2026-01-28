use translate_macros::DefaultHook;

/// 默认实现的钩子，应该可以应对大部分场景
#[allow(dead_code)]
#[derive(Default, DefaultHook)]
pub struct DefaultImplHook;

impl crate::hook::traits::CoreHook for DefaultImplHook {}
