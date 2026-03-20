use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::impls::utils::{
    ArrowSeparatedPaths, collect_files_in_dir, ensure_dir, read_file_string, resolve_manifest_path,
};

pub fn generate_text_patch_data(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<ArrowSeparatedPaths>(input)?;

    let raw_dir = resolve_manifest_path(&parsed.left)
        .map_err(|e| syn_err!(&parsed.left, "解析原始文件夹路径失败: {e}"))?;
    let translated_dir = resolve_manifest_path(&parsed.right)
        .map_err(|e| syn_err!(&parsed.right, "解析翻译文件夹路径失败: {e}"))?;

    ensure_dir(&raw_dir, &parsed.left)?;
    ensure_dir(&translated_dir, &parsed.right)?;

    let raw_entries = collect_files_in_dir(&raw_dir)
        .map_err(|e| syn_err!(&parsed.left, "读取原始文件夹失败: {e}"))?;

    let mut text_map = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for raw_path in raw_entries {
        let file_name = raw_path
            .file_name()
            .ok_or_else(|| syn_err!(&parsed.left, "无法获取文件名"))?;

        let trans_path = translated_dir.join(file_name);

        if !trans_path.exists() {
            syn_bail!(
                &parsed.left,
                "找不到对应的翻译文件: {}",
                trans_path.display()
            );
        }

        let raw_data = read_file_string(&raw_path).map_err(|e| {
            syn_err!(
                &parsed.left,
                "读取原始JSON文件失败: {} - {e}",
                raw_path.display()
            )
        })?;

        let trans_data = read_file_string(&trans_path).map_err(|e| {
            syn_err!(
                &parsed.right,
                "读取翻译JSON文件失败: {} - {e}",
                trans_path.display()
            )
        })?;

        let raw_vals: serde_json::Value = serde_json::from_str(&raw_data).map_err(|e| {
            syn_err!(
                &parsed.left,
                "解析原始JSON数据失败: {} - {e}",
                raw_path.display()
            )
        })?;

        let trans_vals: serde_json::Value = serde_json::from_str(&trans_data).map_err(|e| {
            syn_err!(
                &parsed.right,
                "解析翻译JSON数据失败: {} - {e}",
                trans_path.display()
            )
        })?;

        let raw_arr = raw_vals.as_array().ok_or_else(|| {
            syn_err!(&parsed.left, "原始JSON应为数组格式: {}", raw_path.display())
        })?;

        let trans_arr = trans_vals.as_array().ok_or_else(|| {
            syn_err!(
                &parsed.right,
                "翻译JSON应为数组格式: {}",
                trans_path.display()
            )
        })?;

        if raw_arr.len() != trans_arr.len() {
            syn_bail!(
                &parsed.left,
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
        syn_bail!(&parsed.left, "未找到任何JSON文件或文件内容为空");
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
