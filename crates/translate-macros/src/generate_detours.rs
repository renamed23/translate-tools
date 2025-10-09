use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;
use syn::{
    Attribute, Expr, FnArg, Ident, ItemTrait, Pat, PatIdent, ReturnType, TraitItem, Type,
    parse_macro_input,
};
use syn::{LitStr, TraitItemFn};

struct DetourAttr {
    dll: String,
    symbol: String,
    export: Option<String>,
    fallback: Option<Expr>,
    calling_convention: Option<String>,
}

fn parse_detour_attr(attr: &Attribute) -> syn::Result<Option<DetourAttr>> {
    if !attr.path().is_ident("detour") {
        return Ok(None);
    }

    let mut dll: Option<String> = None;
    let mut symbol: Option<String> = None;
    let mut export: Option<String> = None;
    let mut fallback: Option<Expr> = None;
    let mut  calling_convention: Option<String> = None;

    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            let key = ident.to_string();
            let buf = meta.value()?;
            
            match buf.parse::<LitStr>() {
                Ok(litstr) => {
                    match key.as_str() {
                        "dll" => dll = Some(litstr.value()),
                        "symbol" => symbol = Some(litstr.value()),
                        "export" => export = Some(litstr.value()),
                        "fallback" => {
                            match syn::parse_str::<Expr>(&litstr.value()) {
                                Ok(expr) => fallback = Some(expr),
                                Err(e) => {
                                    return Err(syn::Error::new(litstr.span(), 
                                        format!("解析 fallback 表达式失败: {}", e)));
                                }
                            }
                        }
                        "calling_convention" => {
                            calling_convention = Some(litstr.value());
                        }
                        _ => { /* 忽略未知 key */ }
                    }
                    return Ok(());
                }
                Err(_) => {
                    return Err(syn::Error::new(
                        buf.span(),
                        "detour 属性的值必须使用字符串字面量：例如 dll = \"gdi32.dll\" 或 fallback = \"FALSE\"",
                    ));
                }
            }
        }
        Ok(())
    })?;

    match (dll, symbol) {
        (Some(dll), Some(symbol)) => Ok(Some(DetourAttr {
            dll,
            symbol,
            export,
            fallback,
            calling_convention
        })),
        _ => Err(syn::Error::new(
            attr.path().span(),
            "detour 属性必须包含 dll = \"...\" 和 symbol = \"...\" 两个字符串字面量",
        )),
    }
}

pub fn generate_detours(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 解析 trait
    let input = parse_macro_input!(item as ItemTrait);

    // 最终输出 tokenstream（保留原 trait）
    let mut generated = proc_macro2::TokenStream::new();
    generated.extend(quote! { #input });

    // 遍历 trait 的 item
    for titem in input.items.iter() {
        if let TraitItem::Fn(TraitItemFn { sig, attrs, .. }) = titem {
            // 解析属性：注意 parse_detour_attr 返回 Result<Option<...>, syn::Error>
            let mut detour_meta: Option<DetourAttr> = None;
            for attr in attrs.iter() {
                match parse_detour_attr(attr) {
                    Ok(Some(parsed)) => {
                        detour_meta = Some(parsed);
                        break;
                    }
                    Ok(None) => { /* 这个 attr 不是 detour，继续 */ }
                    Err(e) => {
                        // 如果有语法/解析错误，立即把错误作为编译错误返回
                        return TokenStream::from(e.to_compile_error());
                    }
                }
            }

            if let Some(DetourAttr{dll, symbol, export, fallback, calling_convention}) = detour_meta {
                // 方法名
                let method_ident = sig.ident.clone();

                // 导出名（若 attr 中未指定 export，则使用方法名）
                let export_ident = export
                    .as_ref()
                    .map(|s| Ident::new(s, Span::call_site()))
                    .unwrap_or_else(|| Ident::new(&method_ident.to_string(), method_ident.span()));

                let calling_convention = calling_convention.unwrap_or_else(|| "system".to_string());

                // 静态名 HOOK_<METHODNAME_UPPER>
                let static_name = format!("HOOK_{}", method_ident.to_string().to_uppercase());
                let static_ident = format_ident!("{}", static_name);

                // 收集参数（跳过 receiver &self）
                let mut arg_idents: Vec<Ident> = Vec::new();
                let mut arg_types: Vec<Type> = Vec::new();
                let mut param_pairs_tokens: Vec<proc_macro2::TokenStream> = Vec::new();

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

                // 返回类型（若没有则 unit）
                let ret_type: Type = match &sig.output {
                    ReturnType::Type(_, ty) => *ty.clone(),
                    ReturnType::Default => syn::parse_str("()").expect("解析 unit 类型失败"),
                };

                // 构造函数指针类型：unsafe extern "system" fn(arg_types...) -> ret_type
                let fn_ty_tokens = {
                    let arg_iters = arg_types.iter();
                    quote! {
                        unsafe extern #calling_convention fn( #(#arg_iters),* ) -> #ret_type
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

                 

                // 生成 wrapper + static
                generated.extend(quote! {
                    // 自动生成：导出 wrapper
                    #[translate_macros::ffi_catch_unwind(#fallback_tokens)]
                    #[cfg_attr(feature = "export_hooks", unsafe(no_mangle))]
                    pub unsafe extern #calling_convention fn #export_ident( #(#param_pairs_iter),* ) -> #ret_type {
                       unsafe {
                            hook_instance().#method_ident( #(#call_args_iter),* )
                        }
                    }

                    // 自动生成：once_cell Lazy 的 retour detour 静态
                    pub static #static_ident: once_cell::sync::Lazy<retour::GenericDetour<#fn_ty_tokens>> =
                        once_cell::sync::Lazy::new(|| {
                            let address = crate::hook_utils::get_module_symbol_addr(
                                #dll_lit,
                                concat!(#symbol_lit, "\0").as_ptr() as winapi::shared::ntdef::LPCSTR
                            ).expect(concat!("symbol not found: ", #symbol_lit));
                            let ori: #fn_ty_tokens = unsafe { core::mem::transmute(address) };
                            unsafe {
                                retour::GenericDetour::new(ori, #export_ident).expect(concat!("Failed to create detour for ", #symbol_lit))
                            }
                        });
                });
            }
        }
    }

    TokenStream::from(generated)
}
