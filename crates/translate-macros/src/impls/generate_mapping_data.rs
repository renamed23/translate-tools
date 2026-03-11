use proc_macro2::{Span, TokenStream};
use quote::quote;
use serde::Deserialize;
use std::collections::HashMap;
use syn::{
    LitInt, LitStr,
    parse::{Parse, ParseStream},
};

use crate::impls::utils::get_full_path_by_manifest;

struct PathInput {
    mapping: LitStr,
}

impl Parse for PathInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mapping: LitStr = input.parse()?;
        Ok(PathInput { mapping })
    }
}

#[derive(Deserialize)]
struct MappingConfig {
    #[serde(default)]
    code_page: Option<u32>,
    #[serde(default)]
    src_encoding: Option<String>,
    mapping: HashMap<char, char>,
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

    let mapping_path = get_full_path_by_manifest(parsed.mapping.value())?;

    let mapping_str = std::fs::read_to_string(&mapping_path)
        .map_err(|e| syn_err2!("无法读取 {}: {}", mapping_path.display(), e))?;

    let config: MappingConfig = serde_json::from_str(&mapping_str)
        .map_err(|e| syn_err2!("解析 {} 失败: {}", mapping_path.display(), e))?;

    // 确定代码页
    let code_page = if let Some(cp) = config.code_page {
        cp
    } else if let Some(encoding) = config.src_encoding {
        get_code_page_from_src_encoding(&encoding)?
    } else {
        0
    };

    // 构建映射并校验 BMP 范围
    let mut entries: Vec<(u16, u16)> = Vec::new();
    for (k, v) in config.mapping {
        if (k as u32) > 0xFFFF || (v as u32) > 0xFFFF {
            syn_bail2!("检测到超出 BMP 范围的字符: '{k}' -> '{v}'");
        }
        entries.push((k as u16, v as u16));
    }

    entries.sort_by_key(|e| e.0);

    // 生成 phf tokens
    let kv_tokens: Vec<_> = entries
        .iter()
        .map(|(k, v)| {
            let k_lit = LitInt::new(&format!("0x{:04X}u16", k), Span::call_site());
            let v_lit = LitInt::new(&format!("0x{:04X}u16", v), Span::call_site());
            quote! { #k_lit => #v_lit, }
        })
        .collect();

    let phf_expanded = quote! { ::phf::phf_map! { #(#kv_tokens)* } };
    let code_page_lit = LitInt::new(&code_page.to_string(), Span::call_site());

    Ok(quote! {
        pub(super) static ANSI_CODE_PAGE: u32 = #code_page_lit;
        pub(super) static PHF_MAP: ::phf::Map<u16, u16> = #phf_expanded;
    })
}
