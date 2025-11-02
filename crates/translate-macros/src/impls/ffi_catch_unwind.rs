use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ItemFn};

pub fn ffi_catch_unwind(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let fallback: Expr = if attr.is_empty() {
        syn::parse_quote! { () }
    } else {
        syn::parse2::<Expr>(attr)?
    };

    let mut func = syn::parse2::<ItemFn>(item)?;
    let block = &func.block;

    let new_block = quote! {{
        match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| #block)) {
            Ok(r) => r,
            Err(_) => #fallback,
        }
    }};

    func.block = syn::parse2(new_block).expect("解析生成块失败");

    Ok(quote! {
        #func
    })
}
