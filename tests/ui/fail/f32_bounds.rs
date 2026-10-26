use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad {
    #[treble(param(id="wide",default=0.0,range(min=0.0,max=1e100)))] wide:f32,
    #[treble(param(id="narrow",default=1.0,range(min=1.0,max=1.000000001)))] narrow:f32,
}
fn main(){}
