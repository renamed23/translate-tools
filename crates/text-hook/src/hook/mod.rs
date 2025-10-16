#[cfg(feature = "file_hook")]
#[allow(dead_code)]
pub mod file_hook;

#[cfg(feature = "text_hook")]
#[allow(dead_code)]
pub mod text_hook;

use once_cell::sync::OnceCell;

use crate::debug;
use crate::hook_impl::HookImplType;

pub trait CoreHook: Send + Sync + 'static {
    fn enable_hooks(&self) {}
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
