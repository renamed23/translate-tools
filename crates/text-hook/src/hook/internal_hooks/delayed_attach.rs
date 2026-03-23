pub trait DelayedAttach: Send + Sync + 'static {
    /// 延迟附加回调，在程序入口点被调用时执行
    ///
    /// 此时程序已经完成基本的初始化，可以安全地进行各种需要完整运行环境的操作。
    /// 适合执行那些在`PROCESS_ATTACH`阶段可能导致死锁的操作。
    #[cfg(feature = "enable_delayed_attach")]
    fn on_delayed_attach() -> crate::Result<()> {
        Ok(())
    }
}
