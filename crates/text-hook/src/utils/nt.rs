use ntapi::ntpebteb::PEB;
use ntapi::ntpsapi::PROCESS_BASIC_INFORMATION;
use ntapi::ntpsapi::{NtQueryInformationProcess, ProcessBasicInformation};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::{debug, print_system_error_message};

/// 通过 `NtQueryInformationProcess` 系统调用获取当前进程的 PEB（进程环境块）地址
///
/// # 返回
///
/// - `Some(*mut PEB)` - 成功获取 PEB 基地址，返回指向 PEB 结构的可变指针(保证不为null)
/// - `None` - 查询失败
pub unsafe fn get_current_peb() -> Option<*mut PEB> {
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
        print_system_error_message!(NT status);
        return None;
    }

    if pbi.PebBaseAddress.is_null() {
        debug!("Error: pbi.PebBaseAddress is null");
        return None;
    }

    Some(pbi.PebBaseAddress)
}
