use translate_macros::{DefaultHook, byte_slice, ffi_catch_unwind};
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::w;

use crate::constant;
use crate::hook::traits::CoreHook;
use crate::utils::{exts::ptr_ext::PtrWriteExt, mem::patch::generate_trampoline_stub_32};

#[derive(DefaultHook)]
pub struct BrunsHook;

impl CoreHook for BrunsHook {
    fn on_process_attach(_hinst_dll: HMODULE) -> crate::Result<()> {
        patch_by_arg_game_type()
    }
}

fn patch_v1() -> crate::Result<()> {
    let handle = crate::utils::win32::get_module_handle(w!("libscr.dll"))?;
    let module_addr = handle as *mut u8;

    // 改路径常量字符，让游戏找不到位图字体文件，并跳过错误报告;
    // 最终游戏FALLBACK到GDI文本渲染
    unsafe {
        // push libscr.DCECC
        let char_addr = module_addr as usize + 0xDCECC;
        let mut code_buf = vec![0x68];
        code_buf.extend_from_slice(&char_addr.to_le_bytes());
        module_addr.add(0x1A29A).patch_asm(&code_buf)?;

        // jmp libscr.sub_1A48C
        module_addr.add(0x1A48C).patch_asm(&byte_slice!("EB 3A"))?;

        // 00 00 -> 5F 00 (`/` -> `_`)
        module_addr
            .add(0xDCECC)
            .patch_bytes(&byte_slice!("5F 00"))?;
    }

    // 将 codepage 固定为CP932
    unsafe {
        // (push ebp; push ebx; push 0x1; push 0x3A4; jmp MultibytesToWideChar;) * 2
        module_addr.add(0xD6FC0).patch_asm(&byte_slice!(
            "55 53 6A 01 68 A4 03 00 00 E9 08 85 F3 FF 55 53 6A 01 68 A4 03 00 00 E9 28 85 F3 FF"
        ))?;

        // jmp libscr.D6FC0;
        module_addr
            .add(0x0F4D0)
            .patch_bytes(&byte_slice!("E9 EB 7A 0C 00 90"))?;

        // jmp libscr.D6FCE;
        module_addr
            .add(0x0F4FE)
            .patch_bytes(&byte_slice!("E9 CB 7A 0C 00 90"))?;
    }

    unsafe {
        // jmp libscr.D6FE0;
        module_addr
            .add(0x3F060)
            .patch_asm(&byte_slice!("E9 7B 7F 09 00"))?;

        // mov eax, memcpy2;
        let mut code_buf = vec![0xB8];
        code_buf.extend_from_slice(&(memcpy2 as usize).to_le_bytes());
        // call eax; jmp libscr.3F065;
        code_buf.extend_from_slice(&byte_slice!("FF D0 E9 79 80 F6 FF"));

        module_addr.add(0xD6FE0).patch_asm(&code_buf)?;
    }
    Ok(())
}

fn patch_v2() -> crate::Result<()> {
    let handle = crate::utils::win32::get_module_handle(w!("libscr.dll"))?;
    let module_addr = handle as *mut u8;

    unsafe {
        // push libscr.DEEC4
        let char_addr = module_addr as usize + 0xDEEC4;
        let mut code_buf = vec![0x68];
        code_buf.extend_from_slice(&char_addr.to_le_bytes());
        module_addr.add(0x19EBA).patch_asm(&code_buf)?;

        // jmp libscr.sub_1A0E8
        module_addr.add(0x1A0AC).patch_asm(&byte_slice!("EB 3A"))?;

        // 00 00 -> 5F 00 (`/` -> `_`)
        module_addr
            .add(0xDEEC4)
            .patch_bytes(&byte_slice!("5F 00"))?;
    }

    unsafe {
        // (push ebp; push ebx; push 0x1; push 0x3A4; jmp MultibytesToWideChar;) * 2
        module_addr.add(0xD8FC1).patch_asm(&byte_slice!(
            "55 53 6A 01 68 A4 03 00 00 E9 A7 5F F3 FF 55 53 6A 01 68 A4 03 00 00 E9 C7 5F F3 FF"
        ))?;

        // jmp libscr.D8FC1;
        module_addr
            .add(0x0EF70)
            .patch_bytes(&byte_slice!("E9 4C A0 0C 00 90"))?;

        // jmp libscr.D8FCF;
        module_addr
            .add(0x0EF9E)
            .patch_bytes(&byte_slice!("E9 2C A0 0C 00 90"))?;
    }

    unsafe {
        // jmp libscr.D8FE1;
        module_addr
            .add(0x3E3F0)
            .patch_asm(&byte_slice!("E9 EC AB 09 00"))?;

        // mov eax, memcpy2;
        let mut code_buf = vec![0xB8];
        code_buf.extend_from_slice(&(memcpy2 as usize).to_le_bytes());
        // call eax; jmp libscr.3E3F5;
        code_buf.extend_from_slice(&byte_slice!("FF D0 E9 08 54 F6 FF"));

        module_addr.add(0xD8FE1).patch_asm(&code_buf)?;
    }
    Ok(())
}

fn patch_nerbor() -> crate::Result<()> {
    let handle = crate::utils::win32::get_module_handle(w!("libscr.dll"))?;
    let module_addr = handle as *mut u8;

    unsafe {
        // push exe.134368
        let char_addr = module_addr as usize + 0x134368;
        let mut code_buf = vec![0x68];
        code_buf.extend_from_slice(&char_addr.to_le_bytes());
        module_addr.add(0x70D23).patch_asm(&code_buf)?;

        // jmp exe.sub_71080
        module_addr.add(0x7104A).patch_asm(&byte_slice!("EB 34"))?;

        // 00 00 -> 5F 00 (`/` -> `_`)
        module_addr
            .add(0x134368)
            .patch_bytes(&byte_slice!("5F 00"))?;
    }

    unsafe {
        // (push ebp; push ebx; push 0x1; push 0x3A4; jmp MultibytesToWideChar;) * 2
        module_addr.add(0x132FC1).patch_asm(&byte_slice!(
            "55 53 6A 01 68 A4 03 00 00 E9 77 F9 EE FF 55 53 6A 01 68 A4 03 00 00 E9 97 F9 EE FF"
        ))?;

        // jmp exe.132FC1;
        module_addr
            .add(0x22940)
            .patch_bytes(&byte_slice!("E9 7C 06 11 00 90"))?;

        // jmp exe.132FCF;
        module_addr
            .add(0x2296E)
            .patch_bytes(&byte_slice!("E9 5C 06 11 00 90"))?;
    }

    unsafe {
        // jmp exe.132F90;
        module_addr
            .add(0x11FD1B)
            .patch_asm(&byte_slice!("E9 70 32 01 00"))?;

        let code_buf = generate_trampoline_stub_32(
            crate::patch::process_buffer_ffi as _,
            // mov eax,[esp+0x48]; movebx,[esp+0x70]; push eax; push ebx;
            &byte_slice!("8B 44 24 48 8B 5C 24 70 50 53"),
            // mov esi,eax; add esp,8; jmp 11FD20;
            &byte_slice!("8B F0 83 C4 08 E9 71 CD FE FF"),
        );

        module_addr.add(0x132F90).patch_asm(&code_buf)?;
    }
    Ok(())
}

fn patch_by_arg_game_type() -> crate::Result<()> {
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
