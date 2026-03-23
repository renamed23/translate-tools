use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use translate_macros::{DefaultHook, byte_slice};
use windows_sys::Win32::Foundation::HMODULE;

use crate::{
    constant::ARG_GAME_TYPE,
    hook::internal_hooks::ProcessAttach,
    utils::exts::{
        ptr_ext::{PtrExt, PtrWriteExt},
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[derive(DefaultHook)]
#[exclude(ProcessAttach)]
pub struct G0WinHook;

impl ProcessAttach for G0WinHook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;
        let module = handle as *mut u8;

        unsafe {
            match ARG_GAME_TYPE {
                "うたかな" => {
                    module.add(0x2A78C).patch_asm(&byte_slice!("EB 14 90"))?;
                    module.add(0x2A7A2).write_jmp_instruction(trampoline as _)?;
                }

                "天巫女姫" => {
                    module.add(0x2C18C).patch_asm(&byte_slice!("EB 14 90"))?;
                    module.add(0x2C1A2).write_jmp_instruction(trampoline as _)?;
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

#[allow(clippy::type_complexity)]
static TEXT_CACHE: LazyLock<Mutex<HashMap<Box<[u8]>, &'static [u8]>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn intern_bytes(bytes: Vec<u8>) -> &'static [u8] {
    if let Some(&cached) = TEXT_CACHE.lock().unwrap().get(bytes.as_slice()) {
        return cached;
    }

    let leaked = Box::leak(bytes.into_boxed_slice()) as &'static mut [u8];
    let leaked = leaked as &'static [u8];
    TEXT_CACHE.lock().unwrap().insert(leaked.into(), leaked);
    leaked
}

#[translate_macros::ffi_guard(on_err_or_panic = ptr)]
unsafe extern "system" fn hook_text(ptr: *const u8) -> crate::Result<*const u8> {
    unsafe {
        let slice = ptr.to_slice_until_null_scan();
        let wide_text = slice.try_to_wide(932)?;

        crate::debug!("Get raw slice {}", wide_text.to_string_lossy());
        if let Some(text) = wide_text.lookup_or_store()? {
            crate::debug!("Get translated slice {}", text.to_string_lossy());
            let translated_bytes = text.try_to_multi_byte_null(936)?;

            let text_ptr = intern_bytes(translated_bytes);
            return Ok(text_ptr.as_ptr());
        }
        Ok(ptr)
    }
}
