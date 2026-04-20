use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;

use crate::{
    impls::detour::generate_detour_ident,
    utils::{
        featured_hook::{FeaturedHookLists, parse_cfg_expr},
        input::CommaSeparatedPaths,
        read_json_file, read_optional_json_file, resolve_manifest_path,
    },
};

#[derive(Default, Deserialize)]
pub struct UserHookLists {
    #[serde(default)]
    pub enable: Vec<String>,

    #[serde(default)]
    pub disable: Vec<String>,
}

pub fn generate_hook_lists_from_json(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<CommaSeparatedPaths>(input)?;

    let featured_path = resolve_manifest_path(&parsed.left)?;
    let user_path = resolve_manifest_path(&parsed.right)?;

    // 读取并解析特性化钩子列表json文件
    let featured: FeaturedHookLists = read_json_file(&featured_path)?;

    // 读取并解析用户钩子列表（如果存在）
    let user_json: UserHookLists = read_optional_json_file(&user_path)?;

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
    for (k, entry) in featured.0 {
        let filtered_hooks: Vec<String> = entry
            .fns
            .iter()
            .filter(|&name| !user_hook_set.contains(name))
            .cloned()
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
        let cfg_inner = parse_cfg_expr(&cfg_key)?;

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
