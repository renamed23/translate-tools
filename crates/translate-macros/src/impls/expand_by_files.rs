use convert_case::{Case, Casing};
use proc_macro2::{Group, Ident, Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use std::collections::HashSet;
use std::fs;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Block, LitStr, Token};

use crate::impls::utils::get_full_path_by_manifest;

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
        );

        output.extend(replaced);
    }

    Ok(output)
}

struct Replacement {
    file_ident: Ident,
    file_lit: Literal,
    pascal_ident: Ident,
}

/// 递归遍历 tokenstream，遇到特定 Ident 时尝试替换
fn replace_tokens(ts: TokenStream, r: Replacement) -> TokenStream {
    let mut out = TokenStream::new();

    for tt in ts {
        match tt {
            TokenTree::Ident(id) => {
                let name = id.to_string();
                match name.as_str() {
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
                let replaced = replace_tokens(
                    stream,
                    Replacement {
                        file_ident: r.file_ident.clone(),
                        file_lit: r.file_lit.clone(),
                        pascal_ident: r.pascal_ident.clone(),
                    },
                );
                let mut new_group = Group::new(g.delimiter(), replaced);
                new_group.set_span(g.span());
                out.append(TokenTree::Group(new_group));
            }
            other => {
                out.append(other);
            }
        }
    }

    out
}
