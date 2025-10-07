use translate_macros::ffi_catch_unwind;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, HMODULE, LPVOID, TRUE};

use crate::hook::DefaultHook;

#[ffi_catch_unwind(FALSE)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HMODULE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
) -> BOOL {
    const PROCESS_ATTACH: DWORD = 1;
    if fdw_reason == PROCESS_ATTACH {
        crate::panic_utils::set_debug_panic_hook();
        crate::hook::set_hook_instance(Box::new(DefaultHook));

        #[cfg(feature = "custom_font")]
        crate::custom_font::add_font();

        crate::hook::enable_text_hooks();
    }

    TRUE
}
