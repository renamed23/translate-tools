pub mod iat_patch;
pub mod protect_guard;

use translate_macros::byte_slice;
use winapi::{
    shared::{minwindef::HMODULE, ntdef::LPCSTR},
    um::{
        libloaderapi::{GetModuleHandleW, GetProcAddress},
        processthreadsapi::{FlushInstructionCache, GetCurrentProcess},
    },
};

use crate::hook_utils::protect_guard::ProtectGuard;

/// 获取模块句柄的包装函数
/// 当module_name为空字符串时，获取当前进程的模块句柄
#[allow(dead_code)]
pub fn get_module_handle(module_name: &str) -> Option<HMODULE> {
    if module_name.is_empty() {
        // 空字符串表示获取当前进程的句柄
        unsafe { Some(GetModuleHandleW(core::ptr::null())) }
    } else {
        // 转换为UTF-16并调用GetModuleHandleW
        let module_wide: Vec<u16> = module_name
            .encode_utf16()
            .chain(core::iter::once(0))
            .collect();

        unsafe {
            let handle = GetModuleHandleW(module_wide.as_ptr());
            if handle.is_null() { None } else { Some(handle) }
        }
    }
}

/// 获取目标模块的符号的地址
#[allow(dead_code)]
pub fn get_module_symbol_addr(module: &str, symbol: LPCSTR) -> Option<usize> {
    Some(get_module_symbol_addrs(module, &[symbol])?[0])
}

/// 获取目标模块的符号的地址，只有所有符号地址全部找到才返回Some，否则返回None
#[allow(dead_code)]
pub fn get_module_symbol_addrs(module: &str, symbols: &[LPCSTR]) -> Option<Vec<usize>> {
    let handle = get_module_handle(module)?;
    let mut addrs = Vec::new();
    unsafe {
        for &sym in symbols {
            let func = GetProcAddress(handle, sym);
            if func.is_null() {
                return None;
            }
            addrs.push(func as usize);
        }
    }

    Some(addrs)
}

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
        ProtectGuard::new(
            address,
            data.len(),
            winapi::um::winnt::PAGE_EXECUTE_READWRITE,
        )?
        .write_asm_bytes(data);
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
        ProtectGuard::new(address, data.len(), winapi::um::winnt::PAGE_READWRITE)?
            .write_bytes(data);
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
