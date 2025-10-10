use sha2::{Digest, Sha256};

/// 返回输入字节的sha256哈希值
#[allow(dead_code)]
pub fn sha256_of_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&result);
    arr
}

/// Windows 32位平台上的简单内存访问检查
#[allow(dead_code)]
pub fn quick_memory_check_win32(ptr: *mut u8, len: usize) -> bool {
    if len == 0 {
        return true;
    }

    // 32位 Windows 用户空间典型地址范围
    let addr = ptr as usize;

    // 32位地址范围：0x00010000 - 0x7FFEFFFF
    // 避免 NULL 指针区域和小地址区域
    if !(0x00010000..=0x7FFEFFFF).contains(&addr) {
        return false;
    }

    // 检查地址 + len 是否会越界
    if addr.saturating_add(len - 1) > 0x7FFEFFFF {
        return false;
    }

    true
}

/// 检查切片 `haystack` 是否包含子切片 `needle`
#[allow(dead_code)]
pub fn contains_slice<T: PartialEq>(haystack: &[T], needle: &[T]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
