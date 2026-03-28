use convert_case::{Case, Casing};
use proc_macro2::{Delimiter, Group, Ident, Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use std::{collections::HashSet, fs};
use syn::{
    Block, Expr, Lit, LitStr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::utils::get_full_path_by_manifest;

struct Args {
    path: LitStr,
    template: Block,
    exclude: Vec<Ident>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let _token: Token![=>] = input.parse()?;
        let template: Block = input.parse()?;

        // 解析可选的排除列表: , { Ident, Ident, ... }
        let exclude = if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            let content;
            let _brace = syn::braced!(content in input);
            let punctuated: Punctuated<_, _> = content.parse_terminated(Ident::parse, Token![,])?;
            punctuated.into_iter().collect()
        } else {
            Vec::new()
        };

        Ok(Args {
            path,
            template,
            exclude,
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

    let mut template_ts = TokenStream::new();
    for stmt in args.template.stmts.iter() {
        template_ts.extend(stmt.to_token_stream());
    }

    let mut output = TokenStream::new();

    let read_dir = match fs::read_dir(&full_path) {
        Ok(rd) => rd,
        Err(e) => syn_bail!(args.path, "读取目录失败 `{}`: {}", full_path.display(), e),
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext_ok = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "rs")
            .unwrap_or(false);
        if !ext_ok {
            continue;
        }
        let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        if file_stem == "mod" || file_stem == "lib" {
            continue;
        }

        // 检查是否在排除列表中
        if exclude.contains(&file_stem) {
            continue;
        }

        let file_snake = file_stem.clone();
        let file_ident = Ident::new(&file_snake, Span::call_site());
        let file_lit = Literal::string(&file_snake);

        let pascal = file_snake.to_case(Case::Pascal);
        let pascal_ident = Ident::new(&pascal, Span::call_site());

        let replaced = replace_tokens(
            template_ts.clone(),
            Replacement {
                file_ident,
                file_lit,
                pascal_ident,
            },
        )?;

        output.extend(replaced);
    }

    Ok(output)
}

#[derive(Clone)]
struct Replacement {
    file_ident: Ident,
    file_lit: Literal,
    pascal_ident: Ident,
}

/// 递归遍历 tokenstream，遇到特定 Ident 时尝试替换
fn replace_tokens(ts: TokenStream, r: Replacement) -> syn::Result<TokenStream> {
    let mut out = TokenStream::new();
    let mut iter = ts.into_iter();

    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Ident(id) => {
                let name = id.to_string();
                match name.as_str() {
                    "__concat__" => {
                        let next = iter.next().ok_or_else(|| {
                            syn_err!(
                                id.clone(),
                                "`__concat__` 后必须紧跟括号参数，如 `__concat__(\"a\", \
                                 __file_str__)`",
                            )
                        })?;

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

/// 解析 `__concat__(...)`：参数会先做占位符替换，然后按 `_` 连接为字符串字面量。
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
