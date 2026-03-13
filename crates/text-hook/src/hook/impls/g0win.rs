use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use translate_macros::{DefaultHook, byte_slice};
use windows_sys::Win32::Foundation::HMODULE;

use crate::{
    hook::traits::CoreHook,
    utils::exts::{
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[derive(DefaultHook)]
pub struct G0WinHook;

impl CoreHook for G0WinHook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null()).unwrap();
        let module = handle as *mut u8;

        unsafe {
            crate::utils::mem::patch::write_asm(module.add(0x2A78C), &byte_slice!("EB 14 90"))
                .unwrap();
            crate::utils::mem::patch::write_jmp_instruction(module.add(0x2A7A2), trampoline as _)
                .unwrap();
        }
    }
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline() {
    // `mov [esp + 32], eax;`
    // 让 hook_text 返回的 eax 在`popad`之后
    // 保留在 eax 之中
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        push edx;
        call {0};
        mov [esp + 32], eax;
        popfd;
        popad;
        pop edi;
        ret;
        ",
        sym hook_text
    )
}

static CACHE: LazyLock<Mutex<HashMap<Box<[u8]>, &'static [u8]>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[translate_macros::ffi_catch_unwind(ptr)]
unsafe extern "system" fn hook_text(ptr: *const u8) -> *const u8 {
    unsafe {
        let slice = ptr.to_slice_until_null(8192 * 50);
        if let Some(&text) = CACHE.lock().unwrap().get(slice) {
            crate::debug!("Get cached slice {}", text.to_wide_ansi().to_string_lossy());
            return text.as_ptr();
        }

        let wide_text = slice.to_wide(932);
        crate::debug!("Get raw slice {}", wide_text.to_string_lossy());
        if let Ok(text) = wide_text.lookup_or_add_item() {
            crate::debug!("Get translated slice {}", text.to_string_lossy());
            let text_ptr = Box::leak(text.to_multi_byte_null(936).into_boxed_slice());
            CACHE.lock().unwrap().insert(slice.into(), text_ptr);
            return text_ptr.as_ptr();
        }
        ptr
    }
}
