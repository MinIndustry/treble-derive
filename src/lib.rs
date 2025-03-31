use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(Clone, Debug)]
struct FilterMetaData {
    // The name of the filter
    pub name: syn::Ident,
    // The description of the filter
    pub description: String,
    // The number of input ports
    pub source_amount: usize,
}

impl Default for FilterMetaData {
    fn default() -> Self {
        Self {
            name: syn::Ident::new("FilterMetaData", Span::call_site()),
            description: String::new(),
            source_amount: 0,
        }
    }
}

fn filter_attributes(attrs: &[syn::Attribute], exclude: &[&str]) -> Vec<syn::Attribute> {
    attrs
        .iter()
        .filter(|attr| !exclude.iter().any(|&name| attr.path().is_ident(name)))
        .cloned()
        .collect() // Return the filtered list of attributes
}

#[proc_macro_derive(FilterMetaData)]
pub fn derive_metadata(item: TokenStream) -> TokenStream {
    let mut metadata = FilterMetaData::default();

    let input = parse_macro_input!(item as DeriveInput);

    metadata.description = input
        .attrs
        .iter()
        .filter_map(|attr| {
            if let syn::Meta::NameValue(value) = &attr.meta {
                if let syn::Expr::Lit(value) = &value.value {
                    if let syn::Lit::Str(value) = &value.lit {
                        Some(value.value().trim().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join(" ");

    metadata.name = input.ident;

    if let syn::Data::Struct(s) = &input.data {
        for field in s.fields.iter() {
            if let syn::Type::Array(array) = &field.ty {
                if let syn::Expr::Lit(v) = &array.len {
                    if let syn::Lit::Int(value) = &v.lit {
                        metadata.source_amount = value.base10_parse().unwrap();
                    }
                }
            }
        }
    } else {
        panic!("FilterMetaData can only be derived for structs");
    }

    println!("Metadata: {:?}", metadata);

    let _cleaned_code = filter_attributes(&input.attrs, &["filter_source"]);

    TokenStream::from(quote! {})
}
