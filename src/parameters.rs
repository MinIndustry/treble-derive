//! Legacy positional parameter grammar. Kept compatible, with spanned errors.
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use syn::{
    Expr, Ident, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};
use treble_meta::{ListSize, Literal, Parameter};

struct Arguments {
    kind: Ident,
    values: Vec<Expr>,
}
impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let kind = input.parse()?;
        if input.is_empty() {
            return Ok(Self {
                kind,
                values: vec![],
            });
        }
        input.parse::<Token![,]>()?;
        let values = Punctuated::<Expr, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect();
        Ok(Self { kind, values })
    }
}
fn number(e: &Expr) -> Result<f32> {
    let n = crate::module::attrs::number(e)?;
    let n = n as f32;
    if !n.is_finite() {
        return Err(syn::Error::new(
            e.span(),
            "value must be representable as finite f32",
        ));
    }
    Ok(n)
}
fn int(e: &Expr) -> Result<i32> {
    let n = crate::module::attrs::number(e)?;
    if n.fract() != 0.0 || n < f64::from(i32::MIN) || n > f64::from(i32::MAX) {
        return Err(syn::Error::new(
            e.span(),
            "expected an integer literal fitting i32",
        ));
    }
    Ok(n as i32)
}
fn boolean(e: &Expr) -> Result<bool> {
    if let Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Bool(b),
        ..
    }) = e
    {
        Ok(b.value)
    } else {
        Err(syn::Error::new(e.span(), "expected true or false"))
    }
}
fn count(values: &[Expr], min: usize, max: usize, span: Span) -> Result<()> {
    if values.len() < min || values.len() > max {
        Err(syn::Error::new(
            span,
            format!(
                "expected {min}..={max} positional arguments, found {}",
                values.len()
            ),
        ))
    } else {
        Ok(())
    }
}
fn type_name(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(p) = ty {
        p.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}
fn integer_type(ty: &syn::Type) -> bool {
    matches!(
        type_name(ty).as_deref(),
        Some(
            "usize"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
        )
    )
}
fn bounds(ty: &syn::Type) -> (Option<i32>, Option<i32>) {
    match type_name(ty).as_deref() {
        Some("u8") => (Some(0), Some(i32::from(u8::MAX))),
        Some("u16") => (Some(0), Some(i32::from(u16::MAX))),
        Some("i8") => (Some(i32::from(i8::MIN)), Some(i32::from(i8::MAX))),
        Some("i16") => (Some(i32::from(i16::MIN)), Some(i32::from(i16::MAX))),
        Some("u32" | "usize") => (Some(0), None),
        _ => (None, None),
    }
}

pub fn extract_parameter(
    name: String,
    ty: &syn::Type,
    tokens: TokenStream,
) -> Result<Parameter<String>> {
    let Arguments { kind, values } = syn::parse2(tokens)?;
    let span = kind.span();
    let title = name.to_case(Case::Title);
    let kind = if kind == "val" {
        if integer_type(ty) {
            "int".into()
        } else {
            "float".into()
        }
    } else {
        kind.to_string()
    };
    match kind.as_str() {
        "range" => {
            count(&values, 3, 3, span)?;
            let (min, max, default) = (
                number(&values[0])?,
                number(&values[1])?,
                number(&values[2])?,
            );
            if min > max || default < min || default > max {
                return Err(syn::Error::new(
                    span,
                    "range requires min <= default <= max",
                ));
            }
            Ok(Parameter::Range {
                title,
                field_name: name,
                min,
                max,
                default,
                value: default,
            })
        }
        "float" => {
            count(&values, 1, 1, span)?;
            let default = number(&values[0])?;
            Ok(Parameter::Float {
                title,
                field_name: name,
                default,
                value: default,
            })
        }
        "toggle" => {
            count(&values, 1, 1, span)?;
            let default = boolean(&values[0])?;
            Ok(Parameter::Toggle {
                title,
                field_name: name,
                default,
                value: default,
            })
        }
        "int" => {
            count(&values, 1, 3, span)?;
            let default = int(&values[0])?;
            let (bmin, bmax) = bounds(ty);
            let min = values.get(1).map(int).transpose()?.or(bmin);
            let max = values.get(2).map(int).transpose()?.or(bmax);
            if min.is_some_and(|m| default < m)
                || max.is_some_and(|m| default > m)
                || matches!((min,max),(Some(a),Some(b)) if a>b)
            {
                return Err(syn::Error::new(
                    span,
                    "integer range requires min <= default <= max",
                ));
            }
            Ok(Parameter::Int {
                title,
                field_name: name,
                default,
                value: default,
                min,
                max,
            })
        }
        "list" | "vec" => {
            count(&values, 2, 5, span)?;
            let size = match &values[0] {
                Expr::Path(p) if p.path.get_ident().is_some() => ListSize::Field(
                    p.path
                        .get_ident()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                ),
                Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) => ListSize::Constant(n.base10_parse()?),
                other => {
                    return Err(syn::Error::new(
                        other.span(),
                        "list size must be a field name or nonnegative integer literal",
                    ));
                }
            };
            let element = if let Expr::Path(p) = &values[1] {
                p.path.get_ident().map(ToString::to_string)
            } else {
                None
            }
            .ok_or_else(|| {
                syn::Error::new(
                    values[1].span(),
                    "expected list element type float, range, toggle, or int",
                )
            })?;
            let args = &values[2..];
            let ltype = match element.as_str() {
                "float" => {
                    count(args, 0, 1, span)?;
                    Literal::Float(
                        title.clone(),
                        args.first().map(number).transpose()?.unwrap_or(0.0),
                    )
                }
                "toggle" => {
                    count(args, 0, 1, span)?;
                    Literal::Toggle(
                        title.clone(),
                        args.first().map(boolean).transpose()?.unwrap_or(false),
                    )
                }
                "range" => {
                    count(args, 0, 3, span)?;
                    let min = args.first().map(number).transpose()?.unwrap_or(0.0);
                    let max = args.get(1).map(number).transpose()?.unwrap_or(1.0);
                    let default = args.get(2).map(number).transpose()?.unwrap_or(0.0);
                    if min > max || default < min || default > max {
                        return Err(syn::Error::new(
                            span,
                            "list range requires min <= default <= max",
                        ));
                    }
                    Literal::Range(title.clone(), min, max, default)
                }
                "int" => {
                    count(args, 0, 3, span)?;
                    let default = args.first().map(int).transpose()?.unwrap_or(0);
                    let min = args.get(1).map(int).transpose()?;
                    let max = args.get(2).map(int).transpose()?;
                    if min.is_some_and(|m| default < m)
                        || max.is_some_and(|m| default > m)
                        || matches!((min,max),(Some(a),Some(b)) if a>b)
                    {
                        return Err(syn::Error::new(
                            span,
                            "list integer range requires min <= default <= max",
                        ));
                    }
                    Literal::Int(title.clone(), default, min, max)
                }
                _ => {
                    return Err(syn::Error::new(
                        values[1].span(),
                        format!("unknown list element type `{element}`"),
                    ));
                }
            };
            Ok(Parameter::List {
                title,
                field_name: name,
                size,
                ltype,
            })
        }
        _ => Err(syn::Error::new(
            span,
            format!("unknown legacy parameter type `{kind}`"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_signed_literals_suffixes_and_lists_are_parsed_without_token_loss() {
        let ty = syn::parse_quote!(i32);
        let value =
            extract_parameter("gain".into(), &ty, quote::quote!(range, -1.0f32, 1.0, 0.0)).unwrap();
        assert!(matches!(value, Parameter::Range { min: -1.0, .. }));
        let value = extract_parameter("offset".into(), &ty, quote::quote!(int, -2, -4, 4)).unwrap();
        assert!(matches!(value, Parameter::Int { default: -2, .. }));
        let value =
            extract_parameter("shape".into(), &ty, quote::quote!(list, 4, int, -1, -2, 2)).unwrap();
        assert!(matches!(
            value,
            Parameter::List {
                ltype: Literal::Int(_, -1, Some(-2), Some(2)),
                ..
            }
        ));
    }
    #[test]
    fn legacy_malformed_values_return_errors() {
        for input in [
            quote::quote!(range),
            quote::quote!(toggle, 1),
            quote::quote!(float),
            quote::quote!(int, 999999999999),
            quote::quote!(list, 4, unknown),
        ] {
            assert!(extract_parameter("x".into(), &syn::parse_quote!(f32), input).is_err());
        }
    }
}
