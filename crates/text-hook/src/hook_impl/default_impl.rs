/// 默认实现的钩子，应该可以应对大部分场景
#[allow(dead_code)]
#[derive(Default)]
pub struct DefaultImplHook;

impl crate::hook::CoreHook for DefaultImplHook {}

#[cfg(feature = "text_hook")]
impl crate::hook::text_hook::TextHook for DefaultImplHook {}

#[cfg(feature = "file_hook")]
impl crate::hook::file_hook::FileHook for DefaultImplHook {}

#[cfg(feature = "locale_hook")]
impl crate::hook::locale_hook::LocaleHook for DefaultImplHook {}

#[cfg(feature = "window_hook")]
impl crate::hook::window_hook::WindowHook for DefaultImplHook {}
