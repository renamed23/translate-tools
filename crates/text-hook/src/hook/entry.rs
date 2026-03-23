use windows_sys::{
    Win32::Foundation::{FALSE, HMODULE, TRUE},
    core::BOOL,
};

use crate::hook::core_hook::CoreHook;

#[cfg(feature = "export_default_dll_main")]
#[translate_macros::ffi_guard(on_err_or_panic = FALSE)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut core::ffi::c_void,
) -> crate::Result<BOOL> {
    default_dll_main(_hinst_dll, fdw_reason, _lpv_reserved)
}

/// 默认的 DllMain 实现
pub fn default_dll_main(
    hinst_dll: HMODULE,
    fdw_reason: u32,
    lpv_reserved: *mut core::ffi::c_void,
) -> crate::Result<BOOL> {
    const PROCESS_ATTACH: u32 = 1;
    const PROCESS_DETACH: u32 = 0;
    const _THREAD_ATTACH: u32 = 2;
    const _THREAD_DETACH: u32 = 3;

    match fdw_reason {
        PROCESS_ATTACH => {
            crate::debug!("Process attach");

            crate::utils::panic::set_debug_panic_hook();

            #[cfg(all(feature = "enable_text_patch", feature = "extract_text"))]
            if let Err(e) = crate::text_patch::load_initial_extracted_items_from_json() {
                crate::debug!("Failed to load initial extracted items from JSON: {e:?}");
            }

            #[cfg(feature = "enable_veh")]
            if let Err(e) = unsafe { crate::veh::install_veh_handler(true) } {
                crate::debug!("Install VEH handler failed with {e:?}");
            }

            #[cfg(feature = "enable_locale_emulator")]
            if let Err(e) = crate::locale_emulator::relaunch_with_locale_emulator() {
                crate::debug!("Relaunch with Locale Emulator failed with {e:?}");
            }

            #[cfg(feature = "enable_custom_font")]
            if let Err(e) = unsafe { crate::custom_font::add_font() } {
                crate::debug!("add_font failed with {e:?}");
            }

            #[cfg(feature = "enable_resource_pack")]
            if let Err(e) = crate::resource_pack::extract() {
                crate::debug!("Extract resource pack failed with {e:?}");
            }

            #[cfg(feature = "enable_worker_thread")]
            if let Err(e) = unsafe { crate::worker_thread::start() } {
                crate::debug!("Start worker thread failed with {e:?}");
            }

            #[cfg(feature = "auto_apply_1337_patch_on_attach")]
            if let Err(e) = crate::x64dbg_1337_patch::apply() {
                crate::debug!("Apply 1337 patch failed with {e:?}");
            }

            #[cfg(not(feature = "enable_delayed_attach"))]
            crate::hook::enable_hooks_from_lists();

            #[cfg(feature = "enable_delayed_attach")]
            crate::delayed_attach::enable_entry_point_hook();

            if let Err(e) = crate::hook::impls::HookImplType::on_process_attach(hinst_dll) {
                crate::debug!("on_process_attach failed with {e:?}");
            }
        }
        PROCESS_DETACH => {
            crate::debug!("Process detach");

            if let Err(e) = crate::hook::impls::HookImplType::on_process_detach(
                hinst_dll,
                !lpv_reserved.is_null(),
            ) {
                crate::debug!("on_process_detach failed with {e:?}");
            }
        }
        _ => {}
    }

    Ok(TRUE)
}

/// 对应于 PROCESS_ATTACH 阶段的清理操作。
/// DllMain PROCESS_DETACH 阶段执行该清理操作非常危险。
///
/// 进程退出时，进程内资源是可以被系统正确处理回收。
/// 如果一定需要进行处理，那么应该HOOK ExitProcess，FreeLibrary相关函数，再调用该函数。
#[cfg(feature = "enable_attach_cleanup")]
pub fn attach_cleanup() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static CLEANUP: AtomicBool = AtomicBool::new(false);

    if CLEANUP.swap(true, Ordering::AcqRel) {
        return;
    }

    crate::debug!("Process attach clean up");

    #[cfg(all(feature = "enable_text_patch", feature = "extract_text"))]
    if let Err(e) = crate::text_patch::save_extracted_items_to_json() {
        crate::debug!("Failed to save extracted items to JSON: {e:?}");
    }

    #[cfg(feature = "enable_resource_pack")]
    if let Err(e) = crate::resource_pack::cleanup() {
        crate::debug!("Clean up resource pack failed with {e:?}");
    }

    #[cfg(feature = "enable_worker_thread")]
    if let Err(e) = unsafe { crate::worker_thread::stop() } {
        crate::debug!("Stop worker thread failed with {e:?}");
    }

    #[cfg(feature = "enable_dll_hijacking")]
    unsafe {
        crate::dll_hijacking::unload_library();
    };

    #[cfg(feature = "enable_veh")]
    if let Err(e) = unsafe { crate::veh::uninstall_veh_handler() } {
        crate::debug!("Uninstall VEH handler failed with {e:?}");
    }

    #[cfg(feature = "enable_custom_font")]
    if let Err(e) = unsafe { crate::custom_font::remove_font() } {
        crate::debug!("remove_font failed with {e:?}");
    }

    crate::hook::disable_hooks_from_lists();

    if let Err(e) = crate::hook::impls::HookImplType::on_process_attach_cleanup() {
        crate::debug!("on_process_attach_cleanup failed with {e:?}");
    }

    #[cfg(feature = "enable_delayed_attach")]
    crate::delayed_attach::disable_entry_point_hook();
}
