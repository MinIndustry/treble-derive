use proc_macro2::Span;
use syn::{Expr, Ident, Path, Type};
use treble_meta::*;

pub struct Model {
    pub descriptor: ModuleDescriptor,
    pub parameters: Vec<Parameter>,
    pub spans: std::collections::BTreeMap<String, Span>,
    pub crate_path: syn::Path,
}
pub struct Parameter {
    pub descriptor: ParameterDescriptor,
    pub field: Ident,
    pub ty: Type,
    pub kind: Kind,
    pub default: Expr,
    pub setter: Option<Path>,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Float,
    Int,
    Bool,
    Enum,
}
