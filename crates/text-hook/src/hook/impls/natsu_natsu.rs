use std::ops::Add;
use translate_macros::DefaultHook;
use windows_sys::Win32::Foundation::HMODULE;

use crate::{constant::ARG_PATCH_TYPE, hook::traits::CoreHook, utils::exts::ptr_ext::PtrWriteExt};

#[derive(DefaultHook)]
pub struct NatsuNatsuHook;

translate_macros::embed!(const CHARS: [u8] from "assets/misc/System002");

impl CoreHook for NatsuNatsuHook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;

        let module_addr = handle as *mut u8;

        match ARG_PATCH_TYPE {
            "natsu_natsu" => patch_natsu_natsu(module_addr),
            "mozu" => patch_mozu(module_addr),
            _ => unreachable!(),
        }
    }
}

fn patch_natsu_natsu(module_addr: *mut u8) -> crate::Result<()> {
    unsafe {
        let chars_mem_ptr = CHARS.as_ptr() as u32;

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR}]
        // nop
        let mut buf = vec![0x0F, 0xBE, 0x99];
        buf.extend_from_slice(&chars_mem_ptr.to_le_bytes());
        buf.push(0x90);
        module_addr.add(0xACDE).patch_asm(&buf)?;

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x0F, 0xBE, 0x99];
        buf.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf.push(0x90);
        module_addr.add(0xACF2).patch_asm(&buf)?;

        // mov cl, byte ptr ds:[esi + {CHARS_MEM_PTR}]
        // nop
        let mut buf = vec![0x8A, 0x8E];
        buf.extend_from_slice(&chars_mem_ptr.to_le_bytes());
        buf.push(0x90);
        module_addr.add(0xBC47).patch_asm(&buf)?;
        module_addr.add(0x98E4).patch_asm(&buf)?;

        // mov dl, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x96];
        buf.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf.push(0x90);
        module_addr.add(0xBC52).patch_asm(&buf)?;

        // mov cl, byte ptr ds:[eax + {CHARS_MEM_PTR}]
        // nop
        let mut buf = vec![0x8A, 0x88];
        buf.extend_from_slice(&chars_mem_ptr.to_le_bytes());
        buf.push(0x90);
        module_addr.add(0xD221).patch_asm(&buf)?;
        module_addr.add(0xE692).patch_asm(&buf)?;
        module_addr.add(0xE6EB).patch_asm(&buf)?;

        // mov al, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x80];
        buf.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf.push(0x90);
        module_addr.add(0xD22D).patch_asm(&buf)?;

        // mov dl, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x90];
        buf.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf.push(0x90);
        module_addr.add(0xE69D).patch_asm(&buf)?;
        module_addr.add(0xE6F6).patch_asm(&buf)?;

        // mov al, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x86];
        buf.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf.push(0x90);
        module_addr.add(0x98F4).patch_asm(&buf)?;
    }
    Ok(())
}

fn patch_mozu(module_addr: *mut u8) -> crate::Result<()> {
    unsafe {
        let chars_mem_ptr = CHARS.as_ptr() as u32;

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR}]
        // nop
        let mut buf2 = vec![0x0F, 0xBE, 0x99];
        buf2.extend_from_slice(&chars_mem_ptr.to_le_bytes());
        buf2.push(0x90);

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf3 = vec![0x0F, 0xBE, 0x99];
        buf3.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf3.push(0x90);

        // mov cl, byte ptr ds:[esi + {CHARS_MEM_PTR}]
        // nop
        let mut buf4 = vec![0x8A, 0x8E];
        buf4.extend_from_slice(&chars_mem_ptr.to_le_bytes());
        buf4.push(0x90);

        // mov dl, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf5 = vec![0x8A, 0x96];
        buf5.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf5.push(0x90);

        // mov cl, byte ptr ds:[eax + {CHARS_MEM_PTR}]
        // nop
        let mut buf6 = vec![0x8A, 0x88];
        buf6.extend_from_slice(&chars_mem_ptr.to_le_bytes());
        buf6.push(0x90);

        // mov al, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf7 = vec![0x8A, 0x80];
        buf7.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf7.push(0x90);

        // mov dl, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf8 = vec![0x8A, 0x90];
        buf8.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf8.push(0x90);

        // mov al, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf9 = vec![0x8A, 0x86];
        buf9.extend_from_slice(&chars_mem_ptr.add(1).to_le_bytes());
        buf9.push(0x90);

        module_addr.add(0xA350).patch_asm(&buf4)?;
        module_addr.add(0xA360).patch_asm(&buf9)?;

        module_addr.add(0xB7FE).patch_asm(&buf2)?;
        module_addr.add(0xB812).patch_asm(&buf3)?;

        module_addr.add(0xC77C).patch_asm(&buf4)?;
        module_addr.add(0xC787).patch_asm(&buf5)?;

        module_addr.add(0xDE11).patch_asm(&buf6)?;
        module_addr.add(0xDE1D).patch_asm(&buf7)?;

        module_addr.add(0xF092).patch_asm(&buf6)?;
        module_addr.add(0xF09D).patch_asm(&buf8)?;

        module_addr.add(0xF0EB).patch_asm(&buf6)?;
        module_addr.add(0xF0F6).patch_asm(&buf8)?;
    }
    Ok(())
}
