use translate_macros::byte_slice;
use windows_sys::Win32::Foundation::HMODULE;

use crate::debug;
use crate::hook::traits::{CoreHook, TextHook};
use crate::utils::mem::patch::{create_trampoline_32, write_asm};

#[derive(Default)]
pub struct MizukakeHook;

impl CoreHook for MizukakeHook {
    fn on_process_attach(&self, _hinst_dll: HMODULE) {
        let Some(handle) = crate::utils::win32::get_module_handle("") else {
            debug!("get_module_handle failed");
            return;
        };

        debug!("patch {handle:p}");

        let module_addr = handle as *mut u8;

        unsafe {
            // jmp mizukake_chs.222F80;
            write_asm(module_addr.add(0x1A8F1C), &byte_slice!("E9 5F A0 07 00")).unwrap();

            let code_buf = create_trampoline_32(
                replace_script as _,
                // mov ecx,[esp+0x40]; mov edx,[esp+0x3C]; mov eax,[esp+0x28]; mov ebx,[esp+0x24];
                // push ecx; push edx; push eax; push ebx;
                &byte_slice!("8B 4C 24 40 8B 54 24 3C 8B 44 24 28 8B 5C 24 24 51 52 50 53"),
                // pop edi; pop esi; pop ebx; mov esp,ebp; pop ebp; ret 0xC
                &byte_slice!("5F 5E 5B 8B E5 5D C2 0C 00"),
            );

            write_asm(module_addr.add(0x222F80), &code_buf).unwrap();
        }
    }
}

impl TextHook for MizukakeHook {}

#[translate_macros::ffi_catch_unwind]
pub unsafe extern "system" fn replace_script(ptr: *mut u8, len: usize, ptr2: *mut u8, len2: usize) {
    if !crate::patch::process_buffer(ptr, len) {
        crate::patch::process_buffer(ptr2, len2);
    }
}
