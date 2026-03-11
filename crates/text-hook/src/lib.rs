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

#[cfg(feature = "x64dbg_1337_patch")]
pub(crate) mod x64dbg_1337_patch;

#[cfg(feature = "text_patch")]
pub(crate) mod text_patch;

#[cfg(feature = "win_event_hook")]
pub(crate) mod win_event_hook;

#[cfg(feature = "patch")]
pub(crate) mod patch;

#[cfg(feature = "custom_font")]
pub(crate) mod custom_font;

#[cfg(feature = "delayed_attach")]
pub(crate) mod delayed_attach;

#[cfg(feature = "dll_hijacking")]
pub(crate) mod dll_hijacking;

#[cfg(feature = "locale_emulator")]
pub(crate) mod locale_emulator;

#[cfg(feature = "resource_pack")]
pub(crate) mod resource_pack;

#[cfg(feature = "veh")]
pub(crate) mod veh;

#[cfg(feature = "overlay")]
pub(crate) mod overlay;

#[cfg(feature = "worker_thread")]
pub(crate) mod worker_thread;

#[allow(dead_code)]
pub(crate) mod constant {
    pub const ANSI_CODE_PAGE: u32 = crate::code_cvt::ANSI_CODE_PAGE;

    translate_macros::generate_constants_from_json!(
        "constant_assets/default_config.json",
        "assets/config.json"
    );
}
