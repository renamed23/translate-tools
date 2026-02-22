use translate_macros::detour_trait;
use windows_sys::{
    Win32::{
        Foundation::HANDLE,
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{WIN32_FIND_DATAA, WIN32_FIND_DATAW},
        System::IO::OVERLAPPED,
    },
    core::{BOOL, PCSTR, PCWSTR},
};

#[detour_trait]
pub trait FileHook: Send + Sync + 'static {
    #[detour(
        dll = "kernel32.dll",
        symbol = "CreateFileA",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn create_file_a(
        lp_file_name: PCSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        #[cfg(any(feature = "create_file_redirect", feature = "resource_pack"))]
        unsafe {
            #[cfg(feature = "resource_pack")]
            use crate::utils::exts::slice_ext::ByteSliceExt;

            let filename_bytes = crate::utils::mem::slice_until_null(lp_file_name, 4096);

            #[cfg(feature = "create_file_redirect")]
            {
                use crate::constant::{REDIRECTION_SRC_PATH, REDIRECTION_TARGET_PATH};

                // 检查文件名是否以 REDIRECTION_SRC_PATH 结尾
                if let Some(tail) = filename_bytes.get(
                    filename_bytes
                        .len()
                        .saturating_sub(REDIRECTION_SRC_PATH.len())..,
                ) && tail.eq_ignore_ascii_case(REDIRECTION_SRC_PATH.as_bytes())
                {
                    crate::debug!(
                        "'{REDIRECTION_SRC_PATH}' file hooked, replace to '{REDIRECTION_TARGET_PATH}'"
                    );
                    let mut new_path = filename_bytes
                        [..filename_bytes.len() - REDIRECTION_SRC_PATH.len()]
                        .to_vec();
                    new_path.extend_from_slice(
                        const_str::concat!(REDIRECTION_TARGET_PATH, "\0").as_bytes(),
                    );

                    return crate::call!(
                        HOOK_CREATE_FILE_A,
                        new_path.as_ptr(),
                        dw_desired_access,
                        dw_share_mode,
                        lp_security_attributes,
                        dw_creation_disposition,
                        dw_flags_and_attributes,
                        h_template_file,
                    );
                }
            }

            #[cfg(feature = "resource_pack")]
            if let Some(handle) = try_redirect(
                &filename_bytes.to_wide(0),
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            ) {
                return handle;
            }

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
        }

        #[cfg(not(any(feature = "create_file_redirect", feature = "resource_pack")))]
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "CreateFileW",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn create_file_w(
        lp_file_name: PCWSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        #[cfg(feature = "resource_pack")]
        unsafe {
            if let Some(handle) = try_redirect(
                crate::utils::mem::slice_until_null(lp_file_name, 4096),
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            ) {
                return handle;
            }

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
        }

        #[cfg(not(feature = "resource_pack"))]
        unimplemented!();
    }

    #[allow(unused_variables)]
    #[detour(
        dll = "kernel32.dll",
        symbol = "ReadFile",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn read_file(
        h_file: HANDLE,
        lp_buffer: *mut u8,
        n_number_of_bytes_to_read: u32,
        lp_number_of_bytes_read: *mut u32,
        lp_overlapped: *mut OVERLAPPED,
    ) -> BOOL {
        #[cfg(not(feature = "read_file_patch_impl"))]
        unimplemented!();

        #[cfg(feature = "read_file_patch_impl")]
        unsafe {
            use windows_sys::Win32::Foundation::FALSE;

            let result = crate::call!(
                HOOK_READ_FILE,
                h_file,
                lp_buffer,
                n_number_of_bytes_to_read,
                lp_number_of_bytes_read,
                lp_overlapped,
            );

            if result == FALSE {
                crate::debug!("ReadFile failed");
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
                crate::debug!("ReadFile: lp_number_of_bytes_read is NULL");
                return result;
            };

            crate::patch::process_buffer(lp_buffer, len);
            result
        }
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "CloseHandle",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn close_handle(_h_object: HANDLE) -> BOOL {
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "FindFirstFileA",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn find_first_file_a(
        _lp_file_name: PCSTR,
        _lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> HANDLE {
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "FindFirstFileW",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn find_first_file_w(
        _lp_file_name: PCWSTR,
        _lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> HANDLE {
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "FindNextFileA",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn find_next_file_a(
        _h_find_file: HANDLE,
        _lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> BOOL {
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "FindNextFileW",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn find_next_file_w(
        _h_find_file: HANDLE,
        _lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> BOOL {
        unimplemented!();
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "FindClose",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn find_close(_h_find_file: HANDLE) -> BOOL {
        unimplemented!();
    }
}

/// 尝试将传入文件路径重定向到资源包中的替代文件。
#[cfg(feature = "resource_pack")]
fn try_redirect(
    u16_filename: &[u16],
    dw_desired_access: u32,
    dw_share_mode: u32,
    lp_security_attributes: *const SECURITY_ATTRIBUTES,
    dw_creation_disposition: u32,
    dw_flags_and_attributes: u32,
    h_template_file: HANDLE,
) -> Option<HANDLE> {
    use crate::utils::exts::{path_ext::PathExt, slice_ext::WideSliceExt};

    let orig_path = u16_filename.to_path_buf();
    match crate::resource_pack::get_resource_path(&orig_path) {
        Ok(Some(path)) => {
            crate::debug!(
                "Resource pack hooked file: {}, replace to {}",
                orig_path.to_string_lossy(),
                path.to_string_lossy()
            );

            let handle = unsafe {
                crate::call!(
                    HOOK_CREATE_FILE_W,
                    path.to_wide_null().as_ptr(),
                    dw_desired_access,
                    dw_share_mode,
                    lp_security_attributes,
                    dw_creation_disposition,
                    dw_flags_and_attributes,
                    h_template_file,
                )
            };

            return Some(handle);
        }
        Err(e) => {
            crate::debug!(
                "Failed to get resource path for {}: {e:?}",
                orig_path.to_string_lossy()
            );
        }
        _ => (),
    }

    None
}
