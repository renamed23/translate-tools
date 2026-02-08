use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Ident, Token};

pub fn derive_default_hook(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    let name = input.ident;

    let exclude = parse_exclude_attrs(&input.attrs)?;

    let exclude_tokens = if exclude.is_empty() {
        quote! {}
    } else {
        quote! { , { #(#exclude),* } }
    };

    let expanded = quote! {
        ::translate_macros::expand_by_files!("src/hook/traits" => {
            #[cfg(feature = __file_str__)]
            impl crate::hook::traits::__file_pascal__ for #name {}
        } #exclude_tokens);
    };

    Ok(expanded)
}

fn parse_exclude_attrs(attrs: &[Attribute]) -> syn::Result<Vec<Ident>> {
    let mut result = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("exclude") {
            continue;
        }

        let punctuated: Punctuated<Ident, Token![,]> =
            attr.parse_args_with(Punctuated::parse_terminated)?;

        result.extend(punctuated);
    }

    Ok(result)
}
