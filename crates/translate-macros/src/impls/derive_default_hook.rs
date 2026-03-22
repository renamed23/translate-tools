use std::collections::HashSet;

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Attribute, DeriveInput, Ident, Item, LitStr, Token, punctuated::Punctuated};

use crate::utils::{
    collect_files_in_dir,
    featured_hook::{FeaturedHookLists, build_cfg_not_any, build_featured_trait_cfg_map},
    read_file_string, read_json_file, resolve_manifest_path,
};

pub fn derive_default_hook(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    let name = input.ident;

    let exclude = parse_exclude_attrs(&input.attrs)?;
    let exclude: HashSet<String> = exclude
        .into_iter()
        .map(|ident| ident.to_string().to_case(Case::Pascal))
        .collect();

    let trait_dir = resolve_manifest_path(&LitStr::new("src/hook/traits", Span::call_site()))?;
    let featured_path = resolve_manifest_path(&LitStr::new(
        "constant_assets/featured_hook_lists.json",
        Span::call_site(),
    ))?;

    let featured: FeaturedHookLists = read_json_file(&featured_path)?;
    let trait_cfg_map = build_featured_trait_cfg_map(&featured)?;

    let mut impl_blocks = Vec::new();

    for path in collect_files_in_dir(&trait_dir)? {
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext != "rs" {
            continue;
        }

        let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if matches!(file_stem, "mod" | "lib") {
            continue;
        }

        let module_ident = format_ident!("{file_stem}");

        let file_content = read_file_string(&path)?;
        let parsed_file = syn::parse_file(&file_content)
            .map_err(|e| syn_err2!("解析 {} 失败: {e}", path.display()))?;

        for item in parsed_file.items {
            let Item::Trait(item_trait) = item else {
                continue;
            };

            let trait_ident = item_trait.ident;
            let trait_name = trait_ident.to_string();

            if exclude.contains(&trait_name) {
                continue;
            }

            let cfg_attr = trait_cfg_map
                .get(&trait_name)
                .map(|cfgs| build_cfg_not_any(cfgs))
                .unwrap_or_default();

            impl_blocks.push(quote! {
                #cfg_attr
                impl crate::hook::traits::#module_ident::#trait_ident for #name {}
            });
        }
    }

    Ok(quote! {
        #(#impl_blocks)*
    })
}

fn parse_exclude_attrs(attrs: &[Attribute]) -> syn::Result<Vec<Ident>> {
    let mut result = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("exclude") {
            continue;
        }

        let punctuated: Punctuated<Ident, Token![,]> =
            attr.parse_args_with(Punctuated::parse_terminated)?;

        result.extend(punctuated);
    }

    Ok(result)
}
