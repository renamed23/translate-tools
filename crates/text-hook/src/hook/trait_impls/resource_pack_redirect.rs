use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use windows_sys::Win32::{Foundation::HANDLE, Security::SECURITY_ATTRIBUTES};

use crate::hook::traits::file_hook::HOOK_CREATE_FILE_W;

/// 尝试将传入文件路径重定向到资源包中的替代文件。
pub fn try_redirect(
    u16_filename: &[u16],
    dw_desired_access: u32,
    dw_share_mode: u32,
    lp_security_attributes: *const SECURITY_ATTRIBUTES,
    dw_creation_disposition: u32,
    dw_flags_and_attributes: u32,
    h_template_file: HANDLE,
) -> Option<HANDLE> {
    let orig_path = PathBuf::from(OsString::from_wide(u16_filename));
    match crate::resource_pack::get_resource_path(&orig_path) {
        Ok(Some(path)) => {
            use std::os::windows::ffi::OsStrExt;

            crate::debug!(
                "Resource pack hooked file: {}, replace to {}",
                orig_path.to_string_lossy(),
                path.to_string_lossy()
            );

            let wide_path: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(core::iter::once(0))
                .collect();

            let handle = unsafe {
                crate::call!(
                    HOOK_CREATE_FILE_W,
                    wide_path.as_ptr(),
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
