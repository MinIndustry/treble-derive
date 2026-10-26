use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad { #[treble(param(id="x",default=0.0,default=1.0))] x:f32 }
fn main(){}
