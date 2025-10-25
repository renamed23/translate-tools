use proc_macro::TokenStream;
use quote::quote;
use std::env;
use std::fs;
use std::path::PathBuf;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

/// 语法解析： [pub] static NAME: [u8] from "path"
struct FlateInput {
    pub_kw: Option<Token![pub]>,
    name: Ident,
    path: LitStr,
}

impl Parse for FlateInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pub_kw = if input.peek(Token![pub]) {
            Some(input.parse()?)
        } else {
            None
        };
        let _static_tok: Token![static] = input.parse()?;
        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let _ty: syn::Type = input.parse()?;

        let from_kw: Ident = input.parse()?;
        if from_kw != "from" {
            return Err(syn::Error::new_spanned(from_kw, "需要关键字 `from`"));
        }
        let path: LitStr = input.parse()?;

        Ok(FlateInput { pub_kw, name, path })
    }
}

/// 确定目标文件路径
fn determine_target_file_path(rel_path: &str) -> anyhow::Result<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let full_path = PathBuf::from(&manifest_dir).join(rel_path);

    if full_path.is_file() {
        return Ok(full_path);
    }

    if full_path.is_dir() {
        let entries: Vec<_> = fs::read_dir(&full_path)?.collect::<Result<Vec<_>, _>>()?;

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

pub fn flate(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as FlateInput);

    let name_ident = &input.name;
    let pub_token = if input.pub_kw.is_some() {
        quote! { pub }
    } else {
        quote! {}
    };

    // 确定目标文件路径
    let target_file_path = match determine_target_file_path(&input.path.value()) {
        Ok(path) => path,
        Err(e) => {
            return syn::Error::new_spanned(input.path, format!("路径解析失败: {}", e))
                .to_compile_error()
                .into();
        }
    };

    let file_bytes = match fs::read(&target_file_path) {
        Ok(b) => b,
        Err(e) => {
            return syn::Error::new_spanned(
                input.path,
                format!("读取文件失败 `{}`: {}", target_file_path.display(), e),
            )
            .to_compile_error()
            .into();
        }
    };

    let compressed: Vec<u8> = match zstd::bulk::compress(&file_bytes, 0) {
        Ok(v) => v,
        Err(e) => {
            return syn::Error::new_spanned(input.path, format!("zstd 压缩失败: {}", e))
                .to_compile_error()
                .into();
        }
    };

    let bytes = syn::LitByteStr::new(&compressed, proc_macro2::Span::call_site());
    let bytes_tokens = quote! { #bytes };
    let file_len = file_bytes.len();
    let runtime_fn_path = quote! { crate::utils::decompress_zstd };

    let expanded = quote! {
        #pub_token static #name_ident: ::std::sync::LazyLock<Vec<u8>> = ::std::sync::LazyLock::new(|| {
            #runtime_fn_path(#bytes_tokens, #file_len)
        });
    };

    TokenStream::from(expanded)
}
