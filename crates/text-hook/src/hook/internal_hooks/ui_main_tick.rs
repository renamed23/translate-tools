#[cfg(feature = "enable_ui_thread")]
use crate::ui_thread::LoopAction;

/// UI 线程的主循环 Tick 回调。
pub trait UiMainTick: Send + Sync + 'static {
    /// UI 线程的主循环 Tick 回调
    ///
    /// 此方法在 [`ui_thread`](crate::ui_thread) 的每一帧中被调用一次。
    ///
    /// # 返回值
    /// - `Ok(LoopAction)`: 用于控制 UI 线程的行为
    /// - `Err`: 出现错误
    ///
    /// # 注意事项
    /// - 此处严禁执行高耗时的阻塞操作（如同步 IO、复杂的循环计算），否则会直接降低渲染帧率（FPS）。
    #[cfg(feature = "enable_ui_thread")]
    fn on_ui_main_tick() -> crate::Result<LoopAction> {
        Ok(LoopAction::Continue)
    }
}
