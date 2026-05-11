use windows_sys::Win32::System::{
    Memory::VirtualProtect,
    SystemInformation::{GetSystemInfo, SYSTEM_INFO},
};

use crate::utils::mem::patch::flush_icache;

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
    /// # Safety
    /// 调用者必须确保地址和大小有效
    pub unsafe fn new(address: *mut u8, size: usize, new_protect: u32) -> crate::Result<Self> {
        if size == 0 {
            crate::bail!("size must be > 0");
        }
        if crate::utils::mem::quick_memory_check(address, size).is_err() {
            crate::bail!("address is invalid: ({address:p};{size})");
        }

        // 获取系统 page size
        unsafe {
            let mut sys = SYSTEM_INFO::default();
            GetSystemInfo(&raw mut sys);
            let page_size = sys.dwPageSize as usize;
            if page_size == 0 {
                crate::bail!("GetSystemInfo returned page_size == 0");
            }

            let addr_usize = address as usize;
            let end = addr_usize
                .checked_add(size)
                .ok_or_else(|| crate::anyhow!("address+size overflow"))?;

            // 计算从哪个 page 开始（向下对齐）到哪个 page 结束（不包含 end）
            let start_page = (addr_usize / page_size) * page_size;
            let mut pages: Vec<PageProtect> = Vec::new();

            let mut page = start_page;
            // 逐页设置保护并保存原值
            while page < end {
                // 设置长度为 page_size（VirtualProtect 对齐到页面），
                // 对最后一页也用 page_size 是安全的（系统按页处理）
                let mut old: u32 = 0;
                let ok = VirtualProtect(page as _, page_size as _, new_protect as _, &raw mut old);
                if ok == 0 {
                    // 出错：尝试回滚已经成功修改过的页面（尽量恢复）
                    let mut _tmp: u32 = 0;
                    for p in &pages {
                        let _ =
                            VirtualProtect(p.base as _, p.size as _, p.protect as _, &raw mut _tmp);
                    }
                    crate::bail!("VirtualProtect failed for page {:p}", page as *const u8);
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
                address,
                size,
                pages,
            })
        }
    }

    /// 获取原始地址
    pub const fn address(&self) -> *mut u8 {
        self.address
    }

    /// 获取内存区域大小
    pub const fn size(&self) -> usize {
        self.size
    }

    /// 写入字节切片到受保护的内存
    ///
    /// # 参数
    /// - `data`: 要写入的字节切片
    ///
    /// # Safety
    /// 调用者必须确保切片长度不超过保护范围
    pub unsafe fn patch_bytes(&mut self, data: &[u8]) {
        unsafe { self.patch_bytes_ex(0, data, false) }
    }

    /// 在指定偏移量处写入字节切片
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `data`: 要写入的字节切片
    ///
    /// # Safety
    /// 调用者必须确保切片长度不超过保护范围
    pub unsafe fn patch_bytes_offset(&mut self, offset: usize, data: &[u8]) {
        unsafe { self.patch_bytes_ex(offset, data, false) }
    }

    /// 写入字节切片到受保护的内存，然后刷新指令缓存
    ///
    /// # 参数
    /// - `data`: 要写入的字节切片
    ///
    /// # Safety
    /// 调用者必须确保切片长度不超过保护范围
    pub unsafe fn patch_asm_bytes(&mut self, data: &[u8]) {
        unsafe { self.patch_bytes_ex(0, data, true) }
    }

    /// 在指定偏移量处写入字节切片，然后刷新指令缓存
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `data`: 要写入的字节切片
    ///
    /// # Safety
    /// 调用者必须确保切片长度不超过保护范围
    pub unsafe fn patch_asm_bytes_offset(&mut self, offset: usize, data: &[u8]) {
        unsafe { self.patch_bytes_ex(offset, data, true) }
    }

    /// 在指定偏移量处写入字节切片
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `data`: 要写入的字节切片
    /// - `asm`: 若为true，则在写入后会刷新指令缓存
    ///
    /// # Safety
    /// - 调用者必须确保 `offset + data.len()` 不超过保护范围。
    /// - 当 `asm` 为 `true` 时，调用者需保证写入目标为可执行代码并允许刷新指令缓存。
    pub unsafe fn patch_bytes_ex(&mut self, offset: usize, data: &[u8], asm: bool) {
        if data.is_empty() {
            return;
        }

        let len = data.len();
        self.assert_in_bound(offset, len);

        unsafe {
            let target_addr = self.address.add(offset);
            target_addr.copy_from(data.as_ptr(), len);

            if asm {
                flush_icache(target_addr, len);
            }
        }
    }

    /// 使用特定字节填充受保护的内存
    ///
    /// # 参数
    /// - `value`: 要填充的字节值
    /// - `count`: 填充长度（字节）
    ///
    /// # Safety
    /// 调用者必须确保填充范围在保护范围内
    pub unsafe fn patch_repeated_bytes(&mut self, value: u8, count: usize) {
        unsafe { self.patch_repeated_bytes_ex(0, value, count, false) }
    }

    /// 在指定偏移量处使用特定字节填充内存
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `value`: 要填充的字节值
    /// - `count`: 填充长度（字节）
    ///
    /// # Safety
    /// 调用者必须确保填充范围在保护范围内
    pub unsafe fn patch_repeated_bytes_offset(&mut self, offset: usize, value: u8, count: usize) {
        unsafe { self.patch_repeated_bytes_ex(offset, value, count, false) }
    }

    /// 使用特定字节填充受保护的内存，然后刷新指令缓存
    ///
    /// # 参数
    /// - `value`: 要填充的字节值
    /// - `count`: 填充长度（字节）
    ///
    /// # Safety
    /// 调用者必须确保填充范围在保护范围内
    pub unsafe fn patch_repeated_asm_bytes(&mut self, value: u8, count: usize) {
        unsafe { self.patch_repeated_bytes_ex(0, value, count, true) }
    }

    /// 在指定偏移量处使用特定字节填充内存，然后刷新指令缓存
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `value`: 要填充的字节值
    /// - `count`: 填充长度（字节）
    ///
    /// # Safety
    /// 调用者必须确保填充范围在保护范围内
    pub unsafe fn patch_repeated_asm_bytes_offset(
        &mut self,
        offset: usize,
        value: u8,
        count: usize,
    ) {
        unsafe { self.patch_repeated_bytes_ex(offset, value, count, true) }
    }

    /// 在指定偏移量处使用特定字节填充内存
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `value`: 要填充的字节值
    /// - `count`: 填充长度（字节）
    /// - `asm`: 若为true，则在填充后会刷新指令缓存
    ///
    /// # Safety
    /// - 调用者必须确保 `offset + count` 不超过保护范围。
    /// - 当 `asm` 为 `true` 时，调用者需保证写入目标为可执行代码并允许刷新指令缓存。
    pub unsafe fn patch_repeated_bytes_ex(
        &mut self,
        offset: usize,
        value: u8,
        count: usize,
        asm: bool,
    ) {
        if count == 0 {
            return;
        }

        self.assert_in_bound(offset, count);

        unsafe {
            let target_addr = self.address.add(offset);
            target_addr.write_bytes(value, count);

            if asm {
                flush_icache(target_addr, count);
            }
        }
    }

    /// 从受保护的内存复制字节到缓冲区
    ///
    /// # 参数
    /// - `buffer`: 用于存储读取数据的缓冲区
    ///
    /// # Safety
    /// 调用者必须确保缓冲区有效
    pub unsafe fn copy_bytes_to(&self, buffer: &mut [u8]) {
        unsafe { self.copy_bytes_offset_to(0, buffer) }
    }

    /// 从指定偏移量处复制字节到缓冲区
    ///
    /// # 参数
    /// - `offset`: 字节偏移量
    /// - `buffer`: 用于存储读取数据的缓冲区
    ///
    /// # Safety
    /// 调用者必须确保偏移量和缓冲区有效
    pub unsafe fn copy_bytes_offset_to(&self, offset: usize, buffer: &mut [u8]) {
        if buffer.is_empty() {
            return;
        }

        let len = buffer.len();
        self.assert_in_bound(offset, len);

        unsafe {
            let source_addr = self.address.add(offset);
            buffer.as_mut_ptr().copy_from(source_addr, len);
        }
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
                let _ok = VirtualProtect(p.base as _, p.size as _, p.protect as _, &raw mut _tmp);

                #[cfg(feature = "enable_debug_output")]
                if _ok == 0 {
                    crate::print_last_error_message!();
                    crate::debug!("VirtualProtect restore failed for {:p}", p.base);
                }
            }
        }
    }
}
