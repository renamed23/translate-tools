use translate_macros::ffi_catch_unwind;
use winapi::shared::minwindef::HMODULE;
use winapi::shared::ntdef::LPCSTR;

use crate::debug;
use crate::hook::CoreHook;
use crate::hook::text_hook::TextHook;

#[derive(Default)]
pub struct UminomHook;

impl CoreHook for UminomHook {
    #[cfg(feature = "patch_extracting")]
    fn on_process_attach(&self, _hinst_dll: HMODULE) {
        {
            // 注入代码
            todo!()
        }
    }
}

impl TextHook for UminomHook {}

#[cfg(feature = "patch_extracting")]
#[ffi_catch_unwind]
pub unsafe extern "system" fn extract_script(ptr: *mut u8, len: usize, filename: LPCSTR) {
    unsafe {
        use std::{ffi::CStr, io::Write};

        let filename = CStr::from_ptr(filename).to_string_lossy();
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
