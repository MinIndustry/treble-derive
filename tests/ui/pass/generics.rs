use treble_derive::{ModuleMetadata,ModuleParameters,ParameterEnum};
use treble_meta::{ModuleMetadata as _,ParameterValue};
#[derive(ParameterEnum)]
enum Choice { #[treble(value(id="a"))] A }
#[derive(ModuleMetadata,ModuleParameters)]
#[treble(module(id="example.generic",version=1))]
struct Module<T> { #[treble(param(id="choice",default=T::from_id("a").unwrap()))] choice:T }
fn main(){ let mut m=Module{choice:Choice::A};Module::<Choice>::descriptor().validate().unwrap();m.set_parameter_value("choice",ParameterValue::Enum("a".into())).unwrap(); }
