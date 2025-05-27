use proc_macro2::{TokenStream, TokenTree};
use rustic_meta::Parameter;

use convert_case::{Case, Casing};

/// Extracts the range parameter from the token stream.
fn extract_range_parameter(
    field_name: String,
    title: String,
    stream: TokenStream,
) -> Parameter<String> {
    let mut values = stream.into_iter();
    let minimum = values
        .next()
        .unwrap_or_else(|| panic!("No minimum value found"));
    let maximum = values
        .next()
        .unwrap_or_else(|| panic!("No maximum value found"));
    let default = values
        .next()
        .unwrap_or_else(|| panic!("No default value found"));

    let minimum = if let TokenTree::Literal(minimum) = minimum {
        minimum.to_string().parse().unwrap()
    } else {
        panic!("Minimum value is not a literal");
    };

    let maximum = if let TokenTree::Literal(maximum) = maximum {
        maximum.to_string().parse().unwrap()
    } else {
        panic!("Maximum value is not a literal");
    };

    let default = if let TokenTree::Literal(default) = default {
        default.to_string().parse().unwrap()
    } else {
        panic!("Default value is not a literal");
    };

    Parameter::Range {
        title,
        field_name,
        min: minimum,
        max: maximum,
        default,
        value: default,
    }
}

/// Extracts the toggle parameter from the token stream.
fn extract_toggle_parameter(
    field_name: String,
    title: String,
    stream: TokenStream,
) -> Parameter<String> {
    let mut values = stream.into_iter();
    if let Some(value) = values.next() {
        let value = if let TokenTree::Literal(value) = value {
            value.to_string().parse().unwrap()
        } else {
            panic!("Value is not a literal");
        };

        Parameter::Toggle {
            title,
            field_name,
            value,
            default: value,
        }
    } else {
        panic!("No value found for toggle parameter");
    }
}

/// Extracts the float parameter from the token stream.
/// A float parameter is an unbounded parameter (contrary to
/// a range parameter).
fn extract_float_parameter(
    field_name: String,
    title: String,
    stream: TokenStream,
) -> Parameter<String> {
    let value = stream.into_iter().next().unwrap();
    let value = if let TokenTree::Literal(value) = value {
        value.to_string().parse().unwrap()
    } else {
        panic!("Value is not a literal");
    };

    Parameter::Float {
        title,
        field_name,
        default: value,
        value,
    }
}

/// Iterates through the token stream of a filter_parameter
/// attribute to extract the parameter name and any additional
/// information.
pub fn extract_parameter(name: String, token_stream: TokenStream) -> Parameter<String> {
    let mut stream = token_stream.into_iter();
    let param_type = if let Some(TokenTree::Ident(token)) = stream.next() {
        token.to_string()
    } else {
        panic!("No parameter type found");
    };

    let values = stream
        .filter(|e| {
            if let TokenTree::Punct(punct) = e {
                punct.as_char() != ','
            } else {
                true
            }
        })
        .collect();

    let parameter_title: String = name.to_case(Case::Title);

    match param_type.as_str() {
        "range" => extract_range_parameter(name, parameter_title, values),
        "toggle" => extract_toggle_parameter(name, parameter_title, values),
        "float" => extract_float_parameter(name, parameter_title, values),
        any => panic!("Unknown parameter type: {any}"),
    }
}
