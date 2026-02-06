use translate_macros::DefaultHook;
use windows_sys::Win32::Foundation::HMODULE;

use crate::debug;
use crate::hook::traits::CoreHook;

#[derive(DefaultHook)]
pub struct MizukakeHook;

impl CoreHook for MizukakeHook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        let handle = crate::utils::win32::get_module_handle("").unwrap();

        debug!("patch {handle:p}");

        let module_addr = handle as *mut u8;

        unsafe {
            crate::utils::mem::patch::write_jmp_instruction(
                module_addr.add(0x1A8F1C),
                trampoline as _,
            )
            .unwrap();
        }
    }
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline() {
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov ecx,[esp+0x40];
        mov edx,[esp+0x3C]; 
        mov eax,[esp+0x28];
        mov ebx,[esp+0x24];
        push ecx;
        push edx;
        push eax;
        push ebx;
        call {0};
        popfd;
        popad;
        pop edi; 
        pop esi; 
        pop ebx; 
        mov esp,ebp; 
        pop ebp; 
        ret 0xC
        ",
        sym replace_script,
    );
}

#[translate_macros::ffi_catch_unwind]
pub unsafe extern "system" fn replace_script(ptr: *mut u8, len: usize, ptr2: *mut u8, len2: usize) {
    if !crate::patch::process_buffer(ptr, len) {
        crate::patch::process_buffer(ptr2, len2);
    }
}
