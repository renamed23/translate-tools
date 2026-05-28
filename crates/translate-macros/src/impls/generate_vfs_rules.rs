use std::collections::{HashMap, HashSet};

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
    #[serde(default)]
    create_dirs: Option<Vec<String>>,
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
    let create_dirs = collect_create_dirs(&all_rules)?;

    let output = quote! {
        pub const VFS_RULES: &[RawVfsRule] = &[
            #(#entries),*
        ];

        pub const CREATE_DIRS: &[&str] = &[
            #(#create_dirs),*
        ];
    };

    Ok(output)
}

/// 将 mode 字符串解析为 `VfsMode` 枚举变体的 TokenStream。
fn resolve_vfs_mode(mode: &str) -> syn::Result<TokenStream> {
    match mode {
        "fallback" => Ok(quote! { VfsMode::Fallback }),
        "force" => Ok(quote! { VfsMode::Force }),
        other => syn_bail2!(
            "非法的 VfsMode: `{}`, 期望 `fallback` 或 `force`",
            other
        ),
    }
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

            let mode_ident = resolve_vfs_mode(&rule.mode)?;
            let source = &rule.source;
            let target = &rule.target;

            let entry = quote! {
                RawVfsRule {
                    source: #source,
                    target: #target,
                    mode: #mode_ident,
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

/// 编译期收集并去重 `create_dirs`，生成 `CREATE_DIRS` 常量数组的 token 序列。
///
/// 规则:
/// - 同一 `(cfg, path)` 去重（一个 path 下给定 cfg 只保留一次）
/// - 不同 `cfg` 下的同一 path 各自保留（`any(a, b)` 语义）
/// - 无条件条目覆盖条件条目：若存在无 `cfg` 的 path，则丢弃所有带 `cfg` 的同 path 项
fn collect_create_dirs(rules: &[VfsRuleEntry]) -> syn::Result<Vec<TokenStream>> {
    let mut seen: HashSet<(Option<&str>, &str)> = HashSet::new();
    let mut unconditional: HashSet<&str> = HashSet::new();
    let mut items: Vec<(Option<&str>, &str)> = Vec::new();

    // 第一步：先收集无条件项，并做合法性校验与基本去重
    for rule in rules {
        if let Some(ref dirs) = rule.create_dirs {
            let cfg_ref: Option<&str> = rule.cfg.as_deref();
            for dir in dirs {
                let dir_str = dir.as_str();
                validate_dir_path(dir_str)?;

                if cfg_ref.is_none() {
                    unconditional.insert(dir_str);
                }
            }
        }
    }

    // 第二步：严格按照用户原始物理顺序进行有效条目装载，确保生成顺序100%稳定
    for rule in rules {
        if let Some(ref dirs) = rule.create_dirs {
            let cfg_ref: Option<&str> = rule.cfg.as_deref();
            for dir in dirs {
                let dir_str = dir.as_str();

                // 如果已经有无条件项，带条件的直接不进队列
                if cfg_ref.is_some() && unconditional.contains(dir_str) {
                    continue;
                }

                // 依靠插入判定进行严格的 (cfg, path) 物理去重
                if seen.insert((cfg_ref, dir_str)) {
                    items.push((cfg_ref, dir_str));
                }
            }
        }
    }

    let mut tokens = Vec::with_capacity(items.len());
    let mut cfg_cache: HashMap<&str, TokenStream> = HashMap::new();

    for (cfg, dir) in items {
        if let Some(cfg_str) = cfg {
            let cfg_expr = if let Some(ts) = cfg_cache.get(cfg_str) {
                ts.clone()
            } else {
                let ts = parse_cfg_expr(cfg_str)?;
                cfg_cache.insert(cfg_str, ts.clone());
                ts
            };

            tokens.push(quote! {
                #[cfg(#cfg_expr)]
                #dir
            });
        } else {
            tokens.push(quote! {
                #dir
            });
        }
    }

    Ok(tokens)
}

const ALLOWED_PATH_VARS: &[&str] = &["cwd", "temp_dir", "exe_dir", "resource_pack_dir"];

/// 校验路径中的 `{var}` 占位符是否合法。
///
/// 允许:
/// - `{cwd}`
/// - `{temp_dir}`
/// - `{exe_dir}`
/// - `{resource_pack_dir}`
///
/// 禁止:
/// - 未知变量
/// - 空变量 `{}`
/// - 未闭合 `{abc`
/// - 多余 `}`
/// - 嵌套 `{a{b}}`
fn validate_path_vars(path: &str, field_name: &str) -> syn::Result<()> {
    let mut chars = path.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        match c {
            '{' => {
                // 寻找闭合的 '}'
                let start = i + 1;
                let mut end = None;

                // 检查内部是否有嵌套的 '{' 或者直接找到 '}'
                while let Some(&(next_i, next_c)) = chars.peek() {
                    if next_c == '{' {
                        syn_bail2!("{field_name} 路径 `{path}` 包含非法嵌套 `{{`");
                    }
                    if next_c == '}' {
                        end = Some(next_i);
                        chars.next(); // 消耗掉 '}'
                        break;
                    }
                    chars.next();
                }

                let end_i = match end {
                    Some(pos) => pos,
                    None => syn_bail2!("{field_name} 路径 `{path}` 中的变量未闭合"),
                };

                // 因为是通过 char_indices 拿到的物理位置，切片绝对安全
                let var = &path[start..end_i];

                if var.is_empty() {
                    syn_bail2!("{field_name} 路径 `{path}` 包含空变量 `{{}}`");
                }

                if !ALLOWED_PATH_VARS.contains(&var) {
                    syn_bail2!(
                        "{field_name} 路径 `{path}` 包含未知变量 `{{{var}}}`，允许的变量: {}",
                        ALLOWED_PATH_VARS.join(", ")
                    );
                }
            }
            '}' => {
                syn_bail2!("{field_name} 路径 `{path}` 包含未匹配的 `}}`");
            }
            _ => {}
        }
    }

    Ok(())
}

/// 校验目录路径：
///
/// 规则:
/// - 禁止 `*`（glob 无意义）
/// - 禁止 `\\`
/// - 禁止空串
/// - 只允许白名单 `{var}`
fn validate_dir_path(path: &str) -> syn::Result<()> {
    validate_path_vars(path, "create_dirs")?;

    if path.is_empty() {
        syn_bail2!("create_dirs 路径不能为空");
    }
    if path.contains('\\') {
        syn_bail2!("create_dirs 路径 `{path}` 包含非法分隔符 `\\`, 请使用 `/`");
    }
    if path.contains('*') {
        syn_bail2!("create_dirs 路径 `{path}` 包含非法通配符 `*`, 目录路径不支持 glob");
    }
    Ok(())
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

/// 编译期校验 VFS 路径模式，拒绝所有会导致运行时 pattern 解析出错的非法写法。
///
/// 规则:
/// - 路径分隔符必须严格使用 `/`, 禁止 `\\`
/// - 整个模式中 `**` 最多出现一次
/// - 含 `*` 的非 `**` 段(glob 段), 允许:
///   - 一个 `*` (`*.ext`, `name.*`)
///   - 两个 `*` 仅限 `*.*`
/// - 不含 `*` 的字面量段不能出现 `*`
/// - 只允许白名单 `{var}`
fn validate_pattern(pattern: &str, field_name: &str) -> syn::Result<()> {
    validate_path_vars(pattern, field_name)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_pattern_captures() {
        // 0 捕获
        assert_eq!(count_pattern_captures("abc/def"), 0);
        assert_eq!(count_pattern_captures(""), 0);

        // 1 捕获
        assert_eq!(count_pattern_captures("abc/*"), 1);
        assert_eq!(count_pattern_captures("*.ext"), 1);
        assert_eq!(count_pattern_captures("abc*def"), 1);
        assert_eq!(count_pattern_captures("**"), 1);
        assert_eq!(count_pattern_captures("a/**/b"), 1);

        // 2 捕获
        assert_eq!(count_pattern_captures("*.*"), 2);
        assert_eq!(count_pattern_captures("a/*.*"), 2);
        assert_eq!(count_pattern_captures("a/*/*"), 2);

        // 混合捕获
        assert_eq!(count_pattern_captures("a/**/b/*.json"), 2);
    }

    #[test]
    fn test_validate_path_vars_success() {
        // 合法变量测试
        assert!(validate_path_vars("foo/{cwd}/bar", "test").is_ok());
        assert!(validate_path_vars("{temp_dir}/myapp", "test").is_ok());
        assert!(validate_path_vars("path/to/{exe_dir}", "test").is_ok());
        assert!(validate_path_vars("{resource_pack_dir}/assets", "test").is_ok());
        assert!(validate_path_vars("no_vars_at_all", "test").is_ok());
    }

    #[test]
    fn test_validate_path_vars_failures() {
        // 未知变量
        let res = validate_path_vars("{unknown_var}/foo", "test");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("包含未知变量"));

        // 空变量
        let res = validate_path_vars("foo/{}/bar", "test");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("包含空变量"));

        // 未闭合的左括号
        let res = validate_path_vars("foo/{cwd", "test");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("中的变量未闭合"));

        // 非法嵌套
        let res = validate_path_vars("foo/{a{b}}bar", "test");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("包含非法嵌套"));

        // 未匹配的右括号
        let res = validate_path_vars("foo/cwd}", "test");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("包含未匹配的"));
    }

    #[test]
    fn test_validate_dir_path() {
        // 正常路径
        assert!(validate_dir_path("{cwd}/my_dir").is_ok());

        // 边界：空路径
        let res = validate_dir_path("");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("路径不能为空"));

        // 边界：包含反斜杠
        let res = validate_dir_path("foo\\bar");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("包含非法分隔符"));

        // 边界：包含通配符
        let res = validate_dir_path("foo/*/bar");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("包含非法通配符"));
    }

    #[test]
    fn test_validate_pattern_success() {
        // 允许的常规 glob 模式
        assert!(validate_pattern("abc/def", "source").is_ok());
        assert!(validate_pattern("abc/*.ext", "source").is_ok());
        assert!(validate_pattern("abc/name.*", "source").is_ok());
        assert!(validate_pattern("abc/*.*", "source").is_ok());
        assert!(validate_pattern("abc/**/def", "source").is_ok());
    }

    #[test]
    fn test_validate_pattern_failures() {
        // 空路径
        assert!(validate_pattern("", "source").is_err());

        // 包含反斜杠
        assert!(validate_pattern("foo\\*", "source").is_err());

        // 多个 ** 级联或分散在不同段
        let res = validate_pattern("a/**/**/b", "source");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("最多允许一个"));

        // 单段中出现多余的 * （不满足 *.ext, name.*, *.* 等白名单写法）
        let res = validate_pattern("abc/a*b*c", "source");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("包含 2 个 `*`"));

        // 类似于 ** 但多了其他字符的非法写法（例如 abc/**a）也会被星号计数器和段校验拦截
        let res = validate_pattern("abc/**a/def", "source");
        assert!(res.is_err());
    }

    #[test]
    fn test_resolve_vfs_mode_success() {
        let result = resolve_vfs_mode("fallback");
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("VfsMode") && tokens.contains("Fallback"));

        let result = resolve_vfs_mode("force");
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("VfsMode") && tokens.contains("Force"));
    }

    #[test]
    fn test_resolve_vfs_mode_failures() {
        let result = resolve_vfs_mode("invalid");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("非法的 VfsMode"));

        let result = resolve_vfs_mode("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("非法的 VfsMode"));

        let result = resolve_vfs_mode("FALLBACK");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("非法的 VfsMode"));
    }
}
