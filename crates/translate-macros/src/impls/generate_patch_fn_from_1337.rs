use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Token, Visibility,
    parse::{Parse, ParseStream},
};

use crate::utils::get_full_path_by_manifest;

/// Macro 输入解析器：`"1337 Directory" => <pub> fn <ident>`
struct Input {
    vis: Option<Visibility>,
    fn_ident: Ident,
    path: LitStr,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // 路径字符串
        let path: LitStr = input.parse()?;

        let _token: Token![=>] = input.parse()?;

        // 可选的 pub（或其它 Visibility，这里只接受 pub、pub(crate) 等 syn::Visibility）
        let vis: Option<Visibility> = if input.peek(Token![pub]) {
            Some(input.parse()?)
        } else {
            None
        };

        // 必须要有 `fn`
        let _fn_token: Token![fn] = input.parse()?;

        // 别名标识符
        let fn_ident: Ident = input.parse()?;

        // 确保没有多余内容
        if !input.is_empty() {
            return Err(input.error("在路径字符串后发现非预期的token"));
        }

        Ok(Input {
            vis,
            fn_ident,
            path,
        })
    }
}

pub fn generate_patch_fn_from_1337(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<Input>(input)?;

    let vis_ts = if let Some(vis) = input.vis {
        quote! { #vis }
    } else {
        quote! {}
    };

    let dir_1337 = get_full_path_by_manifest(input.path.value())
        .map_err(|e| syn_err!(&input.path, "无法解析1337目录路径: {e}"))?;

    let mut modules_patches = HashMap::<String, Vec<(u32, u8)>>::new();
    let mut main_module_count = 0;

    // 遍历所有1337文件
    for entry in
        std::fs::read_dir(&dir_1337).map_err(|e| syn_err!(&input.path, "无法读取1337目录: {e}"))?
    {
        let entry = entry.map_err(|e| syn_err!(&input.path, "读取目录项失败: {e}"))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("1337") {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| syn_err!(&input.path, "读取文件 {path:?} 失败: {e}"))?;

        let mut current_module: Option<String> = None;

        for (line_idx, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }

            // 解析模块名
            if let Some(module_name) = line.strip_prefix('>') {
                if module_name.to_lowercase().ends_with(".exe") {
                    main_module_count += 1;
                }
                current_module = Some(module_name.to_string());
                continue;
            }

            // 解析补丁数据
            if let Some((addr_part, byte_part)) = line.split_once(':') {
                if let Some((_, new_byte_str)) = byte_part.split_once("->") {
                    let addr = u32::from_str_radix(addr_part.trim(), 16).map_err(|e| {
                        syn_err!(
                            &input.path,
                            "文件 {:?} 第{}行: 解析地址失败: {}",
                            path,
                            line_idx + 1,
                            e
                        )
                    })?;

                    let new_byte = u8::from_str_radix(new_byte_str.trim(), 16).map_err(|e| {
                        syn_err!(
                            &input.path,
                            "文件 {:?} 第{}行: 解析字节失败: {}",
                            path,
                            line_idx + 1,
                            e
                        )
                    })?;

                    if let Some(module) = &current_module {
                        modules_patches
                            .entry(module.clone())
                            .or_default()
                            .push((addr, new_byte));
                    } else {
                        syn_bail!(
                            &input.path,
                            "文件 {:?} 第{}行: 未找到模块声明",
                            path,
                            line_idx + 1,
                        );
                    }
                } else {
                    syn_bail!(
                        &input.path,
                        "文件 {:?} 第{}行: 无效的格式，应为 '地址:原始->新'",
                        path,
                        line_idx + 1
                    );
                }
            } else {
                syn_bail!(
                    &input.path,
                    "文件 {:?} 第{}行: 无效的格式，应为 '地址:原始->新'",
                    path,
                    line_idx + 1
                );
            }
        }
    }

    // 验证主模块数量
    if main_module_count > 1 {
        syn_bail!(
            &input.path,
            "最多只能有一个主模块（.exe），但找到了 {main_module_count}",
        );
    }

    // 生成每个模块的patch代码
    let mut module_patches_ts = Vec::new();

    for (module, patches) in modules_patches {
        let is_main_module = module.to_lowercase().ends_with(".exe");
        let module_handle_expr = if is_main_module {
            quote! { crate::utils::win32::get_module_handle("") }
        } else {
            quote! { crate::utils::win32::get_module_handle(#module) }
        };

        // 排序并合并连续地址
        let mut sorted_patches = patches;
        sorted_patches.sort_by_key(|&(addr, _)| addr);

        let mut merged_patches: Vec<(u32, Vec<u8>)> = Vec::new();

        if !sorted_patches.is_empty() {
            let mut current_addr = sorted_patches[0].0;
            let mut current_bytes = vec![sorted_patches[0].1];

            for &(addr, byte) in sorted_patches.iter().skip(1) {
                if addr == current_addr + current_bytes.len() as u32 {
                    current_bytes.push(byte);
                } else {
                    merged_patches.push((current_addr, current_bytes));
                    current_addr = addr;
                    current_bytes = vec![byte];
                }
            }
            merged_patches.push((current_addr, current_bytes));
        }

        // 生成patch代码
        let patch_stmts: Vec<_> = merged_patches
            .iter()
            .map(|(addr, bytes)| {
                quote! {
                    let target_addr = module_base.wrapping_add(#addr as usize);
                    let data: &[u8] = &[#(#bytes),*];
                    crate::utils::mem::patch::write_asm(target_addr as *mut u8, data)?;
                }
            })
            .collect();

        module_patches_ts.push(quote! {
            // Patch模块: #module
            let module_base = #module_handle_expr
                .ok_or_else(|| anyhow::anyhow!("Cannot get module handle '{}'", #module))? as usize;
            #(#patch_stmts)*
        });
    }

    let fn_ident = &input.fn_ident;

    let output = quote! {
        #vis_ts fn #fn_ident() -> anyhow::Result<()> {
            #(#module_patches_ts)*
            Ok(())
        }
    };

    Ok(output)
}
