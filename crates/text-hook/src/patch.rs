use crate::utils::sha256_of_bytes;
use cfg_if::cfg_if;

mod patch_data {
    translate_macros::generate_patch_data!("assets/raw_patch" => "assets/translated_patch");
}

/// 根据目标数据，获取补丁数据
pub fn get_patch(src: &[u8]) -> Option<&'static [u8]> {
    if !is_patch_len(src.len()) {
        return None;
    }

    if crate::utils::mem::quick_memory_check(src.as_ptr(), src.len()).is_err() {
        return None;
    }

    let patch = patch_data::PATCHES.get(&sha256_of_bytes(src))?.as_slice();

    #[cfg(feature = "enable_debug_output")]
    crate::debug!(
        "Got Patch file, len={}, filename={}",
        patch.len(),
        get_filename(patch).unwrap()
    );

    Some(patch)
}

/// 获取补丁数据或执行提取。
///
/// - 非 `patch_extracting` 模式：返回 `Ok(Some(patch))` / `Ok(None)`。
/// - `patch_extracting` 模式：尝试提取并返回 `Ok(None)`。
pub fn get_patch_or_extract(src: &[u8]) -> crate::Result<Option<&'static [u8]>> {
    cfg_if! {
        if #[cfg(feature = "extract_patch")] {
            extract_to_file(src)?;
            Ok(None)
        } else {
            Ok(get_patch(src))
        }
    }
}

/// 是否是需要进行处理的补丁的长度？
fn is_patch_len(len: usize) -> bool {
    patch_data::LEN_FILTER.contains(&len)
}

/// 根据目标数据，获取补丁数据对应的原始文件名（仅在 debug_output 特性启用时可用）
#[cfg(feature = "enable_debug_output")]
fn get_filename(src: &[u8]) -> Option<&str> {
    if !is_patch_len(src.len()) {
        return None;
    }

    patch_data::FILENAMES
        .get(&sha256_of_bytes(src))
        .map(|v| &**v)
}

/// 尝试提取传入数据，若为新数据，将会写入 raw 目录。
///
/// # Safety
/// - `ptr` 必须指向长度至少为 `len` 的可读有效内存。
/// - 调用者需保证该内存在本次调用期间保持有效且不被并发修改。
#[allow(dead_code, unused_variables)]
#[cfg(feature = "extract_patch")]
pub fn extract_to_file(buf: &[u8]) -> crate::Result<()> {
    crate::utils::mem::quick_memory_check(buf.as_ptr(), buf.len())?;

    let new_hash = sha256_of_bytes(buf);

    let exe_dir = crate::utils::get_executable_dir();
    let raw_dir = exe_dir.join("raw");
    std::fs::create_dir_all(&raw_dir)?;

    let mut max_index: u64 = 0;

    // 遍历 raw 目录，查找是否已有完全相同的文件（长度相同且 sha 相同）
    let entries = std::fs::read_dir(&raw_dir)?;
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
        let existing_bytes = std::fs::read(&path)?;
        if existing_bytes.len() == buf.len() {
            let existing_hash = sha256_of_bytes(&existing_bytes);
            if existing_hash == new_hash {
                crate::debug!("Identical file already exists, skipping write: {:?}", path);
                return Ok(());
            }
        }
    }

    // --- 如果循环正常结束，说明没有找到任何重复的文件 ---
    // 此时的 max_index 就是目录中最大的索引值。
    let next = max_index + 1;
    let out_path = raw_dir.join(format!("{next}.snr"));
    std::fs::write(&out_path, buf)?;
    crate::debug!("Wrote raw file {:?} (len={})", out_path, buf.len());
    Ok(())
}
