use std::collections::HashSet;

use convert_case::{Case, Casing};
use goblin::Object;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::{
    impls::detour::generate_detour_ident,
    utils::{
        build_dll_map_from_api_hooks,
        featured_hook::{FeaturedHookLists, parse_cfg_expr},
        find_single_file_in_dir, get_full_path_by_manifest, read_json_file,
        read_optional_json_file, resolve_manifest_path,
    },
};

#[derive(Default, Deserialize)]
pub struct UserHookLists {
    #[serde(default)]
    pub enable: Vec<String>,

    #[serde(default)]
    pub disable: Vec<String>,
}

struct Input {
    featured_path: LitStr,
    user_path: LitStr,
    exe_dir: Option<LitStr>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let featured_path: LitStr = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let user_path: LitStr = input.parse()?;

        let mut exe_dir = None;

        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;

            if !input.is_empty() {
                let key: Ident = input.parse()?;
                let _eq: Token![=] = input.parse()?;

                match key.to_string().as_str() {
                    "exe_dir" => {
                        exe_dir = Some(input.parse::<LitStr>()?);
                    }
                    other => syn_bail!(key, "未知参数 `{other}`, 预期 `exe_dir`"),
                }

                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            }
        }

        Ok(Input {
            featured_path,
            user_path,
            exe_dir,
        })
    }
}

/// 解析目标 EXE 的导入表，返回 (已导入函数名集合, 已导入 DLL 名集合)。
///
/// 所有名称均转为小写以实现大小写不敏感比较。
/// 纯序号导入（`ORDINAL N`）不会被加入函数名集合。
fn parse_exe_imports(exe_dir: &LitStr) -> syn::Result<(HashSet<String>, HashSet<String>)> {
    let exe_dir_full =
        get_full_path_by_manifest(exe_dir.value()).map_err(|e| syn_err!(exe_dir, "{e}"))?;
    let exe_path = find_single_file_in_dir(&exe_dir_full, "exe", exe_dir)?;

    let exe_bytes = std::fs::read(&exe_path).map_err(|e| syn_err!(exe_dir, "读取exe失败: {e}"))?;

    let pe = match Object::parse(&exe_bytes).map_err(|e| syn_err!(exe_dir, "解析PE失败: {e}"))?
    {
        Object::PE(pe) => pe,
        other => syn_bail!(exe_dir, "不是PE文件: {other:?}"),
    };

    let mut imported_fns = HashSet::new();
    let mut imported_dlls = HashSet::new();

    for import in &pe.imports {
        if !import.name.starts_with("ORDINAL ") {
            imported_fns.insert(import.name.to_lowercase());
        }
        imported_dlls.insert(import.dll.to_lowercase());
    }

    Ok((imported_fns, imported_dlls))
}

/// 判断某个函数名是否与 EXE 导入表兼容。
///
/// `name` 可为 PascalCase 或 snake_case，查找 `dll_map` 时统一转为 snake_case。
fn is_import_compatible(
    name: &str,
    dll_map: &std::collections::HashMap<String, String>,
    imported_fns: &HashSet<String>,
    imported_dlls: &HashSet<String>,
) -> bool {
    let key = name.to_case(Case::Snake);
    if let Some(dll_name) = dll_map.get(&key)
        && !imported_dlls.contains(&dll_name.to_lowercase())
    {
        return false;
    }
    imported_fns.contains(&name.to_lowercase())
}

/// 从一组函数名按 EXE 导入兼容性分割，转换为 ident。
fn split_idents_by_compat(
    names: &[String],
    dll_map: &std::collections::HashMap<String, String>,
    imported_fns: &HashSet<String>,
    imported_dlls: &HashSet<String>,
) -> (Vec<syn::Ident>, Vec<syn::Ident>) {
    let (compat, incompat): (Vec<_>, Vec<_>) = names
        .iter()
        .partition(|name| is_import_compatible(name, dll_map, imported_fns, imported_dlls));
    let to_idents = |v: Vec<&String>| {
        v.into_iter()
            .map(|n| generate_detour_ident(&format_ident!("{n}")))
            .collect()
    };
    (to_idents(compat), to_idents(incompat))
}

pub fn generate_hook_lists(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<Input>(input)?;

    let featured_path = resolve_manifest_path(&parsed.featured_path)?;
    let user_path = resolve_manifest_path(&parsed.user_path)?;

    let featured: FeaturedHookLists = read_json_file(&featured_path)?;
    let user_json: UserHookLists = read_optional_json_file(&user_path)?;

    let mut user_hook_set: HashSet<String> = HashSet::new();

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

    let (imported_fns, imported_dlls) = if let Some(ref exe_dir_lit) = parsed.exe_dir {
        parse_exe_imports(exe_dir_lit)?
    } else {
        (HashSet::new(), HashSet::new())
    };
    let has_exe_dir = parsed.exe_dir.is_some();

    let dll_map = if has_exe_dir {
        let api_hooks_dir = get_full_path_by_manifest("src/hook/api_hooks")?;
        build_dll_map_from_api_hooks(&api_hooks_dir)?
    } else {
        std::collections::HashMap::new()
    };

    let mut enable_blocks: Vec<TokenStream> = Vec::new();
    let mut disable_blocks: Vec<TokenStream> = Vec::new();

    let mut push_hook_blocks = |idents: &[syn::Ident], cfg: Option<TokenStream>| {
        let enable_body = quote! {
            #(
                if #idents.enable().is_err() {
                    crate::debug!("failed to enable hook: {}", stringify!(#idents));
                }
            )*
        };
        let disable_body = quote! {
            #(
                if #idents.disable().is_err() {
                    crate::debug!("failed to disable hook: {}", stringify!(#idents));
                }
            )*
        };

        match cfg {
            Some(cfg) => {
                enable_blocks.push(quote! { #[cfg(#cfg)] { #enable_body } });
                disable_blocks.push(quote! { #[cfg(#cfg)] { #disable_body } });
            }
            None => {
                enable_blocks.push(quote! { { #enable_body } });
                disable_blocks.push(quote! { { #disable_body } });
            }
        }
    };

    // ── 用户 enable 列表 ────────────────────────────────────────────
    if !user_json.enable.is_empty() {
        let idents: Vec<_> = user_json
            .enable
            .iter()
            .map(|n| generate_detour_ident(&format_ident!("{n}")))
            .collect();
        push_hook_blocks(&idents, None);
    }

    // ── featured hook lists ─────────────────────────────────────────
    for (cfg_key, entry) in featured.0 {
        let hooks: Vec<String> = entry
            .fns
            .iter()
            .filter(|&name| !user_hook_set.contains(name))
            .cloned()
            .collect();

        if hooks.is_empty() {
            continue;
        }

        let cfg_inner = parse_cfg_expr(&cfg_key)?;

        if has_exe_dir {
            let (compat_idents, incompat_idents) =
                split_idents_by_compat(&hooks, &dll_map, &imported_fns, &imported_dlls);

            if !compat_idents.is_empty() {
                push_hook_blocks(&compat_idents, Some(cfg_inner.clone()));
            }

            if !incompat_idents.is_empty() {
                let non_iat_cfg: TokenStream =
                    syn::parse_quote! { all(#cfg_inner, not(feature = "enable_iat_hook")) };
                push_hook_blocks(&incompat_idents, Some(non_iat_cfg));
            }
        } else {
            let idents: Vec<_> = hooks
                .iter()
                .map(|n| generate_detour_ident(&format_ident!("{n}")))
                .collect();
            push_hook_blocks(&idents, Some(cfg_inner));
        }
    }

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
