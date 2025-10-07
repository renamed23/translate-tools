use translate_macros::ffi_catch_unwind;

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn replace_script(ptr: *mut u8, len: usize) {
    unsafe { crate::patch::try_patching(ptr, len) }
}
