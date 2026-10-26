use treble_derive::FilterMetaData;
#[derive(FilterMetaData)]
struct Bad { #[filter_parameter(range, 1.0)] x:f32 }
fn main(){}
