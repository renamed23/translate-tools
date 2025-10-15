use translate_macros::{detour, generate_detours};
use winapi::shared::minwindef::{BOOL, DWORD, LPDWORD, LPVOID};
use winapi::shared::ntdef::HANDLE;
use winapi::um::fileapi::CreateFileW;
use winapi::um::minwinbase::{LPOVERLAPPED, LPSECURITY_ATTRIBUTES};
use winapi::um::winnt::LPCSTR;

use crate::debug;

#[generate_detours]
pub trait FileHook: Send + Sync + 'static {
    #[detour(
        dll = "kernel32.dll",
        symbol = "CreateFileA",
        fallback = "winapi::um::handleapi::INVALID_HANDLE_VALUE"
    )]
    unsafe fn create_file(
        &self,
        lp_file_name: LPCSTR,
        dw_desired_access: DWORD,
        dw_share_mode: DWORD,
        lp_security_attributes: LPSECURITY_ATTRIBUTES,
        dw_creation_disposition: DWORD,
        dw_flags_and_attributes: DWORD,
        h_template_file: HANDLE,
    ) -> HANDLE {
        let mut file_name_u16: Vec<u16> = {
            let bytes = unsafe { core::ffi::CStr::from_ptr(lp_file_name).to_bytes() };
            crate::code_cvt::ansi_font_to_wide_font(bytes)
        };

        file_name_u16.push(0);

        unsafe {
            CreateFileW(
                file_name_u16.as_ptr(),
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            )
        }
    }

    #[allow(unused_variables)]
    #[detour(
        dll = "kernel32.dll",
        symbol = "ReadFile",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn read_file(
        &self,
        h_file: HANDLE,
        lp_buffer: LPVOID,
        n_number_of_bytes_to_read: DWORD,
        lp_number_of_bytes_read: LPDWORD,
        lp_overlapped: LPOVERLAPPED,
    ) -> BOOL {
        let result = unsafe {
            HOOK_READ_FILE.call(
                h_file,
                lp_buffer,
                n_number_of_bytes_to_read,
                lp_number_of_bytes_read,
                lp_overlapped,
            )
        };

        #[cfg(feature = "read_file_patch_impl")]
        unsafe {
            use winapi::shared::minwindef::FALSE;

            if result == FALSE {
                debug!("ReadFile failed");
                return FALSE;
            }

            // 如果 lp_number_of_bytes_read 为 NULL
            // - 若 lp_overlapped 非 NULL（异步），我们无法得知实际读到多少字节，跳过 patch
            // - 若 lp_overlapped 为 NULL（同步），按规范 lp_number_of_bytes_read 不应为 NULL，跳过 patch
            let len: usize = if !lp_number_of_bytes_read.is_null() {
                // 安全地读取并 clamp 到请求的最大值，避免异常值
                let bytes = *lp_number_of_bytes_read as usize;
                let max = n_number_of_bytes_to_read as usize;
                core::cmp::min(bytes, max)
            } else {
                debug!("ReadFile: lp_number_of_bytes_read is NULL");
                return result;
            };

            let ptr = lp_buffer as *mut u8;
            crate::patch::process_buffer(ptr, len);
        }

        result
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "CloseHandle",
        fallback = "winapi::shared::minwindef::FALSE"
    )]
    unsafe fn close_handle(&self, h_object: HANDLE) -> BOOL {
        unsafe { HOOK_CLOSE_HANDLE.call(h_object) }
    }
}

/// 开启文件相关的钩子
#[allow(dead_code)]
pub fn enable_hooks() {
    unsafe {
        HOOK_CREATE_FILE.enable().unwrap();
        HOOK_READ_FILE.enable().unwrap();
        HOOK_CLOSE_HANDLE.enable().unwrap();
    }

    debug!("File Hooked!");
}
