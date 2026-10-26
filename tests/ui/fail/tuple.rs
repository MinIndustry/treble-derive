use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad(f32);
fn main(){}
