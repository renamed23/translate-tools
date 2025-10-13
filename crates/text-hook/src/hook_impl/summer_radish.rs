use translate_macros::ffi_catch_unwind;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, HMODULE, LPVOID};

use crate::debug;
use crate::hook_impl::{DefaultHook, default_dll_main};
use crate::hook_utils::iat_patch::patch_iat;

pub type SummerRadishHook = DefaultHook;

#[ffi_catch_unwind(FALSE)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HMODULE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
) -> BOOL {
    const PROCESS_ATTACH: DWORD = 1;
    if fdw_reason == PROCESS_ATTACH {
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
    }

    default_dll_main(_hinst_dll, fdw_reason, _lpv_reserved)
}
