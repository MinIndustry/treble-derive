use super::{
    attrs::{self, Arg, Options},
    model::{Kind, Model, Parameter},
};
use proc_macro2::Span;
use std::collections::BTreeMap;
use syn::{Data, DeriveInput, Expr, Fields, Result, Type, spanned::Spanned};
use treble_meta::*;

fn error(span: Span, message: impl std::fmt::Display) -> syn::Error {
    syn::Error::new(span, message)
}
fn word(options: &mut Options, key: &str, default: &str) -> Result<String> {
    options
        .take(key)
        .map(Arg::word)
        .transpose()
        .map(|v| v.unwrap_or_else(|| default.into()))
}
fn text(options: &mut Options, key: &str, default: &str) -> Result<String> {
    options
        .take(key)
        .map(Arg::string)
        .transpose()
        .map(|v| v.unwrap_or_else(|| default.into()))
}
fn numeric(options: &mut Options, key: &str, default: f64) -> Result<f64> {
    options
        .take(key)
        .map(Arg::number)
        .transpose()
        .map(|v| v.unwrap_or(default))
}
fn integer(n: f64, span: Span) -> Result<u32> {
    if n.fract() != 0.0 || n < 0.0 || n > f64::from(u32::MAX) {
        Err(error(span, "expected an unsigned 32-bit integer literal"))
    } else {
        Ok(n as u32)
    }
}
fn choose<T>(word: &str, span: Span, choices: &[(&str, T)]) -> Result<T>
where
    T: Clone,
{
    choices
        .iter()
        .find(|(name, _)| *name == word)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| {
            error(
                span,
                format!(
                    "unknown value `{word}`; expected {}",
                    choices
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })
}
fn remember(options: &Options, prefix: &str, spans: &mut BTreeMap<String, Span>) {
    for (name, arg) in &options.0 {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        spans.insert(
            path.clone(),
            match &arg.value {
                attrs::Value::Expr(expr) => expr.span(),
                _ => arg.span(),
            },
        );
        if let attrs::Value::Nested(nested) = &arg.value {
            remember(nested, &path, spans);
        }
    }
}
fn call_options(expr: Expr) -> Result<(String, Options)> {
    let span = expr.span();
    if let Expr::Call(call) = expr
        && let Expr::Path(path) = *call.func
        && let Some(name) = path.path.get_ident()
    {
        let args = call.args;
        use quote::quote;
        return Ok((name.to_string(), syn::parse2(quote!(#args))?));
    }
    Err(error(
        span,
        "expected a named call such as audio(channels = 2)",
    ))
}
fn signal(expr: Expr) -> Result<SignalType> {
    let span = expr.span();
    if let Expr::Path(path) = &expr {
        if path.path.is_ident("note") {
            return Ok(SignalType::Note);
        }
        if path.path.is_ident("gate") {
            return Ok(SignalType::Gate);
        }
    }
    let (name, mut opts) = call_options(expr)?;
    let signal = match name.as_str() {
        "audio" => {
            let n = opts.required("channels", span)?.number()?;
            let n = integer(n, span)?;
            SignalType::Audio {
                channels: u16::try_from(n)
                    .map_err(|_| error(span, "audio channel count must fit u16"))?,
            }
        }
        "control" => SignalType::Control {
            domain: control_domain(&word(&mut opts, "domain", "bipolar")?, span)?,
            rate: control_rate(&word(&mut opts, "rate", "block")?, span)?,
        },
        _ => {
            return Err(error(
                span,
                "expected audio(...), control(...), gate, or note",
            ));
        }
    };
    opts.finish()?;
    Ok(signal)
}
fn control_domain(value: &str, span: Span) -> Result<ControlDomain> {
    choose(
        value,
        span,
        &[
            ("bipolar", ControlDomain::Bipolar),
            ("unipolar", ControlDomain::Unipolar),
        ],
    )
}
fn control_rate(value: &str, span: Span) -> Result<ControlRate> {
    choose(
        value,
        span,
        &[
            ("block", ControlRate::Block),
            ("sample", ControlRate::Sample),
        ],
    )
}
fn mode(value: &str, span: Span) -> Result<ModulationMode> {
    choose(
        value,
        span,
        &[
            ("add", ModulationMode::Add),
            ("multiply", ModulationMode::Multiply),
            ("replace", ModulationMode::Replace),
        ],
    )
}
fn port(
    arg: Arg,
    direction: Direction,
    prefix: &str,
    spans: &mut BTreeMap<String, Span>,
) -> Result<PortDescriptor> {
    let span = arg.span();
    let mut opts = arg.nested()?;
    remember(&opts, prefix, spans);
    spans.insert(prefix.into(), span);
    let id = opts.required("id", span)?.string()?;
    let name = text(&mut opts, "name", &id)?;
    let index = integer(opts.required("index", span)?.number()?, span)?;
    let signal_expr = opts.required("signal", span)?.expr()?;
    if let Ok((_, signal_options)) = call_options(signal_expr.clone()) {
        remember(&signal_options, &format!("{prefix}.signal"), spans);
    }
    let signal = signal(signal_expr)?;
    let connections = choose(
        &word(
            &mut opts,
            "connections",
            if direction == Direction::Output {
                "fan_out"
            } else {
                "single"
            },
        )?,
        span,
        &[
            ("single", ConnectionPolicy::Single),
            ("sum", ConnectionPolicy::Sum),
            ("fan_out", ConnectionPolicy::FanOut),
        ],
    )?;
    opts.finish()?;
    Ok(PortDescriptor {
        id,
        name,
        index,
        signal,
        direction,
        connections,
        binding: PortBinding::Stream,
    })
}
fn unit(value: &str, span: Span) -> Result<Unit> {
    choose(
        value,
        span,
        &[
            ("hz", Unit::Hertz),
            ("hertz", Unit::Hertz),
            ("seconds", Unit::Seconds),
            ("milliseconds", Unit::Milliseconds),
            ("decibels", Unit::Decibels),
            ("ratio", Unit::Ratio),
            ("q", Unit::Q),
            ("semitones", Unit::Semitones),
            ("cents", Unit::Cents),
            ("percent", Unit::Percent),
            ("unitless", Unit::Unitless),
        ],
    )
}
fn inferred_kind(ty: &Type) -> Result<Kind> {
    if let Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
    {
        return Ok(match segment.ident.to_string().as_str() {
            "f32" | "f64" => Kind::Float,
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" => Kind::Int,
            "bool" => Kind::Bool,
            _ => Kind::Enum,
        });
    }
    Err(error(
        ty.span(),
        "parameter fields require scalar or ParameterEnum types",
    ))
}
fn parameter(
    arg: Arg,
    field: &syn::Field,
    prefix: &str,
    spans: &mut BTreeMap<String, Span>,
) -> Result<Parameter> {
    let span = arg.span();
    let mut opts = arg.nested()?;
    remember(&opts, prefix, spans);
    spans.insert(prefix.into(), span);
    let id = opts.required("id", span)?.string()?;
    let name = text(&mut opts, "name", &id)?;
    let description = text(&mut opts, "description", &attrs::docs(&field.attrs))?;
    let kind = if let Some(kind) = opts.take("kind") {
        choose(
            &kind.word()?,
            span,
            &[
                ("float", Kind::Float),
                ("int", Kind::Int),
                ("bool", Kind::Bool),
                ("enumeration", Kind::Enum),
            ],
        )?
    } else {
        inferred_kind(&field.ty)?
    };
    let default = opts.required("default", span)?.expr()?;
    let (value_type, value) = match kind {
        Kind::Float => (
            ValueType::Float,
            ParameterValue::Float(attrs::number(&default)?),
        ),
        Kind::Int => {
            let n = attrs::number(&default)?;
            if n.fract() != 0.0 || n < f64::from(i32::MIN) || n > f64::from(i32::MAX) {
                return Err(error(
                    default.span(),
                    "integer parameter defaults must fit i32",
                ));
            }
            (ValueType::Int, ParameterValue::Int(n as i32))
        }
        Kind::Bool => {
            if let Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Bool(b),
                ..
            }) = &default
            {
                (ValueType::Bool, ParameterValue::Bool(b.value))
            } else {
                return Err(error(
                    default.span(),
                    "boolean parameters require a true/false default",
                ));
            }
        }
        Kind::Enum => (
            ValueType::Enum {
                variants: vec![EnumVariant {
                    id: "__derive_placeholder".into(),
                    name: String::new(),
                }],
            },
            ParameterValue::Enum("__derive_placeholder".into()),
        ),
    };
    let domain = if let Some(range) = opts.take("range") {
        let mut range = range.nested()?;
        // Map syntax `range` onto the public descriptor's `domain` diagnostic paths.
        remember(&range, &format!("{prefix}.domain"), spans);
        let min = range.required("min", span)?.number()?;
        let max = range.required("max", span)?.number()?;
        let step = range.take("step").map(Arg::number).transpose()?;
        range.finish()?;
        Some(NumericDomain {
            min,
            max,
            step,
            unit: unit(&word(&mut opts, "unit", "unitless")?, span)?,
            scale: choose(
                &word(&mut opts, "scale", "linear")?,
                span,
                &[
                    ("linear", Scale::Linear),
                    ("logarithmic", Scale::Logarithmic),
                ],
            )?,
        })
    } else {
        None
    };
    let access = choose(
        &word(&mut opts, "access", "user")?,
        span,
        &[
            ("user", ParameterAccess::User),
            ("engine", ParameterAccess::Engine),
        ],
    )?;
    let update = choose(
        &word(&mut opts, "update", "live")?,
        span,
        &[("live", UpdateMode::Live), ("rebuild", UpdateMode::Rebuild)],
    )?;
    let smoothing_ms = if let Some(smooth) = opts.take("smooth") {
        let expr = smooth.expr()?;
        let s = expr.span();
        if let Expr::Call(call) = expr {
            if let Expr::Path(path) = *call.func {
                if path.path.is_ident("milliseconds") && call.args.len() == 1 {
                    Some(attrs::number(&call.args[0])?)
                } else {
                    return Err(error(s, "expected milliseconds(number)"));
                }
            } else {
                return Err(error(s, "expected milliseconds(number)"));
            }
        } else {
            return Err(error(s, "expected milliseconds(number)"));
        }
    } else {
        None
    };
    let setter = opts
        .take("setter")
        .map(|a| {
            let expr = a.expr()?;
            if let Expr::Path(path) = expr {
                Ok(path.path)
            } else {
                Err(error(expr.span(), "setter must be a function path"))
            }
        })
        .transpose()?;
    opts.finish()?;
    let field_name = field
        .ident
        .clone()
        .ok_or_else(|| error(field.span(), "parameter requires a named field"))?;
    Ok(Parameter {
        descriptor: ParameterDescriptor {
            id,
            name,
            description,
            value_type,
            default: value,
            domain,
            access,
            update,
            smoothing_ms,
            modulation: None,
        },
        field: field_name,
        ty: field.ty.clone(),
        kind,
        default,
        setter,
    })
}
fn modulation(
    arg: Arg,
    parameter: &mut Parameter,
    index: u32,
    prefix: &str,
    spans: &mut BTreeMap<String, Span>,
) -> Result<PortDescriptor> {
    let span = arg.span();
    let mut opts = arg.nested()?;
    remember(&opts, &format!("{prefix}.modulation"), spans);
    let id = opts.required("port", span)?.string()?;
    let name = text(&mut opts, "name", &id)?;
    let index = integer(numeric(&mut opts, "index", f64::from(index))?, span)?;
    let default_mode = mode(&word(&mut opts, "mode", "add")?, span)?;
    let modes = if let Some(modes) = opts.take("modes") {
        let expr = modes.expr()?;
        if let Expr::Array(array) = expr {
            array
                .elems
                .into_iter()
                .map(|e| {
                    let s = e.span();
                    let arg = Arg {
                        key: syn::Ident::new("mode", s),
                        value: attrs::Value::Expr(e),
                    };
                    mode(&arg.word()?, s)
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            return Err(error(expr.span(), "modes must be an array of mode names"));
        }
    } else {
        vec![default_mode.clone()]
    };
    let mapping = choose(
        &word(&mut opts, "mapping", "native")?,
        span,
        &[
            ("native", ModulationMapping::Native),
            ("octaves", ModulationMapping::Octaves),
            ("normalized", ModulationMapping::Normalized),
        ],
    )?;
    let default_depth = numeric(&mut opts, "depth", 1.0)?;
    let domain = control_domain(&word(&mut opts, "domain", "bipolar")?, span)?;
    let rate = control_rate(&word(&mut opts, "rate", "block")?, span)?;
    let connections = choose(
        &word(&mut opts, "connections", "single")?,
        span,
        &[
            ("single", ConnectionPolicy::Single),
            ("sum", ConnectionPolicy::Sum),
        ],
    )?;
    opts.finish()?;
    parameter.descriptor.modulation = Some(ModulationDescriptor {
        modes,
        default_mode,
        mapping,
        default_depth,
    });
    Ok(PortDescriptor {
        id,
        name,
        index,
        direction: Direction::Input,
        signal: SignalType::Control { domain, rate },
        binding: PortBinding::Parameter {
            parameter_id: parameter.descriptor.id.clone(),
        },
        connections,
    })
}
fn group(arg: Arg, prefix: &str, spans: &mut BTreeMap<String, Span>) -> Result<ControlGroup> {
    let span = arg.span();
    let mut opts = arg.nested()?;
    remember(&opts, prefix, spans);
    spans.insert(prefix.into(), span);
    let id = opts.required("id", span)?.string()?;
    let name = text(&mut opts, "name", &id)?;
    let mut members = if let Some(members) = opts.take("members") {
        let expr = members.expr()?;
        if let Expr::Array(array) = expr {
            array
                .elems
                .into_iter()
                .map(|e| {
                    let s = e.span();
                    Arg {
                        key: syn::Ident::new("member", s),
                        value: attrs::Value::Expr(e),
                    }
                    .string()
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            return Err(error(
                expr.span(),
                "members must be an array of parameter ID strings",
            ));
        }
    } else {
        Vec::new()
    };
    let presentation = if let Some(widget) = opts.take("widget") {
        let (name, mut bindings) = call_options(widget.expr()?)?;
        if name != "envelope" {
            return Err(error(span, "expected envelope(...) widget"));
        }
        remember(&bindings, &format!("{prefix}.presentation.bindings"), spans);
        let b = EnvelopeBindings {
            attack: bindings.required("attack", span)?.string()?,
            decay: bindings.required("decay", span)?.string()?,
            sustain: bindings.required("sustain", span)?.string()?,
            release: bindings.required("release", span)?.string()?,
        };
        bindings.finish()?;
        if members.is_empty() {
            members = vec![
                b.attack.clone(),
                b.decay.clone(),
                b.sustain.clone(),
                b.release.clone(),
            ];
        }
        GroupPresentation::Envelope { bindings: b }
    } else {
        GroupPresentation::Generic
    };
    opts.finish()?;
    Ok(ControlGroup {
        id,
        name,
        members,
        presentation,
    })
}

pub fn module(input: &DeriveInput) -> Result<Model> {
    let Data::Struct(data) = &input.data else {
        return Err(error(input.ident.span(), "module derives require a struct"));
    };
    if matches!(data.fields, Fields::Unnamed(_)) {
        return Err(error(
            data.fields.span(),
            "module derives require named fields or a unit struct",
        ));
    }
    let mut spans = BTreeMap::new();
    let mut descriptor = None;
    let mut ports = Vec::new();
    let mut groups = Vec::new();
    let mut crate_override = None;
    for arg in attrs::attributes(&input.attrs)? {
        let span = arg.span();
        match arg.key.to_string().as_str() {
            "module" => {
                if descriptor.is_some() {
                    return Err(error(span, "duplicate module declaration"));
                }
                let mut opts = arg.nested()?;
                remember(&opts, "", &mut spans);
                let type_id = opts.required("id", span)?.string()?;
                spans.insert("type_id".into(), *spans.get("id").unwrap_or(&span));
                let version = integer(opts.required("version", span)?.number()?, span)?;
                let name = text(&mut opts, "name", &input.ident.to_string())?;
                let description = text(&mut opts, "description", &attrs::docs(&input.attrs))?;
                let category = text(&mut opts, "category", "Other")?;
                opts.finish()?;
                descriptor = Some(ModuleDescriptor {
                    type_id,
                    version,
                    name,
                    description,
                    category,
                    parameters: vec![],
                    ports: vec![],
                    groups: vec![],
                });
            }
            "output" => ports.push(port(
                arg,
                Direction::Output,
                &format!("ports[{}]", ports.len()),
                &mut spans,
            )?),
            "group" => groups.push(group(
                arg,
                &format!("groups[{}]", groups.len()),
                &mut spans,
            )?),
            "crate" => {
                if crate_override.is_some() {
                    return Err(error(span, "duplicate crate override"));
                }
                let expr = arg.expr()?;
                if let Expr::Path(path) = expr {
                    crate_override = Some(path.path);
                } else {
                    return Err(error(expr.span(), "crate override must be a Rust path"));
                }
            }
            _ => {
                return Err(error(
                    span,
                    "expected module(...), output(...), group(...), or crate = path on a struct",
                ));
            }
        }
    }
    let mut descriptor = descriptor.ok_or_else(|| {
        error(
            input.ident.span(),
            "missing #[treble(module(id = \"...\", version = 1))]",
        )
    })?;
    let mut parameters = Vec::new();
    let mut pending = Vec::new();
    let mut errors: Option<syn::Error> = None;
    for field in &data.fields {
        let result = (|| {
            let mut param = None;
            let mut modulation_attr = None;
            let mut context = false;
            let mut input_count = 0;
            for arg in attrs::attributes(&field.attrs)? {
                let span = arg.span();
                match arg.key.to_string().as_str() {
                    "param" => {
                        if param.is_some() {
                            return Err(error(span, "duplicate parameter declaration"));
                        }
                        param = Some(parameter(
                            arg,
                            field,
                            &format!("parameters[{}]", parameters.len()),
                            &mut spans,
                        )?);
                    }
                    "modulation" => {
                        if modulation_attr.is_some() {
                            return Err(error(span, "duplicate modulation declaration"));
                        }
                        modulation_attr = Some(arg);
                    }
                    "input" => {
                        ports.push(port(
                            arg,
                            Direction::Input,
                            &format!("ports[{}]", ports.len()),
                            &mut spans,
                        )?);
                        input_count += 1;
                    }
                    "context" => {
                        if context {
                            return Err(error(span, "duplicate context declaration"));
                        }
                        let opts = arg.nested()?;
                        if opts.0.len() != 1
                            || !opts.0.contains_key("sample_rate")
                            || !matches!(opts.0["sample_rate"].value, attrs::Value::Flag)
                        {
                            return Err(error(span, "expected context(sample_rate)"));
                        }
                        context = true;
                    }
                    _ => {
                        return Err(error(
                            span,
                            "expected param(...), input(...), modulation(...), or context(sample_rate) on a field",
                        ));
                    }
                }
            }
            if context && (param.is_some() || input_count > 0 || modulation_attr.is_some()) {
                return Err(error(
                    field.span(),
                    "context fields cannot also declare parameters, ports, or modulation",
                ));
            }
            if input_count > 0 && param.is_some() {
                return Err(error(
                    field.span(),
                    "parameter fields use modulation(...) for their input socket",
                ));
            }
            if let Some(param) = param {
                let index = parameters.len();
                parameters.push(param);
                if let Some(m) = modulation_attr {
                    pending.push((index, m));
                }
            } else if let Some(m) = modulation_attr {
                return Err(error(
                    m.span(),
                    "modulation requires a parameter on the same field",
                ));
            }
            Ok(())
        })();
        if let Err(e) = result {
            if let Some(errors) = &mut errors {
                errors.combine(e);
            } else {
                errors = Some(e);
            }
        }
    }
    if let Some(errors) = errors {
        return Err(errors);
    }
    for (p, arg) in pending {
        let index = ports
            .iter()
            .filter(|p| p.direction == Direction::Input)
            .map(|p| p.index)
            .max()
            .map_or(Some(0), |i| i.checked_add(1))
            .ok_or_else(|| error(arg.span(), "input index overflow"))?;
        let port_prefix = format!("ports[{}]", ports.len());
        spans.insert(port_prefix, arg.span());
        ports.push(modulation(
            arg,
            &mut parameters[p],
            index,
            &format!("parameters[{p}]"),
            &mut spans,
        )?);
    }
    // Store f32 metadata in the same representable domain as the Rust field.
    // Otherwise a default equal to a fractional upper bound could round outside it.
    for parameter in &mut parameters {
        let is_f32 = matches!(&parameter.ty, Type::Path(path) if path.path.is_ident("f32"));
        if is_f32 && parameter.kind == Kind::Float {
            if let ParameterValue::Float(value) = &mut parameter.descriptor.default {
                *value = f64::from(*value as f32);
            }
            if let Some(domain) = &mut parameter.descriptor.domain {
                domain.min = f64::from(domain.min as f32);
                domain.max = f64::from(domain.max as f32);
            }
        }
    }
    descriptor.parameters = parameters.iter().map(|p| p.descriptor.clone()).collect();
    descriptor.ports = ports;
    descriptor.groups = groups;
    Ok(Model {
        descriptor,
        parameters,
        spans,
        crate_path: super::crate_path(crate_override)?,
    })
}
