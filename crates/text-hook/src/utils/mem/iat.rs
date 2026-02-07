use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::Diagnostics::Debug::IMAGE_DIRECTORY_ENTRY_IMPORT;
use windows_sys::Win32::System::SystemServices::IMAGE_IMPORT_DESCRIPTOR;
use windows_sys::core::{PCSTR, PCWSTR};

use crate::utils::mem::patch::{get_dos_and_nt_headers, write_bytes};
use crate::utils::win32::{get_module_handle, get_module_symbol_addrs};
use crate::{debug, w2s};

/// 通用的 IAT 修补函数
///
/// # 参数
/// - `target_mod_name`: 目标模块名（要修补 IAT 的模块），若为null则获取当前进程的模块句柄
/// - `source_dll_name`: 源 DLL 名（包含要替换函数的 DLL）
/// - `functions`: 要替换的函数名和对应的 hook 函数地址列表
///
/// # 安全性
/// 此函数不安全，因为它直接操作内存和指针
pub unsafe fn patch_iat(
    target_mod_name: PCWSTR,
    source_dll_name: PCWSTR,
    functions: &[(PCSTR, usize)],
) -> crate::Result<()> {
    unsafe {
        // 获取目标模块句柄
        let target_mod = get_module_handle(target_mod_name)?;

        // 提取所有函数名
        let func_names: Vec<PCSTR> = functions.iter().map(|&(name, _)| name).collect();

        // 一次性获取所有真实函数地址
        let real_addrs = get_module_symbol_addrs(source_dll_name, &func_names)?;

        for ((func_name, hook_addr), &real_addr) in functions.iter().zip(real_addrs.iter()) {
            // 查找 IAT 条目
            let Ok(iat_entry_ptr) = find_iat_entry(target_mod, real_addr as _) else {
                crate::bail!(
                    "IAT entry for {}!{func_name:?} not found in {}",
                    w2s!(source_dll_name),
                    w2s!(target_mod_name)
                )
            };

            // 写入 hook 地址
            write_bytes(iat_entry_ptr as _, &hook_addr.to_ne_bytes())?;

            debug!(
                "Patched IAT entry for {}!{func_name:?} -> 0x{hook_addr:08X}",
                w2s!(source_dll_name),
            );
        }

        Ok(())
    }
}

/// 找到指定模组的指定导入函数地址的IAT入口地址，返回的指针绝不会为null
pub unsafe fn find_iat_entry(module: HMODULE, target_ptr: usize) -> crate::Result<*mut usize> {
    unsafe {
        let base = module as usize;

        let (_, nt) = get_dos_and_nt_headers(base)?;

        let import_dir = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT as usize];
        if import_dir.VirtualAddress == 0 {
            crate::bail!("No import directory found");
        }

        let mut imp = (base + import_dir.VirtualAddress as usize) as *const IMAGE_IMPORT_DESCRIPTOR;
        while !imp.is_null() && (*imp).Name != 0 {
            let first_thunk = (*imp).FirstThunk as usize;
            if first_thunk != 0 {
                let mut thunk = (base + first_thunk) as *mut usize;
                while !thunk.is_null() && *thunk != 0 {
                    if *thunk == target_ptr {
                        return Ok(thunk);
                    }
                    thunk = thunk.add(1);
                }
            }
            imp = imp.add(1);
        }

        crate::bail!("IAT entry for target pointer 0x{target_ptr:08X} not found");
    }
}
