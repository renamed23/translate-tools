pub(crate) mod iat;
pub(crate) mod patch;
pub(crate) mod protect_guard;

/// 创建一个空切片
pub const fn empty_slice<'a, T>() -> &'a [T] {
    unsafe { core::slice::from_raw_parts(core::ptr::NonNull::<T>::dangling().as_ptr(), 0) }
}

/// 创建一个可变的空切片
pub const fn empty_slice_mut<'a, T>() -> &'a mut [T] {
    unsafe { core::slice::from_raw_parts_mut(core::ptr::NonNull::<T>::dangling().as_ptr(), 0) }
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
        // 如果是非法指针，返回空切片（注意：从任意指针构造 0 长度切片是允许的）
        // 或者长度为 0 的情况直接返回空切片
        if max_len == 0 || !quick_memory_check_win32(ptr as *mut u8, max_len * size_of::<T>()) {
            return empty_slice_mut::<'a>();
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

/// 从 `*const T` 构造切片，会进行快速内存检查。
///
/// # 参数
/// - `ptr`: 指向数据的指针
/// - `len`: 期望的切片长度
///
/// # 返回值
/// 如果指针有效则返回指定长度的切片，否则返回空切片
///
/// # Safety
/// - 如果指针有效，必须保证指向至少 `len` 个 `T` 类型元素的有效内存
/// - `ptr` 必须按 `T` 的对齐方式对齐
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期
pub unsafe fn slice_from_raw_parts<'a, T>(ptr: *const T, len: usize) -> &'a [T]
where
    T: Copy,
{
    unsafe {
        // 长度为 0 或者非法指针时直接返回空切片
        if len == 0 || !quick_memory_check_win32(ptr as *mut u8, len * core::mem::size_of::<T>()) {
            return empty_slice::<'a>();
        }

        // 指针有效，构造切片
        core::slice::from_raw_parts(ptr, len)
    }
}

/// 从 `*mut T` 构造可变切片，会进行快速内存检查。
///
/// # 参数  
/// - `ptr`: 指向数据的可变指针
/// - `len`: 期望的切片长度
///
/// # 返回值
/// 如果指针有效则返回指定长度的可变切片，否则返回空切片
///
/// # Safety
/// - 如果指针有效，必须保证指向至少 `len` 个 `T` 类型元素的有效可写内存
/// - `ptr` 必须按 `T` 的对齐方式对齐
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期
pub unsafe fn slice_from_raw_parts_mut<'a, T>(ptr: *mut T, len: usize) -> &'a mut [T]
where
    T: Copy,
{
    unsafe {
        // 长度为 0 或者非法指针时直接返回空切片
        if len == 0 || !quick_memory_check_win32(ptr as *mut u8, len * core::mem::size_of::<T>()) {
            return empty_slice_mut::<'a>();
        }

        // 指针有效，构造可变切片
        core::slice::from_raw_parts_mut(ptr, len)
    }
}

/// 检查切片 `haystack` 是否包含子切片 `needle`
pub fn contains_slice<T: PartialEq>(haystack: &[T], needle: &[T]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Windows 32位平台上的简单内存访问检查
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

/// 将给定值向上对齐到指定对齐粒度的倍数。
///
/// 这个函数计算大于等于 `value` 的最小值，该值必须是 `alignment` 的倍数。
/// 常用于内存对齐、缓冲区大小调整等场景。
///
/// # 参数
/// - `value`: 需要对齐的原始值
/// - `alignment`: 对齐粒度，必须是 2 的幂次方（虽然函数不强制检查）
///
/// # 返回值
/// 返回向上对齐后的值，该值是 `alignment` 的倍数
///
/// # 示例
/// ```
/// assert_eq!(align_up(7, 8), 8);
/// assert_eq!(align_up(16, 8), 16);
/// assert_eq!(align_up(17, 16), 32);
/// assert_eq!(align_up(0, 4), 0);
/// ```
///
/// # 注意
/// 如果 `alignment` 为 0，函数会 panic（由于除零错误）
pub fn align_up(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}
