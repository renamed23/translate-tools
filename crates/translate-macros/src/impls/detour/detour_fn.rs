use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, ItemFn, LitStr, parse_quote};

use crate::impls::{
    detour::{DetourAttr, generate_detour_ident, parse_detour_attr},
    utils::ReturnKind,
};

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
    let output = ReturnKind::flatten_result(item_fn.sig.output.clone());

    let fn_ty_tokens = quote! {#unsafety #abi fn(#inputs) #output};

    let call_args = item_fn.sig.inputs.iter().filter_map(|arg| match arg {
        syn::FnArg::Receiver(_) => None,
        syn::FnArg::Typed(pt) => Some(match &*pt.pat {
            syn::Pat::Ident(pat_ident) => {
                let ident = &pat_ident.ident;
                quote! { #ident }
            }
            pat => quote! { #pat },
        }),
    });

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
        #[translate_macros::ffi_guard(
            on_panic = #fallback_tokens,
            on_err = unsafe {crate::call!(#static_ident, #(#call_args),*)}
        )]
        #[cfg_attr(feature = "export_hooks", unsafe(no_mangle))]
        #item_fn

        // 自动生成：LazyLock 的 retour detour 静态
        #[cfg(not(feature = "iat_hook"))]
        pub static #static_ident: ::std::sync::LazyLock<::retour::GenericDetour<#fn_ty_tokens>> =
        ::std::sync::LazyLock::new(|| {
            crate::debug!("initialize detour: {}!{}", #dll_lit, #symbol_lit);
            let address = crate::utils::win32::get_module_symbol_addr(
                ::windows_sys::w!(#dll_lit),
                ::windows_sys::s!(#symbol_lit)
            ).expect(concat!("symbol not found: ", #symbol_lit));
            let ori: #fn_ty_tokens = unsafe { ::core::mem::transmute(address) };
            unsafe {
                ::retour::GenericDetour::new(ori, #fn_ident).expect(concat!("Failed to create detour for ", #symbol_lit))
            }
        });

        // 自动生成：LazyLock 的 IAT 静态
        #[cfg(feature = "iat_hook")]
        pub static #static_ident: ::std::sync::LazyLock<crate::utils::mem::iat::IatHook<#fn_ty_tokens>> =
        ::std::sync::LazyLock::new(|| {
            crate::debug!("initialize iat: {}!{}", #dll_lit, #symbol_lit);
            let address = crate::utils::win32::get_module_symbol_addr(
                ::windows_sys::w!(#dll_lit),
                ::windows_sys::s!(#symbol_lit)
            ).expect(concat!("symbol not found: ", #symbol_lit));
            crate::utils::mem::iat::IatHook::new(address, #fn_ident as usize)
        });
    })
}
