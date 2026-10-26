use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad {
    #[treble(param(id="frequency",default=24000.0,range(min=-1.0,max=20000.0),scale=logarithmic))] x:f32,
    #[treble(param(id="other",default=2.0,range(min=0.0,max=1.0)))] y:f32,
}
fn main(){}
