pub(crate) mod impls;
pub(crate) mod traits;

#[allow(dead_code)]
pub(crate) mod trait_impls;

use once_cell::sync::OnceCell;

use crate::debug;
use crate::hook::impls::HookImplType;

static HOOK_INSTANCE: OnceCell<HookImplType> = OnceCell::new();

/// 设置全局钩子实例
#[allow(dead_code)]
pub fn set_hook_instance(h: HookImplType) {
    if HOOK_INSTANCE.set(h).is_err() {
        debug!("Hook instance already set");
    }
}

/// 获取全局钩子实例
#[inline]
pub fn hook_instance() -> &'static HookImplType {
    HOOK_INSTANCE.get().expect("Hook not initialized")
}

/// 开启所有的特性相关钩子
#[allow(dead_code)]
pub fn enable_featured_hooks() {
    translate_macros::expand_by_files!("src/hook/traits" => {
        #[cfg(feature= __file_str__)]
        traits::__file__::enable_featured_hooks();
    });
}

/// 关闭所有的特性相关钩子
#[allow(dead_code)]
pub fn disable_featured_hooks() {
    translate_macros::expand_by_files!("src/hook/traits" => {
        #[cfg(feature= __file_str__)]
        traits::__file__::disable_featured_hooks();
    });
}
