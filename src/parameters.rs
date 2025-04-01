use proc_macro2::{TokenStream, TokenTree};
use rustic_meta::Parameter;

/// Extracts the range parameter from the token stream.
fn extract_range_parameter(name: String, stream: TokenStream) -> Parameter {
    let values = stream.into_iter().filter(|e| {
        if let TokenTree::Punct(punct) = e {
            punct.as_char() != ','
        } else {
            true
        }
    });

    let minimum = values.clone().nth(0).unwrap();
    let maximum = values.clone().nth(1).unwrap();
    let default = values.clone().nth(2).unwrap();

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

    Parameter::Range(name, minimum, maximum, default)
}

/// Extracts the toggle parameter from the token stream.
fn extract_toggle_parameter(name: String, stream: TokenStream) -> Parameter {
    let value = stream.into_iter().next().unwrap();
    let value = if let TokenTree::Literal(value) = value {
        value.to_string().parse().unwrap()
    } else {
        panic!("Value is not a literal");
    };

    Parameter::Toggle(name, value)
}

/// Extracts the float parameter from the token stream.
/// A float parameter is an unbounded parameter (contrary to
/// a range parameter).
fn extract_float_parameter(name: String, stream: TokenStream) -> Parameter {
    let value = stream.into_iter().next().unwrap();
    let value = if let TokenTree::Literal(value) = value {
        value.to_string().parse().unwrap()
    } else {
        panic!("Value is not a literal");
    };

    Parameter::Float(name, value)
}

/// Iterates through the token stream of a filter_parameter
/// attribute to extract the parameter name and any additional
/// information.
pub fn extract_parameter(name: String, token_stream: TokenStream) -> Parameter {
    let mut stream = token_stream.into_iter();
    let param_type = if let Some(TokenTree::Ident(token)) = stream.next() {
        token.to_string()
    } else {
        panic!("No parameter type found");
    };

    match param_type.as_str() {
        "range" => extract_range_parameter(name, stream.collect()),
        "toggle" => extract_toggle_parameter(name, stream.collect()),
        "float" => extract_float_parameter(name, stream.collect()),
        any => panic!("Unknown parameter type: {any}"),
    }
}
