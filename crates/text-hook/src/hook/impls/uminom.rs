use translate_macros::DefaultHook;
#[cfg(feature = "patch_extracting")]
use windows_sys::{Win32::Foundation::HMODULE, core::PCSTR};

use crate::hook::traits::CoreHook;

#[derive(Default, DefaultHook)]
pub struct UminomHook;

impl CoreHook for UminomHook {
    // 解压器实现，仅为了获取解包数据，不要在真补丁中使用
    #[cfg(feature = "patch_extracting")]
    fn on_process_attach(&self, _hinst_dll: HMODULE) {
        let Some(handle) = crate::utils::win32::get_module_handle("") else {
            crate::debug!("get_module_handle failed");
            return;
        };

        crate::debug!("patch {handle:p}");

        let module_addr = handle as *mut u8;

        unsafe {
            crate::utils::mem::patch::write_jmp_instruction(
                module_addr.add(0x5DDDB),
                trampoline as _,
            )
            .unwrap();
        }
    }
}

#[cfg(feature = "patch_extracting")]
#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline() {
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov eax,[esp+0x40];
        mov ebx,[esp+0x2C]; 
        mov ecx,[esp+0x28];
        push eax;
        push ebx;
        push ecx;
        call {0};
        popfd;
        popad;
        ret 0x10
        ",
        sym extract_script,
    );
}

#[cfg(feature = "patch_extracting")]
#[translate_macros::ffi_catch_unwind]
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
                    crate::debug!("Failed to create raw directory: {e}");
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
