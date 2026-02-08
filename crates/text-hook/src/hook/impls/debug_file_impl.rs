use std::sync::RwLock;
use std::{collections::HashMap, sync::LazyLock};

use translate_macros::DefaultHook;
use windows_sys::{
    Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE, MAX_PATH, TRUE},
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{WIN32_FIND_DATAA, WIN32_FIND_DATAW},
        System::IO::OVERLAPPED,
    },
    core::{BOOL, PCSTR, PCWSTR},
};

use crate::code_cvt::multi_byte_to_wide_char;
use crate::debug;
use crate::hook::traits::file_hook::{
    FileHook, HOOK_CLOSE_HANDLE, HOOK_CREATE_FILE_A, HOOK_CREATE_FILE_W, HOOK_FIND_CLOSE,
    HOOK_FIND_FIRST_FILE_A, HOOK_FIND_FIRST_FILE_W, HOOK_FIND_NEXT_FILE_A, HOOK_FIND_NEXT_FILE_W,
    HOOK_READ_FILE,
};
use crate::{hook::traits::CoreHook, utils::mem::slice_until_null};

#[derive(DefaultHook)]
#[exclude(FileHook)]
pub struct DebugFileImplHook;

#[derive(Default)]
struct DebugFileState {
    handles: HashMap<usize, String>,
    find_handles: HashMap<usize, String>,
}

static DEBUG_FILE_STATE: LazyLock<RwLock<DebugFileState>> =
    LazyLock::new(|| RwLock::new(DebugFileState::default()));

impl CoreHook for DebugFileImplHook {
    fn enable_hooks() {
        unsafe {
            HOOK_CREATE_FILE_A.enable().unwrap();
            HOOK_CREATE_FILE_W.enable().unwrap();
            HOOK_READ_FILE.enable().unwrap();
            HOOK_CLOSE_HANDLE.enable().unwrap();
            HOOK_FIND_FIRST_FILE_A.enable().unwrap();
            HOOK_FIND_FIRST_FILE_W.enable().unwrap();
            HOOK_FIND_NEXT_FILE_A.enable().unwrap();
            HOOK_FIND_NEXT_FILE_W.enable().unwrap();
            HOOK_FIND_CLOSE.enable().unwrap();
        }
    }

    fn disable_hooks() {
        unsafe {
            HOOK_CREATE_FILE_A.disable().unwrap();
            HOOK_CREATE_FILE_W.disable().unwrap();
            HOOK_READ_FILE.disable().unwrap();
            HOOK_CLOSE_HANDLE.disable().unwrap();
            HOOK_FIND_FIRST_FILE_A.disable().unwrap();
            HOOK_FIND_FIRST_FILE_W.disable().unwrap();
            HOOK_FIND_NEXT_FILE_A.disable().unwrap();
            HOOK_FIND_NEXT_FILE_W.disable().unwrap();
            HOOK_FIND_CLOSE.disable().unwrap();
        }
    }
}

impl FileHook for DebugFileImplHook {
    unsafe fn create_file_a(
        lp_file_name: PCSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        let file_name = if !lp_file_name.is_null() {
            let ansi_bytes = unsafe { slice_until_null(lp_file_name, MAX_PATH as _) };
            String::from_utf16_lossy(&multi_byte_to_wide_char(ansi_bytes, 0))
        } else {
            String::from("(null)")
        };

        debug!(raw "CreateFileA called: {}", file_name);

        // 调用原始函数
        let result = unsafe {
            crate::call!(
                HOOK_CREATE_FILE_A,
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
            && let Ok(mut state) = DEBUG_FILE_STATE.write()
        {
            state.handles.insert(result as usize, file_name);
        }

        result
    }

    unsafe fn create_file_w(
        lp_file_name: PCWSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        // 使用工具函数安全地获取宽字符串
        let file_name = if !lp_file_name.is_null() {
            let wide_str = unsafe { slice_until_null(lp_file_name, MAX_PATH as _) };
            String::from_utf16_lossy(wide_str)
        } else {
            String::from("(null)")
        };

        debug!(raw "CreateFileW called: {}", file_name);

        // 调用原始函数
        let result = unsafe {
            crate::call!(
                HOOK_CREATE_FILE_W,
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
            && let Ok(mut state) = DEBUG_FILE_STATE.write()
        {
            state.handles.insert(result as usize, file_name);
        }

        result
    }

    unsafe fn read_file(
        h_file: HANDLE,
        lp_buffer: *mut u8,
        n_number_of_bytes_to_read: u32,
        lp_number_of_bytes_read: *mut u32,
        lp_overlapped: *mut OVERLAPPED,
    ) -> BOOL {
        // 检查句柄是否在handles中
        if let Ok(state) = DEBUG_FILE_STATE.read()
            && let Some(file_name) = state.handles.get(&(h_file as usize))
        {
            debug!(raw
                "ReadFile called for: {} (bytes to read: {}, start from buffer: {:p})",
                file_name,
                n_number_of_bytes_to_read,
                lp_buffer
            );
        }

        // 调用原始函数
        let result = unsafe {
            crate::call!(
                HOOK_READ_FILE,
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
            && let Ok(state) = DEBUG_FILE_STATE.read()
            && let Some(file_name) = state.handles.get(&(h_file as usize))
        {
            let bytes_read = unsafe { *lp_number_of_bytes_read };
            debug!(raw
                "ReadFile completed for: {} (bytes read: {})",
                file_name,
                bytes_read
            );
        }

        result
    }

    unsafe fn close_handle(h_object: HANDLE) -> BOOL {
        // 检查句柄是否在handles中
        let file_name = if let Ok(mut state) = DEBUG_FILE_STATE.write() {
            state.handles.remove(&(h_object as usize))
        } else {
            None
        };

        if let Some(name) = &file_name {
            debug!(raw "CloseHandle called for: {}", name);
        }

        unsafe { crate::call!(HOOK_CLOSE_HANDLE, h_object) }
    }

    unsafe fn find_first_file_a(
        lp_file_name: PCSTR,
        lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> HANDLE {
        let search_pattern = if !lp_file_name.is_null() {
            let ansi_bytes = unsafe { slice_until_null(lp_file_name, MAX_PATH as _) };
            String::from_utf16_lossy(&multi_byte_to_wide_char(ansi_bytes, 0))
        } else {
            String::from("(null)")
        };

        debug!(raw "FindFirstFileA called with pattern: {}", search_pattern);

        // 调用原始函数
        let result =
            unsafe { crate::call!(HOOK_FIND_FIRST_FILE_A, lp_file_name, lp_find_file_data) };

        // 如果句柄有效，存入find_handles
        if result != INVALID_HANDLE_VALUE
            && let Ok(mut state) = DEBUG_FILE_STATE.write()
        {
            state.find_handles.insert(result as usize, search_pattern);

            // 打印找到的第一个文件信息
            if !lp_find_file_data.is_null() {
                let find_data = unsafe { &*lp_find_file_data };
                let file_name_bytes = unsafe {
                    slice_until_null(
                        find_data.cFileName.as_ptr() as *const u8,
                        find_data.cFileName.len(),
                    )
                };
                let file_name =
                    String::from_utf16_lossy(&multi_byte_to_wide_char(file_name_bytes, 0));

                debug!(raw "FindFirstFileA found first file: {}", file_name);
            }
        }

        result
    }

    unsafe fn find_first_file_w(
        lp_file_name: PCWSTR,
        lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> HANDLE {
        // 使用工具函数安全地获取宽字符串
        let search_pattern = if !lp_file_name.is_null() {
            let wide_str = unsafe { slice_until_null(lp_file_name, MAX_PATH as _) };
            String::from_utf16_lossy(wide_str)
        } else {
            String::from("(null)")
        };

        debug!(raw "FindFirstFileW called with pattern: {}", search_pattern);

        // 调用原始函数
        let result =
            unsafe { crate::call!(HOOK_FIND_FIRST_FILE_W, lp_file_name, lp_find_file_data) };

        // 如果句柄有效，存入find_handles
        if result != INVALID_HANDLE_VALUE
            && let Ok(mut state) = DEBUG_FILE_STATE.write()
        {
            state.find_handles.insert(result as usize, search_pattern);

            // 打印找到的第一个文件信息
            if !lp_find_file_data.is_null() {
                let find_data = unsafe { &*lp_find_file_data };
                let file_name_wide = unsafe {
                    slice_until_null(find_data.cFileName.as_ptr(), find_data.cFileName.len())
                };
                let file_name = String::from_utf16_lossy(file_name_wide);

                debug!(raw "FindFirstFileW found first file: {}", file_name);
            }
        }

        result
    }

    unsafe fn find_next_file_a(
        h_find_file: HANDLE,
        lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> BOOL {
        // 检查句柄是否在find_handles中
        let search_pattern = if let Ok(state) = DEBUG_FILE_STATE.read() {
            state.find_handles.get(&(h_find_file as usize)).cloned()
        } else {
            None
        };

        if let Some(pattern) = &search_pattern {
            debug!(raw "FindNextFileA called for search pattern: {}", pattern);
        }

        // 调用原始函数
        let result = unsafe { crate::call!(HOOK_FIND_NEXT_FILE_A, h_find_file, lp_find_file_data) };

        // 如果调用成功，打印找到的文件名
        if result == TRUE && !lp_find_file_data.is_null() {
            let find_data = unsafe { &*lp_find_file_data };
            let file_name_bytes = unsafe {
                slice_until_null(
                    find_data.cFileName.as_ptr() as *const u8,
                    find_data.cFileName.len(),
                )
            };
            let file_name = String::from_utf16_lossy(&multi_byte_to_wide_char(file_name_bytes, 0));

            debug!(raw "FindNextFileA found file: {}", file_name);
        }

        result
    }

    unsafe fn find_next_file_w(
        h_find_file: HANDLE,
        lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> BOOL {
        // 检查句柄是否在find_handles中
        let search_pattern = if let Ok(state) = DEBUG_FILE_STATE.read() {
            state.find_handles.get(&(h_find_file as usize)).cloned()
        } else {
            None
        };

        if let Some(pattern) = &search_pattern {
            debug!(raw "FindNextFileW called for search pattern: {}", pattern);
        }

        // 调用原始函数
        let result = unsafe { crate::call!(HOOK_FIND_NEXT_FILE_W, h_find_file, lp_find_file_data) };

        // 如果调用成功，打印找到的文件名
        if result == TRUE && !lp_find_file_data.is_null() {
            let find_data = unsafe { &*lp_find_file_data };
            let file_name_wide = unsafe {
                slice_until_null(find_data.cFileName.as_ptr(), find_data.cFileName.len())
            };
            let file_name = String::from_utf16_lossy(file_name_wide);

            debug!(raw "FindNextFileW found file: {}", file_name);
        }

        result
    }

    unsafe fn find_close(h_find_file: HANDLE) -> BOOL {
        // 检查句柄是否在find_handles中
        let search_pattern = if let Ok(mut state) = DEBUG_FILE_STATE.write() {
            state.find_handles.remove(&(h_find_file as usize))
        } else {
            None
        };

        if let Some(pattern) = &search_pattern {
            debug!(raw "FindClose called for search pattern: {}", pattern);
        } else {
            debug!(raw "FindClose called for unknown handle: {:?}", h_find_file);
        }

        // 调用原始函数
        unsafe { crate::call!(HOOK_FIND_CLOSE, h_find_file) }
    }
}
