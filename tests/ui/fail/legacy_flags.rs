use treble_derive::FilterMetaData;
#[derive(FilterMetaData)]
struct LogFromZero { #[filter_parameter(range, 0.0, 20.0, 1.0, scale = logarithmic)] x: f32 }
#[derive(FilterMetaData)]
struct UnknownAccess { #[filter_parameter(range, 1.0, 192000.0, 44100.0, access = host)] x: f32 }
#[derive(FilterMetaData)]
struct DuplicateFlag { #[filter_parameter(float, 1.0, scale = linear, scale = linear)] x: f32 }
#[derive(FilterMetaData)]
struct FlagBeforePositional { #[filter_parameter(range, 0.0, access = engine, 1.0, 0.5)] x: f32 }
fn main(){}
