#[cfg(feature = "enable_thread_manager")]
use std::thread::JoinHandle;

/// 线程管理器启动回调。
///
/// 在 [`thread_manager::start`](crate::thread_manager::start) 中被调用。
/// 实现者可以通过返回 `Some(JoinHandle<()>)` 来注入自定义线程，
/// 该线程将由 [`thread_manager`](crate::thread_manager) 统一管理生命周期和停止。
///
/// 自定义线程内应通过 [`thread_manager::is_shutdown`](crate::thread_manager::is_shutdown)
/// 轮询停止信号。
pub trait ThreadManagerStart: Send + Sync + 'static {
    /// 在线程管理器启动时调用。
    ///
    /// # 返回值
    /// - `Some(JoinHandle<()>)`: 返回一个由线程管理器托管的线程句柄。
    ///   该线程将在 [`thread_manager::stop`](crate::thread_manager::stop) 中被等待。
    /// - `None`: 不添加自定义线程。
    #[cfg(feature = "enable_thread_manager")]
    fn on_thread_manager_start() -> Option<JoinHandle<()>> {
        None
    }
}
