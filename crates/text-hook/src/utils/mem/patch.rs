use translate_macros::byte_slice;
use windows_sys::Win32::System::{
    Diagnostics::Debug::FlushInstructionCache,
    Memory::{PAGE_EXECUTE_READWRITE, PAGE_READWRITE},
    Threading::GetCurrentProcess,
};

use crate::utils::mem::protect_guard::ProtectGuard;

/// 刷新指令缓存（在修改代码段字节后必须调用）
#[allow(dead_code)]
pub fn flush_icache(addr: *const u8, size: usize) {
    unsafe {
        let _ = FlushInstructionCache(GetCurrentProcess(), addr as _, size as _);
    }
}

/// 写入汇编字节到指定地址，自动处理内存保护和指令缓存刷新
#[allow(dead_code)]
pub fn write_asm(address: *mut u8, data: &[u8]) -> anyhow::Result<()> {
    if address.is_null() {
        anyhow::bail!("address is null");
    }
    if data.is_empty() {
        return Ok(());
    }

    unsafe {
        ProtectGuard::new(address, data.len(), PAGE_EXECUTE_READWRITE)?.write_asm_bytes(data);
    }

    Ok(())
}

/// 写入字节到指定地址，自动处理内存保护
#[allow(dead_code)]
pub fn write_bytes(address: *mut u8, data: &[u8]) -> anyhow::Result<()> {
    if address.is_null() {
        anyhow::bail!("address is null");
    }
    if data.is_empty() {
        return Ok(());
    }

    unsafe {
        ProtectGuard::new(address, data.len(), PAGE_READWRITE)?.write_bytes(data);
    }

    Ok(())
}

/// 创建一个32位的汇编跳板代码
///
/// # 参数
/// - `target_fn_addr`: 目标函数地址
/// - `pre_asm`: 调用目标函数前执行的汇编字节
/// - `pos_asm`: 调用目标函数后执行的汇编字节
///
/// # 返回
/// - 返回包含跳板代码的字节向量
#[allow(dead_code)]
pub fn create_trampoline_32(target_fn_addr: usize, pre_asm: &[u8], post_asm: &[u8]) -> Vec<u8> {
    let mut code_buf: Vec<u8> = Vec::with_capacity(32);

    // pushad; pushfd;
    code_buf.extend_from_slice(&byte_slice!("60 9C"));

    code_buf.extend_from_slice(pre_asm);

    // mov ebx, imm32
    code_buf.push(0xBB);
    code_buf.extend_from_slice(&target_fn_addr.to_le_bytes());

    // call ebx; popfd; popad;
    code_buf.extend_from_slice(&byte_slice!("FF D3 9D 61"));

    code_buf.extend_from_slice(post_asm);

    code_buf
}

/// 解析可修补的32位地址，处理短跳转指令链(最多8次跳转)
///
/// 这个函数用于解析可能包含跳转指令的地址，通过跟随相对短跳转(0xEB)指令，
/// 找到最终的跳转目标地址。这在inline hook中特别有用，
/// 因为短跳转的字节长度太小会导致inline hook失败
#[allow(dead_code)]
pub unsafe fn resolve_patchable_addr_32(mut addr: usize) -> usize {
    // 防止无限循环
    const MAX_FOLLOW: usize = 8;

    for _ in 0..MAX_FOLLOW {
        let opcode = unsafe { *(addr as *const u8) };

        match opcode {
            0xEB => {
                let rel = unsafe { *((addr + 1) as *const i8) } as isize;
                let next = (addr + 2) as isize;
                addr = (next + rel) as usize;
                continue;
            }
            _ => {
                break;
            }
        }
    }

    addr
}

/// 在指定地址写入32位相对跳转指令（E9 jmp）
///
/// 该函数用于在指定的内存地址处写入一个5字节的`jmp`指令，该指令会跳转到目标函数。
/// 使用相对寻址方式，计算从`jmp`指令下一条指令到目标函数的偏移量。
///
/// # 参数
///
/// - `patch_address`: 要写入`jmp`指令的内存地址
/// - `target_function`: 要跳转到的目标函数地址
///
/// # 返回值
///
/// 返回`anyhow::Result<()>`，成功时返回`Ok(())`，失败时返回错误信息。
///
/// # 错误
///
/// 如果相对偏移量超出32位有符号整数范围（±2GB），则返回错误。
pub unsafe fn write_jmp_instruction(
    patch_address: *mut u8,
    target_function: *const u8,
) -> anyhow::Result<()> {
    let next = unsafe { patch_address.add(5) } as isize;
    let target = target_function as isize;

    let offset = target.wrapping_sub(next);

    // 验证偏移量范围
    let rel32 = i32::try_from(offset).map_err(|_| {
        anyhow::anyhow!(
            "rel32 out of range: target={:#x}, next={:#x}, diff={:#x}",
            target as usize,
            next as usize,
            offset
        )
    })?;

    // 在栈上构建机器码
    let mut opcode = [0u8; 5];
    opcode[0] = 0xE9;
    opcode[1..5].copy_from_slice(&rel32.to_le_bytes());

    write_asm(patch_address, &opcode)
}
