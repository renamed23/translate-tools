pub(crate) mod detour_fn;
pub(crate) mod detour_trait;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned as _;
use syn::{
    Attribute, Expr, FnArg, Ident, ItemTrait, Pat, PatIdent, ReturnType, TraitItem, Type,
    parse_macro_input,
};
use syn::{Lit, TraitItemFn};
use syn::{LitStr, Meta};

// TODO: 用 "darling"

/// 存储解析到的 detour 元数据
struct DetourAttr {
    dll: String,
    symbol: String,
    export: Option<String>,
    fallback: Option<Expr>,
    calling_convention: Option<String>,
}

pub struct Attrs {
    pub attrs: Vec<Attribute>,
}

impl Parse for Attrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
        })
    }
}

/// 将 key/lit 列表解析为 DetourAttr（共用逻辑）
fn parse_detour_from_pairs(pairs: Vec<(String, syn::Lit)>) -> syn::Result<DetourAttr> {
    let mut dll: Option<String> = None;
    let mut symbol: Option<String> = None;
    let mut export: Option<String> = None;
    let mut fallback: Option<Expr> = None;
    let mut calling_convention: Option<String> = None;

    for (key, lit) in pairs.into_iter() {
        match lit {
            syn::Lit::Str(litstr) => {
                match key.as_str() {
                    "dll" => dll = Some(litstr.value()),
                    "symbol" => symbol = Some(litstr.value()),
                    "export" => export = Some(litstr.value()),
                    "fallback" => {
                        // 原来的约定：fallback 的字符串内容是表达式，解析为 Expr
                        match syn::parse_str::<Expr>(&litstr.value()) {
                            Ok(expr) => fallback = Some(expr),
                            Err(e) => {
                                return Err(syn::Error::new(
                                    litstr.span(),
                                    format!("解析 fallback 表达式失败: {}", e),
                                ));
                            }
                        }
                    }
                    "calling_convention" => calling_convention = Some(litstr.value()),
                    _ => { /* 忽略未知键 */ }
                }
            }
            other_lit => {
                return Err(syn::Error::new(
                    other_lit.span(),
                    "detour 参数必须是字符串字面量，例如 dll = \"kernel32.dll\"",
                ));
            }
        }
    }

    match (dll, symbol) {
        (Some(dll), Some(symbol)) => Ok(DetourAttr {
            dll,
            symbol,
            export,
            fallback,
            calling_convention,
        }),
        _ => Err(syn::Error::new(
            Span::call_site(),
            "detour 属性必须包含 dll = \"...\" 和 symbol = \"...\" 两个字符串字面量",
        )),
    }
}

// /// 用于 trait 方法上的 attribute（#[detour(...)]）
// /// 返回 Ok(None) 如果 attr 不是 detour
// fn parse_detour_attr(attr: &Attribute) -> syn::Result<Option<DetourAttr>> {
//     if !attr.path().is_ident("detour") {
//         return Ok(None);
//     }

//     // 先把 Attribute 解析成 Meta，再把 NameValue 列表收集成 (key, lit) pairs
//     let meta = attr.parse_meta()?;
//     let mut pairs: Vec<(String, syn::Lit)> = Vec::new();

//     if let Meta::List(meta_list) = meta {
//         for nested in meta_list.nested.into_iter() {
//             if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
//                 if let Some(ident) = nv.path.get_ident() {
//                     pairs.push((ident.to_string(), nv.lit));
//                 } else {
//                     return Err(syn::Error::new(nv.path.span(), "detour: 无法识别参数名"));
//                 }
//             } else {
//                 return Err(syn::Error::new(
//                     nested.span(),
//                     "detour: 参数解析失败，期望 name = \"...\"",
//                 ));
//             }
//         }
//     } else {
//         return Err(syn::Error::new(
//             attr.span(),
//             "detour: 期望列表参数，例如 #[detour(dll = \"...\")]",
//         ));
//     }

//     // 交给通用解析器
//     parse_detour_from_pairs(pairs).map(Some)
// }

// /// 用于 attribute-token 的情况（#[detour_from_this(...)]）
// fn parse_detour_args_from_tokens(attr_tokens: proc_macro::TokenStream) -> syn::Result<DetourAttr> {
//     // 把 tokens 解析为 NestedMeta 列表
//     let args: syn::punctuated::Punctuated<NestedMeta, syn::token::Comma> =
//         syn::parse(attr_tokens.into())?;
//     let mut pairs: Vec<(String, syn::Lit)> = Vec::new();

//     for nested in args.into_iter() {
//         if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
//             if let Some(ident) = nv.path.get_ident() {
//                 pairs.push((ident.to_string(), nv.lit));
//             } else {
//                 return Err(syn::Error::new(
//                     nv.path.span(),
//                     "detour_from_this: 无法识别的参数名",
//                 ));
//             }
//         } else {
//             return Err(syn::Error::new(
//                 nested.span(),
//                 "detour_from_this: 参数解析失败，期望 name = \"...\"",
//             ));
//         }
//     }

//     parse_detour_from_pairs(pairs)
// }

fn parse_detour_attr(attr: &Attribute) -> syn::Result<Option<DetourAttr>> {
    if !attr.path().is_ident("detour") || !attr.path().is_ident("detour_fn") {
        return Ok(None);
    }

    let mut pairs: Vec<(String, Lit)> = Vec::new();

    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            let key = ident.to_string();
            let buf = meta.value()?;
            match buf.parse::<Lit>() {
                Ok(litstr) => {
                    pairs.push((key, litstr));
                    Ok(())
                }
                Err(_) => {
                    Err(syn::Error::new(
                        buf.span(),
                        "属性的值必须使用字符串字面量：例如 dll = \"gdi32.dll\" 或 fallback = \"FALSE\"",
                    ))
                }
            }
        } else {
            Ok(())
        }
    })?;

    // 交给通用解析器
    parse_detour_from_pairs(pairs).map(Some)
}

/// 用于 attribute-token 的情况（#[detour_from_this(...)]）
/// attr_tokens 是 proc_macro::TokenStream（来自 proc-macro attribute 的第一个参数）
fn parse_detour_args_from_tokens(attr_tokens: TokenStream) -> syn::Result<DetourAttr> {
    // 把 tokens 解析为 NestedMeta 列表（逗号分隔）

    let args = syn::parse_macro_input!(attr_tokens as Attrs);

    let mut pairs: Vec<(String, LitStr)> = Vec::new();

    for nested in args.into_iter() {
        match nested {
            NestedMeta::Meta(Meta::NameValue(nv)) => {
                if let Some(ident) = nv.path.get_ident() {
                    match nv.lit {
                        syn::Lit::Str(litstr) => pairs.push((ident.to_string(), litstr)),
                        other => {
                            return Err(syn::Error::new(
                                other.span(),
                                "detour_from_this 的参数必须是字符串字面量，例如 dll = \"kernel32.dll\"",
                            ));
                        }
                    }
                } else {
                    return Err(syn::Error::new(
                        nv.path.span(),
                        "detour_from_this: 无法识别的参数名",
                    ));
                }
            }
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "detour_from_this: 参数解析失败，期望 name = \"...\"",
                ));
            }
        }
    }

    parse_detour_from_pairs(pairs)
}
