use translate_macros::byte_slice;
use windows_sys::Win32::System::{
    Diagnostics::Debug::FlushInstructionCache,
    Memory::{PAGE_EXECUTE_READWRITE, PAGE_READWRITE},
    SystemServices::{IMAGE_DOS_HEADER, IMAGE_DOS_SIGNATURE, IMAGE_NT_SIGNATURE},
    Threading::GetCurrentProcess,
};

use crate::utils::mem::protect_guard::ProtectGuard;

/// 刷新指令缓存（在修改代码段字节后必须调用）
pub fn flush_icache(addr: *const u8, size: usize) {
    unsafe {
        let ok = FlushInstructionCache(GetCurrentProcess(), addr as _, size as _);
        if ok == 0 {
            crate::print_last_error_message!();
            crate::debug!("Warning: FlushInstructionCache failed");
        }
    }
}

/// 写入汇编字节到指定地址，自动处理内存保护和指令缓存刷新
pub fn write_asm(address: *mut u8, data: &[u8]) -> crate::Result<()> {
    if address.is_null() {
        crate::bail!("address is null");
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
pub fn write_bytes(address: *mut u8, data: &[u8]) -> crate::Result<()> {
    if address.is_null() {
        crate::bail!("address is null");
    }
    if data.is_empty() {
        return Ok(());
    }

    unsafe {
        ProtectGuard::new(address, data.len(), PAGE_READWRITE)?.write_bytes(data);
    }

    Ok(())
}

/// 生成一个32位的汇编跳板代码的缓冲区
///
/// # 参数
/// - `target_fn_addr`: 目标函数地址
/// - `pre_asm`: 调用目标函数前执行的汇编字节
/// - `pos_asm`: 调用目标函数后执行的汇编字节
///
/// # 返回
/// - 返回包含跳板代码的字节向量
pub fn generate_trampoline_stub_32(
    target_fn_addr: usize,
    pre_asm: &[u8],
    post_asm: &[u8],
) -> Vec<u8> {
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

/// 解析可修补的地址，处理短跳转指令链(最多8次跳转)
///
/// 这个函数用于解析可能包含跳转指令的地址，通过跟随相对短跳转(0xEB)指令，
/// 找到最终的跳转目标地址。这在inline hook中特别有用，
/// 因为短跳转的字节长度太小会导致inline hook失败
///
/// # Safety
/// - `addr` 必须指向当前进程中可读的有效指令内存。
/// - 调用者必须保证沿跳转链访问的地址在解析期间始终有效。
pub unsafe fn resolve_patchable_addr(mut addr: usize) -> crate::Result<usize> {
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
                return Ok(addr);
            }
        }
    }

    crate::bail!("Too many jumps followed when resolving patchable address");
}

/// 通用的相对偏移指令写入函数（支持 jmp/call 等）
///
/// 在指定地址写入 5 字节的相对调用/跳转指令，格式为：`<OPCODE> + rel32`。
/// 偏移量计算方式：`target - (patch_address + 5)`，需在 i32 范围内。
///
/// # 泛型参数
/// - `OPCODE`: 指令的操作码，例如 jmp 为 0xE9，call 为 0xE8。
///
/// # 参数
/// - `patch_address`: 要写入指令的内存地址
/// - `target_function`: 目标地址
///
/// # 返回值
/// 成功返回 `Ok(())`，若偏移量超出 32 位有符号整数范围则返回错误。
///
/// # Safety
/// - `patch_address` 必须可写且至少有 5 字节可用空间。
/// - `target_function` 必须是有效的可执行地址，且写入的指令不会破坏程序逻辑。
pub unsafe fn write_rel32_instruction<const OPCODE: u8>(
    patch_address: *mut u8,
    target_function: *const u8,
) -> crate::Result<()> {
    let next = unsafe { patch_address.add(5) } as isize;
    let target = target_function as isize;

    let offset = target.wrapping_sub(next);

    // 验证偏移量范围
    let rel32 = i32::try_from(offset).map_err(|_| {
        crate::anyhow!(
            "rel32 out of range: target={:#x}, next={:#x}, diff={:#x}",
            target as usize,
            next as usize,
            offset
        )
    })?;

    // 构建指令
    let mut opcode = [0u8; 5];
    opcode[0] = OPCODE;
    opcode[1..5].copy_from_slice(&rel32.to_le_bytes());

    write_asm(patch_address, &opcode)
}

/// 在指定地址写入 32 位相对跳转指令（E9 jmp）
///
/// 功能同 `write_rel32_instruction<0xE9>`，但提供更明确的命名和文档。
///
/// # 参数
/// - `patch_address`: 要写入指令的内存地址
/// - `target_function`: 目标地址
///
/// # 返回值
/// 成功返回 `Ok(())`，若偏移量超出 32 位有符号整数范围则返回错误。
///
/// # Safety
/// - `patch_address` 必须可写且至少有 5 字节可用空间。
/// - `target_function` 必须是有效的可执行地址，且写入的指令不会破坏程序逻辑。
pub unsafe fn write_jmp_instruction(
    patch_address: *mut u8,
    target_function: *const u8,
) -> crate::Result<()> {
    unsafe { write_rel32_instruction::<0xE9>(patch_address, target_function) }
}

/// 在指定地址写入 32 位相对调用指令（E8 call）
///
/// 功能同 `write_rel32_instruction<0xE8>`，但提供更明确的命名和文档。
///
/// # 参数
/// - `patch_address`: 要写入指令的内存地址
/// - `target_function`: 目标地址
///
/// # 返回值
/// 成功返回 `Ok(())`，若偏移量超出 32 位有符号整数范围则返回错误。
///
/// # Safety
/// - `patch_address` 必须可写且至少有 5 字节可用空间。
/// - `target_function` 必须是有效的可执行地址，且写入的指令不会破坏程序逻辑。
pub unsafe fn write_call_instruction(
    patch_address: *mut u8,
    target_function: *const u8,
) -> crate::Result<()> {
    unsafe { write_rel32_instruction::<0xE8>(patch_address, target_function) }
}

#[cfg(target_pointer_width = "32")]
use windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32 as IMAGE_NT_HEADERS;

#[cfg(target_pointer_width = "64")]
use windows_sys::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64 as IMAGE_NT_HEADERS;

/// 获取目标模块的DOS头和NT头
///
/// # Safety
/// - `module_base` 必须是有效 PE 映像基址，且 DOS/NT 头在当前进程中可读。
/// - 返回的引用依赖于该映像生命周期，调用者必须保证模块不被卸载。
pub unsafe fn get_dos_and_nt_headers(
    module_base: usize,
) -> crate::Result<(&'static IMAGE_DOS_HEADER, &'static IMAGE_NT_HEADERS)> {
    let dos = unsafe { &*(module_base as *const IMAGE_DOS_HEADER) };

    if dos.e_magic != IMAGE_DOS_SIGNATURE {
        crate::bail!(
            "Invalid DOS signature: expected 0x5A4D, found 0x{:X}",
            dos.e_magic
        );
    }

    let nt = unsafe { &*((module_base + dos.e_lfanew as usize) as *const IMAGE_NT_HEADERS) };

    if nt.Signature != IMAGE_NT_SIGNATURE {
        crate::bail!(
            "Invalid NT signature: expected 0x00004550, found 0x{:X}",
            nt.Signature
        );
    }

    Ok((dos, nt))
}

/// 获取当前模块（可执行文件）的入口点地址（Entry Point）
///
/// # Safety
/// - 调用者必须保证当前进程主模块为有效 PE 映像。
/// - 返回地址仅在模块保持加载且未重映射时有效。
pub unsafe fn get_entry_point_addr() -> crate::Result<usize> {
    let h_module = crate::utils::win32::get_module_handle(core::ptr::null())? as usize;

    unsafe {
        let (_, nt_headers) = get_dos_and_nt_headers(h_module)?;

        // 计算入口点：基址 + 偏移 (RVA)
        let rva = nt_headers.OptionalHeader.AddressOfEntryPoint;
        if rva == 0 {
            crate::bail!("Entry point RVA is 0");
        }

        Ok(h_module + rva as usize)
    }
}
