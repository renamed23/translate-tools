use proc_macro::TokenStream;
use quote::quote;

/// 生成 compile_error!(...) 的 TokenStream
pub(crate) fn compile_error(msg: &str) -> TokenStream {
    let tokens = quote! {
        compile_error!(#msg);
    };
    tokens.into()
}

// 辅助函数：将下划线命名转换为大驼峰命名
pub(crate) fn to_pascal_case(s: &str) -> String {
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
}
