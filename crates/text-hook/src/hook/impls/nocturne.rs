use std::cell::Cell;

use translate_macros::{DefaultHook, ffi_guard};
use windows_sys::Win32::Foundation::HMODULE;

use crate::{hook::internal_hooks::ProcessAttach, utils::exts::ptr_ext::PtrWriteExt};

translate_macros::expand_by_files!("assets/translated_patch" => {
        __repeat__(
            translate_macros::embed!{const __file_stem_ident__: [u8] from __concat__("assets/translated_patch/", __file_str__)}
        );

        static PHF_MAP: phf::Map<usize, &'static [u8]> = phf::phf_map! {
            __repeat__(__file_json_value__ => __file_stem_ident__ ,)
        };
    },
    mode = "plain",
    json = "assets/misc/nocturne.json"
);

#[derive(DefaultHook)]
#[exclude(ProcessAttach)]
pub struct NocturneHook;

#[repr(C)]
struct GlobalBuffer {
    buf: *mut u8,
    len: u32,
}

static mut HOOK_RETURN_ADDR: usize = 0;
static mut SET_CURRENT_SURFACE_RETURN_ADDR: usize = 0;
static mut G_BUFFER: *mut GlobalBuffer = core::ptr::null_mut();

impl ProcessAttach for NocturneHook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;
        let module = handle.cast::<u8>();

        unsafe {
            G_BUFFER = module.add(0xA13DC).cast();

            module
                .add(0x6C30)
                .write_jmp_instruction(trampoline_hook as _)?;
            HOOK_RETURN_ADDR = module.add(0x6C37) as usize;

            module
                .add(0x2900)
                .write_jmp_instruction(trampoline_set_current_surface as _)?;
            SET_CURRENT_SURFACE_RETURN_ADDR = module.add(0x2906) as usize;

            module
                .add(0x2B2C)
                .patch_repeated_asm(0x90, 26)?
                .write_call_instruction(trampoline_fix_buffer_write as _)?;
            module
                .add(0x2BA1)
                .patch_repeated_asm(0x90, 26)?
                .write_call_instruction(trampoline_fix_buffer_write as _)?;

            module
                .add(0x22D82)
                .write_jmp_instruction(fix_wrong_v12 as _)?;
            FIX_WRONG_V12_RETURN_ADDR = module.add(0x22D88) as usize;
        }
        Ok(())
    }
}

static mut FIX_WRONG_V12_RETURN_ADDR: usize = 0;

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn fix_wrong_v12() {
    core::arch::naked_asm!(
        "
        mov eax, dword ptr ss:[ebp-0x15C];
        sub eax, 0x1;

        jmp dword ptr [{0}];
        ",
        sym FIX_WRONG_V12_RETURN_ADDR,
    )
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline_hook() {
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov eax, dword ptr [esi + 0x4];
        push eax;
        call {0};
        popfd;
        popad;

        mov dword ptr [ebp - 0x4], 0x1;

        jmp dword ptr [{1}];
        ",
        sym hook_script,
        sym HOOK_RETURN_ADDR,
    )
}

#[ffi_guard(on_err_or_panic = ())]
unsafe extern "system" fn hook_script(offset: usize) {
    crate::debug!("offset: {offset}");

    unsafe {
        if let Some(patch) = PHF_MAP.get(&offset) {
            crate::debug!("Patch found");
            let g_buffer = &mut *G_BUFFER;
            g_buffer.len = patch.len() as u32;
            g_buffer.buf.copy_bytes_from(patch);
        }
    }
}

#[repr(C)]
struct SurfaceBuffer {
    unknown: u32,
    width: u32,
    height: u32,
    gap_0c: [u8; 28],
    pixel_data: [u8; 0],
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline_set_current_surface() {
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov eax, [esp + 0x2C]
        push eax;
        call {0};
        popfd;
        popad;

        push ebp;
        mov ebp, esp;
        add esp, 0xFFFFFFD8;

        jmp dword ptr [{1}];
        ",
        sym set_current_surface,
        sym SET_CURRENT_SURFACE_RETURN_ADDR,
    )
}

thread_local! {
    static CURRENT_SURFACE: Cell<*mut SurfaceBuffer> = const { Cell::new(core::ptr::null_mut()) };
}

#[ffi_guard(on_err_or_panic = ())]
unsafe extern "system" fn set_current_surface(surface: *mut SurfaceBuffer) {
    CURRENT_SURFACE.set(surface);
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline_fix_buffer_write() {
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        push ebx;
        push edx;
        call {0};
        popfd;
        popad;
        ret;
        ",
        sym fix_buffer_write,
    )
}

#[ffi_guard(on_err_or_panic = ())]
unsafe extern "system" fn fix_buffer_write(
    target_ptr: *mut u8,
    source_ptr: *const u8,
) -> crate::Result<()> {
    let surface = CURRENT_SURFACE.get();
    if surface.is_null() {
        crate::bail!("Surface ptr is null!");
    }

    let sur = unsafe { &mut *surface };

    let start = sur.pixel_data.as_ptr();
    let size = (sur.width as usize) * (sur.height as usize) * 3;
    let end = unsafe { start.add(size) };

    if target_ptr >= start.cast_mut() && unsafe { target_ptr.add(2) } < end.cast_mut() {
        unsafe {
            target_ptr.add(0).write(source_ptr.add(0xC4).read());
            target_ptr.add(1).write(source_ptr.add(0xC3).read());
            target_ptr.add(2).write(source_ptr.add(0xC2).read());
        }
    } else {
        crate::debug!(
            "Blocked OOB Write! Target ptr: {target_ptr:?}, Source ptr: {source_ptr:?}, Surface \
             ptr: {surface:?}, Buffer Range: {start:?}..{end:?}, Surface: {}x{}",
            sur.width,
            sur.height
        );
    }

    Ok(())
}
