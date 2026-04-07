use std::sync::LazyLock;
use translate_macros::DefaultHook;
use windows_sys::Win32::Foundation::HMODULE;

use crate::{
    constant::ARG_GAME_TYPE,
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
pub struct HitocosHook;

static mut TEXT_RETURN_ADDR: usize = 0;
static mut NAME_RETURN_ADDR: usize = 0;

impl ProcessAttach for HitocosHook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        fix_game_ini()?;

        let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;
        let module = handle as *mut u8;

        unsafe {
            TEXT_RETURN_ADDR = module.add(0x189B5) as usize;
            NAME_RETURN_ADDR = module.add(0x48555) as usize;
        };

        unsafe {
            match ARG_GAME_TYPE {
                "hitocos" => {
                    module
                        .add(0x189B0)
                        .write_jmp_instruction(text_trampoline as _)?;

                    module
                        .add(0x4854D)
                        .write_jmp_instruction(name_trampoline as _)?;
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

        mov eax, dword ptr [esp + 0xC];
        push ebx;

        jmp dword ptr [{1}];
        ",
        sym hook_text,
        sym TEXT_RETURN_ADDR,
    )
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn name_trampoline() {
    // mov [esp + 0x28], eax; 覆盖第一个参数
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov eax, [esp + 0x28];
        push eax;
        call {0};
        mov [esp + 0x28], eax;
        popfd;
        popad;

        push esi;
        push edi;
        mov edi, dword ptr [esp + 0xC];
        mov esi, ecx;

        jmp dword ptr [{1}];
        ",
        sym hook_text,
        sym NAME_RETURN_ADDR,
    )
}

static CACHE: LazyLock<Interner> = LazyLock::new(Interner::new);

#[translate_macros::ffi_guard(on_err_or_panic = ptr)]
unsafe extern "system" fn hook_text(ptr: *const u8) -> crate::Result<*const u8> {
    unsafe {
        let slice = ptr.to_slice_until_null_scan();
        let wide_text = invert(slice).to_wide_ansi();
        crate::debug!("Get raw slice {}", wide_text.to_string_lossy());

        if let Some(text) = wide_text.lookup_or_store()? {
            crate::debug!("Get translated slice {}", text.to_string_lossy());
            let text_b = CACHE.intern(invert(&text.to_ansi()).with_null());
            return Ok(text_b.as_ptr());
        }
        Ok(ptr)
    }
}

fn invert(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(|&b| !b).collect()
}

fn fix_game_ini() -> crate::Result<()> {
    let current_dir = crate::utils::win32::get_module_file_name(core::ptr::null_mut(), false)?
        .to_path_buf()
        .parent()
        .map(|dir| {
            let mut dir = dir.to_string_lossy().into_owned();
            if !dir.ends_with("\\") {
                dir.push('\\');
            }
            dir
        })
        .ok_or_else(|| crate::anyhow!("Failed to get executable_dir"))?;

    let buf = format!(
        "[PATH]\r\nSetupType=\"1\"\r\nCurrent=\"{}\"\r\nCDDrive=\".\\\"\r\n\r\n",
        current_dir
    );

    std::fs::write(
        "人妻コスプレ喫茶.ini",
        buf.as_bytes().to_wide_utf8().to_ansi(),
    )?;

    Ok(())
}
