#![crate_type = "cdylib"]

pub use error_handling::{Error, Result};

pub(crate) mod debug_output;
pub(crate) mod hook;

#[allow(dead_code)]
pub(crate) mod code_cvt;
#[allow(dead_code)]
pub(crate) mod error_handling;
#[allow(dead_code)]
pub(crate) mod mapping;
#[allow(dead_code)]
pub(crate) mod utils;

#[cfg(feature = "x64dbg_1337_patch")]
pub(crate) mod x64dbg_1337_patch;

#[cfg(feature = "text_patch")]
pub(crate) mod text_patch;

#[cfg(feature = "patch")]
pub(crate) mod patch;

#[cfg(feature = "custom_font")]
pub(crate) mod custom_font;

#[cfg(feature = "delayed_attach")]
pub(crate) mod delayed_attach;

#[cfg(feature = "dll_hijacking")]
pub(crate) mod dll_hijacking;

#[cfg(feature = "emulate_locale")]
pub(crate) mod emulate_locale;

#[allow(dead_code)]
pub(crate) mod constant {
    pub const ANSI_CODE_PAGE: u32 = crate::mapping::ANSI_CODE_PAGE;

    translate_macros::generate_constants_from_json!(
        "constant_assets/default_config.json",
        "assets/config.json"
    );
}
