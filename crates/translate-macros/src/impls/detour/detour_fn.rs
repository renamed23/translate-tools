use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, ItemFn, LitStr, parse_quote};

use crate::impls::detour::{DetourAttr, generate_detour_ident, parse_detour_attr};

pub fn detour_fn(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let attr: Attribute = parse_quote! {
        #[detour(#attr)]
    };

    let DetourAttr {
        dll,
        symbol,
        export,
        fallback,
        calling_convention,
    } = parse_detour_attr(&attr)?.unwrap();

    let item_fn = syn::parse2::<ItemFn>(item)?;

    if export.is_some() {
        syn_bail!(attr, "detour_fn 不允许使用 `export`");
    }

    if calling_convention.is_some() {
        syn_bail!(attr, "detour_fn 不允许使用 `calling_convention`");
    }

    // 构造函数签名
    let unsafety = item_fn.sig.unsafety;
    let abi = item_fn.sig.abi.clone();
    let inputs = item_fn.sig.inputs.clone();
    let output = item_fn.sig.output.clone();

    let fn_ty_tokens = quote! {#unsafety #abi fn(#inputs) #output};

    // fallback：若 attr 给出就用它，否则 Default::default()
    let fallback_tokens = if let Some(expr) = fallback {
        quote! { #expr }
    } else {
        quote! { Default::default() }
    };

    // dll/symbol 作为字面量
    let dll_lit = LitStr::new(&dll, Span::call_site());
    let symbol_lit = LitStr::new(&symbol, Span::call_site());

    let fn_ident = item_fn.sig.ident.clone();
    let static_ident = generate_detour_ident(&fn_ident);

    Ok(quote! {
        // 原函数
        #[translate_macros::ffi_catch_unwind(#fallback_tokens)]
        #[cfg_attr(feature = "export_hooks", unsafe(no_mangle))]
        #item_fn


        // 自动生成：once_cell Lazy 的 retour detour 静态
        #[cfg(panic = "unwind")]
        pub static #static_ident: ::once_cell::sync::Lazy<::retour::GenericDetour<#fn_ty_tokens>> =
            ::once_cell::sync::Lazy::new(|| {
                let address = crate::utils::win32::get_module_symbol_addr(
                    #dll_lit,
                    ::windows_sys::s!(#symbol_lit)
                ).expect(concat!("symbol not found: ", #symbol_lit));
                let ori: #fn_ty_tokens = unsafe { ::core::mem::transmute(address) };
                unsafe {
                    ::retour::GenericDetour::new(ori, #fn_ident).expect(concat!("Failed to create detour for ", #symbol_lit))
                }
            });

        #[cfg(panic = "abort")]
        pub static #static_ident: ::once_cell::sync::Lazy<::retour::GenericDetour<#fn_ty_tokens>> =
            ::once_cell::sync::Lazy::new(|| {
                let address = crate::utils::win32::get_module_symbol_addr(
                    #dll_lit,
                    ::windows_sys::s!(#symbol_lit)
                ).unwrap();
                let ori: #fn_ty_tokens = unsafe { ::core::mem::transmute(address) };
                unsafe {
                    ::retour::GenericDetour::new(ori, #fn_ident).unwrap()
                }
            });
    })
}
