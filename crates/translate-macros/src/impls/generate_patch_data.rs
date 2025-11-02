use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::PathBuf};
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

use crate::utils::get_full_path_by_manifest;

struct PathsInput {
    raw: LitStr,
    translated: LitStr,
}

impl Parse for PathsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let raw: LitStr = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let translated: LitStr = input.parse()?;
        Ok(PathsInput { raw, translated })
    }
}

pub fn generate_patch_data(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathsInput>(input)?;

    let raw_dir = get_full_path_by_manifest(parsed.raw.value()).unwrap();
    let translated_dir = get_full_path_by_manifest(parsed.translated.value()).unwrap();

    let mut raw_files: Vec<PathBuf> = Vec::new();
    if raw_dir.exists() && raw_dir.is_dir() {
        match std::fs::read_dir(&raw_dir) {
            Ok(rd) => {
                for e in rd.filter_map(|r| r.ok()) {
                    if e.file_type().map(|t| t.is_file()).unwrap_or(false) {
                        raw_files.push(e.path());
                    }
                }
            }
            Err(e) => syn_bail!(parsed.raw, "无法读取目录 {}: {}", raw_dir.display(), e),
        }
    }

    raw_files.sort_by_key(|p| p.file_name().map(|n| n.to_os_string()).unwrap_or_default());

    struct FileEntry {
        translated_path: PathBuf,
        raw_filename: String,
        len: usize,
        hash: [u8; 32],
    }

    let mut files: Vec<FileEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut seen_keys: HashSet<[u8; 32]> = HashSet::new();

    for raw_path in &raw_files {
        let translated_path = translated_dir.join(raw_path.file_name().unwrap());
        if !translated_path.exists() {
            errors.push(format!("缺少翻译文件: {}", translated_path.display()));
            continue;
        }

        let raw_data = match std::fs::read(raw_path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("无法读取原始文件 {}: {}", raw_path.display(), e));
                continue;
            }
        };

        let translated_data = match std::fs::read(&translated_path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!(
                    "无法读取翻译文件 {}: {}",
                    translated_path.display(),
                    e
                ));
                continue;
            }
        };

        if raw_data.len() != translated_data.len() {
            errors.push(format!(
                "字节长度不匹配: {} -> raw={} bytes, translated={} bytes",
                raw_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>"),
                raw_data.len(),
                translated_data.len()
            ));
            continue;
        }

        let mut hasher = Sha256::new();
        hasher.update(&raw_data);
        let hash_bytes: [u8; 32] = hasher.finalize().into();

        if seen_keys.contains(&hash_bytes) {
            errors.push(format!(
                "发现重复的原始文件（同 hash）: {} ({:02x?})",
                raw_path.display(),
                hash_bytes,
            ));
            continue;
        }
        seen_keys.insert(hash_bytes);

        let raw_filename = raw_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        files.push(FileEntry {
            translated_path: translated_path.clone(),
            raw_filename,
            len: raw_data.len(),
            hash: hash_bytes,
        });
    }

    if !errors.is_empty() {
        let mut combined = String::new();
        for e in &errors {
            combined.push_str(e);
            combined.push('\n');
        }
        syn_bail2!("生成失败，见错误列表:\n{}", combined);
    }

    // ---- 开始生成代码 TokenStream ----
    let mut statics_tokens: Vec<TokenStream> = Vec::new();
    // 以 PATCH_0001 等命名
    for (idx, item) in files.iter().enumerate() {
        let patch_name = format!("PATCH_{:04}", idx + 1);
        // 翻译文件路径，使用绝对路径（用 / 分隔）
        let rel = item
            .translated_path
            .to_string_lossy()
            .replace('\\', "/")
            .to_string();
        // 生成 translate_macros::flate! 调用文本（作为 token stream）
        // 这里展开为语句： translate_macros::flate!( static PATCH_0001: [u8] from "/abs/path" );
        let ident = Ident::new(&patch_name, Span::call_site());
        let path_lit = Literal::string(&rel);
        let tks = quote! {
            ::translate_macros::flate!(
                static #ident: [u8] from #path_lit
            );
        };
        statics_tokens.push(tks);
    }

    fn bytes_to_escaped_literal(b: &[u8]) -> String {
        let mut s = String::with_capacity(b.len() * 4);
        for &x in b {
            s.push_str(&format!("\\x{:02X}", x));
        }
        s
    }

    // 构造 PATCHES phf_map! 内容
    let mut map_entries = Vec::new();
    for (idx, item) in files.iter().enumerate() {
        let patch_name = format!("PATCH_{:04}", idx + 1);
        let bytes_esc = bytes_to_escaped_literal(&item.hash);
        let rhs_ident = Ident::new(&patch_name, Span::call_site());
        let lhs_str = format!("b\"{}\"", bytes_esc);
        let lhs_ts: TokenStream = lhs_str.parse().expect("lhs字面量解析失败");
        let entry = quote! {
            #lhs_ts => &#rhs_ident,
        };
        map_entries.push(entry);
    }

    let patches_map = quote! {
        pub(super) static PATCHES: ::phf::Map<&'static [u8;32], &::std::sync::LazyLock<Vec<u8>>> = ::phf::phf_map! {
            #(#map_entries)*
        };
    };

    // LEN_FILTER set
    let mut lens: Vec<usize> = files.iter().map(|f| f.len).collect();
    lens.sort_unstable();
    lens.dedup();
    let lens_entries: Vec<TokenStream> = lens
        .iter()
        .map(|l| {
            let lit = Literal::usize_unsuffixed(*l);
            quote! { #lit, }
        })
        .collect();
    let len_filter = quote! {
        pub(super) static LEN_FILTER: ::phf::Set<usize> = ::phf::phf_set! {
            #(#lens_entries)*
        };
    };

    // FILENAMES map
    let mut filenames_entries = Vec::new();
    for item in files.iter() {
        let bytes_esc = bytes_to_escaped_literal(&item.hash);
        let lhs_str = format!("b\"{}\"", bytes_esc);
        let lhs_ts: TokenStream = lhs_str.parse().unwrap();
        let fname = &item.raw_filename;
        let fname_lit = Literal::string(fname);
        let entry = quote! {
            #lhs_ts => #fname_lit,
        };
        filenames_entries.push(entry);
    }
    let filenames_map = quote! {
        #[cfg(feature = "debug_output")]
        pub(super) static FILENAMES: ::phf::Map<&'static [u8;32], &'static str> = ::phf::phf_map! {
            #(#filenames_entries)*
        };
    };

    let generated = quote! {
        #(#statics_tokens)*

        #patches_map

        #len_filter

        #filenames_map
    };

    Ok(generated)
}
