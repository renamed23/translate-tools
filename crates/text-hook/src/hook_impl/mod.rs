#[cfg(feature = "snow_radish")]
pub mod snow_radish;

#[cfg(feature = "bleed")]
pub mod bleed;

#[cfg(feature = "default_hook_impl")]
pub mod default_hook_impl;

#[cfg(feature = "sukisuki")]
pub mod sukisuki;

#[cfg(feature = "ao_vo")]
pub mod ao_vo;

#[cfg(feature = "noise")]
pub mod noise;

#[cfg(feature = "lusts")]
pub mod lusts;

#[cfg(feature = "summer_radish")]
pub mod summer_radish;

#[cfg(feature = "c4")]
pub mod c4;

#[cfg(feature = "debug_file_hook_impl")]
pub mod debug_file_hook_impl;

// ---------------------- 钩子实现类型 ------------------------------

#[cfg(feature = "default_hook_impl")]
pub type HookImplType = default_hook_impl::DefaultHook;

#[cfg(feature = "bleed")]
pub type HookImplType = bleed::BleedHook;

#[cfg(feature = "debug_file_hook_impl")]
pub type HookImplType = debug_file_hook_impl::DebugFileHook;

#[cfg(feature = "snow_radish")]
pub type HookImplType = snow_radish::SnowRadishHook;
