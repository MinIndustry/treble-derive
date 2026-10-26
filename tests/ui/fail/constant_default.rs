use treble_derive::ModuleMetadata;
const DEFAULT:f32=440.0;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad { #[treble(param(id="x",default=DEFAULT))] x:f32 }
fn main(){}
