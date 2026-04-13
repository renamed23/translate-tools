use std::{alloc::Layout, mem::MaybeUninit, ptr::NonNull};

/// 对齐的内存缓冲区。
///
/// 管理一块按指定对齐方式分配的堆内存，生命周期结束时自动释放。
/// 内存内容不保证初始化，可通过 `as_uninit_*` 方法安全地操作未初始化字节，
/// 或通过 `unsafe` 的 `assume_init_*` 方法转换为已初始化的字节切片。
///
/// 类型实现了 `Send`，可安全转移所有权到其他线程，但不实现 `Sync`。
pub struct AlignedBuffer {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl AlignedBuffer {
    /// 创建一个新的对齐缓冲区。
    ///
    /// # 参数
    /// - `size`: 缓冲区字节数，必须大于零。
    /// - `align`: 对齐要求（字节），必须是 2 的幂且不超过系统限制。
    pub fn new(size: usize, align: usize) -> crate::Result<Self> {
        if size == 0 {
            crate::bail!("size must not be zero");
        }

        let layout = Layout::from_size_align(size, align)?;

        unsafe {
            let raw = std::alloc::alloc(layout);
            let ptr = NonNull::new(raw)
                .ok_or_else(|| crate::anyhow!("alloc failed: size={size}, align={align}"))?;

            Ok(Self { ptr, layout })
        }
    }

    /// 为类型 `T` 创建合适大小及对齐的缓冲区。
    ///
    /// 分配大小为 `size_of::<T>()`、对齐为 `align_of::<T>()` 的内存。
    /// `T` 必须不是零大小类型（ZST）
    pub fn new_for<T>() -> crate::Result<Self> {
        let size = core::mem::size_of::<T>();
        let align = core::mem::align_of::<T>();

        if size == 0 {
            crate::bail!(
                "cannot allocate buffer for Zero Sized Type: {}",
                core::any::type_name::<T>()
            );
        }

        Self::new(size, align)
    }

    /// 创建对齐方式满足类型 `T` 要求的缓冲区，可指定字节大小。
    ///
    /// 分配的内存对齐至少为 `align_of::<T>()`，大小为 `size` 字节。
    ///
    /// # 参数
    /// - `size`: 缓冲区字节数。
    pub fn new_aligned_for<T>(size: usize) -> crate::Result<Self> {
        Self::new(size, core::mem::align_of::<T>())
    }

    /// 返回缓冲区字节长度。
    #[inline]
    pub fn len(&self) -> usize {
        self.layout.size()
    }

    /// 返回指向缓冲区起始地址的常量指针。
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// 返回指向缓冲区起始地址的可变指针。
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// 返回未初始化字节的只读切片视图。
    ///
    /// 该视图允许安全地读取或写入 `MaybeUninit<u8>`，不要求底层内存已初始化。
    #[inline]
    pub fn as_uninit_slice(&self) -> &[MaybeUninit<u8>] {
        unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<MaybeUninit<u8>>(), self.len()) }
    }

    /// 返回未初始化字节的可变切片视图。
    ///
    /// 该视图允许安全地修改 `MaybeUninit<u8>`，不要求底层内存已初始化。
    #[inline]
    pub fn as_uninit_mut_slice(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            core::slice::from_raw_parts_mut(self.as_mut_ptr().cast::<MaybeUninit<u8>>(), self.len())
        }
    }

    /// 假设所有字节已初始化，转为只读字节切片。
    ///
    /// # Safety
    /// 调用者必须保证缓冲区中所有字节均已正确初始化
    #[inline]
    pub unsafe fn assume_init_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// 假设所有字节已初始化，转为可变字节切片。
    ///
    /// # Safety
    /// 调用者必须保证缓冲区中所有字节均已正确初始化
    #[inline]
    pub unsafe fn assume_init_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

unsafe impl Send for AlignedBuffer {}
