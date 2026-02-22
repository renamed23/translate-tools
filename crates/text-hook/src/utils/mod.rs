pub(crate) mod error_handling;
pub(crate) mod hwbp;
pub(crate) mod mem;
pub(crate) mod nt;
pub(crate) mod panic;
pub(crate) mod win32;

use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

/// 返回输入字节的sha256哈希值
pub fn sha256_of_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&result);
    arr
}

/// 获取可执行文件所在目录的路径，若失败将会 panic
pub fn get_executable_dir() -> &'static Path {
    static EXECUTABLE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
        crate::utils::win32::get_module_file_name(core::ptr::null_mut())
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .expect("Failed to get executable directory")
    });

    &EXECUTABLE_DIR
}

/// 解压数据，`cap` 是解压后数据的预估大小
pub fn decompress(compressed: &[u8], cap: usize) -> crate::Result<Vec<u8>> {
    let mut data = Vec::with_capacity(cap);
    if let Err(e) = ruzstd::decoding::FrameDecoder::new().decode_all_to_vec(compressed, &mut data) {
        crate::bail!("Decompression failed: {e}");
    }
    Ok(data)
}
