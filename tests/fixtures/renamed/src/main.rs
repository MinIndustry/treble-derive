use treble_derive::{ModuleMetadata,ModuleParameters,ParameterEnum};
use metadata::{ModuleMetadata as _,ParameterValue};
#[derive(ParameterEnum)]
enum Mode { #[treble(value(id="sine"))] Sine }
#[derive(ModuleMetadata,ModuleParameters)]
#[treble(module(id="example.renamed",version=1))]
struct Instrument { #[treble(param(id="mode",default=Mode::Sine))] mode:Mode }
fn main(){let mut instrument=Instrument{mode:Mode::Sine};Instrument::descriptor().validate().unwrap();instrument.set_parameter_value("mode",ParameterValue::Enum("sine".into())).unwrap();}
