use std::{marker::PhantomData, sync::atomic::AtomicBool};

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::Diagnostics::Debug::IMAGE_DIRECTORY_ENTRY_IMPORT;
use windows_sys::Win32::System::SystemServices::IMAGE_IMPORT_DESCRIPTOR;

use crate::utils::mem::patch::{get_dos_and_nt_headers, write_bytes};

/// IAT 修补函数
///
/// # 参数
/// - `target_module`: 目标模块基址 (HMODULE)
/// - `original_addr`: 函数当前的真实地址（IAT 中存储的旧值）
/// - `hook_addr`: 准备替换进去的 Hook 函数地址
///
/// # 返回值
/// - `Result`: 成功返回 `Ok(())`，失败返回错误信息
pub unsafe fn patch_iat(
    target_module: HMODULE,
    original_addr: usize,
    hook_addr: usize,
) -> crate::Result<()> {
    unsafe {
        let iat_entry_ptr = find_iat_entry(target_module, original_addr)?;
        write_bytes(iat_entry_ptr as _, &hook_addr.to_ne_bytes())?;
        Ok(())
    }
}

/// 在导入表中遍历查找特定地址的指针位置
pub unsafe fn find_iat_entry(module: HMODULE, target_ptr: usize) -> crate::Result<*mut usize> {
    unsafe {
        let base = module as usize;
        let (_, nt) = get_dos_and_nt_headers(base)?;

        let import_dir = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT as usize];
        if import_dir.VirtualAddress == 0 {
            crate::bail!("Target module has no import directory");
        }

        let mut imp = (base + import_dir.VirtualAddress as usize) as *const IMAGE_IMPORT_DESCRIPTOR;

        // 遍历所有导入的 DLL
        while (*imp).Name != 0 {
            // FirstThunk 指向 IAT (Import Address Table)
            let first_thunk = (*imp).FirstThunk;
            if first_thunk != 0 {
                let mut thunk = (base + first_thunk as usize) as *mut usize;

                // 遍历该 DLL 下所有的导出函数地址
                while *thunk != 0 {
                    if *thunk == target_ptr {
                        return Ok(thunk);
                    }
                    thunk = thunk.add(1);
                }
            }
            imp = imp.add(1);
        }

        crate::bail!("Could not find target address 0x{target_ptr:X} in IAT");
    }
}

/// IAT（导入地址表）钩子管理器
///
/// 通过修改 PE 导入表中的函数指针实现 API Hook。
/// 线程安全：启用/禁用操作原子化，但非并发安全（需外部同步）
pub struct IatHook<T: Copy + 'static + Sized> {
    orig: usize,
    hook: usize,
    enabled: AtomicBool,
    _marker: PhantomData<T>,
}

impl<T> IatHook<T>
where
    T: Copy + Sized + 'static,
{
    /// 创建未激活的钩子
    pub const fn new(orig: usize, hook: usize) -> Self {
        Self {
            orig,
            hook,
            enabled: AtomicBool::new(false),
            _marker: PhantomData,
        }
    }

    /// 当前是否已启用
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 启用钩子：将 IAT 中 orig 替换为 hook
    /// # Safety
    /// - 非线程安全，避免并发启用/禁用
    pub unsafe fn enable(&self) -> crate::Result<()> {
        if self.is_enabled() {
            return Ok(());
        }

        let module = crate::utils::win32::get_module_handle(core::ptr::null())? as HMODULE;

        unsafe { patch_iat(module, self.orig, self.hook) }?;
        self.enabled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// 禁用钩子：恢复原始地址
    /// # Safety
    /// - 非线程安全，避免并发启用/禁用
    pub unsafe fn disable(&self) -> crate::Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let module = crate::utils::win32::get_module_handle(core::ptr::null())? as HMODULE;

        unsafe { patch_iat(module, self.hook, self.orig) }?;
        self.enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// 获取原始函数指针（用于在 Hook 中调用原函数）
    pub fn orig(&self) -> T {
        const {
            assert!(
                std::mem::size_of::<T>() == std::mem::size_of::<usize>(),
                "IatHook 类型 T 必须和指针相同大小"
            );
        }
        unsafe { core::mem::transmute_copy(&self.orig) }
    }
}
