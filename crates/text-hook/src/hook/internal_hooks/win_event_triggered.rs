#[cfg(feature = "enable_win_event_hook")]
use windows_sys::Win32::Foundation::HWND;

pub trait WinEventTriggered: Send + Sync + 'static {
    /// WinEvent 回调，在通用 WinEventHook 收到事件后调用
    ///
    /// # 参数
    /// - `_event`: 事件类型（如 `EVENT_SYSTEM_FOREGROUND`）
    /// - `_hwnd`: 关联窗口句柄，可能为空
    /// - `_id_object`: 对象 ID（如 `OBJID_WINDOW`）
    /// - `_id_child`: 子对象 ID
    /// - `_id_event_thread`: 触发事件的线程 ID
    /// - `_dwms_event_time`: 事件触发时间戳（毫秒）
    #[cfg(feature = "enable_win_event_hook")]
    fn on_win_event_triggered(
        _event: u32,
        _hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _id_event_thread: u32,
        _dwms_event_time: u32,
    ) -> crate::Result<()> {
        Ok(())
    }
}
