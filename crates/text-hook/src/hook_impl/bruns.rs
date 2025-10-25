use translate_macros::{byte_slice, ffi_catch_unwind};
use windows_sys::Win32::Foundation::HMODULE;

use crate::hook::CoreHook;
use crate::hook::text_hook::TextHook;
use crate::hook::window_hook::WindowHook;
use crate::hook_utils::{write_asm, write_bytes};
use crate::{constant, debug};

#[derive(Default)]
pub struct BrunsHook;

impl CoreHook for BrunsHook {
    fn on_process_attach(&self, _hinst_dll: HMODULE) {
        let Some(handle) = crate::hook_utils::get_module_handle("libscr.dll") else {
            debug!("get_module_handle failed");
            return;
        };

        debug!("patch {handle:p}");

        patch_by_arg1(handle as *mut u8);
    }
}

fn patch_v1(module_addr: *mut u8) {
    // 改路径常量字符，让游戏找不到位图字体文件，并跳过错误报告;
    // 最终游戏FALLBACK到GDI文本渲染
    unsafe {
        // push libscr.DCECC
        let char_addr = module_addr as usize + 0xDCECC;
        let mut code_buf = vec![0x68];
        code_buf.extend_from_slice(&char_addr.to_le_bytes());
        write_asm(module_addr.add(0x1A29A), &code_buf).unwrap();

        // jmp libscr.sub_1A48C
        write_asm(module_addr.add(0x1A48C), &byte_slice!("EB 3A")).unwrap();

        // 00 00 -> 5F 00 (`/` -> `_`)
        write_bytes(module_addr.add(0xDCECC), &byte_slice!("5F 00")).unwrap();
    }

    // 将 codepage 固定为CP932
    unsafe {
        // (push ebp; push ebx; push 0x1; push 0x3A4; jmp MultibytesToWideChar;) * 2
        write_asm(module_addr.add(0xD6FC0), &byte_slice!("55 53 6A 01 68 A4 03 00 00 E9 08 85 F3 FF 55 53 6A 01 68 A4 03 00 00 E9 28 85 F3 FF")).unwrap();

        // jmp libscr.D6FC0;
        write_bytes(module_addr.add(0x0F4D0), &byte_slice!("E9 EB 7A 0C 00 90")).unwrap();

        // jmp libscr.D6FCE;
        write_bytes(module_addr.add(0x0F4FE), &byte_slice!("E9 CB 7A 0C 00 90")).unwrap();
    }

    unsafe {
        // jmp libscr.D6FE0;
        write_asm(module_addr.add(0x3F060), &byte_slice!("E9 7B 7F 09 00")).unwrap();

        // mov eax, memcpy2;
        let mut code_buf = vec![0xB8];
        code_buf.extend_from_slice(&(memcpy2 as usize).to_le_bytes());
        // call eax; jmp libscr.3F065;
        code_buf.extend_from_slice(&byte_slice!("FF D0 E9 79 80 F6 FF"));

        write_asm(module_addr.add(0xD6FE0), &code_buf).unwrap();
    }
}

fn patch_v2(module_addr: *mut u8) {
    unsafe {
        // push libscr.DEEC4
        let char_addr = module_addr as usize + 0xDEEC4;
        let mut code_buf = vec![0x68];
        code_buf.extend_from_slice(&char_addr.to_le_bytes());
        write_asm(module_addr.add(0x19EBA), &code_buf).unwrap();

        // jmp libscr.sub_1A0E8
        write_asm(module_addr.add(0x1A0AC), &byte_slice!("EB 3A")).unwrap();

        // 00 00 -> 5F 00 (`/` -> `_`)
        write_bytes(module_addr.add(0xDEEC4), &byte_slice!("5F 00")).unwrap();
    }

    unsafe {
        // (push ebp; push ebx; push 0x1; push 0x3A4; jmp MultibytesToWideChar;) * 2
        write_asm(module_addr.add(0xD8FC1), &byte_slice!("55 53 6A 01 68 A4 03 00 00 E9 A7 5F F3 FF 55 53 6A 01 68 A4 03 00 00 E9 C7 5F F3 FF")).unwrap();

        // jmp libscr.D8FC1;
        write_bytes(module_addr.add(0x0EF70), &byte_slice!("E9 4C A0 0C 00 90")).unwrap();

        // jmp libscr.D8FCF;
        write_bytes(module_addr.add(0x0EF9E), &byte_slice!("E9 2C A0 0C 00 90")).unwrap();
    }

    unsafe {
        // jmp libscr.D8FE1;
        write_asm(module_addr.add(0x3E3F0), &byte_slice!("E9 EC AB 09 00")).unwrap();

        // mov eax, memcpy2;
        let mut code_buf = vec![0xB8];
        code_buf.extend_from_slice(&(memcpy2 as usize).to_le_bytes());
        // call eax; jmp libscr.3E3F5;
        code_buf.extend_from_slice(&byte_slice!("FF D0 E9 08 54 F6 FF"));

        write_asm(module_addr.add(0xD8FE1), &code_buf).unwrap();
    }
}

fn patch_by_arg1(module_addr: *mut u8) {
    match constant::ARG1 {
        "v1" => patch_v1(module_addr),
        "v2" | "v3" => patch_v2(module_addr),
        _ => unreachable!(),
    }
}

impl TextHook for BrunsHook {}
impl WindowHook for BrunsHook {}

#[ffi_catch_unwind]
pub unsafe extern "C" fn memcpy2(dst: *mut u8, src: *mut u8, len: usize) {
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, len);
        crate::patch::process_buffer(dst, len);
    }
}
