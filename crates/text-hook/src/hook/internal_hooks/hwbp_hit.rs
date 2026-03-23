#[cfg(feature = "enable_veh")]
use crate::utils::hwbp::HwReg;
#[cfg(feature = "enable_veh")]
use windows_sys::Win32::System::Diagnostics::Debug::CONTEXT;

pub trait HwbpHit: Send + Sync + 'static {
    /// 硬件断点命中回调，在 VEH 处理程序检测到 `EXCEPTION_SINGLE_STEP` 时调用
    ///
    /// # 参数
    /// - `_context`: 异常发生时的线程上下文（寄存器状态、调试寄存器等），允许修改
    /// - `_reg`: 命中的硬件断点寄存器（DR0-DR3），指示哪个断点触发
    ///
    /// # 返回值
    /// - `Ok(true)`: 命中后**删除**该硬件断点，VEH 处理程序会自动清除 DR7 中对应的局部使能位（L0-L3）
    /// - `Ok(false)`: **保留**硬件断点，VEH 处理程序仅设置 EFLAGS.RF 位跳过当前指令，断点继续生效
    /// - `Err`: 出现错误
    ///
    /// # 注意事项
    /// - 执行断点（Execute）命中时，返回 `false` 会自动设置 RF 位防止立即重触发；其他类型（Write/Access）无需此处理
    /// - 若返回 `Ok(true)`，断点被清除后该 `HwReg` 可被重新用于新的硬件断点
    /// - 此方法在 VEH 异常处理上下文中执行，**禁止**调用可能引发异常的 API（如内存分配、同步原语），仅限修改寄存器、内存 patch 等原子操作
    #[cfg(feature = "enable_veh")]
    fn on_hwbp_hit(_context: &mut CONTEXT, _reg: HwReg) -> crate::Result<bool> {
        Ok(true)
    }
}
