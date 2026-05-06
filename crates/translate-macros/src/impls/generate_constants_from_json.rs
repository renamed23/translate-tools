use std::collections::HashMap;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::utils::{input::MultiPaths, read_json_file, resolve_manifest_path};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ConfigEntry {
    Complex {
        #[serde(rename = "type")]
        ty: String,
        #[serde(default)]
        value: Option<serde_json::Value>,
        #[serde(default)]
        encode_to_u16: bool,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        expr: bool,
    },
    Simple(serde_json::Value),
}

#[derive(Deserialize)]
pub struct ConstantConfig(pub HashMap<String, ConfigEntry>);

pub fn generate_constants_from_json(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<MultiPaths>(input)?;

    let mut merged: HashMap<String, ConfigEntry> = HashMap::new();

    for path in &parsed.paths {
        let abs_path = resolve_manifest_path(path)?;
        let cfg: ConstantConfig = read_json_file(&abs_path)?;

        for (k, v) in cfg.0 {
            let old_entry = merged.remove(&k);

            let new_entry = match (old_entry, v) {
                // 1. 两个都是 Complex -> 报错
                (Some(ConfigEntry::Complex { .. }), ConfigEntry::Complex { .. }) => {
                    syn_bail2!("字段 '{k}' 只能有一个 Complex 定义");
                }

                // 2. 旧的是 Simple，新的是 Complex -> 合并值
                (
                    Some(ConfigEntry::Simple(sv)),
                    ConfigEntry::Complex {
                        ty,
                        value,
                        encode_to_u16,
                        optional,
                        expr,
                    },
                ) => ConfigEntry::Complex {
                    ty,
                    value: value.or(Some(sv)),
                    encode_to_u16,
                    optional,
                    expr,
                },

                // 3. 旧的是 Complex，新的是 Simple -> 更新 Complex 的值
                (
                    Some(ConfigEntry::Complex {
                        ty,
                        value: _,
                        encode_to_u16,
                        optional,
                        expr,
                    }),
                    ConfigEntry::Simple(sv),
                ) => ConfigEntry::Complex {
                    ty,
                    value: Some(sv),
                    encode_to_u16,
                    optional,
                    expr,
                },

                // 4. 旧的是 Simple，新的也是 Simple -> 覆盖
                // 5. 之前没有 -> 直接插入新的
                (_, new) => new,
            };

            merged.insert(k, new_entry);
        }
    }

    for (k, v) in &merged {
        if let ConfigEntry::Simple(_) = v {
            syn_bail2!("字段 '{k}' 合并后必须为 Complex 类型，但未找到 Complex 定义");
        }
    }

    // 生成代码
    let mut const_tokens = Vec::new();
    for (key, entry) in merged {
        if let ConfigEntry::Complex {
            ty,
            value,
            encode_to_u16,
            optional,
            expr,
        } = entry
        {
            let val_opt = value.as_ref();

            const_tokens.push(json_item_to_const_tokens(
                &key,
                &ty,
                val_opt,
                encode_to_u16,
                optional,
                expr,
            )?);
        }
    }

    Ok(quote! { #(#const_tokens)* })
}

/// 将 JSON 值转换为对应的 TokenStream（支持基本类型和数组）
fn value_to_tokens(v: &JsonValue, encode_to_u16: bool, expr: bool) -> syn::Result<TokenStream> {
    fn primitive_to_tokens(
        v: &JsonValue,
        encode_to_u16: bool,
        expr: bool,
    ) -> syn::Result<TokenStream> {
        match v {
            JsonValue::String(s) => {
                if expr {
                    let parsed = syn::parse_str::<TokenStream>(s).map_err(|e| {
                        syn::Error::new_spanned(Literal::string(s), format!("表达式解析失败: {e}"))
                    })?;
                    Ok(parsed)
                } else if encode_to_u16 {
                    let utf16: Vec<u16> = s.encode_utf16().collect();
                    let elems = utf16.iter().map(|n| quote! { #n as u16 });
                    Ok(quote! { &[ #(#elems),* ] })
                } else {
                    let lit = Literal::string(s);
                    Ok(quote! { #lit })
                }
            }
            JsonValue::Number(n) => {
                let s = n.to_string();
                let lit = syn::parse_str::<TokenStream>(&s)?;
                Ok(lit)
            }
            JsonValue::Bool(b) => {
                if *b {
                    Ok(quote! { true })
                } else {
                    Ok(quote! { false })
                }
            }
            JsonValue::Null => syn_bail2!("null 不能转换为常量值"),
            JsonValue::Array(_) | JsonValue::Object(_) => {
                syn_bail2!("期待基本类型，但收到了复杂类型")
            }
        }
    }

    if let Some(arr) = v.as_array() {
        let mut elems_tokens = Vec::new();
        for el in arr {
            let el_toks = primitive_to_tokens(el, encode_to_u16, expr)?;
            elems_tokens.push(el_toks);
        }
        Ok(quote! { &[ #(#elems_tokens),* ] })
    } else {
        primitive_to_tokens(v, encode_to_u16, expr)
    }
}

/// 根据给定的键、类型字符串和值，生成对应的常量定义 TokenStream
fn json_item_to_const_tokens(
    key: &str,
    type_str: &str,
    v_opt: Option<&JsonValue>,
    encode_to_u16: bool,
    optional: bool,
    expr: bool,
) -> syn::Result<TokenStream> {
    // 生成一个合法的 Rust 标识符（不改变大小写，只做简单替换）
    let ident_name = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let ident = format_ident!("{}", ident_name);

    // 生成类型 token，并在 optional 时包裹 Option<...>
    let ty_tokens = syn::parse_str::<TokenStream>(type_str)?;
    let final_ty = if optional {
        quote! { Option<#ty_tokens> }
    } else {
        quote! { #ty_tokens }
    };

    let rhs = match v_opt {
        None | Some(JsonValue::Null) => {
            // 如果非可选并且没有值，我们直接忽略该条目
            if !optional {
                return Ok(quote! {});
            }
            quote! { None }
        }
        Some(v) => {
            let inner = value_to_tokens(v, encode_to_u16, expr)?;
            if optional {
                quote! { Some(#inner) }
            } else {
                quote! { #inner }
            }
        }
    };

    Ok(quote! {
        pub const #ident: #final_ty = #rhs;
    })
}
