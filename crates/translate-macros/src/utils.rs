use proc_macro::TokenStream;
use quote::quote;

/// 生成 compile_error!(...) 的 TokenStream
pub(crate) fn compile_error(msg: &str) -> TokenStream {
    let tokens = quote! {
        compile_error!(#msg);
    };
    tokens.into()
}
