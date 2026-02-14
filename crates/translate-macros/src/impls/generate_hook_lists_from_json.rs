use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::impls::{detour::generate_detour_ident, utils::get_full_path_by_manifest};

struct PathsInput {
    featured: LitStr,
    user: LitStr,
}

impl Parse for PathsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let featured: LitStr = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let user: LitStr = input.parse()?;
        Ok(PathsInput { featured, user })
    }
}

pub fn generate_hook_lists_from_json(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathsInput>(input)?;

    let featured_path = get_full_path_by_manifest(parsed.featured.value())?;
    let user_path = get_full_path_by_manifest(parsed.user.value())?;

    // 读取并解析特性化钩子列表json文件
    let featured_str = match std::fs::read_to_string(&featured_path) {
        Ok(s) => s,
        Err(e) => {
            syn_bail2!("无法读取特性化钩子列表 {}: {}", featured_path.display(), e);
        }
    };
    let featured_json: HashMap<String, JsonValue> = match serde_json::from_str(&featured_str) {
        Ok(j) => j,
        Err(e) => {
            syn_bail2!(
                "解析特性化钩子列表失败 ({}): {}",
                featured_path.display(),
                e
            );
        }
    };

    // 读取并解析用户钩子列表（如果存在）
    let user_json: HashMap<String, JsonValue> = match std::fs::read_to_string(&user_path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(j) => j,
            Err(e) => {
                syn_bail2!("解析用户钩子列表失败 ({}): {}", user_path.display(), e);
            }
        },
        Err(_) => HashMap::new(),
    };

    let mut user_set: HashSet<String> = HashSet::new();
    let mut user_enable: Vec<String> = Vec::new();

    if let Some(JsonValue::Array(arr)) = user_json.get("disable") {
        for v in arr {
            let Some(new_disabled) = v.as_str() else {
                syn_bail2!("用户钩子列表中的 disable 项目必须为字符串");
            };
            user_set.insert(new_disabled.to_string());
        }
    }
    if let Some(JsonValue::Array(arr)) = user_json.get("enable") {
        for v in arr {
            let Some(new_enabled) = v.as_str() else {
                syn_bail2!("用户钩子列表中的 enable 项目必须为字符串");
            };
            if user_set.contains(new_enabled) {
                syn_bail2!(
                    "用户钩子列表中同时包含 enable 和 disable 的钩子，或者 enable 有重复的钩子: {new_enabled}"
                );
            }
            user_set.insert(new_enabled.to_string());
            user_enable.push(new_enabled.to_string());
        }
    }

    let mut cfg_list: Vec<(String, Vec<String>)> = Vec::new();

    for (k, v) in featured_json.into_iter() {
        if let JsonValue::Array(arr) = v {
            let mut vec_names: Vec<String> = Vec::new();
            for item in arr {
                let Some(s) = item.as_str() else {
                    syn_bail2!("预期为字符串，但并不是: {item}");
                };

                if user_set.contains(s) {
                    continue;
                }
                vec_names.push(s.to_string());
            }
            if !vec_names.is_empty() {
                cfg_list.push((k, vec_names));
            }
        } else {
            syn_bail2!("预期为数组，但并不是: {v}");
        }
    }

    // 生成 token blocks
    let mut enable_blocks: Vec<TokenStream> = Vec::new();
    let mut disable_blocks: Vec<TokenStream> = Vec::new();

    if !user_enable.is_empty() {
        let enable_idents: Vec<_> = user_enable
            .iter()
            .map(|n| generate_detour_ident(&format_ident!("{n}")))
            .collect();
        enable_blocks.push(quote! {
            {
                #(
                    if #enable_idents.enable().is_err() {
                        crate::debug!("failed to enable hook: {}", stringify!(#enable_idents));
                    }
                )*
            }
        });
        let disable_idents: Vec<_> = enable_idents.clone();
        disable_blocks.push(quote! {
            {
                #(
                    if #disable_idents.disable().is_err() {
                        crate::debug!("failed to disable hook: {}", stringify!(#disable_idents));
                    }
                )*
            }
        });
    }

    for (cfg_key, names) in cfg_list.into_iter() {
        let cfg_inner: TokenStream = cfg_key
            .parse()
            .map_err(|e| syn_err2!("无法解析 cfg key `{cfg_key}`: {e}"))?;

        let idents_enable: Vec<_> = names
            .iter()
            .map(|n| generate_detour_ident(&format_ident!("{n}")))
            .collect();
        enable_blocks.push(quote! {
            #[cfg(#cfg_inner)]
            {
                #(
                    if #idents_enable.enable().is_err() {
                        crate::debug!("failed to enable hook: {}", stringify!(#idents_enable));
                    }
                )*
            }
        });

        let idents_disable: Vec<_> = idents_enable.clone();
        disable_blocks.push(quote! {
            #[cfg(#cfg_inner)]
            {
                #(
                    if #idents_disable.disable().is_err() {
                        crate::debug!("failed to disable hook: {}", stringify!(#idents_disable));
                    }
                )*
            }
        });
    }

    // 最终拼接两个函数
    let expanded = quote! {
        pub(super) fn enable_hooks_from_lists() {
            unsafe {
                #(
                    #enable_blocks
                )*
            }
        }

        pub(super) fn disable_hooks_from_lists() {
            unsafe {
                #(
                    #disable_blocks
                )*
            }
        }
    };

    Ok(expanded)
}
