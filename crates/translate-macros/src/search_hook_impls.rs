// src/lib.rs 或在你的宏crate中

use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use std::path::PathBuf;
use syn::{Item, LitStr, parse_macro_input};

pub fn search_hook_impls(input: TokenStream) -> TokenStream {
    let input_str = parse_macro_input!(input as LitStr);
    let relative_path = input_str.value();

    // 获取项目根目录
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());

    // 构建完整路径
    let mut full_path = PathBuf::from(manifest_dir);
    full_path.push(relative_path);

    let mut mod_declarations = Vec::new();
    let mut type_aliases = Vec::new();

    // 读取目录
    if let Ok(entries) = fs::read_dir(&full_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|ext| ext == "rs")
                && let Some(file_name) = path.file_stem().and_then(|s| s.to_str())
            {
                // 跳过 mod.rs 和 lib.rs
                if file_name == "mod" || file_name == "lib" {
                    continue;
                }

                let feature_name = file_name.to_string();
                let mod_name = syn::Ident::new(file_name, proc_macro2::Span::call_site());

                // 生成模块声明
                mod_declarations.push(quote! {
                    #[cfg(feature = #feature_name)]
                    pub mod #mod_name;
                });

                // 读取文件内容并查找对应的结构体
                if let Ok(file_content) = fs::read_to_string(&path)
                    && let Ok(parsed_file) = syn::parse_file(&file_content)
                {
                    let expected_struct_name = to_pascal_case(&feature_name);

                    // 在文件中查找对应的结构体
                    let found_struct = parsed_file.items.iter().any(|item| {
                        if let Item::Struct(item_struct) = item {
                            item_struct.ident == expected_struct_name
                        } else {
                            false
                        }
                    });

                    if found_struct {
                        let type_ident =
                            syn::Ident::new(&expected_struct_name, proc_macro2::Span::call_site());

                        // 生成类型别名
                        type_aliases.push(quote! {
                            #[cfg(feature = #feature_name)]
                            pub type HookImplType = #mod_name::#type_ident;
                        });
                    }
                }
            }
        }
    } else {
        panic!("读取目录失败: {}", full_path.display());
    }

    let output = quote! {
        #(#mod_declarations)*
        #(#type_aliases)*
    };

    output.into()
}

// 辅助函数：将下划线命名转换为大驼峰命名
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join("")
        + "Hook"
}
