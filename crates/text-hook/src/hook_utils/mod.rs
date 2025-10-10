pub mod iat_patch;
pub mod protect_guard;

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
