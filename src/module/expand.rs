use super::{
    attrs,
    model::{Kind, Model, Parameter},
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Result, spanned::Spanned};
use treble_meta::*;

fn variant<T: std::fmt::Debug>(value: &T) -> syn::Ident {
    syn::Ident::new(&format!("{value:?}"), proc_macro2::Span::call_site())
}
fn option_float(value: Option<f64>) -> TokenStream {
    match value {
        Some(value) => quote!(::core::option::Option::Some(#value)),
        None => quote!(::core::option::Option::None),
    }
}
fn domain(value: &Option<NumericDomain>, m: &syn::Path) -> TokenStream {
    match value {
        None => quote!(None),
        Some(d) => {
            let (min, max, unit, scale, step) = (
                d.min,
                d.max,
                variant(&d.unit),
                variant(&d.scale),
                option_float(d.step),
            );
            quote!(Some(#m::NumericDomain{min:#min,max:#max,unit:#m::Unit::#unit,scale:#m::Scale::#scale,step:#step}))
        }
    }
}
fn modulation(value: &Option<ModulationDescriptor>, m: &syn::Path) -> TokenStream {
    match value {
        None => quote!(None),
        Some(d) => {
            let modes = d.modes.iter().map(variant);
            let mode = variant(&d.default_mode);
            let mapping = variant(&d.mapping);
            let depth = d.default_depth;
            quote!(Some(#m::ModulationDescriptor{modes:vec![#(#m::ModulationMode::#modes),*],default_mode:#m::ModulationMode::#mode,mapping:#m::ModulationMapping::#mapping,default_depth:#depth}))
        }
    }
}
fn signal(value: &SignalType, m: &syn::Path) -> TokenStream {
    match value {
        SignalType::Audio { channels } => quote!(#m::SignalType::Audio{channels:#channels}),
        SignalType::Control { domain, rate } => {
            let domain = variant(domain);
            let rate = variant(rate);
            quote!(#m::SignalType::Control{domain:#m::ControlDomain::#domain,rate:#m::ControlRate::#rate})
        }
        SignalType::Gate => quote!(#m::SignalType::Gate),
        SignalType::Note => quote!(#m::SignalType::Note),
    }
}
fn descriptor(param: &Parameter, m: &syn::Path) -> TokenStream {
    let p = &param.descriptor;
    let (id, name, description) = (&p.id, &p.name, &p.description);
    let ty = &param.ty;
    let expr = &param.default;
    let (value_type, default) = match param.kind {
        Kind::Float => (
            quote!(#m::ValueType::Float),
            quote_spanned!(expr.span()=>{let value:#ty=#expr;#m::ParameterValue::Float(::core::convert::Into::<f64>::into(value))}),
        ),
        Kind::Int => {
            let value = if let ParameterValue::Int(value) = p.default {
                value
            } else {
                0
            };
            (
                quote!(#m::ValueType::Int),
                quote_spanned!(expr.span()=>{let _:#ty=#expr;#m::ParameterValue::Int(#value)}),
            )
        }
        Kind::Bool => (
            quote!(#m::ValueType::Bool),
            quote_spanned!(expr.span()=>{let value:#ty=#expr;#m::ParameterValue::Bool(value)}),
        ),
        Kind::Enum => (
            quote_spanned!(ty.span()=>#m::ValueType::Enum{variants:<#ty as #m::ParameterEnum>::variants()}),
            quote_spanned!(expr.span()=>{let value:#ty=#expr;#m::ParameterValue::Enum(<#ty as #m::ParameterEnum>::id(&value).to_owned())}),
        ),
    };
    let domain = domain(&p.domain, m);
    let access = variant(&p.access);
    let update = variant(&p.update);
    let smoothing = option_float(p.smoothing_ms);
    let modulation = modulation(&p.modulation, m);
    quote!(#m::ParameterDescriptor{id:#id.into(),name:#name.into(),description:#description.into(),value_type:#value_type,default:#default,domain:#domain,access:#m::ParameterAccess::#access,update:#m::UpdateMode::#update,smoothing_ms:#smoothing,modulation:#modulation})
}
fn generics(input: &DeriveInput, model: &Model) -> syn::Generics {
    let mut generics = input.generics.clone();
    let m = &model.crate_path;
    for p in &model.parameters {
        if p.kind == Kind::Enum {
            let ty = &p.ty;
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(#ty:#m::ParameterEnum));
        }
    }
    generics
}
pub fn metadata(input: &DeriveInput, model: &Model) -> TokenStream {
    let name = &input.ident;
    let m = &model.crate_path;
    let generics = generics(input, model);
    let (ig, tg, wc) = generics.split_for_impl();
    let d = &model.descriptor;
    let (id, version, title, description, category) =
        (&d.type_id, d.version, &d.name, &d.description, &d.category);
    let parameters = model.parameters.iter().map(|p| descriptor(p, m));
    let ports = d.ports.iter().map(|p| {
        let (id, name, index) = (&p.id, &p.name, p.index);
        let direction = variant(&p.direction);
        let connections = variant(&p.connections);
        let signal = signal(&p.signal, m);
        let binding = match &p.binding {
            PortBinding::Stream => quote!(#m::PortBinding::Stream),
            PortBinding::Parameter { parameter_id } => quote!(
                #m::PortBinding::Parameter { parameter_id: #parameter_id.into() }
            ),
        };
        quote! {
            #m::PortDescriptor {
                id: #id.into(), name: #name.into(), index: #index,
                direction: #m::Direction::#direction,
                connections: #m::ConnectionPolicy::#connections,
                signal: #signal, binding: #binding,
            }
        }
    });
    let groups = d.groups.iter().map(|g| {
        let (id, name, members) = (&g.id, &g.name, &g.members);
        let presentation = match &g.presentation {
            GroupPresentation::Generic => quote!(#m::GroupPresentation::Generic),
            GroupPresentation::Envelope { bindings: b } => {
                let (attack, decay, sustain, release) =
                    (&b.attack, &b.decay, &b.sustain, &b.release);
                quote! {
                    #m::GroupPresentation::Envelope { bindings: #m::EnvelopeBindings {
                        attack: #attack.into(), decay: #decay.into(),
                        sustain: #sustain.into(), release: #release.into(),
                    } }
                }
            }
        };
        quote! {
            #m::ControlGroup {
                id: #id.into(), name: #name.into(),
                members: vec![#(#members.into()),*], presentation: #presentation,
            }
        }
    });
    quote! {
        impl #ig #m::ModuleMetadata for #name #tg #wc {
            fn descriptor() -> #m::ModuleDescriptor {
                #m::ModuleDescriptor {
                    type_id: #id.into(), version: #version, name: #title.into(),
                    description: #description.into(), category: #category.into(),
                    parameters: vec![#(#parameters),*],
                    ports: vec![#(#ports),*], groups: vec![#(#groups),*],
                }
            }
        }
    }
}

fn failure(m: &syn::Path, message: TokenStream) -> TokenStream {
    quote!(#m::ValidationErrors(vec![#m::ValidationError{path:format!("parameters.{}",id),message:#message}]))
}
pub fn parameters(input: &DeriveInput, model: &Model) -> TokenStream {
    let name = &input.ident;
    let m = &model.crate_path;
    let generics = generics(input, model);
    let (ig, tg, wc) = generics.split_for_impl();
    let unknown = failure(m, quote!("unknown parameter".into()));
    let readonly = failure(m, quote!("engine-owned parameter is read-only".into()));
    let invalid = failure(
        m,
        quote!("value is not representable by the Rust field type".into()),
    );
    let wrong = failure(m, quote!("value does not match parameter type".into()));
    let validation = quote! {
        parameter.validate_value(&value).map_err(|errors| #m::ValidationErrors(
            errors.0.into_iter().map(|error| #m::ValidationError {
                path: format!("parameters.{}", id), message: error.message,
            }).collect()
        ))?;
    };
    let getters = model.parameters.iter().map(|p| {
        let id = &p.descriptor.id;
        let field = &p.field;
        let ty = &p.ty;
        let value = match p.kind {
            Kind::Float => {
                quote!(#m::ParameterValue::Float(::core::convert::Into::<f64>::into(self.#field)))
            }
            Kind::Int => quote!(#m::ParameterValue::Int(
                ::core::convert::TryInto::<i32>::try_into(self.#field).map_err(|_| #invalid)?
            )),
            Kind::Bool => quote!(#m::ParameterValue::Bool(self.#field)),
            Kind::Enum => quote!(#m::ParameterValue::Enum(
                <#ty as #m::ParameterEnum>::id(&self.#field).into()
            )),
        };
        quote!(#id => Ok(#value),)
    });
    let setters = model.parameters.iter().map(|p| {
        let id = &p.descriptor.id;
        let field = &p.field;
        let ty = &p.ty;
        if p.descriptor.access == ParameterAccess::Engine {
            return quote!(#id => Err(#readonly),);
        }
        let converted = match p.kind {
            Kind::Float => quote!({
                let #m::ParameterValue::Float(number) = value else { return Err(#wrong); };
                let converted = number as #ty;
                if !converted.is_finite() { return Err(#invalid); }
                let value = #m::ParameterValue::Float(::core::convert::Into::<f64>::into(converted));
                #validation
                converted
            }),
            Kind::Int => quote!({
                let #m::ParameterValue::Int(number) = value else { return Err(#wrong); };
                <#ty as ::core::convert::TryFrom<i32>>::try_from(number).map_err(|_| #invalid)?
            }),
            Kind::Bool => quote!({
                let #m::ParameterValue::Bool(number) = value else { return Err(#wrong); };
                number
            }),
            Kind::Enum => quote!({
                let #m::ParameterValue::Enum(name) = value else { return Err(#wrong); };
                <#ty as #m::ParameterEnum>::from_id(&name).ok_or_else(|| #invalid)?
            }),
        };
        let apply = if let Some(setter) = &p.setter {
            let error = failure(m, quote!(message));
            quote!(#setter(self, converted).map_err(|message: ::std::string::String| #error)?;)
        } else {
            quote!(self.#field = converted;)
        };
        quote!(#id => { let converted: #ty = #converted; #apply Ok(parameter.update.clone()) },)
    });
    quote! {
        impl #ig #name #tg #wc {
            /// Read a typed value by stable parameter ID. No lossy scalar coercion.
            pub fn parameter_value(&self, id: &str) -> ::core::result::Result<#m::ParameterValue, #m::ValidationErrors> {
                match id { #(#getters)* _ => Err(#unknown) }
            }
            /// Validate and mutate configuration, returning an update hint. A Rebuild
            /// result does not rebuild or install DSP; hosts must apply it separately.
            pub fn set_parameter_value(&mut self, id: &str, value: #m::ParameterValue) -> ::core::result::Result<#m::UpdateMode, #m::ValidationErrors> {
                let descriptor = <Self as #m::ModuleMetadata>::descriptor();
                let parameter = descriptor.parameters.iter().find(|p| p.id == id).ok_or_else(|| #unknown)?;
                if parameter.access == #m::ParameterAccess::Engine { return Err(#readonly); }
                // Float bounds describe the field's representable domain, so
                // check those after conversion. Other kinds validate directly.
                if parameter.value_type != #m::ValueType::Float {
                    #validation
                }
                match id { #(#setters)* _ => Err(#unknown) }
            }
        }
    }
}

pub fn enumeration(input: &DeriveInput) -> Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "ParameterEnum requires an enum",
        ));
    };
    let mut override_path = None;
    for arg in attrs::attributes(&input.attrs)? {
        let span = arg.span();
        if arg.key != "crate" {
            return Err(syn::Error::new(
                span,
                "only crate = path is allowed on a ParameterEnum container",
            ));
        }
        if override_path.is_some() {
            return Err(syn::Error::new(span, "duplicate crate override"));
        }
        let expr = arg.expr()?;
        if let syn::Expr::Path(path) = expr {
            override_path = Some(path.path);
        } else {
            return Err(syn::Error::new(
                expr.span(),
                "crate override must be a Rust path",
            ));
        }
    }
    let m = super::crate_path(override_path)?;
    let mut ids = std::collections::BTreeSet::new();
    let mut variants = Vec::new();
    let mut errors: Option<syn::Error> = None;
    for v in &data.variants {
        let result = (|| {
            if !matches!(v.fields, syn::Fields::Unit) {
                return Err(syn::Error::new(
                    v.fields.span(),
                    "ParameterEnum variants must be unit variants",
                ));
            }
            let mut declaration = None;
            for arg in attrs::attributes(&v.attrs)? {
                let span = arg.span();
                if arg.key != "value" {
                    return Err(syn::Error::new(
                        span,
                        "expected value(id = \"...\", name = \"...\")",
                    ));
                }
                if declaration.is_some() {
                    return Err(syn::Error::new(span, "duplicate enum value declaration"));
                }
                declaration = Some(arg);
            }
            let arg = declaration.ok_or_else(|| {
                syn::Error::new(v.ident.span(), "missing #[treble(value(id = \"...\"))]")
            })?;
            let span = arg.span();
            let mut opts = arg.nested()?;
            let id_arg = opts.required("id", span)?;
            let id_span = id_arg.span();
            let id = id_arg.string()?;
            if id.is_empty() || id.trim() != id {
                return Err(syn::Error::new(
                    id_span,
                    "enum ID must be nonempty and have no surrounding whitespace",
                ));
            }
            if !ids.insert(id.clone()) {
                return Err(syn::Error::new(
                    id_span,
                    format!("duplicate enum value ID `{id}`"),
                ));
            }
            let name = opts
                .take("name")
                .map(attrs::Arg::string)
                .transpose()?
                .unwrap_or_else(|| v.ident.to_string());
            opts.finish()?;
            variants.push((&v.ident, id, name));
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
    if variants.is_empty() && errors.is_none() {
        return Err(syn::Error::new(
            input.ident.span(),
            "ParameterEnum requires at least one variant",
        ));
    }
    if let Some(e) = errors {
        return Err(e);
    }
    let name = &input.ident;
    let (ig, tg, wc) = input.generics.split_for_impl();
    let descriptions = variants
        .iter()
        .map(|(_, id, name)| quote!(#m::EnumVariant{id:#id.into(),name:#name.into()}));
    let get = variants.iter().map(|(v, id, _)| quote!(Self::#v=>#id));
    let from = variants
        .iter()
        .map(|(v, id, _)| quote!(#id=>Some(Self::#v)));
    Ok(
        quote!(impl #ig #m::ParameterEnum for #name #tg #wc {fn variants()->::std::vec::Vec<#m::EnumVariant>{vec![#(#descriptions),*]}fn id(&self)->&'static str{match self{#(#get),*}}fn from_id(id:&str)->Option<Self>{match id{#(#from),*,_=>None}}}),
    )
}
