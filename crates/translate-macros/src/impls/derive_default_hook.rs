use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn derive_default_hook(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;

    let name = input.ident;

    let expanded = quote! {
        ::translate_macros::expand_by_files!("src/hook/traits" => {
            #[cfg(feature = __file_str__)]
            impl crate::hook::traits::__file_pascal__ for #name {}
        });
    };

    Ok(expanded)
}
