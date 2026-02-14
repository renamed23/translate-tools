use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Expr, LitByteStr, LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::utils::get_full_path_by_manifest;

struct PathInput {
    resource_dir: LitStr,
    temp_dir_name: Expr,
}

impl Parse for PathInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let resource_dir: LitStr = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let temp_dir_name: Expr = input.parse()?;

        Ok(PathInput {
            resource_dir,
            temp_dir_name,
        })
    }
}

pub fn generate_resource_pack(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathInput>(input)?;
    let resource_dir_path = get_full_path_by_manifest(parsed.resource_dir.value()).unwrap();

    // 用 WalkDir 收集所有文件
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for entry in walkdir::WalkDir::new(&resource_dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(&resource_dir_path)
            .map_err(|e| syn_err!(&parsed.resource_dir, "路径处理失败: {e}"))?
            .to_string_lossy()
            .to_lowercase()
            .replace('\\', "/");

        let content = std::fs::read(path).map_err(|e| {
            syn_err!(
                &parsed.resource_dir,
                "读取文件失败 {}: {}",
                path.display(),
                e,
            )
        })?;

        files.push((relative, content));
    }

    // 排序保证确定性
    files.sort_by(|a, b| a.0.cmp(&b.0));

    // 提取路径列表用于 phf set
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();

    // cat 格式: [u32:路径长度][u8:路径][u64:内容长度][u8:内容]...
    let mut cat_data: Vec<u8> = Vec::new();
    for (path, content) in &files {
        cat_data.extend_from_slice(&(path.len() as u32).to_le_bytes());
        cat_data.extend_from_slice(path.as_bytes());
        cat_data.extend_from_slice(&(content.len() as u64).to_le_bytes());
        cat_data.extend_from_slice(content);
    }

    // 如果超过阈值则压缩，否则直接使用原数据
    const THRESHOLD: usize = 80 * 1024;
    let original_len = cat_data.len();
    let extract_body = if cat_data.len() > THRESHOLD {
        let compressed = zstd::bulk::compress(&cat_data, 0)
            .map_err(|e| syn_err!(&parsed.resource_dir, "zstd压缩失败: {}", e))?;
        let data_lit = LitByteStr::new(&compressed, Span::call_site());
        quote! {
            let compressed: &[u8] = #data_lit;
            let data = &crate::utils::decompress(compressed, #original_len)?;
        }
    } else {
        let data_lit = LitByteStr::new(&cat_data, Span::call_site());
        quote! {
            let data: &[u8] = #data_lit;
        }
    };

    // 生成 phf set 的条目
    let phf_entries = paths.iter().map(|p| quote! { #p });
    let temp_dir_name = parsed.temp_dir_name;
    let expanded = quote! {
        use std::io::Write;

        pub(super) fn get_temp_dir() -> &'static std::path::Path {
            static TEMP_DIR: ::std::sync::LazyLock<::std::path::PathBuf> = ::std::sync::LazyLock::new(|| {
                std::env::temp_dir().join(#temp_dir_name)
            });
            TEMP_DIR.as_ref()
        }

        /// 所有资源文件的相对路径集合，用于快速匹配
        pub(super) static RESOURCE_PATHS: phf::Set<&'static str> = phf::phf_set! {
            #(#phf_entries),*
        };

        /// 检查路径是否是资源文件
        pub(super) fn is_resource(path: &str) -> bool {
            RESOURCE_PATHS.contains(path)
        }

        pub(super) fn extract() -> crate::Result<()> {
            let temp_dir = get_temp_dir();
            if temp_dir.exists() {
                clean_up()?;
            }

            std::fs::create_dir_all(temp_dir)?;
            #extract_body;

            let mut offset = 0usize;
            while offset < data.len() {
                let path_len = u32::from_le_bytes(
                    data[offset..offset+4].try_into()?
                ) as usize;
                offset += 4;

                let path = std::str::from_utf8(&data[offset..offset+path_len])?;
                offset += path_len;

                let content_len = u64::from_le_bytes(
                    data[offset..offset+8].try_into()?
                ) as usize;
                offset += 8;

                let content = &data[offset..offset+content_len];
                offset += content_len;

                let file_path = temp_dir.join(path);
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&file_path, content)?;
            }

            Ok(())
        }

        pub(super) fn clean_up() -> crate::Result<()> {
            let temp_dir = get_temp_dir();
            if temp_dir.exists() {
                std::fs::remove_dir_all(temp_dir)?;
            }
            Ok(())
        }

    };

    Ok(expanded)
}
