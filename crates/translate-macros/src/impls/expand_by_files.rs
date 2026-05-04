use std::{
    collections::{HashMap, HashSet},
    fs,
};

use convert_case::{Case, Casing};
use proc_macro2::{Delimiter, Group, Ident, Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    Block, Expr, Lit, LitStr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::utils::get_full_path_by_manifest;

#[derive(Clone, Copy)]
enum FileMode {
    Rust,
    Plain,
}

struct Args {
    path: LitStr,
    template: Block,
    json_path: Option<LitStr>,
    exclude: Vec<Ident>,
    mode: FileMode,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let _token: Token![=>] = input.parse()?;
        let template: Block = input.parse()?;

        let mut json_path = None;
        let mut exclude: Vec<Ident> = Vec::new();
        let mut mode = None;

        while input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;

            match key.to_string().as_str() {
                "json" => {
                    if json_path.is_some() {
                        syn_bail!(key, "duplicate `json` parameter");
                    }
                    json_path = Some(input.parse::<LitStr>()?);
                }
                "exclude" => {
                    if !exclude.is_empty() {
                        syn_bail!(key, "duplicate `exclude` parameter",);
                    }
                    let content;
                    syn::bracketed!(content in input);
                    let punctuated: Punctuated<_, _> =
                        content.parse_terminated(Ident::parse, Token![,])?;
                    exclude = punctuated.into_iter().collect();
                }
                "mode" => {
                    if mode.is_some() {
                        syn_bail!(key, "duplicate `mode` parameter");
                    }
                    let mode_str: LitStr = input.parse()?;
                    mode = Some(match mode_str.value().as_str() {
                        "rust" => FileMode::Rust,
                        "plain" => FileMode::Plain,
                        other => {
                            syn_bail!(
                                &mode_str,
                                "invalid mode `{other}`, expected `rust` or `plain`",
                            );
                        }
                    });
                }
                other => {
                    syn_bail!(
                        key,
                        "unknown parameter `{other}`, expected `json`, `exclude`, or `mode`",
                    );
                }
            }
        }

        Ok(Args {
            path,
            template,
            json_path,
            exclude,
            mode: mode.unwrap_or(FileMode::Rust),
        })
    }
}

pub fn expand_by_files(input: TokenStream) -> syn::Result<TokenStream> {
    let args = syn::parse2::<Args>(input)?;
    let full_path = get_full_path_by_manifest(args.path.value())?;

    // 构建排除集合
    let exclude: HashSet<String> = args
        .exclude
        .iter()
        .map(|ident| ident.to_string().to_case(Case::Snake))
        .collect();

    // 加载 JSON 配置
    let json_map: Option<HashMap<String, TokenStream>> =
        if let Some(ref json_path_lit) = args.json_path {
            let json_full_path = get_full_path_by_manifest(json_path_lit.value())?;
            let raw: HashMap<String, String> =
                serde_json::from_str(&fs::read_to_string(&json_full_path).map_err(|e| {
                    syn_err!(
                        json_path_lit,
                        "读取JSON文件失败 `{}`: {}",
                        json_full_path.display(),
                        e
                    )
                })?)
                .map_err(|e| {
                    syn_err!(
                        json_path_lit,
                        "解析JSON失败 `{}`: {}",
                        json_full_path.display(),
                        e
                    )
                })?;

            let mut parsed: HashMap<String, TokenStream> = HashMap::new();
            for (file, expr_str) in raw {
                let tokens = syn::parse_str::<TokenStream>(&expr_str).map_err(|e| {
                    syn_err2!("JSON中文件 `{file}` 的值 `{expr_str}` 不是合法的Rust表达式: {e}")
                })?;
                parsed.insert(file, tokens);
            }
            Some(parsed)
        } else {
            None
        };

    let mut template_ts = TokenStream::new();
    for stmt in args.template.stmts.iter() {
        template_ts.extend(stmt.to_token_stream());
    }

    let is_rust_mode = matches!(args.mode, FileMode::Rust);

    let read_dir = match fs::read_dir(&full_path) {
        Ok(rd) => rd,
        Err(e) => syn_bail!(args.path, "读取目录失败 `{}`: {}", full_path.display(), e),
    };

    let mut file_replacements: Vec<(Replacement, String)> = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Rust 模式下仅处理 .rs 文件
        if is_rust_mode {
            let ext_ok = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e == "rs")
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
        }

        let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        // Rust 模式下跳过 mod.rs 和 lib.rs
        if is_rust_mode && (file_stem == "mod" || file_stem == "lib") {
            continue;
        }

        // 检查是否在排除列表中
        if exclude.contains(&file_stem) {
            continue;
        }

        // 查找 JSON 配置
        let file_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let json_value = json_map
            .as_ref()
            .and_then(|map| map.get(&file_name).cloned());

        let file_snake = file_stem.clone();
        let file_ident = Ident::new(&file_snake, Span::call_site());
        let file_lit = Literal::string(&file_snake);

        let pascal = file_snake.to_case(Case::Pascal);
        let pascal_ident = Ident::new(&pascal, Span::call_site());

        file_replacements.push((
            Replacement {
                file_ident,
                file_lit,
                pascal_ident,
                json_value,
            },
            file_stem,
        ));
    }

    let wrapped_ts = if has_repeat_marker(&template_ts) {
        template_ts
    } else {
        let mut implicit_ts = TokenStream::new();
        implicit_ts.extend([
            TokenTree::Ident(Ident::new("__repeat__", Span::call_site())),
            TokenTree::Group(Group::new(Delimiter::Parenthesis, template_ts)),
        ]);
        implicit_ts
    };

    process_template_with_repeat(wrapped_ts, &file_replacements)
}

/// 扫描 tokenstream 中是否包含 `__repeat__` 标记。
fn has_repeat_marker(ts: &TokenStream) -> bool {
    for tt in ts.clone() {
        match tt {
            TokenTree::Ident(id) if id == "__repeat__" => return true,
            TokenTree::Group(g) if has_repeat_marker(&g.stream()) => {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// 处理模板 tokenstream：`__repeat__` 内部按文件展开，外部内容原样输出一次。
fn process_template_with_repeat(
    ts: TokenStream,
    replacements: &[(Replacement, String)],
) -> syn::Result<TokenStream> {
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let mut out = TokenStream::new();
    let mut i = 0;

    while i < tokens.len() {
        let tt = tokens[i].clone();
        i += 1;

        match tt {
            TokenTree::Ident(id) if id == "__repeat__" => {
                if i >= tokens.len() {
                    return Err(syn_err!(id, "`__repeat__` 后必须紧跟括号参数"));
                }
                let next = tokens[i].clone();
                i += 1;
                let group = match next {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                    other => {
                        syn_bail!(other, "`__repeat__` 只支持括号调用语法：`__repeat__(...)`")
                    }
                };
                let inner = group.stream();
                for (r, _file_name) in replacements {
                    let replaced = replace_tokens(inner.clone(), r.clone())?;
                    out.extend(replaced);
                }
                // 消费可选的 ;
                if i < tokens.len()
                    && let TokenTree::Punct(p) = &tokens[i]
                    && p.as_char() == ';'
                {
                    i += 1;
                }
            }
            TokenTree::Group(g) => {
                let processed = process_template_with_repeat(g.stream(), replacements)?;
                let mut new_group = Group::new(g.delimiter(), processed);
                new_group.set_span(g.span());
                out.append(TokenTree::Group(new_group));
            }
            other => out.append(other),
        }
    }

    Ok(out)
}

#[derive(Clone)]
struct Replacement {
    file_ident: Ident,
    file_lit: Literal,
    pascal_ident: Ident,
    json_value: Option<TokenStream>,
}

/// 递归遍历 tokenstream，遇到特定 Ident 时尝试替换
fn replace_tokens(ts: TokenStream, r: Replacement) -> syn::Result<TokenStream> {
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let mut out = TokenStream::new();
    let mut i = 0;

    while i < tokens.len() {
        let tt = tokens[i].clone();
        i += 1;

        match tt {
            TokenTree::Ident(id) => {
                let name = id.to_string();
                match name.as_str() {
                    "__concat__" => {
                        if i >= tokens.len() {
                            return Err(syn_err!(
                                id,
                                "`__concat__` 后必须紧跟括号参数，如 `__concat__(\"a\", \
                                 __file_str__)`",
                            ));
                        }
                        let next = tokens[i].clone();
                        i += 1;

                        let group = match next {
                            TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                            other => {
                                syn_bail!(
                                    other,
                                    "`__concat__` 只支持括号调用语法：`__concat__(...)`",
                                );
                            }
                        };

                        let concat_lit = parse_concat_group(&group, &r)?;
                        out.append(TokenTree::Literal(concat_lit));

                        // 消费可选的 ;
                        if i < tokens.len()
                            && let TokenTree::Punct(p) = &tokens[i]
                            && p.as_char() == ';'
                        {
                            i += 1;
                        }
                    }
                    "__file__" => {
                        out.append(TokenTree::Ident(r.file_ident.clone()));
                    }
                    "__file_str__" => {
                        out.append(TokenTree::Literal(r.file_lit.clone()));
                    }
                    "__file_pascal__" => {
                        out.append(TokenTree::Ident(r.pascal_ident.clone()));
                    }
                    "__file_json_value__" => match &r.json_value {
                        Some(ts) => out.extend(ts.clone()),
                        None => syn_bail!(
                            id,
                            "`__file_json_value__` 需要提供 `json = \"...\"` \
                             参数，且当前文件必须在JSON中有对应映射",
                        ),
                    },
                    other => {
                        out.append(TokenTree::Ident(Ident::new(other, Span::call_site())));
                    }
                }
            }
            TokenTree::Group(g) => {
                let stream = g.stream();
                let replaced = replace_tokens(stream, r.clone())?;
                let mut new_group = Group::new(g.delimiter(), replaced);
                new_group.set_span(g.span());
                out.append(TokenTree::Group(new_group));
            }
            other => {
                out.append(other);
            }
        }
    }

    Ok(out)
}

/// 解析 `__concat__(...)`：参数会先做占位符替换，然后连接为字符串字面量。
fn parse_concat_group(group: &Group, r: &Replacement) -> syn::Result<Literal> {
    let args = split_concat_args(group.stream())?;

    if args.len() < 2 {
        syn_bail!(
            group,
            "`__concat__` 至少需要两个参数，例如 `__concat__(\"enable_egui\", __file_str__)`",
        );
    }

    let mut parts = Vec::with_capacity(args.len());
    for arg in args {
        let replaced = replace_tokens(arg, r.clone())?;
        let part = concat_arg_to_string(replaced)?;
        if part.is_empty() {
            syn_bail!(group, "`__concat__` 参数展开后不能为空字符串",);
        }
        parts.push(part);
    }

    Ok(Literal::string(&parts.join("")))
}

/// 将 `__concat__` 的参数切分为逗号分隔的 tokenstream 列表。
fn split_concat_args(ts: TokenStream) -> syn::Result<Vec<TokenStream>> {
    let mut args = Vec::new();
    let mut current = TokenStream::new();

    for tt in ts {
        if let TokenTree::Punct(p) = &tt
            && p.as_char() == ','
        {
            if current.is_empty() {
                syn_bail2!("`__concat__` 参数列表存在空参数",);
            }
            args.push(current);
            current = TokenStream::new();
            continue;
        }

        current.extend([tt]);
    }

    if current.is_empty() {
        if args.is_empty() {
            syn_bail2!("`__concat__` 不能为空，至少需要两个参数",);
        }

        syn_bail2!("`__concat__` 参数列表末尾存在多余逗号",);
    }

    args.push(current);
    Ok(args)
}

/// 将单个 `__concat__` 参数转换为字符串片段。
///
/// 支持：
/// - 字符串字面量（如：`"enable_egui"`）
/// - 单个标识符（如：`__file__` 展开后的 `logger`）
fn concat_arg_to_string(ts: TokenStream) -> syn::Result<String> {
    let expr: Expr = syn::parse2(ts.clone())
        .map_err(|_| syn_err!(ts, "`__concat__` 参数必须是字符串字面量或单个标识符"))?;

    match expr {
        Expr::Lit(expr_lit) => match expr_lit.lit {
            Lit::Str(s) => Ok(s.value()),
            other => syn_bail!(other, "`__concat__` 只支持字符串字面量",),
        },
        Expr::Path(expr_path) if expr_path.qself.is_none() => {
            if let Some(ident) = expr_path.path.get_ident() {
                Ok(ident.to_string())
            } else {
                syn_bail!(expr_path, "`__concat__` 路径参数必须是单个标识符",)
            }
        }
        other => syn_bail!(other, "`__concat__` 参数必须是字符串字面量或单个标识符",),
    }
}
