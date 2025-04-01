use convert_case::{Case, Casing};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod parameters;

use parameters::extract_parameter;
use rustic_meta::Parameter;

/// Extracts the description from the filter's
/// docstring. If multiple docstrings are found,
/// they are concatenated.
fn filter_description(input: &DeriveInput) -> String {
    input
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
        .join(" ")
}

/// Finds the number of input ports for the filter
/// To do this, we look for the field with the
/// attribute `#[filter_source]`. If this field is an
/// array, we extract the length of the array. Otherwise,
/// we return 1.
fn filter_input_ports(input: &DeriveInput) -> usize {
    if let syn::Data::Struct(filter_structure) = &input.data {
        for field in filter_structure.fields.iter() {
            if field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("filter_source"))
            {
                if let syn::Type::Array(array) = &field.ty {
                    if let syn::Expr::Lit(value) = &array.len {
                        if let syn::Lit::Int(value) = &value.lit {
                            return value.base10_parse().unwrap();
                        }
                    }
                }
            }
        }
        1
    } else {
        panic!("FilterMetaData can only be derived for structs");
    }
}

/// Extracts the parameters from the filter structure.
fn filter_parameters(input: &DeriveInput) -> Vec<Parameter> {
    let mut parameters = vec![];
    if let syn::Data::Struct(filter_structure) = &input.data {
        for field in filter_structure.fields.iter() {
            if let Some(position) = field
                .attrs
                .iter()
                .position(|e| e.path().is_ident("filter_parameter"))
            {
                if let syn::Meta::List(token_list) = &field.attrs[position].meta {
                    let name: String = field
                        .ident
                        .clone()
                        .unwrap_or(syn::Ident::new("unknown", proc_macro2::Span::call_site()))
                        .to_string()
                        .to_case(Case::Title);
                    parameters.push(extract_parameter(name, token_list.tokens.clone()));
                } else {
                    println!("{:?}", field.attrs[position].meta);
                }
            }
        }
    }

    parameters
}

#[proc_macro_derive(FilterMetaData, attributes(filter_source, filter_parameter))]
/// Derives the metadata from a filter structure.
/// This metadata is used to generate the required
/// data for the frontend to render the filter.
pub fn derive_metadata(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let name = input.ident.clone().to_string();
    let description = filter_description(&input);
    let source_amount = filter_input_ports(&input);
    let parameters: Vec<Parameter> = filter_parameters(&input);

    let struct_name = input.ident.clone();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let tokens = quote! {
        impl #impl_generics crate::meta::FilterMetadata for #struct_name #ty_generics #where_clause {
            fn name(&self) -> String {
                String::from(#name)
            }

            fn description(&self) -> String {
                String::from(#description)
            }

            fn source_amount(&self) -> usize {
                #source_amount
            }

            fn parameters(&self) -> Vec<rustic_meta::Parameter> {
                vec![#(#parameters),*]
            }
        }
    };

    println!("{}", tokens);
    proc_macro::TokenStream::from(tokens)
}
