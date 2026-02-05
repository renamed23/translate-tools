use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::System::{
    ProcessStatus::{GetModuleInformation, MODULEINFO},
    Threading::GetCurrentProcess,
};

use crate::{debug, hook::traits::CoreHook, print_system_error_message};

static ENTRY_POINT_ADDR: Lazy<usize> = Lazy::new(|| {
    let hmod = crate::utils::win32::get_module_handle("").unwrap();

    let mut mi = MODULEINFO {
        lpBaseOfDll: core::ptr::null_mut(),
        SizeOfImage: 0,
        EntryPoint: core::ptr::null_mut(),
    };

    unsafe {
        let ok = GetModuleInformation(
            GetCurrentProcess(),
            hmod,
            &mut mi as *mut _,
            std::mem::size_of::<MODULEINFO>() as u32,
        );

        if ok == 0 {
            print_system_error_message!();
            panic!("GetModuleInformation failed");
        }
    };

    mi.EntryPoint as usize
});

static HOOK_ENTRY_POINT: Lazy<retour::GenericDetour<unsafe extern "C" fn()>> =
    Lazy::new(|| unsafe {
        let resolved = crate::utils::mem::patch::resolve_patchable_addr_32(*ENTRY_POINT_ADDR);
        let ori_entry: unsafe extern "C" fn() = core::mem::transmute(resolved);

        retour::GenericDetour::new(ori_entry, entry_point)
            .expect("Failed to create detour for EntryPoint")
    });

fn delayed_attach() {
    debug!("Delayed attach start...");

    #[cfg(feature = "dll_hijacking")]
    unsafe {
        crate::dll_hijacking::load_library()
    };

    crate::hook::impls::HookImplType::enable_hooks();
    crate::hook::enable_hooks_from_lists();
    crate::hook::impls::HookImplType::on_delayed_attach();
}

fn delayed_attach_clean() {
    debug!("Delayed attach clean start...");

    #[cfg(feature = "dll_hijacking")]
    unsafe {
        crate::dll_hijacking::unload_library()
    };

    crate::hook::impls::HookImplType::on_delayed_attach_clean();
}

unsafe extern "C" fn entry_point() {
    static ATTACHED: AtomicBool = AtomicBool::new(false);

    // 只执行一次`delayed_attach`
    if ATTACHED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        delayed_attach();
    }

    unsafe {
        HOOK_ENTRY_POINT.call();
    };
}

/// 启用入口点钩子
///
/// 安装对程序主入口点的钩子，当入口点被调用时会执行延迟初始化操作。
/// 这允许在程序完成基本初始化后进行安全的附加操作。
pub fn enable_entry_point_hook() {
    // 检测是否有入口断点，一般用x32dbg之类的调试器都会有，打印出警告
    #[cfg(feature = "debug_output")]
    {
        let entry_point_addr = (*ENTRY_POINT_ADDR) as *const u8;
        if unsafe { entry_point_addr.read_unaligned() } == 0xCC {
            debug!("Warning: detect `INT3` at entry point");
        }
    }

    unsafe { HOOK_ENTRY_POINT.enable().unwrap() };
}

/// 禁用入口点钩子
///
/// 禁用入口点钩子，恢复原始的执行流程，并清理延迟初始化相关的资源。
/// 这个函数应该在 `DllMain` 的 `PROCESS_DETACH` 分支中调用。
pub fn disable_entry_point_hook() {
    delayed_attach_clean();
    unsafe { HOOK_ENTRY_POINT.disable().unwrap() };
}
