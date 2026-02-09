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

/// Extracts the int parameter from the token stream.
/// Syntax: `filter_parameter(int, <default>)` or
///         `filter_parameter(int, <default>, <min>)` or
///         `filter_parameter(int, <default>, <min>, <max>)`
fn extract_int_parameter(
    field_name: String,
    title: String,
    stream: TokenStream,
) -> Parameter<String> {
    let values: Vec<TokenTree> = stream.into_iter().collect();

    let default: i32 = values
        .first()
        .map(|t| {
            t.to_string()
                .parse()
                .expect("Expected an integer default value")
        })
        .expect("No default value found for int parameter");

    let min: Option<i32> = values.get(1).map(|t| {
        t.to_string()
            .parse()
            .expect("Expected an integer min value")
    });

    let max: Option<i32> = values.get(2).map(|t| {
        t.to_string()
            .parse()
            .expect("Expected an integer max value")
    });

    Parameter::Int {
        title,
        field_name,
        default,
        value: default,
        min,
        max,
    }
}

pub fn extract_vector_parameter(
    field_name: String,
    title: String,
    stream: TokenStream,
) -> Parameter<String> {
    let mut values = stream.into_iter();

    // First token: the name of the field that determines the vec size
    let size_field = if let Some(TokenTree::Ident(ident)) = values.next() {
        ident.to_string()
    } else {
        panic!("Expected an identifier for the size field name");
    };

    // Second token: the element type (float, range, toggle)
    let ltype_name = if let Some(TokenTree::Ident(ident)) = values.next() {
        ident.to_string()
    } else {
        panic!("Expected an identifier for the element type");
    };

    // Remaining tokens: parameters for the element type
    let remaining: Vec<TokenTree> = values.collect();

    let element_title = title.clone();
    let ltype = match ltype_name.as_str() {
        "float" => {
            let default: f32 = remaining
                .first()
                .map(|t| {
                    t.to_string()
                        .parse()
                        .expect("Expected a float default value")
                })
                .unwrap_or(0.0);
            rustic_meta::Literal::Float(element_title, default)
        }
        "range" => {
            let min: f32 = remaining
                .first()
                .map(|t| t.to_string().parse().expect("Expected a float min value"))
                .unwrap_or(0.0);
            let max: f32 = remaining
                .get(1)
                .map(|t| t.to_string().parse().expect("Expected a float max value"))
                .unwrap_or(1.0);
            let default: f32 = remaining
                .get(2)
                .map(|t| {
                    t.to_string()
                        .parse()
                        .expect("Expected a float default value")
                })
                .unwrap_or(0.0);
            rustic_meta::Literal::Range(element_title, min, max, default)
        }
        "toggle" => {
            let default: bool = remaining
                .first()
                .map(|t| {
                    t.to_string()
                        .parse()
                        .expect("Expected a bool default value")
                })
                .unwrap_or(false);
            rustic_meta::Literal::Toggle(element_title, default)
        }
        "int" => {
            let default: i32 = remaining
                .first()
                .map(|t| {
                    t.to_string()
                        .parse()
                        .expect("Expected an int default value")
                })
                .unwrap_or(0);
            let min: Option<i32> = remaining
                .get(1)
                .map(|t| t.to_string().parse().expect("Expected an int min value"));
            let max: Option<i32> = remaining
                .get(2)
                .map(|t| t.to_string().parse().expect("Expected an int max value"));
            rustic_meta::Literal::Int(element_title, default, min, max)
        }
        other => panic!("Unknown element type for list parameter: {other}"),
    };

    Parameter::List {
        title,
        field_name,
        size: size_field,
        ltype,
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
        "int" => extract_int_parameter(name, parameter_title, values),
        "list" | "vec" => extract_vector_parameter(name, parameter_title, values),
        any => panic!("Unknown parameter type: {any}"),
    }
}
