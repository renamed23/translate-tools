use std::sync::atomic::{AtomicBool, Ordering};

use cfg_if::cfg_if;

use crate::{
    debug,
    hook::{impls::HookImplType, internal_hooks::DelayedAttach},
};

cfg_if! {
    if #[cfg(feature = "enable_delayed_attach_static")] {
        translate_macros::generate_entry_point_hook!(
            exe_dir = "assets/exe",
            config_path = "assets/config.json",
            handler_fn = entry_point
        );
    } else {
        static HOOK_ENTRY_POINT: std::sync::LazyLock<retour::GenericDetour<unsafe extern "C" fn()>> =
            std::sync::LazyLock::new(|| unsafe {
                let entry_point_addr = crate::constant::ENTRY_POINT_RVA.unwrap_or_else(|| {
                    crate::utils::mem::patch::get_entry_point_addr().expect("Get entry point addr failed")
                });

                // 检测是否有入口断点，一般用x32dbg之类的调试器都会有，打印出警告
                #[cfg(feature = "enable_debug_output")]
                if (entry_point_addr as *const u8).read_unaligned() == 0xCC {
                    debug!("Warning: detect `INT3` at entry point");
                }

                let resolved = crate::utils::mem::patch::resolve_patchable_addr(entry_point_addr)
                    .expect("Resolve patchable entry point addr failed");
                let ori_entry: unsafe extern "C" fn() = core::mem::transmute(resolved);

                retour::GenericDetour::new(ori_entry, entry_point)
                    .expect("Failed to create detour for EntryPoint")
            });
    }
}

fn delayed_attach() {
    debug!("Delayed attach start...");

    #[cfg(feature = "enable_dll_hijacking")]
    unsafe {
        crate::dll_hijacking::load_library();
    };

    #[cfg(feature = "enable_hwbp_from_constants")]
    if let Ok(addr) = crate::utils::win32::get_module_handle(crate::constant::HWBP_MODULE) {
        let target_addr = addr as usize + crate::constant::HWBP_RVA;

        crate::veh::request_set_hw_breakpoint_on_current_thread(
            target_addr,
            crate::constant::HWBP_TYPE,
            crate::constant::HWBP_LEN,
            crate::constant::HWBP_REG,
        );
    }

    crate::hook::enable_hooks_from_lists();
    if let Err(e) = <HookImplType as DelayedAttach>::on_delayed_attach() {
        crate::debug!("on_delayed_attach failed with {e:?}");
    }
}

fn delayed_attach_clean() {
    debug!("Delayed attach clean start...");

    #[cfg(feature = "enable_dll_hijacking")]
    unsafe {
        crate::dll_hijacking::unload_library();
    };
}

unsafe extern "C" fn entry_point() {
    static ATTACHED: AtomicBool = AtomicBool::new(false);

    // 只执行一次`delayed_attach`
    if !ATTACHED.swap(true, Ordering::AcqRel) {
        delayed_attach();
    }

    #[cfg(not(feature = "enable_delayed_attach_static"))]
    unsafe {
        HOOK_ENTRY_POINT.call();
    };
}

/// 启用入口点钩子
///
/// 安装对程序主入口点的钩子，当入口点被调用时会执行延迟初始化操作。
/// 这允许在程序完成基本初始化后进行安全的附加操作。
pub fn enable_entry_point_hook() -> crate::Result<()> {
    cfg_if! {
        if #[cfg(feature = "enable_delayed_attach_static")] {
            entry_point_init()?;
        } else {
            unsafe {
                HOOK_ENTRY_POINT.enable()?;
            };
        }
    }

    Ok(())
}

/// 禁用入口点钩子
///
/// 禁用入口点钩子，恢复原始的执行流程，并清理延迟初始化相关的资源。
/// 这个函数应该在 `DllMain` 的 `PROCESS_DETACH` 分支中调用。
pub fn disable_entry_point_hook() -> crate::Result<()> {
    delayed_attach_clean();

    #[cfg(not(feature = "enable_delayed_attach_static"))]
    unsafe {
        HOOK_ENTRY_POINT.disable()?;
    };

    Ok(())
}
