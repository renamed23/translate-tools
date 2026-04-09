pub trait ProcessAttachCleanup: Send + Sync + 'static {
    /// 进程附加清理回调，通常配合`bind_lifecycle_guard`使用
    ///
    /// 此时可以进行安全的各种清理操作的，比如保存文件，清理临时文件等等。
    fn on_process_attach_cleanup() -> crate::Result<()> {
        Ok(())
    }
}
