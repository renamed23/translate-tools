use translate_macros::ffi_catch_unwind;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, HMODULE, LPVOID, TRUE};

use crate::debug;
use crate::hook::set_hook_instance;
use crate::hook_impl::default_hook_impl::DefaultHook;
use crate::hook_utils::iat_patch::patch_iat;
use crate::panic_utils::set_debug_panic_hook;

#[ffi_catch_unwind(FALSE)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HMODULE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
) -> BOOL {
    const PROCESS_ATTACH: DWORD = 1;

    if fdw_reason == PROCESS_ATTACH {
        set_debug_panic_hook();

        match unsafe {
            patch_iat(
                "GObj_Font.mod",
                "gdi32.dll",
                &[(
                    c"GetGlyphOutlineA".as_ptr(),
                    crate::hook::get_glyph_outline as usize,
                )],
            )
        } {
            Ok(()) => debug!("patch_iat OK"),
            Err(e) => debug!("patch_iat failed with {e}"),
        }

        set_hook_instance(Box::new(DefaultHook));

        debug!("hook instance set");
    }
    TRUE
}
