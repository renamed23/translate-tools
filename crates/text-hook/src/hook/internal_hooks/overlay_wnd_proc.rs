#[cfg(feature = "enable_overlay")]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

pub trait OverlayWndProc: Send + Sync + 'static {
    /// Overlay 窗口消息回调
    ///
    /// # 说明
    /// 此方法直接挂载在 Overlay 窗口的 `WndProc` 中。实现者可以通过此接口拦截、
    /// 修改或响应发往 Overlay 窗口的所有 Windows 消息。
    ///
    /// # 参数
    /// - `hwnd`: Overlay 窗口本身的句柄。
    /// - `msg`: Windows 消息 ID（如 `WM_PAINT`, `WM_MOUSEMOVE`, `WM_SIZE` 等）。
    /// - `w_param`: 消息附加参数。
    /// - `l_param`: 消息附加参数。
    ///
    /// # 返回值
    /// - `Ok(Some(LRESULT))`: 表示消息已被 Hook 消费。框架将直接返回此值给系统，**不再**调用 `DefWindowProcW`。
    /// - `Ok(None)`: 表示 Hook 不关心此消息。框架将自动调用 `DefWindowProcW` 进行系统默认处理。
    /// - `Err`: 出现错误
    ///
    /// # 注意事项
    /// - 此方法在 **Overlay 窗口所属线程**（`ui_thread`）的消息循环中执行。
    /// - **禁止**在此处执行任何阻塞操作，否则会导致窗口失去响应或渲染卡顿。
    /// - 如果打算实现点击穿透之外的交互（如菜单、按钮），需要在此处通过 `egui-winit` 或手动逻辑
    ///   判断鼠标是否落在 UI 元素上，并据此决定是否拦截消息。
    #[cfg(feature = "enable_overlay")]
    fn on_overlay_wnd_proc(
        _hwnd: HWND,
        _msg: u32,
        _w_param: WPARAM,
        _l_param: LPARAM,
    ) -> crate::Result<Option<LRESULT>> {
        Ok(None)
    }
}
