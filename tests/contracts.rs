#![allow(dead_code)]
use treble_derive::{ModuleMetadata, ModuleParameters, ParameterEnum};
use treble_meta::{ModuleMetadata as _, ParameterEnum as _, ParameterValue, UpdateMode};

#[derive(ParameterEnum, Debug, PartialEq)]
enum Waveform {
    #[treble(value(id = "sine", name = "Sine"))]
    Sine,
    #[treble(value(id = "saw", name = "Sawtooth"))]
    Saw,
}

/// A source with explicitly declared note and gate sockets.
#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(
    id = "example.oscillator",
    version = 1,
    name = "Oscillator",
    category = "Sources"
))]
#[treble(output(id = "out", index = 0, signal = audio(channels = 2)))]
struct Oscillator {
    #[treble(param(id = "frequency", default = 440.0, range(min = 20.0, max = 20000.0), unit = hz, scale = logarithmic, smooth = milliseconds(5.0), setter = Self::set_frequency))]
    #[treble(modulation(port = "pitch_cv", mode = add, modes = [add, multiply], mapping = octaves, depth = 1.0, domain = bipolar, rate = sample))]
    frequency_hz: f32,
    #[treble(param(id = "follow_key", default = true))]
    follow_key: bool,
    #[treble(param(id = "waveform", default = Waveform::Sine))]
    waveform: Waveform,
    #[treble(param(id = "voices", default = 8, range(min = 1, max = 32), update = rebuild))]
    voices: usize,
    #[treble(param(id = "internal", default = 42, access = engine))]
    internal: i32,
    #[treble(input(id = "note", index = 0, signal = note))]
    note: (),
    #[treble(input(id = "trigger", index = 1, signal = gate))]
    trigger: (),
    #[treble(context(sample_rate))]
    sample_rate: f32,
    setter_count: usize,
}
impl Oscillator {
    fn new() -> Self {
        Self {
            frequency_hz: 440.0,
            follow_key: true,
            waveform: Waveform::Sine,
            voices: 8,
            internal: 42,
            note: (),
            trigger: (),
            sample_rate: 48000.0,
            setter_count: 0,
        }
    }
    fn set_frequency(&mut self, value: f32) -> Result<(), String> {
        if value == 666.0 {
            return Err("reserved fixture value".into());
        }
        self.frequency_hz = value;
        self.setter_count += 1;
        Ok(())
    }
}

#[test]
fn generated_descriptor_matches_values_and_has_no_implicit_sockets_or_context_knob() {
    let d = Oscillator::descriptor();
    d.validate().unwrap();
    let osc = Oscillator::new();
    for p in &d.parameters {
        assert_eq!(osc.parameter_value(&p.id).unwrap(), p.default);
    }
    assert_eq!(
        d.description,
        "A source with explicitly declared note and gate sockets."
    );
    assert_eq!(d.ports.len(), 4);
    assert_eq!(
        d.ports.iter().find(|p| p.id == "pitch_cv").unwrap().index,
        2
    );
    assert!(!d.parameters.iter().any(|p| p.id == "sample_rate"));
}
#[test]
fn typed_updates_validate_before_mutation_and_return_rebuild_hint() {
    let mut osc = Oscillator::new();
    assert_eq!(
        osc.set_parameter_value("frequency", ParameterValue::Float(880.0))
            .unwrap(),
        UpdateMode::Live
    );
    assert_eq!(osc.setter_count, 1);
    for value in [
        ParameterValue::Int(880),
        ParameterValue::Float(f64::NAN),
        ParameterValue::Float(0.0),
        ParameterValue::Float(666.0),
    ] {
        assert!(osc.set_parameter_value("frequency", value).is_err());
        assert_eq!(osc.frequency_hz, 880.0);
    }
    assert_eq!(osc.setter_count, 1);
    osc.set_parameter_value("follow_key", ParameterValue::Bool(false))
        .unwrap();
    assert!(!osc.follow_key);
    osc.set_parameter_value("waveform", ParameterValue::Enum("saw".into()))
        .unwrap();
    assert_eq!(osc.waveform, Waveform::Saw);
    assert!(
        osc.set_parameter_value("waveform", ParameterValue::Enum("square".into()))
            .is_err()
    );
    assert_eq!(
        osc.set_parameter_value("voices", ParameterValue::Int(16))
            .unwrap(),
        UpdateMode::Rebuild
    );
    assert_eq!(osc.voices, 16);
    assert!(
        osc.set_parameter_value("internal", ParameterValue::Int(0))
            .unwrap_err()
            .to_string()
            .contains("read-only")
    );
    assert_eq!(
        osc.parameter_value("missing").unwrap_err().0[0].path,
        "parameters.missing"
    );
    assert!(
        osc.set_parameter_value("missing", ParameterValue::Bool(true))
            .is_err()
    );
}
#[test]
fn enum_ids_are_not_ordinals() {
    assert_eq!(Waveform::Saw.id(), "saw");
    assert_eq!(Waveform::from_id("sine"), Some(Waveform::Sine));
    assert!(Waveform::from_id("1").is_none());
}

#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(id = "example.numeric", version = 1))]
struct Numeric {
    #[treble(param(id = "small", default = 0))]
    small: u8,
    #[treble(param(id = "wide", default = 0))]
    wide: usize,
    #[treble(param(id = "float", default = 0.0))]
    float: f32,
}
#[test]
fn conversions_are_checked_in_both_directions() {
    let mut n = Numeric {
        small: 0,
        wide: usize::MAX,
        float: 0.0,
    };
    assert!(n.parameter_value("wide").is_err());
    assert!(
        n.set_parameter_value("small", ParameterValue::Int(-1))
            .is_err()
    );
    assert_eq!(n.small, 0);
    assert!(
        n.set_parameter_value("small", ParameterValue::Int(256))
            .is_err()
    );
    assert_eq!(n.small, 0);
    assert!(
        n.set_parameter_value("float", ParameterValue::Float(f64::MAX))
            .is_err()
    );
    assert_eq!(n.float, 0.0);
}

#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(id = "example.envelope", version = 1))]
#[treble(group(id="adsr",name="Envelope",widget=envelope(attack="a",decay="d",sustain="s",release="r")))]
struct Envelope {
    #[treble(param(id="a",default=0.01,range(min=0.0,max=10.0),unit=seconds))]
    a: f64,
    #[treble(param(id="d",default=0.1,range(min=0.0,max=10.0),unit=seconds))]
    d: f64,
    #[treble(param(id="s",default=0.7,range(min=0.0,max=1.0),unit=ratio))]
    s: f64,
    #[treble(param(id="r",default=0.2,range(min=0.0,max=10.0),unit=seconds))]
    r: f64,
}
#[test]
fn structured_envelope_is_semantic_metadata() {
    let d = Envelope::descriptor();
    d.validate().unwrap();
    assert_eq!(d.groups[0].members, ["a", "d", "s", "r"]);
}

#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(id = "example.rounding", version = 1))]
struct Rounding {
    #[treble(param(id = "value", default = 0.1, range(min = 0.0, max = 0.1)))]
    value: f32,
}
#[test]
fn fractional_f32_bounds_and_defaults_share_the_field_representation() {
    let d = Rounding::descriptor();
    d.validate().unwrap();
    let mut value = Rounding { value: 0.1 };
    value
        .set_parameter_value("value", value.parameter_value("value").unwrap())
        .unwrap();
    assert_eq!(
        d.parameters[0].default,
        ParameterValue::Float(f64::from(0.1f32))
    );
}

#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(id = "example.endpoints", version = 1))]
struct Endpoints {
    #[treble(param(id = "upper", default = 0.3, range(min = 0.0, max = 0.3)))]
    upper: f32,
    #[treble(param(id = "lower", default = 0.3, range(min = 0.3, max = 1.0)))]
    lower: f32,
}
#[test]
fn fractional_f32_endpoints_round_trip_through_typed_setters() {
    let d = Endpoints::descriptor();
    d.validate().unwrap();
    let mut instance = Endpoints {
        upper: 0.3,
        lower: 0.3,
    };
    instance
        .set_parameter_value("upper", ParameterValue::Float(0.3))
        .unwrap();
    instance
        .set_parameter_value("lower", ParameterValue::Float(0.3))
        .unwrap();
    assert!(
        instance
            .set_parameter_value("lower", ParameterValue::Float(0.2))
            .is_err()
    );
    for p in d.parameters {
        instance
            .set_parameter_value(&p.id, p.default.clone())
            .unwrap();
        assert_eq!(instance.parameter_value(&p.id).unwrap(), p.default);
    }
}
