use windows_sys::{
    Win32::{Foundation::HANDLE, Security::SECURITY_ATTRIBUTES},
    core::{PCSTR, PCWSTR},
};

use crate::{
    hook::api_hooks::filesystem::{CreateFile, HOOK_CREATE_FILE_A, HOOK_CREATE_FILE_W},
    utils::exts::{
        path_ext::PathExt,
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[allow(dead_code)]
pub struct Vfs;

type VfsSlot = cfg_select! {
    feature = "bind_vfs" => crate::hook::impls::HookImplType,
    _ => Vfs
};

impl CreateFile for VfsSlot {
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
            let wide = lp_file_name.to_slice_until_null_scan().to_wide(0);
            let path = wide.to_path_buf();

            match crate::vfs::try_redirect(&path) {
                Ok(Some(target)) => {
                    crate::debug!(
                        "VFS: {} -> {}",
                        path.to_string_lossy(),
                        target.to_string_lossy()
                    );
                    return crate::call!(
                        HOOK_CREATE_FILE_W,
                        target.to_wide_null().as_ptr(),
                        dw_desired_access,
                        dw_share_mode,
                        lp_security_attributes,
                        dw_creation_disposition,
                        dw_flags_and_attributes,
                        h_template_file,
                    );
                }
                Err(e) => {
                    crate::debug!(
                        "VFS redirect failed for {}: {e:?}",
                        path.to_string_lossy()
                    );
                }
                _ => {}
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

    unsafe fn create_file_w(
        lp_file_name: PCWSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        unsafe {
            let path = lp_file_name.to_slice_until_null_scan().to_path_buf();

            match crate::vfs::try_redirect(&path) {
                Ok(Some(target)) => {
                    crate::debug!(
                        "VFS: {} -> {}",
                        path.to_string_lossy(),
                        target.to_string_lossy()
                    );
                    return crate::call!(
                        HOOK_CREATE_FILE_W,
                        target.to_wide_null().as_ptr(),
                        dw_desired_access,
                        dw_share_mode,
                        lp_security_attributes,
                        dw_creation_disposition,
                        dw_flags_and_attributes,
                        h_template_file,
                    );
                }
                Err(e) => {
                    crate::debug!(
                        "VFS redirect failed for {}: {e:?}",
                        path.to_string_lossy()
                    );
                }
                _ => {}
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
    }
}
