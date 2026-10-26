use treble_derive::ModuleMetadata;
#[derive(ModuleMetadata)]
#[treble(module(id="x",version=1))]
struct Bad {
    #[treble(context(sample_rate),param(id="sample_rate",default=48000.0))] rate:f32,
    #[treble(modulation(port="cv"))] value:f32,
}
fn main(){}
