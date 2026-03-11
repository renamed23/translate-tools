use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
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

#[derive(Deserialize)]
pub struct UserHookLists {
    #[serde(default)]
    pub enable: Vec<String>,

    #[serde(default)]
    pub disable: Vec<String>,
}

#[derive(Deserialize)]
pub struct FeaturedHookLists(#[serde(default)] HashMap<String, Vec<String>>);

pub fn generate_hook_lists_from_json(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathsInput>(input)?;

    let featured_path = get_full_path_by_manifest(parsed.featured.value())?;
    let user_path = get_full_path_by_manifest(parsed.user.value())?;

    // 读取并解析特性化钩子列表json文件
    let featured_str = std::fs::read_to_string(&featured_path)
        .map_err(|e| syn_err2!("无法读取特性化钩子列表 {}: {}", featured_path.display(), e))?;
    let featured: FeaturedHookLists = serde_json::from_str(&featured_str).map_err(|e| {
        syn_err2!(
            "解析特性化钩子列表失败 ({}): {}",
            featured_path.display(),
            e
        )
    })?;

    // 读取并解析用户钩子列表（如果存在）
    let user_json: UserHookLists =
        serde_json::from_str(&std::fs::read_to_string(&user_path).unwrap_or("{}".to_string()))
            .map_err(|e| syn_err2!("解析用户钩子列表失败 ({}): {}", user_path.display(), e))?;

    // 使用 HashSet 记录所有被强制设定的钩子（包括 enable 和 disable）
    let mut user_hook_set: HashSet<String> = HashSet::new();

    // 检查是否有冲突
    for hook in &user_json.enable {
        if !user_hook_set.insert(hook.clone()) {
            syn_bail2!("用户钩子列表 enable 中存在重复项或与 disable 冲突: {hook}");
        }
    }

    for hook in &user_json.disable {
        if !user_hook_set.insert(hook.clone()) {
            syn_bail2!("用户钩子列表 disable 中存在重复项或与 enable 冲突: {hook}");
        }
    }

    // 筛选出需要添加的特性化钩子
    // 如果一个钩子在 user_hook_set 中，则跳过

    let mut cfg_list: Vec<(String, Vec<String>)> = Vec::new();
    for (k, v) in featured.0 {
        let filtered_hooks: Vec<String> = v
            .into_iter()
            .filter(|name| !user_hook_set.contains(name))
            .collect();

        if !filtered_hooks.is_empty() {
            cfg_list.push((k, filtered_hooks));
        }
    }

    // 生成 token blocks
    let mut enable_blocks: Vec<TokenStream> = Vec::new();
    let mut disable_blocks: Vec<TokenStream> = Vec::new();

    if !user_json.enable.is_empty() {
        let enable_idents: Vec<_> = user_json
            .enable
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

    for (cfg_key, names) in cfg_list {
        let cfg_inner: TokenStream = cfg_key
            .parse()
            .map_err(|e| syn_err2!("无法解析 cfg key `{cfg_key}`: {e}"))?;

        let idents: Vec<_> = names
            .iter()
            .map(|n| generate_detour_ident(&format_ident!("{n}")))
            .collect();

        enable_blocks.push(quote! {
            #[cfg(#cfg_inner)]
            {
                #(
                    if #idents.enable().is_err() {
                        crate::debug!("failed to enable hook: {}", stringify!(#idents));
                    }
                )*
            }
        });

        disable_blocks.push(quote! {
            #[cfg(#cfg_inner)]
            {
                #(
                    if #idents.disable().is_err() {
                        crate::debug!("failed to disable hook: {}", stringify!(#idents));
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
