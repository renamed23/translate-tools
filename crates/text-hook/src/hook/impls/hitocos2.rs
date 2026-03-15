use std::{
    collections::HashMap,
    ffi::{c_char, c_int, c_uint},
    sync::{LazyLock, Mutex},
};

use translate_macros::DefaultHook;
use windows_sys::Win32::Foundation::HMODULE;

use crate::{
    constant::ARG_GAME_TYPE,
    hook::traits::CoreHook,
    utils::exts::{
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, WideSliceExt},
    },
};

#[derive(DefaultHook)]
pub struct Hitocos2Hook;

#[repr(C)]
struct MsvcString {
    unknown: u32,
    buf: [u8; 16],
    size: u32,
    cap: u32,
}

#[allow(unsupported_calling_conventions)]
type Sub402B70 = extern "fastcall" fn(
    string: *mut MsvcString,
    unused: c_int,      // 通常为0
    src: *const c_char, // 源字符串
    len: c_uint,        // 字符串长度
) -> *mut MsvcString;

static mut TEXT_RETURN_ADDR: usize = 0;
static mut SUB_402B70: usize = 0;

impl CoreHook for Hitocos2Hook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null()).unwrap();
        let module = handle as *mut u8;

        unsafe {
            TEXT_RETURN_ADDR = module.add(0xE495) as usize;
            SUB_402B70 = module.add(0x2B70) as usize;
        };

        unsafe {
            match ARG_GAME_TYPE {
                "hitocos2" => {
                    crate::utils::mem::patch::write_jmp_instruction(
                        module.add(0xF686),
                        name_trampoline as _,
                    )
                    .unwrap();

                    crate::utils::mem::patch::write_jmp_instruction(
                        module.add(0xE490),
                        text_trampoline as _,
                    )
                    .unwrap();
                }

                _ => {
                    unreachable!()
                }
            }
        }
    }
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn name_trampoline() {
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov eax, [esp + 0x28];
        push eax;
        call {0};
        popfd;
        popad;
        ret 0x4;
        ",
        sym hook_name,
    )
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn text_trampoline() {
    // mov [esp + 0x2C], eax; 覆盖第二个参数
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov eax, [esp + 0x2C];
        push eax;
        call {0};
        mov [esp + 0x2C], eax;
        popfd;
        popad;

        sub esp, 0x8;
        push ebx;
        push ebp;

        jmp dword ptr [{1}];
        ",
        sym hook_text,
        sym TEXT_RETURN_ADDR,
    )
}

fn invert(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(|&b| !b).collect()
}

#[allow(clippy::type_complexity)]
static CACHE: LazyLock<Mutex<HashMap<Box<[u8]>, &'static [u8]>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[translate_macros::ffi_catch_unwind]
unsafe extern "system" fn hook_name(string_ptr: *mut MsvcString) {
    unsafe {
        if string_ptr.is_null() {
            crate::debug!("Hook name but string is null");
            return;
        }
        let string = { &*string_ptr };

        let name_ptr = if string.cap < 16 {
            // SSO模式：数据在 buf 字段
            string.buf.as_ptr()
        } else {
            // 堆分配模式：buf 字段存储的是指针
            *(string.buf.as_ptr() as *const *const u8)
        };

        let slice = name_ptr.to_slice_until_null(8192 * 50);
        let sub_402b70: Sub402B70 = core::mem::transmute(SUB_402B70);

        // 注意，因为已经是解码过的字符串，所以不要invert
        if let Some(&text) = CACHE.lock().unwrap().get(slice) {
            crate::debug!(
                "Get cached slice {}",
                invert(&text[0..text.len() - 1])
                    .to_wide_ansi()
                    .to_string_lossy()
            );
            sub_402b70(string_ptr, 0, text.as_ptr() as _, text.len() as _);
            return;
        }

        let wide_name = slice.to_wide_ansi();
        crate::debug!("Get raw slice {}", wide_name.to_string_lossy());
        if let Ok(name) = wide_name.lookup_or_add_item() {
            crate::debug!("Get translated slice {}", name.to_string_lossy());
            let name_ptr = Box::leak(name.to_ansi_null().into_boxed_slice());
            CACHE.lock().unwrap().insert(slice.into(), name_ptr);
            sub_402b70(string_ptr, 0, name_ptr.as_ptr() as _, name_ptr.len() as _);
        }
    }
}

#[translate_macros::ffi_catch_unwind(ptr)]
unsafe extern "system" fn hook_text(ptr: *const u8) -> *const u8 {
    unsafe {
        let slice = ptr.to_slice_until_null(8192 * 50);
        if let Some(&text) = CACHE.lock().unwrap().get(slice) {
            crate::debug!(
                "Get cached slice {}",
                invert(&text[0..text.len() - 1])
                    .to_wide_ansi()
                    .to_string_lossy()
            );
            return text.as_ptr();
        }

        let wide_text = invert(slice).to_wide_ansi();
        crate::debug!("Get raw slice {}", wide_text.to_string_lossy());
        if let Ok(text) = wide_text.lookup_or_add_item() {
            crate::debug!("Get translated slice {}", text.to_string_lossy());
            let text_ptr = Box::leak(invert(&text.to_ansi()).with_null().into_boxed_slice());
            CACHE.lock().unwrap().insert(slice.into(), text_ptr);
            return text_ptr.as_ptr();
        }
        ptr
    }
}
