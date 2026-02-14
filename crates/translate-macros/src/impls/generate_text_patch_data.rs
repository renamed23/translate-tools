use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

use crate::impls::utils::get_full_path_by_manifest;

struct PathsInput {
    raw: LitStr,
    translated: LitStr,
}

impl Parse for PathsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let raw: LitStr = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let translated: LitStr = input.parse()?;
        Ok(PathsInput { raw, translated })
    }
}

pub fn generate_text_patch_data(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathsInput>(input)?;

    let raw_dir = get_full_path_by_manifest(parsed.raw.value())
        .map_err(|e| syn_err!(&parsed.raw, "解析原始文件夹路径失败: {e}"))?;
    let translated_dir = get_full_path_by_manifest(parsed.translated.value())
        .map_err(|e| syn_err!(&parsed.translated, "解析翻译文件夹路径失败: {e}"))?;

    if !raw_dir.is_dir() {
        syn_bail!(&parsed.raw, "原始路径不是文件夹: {}", raw_dir.display());
    }
    if !translated_dir.is_dir() {
        syn_bail!(
            &parsed.translated,
            "翻译路径不是文件夹: {}",
            translated_dir.display()
        );
    }

    let raw_entries = std::fs::read_dir(&raw_dir)
        .map_err(|e| syn_err!(&parsed.raw, "读取原始文件夹失败: {e}"))?;

    let mut text_map = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for entry in raw_entries {
        let entry = entry.map_err(|e| syn_err!(&parsed.raw, "读取文件夹条目失败: {e}"))?;
        let raw_path = entry.path();

        let file_name = raw_path
            .file_name()
            .ok_or_else(|| syn_err!(&parsed.raw, "无法获取文件名"))?;

        let trans_path = translated_dir.join(file_name);

        if !trans_path.exists() {
            syn_bail!(
                &parsed.raw,
                "找不到对应的翻译文件: {}",
                trans_path.display()
            );
        }

        let raw_data = std::fs::read_to_string(&raw_path).map_err(|e| {
            syn_err!(
                &parsed.raw,
                "读取原始JSON文件失败: {} - {e}",
                raw_path.display()
            )
        })?;

        let trans_data = std::fs::read_to_string(&trans_path).map_err(|e| {
            syn_err!(
                &parsed.translated,
                "读取翻译JSON文件失败: {} - {e}",
                trans_path.display()
            )
        })?;

        let raw_vals: serde_json::Value = serde_json::from_str(&raw_data).map_err(|e| {
            syn_err!(
                &parsed.raw,
                "解析原始JSON数据失败: {} - {e}",
                raw_path.display()
            )
        })?;

        let trans_vals: serde_json::Value = serde_json::from_str(&trans_data).map_err(|e| {
            syn_err!(
                &parsed.translated,
                "解析翻译JSON数据失败: {} - {e}",
                trans_path.display()
            )
        })?;

        let raw_arr = raw_vals
            .as_array()
            .ok_or_else(|| syn_err!(&parsed.raw, "原始JSON应为数组格式: {}", raw_path.display()))?;

        let trans_arr = trans_vals.as_array().ok_or_else(|| {
            syn_err!(
                &parsed.translated,
                "翻译JSON应为数组格式: {}",
                trans_path.display()
            )
        })?;

        if raw_arr.len() != trans_arr.len() {
            syn_bail!(
                &parsed.raw,
                "原文数组({})和译文数组({})数量不相等，文件: {}",
                raw_arr.len(),
                trans_arr.len(),
                file_name.to_string_lossy()
            );
        }

        for i in 0..raw_arr.len() {
            let r = &raw_arr[i];
            let t = &trans_arr[i];

            for field in ["name", "message"] {
                if let (Some(orig), Some(trans)) = (
                    r.get(field).and_then(|v| v.as_str()),
                    t.get(field).and_then(|v| v.as_str()),
                ) && !orig.is_empty()
                    && seen.insert(orig.to_string())
                {
                    text_map.push((orig.to_string(), trans.to_string()));
                }
            }
        }
    }

    if text_map.is_empty() {
        syn_bail!(&parsed.raw, "未找到任何JSON文件或文件内容为空");
    }

    let phf_entries = text_map.iter().map(|(k, v)| {
        let k_lit = Literal::string(k);
        let v_lit = Literal::string(v);
        quote! { #k_lit => #v_lit }
    });

    let generated = quote! {
        /// 原文 -> 译文
        pub(super) static TEXT_PHF: ::phf::Map<&'static str, &'static str> =
            ::phf::phf_map! {
                #(#phf_entries, )*
            };

        /// 统一查找接口
        pub(super) fn lookup(original: &str) -> Option<&'static str> {
            TEXT_PHF.get(original).copied()
        }
    };

    Ok(generated)
}
