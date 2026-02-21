use windows_sys::Win32::{
    Foundation::{EXCEPTION_SINGLE_STEP, HANDLE, NTSTATUS},
    System::Diagnostics::Debug::{
        AddVectoredExceptionHandler, EXCEPTION_CONTINUE_EXECUTION, EXCEPTION_CONTINUE_SEARCH,
        EXCEPTION_POINTERS, RaiseException, RemoveVectoredExceptionHandler,
    },
};

use crate::{
    hook::{impls::HookImplType, traits::CoreHook},
    utils::hwbp::{HwBreakpointLen, HwBreakpointType, HwReg},
};

static mut VEH_HANDLE: Option<HANDLE> = None;

/// 安装 VEH 处理程序
///
/// # Safety
/// - 必须在 DLL attach 时调用，且仅调用一次
/// - 非线程安全，需由调用者保证初始化顺序
pub unsafe fn install_veh_handler(first: bool) -> crate::Result<()> {
    crate::debug!("Installing VEH handler (first={first})");

    #[allow(clippy::redundant_pattern_matching)]
    if unsafe { matches!(VEH_HANDLE, Some(_)) } {
        return Ok(());
    }

    let first = u32::from(first);
    let handle = unsafe { AddVectoredExceptionHandler(first, Some(veh_handler)) };
    if handle.is_null() {
        crate::bail!("AddVectoredExceptionHandler failed");
    }

    unsafe { VEH_HANDLE = Some(handle) };

    Ok(())
}

/// 卸载 VEH 处理程序
///
/// # Safety
/// - 必须在 DLL detach 时调用，且仅调用一次
pub unsafe fn uninstall_veh_handler() -> crate::Result<()> {
    crate::debug!("Uninstalling VEH handler");

    unsafe {
        if let Some(handle) = VEH_HANDLE {
            VEH_HANDLE = None;
            if RemoveVectoredExceptionHandler(handle) != 0 {
                Ok(())
            } else {
                crate::bail!("RemoveVectoredExceptionHandler failed");
            }
        } else {
            crate::bail!("VEH handler is not installed");
        }
    }
}

/// 自定义异常代码：请求在当前线程设置硬件断点
///
/// 使用 RaiseException 触发，参数传递断点配置（地址/类型/长度/寄存器）
pub const EXCEPTION_SET_HW_BREAK: NTSTATUS = 0xEABC0001u32 as _;

/// 请求在当前线程上下文设置硬件断点
///
/// 通过自触发异常进入 VEH，在异常处理函数中安全修改 DR 寄存器。
/// 避免直接操作 DR 寄存器导致的线程同步问题。
pub fn request_set_hw_breakpoint_on_current_thread(
    addr: usize,
    kind: HwBreakpointType,
    len: HwBreakpointLen,
    reg: HwReg,
) {
    crate::debug!(
        "Requesting to set HWBP on current thread: addr={addr:#x}, kind={kind:?}, len={len:?}, reg={reg:?}"
    );

    let args = [addr, kind as usize, len as usize, reg as usize];
    unsafe {
        RaiseException(
            EXCEPTION_SET_HW_BREAK as u32,
            0,
            args.len() as u32,
            args.as_ptr(),
        )
    };
}

/// VEH 异常处理函数
///
/// 处理两类异常：
/// 1. EXCEPTION_SET_HW_BREAK: 响应设置断点请求，解包参数并调用 `set_hw_break_in_context`
/// 2. EXCEPTION_SINGLE_STEP: 处理断点命中，回调用户逻辑，管理 DR6/EFlags.RF 位
///
/// # 死锁警告
/// SINGLE_STEP 处理中若触发堆分配（String/Vec/线程创建），且目标线程已持有堆锁，将导致死锁。
/// 生产环境建议改用无锁通知机制。
unsafe extern "system" fn veh_handler(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    // 基础校验
    let exception_info = if exception_info.is_null() {
        return EXCEPTION_CONTINUE_SEARCH;
    } else {
        unsafe { &*exception_info }
    };
    let record = if exception_info.ExceptionRecord.is_null() {
        return EXCEPTION_CONTINUE_SEARCH;
    } else {
        unsafe { &*exception_info.ExceptionRecord }
    };
    let context = if exception_info.ContextRecord.is_null() {
        return EXCEPTION_CONTINUE_SEARCH;
    } else {
        unsafe { &mut *exception_info.ContextRecord }
    };

    // 处理设置断点请求
    if record.ExceptionCode == EXCEPTION_SET_HW_BREAK {
        if record.NumberParameters == 4 {
            crate::debug!(
                "VEH handler received set HWBP request: addr={:#x}, kind={}, len={}, reg={}",
                record.ExceptionInformation[0],
                record.ExceptionInformation[1],
                record.ExceptionInformation[2],
                record.ExceptionInformation[3],
            );

            let addr = record.ExceptionInformation[0];
            let kind = match record.ExceptionInformation[1] {
                0 => HwBreakpointType::Execute,
                1 => HwBreakpointType::Write,
                2 => HwBreakpointType::Access,
                _ => return EXCEPTION_CONTINUE_SEARCH,
            };
            let len = match record.ExceptionInformation[2] {
                0 => HwBreakpointLen::Byte1,
                1 => HwBreakpointLen::Byte2,
                2 => HwBreakpointLen::Byte4,
                #[cfg(target_arch = "x86_64")]
                3 => HwBreakpointLen::Byte8,
                _ => return EXCEPTION_CONTINUE_SEARCH,
            };
            let reg = match record.ExceptionInformation[3] {
                0 => HwReg::Dr0,
                1 => HwReg::Dr1,
                2 => HwReg::Dr2,
                3 => HwReg::Dr3,
                _ => return EXCEPTION_CONTINUE_SEARCH,
            };

            if let Err(e) =
                crate::utils::hwbp::set_hw_break_in_context(context, addr, kind, len, reg)
            {
                crate::debug!("set_hw_break_in_context failed with {e:?}");
            }
            return EXCEPTION_CONTINUE_EXECUTION;
        }
        return EXCEPTION_CONTINUE_EXECUTION;
    }

    // 处理断点命中
    if record.ExceptionCode == EXCEPTION_SINGLE_STEP {
        let dr6 = context.Dr6 as usize;

        crate::debug!("VEH handler: SINGLE_STEP exception, DR6={:#x}", dr6);

        for i in 0..4usize {
            let hit_bit = 1usize << i;
            if (dr6 & hit_bit) != 0 {
                let reg: HwReg = unsafe { core::mem::transmute(i) };
                /* * 【重要风险警告 - 必读】
                 * 此处是你进行后续处理（如创建线程、分配内存、打印日志）的地方。
                 * * 事实风险：
                 * 虽然此时你已经从 Context 中删除了断点，但你依然处于 VEH 的异常上下文中。
                 * 如果触发断点的线程在进入 VEH 前已经持有了“堆锁”（Heap Lock），
                 * 而你在下面的逻辑中尝试分配内存（如 String/Vec/format!）或创建线程，
                 * 那么程序将会有极高概率发生【死锁】，且无法通过删除断点来解除。
                 * * 建议：出问题后，请将此处逻辑改为无锁信号通知模式。
                 */

                // 用户回调：返回 true 则清除断点
                if HookImplType::on_hwbp_hit(context, reg) {
                    crate::utils::hwbp::clear_hw_break_in_context(context, reg);
                }

                #[cfg(feature = "apply_1337_patch_on_hwbp_hit")]
                crate::x64dbg_1337_patch::apply();

                // 清除 DR6 命中标志
                context.Dr6 &= !0b1111;

                // 执行断点：设置 EFlags.RF 避免无限循环
                let dr7 = context.Dr7 as usize;
                let rw_bits = (dr7 >> (16 + i * 4)) & 0b11;
                if rw_bits == 0b00 {
                    context.EFlags |= 1 << 16;
                }

                return EXCEPTION_CONTINUE_EXECUTION;
            }
        }
    }

    EXCEPTION_CONTINUE_SEARCH
}
