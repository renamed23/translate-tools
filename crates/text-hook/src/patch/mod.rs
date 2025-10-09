mod patch_data;

use crate::{
    debug,
    utils::{quick_memory_check_win32, sha256_of_bytes},
};

/// 根据目标数据，获取补丁数据
pub fn get_patch(src: &[u8]) -> Option<&'static [u8]> {
    if !patch_data::LEN_FILTER.contains(&src.len()) {
        return None;
    }

    patch_data::PATCHES
        .get(&sha256_of_bytes(src))
        .map(|p| p.as_slice())
}

/// 根据目标数据，获取补丁数据对应的原始文件名（仅在 debug_output 特性启用时可用）
#[cfg(feature = "debug_output")]
pub fn get_filename(src: &[u8]) -> Option<&str> {
    if !patch_data::LEN_FILTER.contains(&src.len()) {
        return None;
    }

    patch_data::FILENAMES
        .get(&sha256_of_bytes(src))
        .map(|v| &**v)
}

/// 尝试匹配传入数据，若为目标数据，将会覆盖对应的补丁数据
#[cfg(not(feature = "patch_extracting"))]
pub unsafe fn try_patching(ptr: *mut u8, len: usize) {
    debug!("Buffer len: {len}",);

    if !quick_memory_check_win32(ptr, len) {
        return;
    }

    let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };

    if let Some(patch) = get_patch(slice) {
        if patch.len() != slice.len() {
            debug!("Error: Patch and raw have different lengths");
            return;
        }

        #[cfg(feature = "debug_output")]
        {
            use crate::patch::get_filename;
            debug!(
                "Patch file applied, len={}, filename={}",
                slice.len(),
                get_filename(slice).unwrap()
            );
        }

        slice.copy_from_slice(patch);
    }
}

/// 尝试提取传入数据，若为新数据，将会写入 raw 目录
#[allow(dead_code, unused_variables)]
#[cfg(feature = "patch_extracting")]
pub unsafe fn try_extracting(ptr: *mut u8, len: usize) {
    debug!("Buffer len: {len}");

    if !quick_memory_check_win32(ptr, len) {
        return;
    }

    let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
    let new_hash = sha256_of_bytes(slice);

    let exe_dir = match std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
    {
        Some(d) => d,
        None => {
            debug!("extract: failed to determine current exe directory");
            return;
        }
    };

    let raw_dir = exe_dir.join("raw");

    if let Err(e) = std::fs::create_dir_all(&raw_dir) {
        debug!("extract: failed to create raw dir {:?}: {:?}", raw_dir, e);
        return;
    }

    // 遍历 raw 目录，查找是否已有完全相同的文件（长度相同且 sha 相同）
    if let Ok(entries) = std::fs::read_dir(&raw_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // 只对数字命名文件进行检查（例如 1.snr, 2.snr）
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if !stem.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
            } else {
                continue;
            }

            match std::fs::read(&path) {
                Ok(existing_bytes) => {
                    if existing_bytes.len() == slice.len() {
                        let existing_hash = sha256_of_bytes(&existing_bytes);
                        if existing_hash == new_hash {
                            debug!(
                                "extract: identical file already exists, skipping write: {:?}",
                                path
                            );
                            return;
                        }
                    }
                }
                Err(e) => {
                    debug!("extract: failed to read existing file {:?}: {:?}", path, e);
                }
            }
        }
    } else {
        debug!("extract: failed to read raw dir {:?}", raw_dir);
    }

    // 没有找到完全相同的文件，确定下一个可用的数字文件名
    let mut max_index: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(&raw_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && let Ok(n) = stem.parse::<u64>()
                && n > max_index
            {
                max_index = n;
            }
        }
    }

    let next = max_index + 1;
    let out_path = raw_dir.join(format!("{next}.snr"));

    match std::fs::write(&out_path, slice) {
        Ok(_) => debug!(
            "extract: wrote raw file {:?} (len={})",
            out_path,
            slice.len()
        ),
        Err(e) => debug!("extract: failed to write file {:?}: {:?}", out_path, e),
    }
}

#[cfg(feature = "default_patch_impl")]
#[translate_macros::ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn replace_script(ptr: *mut u8, len: usize) {
    unsafe {
        #[cfg(not(feature = "patch_extracting"))]
        try_patching(ptr, len);
        #[cfg(feature = "patch_extracting")]
        try_extracting(ptr, len);
    }
}
