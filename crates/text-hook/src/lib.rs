#![crate_type = "cdylib"]

#[allow(dead_code)]
pub(crate) mod constant;

pub(crate) mod code_cvt;
pub(crate) mod debug_output;
pub(crate) mod hook;
pub(crate) mod hook_impl;
pub(crate) mod hook_utils;
pub(crate) mod mapping;
pub(crate) mod panic_utils;
pub(crate) mod utils;

#[cfg(feature = "embedded_dict")]
pub(crate) mod embedded_dict;

#[cfg(feature = "patch")]
pub(crate) mod patch;

#[cfg(feature = "custom_font")]
pub(crate) mod custom_font;
