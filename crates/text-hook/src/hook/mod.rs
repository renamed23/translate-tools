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
    /// 启用钩子，会在`PROCESS_ATTACH`时调用
    fn enable_hooks(&self) {}

    /// 禁用钩子，会在`PROCESS_DETACH`时调用
    fn disable_hooks(&self) {}

    /// 会在主模块入口点时被调用
    #[cfg(feature = "delayed_attach")]
    fn on_delayed_attach(&self) {}

    /// 会在`PROCESS_ATTACH`时调用，若有会导致死锁的操作
    /// 请开启`delayed_attach`特性，并在`on_delayed_attach`中进行
    fn on_process_attach(&self, _hinst_dll: HMODULE) {}

    /// 会在`PROCESS_DETACH`时调用
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
