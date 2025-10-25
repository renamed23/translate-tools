use once_cell::sync::OnceCell;
use windows_sys::Win32::Foundation::HMODULE;

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
    ///
    /// 在这个方法中应该安装所有需要的API钩子。
    /// 注意：此时程序初始化可能不完整，某些操作可能导致死锁。
    fn enable_hooks(&self) {}

    /// 禁用钩子，会在`PROCESS_DETACH`时调用
    ///
    /// 在这个方法中应该卸载所有安装的API钩子，恢复原始函数。
    fn disable_hooks(&self) {}

    /// 延迟附加回调，在程序入口点被调用时执行
    ///
    /// 此时程序已经完成基本的初始化，可以安全地进行各种需要完整运行环境的操作。
    /// 适合执行那些在`PROCESS_ATTACH`阶段可能导致死锁的操作。
    #[cfg(feature = "delayed_attach")]
    fn on_delayed_attach(&self) {}

    /// 延迟附加清理回调，在禁用延迟附加钩子时调用
    ///
    /// 用于清理在`on_delayed_attach`中分配的资源。
    /// 这个方法的调用时机早于`on_process_detach`。
    #[cfg(feature = "delayed_attach")]
    fn on_delayed_attach_clean(&self) {}

    /// 进程附加回调，在`DllMain`的`PROCESS_ATTACH`分支中调用
    ///
    /// 注意：此时程序初始化可能不完整，某些操作（如创建线程、加载DLL等）可能导致死锁。
    /// 如果有此类操作，请使用`on_delayed_attach`方法。
    fn on_process_attach(&self, _hinst_dll: HMODULE) {}

    /// 进程分离回调，在`DllMain`的`PROCESS_DETACH`分支中调用
    ///
    /// 在这个方法中应该执行所有最终的清理操作。
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
