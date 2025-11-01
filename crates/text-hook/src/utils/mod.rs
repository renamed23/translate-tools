pub(crate) mod mem;
pub(crate) mod nt;
pub(crate) mod panic;
pub(crate) mod trait_impls;
pub(crate) mod win32;

use sha2::{Digest, Sha256};

/// 返回输入字节的sha256哈希值
pub fn sha256_of_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&result);
    arr
}

/// 使用 zstd 解压数据，`cap` 是解压后数据的预估大小
pub fn decompress_zstd(data: &[u8], cap: usize) -> Vec<u8> {
    zstd::bulk::decompress(data, cap).unwrap()
}
