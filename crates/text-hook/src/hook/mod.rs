pub(crate) mod impls;
pub(crate) mod traits;

#[allow(dead_code)]
pub(crate) mod trait_impls;

use once_cell::sync::OnceCell;

use crate::debug;
use crate::hook::impls::HookImplType;

mod hook_lists {
    translate_macros::expand_by_files!("src/hook/traits" => {
        #[cfg(feature = __file_str__)]
        use super::traits::__file__::*;
    });

    translate_macros::generate_hook_lists_from_json!(
        "constant_assets/featured_hook_lists.json",
        "assets/hook_lists.json"
    );
}

/// 从钩子列表中开启所有的钩子
#[allow(dead_code)]
pub fn enable_hooks_from_lists() {
    hook_lists::enable_hooks_from_lists();
}

/// 从钩子列表中关闭所有的钩子
#[allow(dead_code)]
pub fn disable_hooks_from_lists() {
    hook_lists::disable_hooks_from_lists();
}

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
