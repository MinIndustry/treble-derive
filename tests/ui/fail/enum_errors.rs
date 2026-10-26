use treble_derive::ParameterEnum;
#[derive(ParameterEnum)]
enum Bad {
    #[treble(value(id="same"))] A,
    #[treble(value(id="same"))] B,
    C,
    #[treble(value(id="data"))] Data(u32),
}
fn main(){}
