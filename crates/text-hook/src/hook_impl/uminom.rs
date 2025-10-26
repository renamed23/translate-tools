use translate_macros::ffi_catch_unwind;
#[cfg(feature = "patch_extracting")]
use windows_sys::{Win32::Foundation::HMODULE, core::PCSTR};

use crate::debug;
use crate::hook::CoreHook;
use crate::hook::text_hook::TextHook;
use crate::hook::window_hook::WindowHook;

#[derive(Default)]
pub struct UminomHook;

impl CoreHook for UminomHook {
    // 解压器实现，仅为了获取解包数据，不要在真补丁中使用
    #[cfg(feature = "patch_extracting")]
    fn on_process_attach(&self, _hinst_dll: HMODULE) {
        use translate_macros::byte_slice;

        let Some(handle) = crate::utils::win32::get_module_handle("") else {
            debug!("get_module_handle failed");
            return;
        };

        debug!("patch {handle:p}");

        let module_addr = handle as *mut u8;

        unsafe {
            crate::utils::mem::patch::write_asm(
                module_addr.add(0x5DDDB),
                &byte_slice!("E9 20 91 03 00"), // jmp 0x00496F00
            )
            .unwrap();

            let code_buf = crate::utils::mem::patch::create_trampoline_32(
                extract_script as _,
                // mov eax,[esp+0x40]; mov ebx,[esp+0x2C]; mov ecx,[esp+0x28];
                // push eax; push ebx; pushcx;
                &byte_slice!("8B 44 24 40 8B 5C 24 2C 8B 4C 24 28 50 53 51"),
                // ret 0x10;
                &byte_slice!("C2 10 00"),
            );

            crate::utils::mem::patch::write_asm(module_addr.add(0x96F00), &code_buf).unwrap();
        }
    }
}

impl TextHook for UminomHook {}
impl WindowHook for UminomHook {}

#[cfg(feature = "patch_extracting")]
#[ffi_catch_unwind]
pub unsafe extern "system" fn extract_script(ptr: *mut u8, len: usize, filename: PCSTR) {
    unsafe {
        use std::io::Write;
        use windows_sys::Win32::Foundation::MAX_PATH;

        let filename =
            String::from_utf8_lossy(crate::utils::mem::slice_until_null(filename, MAX_PATH as _));
        if crate::patch::try_extracting(ptr, len) {
            let new_filename = format!("{filename}.isf");

            // 读取cwd的`raw/filenames.txt`，如果没有就创建，然后在末尾添加`{new_filename}\n`
            if let Ok(current_dir) = std::env::current_dir() {
                let raw_dir = current_dir.join("raw");

                // 创建raw目录（如果不存在）
                if let Err(e) = std::fs::create_dir_all(&raw_dir) {
                    debug!("Failed to create raw directory: {e}");
                    return;
                }

                // 文件名列表文件路径
                let filenames_path = raw_dir.join("filenames.txt");

                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&filenames_path)
                {
                    writeln!(file, "{}", new_filename).unwrap();
                }
            }
        }
    }
}
