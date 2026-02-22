use core::ffi::c_void;

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, SYSTEMTIME};
use windows_sys::Win32::Globalization::GetACP;
use windows_sys::Win32::Graphics::Gdi::LF_FACESIZE;
use windows_sys::Win32::System::Environment::GetCommandLineW;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_BINARY, REG_SZ, RegCloseKey, RegOpenKeyExW,
    RegQueryValueExW,
};
use windows_sys::Win32::System::Threading::{
    ExitProcess, INFINITE, PROCESS_INFORMATION, STARTUPINFOW, WaitForSingleObject,
};
use windows_sys::core::PCWSTR;
use windows_sys::{s, w};

use crate::debug;
use crate::utils::exts::slice_ext::ByteSliceExt;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TimeFields {
    year: i16,
    month: i16,
    day: i16,
    hour: i16,
    minute: i16,
    second: i16,
    milliseconds: i16,
    weekday: i16,
}

impl TimeFields {
    #[inline]
    fn from_system_time(st: SYSTEMTIME) -> Self {
        Self {
            year: st.wYear as i16,
            month: st.wMonth as i16,
            day: st.wDay as i16,
            hour: st.wHour as i16,
            minute: st.wMinute as i16,
            second: st.wSecond as i16,
            milliseconds: st.wMilliseconds as i16,
            weekday: st.wDayOfWeek as i16,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RtlTimeZoneInformation {
    bias: i32,
    standard_name: [u16; 32],
    standard_start: TimeFields,
    standard_bias: i32,
    daylight_name: [u16; 32],
    daylight_start: TimeFields,
    daylight_bias: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RegTziFormat {
    bias: i32,
    standard_bias: i32,
    daylight_bias: i32,
    standard_date: SYSTEMTIME,
    daylight_date: SYSTEMTIME,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct UnicodeString3264 {
    length: u16,
    maximum_length: u16,
    buffer: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RegistryEntry64 {
    root: u64,
    sub_key: UnicodeString3264,
    value_name: UnicodeString3264,
    data_type: u32,
    data: u64,
    data_size: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RegistryRedirectionEntry64 {
    original: RegistryEntry64,
    redirected: RegistryEntry64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LocaleEmulatorEnvironmentBlock {
    ansi_code_page: u32,
    oem_code_page: u32,
    locale_id: u32,
    default_charset: u32,
    hook_ui_language_api: u32,
    default_face_name: [u16; LF_FACESIZE as usize],
    timezone: RtlTimeZoneInformation,
    number_of_registry_redirection_entries: u64,
    registry_replacement: [RegistryRedirectionEntry64; 1],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct MlProcessInformation {
    process_information: PROCESS_INFORMATION,
    first_call_ldr_load_dll: *mut c_void,
}

type LeCreateProcessFn = unsafe extern "system" fn(
    leb: *mut LocaleEmulatorEnvironmentBlock,
    application_name: *const u16,
    command_line: *const u16,
    current_directory: *const u16,
    creation_flags: u32,
    startup_info: *mut STARTUPINFOW,
    process_information: *mut MlProcessInformation,
    process_attributes: *const c_void,
    thread_attributes: *const c_void,
    environment: *const c_void,
    token: isize,
) -> u32;

fn query_reg_value_into<T: Copy>(
    hkey: HKEY,
    value_name: PCWSTR,
    expected_type: u32,
) -> crate::Result<T> {
    let mut value: T = unsafe { core::mem::zeroed() };
    let mut data_type: u32 = 0;
    let mut data_size = core::mem::size_of::<T>() as u32;

    let ret = unsafe {
        RegQueryValueExW(
            hkey,
            value_name,
            core::ptr::null_mut(),
            &mut data_type,
            &mut value as *mut T as *mut u8,
            &mut data_size,
        )
    };

    if ret != 0 {
        crate::bail!(
            "RegQueryValueExW failed, value={:?}, code={ret}",
            value_name
        );
    }

    if data_type != expected_type {
        crate::bail!(
            "Unexpected registry value type, expected={expected_type}, actual={data_type}"
        );
    }

    Ok(value)
}

fn load_timezone_info(
    timezone: &str,
    leb: &mut LocaleEmulatorEnvironmentBlock,
) -> crate::Result<()> {
    let key = format!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Time Zones\\{timezone}");
    let key_w = key.as_bytes().to_wide_null_utf8();

    let mut hkey: HKEY = core::ptr::null_mut();
    let open_ret =
        unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, key_w.as_ptr(), 0, KEY_READ, &mut hkey) };
    if open_ret != 0 {
        crate::bail!("RegOpenKeyExW failed for timezone '{timezone}', code={open_ret}");
    }

    scopeguard::defer!(unsafe {
        RegCloseKey(hkey);
    });

    let std_name: [u16; 32] = query_reg_value_into(hkey, w!("Std"), REG_SZ)?;
    let dlt_name: [u16; 32] = query_reg_value_into(hkey, w!("Dlt"), REG_SZ)?;
    let tzi: RegTziFormat = query_reg_value_into(hkey, w!("TZI"), REG_BINARY)?;

    leb.timezone.standard_name = std_name;
    leb.timezone.daylight_name = dlt_name;
    leb.timezone.bias = tzi.bias;
    leb.timezone.standard_bias = tzi.standard_bias;
    // 与上游 C++ 保持一致：固定写 0
    leb.timezone.daylight_bias = 0;
    leb.timezone.standard_start = TimeFields::from_system_time(tzi.standard_date);
    leb.timezone.daylight_start = TimeFields::from_system_time(tzi.daylight_date);

    Ok(())
}

unsafe fn relaunch(process_info: *mut MlProcessInformation) -> crate::Result<()> {
    let mut leb = LocaleEmulatorEnvironmentBlock {
        ansi_code_page: crate::constant::EMULATE_LOCALE_CODEPAGE,
        oem_code_page: crate::constant::EMULATE_LOCALE_CODEPAGE,
        locale_id: crate::constant::EMULATE_LOCALE_LOCALE,
        default_charset: crate::constant::EMULATE_LOCALE_CHARSET,
        ..Default::default()
    };

    if let Err(_e) = load_timezone_info(crate::constant::EMULATE_LOCALE_TIMEZONE, &mut leb) {
        debug!("Failed to load timezone info: {_e:?}");
    }

    let exe_path = crate::utils::win32::get_module_file_name(core::ptr::null_mut(), true)?;
    let current_directory = crate::utils::win32::get_current_dir(true)?;

    let loader = crate::utils::win32::load_library(w!("LoaderDll.dll"))?;

    let proc =
        crate::utils::win32::get_module_symbol_addr_from_handle(loader, s!("LeCreateProcess"))?;
    let le_create_process: LeCreateProcessFn = unsafe { core::mem::transmute(proc) };

    let mut startup_info: STARTUPINFOW = unsafe { core::mem::zeroed() };
    startup_info.cb = core::mem::size_of::<STARTUPINFOW>() as u32;
    let mut local_process_info = MlProcessInformation::default();
    let target_process_info = if process_info.is_null() {
        &mut local_process_info
    } else {
        unsafe { &mut *process_info }
    };

    let ret = unsafe {
        le_create_process(
            &mut leb,
            exe_path.as_ptr(),
            GetCommandLineW(),
            current_directory.as_ptr(),
            0,
            &mut startup_info,
            target_process_info,
            core::ptr::null(),
            core::ptr::null(),
            core::ptr::null(),
            0,
        )
    };

    if ret != ERROR_SUCCESS {
        crate::bail!("LeCreateProcess failed with error code {ret}");
    }

    Ok(())
}

/// 使用 Locale Emulator (LE) 环境重新启动当前进程。
///
/// ### 工作原理
/// 1. **检测环境**：首先调用 `GetACP()` 获取当前进程的 ANSI 代码页。
///    - 如果当前代码页已符合 `EMULATE_LOCALE_CODEPAGE` 的预期，则直接返回 `Ok(())`。
/// 2. **构造环境**：从注册表中加载目标时区的详细信息（`TZI`），并填充 `LocaleEmulatorEnvironmentBlock` (LEB)。
/// 3. **创建子进程**：通过加载 `LoaderDll.dll` 并调用其导出的 `LeCreateProcess` 函数，以挂起状态启动当前程序副本。
/// 4. **注入与 Hook**：LE 引擎会向新进程注入模拟环境，使其在后续执行中认为自己运行在设定的区域、语言和时区下。
/// 5. **接力退出**：新进程启动成功后，当前进程会根据配置决定是否等待子进程结束，随后调用 `ExitProcess(0)` 自行终止。
///
/// ### 关键细节说明
/// - **原始命令行**：使用 `GetCommandLineW()` 直接传递当前进程的原始 UTF-16 命令行字符串，
///   确保了参数在传递过程中不会因为不当的字符串处理（如错误的空格转义）而发生信息丢失。
/// - **结构体校验**：内部显式初始化了 `STARTUPINFOW.cb`，以符合 Win32 API 对结构体大小校验的要求。
/// - **时区同步**：会自动同步 `standard_bias` 等关键时区参数，确保模拟环境的时间行为与真实区域一致。
///
/// ### 注意事项
/// - 必须确保程序运行目录下存在 `LoaderDll.dll`。
/// - 此操作会导致当前进程终止，请确保在调用前已保存所有必要状态。
///
/// # Errors
/// 如果发生以下情况，将返回 `Err`：
/// - 无法读取系统注册表中的时区信息。
/// - `LoaderDll.dll` 加载失败或找不到 `LeCreateProcess` 符号。
/// - 创建新进程失败。
pub fn relaunch_with_locale_emulator() -> crate::Result<()> {
    let current_cp = unsafe { GetACP() };

    if current_cp == crate::constant::EMULATE_LOCALE_CODEPAGE {
        debug!(
            "Codepage {} already matches expectation",
            crate::constant::EMULATE_LOCALE_CODEPAGE
        );
        return Ok(());
    }

    debug!(
        "Codepage {} did not match expectation ({})",
        current_cp,
        crate::constant::EMULATE_LOCALE_CODEPAGE
    );

    let mut process_info = MlProcessInformation::default();
    unsafe { relaunch(&mut process_info)? };

    if crate::constant::EMULATE_LOCALE_WAIT_FOR_EXIT {
        let process_handle = process_info.process_information.hProcess;
        if !process_handle.is_null() {
            unsafe {
                WaitForSingleObject(process_handle, INFINITE);
            }
        }
    }

    debug!("LocaleEmulator relaunch finished, terminating current process");

    unsafe { ExitProcess(0) }
}
