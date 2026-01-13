use std::ops::Add;

use windows_sys::Win32::Foundation::HMODULE;

use crate::constant::ARG1;
use crate::debug;
use crate::hook::traits::CoreHook;
use crate::utils::mem::patch::write_asm;

#[derive(Default)]
pub struct NatsuNatsuHook;

static mut CHARS_MEM_PTR: usize = 0;

impl CoreHook for NatsuNatsuHook {
    fn on_process_attach(&self, _hinst_dll: HMODULE) {
        unsafe {
            CHARS_MEM_PTR =
                Box::leak(Box::<[u8]>::new_zeroed_slice(ARG1.parse().unwrap())).as_ptr() as usize
        };

        let Some(handle) = crate::utils::win32::get_module_handle("") else {
            debug!("get_module_handle failed");
            return;
        };

        let module_addr = handle as *mut u8;

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
            write_asm(module_addr.add(0x98F4), &buf).unwrap();

            // mov dl, byte ptr ds:[eax + {CHARS_MEM_PTR} + 1]
            // nop
            let mut buf = vec![0x8A, 0x90];
            buf.extend_from_slice(&CHARS_MEM_PTR.add(1).to_le_bytes());
            buf.push(0x90);
            write_asm(module_addr.add(0xE69D), &buf).unwrap();
            write_asm(module_addr.add(0xE6F6), &buf).unwrap();
        }
    }
}

translate_macros::expand_by_files!("src/hook/traits" => {
    #[cfg(feature = __file_str__)]
    impl crate::hook::traits::__file_pascal__ for NatsuNatsuHook {}
});
