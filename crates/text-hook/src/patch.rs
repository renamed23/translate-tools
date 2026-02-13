use crate::{debug, utils::sha256_of_bytes};

mod patch_data {
    translate_macros::generate_patch_data!("assets/raw" => "assets/translated");
}

/// 根据目标数据，获取补丁数据
pub fn get_patch(src: &[u8]) -> Option<&'static [u8]> {
    if !is_patch_len(src.len()) {
        return None;
    }

    let data = patch_data::PATCHES.get(&sha256_of_bytes(src))?.as_slice();
    if data.len() != src.len() {
        debug!("Error: Patch and raw have different lengths");
        return None;
    }

    Some(data)
}

/// 是否是需要进行处理的补丁的长度？
pub fn is_patch_len(len: usize) -> bool {
    patch_data::LEN_FILTER.contains(&len)
}

/// 根据目标数据，获取补丁数据对应的原始文件名（仅在 debug_output 特性启用时可用）
#[cfg(feature = "debug_output")]
pub fn get_filename(src: &[u8]) -> Option<&str> {
    if !is_patch_len(src.len()) {
        return None;
    }

    patch_data::FILENAMES
        .get(&sha256_of_bytes(src))
        .map(|v| &**v)
}

/// 尝试匹配传入数据，若为目标数据，将会覆盖对应的补丁数据。
/// 返回`true`表示修补成功
#[cfg(not(feature = "patch_extracting"))]
pub unsafe fn try_patching(ptr: *mut u8, len: usize) -> bool {
    if !crate::utils::mem::quick_memory_check_win32(ptr, len) {
        return false;
    }

    let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };

    if let Some(patch) = get_patch(slice) {
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
        true
    } else {
        false
    }
}

/// 尝试提取传入数据，若为新数据，将会写入 raw 目录。
/// 返回`true`表示提取成功
#[allow(dead_code, unused_variables)]
#[cfg(feature = "patch_extracting")]
pub unsafe fn try_extracting(ptr: *mut u8, len: usize) -> bool {
    if !crate::utils::mem::quick_memory_check_win32(ptr, len) {
        return false;
    }

    let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
    let new_hash = sha256_of_bytes(slice);

    let exe_dir = crate::utils::get_executable_dir();

    let raw_dir = exe_dir.join("raw");

    if let Err(e) = std::fs::create_dir_all(&raw_dir) {
        debug!("Failed to create raw dir {:?}: {:?}", raw_dir, e);
        return false;
    }

    let mut max_index: u64 = 0;

    // 遍历 raw 目录，查找是否已有完全相同的文件（长度相同且 sha 相同）
    if let Ok(entries) = std::fs::read_dir(&raw_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // --- 步骤1: 尝试更新 max_index ---
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(n) = stem.parse::<u64>() {
                    max_index = max_index.max(n);
                } else {
                    // 如果文件名不是纯数字，则跳过后续的哈希检查
                    continue;
                }
            } else {
                continue;
            }

            // --- 步骤2: 检查文件内容是否重复 ---
            match std::fs::read(&path) {
                Ok(existing_bytes) => {
                    if existing_bytes.len() == slice.len() {
                        let existing_hash = sha256_of_bytes(&existing_bytes);
                        if existing_hash == new_hash {
                            debug!("Identical file already exists, skipping write: {:?}", path);
                            return false;
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to read existing file {:?}: {:?}", path, e);
                }
            }
        }
    } else {
        debug!("Failed to read raw dir {:?}", raw_dir);
    }

    // --- 如果循环正常结束，说明没有找到任何重复的文件 ---
    // 此时的 max_index 就是目录中最大的索引值。
    let next = max_index + 1;
    let out_path = raw_dir.join(format!("{next}.snr"));

    match std::fs::write(&out_path, slice) {
        Ok(_) => {
            debug!("Wrote raw file {:?} (len={})", out_path, slice.len());
            true
        }
        Err(e) => {
            debug!("Failed to write file {:?}: {:?}", out_path, e);
            false
        }
    }
}

/// 处理传入的缓冲区，进行修补或提取。
/// 返回`true`表示修补或提取成功。
///
/// 仅限RUST内部使用，若要用于外部代码，请使用`process_buffer_ffi`
#[inline]
pub fn process_buffer(ptr: *mut u8, len: usize) -> bool {
    unsafe {
        #[cfg(not(feature = "patch_extracting"))]
        return try_patching(ptr, len);

        #[cfg(feature = "patch_extracting")]
        return try_extracting(ptr, len);
    }
}

/// FFI版本的`process_buffer`
#[translate_macros::ffi_catch_unwind]
#[cfg_attr(feature = "export_patch_process_fn", unsafe(no_mangle))]
pub unsafe extern "system" fn process_buffer_ffi(dst: *mut u8, len: usize) {
    process_buffer(dst, len);
}
