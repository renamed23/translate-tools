use windows_sys::Win32::Foundation::HMODULE;

// 声明所有的Hook接口的模块文件，并导出Trait
translate_macros::expand_by_files!("src/hook/traits" => {
    #[cfg(feature = __file_str__)]
    pub use __file__::__file_pascal__;

    #[cfg(feature = __file_str__)]
    #[allow(dead_code)]
    pub mod __file__;
});

pub trait CoreHook: Send + Sync + 'static {
    /// 启用钩子，如果未开启`delayed_attach`，会在`PROCESS_ATTACH`时调用；
    /// 否则会在入口点被调用。
    ///
    /// 在这个方法中应该安装所有需要的API钩子。
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
