use translate_macros::{byte_slice, ffi_catch_unwind};

use crate::debug;

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn patch_asm() {
    let handle = crate::utils::win32::get_module_handle("system.unt").unwrap();

    debug!("patch {handle:p}");

    let module_addr = handle as *mut u8;

    unsafe {
        crate::utils::mem::patch::write_asm(
            module_addr.add(0x27F4E),
            // jmp system.10034A00
            &byte_slice!("E9 AD CA 00 00"),
        )
        .unwrap();

        let code_buf = crate::utils::mem::patch::generate_trampoline_stub_32(
            replace_script as *const () as _,
            // mov eax,[esp+28]; push eax
            &byte_slice!("8B 44 24 28 50"),
            // ret 0xC;
            &byte_slice!("C2 0C 00"),
        );

        crate::utils::mem::patch::write_asm(module_addr.add(0x34A00), &code_buf).unwrap();
    }
}

#[ffi_catch_unwind]
pub unsafe extern "system" fn replace_script(ptr: *mut u8) {
    unsafe {
        if !crate::utils::mem::quick_memory_check_win32(ptr, 0x18) {
            return;
        }

        // 读取 ptr + 0x14上的u32小端
        let len = std::ptr::read_unaligned(ptr.add(0x14) as *const u32) as usize;
        crate::patch::process_buffer(ptr, len);
    }
}
