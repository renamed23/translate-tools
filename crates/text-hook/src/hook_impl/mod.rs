#[allow(unused_imports)]
use windows_sys::{
    Win32::Foundation::{FALSE, HMODULE, TRUE},
    core::BOOL,
};

// 声明所有的Hook实现的模块文件
translate_macros::expand_by_files!("src/hook_impl" => {
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
translate_macros::search_hook_impls!("src/hook_impl" => pub type HookImplType);

/// 默认的 DllMain 实现
#[allow(dead_code)]
pub fn default_dll_main(
    hinst_dll: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut core::ffi::c_void,
) -> BOOL {
    use crate::hook::CoreHook;

    const PROCESS_ATTACH: u32 = 1;
    const PROCESS_DETACH: u32 = 0;
    const THREAD_ATTACH: u32 = 2;
    const THREAD_DETACH: u32 = 3;

    match fdw_reason {
        PROCESS_ATTACH => {
            crate::panic_utils::set_debug_panic_hook();

            crate::hook::set_hook_instance(HookImplType::default());

            #[cfg(feature = "custom_font")]
            unsafe {
                crate::custom_font::add_font();
            }

            crate::hook::hook_instance().enable_hooks();

            crate::hook::enable_featured_hooks();

            #[cfg(feature = "delayed_attach")]
            crate::delayed_attach::enable_entry_point_hook();

            crate::hook::hook_instance().on_process_attach(hinst_dll);
        }
        PROCESS_DETACH => {
            #[cfg(feature = "custom_font")]
            unsafe {
                crate::custom_font::remove_font();
            }

            crate::hook::hook_instance().disable_hooks();

            crate::hook::disable_featured_hooks();

            #[cfg(feature = "delayed_attach")]
            crate::delayed_attach::disable_entry_point_hook();

            crate::hook::hook_instance().on_process_detach(hinst_dll);
        }
        _ => {}
    }

    TRUE
}
