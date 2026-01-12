use proc_macro2::{Span, TokenStream};
use quote::quote;
use serde_json::Value;
use std::collections::HashMap;
use syn::{
    LitInt, LitStr,
    parse::{Parse, ParseStream},
};

use crate::utils::get_full_path_by_manifest;

struct PathInput {
    mapping: LitStr,
}

impl Parse for PathInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mapping: LitStr = input.parse()?;
        Ok(PathInput { mapping })
    }
}

fn get_code_page_from_src_encoding(src_encoding: &str) -> syn::Result<u32> {
    match src_encoding {
        "ShiftJIS" | "CP932" => Ok(932),
        "GBK" => Ok(936),
        _ => syn_bail2!("不支持的 src_encoding: {}", src_encoding),
    }
}

pub fn generate_mapping_data(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathInput>(input)?;

    let mapping_path = get_full_path_by_manifest(parsed.mapping.value()).unwrap();

    let mapping_str = std::fs::read_to_string(&mapping_path)
        .map_err(|e| syn_err2!("无法读取 {}: {}", mapping_path.display(), e))?;

    let json_value: Value = serde_json::from_str(&mapping_str)
        .map_err(|e| syn_err2!("解析 {} 失败: {}", mapping_path.display(), e))?;

    // 获取映射数据
    let mapping_obj = json_value["mapping"]
        .as_object()
        .ok_or_else(|| syn_err2!("mapping 字段必须是对象"))?;

    // 确定代码页：优先使用 code_page 字段，否则根据编码推断
    // 若都没有则使用默认值0
    let code_page = if let Some(cp_value) = json_value.get("code_page") {
        cp_value
            .as_u64()
            .ok_or_else(|| syn_err2!("code_page 必须是数字"))? as u32
    } else if let Some(src_encoding) = json_value["src_encoding"].as_str() {
        get_code_page_from_src_encoding(src_encoding)?
    } else {
        0
    };

    // ----------------------------
    // 构建 u16 映射表
    // ----------------------------
    let mut char_map: HashMap<u16, u16> = HashMap::new();

    for (k, v) in mapping_obj {
        let v_str = v
            .as_str()
            .ok_or_else(|| syn_err2!("映射值必须是字符串: {v:?}"))?;

        if k.chars().count() != 1 {
            syn_bail2!("映射键必须是单个字符，发现: {k:?}");
        }
        if v_str.chars().count() != 1 {
            syn_bail2!("映射值必须是单个字符，发现: {v_str:?}");
        }

        let kc = k.chars().next().unwrap();
        let vc = v_str.chars().next().unwrap();

        if kc > '\u{FFFF}' {
            syn_bail2!(
                "字符 '{kc}' (U+{:04X}) 超过 BMP（>0xFFFF），目前不支持",
                kc as u32
            );
        }

        if vc > '\u{FFFF}' {
            syn_bail2!(
                "字符 '{vc}' (U+{:04X}) 超过 BMP（>0xFFFF），目前不支持",
                vc as u32
            );
        }

        let key_code = kc as u16;
        let val_code = vc as u16;

        if char_map.contains_key(&key_code) {
            syn_bail2!("发现重复的键 0x{key_code:04X} 对应多个映射");
        }
        char_map.insert(key_code, val_code);
    }

    // 转换为排序的 Vec 以生成 phf tokens
    let mut entries: Vec<(u16, u16)> = char_map.into_iter().collect();
    entries.sort_by_key(|e| e.0);

    // 生成 phf map tokens
    let mut kv_tokens = Vec::new();
    for (kcode, vcode) in &entries {
        let k_lit = LitInt::new(&format!("0x{:04X}u16", kcode), Span::call_site());
        let v_lit = LitInt::new(&format!("0x{:04X}u16", vcode), Span::call_site());
        kv_tokens.push(quote! {
            #k_lit => #v_lit,
        });
    }

    let phf_expanded = quote! {
        ::phf::phf_map! {
            #(#kv_tokens)*
        }
    };

    let code_page_lit = LitInt::new(&code_page.to_string(), Span::call_site());

    let final_ts = quote! {
        pub(super) static ANSI_CODE_PAGE: u32 = #code_page_lit;
        pub(super) static PHF_MAP: ::phf::Map<u16, u16> = #phf_expanded;
    };

    Ok(final_ts)
}
