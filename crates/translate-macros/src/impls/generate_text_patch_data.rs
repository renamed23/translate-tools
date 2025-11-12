use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

use crate::utils::get_full_path_by_manifest;

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

    let raw_json = get_full_path_by_manifest(parsed.raw.value())
        .map_err(|e| syn_err!(&parsed.raw, "解析原始文件路径失败: {e}"))?;
    let translated_json = get_full_path_by_manifest(parsed.translated.value())
        .map_err(|e| syn_err!(&parsed.translated, "解析翻译文件路径失败: {e}"))?;

    // 读取文件
    let raw_data = std::fs::read_to_string(&raw_json)
        .map_err(|e| syn_err!(&parsed.raw, "读取原始JSON文件失败: {e}"))?;
    let trans_data = std::fs::read_to_string(&translated_json)
        .map_err(|e| syn_err!(&parsed.translated, "读取翻译JSON文件失败: {e}"))?;

    // 解析为 JSON 数组
    let raw_vals: serde_json::Value = serde_json::from_str(&raw_data)
        .map_err(|e| syn_err!(&parsed.raw, "解析原始JSON数据失败: {e}"))?;
    let trans_vals: serde_json::Value = serde_json::from_str(&trans_data)
        .map_err(|e| syn_err!(&parsed.translated, "解析翻译JSON数据失败: {e}"))?;

    let raw_arr = raw_vals
        .as_array()
        .ok_or_else(|| syn_err!(&parsed.raw, "原始JSON应为数组格式"))?;
    let trans_arr = trans_vals
        .as_array()
        .ok_or_else(|| syn_err!(&parsed.translated, "翻译JSON应为数组格式"))?;

    let len = raw_arr.len();

    if raw_arr.len() != trans_arr.len() {
        syn_bail2!(
            "原文数组({})和译文数组({})数量不相等",
            raw_arr.len(),
            trans_arr.len()
        )
    }

    // 分开去重
    let mut seen_names = std::collections::HashSet::new();
    let mut seen_msgs = std::collections::HashSet::new();

    let mut name_map = Vec::new(); // orig_name -> trans_name
    let mut msg_map = Vec::new(); // orig_msg -> trans_msg

    for i in 0..len {
        let r = &raw_arr[i];
        let t = &trans_arr[i];

        // 处理名字：分开去重
        if let (Some(orig_name), Some(trans_name)) = (
            r.get("name").and_then(|v| v.as_str()),
            t.get("name").and_then(|v| v.as_str()),
        ) && !orig_name.is_empty()
            && seen_names.insert(orig_name.to_string())
        {
            name_map.push((orig_name.to_string(), trans_name.to_string()));
        }

        // 处理句子：分开去重
        if let (Some(orig_msg), Some(trans_msg)) = (
            r.get("message").and_then(|v| v.as_str()),
            t.get("message").and_then(|v| v.as_str()),
        ) && !orig_msg.is_empty()
            && seen_msgs.insert(orig_msg.to_string())
        {
            msg_map.push((orig_msg.to_string(), trans_msg.to_string()));
        }
    }

    // 生成 phf 条目
    let name_phf_entries = name_map.iter().map(|(k, v)| {
        let k_lit = Literal::string(k);
        let v_lit = Literal::string(v);
        quote! { #k_lit => #v_lit }
    });

    let msg_phf_entries = msg_map.iter().map(|(k, v)| {
        let k_lit = Literal::string(k);
        let v_lit = Literal::string(v);
        quote! { #k_lit => #v_lit }
    });

    // 输出最终 phf 映射和查找函数
    let generated = quote! {
        /// 原名 -> 译名
        pub(super) static NAME_PHF: ::phf::Map<&'static str, &'static str> = ::phf::phf_map! {
            #(#name_phf_entries, )*
        };

        /// 原句 -> 译句
        pub(super) static MSG_PHF: ::phf::Map<&'static str, &'static str> = ::phf::phf_map! {
            #(#msg_phf_entries, )*
        };

        /// 查名字
        pub(super) fn lookup_name(original_name: &str) -> Option<&'static str> {
            NAME_PHF.get(original_name).copied()
        }

        /// 查句子
        pub(super) fn lookup_message(original_message: &str) -> Option<&'static str> {
            MSG_PHF.get(original_message).copied()
        }
    };

    Ok(generated)
}
