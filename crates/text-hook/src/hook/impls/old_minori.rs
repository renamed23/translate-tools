use translate_macros::{DefaultHook, ffi_guard};
use windows_sys::Win32::Foundation::HMODULE;

use crate::{
    constant::ARG_GAME_TYPE,
    hook::internal_hooks::ProcessAttach,
    utils::exts::{
        ptr_ext::{PtrExt, PtrWriteExt},
        slice_ext::ByteSliceExt,
    },
};

#[derive(DefaultHook)]
#[exclude(ProcessAttach)]
pub struct OldMinoriHook;

static mut HOOK_RETURN_ADDR: usize = 0;

impl ProcessAttach for OldMinoriHook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;
        let module = handle.cast::<u8>();

        unsafe {
            match ARG_GAME_TYPE {
                "haruotoFD" => {
                    module.add(0x785B2).write_jmp_instruction(trampoline as _)?;
                    HOOK_RETURN_ADDR = module.add(0x785B7) as usize;
                }

                _ => {
                    unreachable!()
                }
            }
        }
        Ok(())
    }
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline() {
    core::arch::naked_asm!(
        "
        call dword ptr [edx + 0x14];

        pushad;
        pushfd;
        mov eax, dword ptr [esp + 0xC];
        mov ebx, dword ptr [esp + 0x60];
        push eax;
        push ebx;
        call {0};
        popfd;
        popad;

        cmp eax, ebp;

        jmp dword ptr [{1}];
        ",
        sym hook_script,
        sym HOOK_RETURN_ADDR,
    )
}

#[ffi_guard(on_err_or_panic = ())]
pub unsafe extern "system" fn hook_script(ptr: *mut u8, len: usize) {
    crate::debug!("ptr: 0x{:X}, len: 0x{len:X}", ptr as usize);

    unsafe {
        if let Ok(Some(patch)) = ptr.to_slice_mut(len).get_patch_or_extract() {
            ptr.copy_bytes_from(patch);
        }
    }
}
