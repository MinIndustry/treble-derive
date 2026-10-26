use super::model::{Kind, Model};
use proc_macro2::Span;
use syn::{Result, spanned::Spanned};

pub fn module(model: &Model) -> Result<()> {
    let mut errors: Option<syn::Error> = None;
    let mut push = |span, message| {
        let error = syn::Error::new(span, message);
        if let Some(errors) = &mut errors {
            errors.combine(error);
        } else {
            errors = Some(error);
        }
    };
    if let Err(issues) = model.descriptor.validate() {
        for issue in issues.0 {
            let mut path = issue.path.as_str();
            let span = loop {
                if let Some(span) = model.spans.get(path) {
                    break *span;
                }
                if let Some(index) = path.rfind('.') {
                    path = &path[..index];
                } else {
                    break Span::call_site();
                }
            };
            push(span, issue.message);
        }
    }
    for param in &model.parameters {
        let name = if let syn::Type::Path(path) = &param.ty {
            path.path.segments.last().map(|s| s.ident.to_string())
        } else {
            None
        };
        let bounds = match name.as_deref() {
            Some("u8") => Some((0.0, f64::from(u8::MAX))),
            Some("u16") => Some((0.0, f64::from(u16::MAX))),
            Some("u32" | "u64" | "u128" | "usize") => Some((0.0, f64::from(i32::MAX))),
            Some("i8") => Some((f64::from(i8::MIN), f64::from(i8::MAX))),
            Some("i16") => Some((f64::from(i16::MIN), f64::from(i16::MAX))),
            _ if param.kind == Kind::Int => Some((f64::from(i32::MIN), f64::from(i32::MAX))),
            _ => None,
        };
        if let Some((min, max)) = bounds {
            if let treble_meta::ParameterValue::Int(default) = param.descriptor.default
                && (f64::from(default) < min || f64::from(default) > max)
            {
                push(
                    param.default.span(),
                    format!(
                        "default is not representable by field type `{}`",
                        name.as_deref().unwrap_or("integer")
                    ),
                );
            }
            if let Some(domain) = &param.descriptor.domain
                && (domain.min < min || domain.max > max)
            {
                push(
                    param.ty.span(),
                    "integer range exceeds the field type or portable i32 representation".into(),
                );
            }
        }
        if name.as_deref() == Some("f32")
            && let treble_meta::ParameterValue::Float(default) = param.descriptor.default
            && !(default as f32).is_finite()
        {
            push(
                param.default.span(),
                "default is not representable as finite f32".into(),
            );
        }
    }
    if let Some(errors) = errors {
        Err(errors)
    } else {
        Ok(())
    }
}
