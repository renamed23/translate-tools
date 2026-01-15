use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::utils::get_full_path_by_manifest;

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

    let default_path = get_full_path_by_manifest(parsed.default.value()).unwrap();
    let user_path = get_full_path_by_manifest(parsed.user.value()).unwrap();

    // 读取并解析默认配置文件
    let default_str = match std::fs::read_to_string(&default_path) {
        Ok(s) => s,
        Err(e) => {
            syn_bail2!("无法读取默认配置 {}: {}", default_path.display(), e);
        }
    };
    let default_json: HashMap<String, JsonValue> = match serde_json::from_str(&default_str) {
        Ok(j) => j,
        Err(e) => {
            syn_bail2!("解析默认配置 JSON 失败 ({}): {}", default_path.display(), e);
        }
    };

    // 读取并解析用户配置（如果存在）
    let user_json: HashMap<String, JsonValue> = match std::fs::read_to_string(&user_path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(j) => j,
            Err(e) => {
                syn_bail2!("解析用户配置 JSON 失败 ({}): {}", user_path.display(), e);
            }
        },
        Err(_) => HashMap::new(), // 文件不存在则当空覆盖
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

        let value = match entry.get("value") {
            Some(v) => v,
            None => syn_bail2!("配置字段 '{key}' 缺少 value"),
        };

        let encode_to_u16 = entry
            .get("encode_to_u16")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);

        match json_value_to_rust_tokens(key, type_str, value, encode_to_u16) {
            Ok(token) => const_tokens.push(token),
            Err(e) => syn_bail2!("解析 {key} (type '{type_str}') 失败，错误信息为: {e}"),
        };
    }

    let expanded = quote! {
        #(#const_tokens)*
    };

    Ok(expanded)
}

/// 将 JSON 值转换为用于生成 const 的 token（字符串/数组/数字/布尔）
fn json_value_to_rust_tokens(
    key: &str,
    type_str: &str,
    v: &JsonValue,
    encode_to_u16: bool,
) -> syn::Result<TokenStream> {
    // 生成一个合法的 Rust 标识符（不改变大小写，只做简单替换）
    let ident_name = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let ident = format_ident!("{}", ident_name);

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

    // 支持数组（数组内元素可以是 primitive 或字符串）
    let rhs = if let Some(arr) = v.as_array() {
        // 对数组的每个元素应用 primitive_to_tokens
        let mut elems_tokens = Vec::new();
        for el in arr {
            let el_toks = primitive_to_tokens(el, encode_to_u16)?;
            elems_tokens.push(el_toks);
        }
        // 这里我们生成 &[ elem0, elem1, ... ]
        quote! { &[ #(#elems_tokens),* ] }
    } else {
        primitive_to_tokens(v, encode_to_u16)?
    };

    // 最终生成： pub const <IDENT>: <type_str> = <rhs>;
    // type_str 直接插入为标识符或路径；但它可能包含 `::` 等。我们简单把它作为 TokenStream 解析。
    let ty_tokens = syn::parse_str::<TokenStream>(type_str)?;

    Ok(quote! {
        pub const #ident: #ty_tokens = #rhs;
    })
}
