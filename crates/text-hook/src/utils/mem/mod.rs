pub(crate) mod aligned_buf;
pub(crate) mod iat;
pub(crate) mod patch;
pub(crate) mod protect_guard;

/// 从 `*const T` 开始搜索第一个值为 0 的元素，返回 `&[T]`（长度 <= `max_len`）。
///
/// # Safety
/// - `ptr` 必须指向至少 `max_len` 个元素的可读内存，或者在此范围内提前遇到第一个 `0`。
/// - `ptr` 必须按 `T` 的对齐方式对齐。
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期。
/// - `T` 必须实现 `PartialEq`、`Copy` 与 `Default`。
///
/// # 返回值
/// - 如果在 `max_len` 范围内找到 `T::default()`，返回终止符之前的切片。
/// - 如果未找到终止符，返回长度为 `max_len` 的切片。
/// - 如果指针范围未通过快速内存检查，则返回错误。
pub unsafe fn try_slice_until_null<'a, T>(ptr: *const T, max_len: usize) -> crate::Result<&'a [T]>
where
    T: PartialEq + Copy + Default,
{
    unsafe {
        if max_len == 0 {
            return Ok(&[]);
        }

        quick_memory_check(ptr.cast::<u8>(), max_len * size_of::<T>())?;

        let zero = T::default();

        for i in 0..max_len {
            if *ptr.add(i) == zero {
                return Ok(core::slice::from_raw_parts(ptr, i));
            }
        }

        // 未找到 0，则以 max_len 返回
        Ok(core::slice::from_raw_parts(ptr, max_len))
    }
}

/// 从 `*mut T` 开始搜索第一个值为 0 的元素，返回 `&mut [T]`（长度 <= `max_len`）。
///
/// # Safety
/// - `ptr` 必须指向至少 `max_len` 个元素的可读可写内存，或者在此范围内提前遇到第一个 `0`。
/// - `ptr` 必须按 `T` 的对齐方式对齐。
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期。
/// - 必须满足可变引用的别名规则。
///
/// # 返回值
/// - 如果在 `max_len` 范围内找到 `T::default()`，返回终止符之前的切片。
/// - 如果未找到终止符，返回长度为 `max_len` 的切片。
/// - 如果指针范围未通过快速内存检查，则返回错误。
pub unsafe fn try_slice_until_null_mut<'a, T>(
    ptr: *mut T,
    max_len: usize,
) -> crate::Result<&'a mut [T]>
where
    T: PartialEq + Copy + Default,
{
    unsafe {
        if max_len == 0 {
            return Ok(&mut []);
        }

        quick_memory_check(ptr as *const u8, max_len * size_of::<T>())?;

        let zero = T::default();

        for i in 0..max_len {
            if *ptr.add(i) == zero {
                return Ok(core::slice::from_raw_parts_mut(ptr, i));
            }
        }

        Ok(core::slice::from_raw_parts_mut(ptr, max_len))
    }
}

/// 从 `*const T` 构造切片，并进行快速内存检查。
///
/// # Safety
/// - 如果指针有效，必须保证指向至少 `len` 个 `T` 类型元素的有效内存
/// - `ptr` 必须按 `T` 的对齐方式对齐
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期
///
/// # 返回值
/// - 成功时返回长度为 `len` 的切片。
/// - 如果指针范围未通过快速内存检查，则返回错误。
pub unsafe fn try_slice_from_raw_parts<'a, T>(ptr: *const T, len: usize) -> crate::Result<&'a [T]>
where
    T: Copy,
{
    unsafe {
        if len == 0 {
            return Ok(&[]);
        }

        quick_memory_check(ptr.cast::<u8>(), len * size_of::<T>())?;

        Ok(core::slice::from_raw_parts(ptr, len))
    }
}

/// 从 `*mut T` 构造可变切片，并进行快速内存检查。
///
/// # Safety
/// - 如果指针有效，必须保证指向至少 `len` 个 `T` 类型元素的有效可写内存
/// - `ptr` 必须按 `T` 的对齐方式对齐
/// - 返回的切片生命周期 `'a` 必须小于等于该内存有效期
/// - 必须满足可变引用的别名规则
///
/// # 返回值
/// - 成功时返回长度为 `len` 的可变切片。
/// - 如果指针范围未通过快速内存检查，则返回错误。
pub unsafe fn try_slice_from_raw_parts_mut<'a, T>(
    ptr: *mut T,
    len: usize,
) -> crate::Result<&'a mut [T]>
where
    T: Copy,
{
    unsafe {
        if len == 0 {
            return Ok(&mut []);
        }

        quick_memory_check(ptr as *const u8, len * size_of::<T>())?;

        Ok(core::slice::from_raw_parts_mut(ptr, len))
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
    let user_space_limit = 0x7FFE_FFFF;

    #[cfg(target_arch = "x86_64")]
    let user_space_limit = 0x0000_7FFF_FFFF_FFFF;

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
