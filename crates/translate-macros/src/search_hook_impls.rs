use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::fs;
use std::path::PathBuf;
use syn::{Ident, Item, LitStr, Result, Token, Visibility, parse::Parse, parse_macro_input};

/// Macro 输入解析器：`[pub] type <AliasIdent> , "relative/path"`
struct SearchHookInput {
    vis: Option<Visibility>,
    alias: Ident,
    path: LitStr,
}

impl Parse for SearchHookInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        // 路径字符串
        let path: LitStr = input.parse()?;

        let _token: Token![=>] = input.parse()?;

        // 可选的 pub（或其它 Visibility，这里只接受 pub、pub(crate) 等 syn::Visibility）
        let vis: Option<Visibility> = if input.peek(Token![pub]) {
            Some(input.parse()?)
        } else {
            None
        };

        // 必须要有 `type`
        let _type_token: Token![type] = input.parse()?;

        // 别名标识符
        let alias: Ident = input.parse()?;

        // 确保没有多余内容
        if !input.is_empty() {
            return Err(input.error("Unexpected tokens after path literal"));
        }

        Ok(SearchHookInput { vis, alias, path })
    }
}

pub fn search_hook_impls(input: TokenStream) -> TokenStream {
    // 解析输入
    let input = parse_macro_input!(input as SearchHookInput);

    let relative_path = input.path.value();

    // 用户传入的别名（例如 HookImplType）
    let alias_ident = input.alias;

    // 可选的 pub tokenstream
    let vis_ts = if let Some(vis) = input.vis {
        quote! { #vis }
    } else {
        quote! {}
    };

    // 获取项目根目录
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("无法获取 CARGO_MANIFEST_DIR");

    // 构建完整路径
    let mut full_path = PathBuf::from(manifest_dir);
    full_path.push(&relative_path);

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
                let mod_name = Ident::new(file_name, Span::call_site());

                // 读取文件内容并解析
                if let Ok(file_content) = fs::read_to_string(&path)
                    && let Ok(parsed_file) = syn::parse_file(&file_content)
                {
                    // 期望的结构体名：PascalCase + "Hook"
                    let expected_struct_name = crate::utils::to_pascal_case(&feature_name) + "Hook";
                    let expected_ident = Ident::new(&expected_struct_name, Span::call_site());

                    // 在文件中查找对应的结构体
                    let found_struct = parsed_file.items.iter().any(|item| {
                        if let Item::Struct(item_struct) = item {
                            item_struct.ident == expected_ident
                        } else {
                            false
                        }
                    });

                    if found_struct {
                        // 将 feature 名作为字面量，用于 cfg(attr)
                        let feature_lit = LitStr::new(&feature_name, Span::call_site());

                        // 生成类型别名：可选 vis (pub), type <Alias> = <mod>::<ExpectedStruct>
                        type_aliases.push(quote! {
                            #[cfg(feature = #feature_lit)]
                            #vis_ts type #alias_ident = #mod_name::#expected_ident;
                        });
                    }
                }
            }
        }
    } else {
        panic!("读取目录失败: {}", full_path.display());
    }

    let output = quote! {
        #(#type_aliases)*
    };

    output.into()
}
