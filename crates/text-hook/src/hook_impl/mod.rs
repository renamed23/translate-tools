#[cfg(feature = "snow_radish")]
pub mod snow_radish;

#[cfg(feature = "bleed")]
pub mod bleed;

#[cfg(feature = "sukisuki")]
pub mod sukisuki;

#[cfg(feature = "ao_vo")]
pub mod ao_vo;

#[cfg(feature = "noise")]
pub mod noise;

#[cfg(feature = "lusts")]
pub mod lusts;

#[cfg(feature = "summer_radish")]
pub mod summer_radish;

#[cfg(feature = "c4")]
pub mod c4;

#[cfg(feature = "debug_file_hook_impl")]
pub mod debug_file_hook_impl;

#[cfg(feature = "bittersweet_fools")]
pub mod bittersweet_fools;

#[cfg(feature = "white_breath")]
pub mod white_breath;

// ---------------------- 钩子实现类型 ------------------------------

#[cfg(feature = "default_hook_impl")]
pub type HookImplType = DefaultHook;

#[cfg(feature = "bleed")]
pub type HookImplType = bleed::BleedHook;

#[cfg(feature = "debug_file_hook_impl")]
pub type HookImplType = debug_file_hook_impl::DebugFileHook;

#[cfg(feature = "snow_radish")]
pub type HookImplType = snow_radish::SnowRadishHook;

#[cfg(feature = "bittersweet_fools")]
pub type HookImplType = bittersweet_fools::BittersweetFools;

#[cfg(feature = "summer_radish")]
pub type HookImplType = summer_radish::SummerRadishHook;

// ---------------------- DLL MAIN ----------------------------------

#[allow(unused_imports)]
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, HMODULE, LPVOID, TRUE};

use crate::hook::Hook;

/// 默认实现的钩子，应该可以应对大部分场景
#[allow(dead_code)]
#[derive(Default)]
pub struct DefaultHook;

impl Hook for DefaultHook {}

#[cfg(feature = "export_default_dll_main")]
#[translate_macros::ffi_catch_unwind(FALSE)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HMODULE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
) -> BOOL {
    default_dll_main(_hinst_dll, fdw_reason, _lpv_reserved)
}

/// 默认的 DllMain 实现
#[allow(dead_code)]
pub fn default_dll_main(_hinst_dll: HMODULE, fdw_reason: DWORD, _lpv_reserved: LPVOID) -> BOOL {
    const PROCESS_ATTACH: DWORD = 1;
    if fdw_reason == PROCESS_ATTACH {
        crate::panic_utils::set_debug_panic_hook();
        crate::hook::set_hook_instance(HookImplType::default());

        #[cfg(feature = "custom_font")]
        crate::custom_font::add_font();

        crate::hook::enable_text_hooks();

        #[cfg(feature = "debug_file_hook_impl")]
        crate::hook::enable_file_hooks();

        #[cfg(feature = "read_file_patch_impl")]
        unsafe {
            crate::hook::HOOK_READ_FILE.enable().unwrap();
        }
    }

    TRUE
}
