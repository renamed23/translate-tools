use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, ItemFn, parse_macro_input};

pub fn ffi_catch_unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    let fallback: Expr = if attr.is_empty() {
        syn::parse_quote! { () }
    } else {
        parse_macro_input!(attr as Expr)
    };

    let mut func = parse_macro_input!(item as ItemFn);
    let block = &func.block;

    let new_block = quote! {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| #block)) {
            Ok(r) => r,
            Err(_) => #fallback,
        }
    }};

    func.block = syn::parse2(new_block).expect("解析生成块失败");

    TokenStream::from(quote! {
        #func
    })
}
