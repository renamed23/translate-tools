use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ItemFn};

pub fn ffi_catch_unwind(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    // 解析 fallback 表达式（如果没有 attr，使用 `()`）
    let fallback: Expr = if attr.is_empty() {
        syn::parse_quote! { () }
    } else {
        syn::parse2::<Expr>(attr)?
    };

    // 解析函数
    let mut func = syn::parse2::<ItemFn>(item)?;
    let original_block = &func.block;

    // 生成根据 panic 策略选择的块：
    // - panic = "unwind" 时：用 catch_unwind 包装并在 Err 时返回 fallback
    // - 否则：直接执行原始块（没有包装）
    let new_block = quote! {{
        #[cfg(panic = "unwind")]
        {
            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| #original_block)) {
                Ok(r) => r,
                Err(_) => #fallback,
            }
        }

        #[cfg(not(panic = "unwind"))]
        {
            #original_block
        }
    }};

    // 将生成的块解析回 syn::Block 并替换原函数的 block
    func.block = syn::parse2(new_block).expect("解析生成块失败");

    Ok(quote! { #func })
}
