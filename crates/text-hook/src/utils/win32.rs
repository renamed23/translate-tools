use scopeguard::defer;
use windows_sys::{
    Win32::{
        Foundation::HMODULE,
        Storage::FileSystem::{Wow64DisableWow64FsRedirection, Wow64RevertWow64FsRedirection},
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
            SystemInformation::GetSystemDirectoryW,
        },
        UI::WindowsAndMessaging::{
        CB_ADDSTRING, CB_INSERTSTRING, CB_FINDSTRING, CB_FINDSTRINGEXACT, CB_SELECTSTRING,
        CB_GETLBTEXT, LB_ADDSTRING, LB_INSERTSTRING, LB_FINDSTRING, LB_FINDSTRINGEXACT,
        LB_SELECTSTRING, LB_GETTEXT,
    },
    },
    core::PCSTR,
};

use crate::{constant, print_system_error_message};

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
            if handle.is_null() {
                print_system_error_message!();
                None
            } else {
                Some(handle)
            }
        }
    }
}

/// 获取指定模块中单个符号的地址
#[allow(dead_code)]
pub fn get_module_symbol_addr(module: &str, symbol: PCSTR) -> Option<usize> {
    let handle = get_module_handle(module)?;
    get_module_symbol_addr_from_handle(handle, symbol)
}

/// 获取指定模块中多个符号的地址，只有所有符号地址全部找到才返回Some
#[allow(dead_code)]
pub fn get_module_symbol_addrs(module: &str, symbols: &[PCSTR]) -> Option<Vec<usize>> {
    let handle = get_module_handle(module)?;
    get_module_symbol_addrs_from_handle(handle, symbols)
}

/// 从模块句柄获取单个符号的地址
#[allow(dead_code)]
pub fn get_module_symbol_addr_from_handle(module: HMODULE, symbol: PCSTR) -> Option<usize> {
    Some(get_module_symbol_addrs_from_handle(module, &[symbol])?[0])
}

/// 从模块句柄获取多个符号的地址，只有所有符号地址全部找到才返回Some
#[allow(dead_code)]
pub fn get_module_symbol_addrs_from_handle(
    module: HMODULE,
    symbols: &[PCSTR],
) -> Option<Vec<usize>> {
    let mut addrs = Vec::with_capacity(symbols.len());

    unsafe {
        for &sym in symbols {
            let func = GetProcAddress(module, sym)?;
            addrs.push(func as usize);
        }
    }

    Some(addrs)
}

/// 获取系统目录的路径，若失败返回None
#[allow(dead_code)]
pub fn get_system_directory() -> Option<String> {
    // 获取系统目录缓冲区大小
    let size = unsafe { GetSystemDirectoryW(core::ptr::null_mut(), 0) };
    if size == 0 {
        print_system_error_message!();
        return None;
    }

    // 分配缓冲区并获取系统目录路径
    let mut system_dir = Vec::<u16>::with_capacity(size as usize);
    let actual_size = unsafe { GetSystemDirectoryW(system_dir.as_mut_ptr(), size) };

    if actual_size == 0 || actual_size >= size {
        return None;
    }

    // 设置缓冲区实际长度
    unsafe {
        system_dir.set_len(actual_size as usize);
    }

    String::from_utf16(&system_dir).ok()
}

/// 根据路径加载指定DLL，若失败返回None
#[allow(dead_code)]
pub fn load_library(path: &str) -> Option<HMODULE> {
    let path: Vec<u16> = path.encode_utf16().chain(core::iter::once(0)).collect();
    let handle = unsafe { LoadLibraryW(path.as_ptr()) };
    if handle.is_null() {
        print_system_error_message!();
        None
    } else {
        Some(handle)
    }
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
/// * `Option<HMODULE>` - 成功时返回DLL模块句柄，失败时返回None
#[allow(dead_code, clippy::const_is_empty)]
pub fn load_hijacked_library(dll_name: &str) -> Option<HMODULE> {
    // 检查是否设置了自定义劫持路径
    if constant::HIJACKED_DLL_PATH.is_empty() {
        let system_dir = get_system_directory()?;
        let full_path = format!("{system_dir}/{dll_name}");
        load_library(&full_path)
    } else {
        // 直接从自定义劫持路径加载
        load_library(constant::HIJACKED_DLL_PATH)
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
        print_system_error_message!();
    }

    defer!(if success {
        unsafe { Wow64RevertWow64FsRedirection(old_state) };
    });

    callback()
}


/// 判断消息是否需要文本转换/映射
#[inline(always)]
pub const fn needs_text_conversion(msg: u32) -> bool {
    matches!(msg,
        // ComboBox
        CB_ADDSTRING | CB_INSERTSTRING | CB_FINDSTRING | CB_FINDSTRINGEXACT 
        | CB_SELECTSTRING | CB_GETLBTEXT |
        
        // ListBox  
        LB_ADDSTRING | LB_INSERTSTRING | LB_FINDSTRING | LB_FINDSTRINGEXACT 
        | LB_SELECTSTRING | LB_GETTEXT

    )
}