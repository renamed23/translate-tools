use translate_macros::{byte_slice, ffi_catch_unwind};

use crate::debug;
use crate::hook_utils::{get_module_handle, write_asm};

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn patch_asm() {
    let Some(handle) = get_module_handle("system.unt") else {
        debug!("patch_asm: get_module_handle failed");
        return;
    };
    debug!("patch_asm: start {handle:p}");

    let dll_addr = handle as *mut u8;
    unsafe {
        // jmp system.10034A00
        write_asm(dll_addr.add(0x27F4E), &byte_slice!("E9 AD CA 00 00")).unwrap();

        // 跳板代码 到 replace_script
        let replace_addr = replace_script as usize;
        let mut buf: Vec<u8> = Vec::with_capacity(32);
        // pushad; pushfd; mov eax,[esp+28]; push eax
        buf.extend_from_slice(&byte_slice!("60 9C 8B 44 24 28 50"));
        // mov ebx, imm32
        buf.push(0xBB);
        buf.extend_from_slice(&replace_addr.to_le_bytes());
        // call ebx; popfd; popad; ret 0xC;
        buf.extend_from_slice(&byte_slice!("FF D3 9D 61 C2 0C 00"));

        write_asm(dll_addr.add(0x34A00), &buf).unwrap();
    }
}

#[ffi_catch_unwind]
pub unsafe extern "system" fn replace_script(ptr: *mut u8) {
    unsafe {
        if !crate::utils::quick_memory_check_win32(ptr, 0x18) {
            return;
        }

        // 读取 ptr + 0x14上的u32小端
        let len = std::ptr::read_unaligned(ptr.add(0x14) as *const u32) as usize;
        crate::patch::process_buffer(ptr, len);
    }
}
