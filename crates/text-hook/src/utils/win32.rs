use scopeguard::defer;
use std::{
    ffi::OsString,
    os::windows::ffi::{OsStrExt, OsStringExt as _},
    path::{Path, PathBuf},
};
use windows_sys::{
    Win32::{
        Foundation::HMODULE,
        Storage::FileSystem::{Wow64DisableWow64FsRedirection, Wow64RevertWow64FsRedirection},
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
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

use crate::{constant, print_last_error_message};

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
pub fn get_system_directory() -> crate::Result<PathBuf> {
    let size = unsafe { GetSystemDirectoryW(core::ptr::null_mut(), 0) };
    if size == 0 {
        print_last_error_message!();
        crate::bail!("Failed to get buffer size");
    }

    let mut buffer = vec![0u16; size as usize];
    let actual_size = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), size) };

    if actual_size == 0 || actual_size >= size {
        print_last_error_message!();
        crate::bail!("Failed to get system directory");
    }

    buffer.truncate(actual_size as usize);
    Ok(PathBuf::from(OsString::from_wide(&buffer)))
}

/// 根据路径加载指定DLL，若失败返回Err
pub fn load_library(path: &Path) -> crate::Result<HMODULE> {
    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(core::iter::once(0))
        .collect();
    let handle = unsafe { LoadLibraryW(wide_path.as_ptr()) };
    if handle.is_null() {
        print_last_error_message!();
        crate::bail!("LoadLibraryW failed for: {}", path.to_string_lossy());
    }
    Ok(handle)
}

/// 加载被劫持的DLL库
///
/// 此函数用于加载DLL，支持两种模式：
/// - 如果 `constant::HIJACKED_DLL_PATH` 为空字符串，则从系统目录加载指定的DLL
/// - 如果 `constant::HIJACKED_DLL_PATH` 不为空，则直接从该路径加载DLL
///
/// # 参数
/// * `dll_name` - 要加载的DLL文件名（例如："kernel32.dll"）
///
/// # 返回值
/// * `Result<HMODULE>` - 成功时返回DLL模块句柄，失败时返回Err
#[allow(clippy::const_is_empty)]
pub fn load_hijacked_library(dll_name: &str) -> crate::Result<HMODULE> {
    if constant::HIJACKED_DLL_PATH.is_empty() {
        let system_dir = get_system_directory()?;
        let full_path = system_dir.join(dll_name);
        load_library(&full_path)
    } else {
        load_library(Path::new(constant::HIJACKED_DLL_PATH))
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
