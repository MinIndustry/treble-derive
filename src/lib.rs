use quote::{format_ident, quote};
use syn::{DeriveInput, parse_macro_input};

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
        let mut total = 0usize;
        for field in filter_structure.fields.iter() {
            if field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("filter_source"))
            {
                // If the type is [T; N], return N, else 1
                if let syn::Type::Array(array) = &field.ty
                    && let syn::Expr::Lit(value) = &array.len
                    && let syn::Lit::Int(value) = &value.lit
                {
                    total += value.base10_parse::<usize>().unwrap_or(1);
                } else {
                    total += 1;
                }
            }
        }
        if total == 0 { 1 } else { total }
    } else {
        panic!("FilterMetaData can only be derived for structs");
    }
}

/// Extracts the parameters from the filter structure,
/// returning each parameter alongside its field type for code generation.
fn filter_parameters(input: &DeriveInput) -> Vec<(Parameter<String>, syn::Type)> {
    let mut parameters = vec![];
    if let syn::Data::Struct(filter_structure) = &input.data {
        for field in filter_structure.fields.iter() {
            if let Some(position) = field
                .attrs
                .iter()
                .position(|e| e.path().is_ident("filter_parameter"))
                && let syn::Meta::List(token_list) = &field.attrs[position].meta
            {
                let field_name = field
                    .ident
                    .clone()
                    .expect("Field name is required")
                    .to_string();
                let field_type = field.ty.clone();
                let param = extract_parameter(field_name, &field_type, token_list.tokens.clone());
                parameters.push((param, field_type));
            }
        }
    }

    parameters
}

/// Returns the field_name string from any Parameter variant.
fn get_param_field_name(param: &Parameter<String>) -> &str {
    match param {
        Parameter::Toggle { field_name, .. } => field_name,
        Parameter::Range { field_name, .. } => field_name,
        Parameter::Float { field_name, .. } => field_name,
        Parameter::Int { field_name, .. } => field_name,
        Parameter::List { field_name, .. } => field_name,
    }
}

fn build_filter_info(
    parameters: &[(Parameter<String>, syn::Type)],
    name: &str,
    description: &str,
    source_amount: usize,
) -> proc_macro2::TokenStream {
    // One audio FilterInput per audio source port
    let audio_inputs: Vec<proc_macro2::TokenStream> = (0..source_amount)
        .map(|_| {
            quote! { rustic_meta::FilterInput { label: None, parameter: None } }
        })
        .collect();

    // One parameter FilterInput per filter_parameter field (skip List params)
    let param_inputs: Vec<proc_macro2::TokenStream> = parameters
        .iter()
        .filter_map(|(param, _)| {
            if matches!(param, Parameter::List { .. }) {
                return None;
            }
            let label = get_param_field_name(param);
            Some(quote! {
                rustic_meta::FilterInput {
                    label: Some(#label),
                    parameter: Some(#param),
                }
            })
        })
        .collect();

    quote! {
        rustic_meta::FilterInfo {
            name: #name,
            description: #description,
            inputs: vec![
                #(#audio_inputs,)*
                #(#param_inputs,)*
            ],
            outputs: 1,
        }
    }
}

/// Generates the `impl MetaFilter` block containing both `set_parameter` and `metadata`.
///
/// `set_parameter` match arms per parameter type:
/// - Range: cast to field type with clamping to min/max
/// - Float/Val(float): cast to field type
/// - Toggle: `bool` via `value != 0.0`
/// - Int/Val(int): cast to field type with optional clamping
/// - List: skipped (can't set with a single f32)
fn generate_meta_filter_impl(
    struct_name: &syn::Ident,
    parameters: &[(Parameter<String>, syn::Type)],
    filter_info: proc_macro2::TokenStream,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let arms: Vec<proc_macro2::TokenStream> = parameters
        .iter()
        .filter_map(|(param, field_type)| match param {
            Parameter::Range {
                field_name,
                min,
                max,
                ..
            } => {
                let ident = format_ident!("{}", field_name);
                Some(quote! {
                    #field_name => { self.#ident = (value as #field_type).clamp(#min as #field_type, #max as #field_type); }
                })
            }
            Parameter::Float { field_name, .. } => {
                let ident = format_ident!("{}", field_name);
                Some(quote! {
                    #field_name => { self.#ident = value as #field_type; }
                })
            }
            Parameter::Toggle { field_name, .. } => {
                let ident = format_ident!("{}", field_name);
                Some(quote! {
                    #field_name => { self.#ident = value != 0.0; }
                })
            }
            Parameter::Int {
                field_name,
                min,
                max,
                ..
            } => {
                let ident = format_ident!("{}", field_name);
                let cast = quote! { value as #field_type };
                let assignment = match (min, max) {
                    (Some(lo), Some(hi)) => quote! { (#cast).clamp(#lo as #field_type, #hi as #field_type) },
                    (Some(lo), None) => quote! { (#cast).max(#lo as #field_type) },
                    (None, Some(hi)) => quote! { (#cast).min(#hi as #field_type) },
                    (None, None) => cast,
                };
                Some(quote! {
                    #field_name => { self.#ident = #assignment; }
                })
            }
            Parameter::List { .. } => None,
        })
        .collect();

    let filter_name = struct_name.to_string();

    quote! {
        impl #impl_generics rustic_meta::MetaFilter for #struct_name #ty_generics #where_clause {
            fn set_parameter(&mut self, name: &str, value: f32) {
                match name {
                    #(#arms)*
                    _other => {
                        log::warn!("Unknown parameter '{}' for {}", _other, #filter_name);
                    }
                }
            }

            fn metadata() -> rustic_meta::FilterInfo {
                #filter_info
            }
        }
    }
}

/// Derives the metadata from a filter structure.
/// This metadata is used to generate the required
/// data for the frontend to render the filter.
#[proc_macro_derive(FilterMetaData, attributes(filter_source, filter_parameter))]
pub fn derive_metadata(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let name = input.ident.clone().to_string();
    let description = filter_description(&input);
    let source_amount = filter_input_ports(&input);
    let parameter_infos = filter_parameters(&input);

    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let filter_info = build_filter_info(&parameter_infos, &name, &description, source_amount);

    let meta_filter_impl = generate_meta_filter_impl(
        struct_name,
        &parameter_infos,
        filter_info,
        &impl_generics,
        &ty_generics,
        where_clause,
    );

    let tokens = quote! {
        impl #impl_generics crate::meta::traits::FilterFactory for #struct_name #ty_generics #where_clause {
            fn create_instance(&self) -> Box<dyn crate::core::graph::Filter> {
                Box::from(#struct_name::default()) as Box<dyn crate::core::graph::Filter>
            }
        }

        #meta_filter_impl
    };

    proc_macro::TokenStream::from(tokens)
}
