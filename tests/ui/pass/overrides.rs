use treble_meta as renamed;
use treble_derive::{ModuleMetadata,ModuleParameters,ParameterEnum};
use renamed::ModuleMetadata as _;
type Frequency=f64;
#[derive(ParameterEnum)] #[treble(crate=renamed)]
enum Choice{ #[treble(value(id="a"))] A }
#[derive(ModuleMetadata,ModuleParameters)]
#[treble(crate=renamed,module(id="example.alias",version=1))]
struct Alias{ #[treble(param(id="frequency",kind=float,default=-1.0f64,range(min=-2.0,max=2.0)))] frequency:Frequency }
fn main(){Alias::descriptor().validate().unwrap();let mut a=Alias{frequency:-1.0};a.set_parameter_value("frequency",renamed::ParameterValue::Float(0.0)).unwrap();let _=Choice::A;}
