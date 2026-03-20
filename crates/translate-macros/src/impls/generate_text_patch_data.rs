use std::collections::BTreeMap;

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

    let mut dict_map: BTreeMap<String, String> = BTreeMap::new();
    let mut text_map: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    let mut text_index = 0usize;

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

            let (Some(orig), Some(trans)) = (
                r.get("message").and_then(|v| v.as_str()),
                t.get("message").and_then(|v| v.as_str()),
            ) else {
                continue;
            };

            if orig.is_empty() {
                continue;
            }

            let is_dict = r.get("is_name").and_then(|v| v.as_bool()) == Some(true)
                || r.get("is_dict").and_then(|v| v.as_bool()) == Some(true);

            if is_dict {
                match dict_map.get(orig) {
                    Some(existing) if existing != trans => {
                        syn_bail!(
                            &parsed.left,
                            "DICT 条目必须保持 1:1 映射，但 `{}` 出现了多个不同译文: `{}` / \
                             `{}`，文件: {}",
                            orig,
                            existing,
                            trans,
                            file_name.to_string_lossy()
                        );
                    }
                    Some(_) => {}
                    None => {
                        dict_map.insert(orig.to_string(), trans.to_string());
                    }
                }
                continue;
            }

            text_map
                .entry(orig.to_string())
                .or_default()
                .push((text_index, trans.to_string()));
            text_index += 1;
        }
    }

    if dict_map.is_empty() && text_map.is_empty() {
        syn_bail!(&parsed.left, "未找到任何JSON文件或文件内容为空");
    }

    let dict_entries = dict_map.into_iter().collect::<Vec<_>>();
    let mut single_entries = Vec::new();
    let mut multi_entries = Vec::new();

    for (orig, candidates) in text_map {
        if candidates.len() == 1 {
            let (index, trans) = &candidates[0];
            single_entries.push((orig, (*index, trans.clone())));
        } else {
            multi_entries.push((orig, candidates));
        }
    }

    let dict_phf_entries = dict_entries.iter().map(|(k, v)| {
        let k_lit = Literal::string(k);
        let v_lit = Literal::string(v);
        quote! { #k_lit => #v_lit }
    });

    let single_phf_entries = single_entries.iter().map(|(k, (index, v))| {
        let k_lit = Literal::string(k);
        let index_lit = Literal::usize_unsuffixed(*index);
        let v_lit = Literal::string(v);
        quote! { #k_lit => (#index_lit, #v_lit) }
    });

    let multi_phf_entries = multi_entries.iter().map(|(orig, candidates)| {
        let orig_lit = Literal::string(orig);
        let candidates = candidates.iter().map(|(index, trans)| {
            let index_lit = Literal::usize_unsuffixed(*index);
            let trans_lit = Literal::string(trans);
            quote! { (#index_lit, #trans_lit) }
        });

        quote! {
            #orig_lit => &[
                #(#candidates,)*
            ]
        }
    });

    let generated = quote! {
        #[derive(Clone, Copy)]
        pub(super) struct LookupResult {
            pub translated: &'static str,
            pub matched_index: Option<usize>,
        }

        /// 上下文无关的绝对 1:1 字典项（如名字、UI 固定词条）
        pub(super) static DICT_PHF: ::phf::Map<&'static str, &'static str> =
            ::phf::phf_map! {
                #(#dict_phf_entries, )*
            };

        /// 正文中的无歧义原文 -> (文本索引, 译文)
        pub(super) static TEXT_SINGLE_PHF: ::phf::Map<&'static str, (usize, &'static str)> =
            ::phf::phf_map! {
                #(#single_phf_entries, )*
            };

        /// 存在多个候选正文项的原文 -> [(文本索引, 译文)]
        ///
        /// 注意：这里按上下文位置保留全部候选项，即使多个候选项的译文文本完全相同，
        /// 也不会合并，因为它们对应的文本索引不同。
        pub(super) static TEXT_MULTI_PHF: ::phf::Map<&'static str, &'static [(usize, &'static str)]> =
            ::phf::phf_map! {
                #(#multi_phf_entries, )*
            };

        fn select_nearest<'a>(
            candidates: &'a [(usize, &'static str)],
            last_index: Option<usize>,
        ) -> Option<(usize, &'static str)> {
            match last_index {
                Some(last_index) => candidates.iter().copied().min_by_key(|(index, _)| {
                    (index.abs_diff(last_index), *index)
                }),
                None => candidates.first().copied(),
            }
        }

        /// 带上下文的统一查找接口：先查 DICT，再查正文 1:1，最后按最近索引查正文 1:N。
        pub(super) fn lookup_result(
            original: &str,
            last_index: Option<usize>,
        ) -> Option<LookupResult> {
            if let Some(translated) = DICT_PHF.get(original).copied() {
                return Some(LookupResult {
                    translated,
                    matched_index: None,
                });
            }

            if let Some((matched_index, translated)) = TEXT_SINGLE_PHF.get(original).copied() {
                return Some(LookupResult {
                    translated,
                    matched_index: Some(matched_index),
                });
            }

            let (matched_index, translated) = select_nearest(TEXT_MULTI_PHF.get(original)?, last_index)?;
            Some(LookupResult {
                translated,
                matched_index: Some(matched_index),
            })
        }

        /// 统一查找接口
        pub(super) fn lookup(original: &str) -> Option<&'static str> {
            lookup_result(original, None).map(|result| result.translated)
        }
    };

    Ok(generated)
}
