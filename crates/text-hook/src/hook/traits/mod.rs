use windows_sys::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Diagnostics::Debug::CONTEXT;

use crate::overlay::OverlayContext;
use crate::utils::hwbp::HwReg;
#[cfg(feature = "worker_thread")]
use crate::worker_thread::LoopAction;

// 声明所有的Hook接口的模块文件，并导出Trait
translate_macros::expand_by_files!("src/hook/traits" => {
    #[cfg(feature = __file_str__)]
    pub use __file__::__file_pascal__;

    #[cfg(feature = __file_str__)]
    #[allow(dead_code)]
    pub mod __file__;
});

pub trait CoreHook: Send + Sync + 'static {
    /// 延迟附加回调，在程序入口点被调用时执行
    ///
    /// 此时程序已经完成基本的初始化，可以安全地进行各种需要完整运行环境的操作。
    /// 适合执行那些在`PROCESS_ATTACH`阶段可能导致死锁的操作。
    #[cfg(feature = "delayed_attach")]
    fn on_delayed_attach() {}

    /// 进程附加回调，在`DllMain`的`PROCESS_ATTACH`分支中调用
    ///
    /// 注意：此时程序初始化可能不完整，某些操作（如创建线程、加载DLL等）可能导致死锁。
    /// 如果有此类操作，请使用`on_delayed_attach`方法。
    fn on_process_attach(_hinst_dll: HMODULE) {}

    /// 进程附加清理回调，会在开启`attach_clean_up`时调用
    ///
    /// 此时可以进行安全的各种清理操作的，比如保存文件，清理临时文件等等。
    #[cfg(feature = "attach_clean_up")]
    fn on_process_attach_clean_up() {}

    /// 进程分离回调，在`DllMain`的`PROCESS_DETACH`分支中调用
    ///
    /// 在这个方法中应该执行所有最终的清理操作。
    /// 注意不要执行任何不要会导致死锁的操作，如果必须，请选择使用`on_process_attach_clean_up`。
    fn on_process_detach(_hinst_dll: HMODULE, _process_terminated: bool) {}

    /// WinEvent 回调，在通用 WinEventHook 收到事件后调用
    ///
    /// # 参数
    /// - `_event`: 事件类型（如 `EVENT_SYSTEM_FOREGROUND`）
    /// - `_hwnd`: 关联窗口句柄，可能为空
    /// - `_id_object`: 对象 ID（如 `OBJID_WINDOW`）
    /// - `_id_child`: 子对象 ID
    /// - `_id_event_thread`: 触发事件的线程 ID
    /// - `_dwms_event_time`: 事件触发时间戳（毫秒）
    #[cfg(feature = "win_event_hook")]
    fn on_win_event_triggered(
        _event: u32,
        _hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _id_event_thread: u32,
        _dwms_event_time: u32,
    ) {
    }

    /// 硬件断点命中回调，在 VEH 处理程序检测到 `EXCEPTION_SINGLE_STEP` 时调用
    ///
    /// # 参数
    /// - `_context`: 异常发生时的线程上下文（寄存器状态、调试寄存器等），允许修改
    /// - `_reg`: 命中的硬件断点寄存器（DR0-DR3），指示哪个断点触发
    ///
    /// # 返回值
    /// - `true`: 命中后**删除**该硬件断点，VEH 处理程序会自动清除 DR7 中对应的局部使能位（L0-L3）
    /// - `false`: **保留**硬件断点，VEH 处理程序仅设置 EFLAGS.RF 位跳过当前指令，断点继续生效
    ///
    /// # 注意事项
    /// - 执行断点（Execute）命中时，返回 `false` 会自动设置 RF 位防止立即重触发；其他类型（Write/Access）无需此处理
    /// - 若返回 `true`，断点被清除后该 `HwReg` 可被重新用于新的硬件断点
    /// - 此方法在 VEH 异常处理上下文中执行，**禁止**调用可能引发异常的 API（如内存分配、同步原语），仅限修改寄存器、内存 patch 等原子操作
    #[cfg(feature = "veh")]
    fn on_hwbp_hit(_context: &mut CONTEXT, _reg: HwReg) -> bool {
        true
    }

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
    /// - `Some(LRESULT)`: 表示消息已被 Hook 消费。框架将直接返回此值给系统，**不再**调用 `DefWindowProcW`。
    /// - `None`: 表示 Hook 不关心此消息。框架将自动调用 `DefWindowProcW` 进行系统默认处理。
    ///
    /// # 注意事项
    /// - 此方法在 **Overlay 窗口所属线程**（`worker_thread`）的消息循环中执行。
    /// - **禁止**在此处执行任何阻塞操作，否则会导致窗口失去响应或渲染卡顿。
    /// - 如果打算实现点击穿透之外的交互（如菜单、按钮），需要在此处通过 `egui-winit` 或手动逻辑
    ///   判断鼠标是否落在 UI 元素上，并据此决定是否拦截消息。
    #[cfg(feature = "overlay")]
    fn on_overlay_wnd_proc(
        _hwnd: HWND,
        _msg: u32,
        _w_param: WPARAM,
        _l_param: LPARAM,
    ) -> Option<LRESULT> {
        None
    }

    /// Overlay 渲染回调
    ///
    /// # 参数
    /// - `_context`: 包含当前 Overlay 运行状态的上下文引用
    ///
    /// # 注意事项
    /// - 此方法在 `worker_thread` 中执行，请确保绘制操作的线程安全性。
    /// - 严禁在此回调中执行耗时过长的阻塞操作，否则会拖慢渲染帧率及消息循环。
    #[cfg(feature = "overlay")]
    fn on_overlay_render(_context: &OverlayContext) {}

    /// 工作线程的主循环 Tick 回调
    ///
    /// # 返回值
    /// - `LoopAction`: 用于控制worker_thread的行为
    ///
    /// # 注意事项
    /// - 此处严禁执行高耗时的阻塞操作（如同步 IO、复杂的循环计算），否则会直接降低渲染帧率（FPS）。
    #[cfg(feature = "worker_thread")]
    fn on_worker_main_tick() -> LoopAction {
        LoopAction::Continue
    }
}
