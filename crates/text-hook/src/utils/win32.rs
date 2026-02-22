use scopeguard::defer;
use std::mem::MaybeUninit;
use windows_sys::{
    Win32::{
        Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError, HMODULE, SetLastError},
        Storage::FileSystem::{Wow64DisableWow64FsRedirection, Wow64RevertWow64FsRedirection},
        System::{
            Environment::GetCurrentDirectoryW,
            LibraryLoader::{GetModuleFileNameW, GetModuleHandleW, GetProcAddress, LoadLibraryW},
            SystemInformation::GetSystemDirectoryW,
        },
        UI::WindowsAndMessaging::{
            CB_ADDSTRING, CB_FINDSTRING, CB_FINDSTRINGEXACT, CB_GETLBTEXT, CB_INSERTSTRING,
            CB_SELECTSTRING, LB_ADDSTRING, LB_FINDSTRING, LB_FINDSTRINGEXACT, LB_GETTEXT,
            LB_INSERTSTRING, LB_SELECTSTRING,
        },
    },
    core::{PCSTR, PCWSTR},
};

use crate::{
    constant, print_last_error_message,
    utils::exts::{path_ext::PathExt, slice_ext::WideSliceExt},
};

/// 获取模块句柄的包装函数
pub fn get_module_handle(module_name: PCWSTR) -> crate::Result<HMODULE> {
    let handle = unsafe { GetModuleHandleW(module_name) };
    if handle.is_null() {
        print_last_error_message!();
        crate::bail!("GetModuleHandleW for {:?} failed", module_name);
    } else {
        Ok(handle)
    }
}

/// 获取指定模块中单个符号的地址
pub fn get_module_symbol_addr(module: PCWSTR, symbol: PCSTR) -> crate::Result<usize> {
    let handle = get_module_handle(module)?;
    get_module_symbol_addr_from_handle(handle, symbol)
}

/// 获取指定模块中多个符号的地址，只有所有符号地址全部找到才返回Some
pub fn get_module_symbol_addrs(module: PCWSTR, symbols: &[PCSTR]) -> crate::Result<Vec<usize>> {
    let handle = get_module_handle(module)?;
    get_module_symbol_addrs_from_handle(handle, symbols)
}

/// 从模块句柄获取单个符号的地址
pub fn get_module_symbol_addr_from_handle(module: HMODULE, symbol: PCSTR) -> crate::Result<usize> {
    unsafe {
        let func = GetProcAddress(module, symbol).ok_or_else(|| {
            print_last_error_message!();
            crate::anyhow!("GetProcAddress failed")
        })?;
        Ok(func as usize)
    }
}

/// 从模块句柄获取多个符号的地址，只有所有符号地址全部找到才返回Ok
pub fn get_module_symbol_addrs_from_handle(
    module: HMODULE,
    symbols: &[PCSTR],
) -> crate::Result<Vec<usize>> {
    symbols
        .iter()
        .map(|&sym| get_module_symbol_addr_from_handle(module, sym))
        .collect()
}

/// 获取系统目录的路径，若失败返回Err
pub fn get_system_directory(add_null: bool) -> crate::Result<Vec<u16>> {
    fetch_win32_string(add_null, |ptr, size| unsafe {
        GetSystemDirectoryW(ptr, size)
    })
}

/// 根据路径加载指定DLL，若失败返回Err
pub fn load_library(path: PCWSTR) -> crate::Result<HMODULE> {
    let handle = unsafe { LoadLibraryW(path) };
    if handle.is_null() {
        print_last_error_message!();
        crate::bail!("LoadLibraryW failed for: {path:?}");
    }
    Ok(handle)
}

/// 获取模块文件路径
pub fn get_module_file_name(module: HMODULE, add_null: bool) -> crate::Result<Vec<u16>> {
    fetch_win32_string(add_null, |ptr, size| unsafe {
        GetModuleFileNameW(module, ptr, size)
    })
}

/// 获取当前工作目录
pub fn get_current_dir(add_null: bool) -> crate::Result<Vec<u16>> {
    fetch_win32_string(add_null, |ptr, size| unsafe {
        GetCurrentDirectoryW(size, ptr)
    })
}

/// 加载被劫持的DLL库
///
/// 此函数用于加载DLL，支持两种模式：
/// - 如果 `constant::HIJACKED_DLL_PATH` 为空字符串，则从系统目录加载指定的DLL
/// - 如果 `constant::HIJACKED_DLL_PATH` 不为空，则直接从该路径加载DLL
#[allow(clippy::const_is_empty)]
pub fn load_hijacked_library(dll_name: &str) -> crate::Result<HMODULE> {
    if constant::HIJACKED_DLL_PATH.is_empty() {
        let system_dir = get_system_directory(false)?.to_path_buf();
        let full_path = system_dir.join(dll_name);
        load_library(full_path.to_wide_null().as_ptr())
    } else {
        load_library(constant::HIJACKED_DLL_PATH.with_null().as_ptr())
    }
}

/// 在禁用 WOW64 文件系统重定向的情况下执行回调函数。
///
/// WOW64 文件系统重定向是 32 位应用程序在 64 位 Windows 上运行时，
/// 将系统目录（如 System32）重定向到 SysWOW64 的机制。
/// 此函数禁用该重定向，使得回调函数可以访问真实的系统目录，
/// 然后在回调执行完毕后恢复重定向（仅在禁用成功的情况下）。
///
/// # 参数
///
/// * `callback` - 要在禁用 WOW64 重定向状态下执行的回调函数
///
/// # 返回值
///
/// 返回回调函数的执行结果
pub fn with_wow64_redirection_disabled<F, R>(callback: F) -> R
where
    F: FnOnce() -> R,
{
    let mut old_state = core::ptr::null_mut();
    let success = unsafe { Wow64DisableWow64FsRedirection(&mut old_state) != 0 };

    #[cfg(feature = "debug_output")]
    if !success {
        print_last_error_message!();
    }

    defer!(if success {
        unsafe { Wow64RevertWow64FsRedirection(old_state) };
    });

    callback()
}

/// 判断消息是否需要文本转换/映射
#[inline(always)]
pub const fn needs_text_conversion(msg: u32) -> bool {
    matches!(
        msg,
        // ComboBox
        CB_ADDSTRING | CB_INSERTSTRING | CB_FINDSTRING | CB_FINDSTRINGEXACT
        | CB_SELECTSTRING | CB_GETLBTEXT |

        // ListBox
        LB_ADDSTRING | LB_INSERTSTRING | LB_FINDSTRING | LB_FINDSTRINGEXACT
        | LB_SELECTSTRING | LB_GETTEXT
    )
}

/// 通用工具：处理 Win32 字符串获取逻辑
pub fn fetch_win32_string<T, F>(add_null: bool, mut f: F) -> crate::Result<Vec<T>>
where
    T: Default + Copy,
    F: FnMut(*mut T, u32) -> u32,
{
    // 小栈缓冲，避免小字符串直接分配堆
    const STACK_CAP: usize = 512;

    // 只对 u8 / u16 有意义，但保持泛型
    let mut stack_buf: [MaybeUninit<T>; STACK_CAP] = unsafe { MaybeUninit::uninit().assume_init() };

    let mut heap_buf: Vec<MaybeUninit<T>> = Vec::new();

    unsafe {
        let mut n = STACK_CAP;

        loop {
            let buf: &mut [MaybeUninit<T>] = if n <= STACK_CAP {
                &mut stack_buf[..n]
            } else {
                if heap_buf.capacity() < n {
                    heap_buf.reserve(n - heap_buf.len());
                }

                // 使用 capacity，避免反复 realloc
                n = heap_buf.capacity().min(u32::MAX as usize);

                heap_buf.set_len(n);
                &mut heap_buf[..n]
            };

            // 清理 last error，避免读到历史值
            SetLastError(0);

            let k = match f(buf.as_mut_ptr() as *mut T, n as u32) {
                0 => {
                    let err = GetLastError();
                    if err == 0 {
                        0usize
                    } else {
                        crate::print_last_error_message!(ec err);
                        crate::bail!("Win32 string fetch failed with error code: {}", err);
                    }
                }
                v => v as usize,
            };

            let err = GetLastError();

            if k == n && err == ERROR_INSUFFICIENT_BUFFER {
                // 某些 API 返回 n 并设置错误
                n = n.saturating_mul(2).min(u32::MAX as usize);
                continue;
            } else if k > n {
                // 某些 API 返回所需长度
                n = k;
                continue;
            } else if k == n {
                // 理论上不可达（成功时 k 不含 null）
                unreachable!();
            } else {
                // 成功，前 k 项已初始化
                let reserve = if add_null { 1 } else { 0 };
                // 预先分配好 k + reserve 的容量
                let mut result = Vec::with_capacity(k + reserve);

                let ptr = buf.as_ptr().cast::<T>();
                result.extend_from_slice(core::slice::from_raw_parts(ptr, k));

                if add_null {
                    result.push(T::default());
                }

                return Ok(result);
            }
        }
    }
}
