#[cfg(feature = "enable_worker_thread")]
use crate::worker_thread::LoopAction;

pub trait WorkerMainTick: Send + Sync + 'static {
    /// 工作线程的主循环 Tick 回调
    ///
    /// # 返回值
    /// - `Ok(LoopAction)`: 用于控制worker_thread的行为
    /// - `Err`: 出现错误
    ///
    /// # 注意事项
    /// - 此处严禁执行高耗时的阻塞操作（如同步 IO、复杂的循环计算），否则会直接降低渲染帧率（FPS）。
    #[cfg(feature = "enable_worker_thread")]
    fn on_worker_main_tick() -> crate::Result<LoopAction> {
        Ok(LoopAction::Continue)
    }
}
