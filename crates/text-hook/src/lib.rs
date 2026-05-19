#![crate_type = "cdylib"]

pub use utils::error_handling::{Error, Result};

pub(crate) mod feature_conflicts;

pub(crate) mod debug_output;
pub(crate) mod hook;

#[allow(dead_code)]
pub(crate) mod code_cvt;
#[allow(dead_code)]
pub(crate) mod gl;
#[allow(dead_code)]
pub(crate) mod utils;

#[cfg(feature = "enable_x64dbg_1337_patch")]
pub(crate) mod x64dbg_1337_patch;

#[cfg(feature = "enable_text_patch")]
pub(crate) mod text_patch;

#[cfg(feature = "enable_win_event_hook")]
pub(crate) mod win_event_hook;

#[cfg(feature = "enable_patch")]
pub(crate) mod patch;

#[cfg(feature = "enable_embedded_font")]
pub(crate) mod embedded_font;

#[cfg(feature = "enable_custom_font")]
pub(crate) mod custom_font;

#[cfg(feature = "enable_delayed_attach")]
pub(crate) mod delayed_attach;

#[cfg(feature = "enable_dll_hijacking")]
pub(crate) mod dll_hijacking;

#[cfg(feature = "enable_locale_emulator")]
pub(crate) mod locale_emulator;

#[cfg(feature = "enable_resource_pack")]
pub(crate) mod resource_pack;

#[cfg(feature = "enable_vfs")]
pub(crate) mod vfs;

#[cfg(feature = "enable_storage")]
pub(crate) mod storage;

#[cfg(feature = "enable_veh")]
pub(crate) mod veh;

#[cfg(feature = "enable_overlay")]
pub(crate) mod overlay;

#[cfg(feature = "enable_thread_manager")]
pub(crate) mod thread_manager;

#[cfg(feature = "enable_ui_thread")]
pub(crate) mod ui_thread;

#[allow(dead_code)]
pub(crate) mod constant {
    pub const ANSI_CODE_PAGE: u32 = crate::code_cvt::ANSI_CODE_PAGE;

    translate_macros::generate_constants_from_json!(
        "constant_assets/default_config.json",
        "assets/config.json"
    );
}
