use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
#[treble(group(id="env",widget=envelope(attack="a",decay="d",sustain="s",release="missing")))]
struct Bad {
    #[treble(param(id="a",default=0.0))] a:f32,
    #[treble(param(id="d",default=0.0))] d:f32,
    #[treble(param(id="s",default=0.0))] s:f32,
}
fn main(){}
