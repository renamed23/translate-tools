use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::path::PathBuf;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitByteStr, LitStr, Token};

use crate::impls::utils::get_full_path_by_manifest;

enum Kind {
    Static,
    Const,
}

struct EmbedInput {
    pub_kw: Option<Token![pub]>,
    kind: Kind,
    name: Ident,
    path: LitStr,
}

impl Parse for EmbedInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pub_kw = if input.peek(Token![pub]) {
            Some(input.parse()?)
        } else {
            None
        };

        // 支持 static 或 const
        let kind = if input.peek(Token![static]) {
            let _tok: Token![static] = input.parse()?;
            Kind::Static
        } else if input.peek(Token![const]) {
            let _tok: Token![const] = input.parse()?;
            Kind::Const
        } else {
            syn_bail2!("预期关键字 `static` 或 `const`")
        };

        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let _ty: syn::Type = input.parse()?;

        let from_kw: Ident = input.parse()?;
        if from_kw != "from" {
            syn_bail!(from_kw, "需要关键字 `from`");
        }
        let path: LitStr = input.parse()?;

        Ok(EmbedInput {
            pub_kw,
            kind,
            name,
            path,
        })
    }
}

pub fn embed(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<EmbedInput>(input)?;

    let name_ident = &input.name;
    let pub_token = if input.pub_kw.is_some() {
        quote! { pub }
    } else {
        quote! {}
    };

    let target_file_path = determine_target_file_path(&input.path.value())
        .map_err(|e| syn_err!(&input.path, "路径解析失败: {e}"))?;

    let file_bytes = std::fs::read(&target_file_path).map_err(|e| {
        syn_err!(
            &input.path,
            "读取文件失败 `{}`: {e}",
            target_file_path.display()
        )
    })?;

    // 根据 kind 决定生成逻辑
    let expanded = match input.kind {
        Kind::Static => {
            // Static 模式：压缩 + LazyLock 解压
            let compressed: Vec<u8> = zstd::bulk::compress(&file_bytes, 3)
                .map_err(|e| syn_err!(input.path, "zstd 压缩失败: {}", e))?;

            let bytes = LitByteStr::new(&compressed, Span::call_site());
            let file_len = file_bytes.len();

            quote! {
                #pub_token static #name_ident: ::std::sync::LazyLock<Vec<u8>> = ::std::sync::LazyLock::new(|| {
                    crate::utils::decompress(#bytes, #file_len).unwrap()
                });
            }
        }
        Kind::Const => {
            // Const 模式：不压缩，直接生成原始字节
            let bytes = LitByteStr::new(&file_bytes, Span::call_site());

            quote! {
                #pub_token const #name_ident: &[u8] = #bytes;
            }
        }
    };

    Ok(expanded)
}

/// 确定目标文件路径
fn determine_target_file_path(rel_path: &str) -> anyhow::Result<PathBuf> {
    let full_path = get_full_path_by_manifest(rel_path)?;

    if full_path.is_file() {
        return Ok(full_path);
    }

    if full_path.is_dir() {
        let entries: Vec<_> = std::fs::read_dir(&full_path)?.collect::<Result<Vec<_>, _>>()?;

        let files: Vec<_> = entries
            .into_iter()
            .filter(|entry| entry.path().is_file())
            .collect();

        match files.len() {
            0 => anyhow::bail!("目录中没有文件: {}", full_path.display()),
            1 => Ok(files[0].path()),
            _ => anyhow::bail!(
                "目录中有多个文件，无法确定使用哪个: {}",
                full_path.display()
            ),
        }
    } else {
        anyhow::bail!("路径不存在或不是文件/目录: {}", full_path.display())
    }
}
