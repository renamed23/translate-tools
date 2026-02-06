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
            crate::text_patch::read_extracted_items_from_json();

            #[cfg(feature = "emulate_locale")]
            crate::emulate_locale::set_japanese_locale();

            #[cfg(feature = "custom_font")]
            unsafe {
                if crate::custom_font::add_font().is_err() {
                    crate::debug!("add_font failed");
                }
            }

            #[cfg(feature = "apply_1337_patch_on_attach")]
            if crate::x64dbg_1337_patch::apply().is_err() {
                crate::debug!("Apply 1337 patch failed");
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
            crate::text_patch::write_extracted_items_to_json();

            #[cfg(feature = "custom_font")]
            unsafe {
                if crate::custom_font::remove_font().is_err() {
                    crate::debug!("remove_font failed");
                }
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
