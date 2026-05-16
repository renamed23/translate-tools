use windows_sys::{
    Win32::{Foundation::HANDLE, Security::SECURITY_ATTRIBUTES},
    core::PCSTR,
};

use crate::{
    constant::{REDIRECTION_SRC_PATH, REDIRECTION_TARGET_PATH},
    hook::api_hooks::filesystem::{CreateFile, HOOK_CREATE_FILE_A},
    utils::exts::ptr_ext::PtrExt,
};

#[allow(dead_code)]
pub struct PathRedirector;

cfg_select! {
    feature = "bind_path_redirector" => {
        type PathRedirectorSlot = crate::hook::impls::HookImplType;
    }
    _ => {
        type PathRedirectorSlot = PathRedirector;
    }
}

impl CreateFile for PathRedirectorSlot {
    unsafe fn create_file_a(
        lp_file_name: PCSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        unsafe {
            let filename_bytes = lp_file_name.to_slice_until_null_scan();

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
                let mut new_path =
                    filename_bytes[..filename_bytes.len() - REDIRECTION_SRC_PATH.len()].to_vec();
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
    }
}
