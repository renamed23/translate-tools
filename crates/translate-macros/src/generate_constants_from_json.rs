use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::utils::compile_error;

pub fn generate_constants_from_json(input: TokenStream) -> TokenStream {
    // 解析两个字符串字面量（默认配置, 覆盖配置）
    let parsed = syn::parse_macro_input!(input as PathsInput);

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("无法获取 CARGO_MANIFEST_DIR");

    let rel_default_path = parsed.default.value();
    let rel_user_path = parsed.user.value();
    let default_path = PathBuf::from(&manifest_dir).join(&rel_default_path);
    let user_path = PathBuf::from(&manifest_dir).join(&rel_user_path);

    // 读取并解析默认配置文件
    let default_str = match fs::read_to_string(&default_path) {
        Ok(s) => s,
        Err(e) => {
            return compile_error(&format!(
                "generate_constants: 无法读取默认配置 {}: {}",
                default_path.display(),
                e
            ));
        }
    };
    let default_json: HashMap<String, JsonValue> = match serde_json::from_str(&default_str) {
        Ok(j) => j,
        Err(e) => {
            return compile_error(&format!(
                "generate_constants: 解析默认配置 JSON 失败 ({}): {}",
                default_path.display(),
                e
            ));
        }
    };

    // 读取并解析用户配置（如果存在）
    let user_json: HashMap<String, JsonValue> = match fs::read_to_string(&user_path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(j) => j,
            Err(e) => {
                return compile_error(&format!(
                    "generate_constants: 解析用户配置 JSON 失败 ({}): {}",
                    user_path.display(),
                    e
                ));
            }
        },
        Err(_) => HashMap::new(), // 文件不存在则当空覆盖
    };

    // 为每个键生成 const
    let mut const_tokens = Vec::new();
    for (key, def_val) in &default_json {
        // 获取 type & value 字段
        let type_str = match def_val.get("type").and_then(|t| t.as_str()) {
            Some(s) => s,
            None => {
                return compile_error(&format!(
                    "generate_constants: default_config.json 中字段 '{}' 缺少 type",
                    key
                ));
            }
        };

        let default_v = match def_val.get("value") {
            Some(v) => v,
            None => {
                return compile_error(&format!(
                    "generate_constants: default_config.json 中字段 '{}' 缺少 value",
                    key
                ));
            }
        };

        let encode_to_u16 = def_val
            .get("encode_to_u16")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);

        // final_value 优先使用 user_json 中的值
        let final_value = user_json.get(key).unwrap_or(default_v);

        match json_value_to_rust_tokens(key, type_str, final_value, encode_to_u16) {
            Ok(tokens) => const_tokens.push(tokens),
            Err(e) => return compile_error(&format!("generate_constants: {}", e)),
        }
    }

    let expanded = quote! {
        #(#const_tokens)*
    };

    TokenStream::from(expanded)
}

/// 解析 proc-macro 输入：两个 string literal
struct PathsInput {
    default: syn::LitStr,
    user: syn::LitStr,
}
impl syn::parse::Parse for PathsInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let default: syn::LitStr = input.parse()?;
        let _comma: syn::Token![,] = input.parse()?;
        let user: syn::LitStr = input.parse()?;
        Ok(PathsInput { default, user })
    }
}

/// 将 JSON 值转换为用于生成 const 的 token（字符串/数组/数字/布尔）
fn json_value_to_rust_tokens(
    key: &str,
    type_str: &str,
    v: &JsonValue,
    encode_to_u16: bool,
) -> Result<proc_macro2::TokenStream, String> {
    // 生成一个合法的 Rust 标识符（不改变大小写，只做简单替换）
    let ident_name = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let ident = format_ident!("{}", ident_name);

    fn primitive_to_tokens(
        v: &JsonValue,
        encode_to_u16: bool,
    ) -> Result<proc_macro2::TokenStream, String> {
        match v {
            JsonValue::String(s) => {
                if encode_to_u16 {
                    let utf16: Vec<u16> = s.encode_utf16().collect();
                    let elems = utf16.iter().map(|n| quote! { #n as u16 });
                    Ok(quote! { &[ #(#elems),* ] })
                } else {
                    let lit = proc_macro2::Literal::string(s);
                    Ok(quote! { #lit })
                }
            }
            JsonValue::Number(n) => {
                // 保守地把数字用其字符串表示插入（让 Rust 自行解析字面量）
                let s = n.to_string();
                let lit = syn::parse_str::<proc_macro2::TokenStream>(&s)
                    .map_err(|e| format!("无法将数字 '{}' 转为 token: {}", s, e))?;
                Ok(lit)
            }
            JsonValue::Bool(b) => {
                if *b {
                    Ok(quote! { true })
                } else {
                    Ok(quote! { false })
                }
            }
            JsonValue::Null => Err("null 不能转换为常量值".to_string()),
            JsonValue::Array(_) | JsonValue::Object(_) => {
                Err("期待基本类型，但收到了复杂类型".to_string())
            }
        }
    }

    // 支持数组（数组内元素可以是 primitive 或字符串）
    let rhs = if let Some(arr) = v.as_array() {
        // 对数组的每个元素应用 primitive_to_tokens
        let mut elems_tokens = Vec::new();
        for el in arr {
            let el_toks = primitive_to_tokens(el, encode_to_u16)
                .map_err(|e| format!("字段 '{}': {}", key, e))?;
            elems_tokens.push(el_toks);
        }
        // 这里我们生成 &[ elem0, elem1, ... ]
        quote! { &[ #(#elems_tokens),* ] }
    } else {
        primitive_to_tokens(v, encode_to_u16).map_err(|e| format!("字段 '{}': {}", key, e))?
    };

    // 最终生成： pub const <IDENT>: <type_str> = <rhs>;
    // type_str 直接插入为标识符或路径；但它可能包含 `::` 等。我们简单把它作为 TokenStream 解析。
    let ty_tokens = syn::parse_str::<proc_macro2::TokenStream>(type_str)
        .map_err(|e| format!("无法解析 type '{}' 为 Rust 类型: {}", type_str, e))?;

    Ok(quote! {
        pub const #ident: #ty_tokens = #rhs;
    })
}
