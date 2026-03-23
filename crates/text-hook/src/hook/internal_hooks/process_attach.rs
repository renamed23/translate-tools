use windows_sys::Win32::Foundation::HMODULE;

pub trait ProcessAttach: Send + Sync + 'static {
    /// 进程附加回调，在`DllMain`的`PROCESS_ATTACH`分支中调用
    ///
    /// 注意：此时程序初始化可能不完整，某些操作（如创建线程、加载DLL等）可能导致死锁。
    /// 如果有此类操作，请使用`on_delayed_attach`方法。
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        Ok(())
    }
}
