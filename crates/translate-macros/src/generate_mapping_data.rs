use proc_macro::TokenStream;
use quote::quote;
use std::{collections::HashMap, fs, path::PathBuf};
use translate_utils::{jis0208::is_jis0208, text::Text};

use crate::utils::compile_error;

pub fn generate_mapping_data(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as PathsInput);

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("无法获取 CARGO_MANIFEST_DIR");

    let mapping_path = PathBuf::from(&manifest_dir).join(parsed.mapping.value());
    let translated_path = parsed
        .translated
        .map(|t| PathBuf::from(&manifest_dir).join(t.value()));

    if !mapping_path.exists() {
        return compile_error(&format!(
            "generate_mapping_data: 映射JSON文件未找到: {}",
            mapping_path.display()
        ));
    }

    let mapping_str = match fs::read_to_string(&mapping_path) {
        Ok(s) => s,
        Err(e) => {
            return compile_error(&format!(
                "generate_mapping_data: 无法读取 {}: {}",
                mapping_path.display(),
                e
            ));
        }
    };

    let map: HashMap<String, String> = match serde_json::from_str(&mapping_str) {
        Ok(m) => m,
        Err(e) => {
            return compile_error(&format!(
                "generate_mapping_data: 解析 {} 失败: {}",
                mapping_path.display(),
                e
            ));
        }
    };

    let mut char_mapping = HashMap::new();

    // 如果指定了translated路径，从 译文JSON 提取字符
    if let Some(translated_path) = translated_path {
        if !translated_path.exists() {
            return compile_error(&format!(
                "generate_mapping_data: GENERATE_FULL_MAPPING_DATA 开启，但是 {} 未找到",
                translated_path.display()
            ));
        }

        let text = match Text::from_path(&translated_path) {
            Ok(s) => s,
            Err(e) => {
                return compile_error(&format!(
                    "generate_mapping_data: 无法读取 {}: {}",
                    translated_path.display(),
                    e
                ));
            }
        };

        // 提取所有双字节字符（JIS0208兼容）
        let all_chars = text.get_filtered_chars(is_jis0208);

        // 为每个字符创建自映射（字符 -> 相同字符）
        for ch in all_chars {
            let s = ch.to_string();
            char_mapping.insert(s.clone(), s);
        }
    }

    // 用 mapping.json 覆盖或添加映射
    for (k, v) in map.into_iter() {
        char_mapping.insert(k, v);
    }

    let mut entries: Vec<(u16, u16, String, String)> = Vec::new();
    let mut seen_codes = std::collections::HashSet::new();

    for (k, v) in char_mapping.into_iter() {
        if k.chars().count() != 1 {
            return compile_error(&format!("映射键必须是单个字符，发现: {:?}", k));
        }
        if v.chars().count() != 1 {
            return compile_error(&format!("映射值必须是单个字符，发现: {:?}", v));
        }

        let kc = k.chars().next().unwrap();
        let vc = v.chars().next().unwrap();

        // 使用 is_jis0208 判断 key 是否为 JIS0208（可被 Shift_JIS 编码）
        if !is_jis0208(kc) {
            return compile_error(&format!(
                "映射键 '{kc}' 不是 JIS0208（不可被 Shift_JIS 编码）"
            ));
        }

        let (enc, _, had_errors) = encoding_rs::SHIFT_JIS.encode(&k);
        if had_errors {
            return compile_error(&format!("键 '{k}' 编码为 Shift_JIS 时出现错误"));
        }
        if enc.len() != 2 {
            return compile_error(&format!(
                "键 '{}' 编码为 Shift_JIS 后长度异常: {}",
                k,
                enc.len()
            ));
        }
        let key_code: u16 = ((enc[0] as u16) << 8) | (enc[1] as u16);

        if seen_codes.contains(&key_code) {
            return compile_error(&format!(
                "发现重复的 Shift_JIS 编码 0x{key_code:04X} 对应多个键（请检查映射）"
            ));
        }
        seen_codes.insert(key_code);

        // value -> utf16 codepoint（仅支持 BMP）
        let val_u32 = vc as u32;
        if val_u32 > 0xFFFF {
            return compile_error(&format!("映射值 '{vc}' 超过 BMP（>0xFFFF），目前不支持"));
        }
        let val_code: u16 = val_u32 as u16;

        entries.push((key_code, val_code, k, v));
    }

    // 排序（按 key 的编码）
    entries.sort_by_key(|e| e.0);

    // 生成文件 (替换原来的 HashMap 输出)
    let mut kv_tokens = Vec::new();
    for (kcode, vcode, _kch, _vch) in &entries {
        let k_lit = syn::LitInt::new(
            &format!("0x{:04X}u16", kcode),
            proc_macro2::Span::call_site(),
        );
        let v_lit = syn::LitInt::new(
            &format!("0x{:04X}u16", vcode),
            proc_macro2::Span::call_site(),
        );
        kv_tokens.push(quote! {
            #k_lit => #v_lit,
        });
    }

    let expanded = quote! {
        ::phf::phf_map! {
            #(#kv_tokens)*
        }
    };

    let final_ts = quote! {
        pub(super) static SJIS_PHF_MAP: ::phf::Map<u16, u16> = #expanded;
    };

    TokenStream::from(final_ts)
}

struct PathsInput {
    mapping: syn::LitStr,
    translated: Option<syn::LitStr>,
}

impl syn::parse::Parse for PathsInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mapping: syn::LitStr = input.parse()?;
        if input.is_empty() {
            return Ok(PathsInput {
                mapping,
                translated: None,
            });
        }
        let _comma: syn::Token![,] = input.parse()?;
        let translated: syn::LitStr = input.parse()?;
        Ok(PathsInput {
            mapping,
            translated: Some(translated),
        })
    }
}
