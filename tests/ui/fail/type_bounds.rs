use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=0))]
struct Bad {
    #[treble(param(id="small",default=256))] small:u8,
    #[treble(param(id="float",default=1e100))] float:f32,
}
fn main(){}
