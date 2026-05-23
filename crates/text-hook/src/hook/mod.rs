pub(crate) mod entry;
pub(crate) mod hook_guard;

pub(crate) mod api_hooks;
pub(crate) mod components;
pub(crate) mod impls;
pub(crate) mod internal_hooks;

mod hook_lists {
    translate_macros::expand_by_files!("src/hook/api_hooks" => {
        use super::api_hooks::__file_stem_ident__::*;
    });

    cfg_select! {
        feature = "enable_iat_hook_with_strip" => {
            translate_macros::generate_hook_lists!(
                "constant_assets/featured_hook_lists.json",
                "assets/hook_lists.json",
                exe_dir = "assets/exe"
            );
        }
        _ => {
            translate_macros::generate_hook_lists!(
                "constant_assets/featured_hook_lists.json",
                "assets/hook_lists.json",
            );
        }
    }
}

/// 从钩子列表中开启所有的钩子
#[allow(dead_code)]
pub fn enable_hooks_from_lists() {
    hook_lists::enable_hooks_from_lists();
}

/// 从钩子列表中关闭所有的钩子
#[allow(dead_code)]
pub fn disable_hooks_from_lists() {
    hook_lists::disable_hooks_from_lists();
}

/// 调用该HOOK函数对应的原始函数
#[macro_export]
macro_rules! call {
    ($hook:ident, $($arg:tt)*) => {{
        cfg_select! {
            feature = "enable_iat_hook" => $hook.orig()($($arg)*),
            _ => $hook.call($($arg)*)
        }
    }};
}
