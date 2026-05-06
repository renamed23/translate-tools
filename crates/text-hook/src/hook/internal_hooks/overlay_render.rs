#[cfg(feature = "enable_overlay")]
use crate::overlay::OverlayContext;

pub trait OverlayRender: Send + Sync + 'static {
    /// Overlay 渲染回调
    ///
    /// # 参数
    /// - `_context`: 包含当前 Overlay 运行状态的上下文引用
    ///
    /// # 注意事项
    /// - 此方法在 `ui_thread` 中执行，请确保绘制操作的线程安全性。
    /// - 严禁在此回调中执行耗时过长的阻塞操作，否则会拖慢渲染帧率及消息循环。
    #[cfg(feature = "enable_overlay")]
    fn on_overlay_render(_context: &mut OverlayContext) -> crate::Result<()> {
        Ok(())
    }
}
