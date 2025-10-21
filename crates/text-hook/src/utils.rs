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

/// 使用 zstd 解压数据，`cap` 是解压后数据的预估大小
#[allow(dead_code)]
pub fn decompress_zstd(data: &[u8], cap: usize) -> Vec<u8> {
    zstd::bulk::decompress(data, cap).unwrap()
}

/// 将 u16 切片转换为带有结尾 NULL 的新 Vec<u16>
#[inline]
#[allow(dead_code)]
pub fn u16_with_null(u16_slice: &[u16]) -> Vec<u16> {
    u16_slice
        .iter()
        .copied()
        .chain(std::iter::once(0u16))
        .collect()
}

/// 从 `*const T` 开始搜索第一个值为 0 的元素，返回 `&[T]`（长度 ≤ max_len）。
///
/// # Safety
/// - `ptr` 必须指向至少 `max_len` 或到第一个 `0` 之前那段可读内存（由调用者保证）。
/// - `ptr` 必须按 `T` 的对齐方式对齐。
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期。
/// - `T` 必须实现 `From<u8>`, `PartialEq` 与 `Copy`（内置无符号整型符合此条件）。
pub unsafe fn slice_until_null<'a, T>(ptr: *const T, max_len: usize) -> &'a [T]
where
    T: From<u8> + PartialEq + Copy,
{
    unsafe { slice_until_null_mut(ptr as *mut T, max_len) }
}

/// 从 `*mut T` 开始搜索第一个值为 0 的元素，返回 `&[T]`（长度 ≤ max_len）。
///
/// # Safety
/// - `ptr` 必须指向至少 `max_len` 或到第一个 `0` 之前那段可读内存（由调用者保证）。
/// - `ptr` 必须按 `T` 的对齐方式对齐。
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期。
/// - `T` 必须实现 `From<u8>`, `PartialEq` 与 `Copy`（内置无符号整型符合此条件）。
pub unsafe fn slice_until_null_mut<'a, T>(ptr: *mut T, max_len: usize) -> &'a mut [T]
where
    T: From<u8> + PartialEq + Copy,
{
    unsafe {
        // 如果是空指针，返回空切片（注意：从任意指针构造 0 长度切片是允许的）
        // 或者长度为 0 的情况直接返回空切片
        if max_len == 0 || ptr.is_null() {
            return core::slice::from_raw_parts_mut(
                core::ptr::NonNull::<T>::dangling().as_ptr(),
                0,
            );
        }

        let zero = T::from(0u8);

        // 遍历查找第一个 0
        for i in 0..max_len {
            let v = ptr.add(i).read();
            if v == zero {
                return core::slice::from_raw_parts_mut(ptr, i);
            }
        }

        // 未找到 0，则以 max_len 返回
        core::slice::from_raw_parts_mut(ptr, max_len)
    }
}
