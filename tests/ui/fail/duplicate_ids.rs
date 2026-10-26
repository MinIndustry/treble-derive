use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
#[treble(output(id="out",index=0,signal=audio(channels=0)))]
#[treble(output(id="out",index=0,signal=gate))]
struct Bad { #[treble(param(id="x",default=0.0))] a:f32, #[treble(param(id="x",default=0.0))] b:f32 }
fn main(){}
