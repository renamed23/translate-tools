use translate_macros::{DefaultHook, byte_slice, ffi_catch_unwind};
use windows_sys::Win32::Foundation::HMODULE;

use crate::hook::traits::CoreHook;
use crate::utils::mem::patch::{generate_trampoline_stub_32, write_asm, write_bytes};
use crate::{constant, debug};

#[derive(DefaultHook)]
pub struct BrunsHook;

impl CoreHook for BrunsHook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        patch_by_arg_game_type();
    }
}

fn patch_v1() {
    let handle = crate::utils::win32::get_module_handle("libscr.dll").unwrap();
    let module_addr = handle as *mut u8;

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

fn patch_v2() {
    let handle = crate::utils::win32::get_module_handle("libscr.dll").unwrap();
    let module_addr = handle as *mut u8;

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

fn patch_nerbor() {
    let handle = crate::utils::win32::get_module_handle("libscr.dll").unwrap();
    let module_addr = handle as *mut u8;

    unsafe {
        // push exe.134368
        let char_addr = module_addr as usize + 0x134368;
        let mut code_buf = vec![0x68];
        code_buf.extend_from_slice(&char_addr.to_le_bytes());
        write_asm(module_addr.add(0x70D23), &code_buf).unwrap();

        // jmp exe.sub_71080
        write_asm(module_addr.add(0x7104A), &byte_slice!("EB 34")).unwrap();

        // 00 00 -> 5F 00 (`/` -> `_`)
        write_bytes(module_addr.add(0x134368), &byte_slice!("5F 00")).unwrap();
    }

    unsafe {
        // (push ebp; push ebx; push 0x1; push 0x3A4; jmp MultibytesToWideChar;) * 2
        write_asm(module_addr.add(0x132FC1), &byte_slice!("55 53 6A 01 68 A4 03 00 00 E9 77 F9 EE FF 55 53 6A 01 68 A4 03 00 00 E9 97 F9 EE FF")).unwrap();

        // jmp exe.132FC1;
        write_bytes(module_addr.add(0x22940), &byte_slice!("E9 7C 06 11 00 90")).unwrap();

        // jmp exe.132FCF;
        write_bytes(module_addr.add(0x2296E), &byte_slice!("E9 5C 06 11 00 90")).unwrap();
    }

    unsafe {
        // jmp exe.132F90;
        write_asm(module_addr.add(0x11FD1B), &byte_slice!("E9 70 32 01 00")).unwrap();

        let code_buf = generate_trampoline_stub_32(
            crate::patch::process_buffer_ffi as _,
            // mov eax,[esp+0x48]; movebx,[esp+0x70]; push eax; push ebx;
            &byte_slice!("8B 44 24 48 8B 5C 24 70 50 53"),
            // mov esi,eax; add esp,8; jmp 11FD20;
            &byte_slice!("8B F0 83 C4 08 E9 71 CD FE FF"),
        );

        write_asm(module_addr.add(0x132F90), &code_buf).unwrap();
    }
}

fn patch_by_arg_game_type() {
    match constant::ARG_GAME_TYPE {
        "v1" => patch_v1(),
        "v2" | "v3" => patch_v2(),
        "隣人" => patch_nerbor(),
        _ => unreachable!(),
    }
}

#[ffi_catch_unwind]
pub unsafe extern "C" fn memcpy2(dst: *mut u8, src: *mut u8, len: usize) {
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, len);
        crate::patch::process_buffer(dst, len);
    }
}
