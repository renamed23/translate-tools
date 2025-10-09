use core::ptr;
use winapi::shared::ntdef::LPCSTR;
use winapi::um::winnt::PAGE_EXECUTE_READWRITE;
use winapi::{
    shared::minwindef::{FARPROC, HMODULE},
    um::winnt::{
        IMAGE_DIRECTORY_ENTRY_IMPORT, IMAGE_DOS_HEADER, IMAGE_IMPORT_DESCRIPTOR, IMAGE_NT_HEADERS32,
    },
};

use crate::debug;
use crate::hook_utils::protect_guard::ProtectGuard;
use crate::hook_utils::{get_module_handle, get_module_symbol_addrs};

/// 通用的 IAT 修补函数
///
/// # 参数
/// - `target_mod_name`: 目标模块名（要修补 IAT 的模块），若为空字符串则获取当前进程的模块句柄
/// - `source_dll_name`: 源 DLL 名（包含要替换函数的 DLL）
/// - `functions`: 要替换的函数名和对应的 hook 函数地址列表
///
/// # 安全性
/// 此函数不安全，因为它直接操作内存和指针
#[allow(dead_code)]
pub unsafe fn patch_iat(
    target_mod_name: &str,
    source_dll_name: &str,
    functions: &[(LPCSTR, usize)],
) -> anyhow::Result<()> {
    unsafe {
        // 获取目标模块句柄
        let target_mod = get_module_handle(target_mod_name)
            .ok_or_else(|| anyhow::anyhow!("GetModuleHandle({target_mod_name}) failed"))?;

        // 提取所有函数名
        let func_names: Vec<LPCSTR> = functions.iter().map(|&(name, _)| name).collect();

        // 一次性获取所有真实函数地址
        let real_addrs =
            get_module_symbol_addrs(source_dll_name, &func_names).ok_or_else(|| {
                anyhow::anyhow!(
                    "GetProcAddress for one or more functions in {source_dll_name} failed"
                )
            })?;

        for ((func_name, hook_addr), &real_addr) in functions.iter().zip(real_addrs.iter()) {
            // 查找 IAT 条目
            let iat_entry = find_iat_entry_32(target_mod, real_addr as _);
            if iat_entry.is_null() {
                anyhow::bail!(
                    "IAT entry for {source_dll_name}!{func_name:?} not found in {target_mod_name}",
                );
            }

            debug!("Found IAT entry for {source_dll_name}!{func_name:?} in {target_mod_name}");

            // 修改内存保护以允许写入
            let guard = ProtectGuard::new(
                iat_entry,
                core::mem::size_of::<usize>(),
                PAGE_EXECUTE_READWRITE,
            )?;

            // 写入 hook 地址
            guard.write(*hook_addr);

            debug!("Patched IAT entry for {source_dll_name}!{func_name:?} -> 0x{hook_addr:08X}",);
        }

        Ok(())
    }
}

/// 找到指定模组的指定导入函数地址的IAT入口地址
#[allow(dead_code)]
pub unsafe fn find_iat_entry_32(module: HMODULE, target_ptr: FARPROC) -> *mut FARPROC {
    unsafe {
        let base = module as usize;
        let dos = base as *const IMAGE_DOS_HEADER;

        const MZ: u16 = 0x5A4D;
        if dos.is_null() || (*dos).e_magic != MZ {
            return ptr::null_mut();
        }

        let nt = (base + (*dos).e_lfanew as usize) as *const IMAGE_NT_HEADERS32;
        if nt.is_null() {
            return ptr::null_mut();
        }

        let import_dir = (*nt).OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT as usize];
        if import_dir.VirtualAddress == 0 {
            return ptr::null_mut();
        }

        let mut imp = (base + import_dir.VirtualAddress as usize) as *const IMAGE_IMPORT_DESCRIPTOR;
        while !imp.is_null() && (*imp).Name != 0 {
            let first_thunk = (*imp).FirstThunk as usize;
            if first_thunk != 0 {
                let mut thunk = (base + first_thunk) as *mut u32;
                while !thunk.is_null() && *thunk != 0 {
                    if *thunk as usize == target_ptr as usize {
                        return thunk as *mut FARPROC;
                    }
                    thunk = thunk.add(1);
                }
            }
            imp = imp.add(1);
        }
        ptr::null_mut()
    }
}
