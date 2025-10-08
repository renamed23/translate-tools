mod patch_data;

use crate::{
    debug,
    utils::{quick_memory_check_win32, sha256_of_bytes},
};

/// 零分配构造 byte-key 并执行 body（body 中可使用名为 `$key` 的 `&[u8]`）
///
/// 用法：
// make_key_bytes!(src, key, {
//     patch_data::PATCHES.get(key).map(|p| p.as_slice())
// })
macro_rules! make_key_bytes {
    ($src:expr, $key:ident, $body:block) => {{
        // 计算 sha（[u8;32]）
        let __sha: [u8; 32] = crate::utils::sha256_of_bytes($src);

        // 缓冲：64 hex + ':' + up to 31 digits -> 96 bytes 足够
        let mut __buf = [0u8; 96];
        const __HEX: &[u8; 16] = b"0123456789abcdef";

        // 写 hex 小写
        let mut __i = 0usize;
        while __i < 32 {
            let __b = __sha[__i];
            __buf[2 * __i] = __HEX[((__b >> 4) & 0xF) as usize];
            __buf[2 * __i + 1] = __HEX[(__b & 0xF) as usize];
            __i += 1;
        }

        // 写 ':'
        let mut __pos = 64;
        __buf[__pos] = b':';
        __pos += 1;

        // 写 len 的十进制（先倒序写入 tmp，再反转）
        let mut __tbuf = [0u8; 32];
        let mut __t = 0usize;
        let mut __n = $src.len();
        if __n == 0 {
            __tbuf[0] = b'0';
            __t = 1;
        } else {
            while __n > 0 {
                __tbuf[__t] = b'0' + ((__n % 10) as u8);
                __n /= 10;
                __t += 1;
            }
        }
        let mut __j = 0usize;
        while __j < __t {
            __buf[__pos + __j] = __tbuf[__t - 1 - __j];
            __j += 1;
        }
        let __total = __pos + __t;

        let $key: &[u8] = &__buf[..__total];

        $body
    }};
}

/// 根据目标数据，获取补丁数据
pub fn get_patch(src: &[u8]) -> Option<&'static [u8]> {
    if !patch_data::LEN_FILTER.contains(&src.len()) {
        return None;
    }

    make_key_bytes!(src, key, {
        patch_data::PATCHES.get(key).map(|p| p.as_slice())
    })
}

/// 根据目标数据，获取补丁数据对应的原始文件名（仅在 debug_output 特性启用时可用）
#[cfg(feature = "debug_output")]
pub fn get_filename(src: &[u8]) -> Option<&str> {
    if !patch_data::LEN_FILTER.contains(&src.len()) {
        return None;
    }

    make_key_bytes!(src, key, { patch_data::FILENAMES.get(key).map(|v| &**v) })
}

/// 尝试匹配传入数据，若为目标数据，将会覆盖对应的补丁数据
pub unsafe fn try_patching(ptr: *mut u8, len: usize) {
    debug!(
        "Buffer len: {len}, thread: {:?}",
        std::thread::current().id()
    );

    if !quick_memory_check_win32(ptr, len) {
        return;
    }

    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

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
#[allow(dead_code)]
pub unsafe fn try_extracting(ptr: *mut u8, len: usize) {
    debug!(
        "Buffer len: {len}, thread: {:?}",
        std::thread::current().id()
    );

    if !quick_memory_check_win32(ptr, len) {
        return;
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let new_sha = sha256_of_bytes(slice);

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
                        let existing_sha = sha256_of_bytes(&existing_bytes);
                        if existing_sha == new_sha {
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
        try_patching(ptr, len);
        // try_extracting(ptr, len);
    }
}
