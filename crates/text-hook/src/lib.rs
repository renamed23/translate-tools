#![crate_type = "cdylib"]

pub(crate) mod code_cvt;
pub(crate) mod debug_output;
pub(crate) mod hook;
pub(crate) mod hook_impl;
pub(crate) mod mapping;

#[allow(dead_code)]
pub(crate) mod utils;

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
    translate_macros::generate_constants_from_json!(
        "constant_assets/default_config.json",
        "assets/config.json"
    );
}
