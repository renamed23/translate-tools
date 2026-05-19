use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::utils::{
    featured_hook::parse_cfg_expr, input::MultiPaths, read_optional_json_file,
    resolve_manifest_path,
};

#[derive(Deserialize)]
struct VfsRuleEntry {
    source: String,
    target: String,
    mode: String,
    #[serde(default)]
    cfg: Option<String>,
}

pub fn generate_vfs_rules(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<MultiPaths>(input)?;

    let mut all_rules = Vec::new();

    for path in parsed.paths.iter() {
        let abs_path = resolve_manifest_path(path)?;
        let rules: Vec<VfsRuleEntry> = read_optional_json_file(&abs_path)?;

        all_rules.extend(rules);
    }

    let entries = make_rule_entries(&all_rules)?;

    let output = quote! {
        pub const VFS_RULES: &[RawVfsRule] = &[
            #(#entries),*
        ];

    };

    Ok(output)
}

fn make_rule_entries(rules: &[VfsRuleEntry]) -> syn::Result<Vec<TokenStream>> {
    rules
        .iter()
        .map(|rule| {
            validate_pattern(&rule.source, "source")?;
            validate_pattern(&rule.target, "target")?;

            let source_capture_count = count_pattern_captures(&rule.source);
            let target_capture_count = count_pattern_captures(&rule.target);

            if source_capture_count != target_capture_count {
                syn_bail2!(
                    "VFS 规则 capture 数量不匹配:\nsource: `{}` ({} captures)\ntarget: `{}` ({} \
                     captures)\nsource 和 target 的 `*` / `**` 捕获数量必须完全一致",
                    rule.source,
                    source_capture_count,
                    rule.target,
                    target_capture_count,
                );
            }

            let mode = rule.mode.as_str();
            let source = &rule.source;
            let target = &rule.target;

            let entry = quote! {
                RawVfsRule {
                    source: #source,
                    target: #target,
                    mode: get_vfs_mode!(#mode),
                }
            };

            if let Some(cfg) = &rule.cfg {
                let cfg_expr = parse_cfg_expr(cfg)?;

                Ok(quote! {
                    #[cfg(#cfg_expr)]
                    #entry
                })
            } else {
                Ok(entry)
            }
        })
        .collect()
}

/// 统计 pattern 中的 capture 数量
///
/// 规则:
/// - `*` => 1 capture
/// - `abc*def` => 1 capture
/// - `*.*` => 2 captures
/// - `**` => 1 capture
/// - Literal => 0
fn count_pattern_captures(pattern: &str) -> usize {
    pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            if seg == "**" {
                1
            } else if seg == "*.*" {
                2
            } else if seg.contains('*') {
                1
            } else {
                0
            }
        })
        .sum()
}

/// 编译期校验 VFS 路径模式, 拒绝所有会导致运行时 pattern 解析出错的非法写法。
///
/// 规则:
/// - 路径分隔符必须严格使用 `/`, 禁止 `\\`
/// - 整个模式中 `**` 最多出现一次
/// - 含 `*` 的非 `**` 段(glob 段), 允许一个 `*` (`*.ext`, `name.*`), 或两个 `*` 仅限 `*.*`
/// - 不含 `*` 的字面量段不能出现 `*`
fn validate_pattern(pattern: &str, field_name: &str) -> syn::Result<()> {
    if pattern.contains('\\') {
        syn_bail2!("{field_name} 路径 `{pattern}` 包含非法分隔符 `\\`, 请使用 `/`");
    }

    if pattern.is_empty() {
        syn_bail2!("{field_name} 路径不能为空");
    }

    let segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();

    let recursive_wild_count = segments.iter().filter(|s| **s == "**").count();
    if recursive_wild_count > 1 {
        syn_bail2!(
            "{field_name} 路径 `{pattern}` 包含 {recursive_wild_count} 个 `**`, 最多允许一个"
        );
    }

    for (i, seg) in segments.iter().enumerate() {
        let star_count = seg.bytes().filter(|b| *b == b'*').count();

        if *seg == "**" {
            continue;
        }

        match star_count {
            0 => {}
            1 => {}
            2 if *seg == "*.*" => {}
            n => {
                syn_bail2!(
                    "{field_name} 路径 `{pattern}` 第 {} 段 `{seg}` 包含 {n} 个 `*`, 只允许 \
                     `*.ext`, `name.*`, `*.*` 等写法",
                    i + 1
                );
            }
        }
    }

    Ok(())
}
