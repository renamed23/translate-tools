use ntapi::ntpebteb::PEB;
use ntapi::ntpsapi::PROCESS_BASIC_INFORMATION;
use ntapi::ntpsapi::{NtQueryInformationProcess, ProcessBasicInformation};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::print_last_error_message;

/// 通过 `NtQueryInformationProcess` 系统调用获取当前进程的 PEB（进程环境块）地址
///
/// # 返回
///
/// - `Ok(*mut PEB)` - 成功获取 PEB 基地址，返回指向 PEB 结构的可变指针(保证不为null)
/// - `Err` - 查询失败
///
/// # Safety
/// - 调用者必须保证当前进程环境允许调用 `NtQueryInformationProcess`。
/// - 返回的 PEB 指针仅在进程生命周期内有效，且调用者需自行保证后续解引用安全。
pub unsafe fn get_current_peb() -> crate::Result<*mut PEB> {
    let mut pbi: PROCESS_BASIC_INFORMATION = unsafe { core::mem::zeroed() };
    let status = unsafe {
        NtQueryInformationProcess(
            GetCurrentProcess() as *mut _,
            ProcessBasicInformation,
            &mut pbi as *mut PROCESS_BASIC_INFORMATION as *mut _,
            size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            core::ptr::null_mut(),
        )
    };

    if status < 0 {
        print_last_error_message!(nt status);
        crate::bail!(
            "NtQueryInformationProcess failed with status 0x{:X}",
            status
        );
    }

    if pbi.PebBaseAddress.is_null() {
        crate::bail!("PEB base address is null");
    }

    Ok(pbi.PebBaseAddress)
}
