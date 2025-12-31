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

    let raw_dir = get_full_path_by_manifest(parsed.raw.value())
        .map_err(|e| syn_err!(&parsed.raw, "解析原始文件夹路径失败: {e}"))?;
    let translated_dir = get_full_path_by_manifest(parsed.translated.value())
        .map_err(|e| syn_err!(&parsed.translated, "解析翻译文件夹路径失败: {e}"))?;

    // 检查路径是否为目录
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

    // 获取原始文件夹中的所有json文件
    let raw_entries = std::fs::read_dir(&raw_dir)
        .map_err(|e| syn_err!(&parsed.raw, "读取原始文件夹失败: {e}"))?;

    let mut name_map = Vec::new();
    let mut msg_map = Vec::new();

    // 全局的去重集合
    let mut seen_names = std::collections::HashSet::new();
    let mut seen_msgs = std::collections::HashSet::new();

    for entry in raw_entries {
        let entry = entry.map_err(|e| syn_err!(&parsed.raw, "读取文件夹条目失败: {e}"))?;
        let raw_path = entry.path();

        // 获取文件名
        let file_name = raw_path
            .file_name()
            .ok_or_else(|| syn_err!(&parsed.raw, "无法获取文件名"))?;

        // 构建翻译文件路径
        let trans_path = translated_dir.join(file_name);

        // 检查翻译文件是否存在
        if !trans_path.exists() {
            syn_bail!(
                &parsed.raw,
                "找不到对应的翻译文件: {}",
                trans_path.display()
            );
        }

        // 读取文件内容
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

        // 解析为 JSON 数组
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

        // 检查数组长度
        if raw_arr.len() != trans_arr.len() {
            syn_bail!(
                &parsed.raw,
                "原文数组({})和译文数组({})数量不相等，文件: {}",
                raw_arr.len(),
                trans_arr.len(),
                file_name.to_string_lossy()
            );
        }

        // 处理每个文件中的数据
        for i in 0..raw_arr.len() {
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
    }

    // 检查是否处理了文件
    if name_map.is_empty() && msg_map.is_empty() {
        syn_bail!(&parsed.raw, "未找到任何JSON文件或文件内容为空");
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
