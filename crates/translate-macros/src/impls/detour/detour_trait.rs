use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Block, FnArg, Ident, ItemTrait, LitStr, Pat, PatIdent, TraitItem, TraitItemFn, Type,
    parse_quote,
};

use crate::{
    impls::detour::{DetourAttr, generate_detour_ident, parse_detour_attrs},
    utils::return_kind::ReturnKind,
};

pub fn detour_trait(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut input = syn::parse2::<ItemTrait>(item)?;
    let trait_name = input.ident.clone();

    // 保留原始 trait
    let mut generated = TokenStream::new();

    // 遍历 trait 的 item
    for titem in input.items.iter() {
        if let TraitItem::Fn(TraitItemFn { sig, attrs, .. }) = titem {
            let detour_meta = parse_detour_attrs(attrs.iter())?;

            let Some(DetourAttr {
                dll,
                symbol,
                export,
                fallback,
                calling_convention,
            }) = detour_meta
            else {
                continue;
            };

            // 方法名
            let method_ident = sig.ident.clone();

            // 导出名（若 attr 中未指定 export，则使用方法名）
            let export_ident = export
                .as_ref()
                .map(|s| Ident::new(s, Span::call_site()))
                .unwrap_or_else(|| Ident::new(&method_ident.to_string(), method_ident.span()));

            let calling_convention = calling_convention.unwrap_or_else(|| "system".to_string());

            // 收集参数（跳过 receiver &self）
            let mut arg_idents: Vec<Ident> = Vec::new();
            let mut arg_types: Vec<Type> = Vec::new();
            let mut param_pairs_tokens: Vec<TokenStream> = Vec::new();

            for (idx, input_arg) in sig.inputs.iter().enumerate() {
                match input_arg {
                    FnArg::Receiver(_) => {
                        // 跳过 self
                    }
                    FnArg::Typed(pt) => {
                        let ty = &*pt.ty;
                        let ident = if let Pat::Ident(PatIdent { ident, .. }) = &*pt.pat {
                            ident.clone()
                        } else {
                            Ident::new(&format!("arg{}", idx), Span::call_site())
                        };
                        arg_idents.push(ident.clone());
                        arg_types.push(ty.clone());
                        param_pairs_tokens.push(quote! { #ident: #ty });
                    }
                }
            }

            let output = sig.output.clone();
            let real_output = ReturnKind::flatten_result(output.clone());

            // 构造函数签名
            let fn_ty_tokens = {
                let arg_iters = arg_types.iter();
                quote! {
                    unsafe extern #calling_convention fn( #(#arg_iters),* ) #real_output
                }
            };

            // fallback：若 attr 给出就用它，否则 Default::default()
            let fallback_tokens = if let Some(expr) = fallback {
                quote! { #expr }
            } else {
                quote! { Default::default() }
            };

            // dll/symbol 作为字面量
            let dll_lit = LitStr::new(&dll, Span::call_site());
            let symbol_lit = LitStr::new(&symbol, Span::call_site());

            let call_args_tokens: Vec<TokenStream> =
                arg_idents.iter().map(|ident| quote! { #ident }).collect();
            let param_pairs_iter = param_pairs_tokens.iter();

            let static_ident = generate_detour_ident(&method_ident);

            // 生成 wrapper + static
            generated.extend(quote! {
                    // 自动生成：导出 wrapper，使用完全限定语法调用 trait 实现以消除方法分发歧义
                    #[translate_macros::ffi_guard(
                        on_panic = #fallback_tokens,
                        on_err = unsafe {crate::call!(#static_ident, #(#call_args_tokens),*)}
                    )]
                    #[cfg_attr(feature = "export_hook_symbols", unsafe(no_mangle))]
                    pub unsafe extern #calling_convention fn #export_ident( #(#param_pairs_iter),* ) #output {
                       unsafe {
                            <crate::hook::impls::HookImplType as #trait_name>::#method_ident( #(#call_args_tokens),* )
                        }
                    }

                    // 自动生成：LazyLock 的 retour detour 静态
                    #[cfg(not(feature = "enable_iat_hook"))]
                    pub static #static_ident: ::std::sync::LazyLock<retour::GenericDetour<#fn_ty_tokens>> =
                        ::std::sync::LazyLock::new(|| {
                            crate::debug!("initialize detour: {}!{}", #dll_lit, #symbol_lit);
                            let address = crate::utils::win32::get_module_symbol_addr(
                                ::windows_sys::w!(#dll_lit),
                                ::windows_sys::s!(#symbol_lit)
                            ).expect(concat!("symbol not found: ", #symbol_lit));
                            let ori: #fn_ty_tokens = unsafe { ::core::mem::transmute(address) };
                            unsafe {
                                ::retour::GenericDetour::new(ori, #export_ident).expect(concat!("Failed to create detour for ", #symbol_lit))
                            }
                        });

                    // 自动生成：LazyLock 的 IAT 静态
                    #[cfg(feature = "enable_iat_hook")]
                    pub static #static_ident: ::std::sync::LazyLock<crate::utils::mem::iat::IatHook<#fn_ty_tokens>> =
                    ::std::sync::LazyLock::new(|| {
                        crate::debug!("initialize iat: {}!{}", #dll_lit, #symbol_lit);
                        let address = crate::utils::win32::get_module_symbol_addr(
                            ::windows_sys::w!(#dll_lit),
                            ::windows_sys::s!(#symbol_lit)
                        ).expect(concat!("symbol not found: ", #symbol_lit));
                        crate::utils::mem::iat::IatHook::new(address, #export_ident as usize)

                    });
                });
        }
    }

    let mut final_generated = TokenStream::new();
    let default_impl: Block = parse_quote!({ unimplemented!() });

    for titem in &mut input.items {
        if let TraitItem::Fn(func) = titem {
            let mut has_detour = false;

            func.attrs.retain(|attr| {
                if attr.path().is_ident("detour") {
                    has_detour = true;
                    false
                } else {
                    true
                }
            });

            if has_detour && func.default.is_none() {
                func.default = Some(default_impl.clone());
            }
        }
    }

    final_generated.extend(quote! { #input });
    final_generated.extend(generated);

    Ok(final_generated)
}
