pub(crate) mod iat;
pub(crate) mod patch;
pub(crate) mod protect_guard;

/// 从 `*const T` 开始搜索第一个值为 0 的元素，返回 `&[T]`（长度 ≤ max_len）。
///
/// # Safety
/// - `ptr` 必须指向至少 `max_len` 或到第一个 `0` 之前那段可读内存（由调用者保证）。
/// - `ptr` 必须按 `T` 的对齐方式对齐。
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期。
/// - `T` 必须实现 `From<u8>`, `PartialEq` 与 `Copy`（内置无符号整型符合此条件）。
pub unsafe fn slice_until_null<'a, T>(ptr: *const T, max_len: usize) -> &'a [T]
where
    T: PartialEq + Copy + Default,
{
    unsafe {
        if max_len == 0 || quick_memory_check(ptr as *const u8, max_len * size_of::<T>()).is_err() {
            return &[];
        }

        let zero = T::default();

        // 遍历查找第一个 0
        for i in 0..max_len {
            if *ptr.add(i) == zero {
                return core::slice::from_raw_parts(ptr, i);
            }
        }

        // 未找到 0，则以 max_len 返回
        core::slice::from_raw_parts(ptr, max_len)
    }
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
    T: PartialEq + Copy + Default,
{
    unsafe {
        // 如果是非法指针，返回空切片（注意：从任意指针构造 0 长度切片是允许的）
        // 或者长度为 0 的情况直接返回空切片
        if max_len == 0 || quick_memory_check(ptr as *const u8, max_len * size_of::<T>()).is_err() {
            return &mut [];
        }

        let zero = T::default();

        // 遍历查找第一个 0
        for i in 0..max_len {
            if *ptr.add(i) == zero {
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
        if len == 0 || quick_memory_check(ptr as *const u8, len * size_of::<T>()).is_err() {
            return &[];
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
        if len == 0 || quick_memory_check(ptr as *const u8, len * size_of::<T>()).is_err() {
            return &mut [];
        }

        // 指针有效，构造可变切片
        core::slice::from_raw_parts_mut(ptr, len)
    }
}

/// Windows 平台上的简单内存访问检查
pub fn quick_memory_check(ptr: *const u8, len: usize) -> crate::Result<()> {
    if len == 0 {
        return Ok(());
    }

    let addr = ptr as usize;

    // 1. 基础范围检查：避开 Null Page (0 - 64KB)
    if addr < 0x10000 {
        crate::bail!("Pointer address {:#X} is within null page range", addr);
    }

    // 2. 根据架构检查用户空间上限
    #[cfg(target_arch = "x86")]
    let user_space_limit = 0x7FFEFFFF;

    #[cfg(target_arch = "x86_64")]
    let user_space_limit = 0x00007FFFFFFFFFFF;

    // 3. 边界与溢出检查
    if addr > user_space_limit {
        crate::bail!("Address {:#X} exceeds user space limit", addr);
    }

    let end_addr = addr
        .checked_add(len)
        .ok_or_else(|| crate::anyhow!("Memory range overflow: addr {:#X}, len {}", addr, len))?;

    if end_addr > user_space_limit {
        crate::bail!("Memory range end {:#X} exceeds user space limit", end_addr);
    }

    Ok(())
}
