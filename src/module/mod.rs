pub(crate) mod attrs;
mod expand;
mod model;
mod parse;
mod validate;

use proc_macro2::TokenStream;
use syn::{DeriveInput, Result};

pub fn metadata(input: DeriveInput) -> Result<TokenStream> {
    let model = parse::module(&input)?;
    validate::module(&model)?;
    Ok(expand::metadata(&input, &model))
}
pub fn parameters(input: DeriveInput) -> Result<TokenStream> {
    let model = parse::module(&input)?;
    validate::module(&model)?;
    Ok(expand::parameters(&input, &model))
}
pub fn enumeration(input: DeriveInput) -> Result<TokenStream> {
    expand::enumeration(&input)
}

pub fn crate_path(override_path: Option<syn::Path>) -> Result<syn::Path> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    match proc_macro_crate::crate_name("treble-meta") {
        Ok(proc_macro_crate::FoundCrate::Itself) => Ok(syn::parse_quote!(crate)),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            Ok(syn::parse_quote!(::#ident))
        }
        Err(error) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("treble-meta dependency is required: {error}; or set #[treble(crate = path)]"),
        )),
    }
}
