#![crate_type = "cdylib"]

pub(crate) mod code_cvt;
pub(crate) mod debug_output;
pub(crate) mod hook;
pub(crate) mod hook_impl;
pub(crate) mod hook_utils;
pub(crate) mod mapping;
pub(crate) mod panic_utils;
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

#[allow(dead_code)]
pub(crate) mod constant {
    translate_macros::generate_constants_from_json!(
        "constant_assets/default_config.json",
        "assets/config.json"
    );
}
