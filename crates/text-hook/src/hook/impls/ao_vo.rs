use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::core::PCSTR;

use crate::debug;
use crate::hook::traits::file_hook::HOOK_CREATE_FILE_A;
use crate::hook::traits::{CoreHook, FileHook, TextHook, WindowHook};

#[derive(Default)]
pub struct AoVoHook;

impl CoreHook for AoVoHook {
    fn enable_hooks(&self) {
        unsafe { HOOK_CREATE_FILE_A.enable().unwrap() };
    }

    fn disable_hooks(&self) {
        unsafe { HOOK_CREATE_FILE_A.disable().unwrap() };
    }
}

impl TextHook for AoVoHook {}

impl WindowHook for AoVoHook {}

impl FileHook for AoVoHook {
    unsafe fn create_file_a(
        &self,
        lp_file_name: PCSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        unsafe {
            let filename_bytes = crate::utils::mem::slice_until_null(lp_file_name, 512);
            let new_path;

            // 检查文件名是否以 "DATA2.TCD" 结尾
            let file_name_ptr = if filename_bytes.ends_with(b"DATA2.TCD") {
                debug!("'DATA2.TCD' file reading hooked, replace to 'DATA_chs.TCD'");
                let mut new_path_vec = filename_bytes[..filename_bytes.len() - 9].to_vec();
                new_path_vec.extend_from_slice(b"DATA_chs.TCD\0");
                new_path = Some(new_path_vec);
                new_path.as_ref().unwrap().as_ptr()
            } else {
                lp_file_name
            };

            HOOK_CREATE_FILE_A.call(
                file_name_ptr,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            )
        }
    }
}
