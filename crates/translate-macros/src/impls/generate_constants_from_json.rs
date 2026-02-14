use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::impls::utils::{get_full_path_by_manifest, read_config_json};

struct PathsInput {
    default: LitStr,
    user: LitStr,
}

impl Parse for PathsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let default: LitStr = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let user: LitStr = input.parse()?;
        Ok(PathsInput { default, user })
    }
}

pub fn generate_constants_from_json(input: TokenStream) -> syn::Result<TokenStream> {
    // 解析两个字符串字面量（默认配置, 覆盖配置）
    let parsed = syn::parse2::<PathsInput>(input)?;

    let default_path = get_full_path_by_manifest(parsed.default.value())?;
    let user_path = get_full_path_by_manifest(parsed.user.value())?;

    // 读取并解析默认配置文件
    let default_json = read_config_json(&default_path)?;

    // 读取并解析用户配置（如果存在）
    let user_json = if user_path.is_file() {
        read_config_json(&user_path)?
    } else {
        HashMap::new()
    };

    let mut merged_json: HashMap<String, JsonValue> = HashMap::new();

    // 1. 先处理 default_json
    for (key, def_entry) in &default_json {
        let mut merged_entry = def_entry.clone();

        // 如果 user 有对应值，则覆盖
        if let Some(user_entry) = user_json.get(key)
            && let Some(obj) = merged_entry.as_object_mut()
        {
            obj.insert("value".to_string(), user_entry.clone());
        }

        merged_json.insert(key.clone(), merged_entry);
    }

    // 2. 再处理 user_json 中 default 不存在的 key
    for (key, user_entry) in &user_json {
        if !merged_json.contains_key(key) {
            merged_json.insert(key.clone(), user_entry.clone());
        }
    }

    // 为每个键生成 const
    let mut const_tokens = Vec::new();
    for (key, entry) in &merged_json {
        let type_str = match entry.get("type").and_then(|t| t.as_str()) {
            Some(s) => s,
            None => syn_bail2!("配置字段 '{key}' 缺少 type"),
        };

        let encode_to_u16 = entry
            .get("encode_to_u16")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);

        // 新增 optional 标记
        let optional = entry
            .get("optional")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);

        // 这里不再直接取 value；把 Option<&JsonValue> 传入转换函数以便处理 optional
        let value_opt = entry.get("value");

        match json_item_to_const_tokens(key, type_str, value_opt, encode_to_u16, optional) {
            Ok(token) => const_tokens.push(token),
            Err(e) => syn_bail2!("解析 {key} (type '{type_str}') 失败，错误信息为: {e}"),
        };
    }

    let expanded = quote! {
        #(#const_tokens)*
    };

    Ok(expanded)
}

/// 将 JSON 值转换为对应的 TokenStream（支持基本类型和数组）
fn value_to_tokens(v: &JsonValue, encode_to_u16: bool) -> syn::Result<TokenStream> {
    fn primitive_to_tokens(v: &JsonValue, encode_to_u16: bool) -> syn::Result<TokenStream> {
        match v {
            JsonValue::String(s) => {
                if encode_to_u16 {
                    let utf16: Vec<u16> = s.encode_utf16().collect();
                    let elems = utf16.iter().map(|n| quote! { #n as u16 });
                    Ok(quote! { &[ #(#elems),* ] })
                } else {
                    let lit = Literal::string(s);
                    Ok(quote! { #lit })
                }
            }
            JsonValue::Number(n) => {
                // 保守地把数字用其字符串表示插入（让 Rust 自行解析字面量）
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
            let el_toks = primitive_to_tokens(el, encode_to_u16)?;
            elems_tokens.push(el_toks);
        }
        Ok(quote! { &[ #(#elems_tokens),* ] })
    } else {
        primitive_to_tokens(v, encode_to_u16)
    }
}

/// 根据给定的键、类型字符串和值，生成对应的常量定义 TokenStream
fn json_item_to_const_tokens(
    key: &str,
    type_str: &str,
    v_opt: Option<&JsonValue>,
    encode_to_u16: bool,
    optional: bool,
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
            let inner = value_to_tokens(v, encode_to_u16)?;
            if optional {
                quote! { Some( #inner ) }
            } else {
                quote! { #inner }
            }
        }
    };

    Ok(quote! {
        pub const #ident: #final_ty = #rhs;
    })
}
