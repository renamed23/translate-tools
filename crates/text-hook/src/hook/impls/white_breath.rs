use translate_macros::{byte_slice, ffi_catch_unwind};
use windows_sys::w;

use crate::{
    debug,
    utils::exts::ptr_ext::{PtrExt, PtrWriteExt},
};

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn patch_asm() {
    let handle = crate::utils::win32::get_module_handle(w!("system.unt")).unwrap();

    debug!("patch {handle:p}");

    let module_addr = handle as *mut u8;

    unsafe {
        module_addr
            .add(0x27F4E)
            // jmp system.10034A00
            .patch_asm(&byte_slice!("E9 AD CA 00 00"))
            .unwrap();

        let code_buf = crate::utils::mem::patch::generate_trampoline_stub_32(
            replace_script as *const () as _,
            // mov eax,[esp+28]; push eax
            &byte_slice!("8B 44 24 28 50"),
            // ret 0xC;
            &byte_slice!("C2 0C 00"),
        );

        module_addr.add(0x34A00).patch_asm(&code_buf).unwrap();
    }
}

#[ffi_catch_unwind]
pub unsafe extern "system" fn replace_script(ptr: *mut u8) {
    unsafe {
        use crate::utils::exts::slice_ext::ByteSliceExt;

        if crate::utils::mem::quick_memory_check(ptr, 0x18).is_err() {
            return;
        }

        // 读取 ptr + 0x14上的u32小端
        let len = core::ptr::read_unaligned(ptr.add(0x14) as *const u32) as usize;
        if let Ok(Some(patch)) = ptr.to_slice_mut(len).get_patch_or_extract() {
            ptr.to_slice_mut(len).copy_from_slice(patch);
        }
    }
}
