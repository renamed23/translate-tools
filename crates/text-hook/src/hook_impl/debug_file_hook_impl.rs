use std::collections::HashMap;
use std::sync::RwLock;

use winapi::shared::minwindef::{BOOL, DWORD, LPDWORD, LPVOID, TRUE};
use winapi::shared::ntdef::LPCSTR;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::minwinbase::{LPOVERLAPPED, LPSECURITY_ATTRIBUTES};
use winapi::um::winnt::HANDLE;

use crate::code_cvt::ansi_to_wide_char;
use crate::debug_msg;
use crate::hook::file_hook::{FileHook, HOOK_CLOSE_HANDLE, HOOK_CREATE_FILE, HOOK_READ_FILE};

#[derive(Default)]
pub struct DebugFileHook {
    handles: RwLock<HashMap<usize, String>>,
}

impl FileHook for DebugFileHook {
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
        // 将lp_file_name转换为String
        let file_name = if !lp_file_name.is_null() {
            let wide_str =
                ansi_to_wide_char(unsafe { core::ffi::CStr::from_ptr(lp_file_name).to_bytes() });
            String::from_utf16_lossy(&wide_str)
        } else {
            String::from("(null)")
        };

        debug_msg!("CreateFileA called: {}", file_name);

        // 调用原始函数
        let result = unsafe {
            HOOK_CREATE_FILE.call(
                lp_file_name,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            )
        };

        // 如果句柄有效，存入handles
        if result != INVALID_HANDLE_VALUE
            && let Ok(mut handles) = self.handles.write()
        {
            handles.insert(result as usize, file_name);
        }

        result
    }

    unsafe fn read_file(
        &self,
        h_file: HANDLE,
        lp_buffer: LPVOID,
        n_number_of_bytes_to_read: DWORD,
        lp_number_of_bytes_read: LPDWORD,
        lp_overlapped: LPOVERLAPPED,
    ) -> BOOL {
        // 检查句柄是否在handles中
        if let Ok(handles) = self.handles.read()
            && let Some(file_name) = handles.get(&(h_file as usize))
        {
            debug_msg!(
                "ReadFile called for: {} (bytes to read: {}, start from buffer: {:p})",
                file_name,
                n_number_of_bytes_to_read,
                lp_buffer
            );
        }

        // 调用原始函数
        let result = unsafe {
            HOOK_READ_FILE.call(
                h_file,
                lp_buffer,
                n_number_of_bytes_to_read,
                lp_number_of_bytes_read,
                lp_overlapped,
            )
        };

        // 记录实际读取的字节数
        if result == TRUE
            && !lp_number_of_bytes_read.is_null()
            && let Ok(handles) = self.handles.read()
            && let Some(file_name) = handles.get(&(h_file as usize))
        {
            let bytes_read = unsafe { *lp_number_of_bytes_read };
            debug_msg!(
                "ReadFile completed for: {} (bytes read: {})",
                file_name,
                bytes_read
            );
        }

        result
    }

    unsafe fn close_handle(&self, h_object: HANDLE) -> BOOL {
        // 检查句柄是否在handles中
        let file_name = if let Ok(mut handles) = self.handles.write() {
            handles.remove(&(h_object as usize))
        } else {
            None
        };

        if let Some(name) = &file_name {
            debug_msg!("CloseHandle called for: {}", name);
        }

        unsafe { HOOK_CLOSE_HANDLE.call(h_object) }
    }
}
