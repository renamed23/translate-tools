macro_rules! syn_bail {
    // 使用 token 的 span（自动提取）
    ($token:expr, $($arg:tt)*) => {
        return Err(syn::Error::new_spanned(
            $token,
            format!($($arg)*)
        ))
    };
}

macro_rules! syn_bail2 {
    // 使用 call_site span（宏调用位置）
    ($($arg:tt)*) => {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!($($arg)*)
        ))
    };
}

pub(crate) mod byte_slice;
// pub(crate) mod detour;
pub(crate) mod expand_by_files;
pub(crate) mod ffi_catch_unwind;
pub(crate) mod flate;
pub(crate) mod generate_constants_from_json;
pub(crate) mod generate_detours;
pub(crate) mod generate_exports_from_hijacked_dll;
pub(crate) mod generate_mapping_data;
pub(crate) mod generate_patch_data;
pub(crate) mod search_hook_impls;
