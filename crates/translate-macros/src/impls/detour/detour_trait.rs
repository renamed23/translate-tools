use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, Ident, ItemTrait, Pat, PatIdent, TraitItem, Type};
use syn::{LitStr, TraitItemFn};

use crate::impls::detour::{DetourAttr, generate_detour_ident, parse_detour_attrs};

pub fn detour_trait(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<ItemTrait>(item)?;

    // 保留原始 trait
    let mut generated = TokenStream::new();
    generated.extend(quote! { #input });

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

            // 构造函数签名
            let fn_ty_tokens = {
                let arg_iters = arg_types.iter();
                quote! {
                    unsafe extern #calling_convention fn( #(#arg_iters),* ) #output
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

            let call_args_iter = arg_idents.iter();
            let param_pairs_iter = param_pairs_tokens.iter();

            let static_ident = generate_detour_ident(&method_ident);

            // 生成 wrapper + static
            generated.extend(quote! {
                    // 自动生成：导出 wrapper
                    #[translate_macros::ffi_catch_unwind(#fallback_tokens)]
                    #[cfg_attr(feature = "export_hooks", unsafe(no_mangle))]
                    pub unsafe extern #calling_convention fn #export_ident( #(#param_pairs_iter),* ) #output {
                       unsafe {
                            crate::hook::hook_instance().#method_ident( #(#call_args_iter),* )
                        }
                    }

                    // 自动生成：once_cell Lazy 的 retour detour 静态
                    pub static #static_ident: ::once_cell::sync::Lazy<retour::GenericDetour<#fn_ty_tokens>> =
                        ::once_cell::sync::Lazy::new(|| {
                            crate::debug!("initialize detour: {}!{}", #dll_lit, #symbol_lit);
                            let address = crate::utils::win32::get_module_symbol_addr(
                                #dll_lit,
                                ::windows_sys::s!(#symbol_lit)
                            ).expect(concat!("symbol not found: ", #symbol_lit));
                            let ori: #fn_ty_tokens = unsafe { ::core::mem::transmute(address) };
                            unsafe {
                                #[cfg(panic = "unwind")]
                                return ::retour::GenericDetour::new(ori, #export_ident).expect(concat!("Failed to create detour for ", #symbol_lit));

                                #[cfg(panic = "abort")]
                                return ::retour::GenericDetour::new(ori, #export_ident).unwrap();
                            }
                        });
                });
        }
    }

    Ok(generated)
}
