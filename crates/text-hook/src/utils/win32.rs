use scopeguard::defer;
use windows_sys::{
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, SetLastError},
        Storage::FileSystem::{Wow64DisableWow64FsRedirection, Wow64RevertWow64FsRedirection},
        System::{
            Environment::GetCurrentDirectoryW,
            LibraryLoader::{GetModuleFileNameW, GetModuleHandleW, GetProcAddress, LoadLibraryW},
            SystemInformation::GetSystemDirectoryW,
        },
        UI::WindowsAndMessaging::{
            CB_ADDSTRING, CB_FINDSTRING, CB_FINDSTRINGEXACT, CB_GETLBTEXT, CB_INSERTSTRING,
            CB_SELECTSTRING, GetClassNameW, GetWindowTextW, LB_ADDSTRING, LB_FINDSTRING,
            LB_FINDSTRINGEXACT, LB_GETTEXT, LB_INSERTSTRING, LB_SELECTSTRING,
        },
    },
    core::{PCSTR, PCWSTR},
};

use crate::{
    constant, print_last_error_message,
    utils::{
        exts::{
            path_ext::PathExt,
            ptr_ext::PtrExt,
            slice_ext::{ByteSliceExt, WideSliceExt},
        },
        raii_wrapper::OwnedHMODULE,
    },
};

/// 获取模块句柄的包装函数
pub fn get_module_handle(module_name: PCWSTR) -> crate::Result<HMODULE> {
    unsafe {
        let handle = GetModuleHandleW(module_name);
        if handle.is_null() {
            print_last_error_message!();
            crate::bail!(
                "GetModuleHandleW for {} failed",
                module_name.to_slice_until_null(8192).to_string_lossy()
            );
        } else {
            Ok(handle)
        }
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
            crate::anyhow!(
                "GetProcAddress failed, symbol: '{}'",
                symbol.to_slice_until_null(8192).to_string_lossy()
            )
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
        crate::debug!("GetSystemDirectoryW fetch_win32_string {size}");

        let k = GetSystemDirectoryW(ptr, size as u32) as usize;

        if k == 0 {
            FetchResult::Error(GetLastError())
        } else if k > size {
            FetchResult::Required(k)
        } else {
            FetchResult::Success(k)
        }
    })
}

/// 根据路径加载指定DLL，若失败返回Err
pub fn load_library(path: PCWSTR) -> crate::Result<OwnedHMODULE> {
    unsafe {
        let handle = LoadLibraryW(path);
        if handle.is_null() {
            print_last_error_message!();
            crate::bail!(
                "LoadLibraryW failed for: {}",
                path.to_slice_until_null(8192).to_string_lossy()
            );
        }
        Ok(OwnedHMODULE(handle))
    }
}

/// 获取模块文件路径
pub fn get_module_file_name(module: HMODULE, add_null: bool) -> crate::Result<Vec<u16>> {
    fetch_win32_string(add_null, |ptr, size| unsafe {
        crate::debug!("GetModuleFileNameW fetch_win32_string {size}");

        let k = GetModuleFileNameW(module, ptr, size as u32) as usize;

        if k == 0 {
            FetchResult::Error(GetLastError())
        } else if k >= size {
            FetchResult::Retry
        } else {
            FetchResult::Success(k)
        }
    })
}

/// 获取当前工作目录
pub fn get_current_dir(add_null: bool) -> crate::Result<Vec<u16>> {
    fetch_win32_string(add_null, |ptr, size| unsafe {
        crate::debug!("GetCurrentDirectoryW fetch_win32_string {size}");

        let k = GetCurrentDirectoryW(size as u32, ptr) as usize;

        if k == 0 {
            FetchResult::Error(GetLastError())
        } else if k > size {
            FetchResult::Required(k)
        } else {
            FetchResult::Success(k)
        }
    })
}

/// 加载被劫持的DLL库
///
/// 此函数用于加载DLL，支持两种模式：
/// - 如果 `constant::HIJACKED_DLL_PATH` 为空字符串，则从系统目录加载指定的DLL
/// - 如果 `constant::HIJACKED_DLL_PATH` 不为空，则直接从该路径加载DLL
#[allow(clippy::const_is_empty)]
pub fn load_hijacked_library(dll_name: &str) -> crate::Result<OwnedHMODULE> {
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

/// 获取窗口标题
pub fn get_window_text(hwnd: HWND, add_null: bool) -> crate::Result<Vec<u16>> {
    fetch_win32_string(add_null, |ptr, size| unsafe {
        crate::debug!("GetWindowTextW fetch_win32_string {size}");

        let k = GetWindowTextW(hwnd, ptr, size as i32) as usize;
        if k == 0 {
            let err = GetLastError();
            if err == 0 {
                FetchResult::Success(0)
            } else {
                FetchResult::Error(err)
            }
        } else if k == size - 1 {
            FetchResult::Retry
        } else {
            FetchResult::Success(k)
        }
    })
}

/// 获取窗口类名
pub fn get_window_class_name(hwnd: HWND, add_null: bool) -> crate::Result<Vec<u16>> {
    fetch_win32_string(add_null, |ptr, size| unsafe {
        crate::debug!("GetClassNameW fetch_win32_string {size}");

        let k = GetClassNameW(hwnd, ptr, size as i32) as usize;

        if k == 0 {
            FetchResult::Error(GetLastError())
        } else if k == size - 1 {
            FetchResult::Retry
        } else {
            FetchResult::Success(k)
        }
    })
}

/// 获取字符串结果
pub enum FetchResult {
    /// 成功，返回实际写入长度（不含 null）
    Success(usize),

    /// 缓冲区不足，需要 API 告知的准确容量（包含 null）
    Required(usize),

    /// 缓冲区不足但 API 没给具体大小，建议翻倍扩容重试
    Retry,

    /// 真实错误
    Error(u32),
}

/// 通用工具：处理 Win32 字符串获取逻辑
pub fn fetch_win32_string<T, F>(add_null: bool, mut f: F) -> crate::Result<Vec<T>>
where
    T: Default + Copy + PartialEq,
    F: FnMut(*mut T, usize) -> FetchResult,
{
    const STACK_CAP: usize = 512;
    let mut stack_buf = [core::mem::MaybeUninit::<T>::uninit(); STACK_CAP];
    let mut heap_buf: Vec<T> = Vec::new();
    let mut cap = STACK_CAP;

    loop {
        let ptr = if cap <= STACK_CAP {
            stack_buf.as_mut_ptr() as *mut T
        } else {
            if heap_buf.capacity() < cap {
                heap_buf.reserve_exact(cap - heap_buf.capacity());
            }
            heap_buf.as_mut_ptr()
        };

        unsafe { SetLastError(0) };

        match f(ptr, cap) {
            FetchResult::Success(len) => {
                let mut result = if cap <= STACK_CAP {
                    let mut v = Vec::with_capacity(len);
                    unsafe {
                        core::ptr::copy_nonoverlapping(ptr, v.as_mut_ptr(), len);
                        v.set_len(len);
                    }
                    v
                } else {
                    unsafe { heap_buf.set_len(len) };
                    heap_buf
                };

                if add_null {
                    if result.last() != Some(&T::default()) {
                        result.push(T::default());
                    }
                } else {
                    while result.last() == Some(&T::default()) {
                        result.pop();
                    }
                }
                return Ok(result);
            }
            FetchResult::Required(req_cap) => {
                cap = req_cap;
            }
            FetchResult::Retry => {
                cap = cap
                    .checked_mul(2)
                    .ok_or_else(|| crate::anyhow!("Buffer overflow"))?;
            }
            FetchResult::Error(err) => {
                crate::print_last_error_message!(ec err);
                crate::bail!("Win32 string fetch failed with error code: {}", err);
            }
        }
    }
}
