use cfg_if::cfg_if;

use windows_sys::{
    Win32::{Foundation::HANDLE, Security::SECURITY_ATTRIBUTES},
    core::{PCSTR, PCWSTR},
};

use crate::{
    hook::api_hooks::filesystem::{CreateFile, HOOK_CREATE_FILE_A, HOOK_CREATE_FILE_W},
    utils::exts::{ptr_ext::PtrExt, slice_ext::ByteSliceExt},
};

#[allow(dead_code)]
pub struct AssetVirtualizer;

cfg_if! {
    if #[cfg(feature = "bind_asset_virtualizer")] {
        type AssetVirtualizerSlot = crate::hook::impls::HookImplType;
    } else {
        type AssetVirtualizerSlot = AssetVirtualizer;
    }
}

impl CreateFile for AssetVirtualizerSlot {
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
            if let Some(handle) = try_redirect(
                lp_file_name.to_slice_until_null_scan(),
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
    }
}

/// 尝试将传入文件路径重定向到资源包中的替代文件。
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
