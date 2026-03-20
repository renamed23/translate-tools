use std::cell::Cell;

use translate_macros::ffi_guard;

use crate::{
    debug,
    utils::exts::{ptr_ext::PtrExt, slice_ext::ByteSliceExt},
};

thread_local! {
    static SNR_FILE_OCCUR: Cell<bool> = const { Cell::new(false) };
}

#[ffi_guard(on_err_or_panic = ())]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn prepare_for_replace() {
    #[cfg(feature = "debug_output")]
    if SNR_FILE_OCCUR.get() {
        debug!("SNR_FILE_OCCUR set to true more than once!");
    }

    SNR_FILE_OCCUR.set(true);
}

#[ffi_guard(on_err_or_panic = ())]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn replace_script(ptr: *mut u8, len: usize) {
    if SNR_FILE_OCCUR.get() {
        SNR_FILE_OCCUR.set(false);

        debug!("ptr: 0x{:X}, len: 0x{len:X}", ptr as usize);

        unsafe {
            if let Ok(Some(patch)) = ptr.to_slice_mut(len).get_patch_or_extract() {
                ptr.to_slice_mut(len).copy_from_slice(patch);
            }
        }
    }
}
