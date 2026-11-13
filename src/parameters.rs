//! Legacy positional parameter grammar. Kept compatible, with spanned errors.
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use syn::{
    Expr, Ident, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};
use treble_meta::{ListSize, Literal, Parameter, ParameterAccess, Scale};

struct Arguments {
    kind: Ident,
    values: Vec<Expr>,
    flags: Vec<(Ident, Ident)>,
}
impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let kind = input.parse()?;
        let mut values = vec![];
        let mut flags: Vec<(Ident, Ident)> = vec![];
        if input.is_empty() {
            return Ok(Self {
                kind,
                values,
                flags,
            });
        }
        input.parse::<Token![,]>()?;
        for expr in Punctuated::<Expr, Token![,]>::parse_terminated(input)? {
            match expr {
                Expr::Assign(assign) => flags.push(flag(&assign)?),
                other if !flags.is_empty() => {
                    return Err(syn::Error::new(
                        other.span(),
                        "positional arguments must precede named flags",
                    ));
                }
                other => values.push(other),
            }
        }
        Ok(Self {
            kind,
            values,
            flags,
        })
    }
}

fn ident_of(e: &Expr) -> Option<Ident> {
    match e {
        Expr::Path(p) => p.path.get_ident().cloned(),
        _ => None,
    }
}
fn flag(assign: &syn::ExprAssign) -> Result<(Ident, Ident)> {
    let key = ident_of(&assign.left)
        .ok_or_else(|| syn::Error::new(assign.left.span(), "expected a flag name"))?;
    let value = ident_of(&assign.right)
        .ok_or_else(|| syn::Error::new(assign.right.span(), "expected an identifier value"))?;
    Ok((key, value))
}

/// Optional named flags of a legacy parameter, emitted on its `FilterInput`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LegacyFlags {
    pub access: ParameterAccess,
    pub scale: Option<Scale>,
}

fn parse_flags(parameter: &Parameter<String>, flags: &[(Ident, Ident)]) -> Result<LegacyFlags> {
    let mut out = LegacyFlags::default();
    let (mut seen_access, mut seen_scale) = (false, false);
    for (key, value) in flags {
        if matches!(parameter, Parameter::List { .. }) {
            return Err(syn::Error::new(
                key.span(),
                "named flags are not supported on list parameters",
            ));
        }
        let seen = match key.to_string().as_str() {
            "access" => &mut seen_access,
            "scale" => &mut seen_scale,
            other => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown flag `{other}`; expected `access` or `scale`"),
                ));
            }
        };
        if std::mem::replace(seen, true) {
            return Err(syn::Error::new(
                key.span(),
                format!("duplicate flag `{key}`"),
            ));
        }
        if key == "access" {
            out.access = match value.to_string().as_str() {
                "user" => ParameterAccess::User,
                "engine" => ParameterAccess::Engine,
                _ => {
                    return Err(syn::Error::new(
                        value.span(),
                        "expected access `user` or `engine`",
                    ));
                }
            };
            continue;
        }
        let scale = match value.to_string().as_str() {
            "linear" => Scale::Linear,
            "logarithmic" => Scale::Logarithmic,
            _ => {
                return Err(syn::Error::new(
                    value.span(),
                    "expected scale `linear` or `logarithmic`",
                ));
            }
        };
        let min = match parameter {
            Parameter::Range { min, .. } => Some(*min),
            Parameter::Int { min, .. } => min.map(|m| m as f32),
            Parameter::Float { .. } => None,
            _ => {
                return Err(syn::Error::new(
                    key.span(),
                    "scale applies only to numeric parameters",
                ));
            }
        };
        if scale == Scale::Logarithmic && !min.is_some_and(|m| m > 0.0) {
            return Err(syn::Error::new(
                value.span(),
                "logarithmic scale requires a declared min > 0",
            ));
        }
        out.scale = Some(scale);
    }
    Ok(out)
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
) -> Result<(Parameter<String>, LegacyFlags)> {
    let Arguments {
        kind,
        values,
        flags,
    } = syn::parse2(tokens)?;
    let parameter = positional_parameter(name, ty, kind, values)?;
    let flags = parse_flags(&parameter, &flags)?;
    Ok((parameter, flags))
}

fn positional_parameter(
    name: String,
    ty: &syn::Type,
    kind: Ident,
    values: Vec<Expr>,
) -> Result<Parameter<String>> {
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
        let value = extract_parameter("gain".into(), &ty, quote::quote!(range, -1.0f32, 1.0, 0.0))
            .unwrap()
            .0;
        assert!(matches!(value, Parameter::Range { min: -1.0, .. }));
        let value = extract_parameter("offset".into(), &ty, quote::quote!(int, -2, -4, 4))
            .unwrap()
            .0;
        assert!(matches!(value, Parameter::Int { default: -2, .. }));
        let value = extract_parameter("shape".into(), &ty, quote::quote!(list, 4, int, -1, -2, 2))
            .unwrap()
            .0;
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

    fn flags_of(tokens: TokenStream) -> Result<LegacyFlags> {
        extract_parameter("x".into(), &syn::parse_quote!(f32), tokens).map(|(_, f)| f)
    }
    #[test]
    fn legacy_flags_default_to_user_without_scale() {
        assert_eq!(
            flags_of(quote::quote!(range, 0.0, 1.0, 0.5)).unwrap(),
            LegacyFlags::default()
        );
    }
    #[test]
    fn legacy_flags_parse_in_any_order() {
        let flags = flags_of(quote::quote!(
            range,
            20.0,
            20000.0,
            1000.0,
            scale = logarithmic,
            access = engine
        ))
        .unwrap();
        assert_eq!(flags.access, ParameterAccess::Engine);
        assert_eq!(flags.scale, Some(Scale::Logarithmic));
        let flags = flags_of(quote::quote!(float, 1.0, access = engine, scale = linear)).unwrap();
        assert_eq!(flags.scale, Some(Scale::Linear));
        let ty = syn::parse_quote!(usize);
        let (_, flags) = extract_parameter(
            "size".into(),
            &ty,
            quote::quote!(val, 3, 1, 64, scale = logarithmic),
        )
        .unwrap();
        assert_eq!(flags.scale, Some(Scale::Logarithmic));
    }
    #[test]
    fn legacy_flags_reject_invalid_declarations() {
        for input in [
            quote::quote!(range, 0.0, 20.0, 1.0, scale = logarithmic),
            quote::quote!(float, 1.0, scale = logarithmic),
            quote::quote!(toggle, true, scale = linear),
            quote::quote!(list, 4, float, access = engine),
            quote::quote!(range, 0.0, 1.0, 0.5, scale = log),
            quote::quote!(range, 0.0, 1.0, 0.5, access = host),
            quote::quote!(range, 0.0, 1.0, 0.5, unit = hz),
            quote::quote!(range, 0.0, 1.0, 0.5, access = user, access = engine),
            quote::quote!(range, 0.0, access = engine, 1.0, 0.5),
        ] {
            assert!(flags_of(input.clone()).is_err(), "accepted {input}");
        }
    }
}
