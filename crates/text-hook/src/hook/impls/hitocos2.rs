use std::{
    collections::HashMap,
    ffi::{c_char, c_int, c_uint},
    sync::{LazyLock, Mutex},
};

use translate_macros::DefaultHook;
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

impl ProcessAttach for Hitocos2Hook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;
        let module = handle as *mut u8;

        unsafe {
            TEXT_RETURN_ADDR = module.add(0xE495) as usize;
            SUB_402B70 = module.add(0x2B70) as usize;
        };

        unsafe {
            match ARG_GAME_TYPE {
                "hitocos2" => {
                    module
                        .add(0xF686)
                        .write_jmp_instruction(name_trampoline as _)?;

                    module
                        .add(0xE490)
                        .write_jmp_instruction(text_trampoline as _)?;
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

type Cache = LazyLock<Mutex<HashMap<Box<[u8]>, &'static [u8]>>>;
static NAME_CACHE: Cache = LazyLock::new(|| Mutex::new(HashMap::new()));
static TEXT_CACHE: Cache = LazyLock::new(|| Mutex::new(HashMap::new()));

fn intern_bytes(cache: &Cache, bytes: Vec<u8>) -> &'static [u8] {
    if let Some(&cached) = cache.lock().unwrap().get(bytes.as_slice()) {
        return cached;
    }

    let leaked = Box::leak(bytes.into_boxed_slice()) as &'static mut [u8];
    let leaked = leaked as &'static [u8];
    cache.lock().unwrap().insert(leaked.into(), leaked);
    leaked
}

#[translate_macros::ffi_guard(on_err_or_panic = ())]
unsafe extern "system" fn hook_name(string_ptr: *mut MsvcString) -> crate::Result<()> {
    unsafe {
        if string_ptr.is_null() {
            crate::bail!("Hook name but string is null");
        }
        let string = { &*string_ptr };

        let name_ptr = if string.cap < 16 {
            // SSO模式：数据在 buf 字段
            string.buf.as_ptr()
        } else {
            // 堆分配模式：buf 字段存储的是指针
            *(string.buf.as_ptr() as *const *const u8)
        };

        let slice = name_ptr.to_slice_until_null_scan();
        let sub_402b70: Sub402B70 = core::mem::transmute(SUB_402B70);

        let wide_name = slice.to_wide_ansi();
        crate::debug!("Get raw slice {}", wide_name.to_string_lossy());
        if let Some(name) = wide_name.lookup_or_store()? {
            crate::debug!("Get translated slice {}", name.to_string_lossy());
            let name_b = intern_bytes(&NAME_CACHE, name.to_ansi_null());
            sub_402b70(string_ptr, 0, name_b.as_ptr() as _, name_b.len() as _);
        }

        Ok(())
    }
}

#[translate_macros::ffi_guard(on_err_or_panic = ptr)]
unsafe extern "system" fn hook_text(ptr: *const u8) -> crate::Result<*const u8> {
    unsafe {
        let slice = ptr.to_slice_until_null_scan();
        let wide_text = invert(slice).to_wide_ansi();
        crate::debug!("Get raw slice {}", wide_text.to_string_lossy());
        if let Some(text) = wide_text.lookup_or_store()? {
            crate::debug!("Get translated slice {}", text.to_string_lossy());
            let text_b = intern_bytes(&TEXT_CACHE, invert(&text.to_ansi()).with_null());
            return Ok(text_b.as_ptr());
        }
        Ok(ptr)
    }
}
