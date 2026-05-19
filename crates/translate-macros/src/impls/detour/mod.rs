pub(crate) mod detour_fn;
pub(crate) mod detour_trait;

use convert_case::{Case, Casing};
use quote::format_ident;
use syn::{
    Attribute, Expr, Ident, LitBool, LitStr, Token,
    parse::{Parse, ParseStream},
};

pub struct DetourAttr {
    pub dll: String,
    pub symbol: String,
    pub export: Option<String>,
    pub fallback: Option<Expr>,
    pub calling_convention: Option<String>,
    pub enable_hook_guard: bool,
}

impl Parse for DetourAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut dll: Option<LitStr> = None;
        let mut symbol: Option<LitStr> = None;
        let mut export: Option<LitStr> = None;
        let mut fallback: Option<Expr> = None;
        let mut calling_convention: Option<LitStr> = None;
        let mut enable_hook_guard: Option<LitBool> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;

            match key.to_string().as_str() {
                "dll" => {
                    if dll.is_some() {
                        syn_bail!(key, "重复的 `dll` 参数");
                    }
                    dll = Some(input.parse::<LitStr>()?);
                }
                "symbol" => {
                    if symbol.is_some() {
                        syn_bail!(key, "重复的 `symbol` 参数");
                    }
                    symbol = Some(input.parse::<LitStr>()?);
                }
                "export" => {
                    if export.is_some() {
                        syn_bail!(key, "重复的 `export` 参数");
                    }
                    export = Some(input.parse::<LitStr>()?);
                }
                "fallback" => {
                    if fallback.is_some() {
                        syn_bail!(key, "重复的 `fallback` 参数");
                    }
                    // 这里可以直接解析为任意合法的 Rust 表达式 (Expr)，不再需要从字符串二次解析！
                    fallback = Some(input.parse::<Expr>()?);
                }
                "calling_convention" => {
                    if calling_convention.is_some() {
                        syn_bail!(key, "重复的 `calling_convention` 参数");
                    }
                    calling_convention = Some(input.parse::<LitStr>()?);
                }
                "enable_hook_guard" => {
                    if enable_hook_guard.is_some() {
                        syn_bail!(key, "重复的 `enable_hook_guard` 参数");
                    }
                    // 直接解析为原生布尔字面量 (true/false)
                    enable_hook_guard = Some(input.parse::<LitBool>()?);
                }
                other => syn_bail!(
                    key,
                    "未知参数 `{other}`, 预期 `dll`, `symbol`, `export`, `fallback`, \
                     `calling_convention`, 或 `enable_hook_guard`"
                ),
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        // 强校验必填项
        let dll_lit = dll.ok_or_else(|| input.error("未指定 `dll` 参数"))?;
        let symbol_lit = symbol.ok_or_else(|| input.error("未指定 `symbol` 参数"))?;

        Ok(DetourAttr {
            dll: dll_lit.value(),
            symbol: symbol_lit.value(),
            export: export.map(|l| l.value()),
            fallback,
            calling_convention: calling_convention.map(|l| l.value()),
            enable_hook_guard: enable_hook_guard.map(|b| b.value).unwrap_or(true),
        })
    }
}

pub fn parse_detour_attr(attr: &Attribute) -> syn::Result<Option<DetourAttr>> {
    if !attr.path().is_ident("detour") {
        return Ok(None);
    }
    // 使用 attr.parse_args::<T>() 自动提取并解析并括号 `#[detour(...)]` 内部的 TokenStream
    let detour_attr = attr.parse_args::<DetourAttr>()?;
    Ok(Some(detour_attr))
}

fn parse_detour_attrs<'a>(
    attrs: impl Iterator<Item = &'a Attribute>,
) -> syn::Result<Option<DetourAttr>> {
    let mut detour_meta: Option<DetourAttr> = None;
    for attr in attrs {
        match parse_detour_attr(attr) {
            Ok(Some(parsed)) => {
                detour_meta = Some(parsed);
                break;
            }
            Ok(None) => { /* 这个 attr 不是 detour，继续 */ }
            Err(e) => syn_bail!(attr, "{e}"),
        }
    }

    Ok(detour_meta)
}

pub fn generate_detour_ident(ident: &Ident) -> Ident {
    let static_name = format!(
        "HOOK_{}",
        ident.to_string().to_case(Case::Snake).to_uppercase()
    );
    format_ident!("{static_name}")
}
