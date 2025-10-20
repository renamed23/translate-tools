use once_cell::sync::OnceCell;
use winapi::shared::minwindef::HMODULE;

use crate::debug;
use crate::hook_impl::HookImplType;

// 声明所有的Hook接口的模块文件
translate_macros::expand_by_files!("src/hook" => {
    #[cfg(feature = __file_str__)]
    #[allow(dead_code)]
    pub mod __file__;
});

pub trait CoreHook: Send + Sync + 'static {
    fn enable_hooks(&self) {}
    fn disable_hooks(&self) {}
    fn on_process_attach(&self, _hinst_dll: HMODULE) {}
    fn on_process_detach(&self, _hinst_dll: HMODULE) {}
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

/// 开启所有的特性相关钩子
#[allow(dead_code)]
pub fn enable_featured_hooks() {
    translate_macros::expand_by_files!("src/hook" => {
        #[cfg(feature= __file_str__)]
        __file__::enable_featured_hooks();
    });
}

/// 关闭所有的特性相关钩子
#[allow(dead_code)]
pub fn disable_featured_hooks() {
    translate_macros::expand_by_files!("src/hook" => {
        #[cfg(feature= __file_str__)]
        __file__::disable_featured_hooks();
    });
}
