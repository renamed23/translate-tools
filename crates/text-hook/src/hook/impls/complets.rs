use translate_macros::{DefaultHook, detour_fn};
use windows_sys::Win32::Foundation::{HMODULE, WIN32_ERROR};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_OPEN_CREATE_OPTIONS, REG_SAM_FLAGS,
};
use windows_sys::core::PCSTR;

use crate::constant::ARG_REG_PATH;
use crate::hook::traits::CoreHook;
use crate::{debug, print_last_error_message};

#[derive(DefaultHook)]
pub struct CompletsHook;

impl CoreHook for CompletsHook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        unsafe {
            HOOK_REG_OPEN_KEY_EX_A.enable().unwrap();
            HOOK_REG_CREATE_KEY_EX_A.enable().unwrap();
        };
    }

    fn on_process_detach(_hinst_dll: HMODULE) {
        unsafe {
            HOOK_REG_OPEN_KEY_EX_A.disable().unwrap();
            HOOK_REG_CREATE_KEY_EX_A.disable().unwrap();
        };
    }
}

const EXPECTED: &str = const_str::concat!("Software\\", ARG_REG_PATH, "\\savedata");

#[detour_fn(dll = "advapi32.dll", symbol = "RegOpenKeyExA", fallback = "1")]
unsafe extern "system" fn reg_open_key_ex_a(
    hkey: HKEY,
    lpsubkey: PCSTR,
    uloptions: u32,
    samdesired: REG_SAM_FLAGS,
    phkresult: *mut HKEY,
) -> WIN32_ERROR {
    unsafe {
        if hkey == HKEY_LOCAL_MACHINE && !lpsubkey.is_null() {
            let subkey =
                String::from_utf8_lossy(crate::utils::mem::slice_until_null(lpsubkey, 1024));

            debug!("get subkey : {subkey}");

            if subkey.eq_ignore_ascii_case(EXPECTED) {
                let result = crate::call!(
                    HOOK_REG_OPEN_KEY_EX_A,
                    HKEY_CURRENT_USER,
                    lpsubkey,
                    uloptions,
                    samdesired,
                    phkresult,
                );

                if result != 0 {
                    print_last_error_message!();
                }

                return result;
            }
        }

        crate::call!(
            HOOK_REG_OPEN_KEY_EX_A,
            hkey,
            lpsubkey,
            uloptions,
            samdesired,
            phkresult
        )
    }
}

#[detour_fn(dll = "advapi32.dll", symbol = "RegCreateKeyExA", fallback = "1")]
unsafe extern "system" fn reg_create_key_ex_a(
    hkey: HKEY,
    lpsubkey: PCSTR,
    reserved: u32,
    lpclass: PCSTR,
    dwoptions: REG_OPEN_CREATE_OPTIONS,
    samdesired: REG_SAM_FLAGS,
    lpsecurityattributes: *const ::core::ffi::c_void,
    phkresult: *mut HKEY,
    lpdwdisposition: *mut u32,
) -> WIN32_ERROR {
    unsafe {
        if hkey == HKEY_LOCAL_MACHINE && !lpsubkey.is_null() {
            let subkey =
                String::from_utf8_lossy(crate::utils::mem::slice_until_null(lpsubkey, 1024));

            debug!("get subkey : {subkey}");

            if subkey.eq_ignore_ascii_case(EXPECTED) {
                let result = crate::call!(
                    HOOK_REG_CREATE_KEY_EX_A,
                    HKEY_CURRENT_USER,
                    lpsubkey,
                    reserved,
                    lpclass,
                    dwoptions,
                    samdesired,
                    lpsecurityattributes,
                    phkresult,
                    lpdwdisposition,
                );

                if result != 0 {
                    print_last_error_message!();
                }

                return result;
            }
        }

        crate::call!(
            HOOK_REG_CREATE_KEY_EX_A,
            hkey,
            lpsubkey,
            reserved,
            lpclass,
            dwoptions,
            samdesired,
            lpsecurityattributes,
            phkresult,
            lpdwdisposition,
        )
    }
}
