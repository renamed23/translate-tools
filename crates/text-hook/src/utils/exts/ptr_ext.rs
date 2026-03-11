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
        unsafe { crate::utils::mem::slice_from_raw_parts(self.as_const_ptr(), len) }
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
        unsafe { crate::utils::mem::slice_from_raw_parts_mut(self.as_mut_ptr(), len) }
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
    /// - 指针必须有效且指向以 `T::default()` 结尾的连续内存
    /// - 如果未找到空值，切片长度将为 `max_len`
    /// - 返回的切片生命周期 `'a` 由调用者控制
    unsafe fn to_slice_until_null<'a>(self, max_len: usize) -> &'a [T] {
        unsafe { crate::utils::mem::slice_until_null(self.as_const_ptr(), max_len) }
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
    /// - 指针必须有效、可变且指向以 `T::default()` 结尾的连续内存
    /// - 如果未找到空值，切片长度将为 `max_len`
    /// - 返回的切片生命周期 `'a` 由调用者控制
    /// - 此操作会创建可变引用，必须确保在此期间没有其他引用访问同一内存
    unsafe fn to_slice_until_null_mut<'a>(self, max_len: usize) -> &'a mut [T] {
        unsafe { crate::utils::mem::slice_until_null_mut(self.as_mut_ptr(), max_len) }
    }
}
