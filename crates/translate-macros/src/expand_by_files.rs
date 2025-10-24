use convert_case::Casing;
use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, Literal, Span, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use std::fs;
use std::path::PathBuf;
use syn::parse::{Parse, ParseStream};
use syn::{Block, LitStr, Token};

pub fn expand_by_files(input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as Args);
    let rel = args.path.value();

    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(s) => s,
        Err(e) => {
            return syn::Error::new_spanned(
                args.path,
                format!("无法获取 CARGO_MANIFEST_DIR: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    let mut full_path = PathBuf::from(manifest_dir);
    full_path.push(rel);

    let mut template_ts = TokenStream2::new();
    for stmt in args.template.stmts.iter() {
        template_ts.extend(stmt.to_token_stream());
    }

    let mut output = TokenStream2::new();

    let read_dir = match fs::read_dir(&full_path) {
        Ok(rd) => rd,
        Err(e) => {
            return syn::Error::new_spanned(
                args.path,
                format!("读取目录失败 `{}`: {}", full_path.display(), e),
            )
            .to_compile_error()
            .into();
        }
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

        let file_snake = file_stem.clone();
        let file_ident = Ident::new(&file_snake, Span::call_site());
        let file_lit = Literal::string(&file_snake);

        let pascal = file_snake.to_case(convert_case::Case::Pascal);
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

    TokenStream::from(output)
}

/// replacement bag
struct Replacement {
    file_ident: Ident,
    file_lit: Literal,
    pascal_ident: Ident,
}

/// 递归遍历 tokenstream，遇到特定 Ident 时尝试替换
fn replace_tokens(ts: TokenStream2, r: Replacement) -> TokenStream2 {
    let mut out = TokenStream2::new();
    let iter = ts.into_iter().peekable();

    for tt in iter {
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

struct Args {
    path: LitStr,
    template: Block,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let _token: Token![=>] = input.parse()?;
        let template: Block = input.parse()?;
        Ok(Args { path, template })
    }
}
