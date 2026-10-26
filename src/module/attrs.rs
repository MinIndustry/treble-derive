use proc_macro2::Span;
use std::collections::BTreeMap;
use syn::{
    Expr, Ident, Result, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

pub struct Arg {
    pub key: Ident,
    pub value: Value,
}
pub enum Value {
    Expr(Expr),
    Nested(Options),
    Flag,
}
#[derive(Default)]
pub struct Options(pub BTreeMap<String, Arg>);
impl Parse for Arg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key = Ident::parse_any(input)?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Value::Expr(input.parse()?)
        } else if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            Value::Nested(content.parse()?)
        } else {
            Value::Flag
        };
        Ok(Self { key, value })
    }
}
impl Parse for Options {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let args = Punctuated::<Arg, Token![,]>::parse_terminated(input)?;
        let mut options = Self::default();
        for arg in args {
            let key = arg.key.to_string();
            if options.0.contains_key(&key) {
                return Err(syn::Error::new(
                    arg.key.span(),
                    format!("duplicate option `{key}`"),
                ));
            }
            options.0.insert(key, arg);
        }
        Ok(options)
    }
}
impl Options {
    pub fn take(&mut self, key: &str) -> Option<Arg> {
        self.0.remove(key)
    }
    pub fn required(&mut self, key: &str, span: Span) -> Result<Arg> {
        if key == "default"
            && !self.0.contains_key(key)
            && let Some(typo) = self.0.get("defaut")
        {
            return Err(syn::Error::new(
                typo.span(),
                "unknown option `defaut`; did you mean `default`?",
            ));
        }
        self.take(key)
            .ok_or_else(|| syn::Error::new(span, format!("missing required option `{key}`")))
    }
    pub fn finish(self) -> Result<()> {
        if let Some((key, arg)) = self.0.into_iter().next() {
            let help = if key == "defaut" {
                "; did you mean `default`?"
            } else {
                ""
            };
            Err(syn::Error::new(
                arg.key.span(),
                format!("unknown option `{key}`{help}"),
            ))
        } else {
            Ok(())
        }
    }
}
impl Arg {
    pub fn span(&self) -> Span {
        self.key.span()
    }
    pub fn expr(self) -> Result<Expr> {
        match self.value {
            Value::Expr(e) => Ok(e),
            _ => Err(syn::Error::new(self.key.span(), "expected `= value`")),
        }
    }
    pub fn nested(self) -> Result<Options> {
        match self.value {
            Value::Nested(o) => Ok(o),
            _ => Err(syn::Error::new(
                self.key.span(),
                "expected parenthesized options",
            )),
        }
    }
    pub fn string(self) -> Result<String> {
        let expr = self.expr()?;
        if let Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) = expr
        {
            Ok(s.value())
        } else {
            Err(syn::Error::new(expr.span(), "expected a string literal"))
        }
    }
    pub fn word(self) -> Result<String> {
        let expr = self.expr()?;
        if let Expr::Path(p) = &expr
            && let Some(id) = p.path.get_ident()
        {
            return Ok(id.to_string());
        }
        Err(syn::Error::new(expr.span(), "expected an option name"))
    }
    pub fn number(self) -> Result<f64> {
        number(&self.expr()?)
    }
}
pub fn number(expr: &Expr) -> Result<f64> {
    let result = match expr {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Float(n),
            ..
        }) => n.base10_parse(),
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(n),
            ..
        }) => n.base10_parse(),
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => return number(&u.expr).map(|n| -n),
        _ => {
            return Err(syn::Error::new(
                expr.span(),
                "expected a numeric literal (constant expressions are not supported here)",
            ));
        }
    };
    let n: f64 = result?;
    if !n.is_finite() {
        return Err(syn::Error::new(
            expr.span(),
            "numeric literal must be finite",
        ));
    }
    Ok(n)
}
pub fn docs(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| {
            if let syn::Meta::NameValue(n) = &a.meta
                && let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &n.value
            {
                return Some(s.value().trim().to_owned());
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn attributes(attrs: &[syn::Attribute]) -> Result<Vec<Arg>> {
    let mut result = Vec::new();
    for attr in attrs.iter().filter(|a| a.path().is_ident("treble")) {
        let args = attr.parse_args_with(Punctuated::<Arg, Token![,]>::parse_terminated)?;
        result.extend(args);
    }
    Ok(result)
}
