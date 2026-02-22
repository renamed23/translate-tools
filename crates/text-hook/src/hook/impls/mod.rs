#[allow(unused_imports)]
use windows_sys::{
    Win32::Foundation::{FALSE, HMODULE, TRUE},
    core::BOOL,
};

// 声明所有的Hook实现的模块文件
translate_macros::expand_by_files!("src/hook/impls" => {
    #[cfg(feature = __file_str__)]
    pub mod __file__;
});

#[cfg(feature = "export_default_dll_main")]
#[translate_macros::ffi_catch_unwind(FALSE)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut core::ffi::c_void,
) -> BOOL {
    default_dll_main(_hinst_dll, fdw_reason, _lpv_reserved)
}

// 在`src/hook_impl`搜索可用的Hook实现类型
translate_macros::search_hook_impls!("src/hook/impls" => pub type HookImplType);

/// 默认的 DllMain 实现
#[allow(dead_code, unused_variables)]
pub fn default_dll_main(
    hinst_dll: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut core::ffi::c_void,
) -> BOOL {
    use crate::hook::traits::CoreHook;

    const PROCESS_ATTACH: u32 = 1;
    const PROCESS_DETACH: u32 = 0;
    const THREAD_ATTACH: u32 = 2;
    const THREAD_DETACH: u32 = 3;

    match fdw_reason {
        PROCESS_ATTACH => {
            crate::utils::panic::set_debug_panic_hook();

            #[cfg(all(feature = "text_patch", feature = "text_extracting"))]
            crate::text_patch::load_initial_extracted_items_from_json();

            #[cfg(feature = "veh")]
            if let Err(e) = unsafe { crate::veh::install_veh_handler(true) } {
                crate::debug!("Install VEH handler failed with {e:?}");
            }

            #[cfg(feature = "locale_emulator")]
            if let Err(e) = crate::locale_emulator::relaunch_with_locale_emulator() {
                crate::debug!("Relaunch with Locale Emulator failed with {e:?}");
            }

            #[cfg(feature = "custom_font")]
            if let Err(e) = unsafe { crate::custom_font::add_font() } {
                crate::debug!("add_font failed with {e:?}");
            }

            #[cfg(feature = "resource_pack")]
            if let Err(e) = crate::resource_pack::extract() {
                crate::debug!("Extract resource pack failed with {e:?}");
            }

            #[cfg(feature = "apply_1337_patch_on_attach")]
            if let Err(e) = crate::x64dbg_1337_patch::apply() {
                crate::debug!("Apply 1337 patch failed with {e:?}");
            }

            #[cfg(not(feature = "delayed_attach"))]
            {
                HookImplType::enable_hooks();
                crate::hook::enable_hooks_from_lists();
            }

            #[cfg(feature = "delayed_attach")]
            crate::delayed_attach::enable_entry_point_hook();

            HookImplType::on_process_attach(hinst_dll);
        }
        PROCESS_DETACH => {
            #[cfg(all(feature = "text_patch", feature = "text_extracting"))]
            crate::text_patch::save_extracted_items_to_json();

            #[cfg(feature = "veh")]
            if let Err(e) = unsafe { crate::veh::uninstall_veh_handler() } {
                crate::debug!("Uninstall VEH handler failed with {e:?}");
            }

            #[cfg(feature = "custom_font")]
            if let Err(e) = unsafe { crate::custom_font::remove_font() } {
                crate::debug!("remove_font failed with {e:?}");
            }

            #[cfg(feature = "resource_pack")]
            if let Err(e) = crate::resource_pack::clean_up() {
                crate::debug!("Clean up resource pack failed with {e:?}");
            }

            HookImplType::disable_hooks();
            crate::hook::disable_hooks_from_lists();

            #[cfg(feature = "delayed_attach")]
            crate::delayed_attach::disable_entry_point_hook();

            HookImplType::on_process_detach(hinst_dll);
        }
        _ => {}
    }

    TRUE
}
