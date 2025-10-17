use translate_macros::{detour, generate_detours};
use winapi::shared::minwindef::{DWORD, WORD};

use crate::debug;

#[generate_detours]
pub trait LocaleHook: Send + Sync + 'static {
    #[detour(
        dll = "kernel32.dll",
        symbol = "GetSystemDefaultLCID",
        fallback = "0x0411"
    )]
    unsafe fn get_system_default_lcid(&self) -> DWORD {
        0x0411
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "GetSystemDefaultLangID",
        fallback = "0x0411"
    )]
    unsafe fn get_system_default_lang_id(&self) -> WORD {
        0x0411
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "GetUserDefaultLCID",
        fallback = "0x0411"
    )]
    unsafe fn get_user_default_lcid(&self) -> DWORD {
        0x0411
    }

    #[detour(
        dll = "kernel32.dll",
        symbol = "GetUserDefaultLangID",
        fallback = "0x0411"
    )]
    unsafe fn get_user_default_lang_id(&self) -> WORD {
        0x0411
    }
}

/// 开启区域设置相关的钩子
#[allow(dead_code)]
pub fn enable_hooks() {
    unsafe {
        HOOK_GET_SYSTEM_DEFAULT_LCID.enable().unwrap();
        HOOK_GET_SYSTEM_DEFAULT_LANG_ID.enable().unwrap();
        HOOK_GET_USER_DEFAULT_LCID.enable().unwrap();
        HOOK_GET_USER_DEFAULT_LANG_ID.enable().unwrap();
    }

    debug!("Locale Hooked!");
}

/// 关闭区域设置相关的钩子
#[allow(dead_code)]
pub fn disable_hooks() {
    unsafe {
        HOOK_GET_SYSTEM_DEFAULT_LCID.disable().unwrap();
        HOOK_GET_SYSTEM_DEFAULT_LANG_ID.disable().unwrap();
        HOOK_GET_USER_DEFAULT_LCID.disable().unwrap();
        HOOK_GET_USER_DEFAULT_LANG_ID.disable().unwrap();
    }

    debug!("Locale Unhooked!");
}
