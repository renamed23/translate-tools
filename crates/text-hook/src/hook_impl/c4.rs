use std::cell::Cell;

use translate_macros::ffi_catch_unwind;

use crate::debug;

thread_local! {
    static SNR_FILE_OCCUR: Cell<bool> = const { Cell::new(false) };
}

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn prepare_for_replace() {
    #[cfg(feature = "debug_output")]
    if SNR_FILE_OCCUR.get() {
        debug!("SNR_FILE_OCCUR set to true more than once!");
    }

    SNR_FILE_OCCUR.set(true);
}

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn replace_script(ptr: *mut u8, len: usize) {
    unsafe {
        if SNR_FILE_OCCUR.get() {
            SNR_FILE_OCCUR.set(false);

            debug!("ptr: 0x{:X}, len: 0x{len:X}", ptr as usize);
            crate::patch::try_patching(ptr, len);
            // crate::patch::try_extracting(ptr, len);
        }
    }
}
