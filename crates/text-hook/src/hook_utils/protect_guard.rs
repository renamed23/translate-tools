use std::mem;
use winapi::um::memoryapi::VirtualProtect;
use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

use crate::hook_utils::flush_icache;

/// RAII 内存保护守卫
/// 在构造时修改内存保护，在析构时自动恢复原来的保护
/// 若创建了多个保护守卫，请确保它们按照创建的逆顺序进行析构
/// 不要显式调用drop，不要放入容器，仅当成普通局部变量使用
pub struct ProtectGuard {
    address: *mut u8,
    size: usize,
    pages: Vec<PageProtect>,
}

struct PageProtect {
    base: *mut u8,
    size: usize,
    protect: u32,
}

impl ProtectGuard {
    /// 创建内存保护守卫，逐页设置新的保护并保存原保护
    ///
    /// # 参数
    /// - `address`: 内存起始地址
    /// - `size`: 内存区域大小（字节）
    /// - `new_protect`: 新的保护标志（u32）
    ///
    /// # 安全性
    /// 调用者必须确保地址和大小有效
    #[allow(dead_code)]
    pub unsafe fn new<T>(address: *mut T, size: usize, new_protect: u32) -> anyhow::Result<Self> {
        if size == 0 {
            anyhow::bail!("size must be > 0");
        }
        if address.is_null() {
            anyhow::bail!("address is null");
        }

        // 获取系统 page size
        unsafe {
            let mut sys: SYSTEM_INFO = mem::zeroed();
            GetSystemInfo(&mut sys as _);
            let page_size = sys.dwPageSize as usize;
            if page_size == 0 {
                anyhow::bail!("GetSystemInfo returned page_size == 0");
            }

            let addr_usize = address as usize;
            let end = addr_usize
                .checked_add(size)
                .ok_or_else(|| anyhow::anyhow!("address+size overflow"))?;

            // 计算从哪个 page 开始（向下对齐）到哪个 page 结束（不包含 end）
            let start_page = (addr_usize / page_size) * page_size;
            let mut pages: Vec<PageProtect> = Vec::new();

            let mut page = start_page;
            // 逐页设置保护并保存原值
            while page < end {
                // 设置长度为 page_size（VirtualProtect 对齐到页面），
                // 对最后一页也用 page_size 是安全的（系统按页处理）
                let mut old: u32 = 0;
                let ok = VirtualProtect(page as _, page_size as _, new_protect as _, &mut old as _);
                if ok == 0 {
                    // 出错：尝试回滚已经成功修改过的页面（尽量恢复）
                    let mut _tmp: u32 = 0;
                    for p in &pages {
                        let _ = VirtualProtect(
                            p.base as _,
                            p.size as _,
                            p.protect as _,
                            &mut _tmp as _,
                        );
                    }
                    anyhow::bail!("VirtualProtect failed for page {:p}", page as *const u8);
                }

                pages.push(PageProtect {
                    base: page as *mut u8,
                    size: page_size,
                    protect: old,
                });

                page = match page.checked_add(page_size) {
                    Some(v) => v,
                    None => break,
                };
            }

            Ok(Self {
                address: address as _,
                size,
                pages,
            })
        }
    }

    /// 获取原始地址
    #[allow(dead_code)]
    pub fn address(&self) -> *mut u8 {
        self.address
    }

    /// 获取内存区域大小
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.size
    }

    /// 安全地写入值到受保护的内存
    ///
    /// # 安全性
    /// 调用者必须确保写入的值类型正确且对齐
    #[allow(dead_code)]
    pub unsafe fn write<U: Copy>(&self, value: U) {
        unsafe { self.write_offset(0, value) };
    }

    /// 在指定偏移量处写入值
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `value`: 要写入的值
    ///
    /// # 安全性
    /// 调用者必须确保偏移量在保护范围内，且类型正确对齐
    #[allow(dead_code)]
    pub unsafe fn write_offset<U: Copy>(&self, offset: usize, value: U) {
        unsafe {
            let elem = mem::size_of::<U>();
            assert!(elem > 0, "ZST not supported");
            self.assert_in_bound(offset, elem);

            let target_addr = self.address.add(offset) as *mut U;
            assert!(
                (target_addr as usize).is_multiple_of(mem::align_of::<U>()),
                "write: target not aligned for type"
            );
            target_addr.write_volatile(value);
        }
    }

    /// 从受保护的内存读取值
    ///
    /// # 安全性
    /// 调用者必须确保读取的类型正确且对齐
    #[allow(dead_code)]
    pub unsafe fn read<U: Copy>(&self) -> U {
        unsafe { self.read_offset(0) }
    }

    /// 从指定偏移量处读取值
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    ///
    /// # 安全性
    /// 调用者必须确保偏移量在保护范围内，且类型正确对齐
    #[allow(dead_code)]
    pub unsafe fn read_offset<U: Copy>(&self, offset: usize) -> U {
        unsafe {
            let elem = mem::size_of::<U>();
            assert!(elem > 0, "ZST not supported");
            self.assert_in_bound(offset, elem);

            let source_addr = self.address.add(offset) as *mut U;
            assert!(
                (source_addr as usize).is_multiple_of(mem::align_of::<U>()),
                "read: target not aligned for type"
            );
            source_addr.read_volatile()
        }
    }

    /// 不对齐地写入值到受保护的内存
    ///
    /// # 安全性
    /// 调用者必须确保写入的值类型正确，且偏移量在保护范围内
    #[allow(dead_code)]
    pub unsafe fn write_unaligned<U: Copy>(&self, value: U) {
        unsafe { self.write_offset_unaligned(0, value) };
    }

    /// 在指定偏移量处不对齐地写入值
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `value`: 要写入的值
    ///
    /// # 安全性
    /// 调用者必须确保偏移量在保护范围内
    #[allow(dead_code)]
    pub unsafe fn write_offset_unaligned<U: Copy>(&self, offset: usize, value: U) {
        unsafe {
            let elem = mem::size_of::<U>();
            assert!(elem > 0, "ZST not supported");
            self.assert_in_bound(offset, elem);

            let target_addr = self.address.add(offset) as *mut U;
            target_addr.write_unaligned(value);
        }
    }

    /// 从受保护的内存不对齐地读取值
    ///
    /// # 安全性
    /// 调用者必须确保读取的类型正确，且偏移量在保护范围内
    #[allow(dead_code)]
    pub unsafe fn read_unaligned<U: Copy>(&self) -> U {
        unsafe { self.read_offset_unaligned(0) }
    }

    /// 从指定偏移量处不对齐地读取值
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    ///
    /// # 安全性
    /// 调用者必须确保偏移量在保护范围内
    #[allow(dead_code)]
    pub unsafe fn read_offset_unaligned<U: Copy>(&self, offset: usize) -> U {
        unsafe {
            let elem = mem::size_of::<U>();
            assert!(elem > 0, "ZST not supported");
            self.assert_in_bound(offset, elem);

            let source_addr = self.address.add(offset) as *mut U;
            source_addr.read_unaligned()
        }
    }

    /// 将受保护的内存区域转换为指定类型的切片引用
    ///
    /// # 安全性
    /// 调用者必须确保类型正确且对齐，且不会超出保护范围
    #[allow(dead_code)]
    pub unsafe fn as_slice<U>(&self) -> &[U] {
        let elem = mem::size_of::<U>();
        assert!(elem > 0, "ZST not supported");
        assert!(
            self.size.is_multiple_of(elem),
            "guard size ({}) is not multiple of element size ({})",
            self.size,
            elem
        );
        assert!(
            (self.address as usize).is_multiple_of(mem::align_of::<U>()),
            "address {:p} is not aligned for element (align={})",
            self.address,
            mem::align_of::<U>()
        );

        let count = self.size / elem;
        unsafe { std::slice::from_raw_parts(self.address as *const U, count) }
    }

    /// 将受保护的内存区域转换为指定类型的可变切片引用
    ///
    /// # 安全性
    /// 调用者必须确保类型正确且对齐，且不会超出保护范围
    #[allow(dead_code)]
    pub unsafe fn as_mut_slice<U>(&mut self) -> &mut [U] {
        let elem = mem::size_of::<U>();
        assert!(elem > 0, "ZST not supported");
        assert!(
            self.size.is_multiple_of(elem),
            "guard size ({}) is not multiple of element size ({})",
            self.size,
            elem
        );
        assert!(
            (self.address as usize).is_multiple_of(mem::align_of::<U>()),
            "address {:p} is not aligned for element (align={})",
            self.address,
            mem::align_of::<U>()
        );

        let count = self.size / elem;
        unsafe { std::slice::from_raw_parts_mut(self.address as *mut U, count) }
    }

    /// 写入字节切片到受保护的内存
    ///
    /// # 参数
    /// - `data`: 要写入的字节切片
    ///
    /// # 安全性
    /// 调用者必须确保切片长度不超过保护范围
    #[allow(dead_code)]
    pub unsafe fn write_bytes(&self, data: &[u8]) {
        unsafe { self.write_bytes_ex(0, data, false) }
    }

    /// 在指定偏移量处写入字节切片
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `data`: 要写入的字节切片
    ///
    /// # 安全性
    /// 调用者必须确保切片长度不超过保护范围
    #[allow(dead_code)]
    pub unsafe fn write_bytes_offset(&self, offset: usize, data: &[u8]) {
        unsafe { self.write_bytes_ex(offset, data, false) }
    }

    /// 写入字节切片到受保护的内存，然后刷新指令缓存
    ///
    /// # 参数
    /// - `data`: 要写入的字节切片
    ///
    /// # 安全性
    /// 调用者必须确保切片长度不超过保护范围
    #[allow(dead_code)]
    pub unsafe fn write_asm_bytes(&self, data: &[u8]) {
        unsafe { self.write_bytes_ex(0, data, true) }
    }

    /// 在指定偏移量处写入字节切片，然后刷新指令缓存
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `data`: 要写入的字节切片
    ///
    /// # 安全性
    /// 调用者必须确保切片长度不超过保护范围
    #[allow(dead_code)]
    pub unsafe fn write_asm_bytes_offset(&self, offset: usize, data: &[u8]) {
        unsafe { self.write_bytes_ex(offset, data, true) }
    }

    /// 在指定偏移量处写入字节切片
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `data`: 要写入的字节切片
    /// - `asm`: 若为true，则在写入后会刷新指令缓存
    #[allow(dead_code)]
    pub unsafe fn write_bytes_ex(&self, offset: usize, data: &[u8], asm: bool) {
        if data.is_empty() {
            return;
        }

        let len = data.len();
        self.assert_in_bound(offset, len);

        unsafe {
            let target_addr = self.address.add(offset);
            std::ptr::copy_nonoverlapping(data.as_ptr(), target_addr, len);

            if asm {
                flush_icache(target_addr, len);
            }
        }
    }

    /// 从受保护的内存读取字节到缓冲区
    ///
    /// # 参数
    /// - `buffer`: 用于存储读取数据的缓冲区
    ///
    /// # 返回值
    /// 实际读取的字节数
    ///
    /// # 安全性
    /// 调用者必须确保缓冲区有效
    #[allow(dead_code)]
    pub unsafe fn read_bytes(&self, buffer: &mut [u8]) -> usize {
        unsafe { self.read_bytes_offset(0, buffer) }
    }

    /// 从指定偏移量处读取字节到缓冲区
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `buffer`: 用于存储读取数据的缓冲区
    ///
    /// # 返回值
    /// 实际读取的字节数
    ///
    /// # 安全性
    /// 调用者必须确保偏移量和缓冲区有效
    #[allow(dead_code)]
    pub unsafe fn read_bytes_offset(&self, offset: usize, buffer: &mut [u8]) -> usize {
        if buffer.is_empty() {
            return 0;
        }

        let len = buffer.len();
        self.assert_in_bound(offset, len);

        unsafe {
            let source_addr = self.address.add(offset);
            std::ptr::copy_nonoverlapping(source_addr, buffer.as_mut_ptr(), len);
        }
        len
    }

    /// 检查指定偏移量和长度是否超出保护范围
    fn assert_in_bound(&self, offset: usize, len: usize) {
        if len == 0 {
            return;
        }

        let out_bound = offset.checked_add(len).is_none_or(|end| end > self.size);

        assert!(
            !out_bound,
            "out of bounds (offset {} + size {} > guard size {})",
            offset, len, self.size
        );
    }
}

impl Drop for ProtectGuard {
    fn drop(&mut self) {
        unsafe {
            let mut _tmp: u32 = 0;
            for p in &self.pages {
                let _ok = VirtualProtect(p.base as _, p.size as _, p.protect as _, &mut _tmp as _);

                #[cfg(feature = "debug_output")]
                if _ok == 0 {
                    crate::debug!("VirtualProtect restore failed for {:p}", p.base);
                }
            }
        }
    }
}

/// 写入汇编字节到指定地址，自动处理内存保护和指令缓存刷新
#[allow(dead_code)]
pub fn write_asm(address: *mut u8, data: &[u8]) -> anyhow::Result<()> {
    if address.is_null() {
        anyhow::bail!("address is null");
    }
    if data.is_empty() {
        return Ok(());
    }

    unsafe {
        ProtectGuard::new(
            address,
            data.len(),
            winapi::um::winnt::PAGE_EXECUTE_READWRITE,
        )?
        .write_asm_bytes(data);
    }

    Ok(())
}

/// 写入字节到指定地址，自动处理内存保护
#[allow(dead_code)]
pub fn write_bytes(address: *mut u8, data: &[u8]) -> anyhow::Result<()> {
    if address.is_null() {
        anyhow::bail!("address is null");
    }
    if data.is_empty() {
        return Ok(());
    }

    unsafe {
        ProtectGuard::new(address, data.len(), winapi::um::winnt::PAGE_READWRITE)?
            .write_bytes(data);
    }

    Ok(())
}
