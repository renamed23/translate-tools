use proc_macro::TokenStream;
use quote::quote;
use std::{collections::HashMap, fs, path::PathBuf};
use translate_utils::{jis0208::is_jis0208, text::Text};

use crate::utils::compile_error;

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

    // ----------------------------
    // 公共函数闭包
    // ----------------------------
    let encode_sjis_u16 = |ch: char| -> Result<u16, String> {
        let s = ch.to_string();
        let (enc, _, had_errors) = encoding_rs::SHIFT_JIS.encode(&s);
        if had_errors {
            return Err(format!("字符 '{}' 编码为 Shift_JIS 时出现错误", s));
        }
        if enc.len() != 2 {
            return Err(format!(
                "字符 '{}' 编码为 Shift_JIS 后长度异常: {}",
                s,
                enc.len()
            ));
        }
        Ok(((enc[0] as u16) << 8) | (enc[1] as u16))
    };

    let char_to_u16 = |ch: char| -> Result<u16, String> {
        let u = ch as u32;
        if u > 0xFFFF {
            return Err(format!("字符 '{}' 超过 BMP（>0xFFFF），目前不支持", ch));
        }
        Ok(u as u16)
    };

    // ----------------------------
    // 1) 先用 mapping.json 构造 UTF16 表（只来自 mapping.json）
    // ----------------------------
    let mut utf16_map: HashMap<u16, u16> = HashMap::new();
    for (k, v) in map.iter() {
        if k.chars().count() != 1 {
            return compile_error(&format!("映射键必须是单个字符，发现: {:?}", k));
        }
        if v.chars().count() != 1 {
            return compile_error(&format!("映射值必须是单个字符，发现: {:?}", v));
        }
        let kc = k.chars().next().unwrap();
        let vc = v.chars().next().unwrap();

        let key_code_utf16 = match char_to_u16(kc) {
            Ok(v) => v,
            Err(e) => return compile_error(&format!("generate_mapping_data: {}", e)),
        };
        let val_code_utf16 = match char_to_u16(vc) {
            Ok(v) => v,
            Err(e) => return compile_error(&format!("generate_mapping_data: {}", e)),
        };

        if utf16_map.contains_key(&key_code_utf16) {
            return compile_error(&format!(
                "发现重复的 UTF-16 键 0x{key_code_utf16:04X} 对应多个映射（请检查映射）"
            ));
        }
        utf16_map.insert(key_code_utf16, val_code_utf16);
    }

    // ----------------------------
    // 2) 构造 SJIS 表：先把 translated_path 的自映射加入（如果有），然后用 mapping.json 覆盖/添加
    // ----------------------------
    let mut sjis_map: HashMap<u16, u16> = HashMap::new();

    if let Some(translated_path) = &translated_path {
        if !translated_path.exists() {
            return compile_error(&format!(
                "generate_mapping_data: GENERATE_FULL_MAPPING_DATA 开启，但是 {} 未找到",
                translated_path.display()
            ));
        }

        let text = match Text::from_path(translated_path) {
            Ok(s) => s,
            Err(e) => {
                return compile_error(&format!(
                    "generate_mapping_data: 无法读取 {}: {}",
                    translated_path.display(),
                    e
                ));
            }
        };

        let all_chars = text.get_filtered_chars(is_jis0208);
        for ch in all_chars {
            let key_code = match encode_sjis_u16(ch) {
                Ok(v) => v,
                Err(e) => return compile_error(&format!("generate_mapping_data: {}", e)),
            };
            let val_code = match char_to_u16(ch) {
                Ok(v) => v,
                Err(e) => return compile_error(&format!("generate_mapping_data: {}", e)),
            };
            // 插入自映射，后续会被 mapping.json 覆盖（如果存在相同编码）
            sjis_map.insert(key_code, val_code);
        }
    }

    // 用 mapping.json 覆盖/添加 SJIS 条目（如果 key 可编码为 Shift_JIS）
    for (k, v) in map.iter() {
        let kc = {
            if k.chars().count() != 1 {
                return compile_error(&format!("映射键必须是单个字符，发现: {:?}", k));
            }
            k.chars().next().unwrap()
        };
        let vc = {
            if v.chars().count() != 1 {
                return compile_error(&format!("映射值必须是单个字符，发现: {:?}", v));
            }
            v.chars().next().unwrap()
        };

        // 使用 is_jis0208 判断 key 是否为 JIS0208（可被 Shift_JIS 编码）
        if !is_jis0208(kc) {
            return compile_error(&format!(
                "映射键 '{kc}' 不是 JIS0208（不可被 Shift_JIS 编码）"
            ));
        }

        let key_code = match encode_sjis_u16(kc) {
            Ok(v) => v,
            Err(e) => return compile_error(&format!("generate_mapping_data: {}", e)),
        };
        let val_code = match char_to_u16(vc) {
            Ok(v) => v,
            Err(e) => return compile_error(&format!("generate_mapping_data: {}", e)),
        };
        // mapping.json 优先覆盖自映射或之前的条目
        sjis_map.insert(key_code, val_code);
    }

    // 把 sjis_map 和 utf16_map 转为排序的 Vec 以生成 phf tokens
    let mut sjis_entries: Vec<(u16, u16)> = sjis_map.into_iter().collect();
    sjis_entries.sort_by_key(|e| e.0);

    let mut utf16_entries: Vec<(u16, u16)> = utf16_map.into_iter().collect();
    utf16_entries.sort_by_key(|e| e.0);

    // 生成 SJIS phf map tokens
    let mut sjis_kv_tokens = Vec::new();
    for (kcode, vcode) in &sjis_entries {
        let k_lit = syn::LitInt::new(
            &format!("0x{:04X}u16", kcode),
            proc_macro2::Span::call_site(),
        );
        let v_lit = syn::LitInt::new(
            &format!("0x{:04X}u16", vcode),
            proc_macro2::Span::call_site(),
        );
        sjis_kv_tokens.push(quote! {
            #k_lit => #v_lit,
        });
    }

    // 生成 UTF-16 phf map tokens
    let mut utf16_kv_tokens = Vec::new();
    for (kcode, vcode) in &utf16_entries {
        let k_lit = syn::LitInt::new(
            &format!("0x{:04X}u16", kcode),
            proc_macro2::Span::call_site(),
        );
        let v_lit = syn::LitInt::new(
            &format!("0x{:04X}u16", vcode),
            proc_macro2::Span::call_site(),
        );
        utf16_kv_tokens.push(quote! {
            #k_lit => #v_lit,
        });
    }

    let sjis_expanded = quote! {
        ::phf::phf_map! {
            #(#sjis_kv_tokens)*
        }
    };

    let utf16_expanded = quote! {
        ::phf::phf_map! {
            #(#utf16_kv_tokens)*
        }
    };

    let final_ts = quote! {
        pub(super) static SJIS_PHF_MAP: ::phf::Map<u16, u16> = #sjis_expanded;
        pub(super) static UTF16_PHF_MAP: ::phf::Map<u16, u16> = #utf16_expanded;
    };

    TokenStream::from(final_ts)
}
