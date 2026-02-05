use std::ops::Add;
use translate_macros::DefaultHook;
use windows_sys::Win32::Foundation::HMODULE;

use crate::constant::{ARG_CHARS_SIZE, ARG_PATCH_TYPE};
use crate::debug;
use crate::hook::traits::CoreHook;
use crate::utils::mem::patch::write_asm;

#[derive(DefaultHook)]
pub struct NatsuNatsuHook;

static mut CHARS_MEM_PTR: usize = 0;

impl CoreHook for NatsuNatsuHook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        unsafe {
            CHARS_MEM_PTR =
                Box::leak(Box::<[u8]>::new_zeroed_slice(ARG_CHARS_SIZE)).as_ptr() as usize
        };

        let Some(handle) = crate::utils::win32::get_module_handle("") else {
            debug!("get_module_handle failed");
            return;
        };

        let module_addr = handle as *mut u8;

        match ARG_PATCH_TYPE {
            "natsu_natsu" => patch_natsu_natsu(module_addr),
            "mozu" => patch_mozu(module_addr),
            _ => unreachable!(),
        }
    }
}

fn patch_natsu_natsu(module_addr: *mut u8) {
    unsafe {
        // lea eax, ds:[{CHARS_MEM_PTR}]
        let mut buf = vec![0x8D, 0x05];
        buf.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());
        write_asm(module_addr.add(0xC7FB), &buf).unwrap();

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR}]
        // nop
        let mut buf = vec![0x0F, 0xBE, 0x99];
        buf.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0xACDE), &buf).unwrap();

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x0F, 0xBE, 0x99];
        buf.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0xACF2), &buf).unwrap();

        // mov cl, byte ptr ds:[esi + {CHARS_MEM_PTR}]
        // nop
        let mut buf = vec![0x8A, 0x8E];
        buf.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0xBC47), &buf).unwrap();
        write_asm(module_addr.add(0x98E4), &buf).unwrap();

        // mov dl, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x96];
        buf.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0xBC52), &buf).unwrap();

        // mov cl, byte ptr ds:[eax + {CHARS_MEM_PTR}]
        // nop
        let mut buf = vec![0x8A, 0x88];
        buf.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0xD221), &buf).unwrap();
        write_asm(module_addr.add(0xE692), &buf).unwrap();
        write_asm(module_addr.add(0xE6EB), &buf).unwrap();

        // mov al, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x80];
        buf.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0xD22D), &buf).unwrap();

        // mov dl, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x90];
        buf.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0xE69D), &buf).unwrap();
        write_asm(module_addr.add(0xE6F6), &buf).unwrap();

        // mov al, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf = vec![0x8A, 0x86];
        buf.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf.push(0x90);
        write_asm(module_addr.add(0x98F4), &buf).unwrap();
    }
}

fn patch_mozu(module_addr: *mut u8) {
    unsafe {
        // lea eax, ds:[{CHARS_MEM_PTR}]
        let mut buf1 = vec![0x8D, 0x05];
        buf1.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR}]
        // nop
        let mut buf2 = vec![0x0F, 0xBE, 0x99];
        buf2.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());
        buf2.push(0x90);

        // movsx ebx, byte ptr ds:[ecx + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf3 = vec![0x0F, 0xBE, 0x99];
        buf3.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf3.push(0x90);

        // mov cl, byte ptr ds:[esi + {CHARS_MEM_PTR}]
        // nop
        let mut buf4 = vec![0x8A, 0x8E];
        buf4.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());
        buf4.push(0x90);

        // mov dl, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf5 = vec![0x8A, 0x96];
        buf5.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf5.push(0x90);

        // mov cl, byte ptr ds:[eax + {CHARS_MEM_PTR}]
        // nop
        let mut buf6 = vec![0x8A, 0x88];
        buf6.extend_from_slice(&CHARS_MEM_PTR.to_le_bytes());
        buf6.push(0x90);

        // mov al, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf7 = vec![0x8A, 0x80];
        buf7.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf7.push(0x90);

        // mov dl, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf8 = vec![0x8A, 0x90];
        buf8.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf8.push(0x90);

        // mov al, byte ptr ds:[esi + {CHARS_MEM_PTR} + 1]
        // nop
        let mut buf9 = vec![0x8A, 0x86];
        buf9.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
        buf9.push(0x90);

        write_asm(module_addr.add(0xA350), &buf4).unwrap();
        write_asm(module_addr.add(0xA360), &buf9).unwrap();

        write_asm(module_addr.add(0xB7FE), &buf2).unwrap();
        write_asm(module_addr.add(0xB812), &buf3).unwrap();

        write_asm(module_addr.add(0xC77C), &buf4).unwrap();
        write_asm(module_addr.add(0xC787), &buf5).unwrap();

        write_asm(module_addr.add(0xD37B), &buf1).unwrap();

        write_asm(module_addr.add(0xDE11), &buf6).unwrap();
        write_asm(module_addr.add(0xDE1D), &buf7).unwrap();

        write_asm(module_addr.add(0xF092), &buf6).unwrap();
        write_asm(module_addr.add(0xF09D), &buf8).unwrap();

        write_asm(module_addr.add(0xF0EB), &buf6).unwrap();
        write_asm(module_addr.add(0xF0F6), &buf8).unwrap();
    }
}
