use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad { #[treble(param(id="x",defaut=1.0))] x:f32 }
fn main(){}
