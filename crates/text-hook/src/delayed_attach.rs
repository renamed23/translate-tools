use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use winapi::um::{
    processthreadsapi::GetCurrentProcess,
    psapi::{GetModuleInformation, MODULEINFO},
};

use crate::{debug, hook::CoreHook, print_system_error_message};

static ENTRY_POINT_ADDR: Lazy<usize> = Lazy::new(|| {
    let hmod = crate::hook_utils::get_module_handle("").unwrap();

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

static HOOK_ENTRY_POINT: Lazy<retour::GenericDetour<unsafe extern "C" fn()>> = Lazy::new(|| {
    let ori_entry: unsafe extern "C" fn() = unsafe { core::mem::transmute(*ENTRY_POINT_ADDR) };

    unsafe {
        retour::GenericDetour::new(ori_entry, entry_point)
            .expect("Failed to create detour for EntryPoint")
    }
});

fn delayed_attach() {
    debug!("Delayed attach start...");
    crate::hook::hook_instance().on_delayed_attach();
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

/// 启用延迟附加钩子
///
/// 在程序入口点被调用时执行延迟附加
pub fn enable_delayed_attach_hook() {
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

/// 禁用延迟附加钩子
///
/// 此函数会禁用之前启用的入口点钩子，恢复原始的执行流程。
/// 通常在卸载或清理阶段调用。
pub fn disable_delayed_attach_hook() {
    unsafe { HOOK_ENTRY_POINT.disable().unwrap() };
}
