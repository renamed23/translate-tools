use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, ItemFn, Token,
    parse::{Parse, ParseStream},
};

use crate::impls::utils::ReturnKind;

struct FFIInput {
    on_panic: Expr,
    on_err: Option<Expr>,
}

impl Parse for FFIInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut on_panic = None;
        let mut on_err = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: Expr = input.parse()?;

            match key.to_string().as_str() {
                "on_panic" => on_panic = Some(value),
                "on_err" => on_err = Some(value),
                "on_err_or_panic" => {
                    on_panic = Some(value.clone());
                    on_err = Some(value);
                }
                _ => {
                    syn_bail!(key, "未知字段，仅支持 on_panic, on_err, on_err_or_panic",);
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(FFIInput {
            on_panic: on_panic.ok_or_else(|| input.error("缺少 on_panic 配置"))?,
            on_err,
        })
    }
}

pub fn ffi_guard(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<FFIInput>(attr)?;
    let on_panic = &input.on_panic;
    let on_err = input.on_err.as_ref();

    let mut func = syn::parse2::<ItemFn>(item)?;

    let original_block = &func.block;
    let fn_ident = &func.sig.ident;

    let return_kind = ReturnKind::from_return_type(&func.sig.output);

    if matches!(return_kind, ReturnKind::Result(_)) && on_err.is_none() {
        return Err(syn_err2!(
            "返回类型为 Result 时必须提供 on_err 或 on_err_or_panic 配置"
        ));
    }

    if let Some(output) = return_kind.try_flatten_result() {
        func.sig.output = output;
    }

    let guarded_logic = match return_kind {
        ReturnKind::Plain => quote! {
            (|| #original_block)()
        },
        ReturnKind::Result(ok_ty) => quote! {
            match (|| -> crate::Result<#ok_ty> #original_block)() {
                Ok(value) => value,
                Err(err) => {
                    crate::debug!("ffi_guard: function {} returned Err: {err:?}", stringify!(#fn_ident));
                    #on_err
                }
            }
        },
    };

    let new_block = quote! {{
        #[cfg(panic = "unwind")]
        {
            #[allow(clippy::unused_unit)]
            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| #guarded_logic)) {
                Ok(r) => r,
                Err(_) => #on_panic,
            }
        }

        #[cfg(not(panic = "unwind"))]
        {
            #[allow(clippy::unused_unit)]
            #guarded_logic
        }
    }};

    func.block = syn::parse2(new_block).expect("解析生成块失败");

    Ok(quote! { #func })
}
