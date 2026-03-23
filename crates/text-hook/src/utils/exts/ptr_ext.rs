#![allow(clippy::wrong_self_convention)]

pub trait AsPtrExt<T> {
    /// 将自身转换为常量指针 `*const T`。
    ///
    /// # Safety
    ///
    /// 调用者必须确保指针有效且生命周期足够长。
    unsafe fn as_const_ptr(self) -> *const T;

    /// 将自身转换为可变指针 `*mut T`。
    ///
    /// # Safety
    ///
    /// 调用者必须确保指针有效、可变且生命周期足够长。
    unsafe fn as_mut_ptr(self) -> *mut T;
}

macro_rules! bulk_impl_ptr {
    ($target:ty => $($src:ty),*) => {
        $(
            impl AsPtrExt<$target> for $src {
                #[inline(always)]
                unsafe fn as_const_ptr(self) -> *const $target { self as _ }
                #[inline(always)]
                unsafe fn as_mut_ptr(self) -> *mut $target { self as _ }
            }
            impl PtrExt<$target> for $src {}
        )*
    };
}

// 为 u8 指针类型实现转换
bulk_impl_ptr!(u8 => *const u8, *mut u8, *const i8, *mut i8);
// 为 u16 指针类型实现转换
bulk_impl_ptr!(u16 => *const u16, *mut u16, *const i16, *mut i16);

pub trait PtrExt<T>: AsPtrExt<T> + Sized
where
    T: Copy + PartialEq + Default,
{
    /// 严格地将指针转换为指定长度的不可变切片引用。
    ///
    /// # Safety
    ///
    /// - 指针必须有效且指向至少 `len` 个连续的元素
    /// - 返回的切片生命周期 `'a` 由调用者控制，必须确保不超过底层内存的有效期
    unsafe fn try_to_slice<'a>(self, len: usize) -> crate::Result<&'a [T]> {
        unsafe { crate::utils::mem::try_slice_from_raw_parts(self.as_const_ptr(), len) }
    }

    /// 将指针转换为指定长度的不可变切片引用。
    ///
    /// # 参数
    ///
    /// * `len` - 切片长度（元素个数）
    ///
    /// # Safety
    ///
    /// - 指针必须有效且指向至少 `len` 个连续的元素
    /// - 返回的切片生命周期 `'a` 由调用者控制，必须确保不超过底层内存的有效期
    unsafe fn to_slice<'a>(self, len: usize) -> &'a [T] {
        match unsafe { self.try_to_slice(len) } {
            Ok(slice) => slice,
            Err(err) => {
                crate::debug!("to_slice failed: {err:?}");
                &[]
            }
        }
    }

    /// 严格地将指针转换为指定长度的可变切片引用。
    ///
    /// # Safety
    ///
    /// - 指针必须有效、可变且指向至少 `len` 个连续的元素
    /// - 返回的切片生命周期 `'a` 由调用者控制，必须确保不超过底层内存的有效期
    /// - 此操作会创建可变引用，必须确保在此期间没有其他引用访问同一内存
    unsafe fn try_to_slice_mut<'a>(self, len: usize) -> crate::Result<&'a mut [T]> {
        unsafe { crate::utils::mem::try_slice_from_raw_parts_mut(self.as_mut_ptr(), len) }
    }

    /// 将指针转换为指定长度的可变切片引用。
    ///
    /// # 参数
    ///
    /// * `len` - 切片长度（元素个数）
    ///
    /// # Safety
    ///
    /// - 指针必须有效、可变且指向至少 `len` 个连续的元素
    /// - 返回的切片生命周期 `'a` 由调用者控制，必须确保不超过底层内存的有效期
    /// - 此操作会创建可变引用，必须确保在此期间没有其他引用访问同一内存
    unsafe fn to_slice_mut<'a>(self, len: usize) -> &'a mut [T] {
        match unsafe { self.try_to_slice_mut(len) } {
            Ok(slice) => slice,
            Err(err) => {
                crate::debug!("to_slice_mut failed: {err:?}");
                &mut []
            }
        }
    }

    /// 严格地将指针转换为以空值（`T::default()`）结尾的不可变切片引用。
    ///
    /// # Safety
    ///
    /// - 指针必须有效，且在 `max_len` 范围内可读
    /// - 如果在 `max_len` 范围内找到 `T::default()`，返回终止符之前的切片
    /// - 如果未找到 `T::default()`，返回长度为 `max_len` 的切片
    /// - 返回的切片生命周期 `'a` 由调用者控制
    unsafe fn try_to_slice_until_null<'a>(self, max_len: usize) -> crate::Result<&'a [T]> {
        unsafe { crate::utils::mem::try_slice_until_null(self.as_const_ptr(), max_len) }
    }

    /// 严格地将指针转换为以空值结尾的不可变切片引用，使用统一扫描上限。
    ///
    /// # Safety
    ///
    /// - 指针必须有效，且在 `crate::constant::SCAN_MAX_LEN` 范围内可读
    /// - 返回的切片生命周期 `'a` 由调用者控制
    unsafe fn try_to_slice_until_null_scan<'a>(self) -> crate::Result<&'a [T]> {
        unsafe { self.try_to_slice_until_null(crate::constant::SCAN_MAX_LEN) }
    }

    /// 将指针转换为以空值（T::default()）结尾的不可变切片引用。
    ///
    /// 常用于处理以 null 结尾的 C 风格字符串或 Windows UTF-16 字符串。
    /// 扫描会在遇到 `T::default()`（通常为 0）或达到 `max_len` 时停止。
    ///
    /// # 参数
    ///
    /// * `max_len` - 最大扫描长度，防止无限扫描无效内存
    ///
    /// # Safety
    ///
    /// - 指针必须有效，且在 `max_len` 范围内可读
    /// - 如果未找到空值，则返回长度为 `max_len` 的切片
    /// - 如果快速内存检查失败，将记录调试日志并返回空切片
    /// - 返回的切片生命周期 `'a` 由调用者控制
    unsafe fn to_slice_until_null<'a>(self, max_len: usize) -> &'a [T] {
        match unsafe { self.try_to_slice_until_null(max_len) } {
            Ok(slice) => slice,
            Err(err) => {
                crate::debug!("to_slice_until_null failed: {err:?}");
                &[]
            }
        }
    }

    /// 将指针转换为以空值结尾的不可变切片引用，使用统一扫描上限。
    ///
    /// 常用于处理仅需防御性限制扫描长度的 C 风格字符串或 UTF-16 字符串。
    ///
    /// # Safety
    ///
    /// - 指针必须有效，且在 `crate::constant::SCAN_MAX_LEN` 范围内可读
    /// - 返回的切片生命周期 `'a` 由调用者控制
    unsafe fn to_slice_until_null_scan<'a>(self) -> &'a [T] {
        match unsafe { self.try_to_slice_until_null_scan() } {
            Ok(slice) => slice,
            Err(err) => {
                crate::debug!("to_slice_until_null_scan failed: {err:?}");
                &[]
            }
        }
    }

    /// 严格地将指针转换为以空值（`T::default()`）结尾的可变切片引用。
    ///
    /// # Safety
    ///
    /// - 指针必须有效、可变，且在 `max_len` 范围内可读可写
    /// - 如果在 `max_len` 范围内找到 `T::default()`，返回终止符之前的切片
    /// - 如果未找到 `T::default()`，返回长度为 `max_len` 的切片
    /// - 返回的切片生命周期 `'a` 由调用者控制
    /// - 此操作会创建可变引用，必须确保在此期间没有其他引用访问同一内存
    unsafe fn try_to_slice_until_null_mut<'a>(self, max_len: usize) -> crate::Result<&'a mut [T]> {
        unsafe { crate::utils::mem::try_slice_until_null_mut(self.as_mut_ptr(), max_len) }
    }

    /// 严格地将指针转换为以空值结尾的可变切片引用，使用统一扫描上限。
    ///
    /// # Safety
    ///
    /// - 指针必须有效、可变，且在 `crate::constant::SCAN_MAX_LEN` 范围内可读可写
    /// - 返回的切片生命周期 `'a` 由调用者控制
    unsafe fn try_to_slice_until_null_mut_scan<'a>(self) -> crate::Result<&'a mut [T]> {
        unsafe { self.try_to_slice_until_null_mut(crate::constant::SCAN_MAX_LEN) }
    }

    /// 将指针转换为以空值（T::default()）结尾的可变切片引用。
    ///
    /// 常用于修改以 null 结尾的 C 风格缓冲区。
    /// 扫描会在遇到 `T::default()`（通常为 0）或达到 `max_len` 时停止。
    ///
    /// # 参数
    ///
    /// * `max_len` - 最大扫描长度，防止无限扫描无效内存
    ///
    /// # Safety
    ///
    /// - 指针必须有效、可变，且在 `max_len` 范围内可读可写
    /// - 如果未找到空值，则返回长度为 `max_len` 的切片
    /// - 如果快速内存检查失败，将记录调试日志并返回空切片
    /// - 返回的切片生命周期 `'a` 由调用者控制
    /// - 此操作会创建可变引用，必须确保在此期间没有其他引用访问同一内存
    unsafe fn to_slice_until_null_mut<'a>(self, max_len: usize) -> &'a mut [T] {
        match unsafe { self.try_to_slice_until_null_mut(max_len) } {
            Ok(slice) => slice,
            Err(err) => {
                crate::debug!("to_slice_until_null_mut failed: {err:?}");
                &mut []
            }
        }
    }

    /// 将指针转换为以空值结尾的可变切片引用，使用统一扫描上限。
    ///
    /// # Safety
    ///
    /// - 指针必须有效、可变，且在 `crate::constant::SCAN_MAX_LEN` 范围内可读可写
    /// - 返回的切片生命周期 `'a` 由调用者控制
    unsafe fn to_slice_until_null_mut_scan<'a>(self) -> &'a mut [T] {
        match unsafe { self.try_to_slice_until_null_mut_scan() } {
            Ok(slice) => slice,
            Err(err) => {
                crate::debug!("to_slice_until_null_mut_scan failed: {err:?}");
                &mut []
            }
        }
    }
}

pub trait PtrWriteExt {
    /// 写入汇编字节到当前地址，返回自身以支持链式调用。
    fn patch_asm(self, data: &[u8]) -> crate::Result<Self>
    where
        Self: Sized;

    /// 写入普通字节到当前地址，返回自身以支持链式调用。
    fn patch_bytes(self, data: &[u8]) -> crate::Result<Self>
    where
        Self: Sized;

    /// 写入 32 位相对偏移指令（支持 jmp/call 等），返回自身以支持链式调用。
    fn write_rel32_instruction<const OPCODE: u8>(
        self,
        target_function: *const u8,
    ) -> crate::Result<Self>
    where
        Self: Sized;

    /// 写入 32 位相对跳转指令（E9 jmp），返回自身以支持链式调用。
    fn write_jmp_instruction(self, target_function: *const u8) -> crate::Result<Self>
    where
        Self: Sized;

    /// 写入 32 位相对调用指令（E8 call），返回自身以支持链式调用。
    fn write_call_instruction(self, target_function: *const u8) -> crate::Result<Self>
    where
        Self: Sized;
}

impl PtrWriteExt for *mut u8 {
    fn patch_asm(self, data: &[u8]) -> crate::Result<Self> {
        crate::utils::mem::patch::write_asm(self, data)?;
        Ok(self)
    }

    fn patch_bytes(self, data: &[u8]) -> crate::Result<Self> {
        crate::utils::mem::patch::write_bytes(self, data)?;
        Ok(self)
    }

    fn write_rel32_instruction<const OPCODE: u8>(
        self,
        target_function: *const u8,
    ) -> crate::Result<Self> {
        unsafe {
            crate::utils::mem::patch::write_rel32_instruction::<OPCODE>(self, target_function)?;
        }
        Ok(self)
    }

    fn write_jmp_instruction(self, target_function: *const u8) -> crate::Result<Self> {
        unsafe {
            crate::utils::mem::patch::write_jmp_instruction(self, target_function)?;
        }
        Ok(self)
    }

    fn write_call_instruction(self, target_function: *const u8) -> crate::Result<Self> {
        unsafe {
            crate::utils::mem::patch::write_call_instruction(self, target_function)?;
        }
        Ok(self)
    }
}
