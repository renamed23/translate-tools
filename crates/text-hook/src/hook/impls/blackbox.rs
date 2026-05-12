use core::sync::atomic::{AtomicPtr, Ordering};
use std::sync::LazyLock;

use translate_macros::DefaultHook;
use windows_sys::Win32::Foundation::HMODULE;

use crate::{
    hook::internal_hooks::ProcessAttach,
    utils::{
        exts::{
            ptr_ext::{PtrExt, PtrWriteExt},
            slice_ext::{ByteSliceExt, WideSliceExt},
        },
        interner::Interner,
    },
};

#[derive(DefaultHook)]
#[exclude(ProcessAttach)]
pub struct BlackboxHook;

static HOOK_RETURN_ADDR: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());

impl ProcessAttach for BlackboxHook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;
        let module = handle.cast::<u8>();

        unsafe {
            module.add(0x1DBD6).write_jmp_instruction(trampoline as _)?;
            HOOK_RETURN_ADDR.store(module.add(0x1DBDB), Ordering::Release);
        }
        Ok(())
    }
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline() {
    core::arch::naked_asm!(
        "
        lea edx, [esp + 0x08];
        
        pushad;
        pushfd;
        push edx;
        call {0};
        mov [esp + 0x18], eax;
        popfd;
        popad;
        
        push edx;
        
        jmp dword ptr [{1}];
        ",
        sym hook_text,
        sym HOOK_RETURN_ADDR
    )
}

static INTERNER: LazyLock<Interner> = LazyLock::new(Interner::new);

#[translate_macros::ffi_guard(on_err_or_panic = ptr)]
unsafe extern "system" fn hook_text(ptr: *const u8) -> crate::Result<*const u8> {
    unsafe {
        let slice = ptr.to_slice_until_null_scan();
        let wide_text = slice.try_to_wide(932)?;

        crate::debug!("Get raw slice {}", wide_text.to_string_lossy());
        if let Some(text) = wide_text.lookup_or_store()? {
            crate::debug!("Get translated slice {}", text.to_string_lossy());
            let translated_bytes = text.try_to_multi_byte_null(932)?;

            let text_ptr = INTERNER.intern(translated_bytes);
            return Ok(text_ptr.as_ptr());
        }
        Ok(ptr)
    }
}
