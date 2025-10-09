use translate_macros::ffi_catch_unwind;

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn replace_script(dst_ptr: *mut u8, src_ptr: *mut u8) {
    unsafe {
        let len = core::ptr::read_unaligned(src_ptr as *const u32) as usize;
        crate::patch::try_patching(dst_ptr, len);
    }
}
