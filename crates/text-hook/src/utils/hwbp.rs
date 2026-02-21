use windows_sys::Win32::System::Diagnostics::Debug::CONTEXT;

// 根据架构定义不同的 Context 标志
#[cfg(target_arch = "x86_64")]
const CONTEXT_DEBUG: u32 =
    windows_sys::Win32::System::Diagnostics::Debug::CONTEXT_DEBUG_REGISTERS_AMD64;
#[cfg(target_arch = "x86")]
const CONTEXT_DEBUG: u32 =
    windows_sys::Win32::System::Diagnostics::Debug::CONTEXT_DEBUG_REGISTERS_X86;

/// 硬件调试寄存器 DR0-DR3 索引
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum HwReg {
    Dr0 = 0,
    Dr1 = 1,
    Dr2 = 2,
    Dr3 = 3,
}

/// 断点类型（对应 DR7.RW 位）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum HwBreakpointType {
    /// 执行断点 (RW=00)
    Execute,
    /// 写断点 (RW=01)
    Write,
    /// 访问断点 (RW=11)
    Access,
}

impl HwBreakpointType {
    #[inline]
    fn rw_bits(self) -> usize {
        match self {
            Self::Execute => 0b00,
            Self::Write => 0b01,
            Self::Access => 0b11,
        }
    }
}

/// 断点长度（对应 DR7.LEN 位）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum HwBreakpointLen {
    Byte1,
    Byte2,
    Byte4,
    #[cfg(target_arch = "x86_64")]
    Byte8,
}

impl HwBreakpointLen {
    #[inline]
    pub const fn bytes(self) -> usize {
        match self {
            Self::Byte1 => 1,
            Self::Byte2 => 2,
            Self::Byte4 => 4,
            #[cfg(target_arch = "x86_64")]
            Self::Byte8 => 8,
        }
    }

    #[inline]
    fn len_bits(self) -> usize {
        match self {
            Self::Byte1 => 0b00,
            Self::Byte2 => 0b01,
            Self::Byte4 => 0b11,
            #[cfg(target_arch = "x86_64")]
            Self::Byte8 => 0b10,
        }
    }
}

/// 清除 DR7 中指定寄存器的控制位（L/G/RW/LEN）
#[inline]
pub fn clear_dr7_slot(dr7: &mut usize, reg: HwReg) {
    let idx = reg as usize;
    // 1. 清除局部使能位 (L0-L3) 和 全局使能位 (G0-G3)
    // 每个寄存器占用 2 bits (L, G)，从 bit 0 开始
    *dr7 &= !(0b11usize << (idx * 2));
    // 2. 清除条件位 (RWn, LENn)
    // 每个寄存器占用 4 bits，从 bit 16 开始
    let ctrl_shift = 16 + (idx * 4);
    *dr7 &= !(0b1111usize << ctrl_shift);
}

/// 配置 DR7 中指定寄存器的控制位
#[inline]
pub fn set_dr7_slot(dr7: &mut usize, reg: HwReg, kind: HwBreakpointType, len: HwBreakpointLen) {
    let idx = reg as usize;

    // 先清理旧状态
    clear_dr7_slot(dr7, reg);

    // 1. 设置局部使能位 (Ln) - 对应位为 idx * 2
    *dr7 |= 1usize << (idx * 2);

    // 2. 设置 RW 和 LEN
    // 结构：LEN (2 bits) | RW (2 bits)
    let ctrl_bits = (len.len_bits() << 2) | kind.rw_bits();
    let ctrl_shift = 16 + (idx * 4);

    *dr7 |= ctrl_bits << ctrl_shift;
}

/// 在 CONTEXT 中设置硬件断点
///
/// # 校验
/// - 执行断点仅支持 1 字节长度
/// - 多字节断点要求地址按长度对齐
pub fn set_hw_break_in_context(
    ctx: &mut CONTEXT,
    addr: usize,
    kind: HwBreakpointType,
    len: HwBreakpointLen,
    reg: HwReg,
) -> crate::Result<()> {
    if addr == 0 {
        crate::bail!("hw breakpoint addr is null");
    }

    if kind == HwBreakpointType::Execute && len != HwBreakpointLen::Byte1 {
        crate::bail!("execute breakpoint only supports 1 byte length");
    }

    let size = len.bytes();
    if size > 1 && (addr & (size - 1)) != 0 {
        crate::bail!("hardware breakpoint address alignment invalid");
    }

    ctx.ContextFlags |= CONTEXT_DEBUG;

    match reg {
        HwReg::Dr0 => ctx.Dr0 = addr as _,
        HwReg::Dr1 => ctx.Dr1 = addr as _,
        HwReg::Dr2 => ctx.Dr2 = addr as _,
        HwReg::Dr3 => ctx.Dr3 = addr as _,
    };

    let mut dr7 = ctx.Dr7 as usize;
    set_dr7_slot(&mut dr7, reg, kind, len);
    ctx.Dr7 = dr7 as _;

    Ok(())
}

/// 在 CONTEXT 中清除指定硬件断点
pub fn clear_hw_break_in_context(ctx: &mut CONTEXT, reg: HwReg) {
    ctx.ContextFlags |= CONTEXT_DEBUG;

    match reg {
        HwReg::Dr0 => ctx.Dr0 = 0,
        HwReg::Dr1 => ctx.Dr1 = 0,
        HwReg::Dr2 => ctx.Dr2 = 0,
        HwReg::Dr3 => ctx.Dr3 = 0,
    };

    let mut dr7 = ctx.Dr7 as usize;
    clear_dr7_slot(&mut dr7, reg);
    ctx.Dr7 = dr7 as _;
}
