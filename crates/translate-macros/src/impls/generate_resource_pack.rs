use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    LitByteStr, LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::impls::utils::{get_full_path_by_manifest, read_config_json};

struct PathInput {
    resource_dir: LitStr,
    config_path: LitStr,
    output: Option<LitStr>,
}

impl Parse for PathInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let resource_dir: LitStr = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let config_path: LitStr = input.parse()?;

        let mut output = None;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            output = Some(input.parse()?);
        }

        Ok(PathInput {
            resource_dir,
            config_path,
            output,
        })
    }
}

pub fn generate_resource_pack(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathInput>(input)?;
    let resource_dir_path = get_full_path_by_manifest(parsed.resource_dir.value())?;
    let config_path = get_full_path_by_manifest(parsed.config_path.value())?;

    let config = read_config_json(config_path)?;
    let Some(pack_name) = config.get("RESOURCE_PACK_NAME").and_then(|v| v.as_str()) else {
        syn_bail!(
            parsed.config_path,
            "在用户配置json中无法找到'RESOURCE_PACK_NAME'"
        );
    };

    let temp_dir_name = format!("text_hook_resource_pack_{pack_name}");
    let pack_file_name = format!("{pack_name}.pak");

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

    let original_len = cat_data.len();
    let is_compressed = parsed.output.is_none() && original_len > 80 * 1024;

    // 统一处理压缩逻辑
    let final_data = if is_compressed {
        zstd::bulk::compress(&cat_data, 0)
            .map_err(|e| syn_err!(&parsed.resource_dir, "zstd压缩失败: {}", e))?
    } else {
        cat_data
    };

    // 根据是否是 external 生成不同的数据读取逻辑
    let data_loading_code = if let Some(output) = parsed.output {
        let out_path = get_full_path_by_manifest(format!("{}/{pack_file_name}", output.value()))?;
        std::fs::create_dir_all(out_path.parent().unwrap())
            .map_err(|e| syn_err!(&output, "创建输出目录失败 {}: {}", out_path.display(), e))?;
        std::fs::write(&out_path, &final_data)
            .map_err(|e| syn_err!(&output, "写入资源包文件失败 {}: {}", out_path.display(), e))?;

        quote! {
            let pak_path = crate::utils::get_executable_dir().join(#pack_file_name);
            let raw_data = std::fs::read(&pak_path)?;
            let data_ref: &[u8] = &raw_data;
        }
    } else {
        let data_lit = LitByteStr::new(&final_data, Span::call_site());
        quote! {
            let data_ref: &[u8] = #data_lit;
        }
    };

    // 生成解压逻辑
    let decompression_code = if is_compressed {
        quote! {
            let decompressed = crate::utils::decompress(data_ref, #original_len)?;
            let data = &decompressed;
        }
    } else {
        quote! {
            let data = data_ref;
        }
    };

    // --- 生成最终代码 ---
    let phf_entries = paths.iter().map(|&p| quote! { #p });

    let expanded = quote! {
        use std::io::Write;

        pub(super) fn get_temp_dir() -> &'static std::path::Path {
            static TEMP_DIR: ::std::sync::LazyLock<::std::path::PathBuf> = ::std::sync::LazyLock::new(|| {
                std::env::temp_dir().join(#temp_dir_name)
            });
            TEMP_DIR.as_ref()
        }

        pub(super) static RESOURCE_PATHS: phf::Set<&'static str> = phf::phf_set! {
            #(#phf_entries),*
        };

        pub(super) fn is_resource(path: &str) -> bool {
            RESOURCE_PATHS.contains(path)
        }

        pub(super) fn extract() -> crate::Result<()> {
            let temp_dir = get_temp_dir();
            if temp_dir.exists() {
                clean_up()?;
            }

            std::fs::create_dir_all(temp_dir)?;

            // 加载数据 (内存或文件)
            #data_loading_code
            // 处理解压
            #decompression_code

            let mut offset = 0usize;
            while offset < data.len() {
                let path_len = u32::from_le_bytes(data[offset..offset+4].try_into()?) as usize;
                offset += 4;
                let path = std::str::from_utf8(&data[offset..offset+path_len])?;
                offset += path_len;
                let content_len = u64::from_le_bytes(data[offset..offset+8].try_into()?) as usize;
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
