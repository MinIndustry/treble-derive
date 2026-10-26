use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad {
    #[treble(param(id="x",default=1.0,range(min=))) ] x:f32,
}
fn main(){}
