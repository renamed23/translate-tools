use windows_sys::Win32::Foundation::HMODULE;

pub trait ProcessDetach: Send + Sync + 'static {
    /// 进程分离回调，在`DllMain`的`PROCESS_DETACH`分支中调用
    ///
    /// 在这个方法中应该执行所有最终的清理操作。
    /// 注意不要执行任何不要会导致死锁的操作，如果必须，请选择使用`on_process_attach_cleanup`。
    fn on_process_detach(_hinst_dll: HMODULE, _process_terminated: bool) -> crate::Result<()> {
        Ok(())
    }
}
