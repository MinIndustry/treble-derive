# Treble Derive

Derives for `treble-meta` 0.3.0 module contracts. The new derives describe modules
and edit typed configuration; they do not register factories or process audio.

## A playable source declaration

```rust
use treble_derive::{ModuleMetadata, ModuleParameters, ParameterEnum};
use treble_meta::{ModuleMetadata as _, ParameterValue, UpdateMode};

#[derive(ParameterEnum)]
enum Waveform {
    #[treble(value(id = "sine", name = "Sine"))]
    Sine,
    #[treble(value(id = "saw", name = "Sawtooth"))]
    Saw,
}

/// An illustrative source; this example contains no DSP implementation.
#[derive(ModuleMetadata, ModuleParameters)]
#[treble(module(
    id = "example.oscillator", version = 1,
    name = "Oscillator", category = "Sources"
))]
#[treble(output(id = "out", index = 0, signal = audio(channels = 2)))]
struct Oscillator {
    /// Base pitch, independent of modulation.
    #[treble(param(
        id = "frequency", default = 440.0,
        range(min = 20.0, max = 20000.0), unit = hz, scale = logarithmic,
        update = live, smooth = milliseconds(5.0)
    ))]
    #[treble(modulation(
        port = "pitch_cv", mode = add, mapping = octaves, depth = 1.0,
        domain = bipolar, rate = sample
    ))]
    frequency_hz: f32,

    #[treble(param(id = "follow_key", default = true))]
    follow_key: bool,

    #[treble(param(id = "waveform", default = Waveform::Sine))]
    waveform: Waveform,

    #[treble(input(id = "note", index = 0, signal = note))]
    note_stream: (),

    #[treble(input(id = "trigger", index = 1, signal = gate))]
    trigger_stream: (),

    #[treble(context(sample_rate))]
    sample_rate: f32,
}

let mut oscillator = Oscillator {
    frequency_hz: 440.0, follow_key: true, waveform: Waveform::Sine,
    note_stream: (), trigger_stream: (), sample_rate: 48000.0,
};
Oscillator::descriptor().validate()?;
assert_eq!(oscillator.set_parameter_value("follow_key", ParameterValue::Bool(false))?, UpdateMode::Live);
assert_eq!(oscillator.parameter_value("frequency")?, ParameterValue::Float(440.0));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`ModuleMetadata` implements the published `treble_meta::ModuleMetadata` trait.
`ParameterEnum` implements its corresponding published trait for unit variants.
`ModuleParameters` generates **inherent** methods because published metadata does
not yet define a dynamic parameter-access trait:

```text
parameter_value(&self, id: &str) -> Result<ParameterValue, ValidationErrors>
set_parameter_value(&mut self, id: &str, value: ParameterValue)
    -> Result<UpdateMode, ValidationErrors>
```

These names avoid collisions with the legacy scalar `MetaFilter::set_parameter`.
Errors use the shared path/message structures, with paths such as
`parameters.frequency`; no per-module public error types are generated.

Setters validate the typed value, ownership, finiteness and declared domain before
assignment, then check conversion to the Rust field type. They reject wrong types,
unknown IDs, unknown enum IDs, integer overflow and float overflow without clamping.
A `Rebuild` result is a hint: the **configuration has changed**, but no graph is
compiled or installed. Hosts should apply structural changes to their draft
configuration and prepare/install audio separately. These methods allocate metadata
and error data and are not intended for a real-time audio callback.

## Attribute grammar

Every exported module, parameter, port and enum variant needs an explicit stable
ID. Rust identifiers, display names and vector order do not define saved identity.
Unannotated fields are private implementation state. Named-field and unit structs
are supported, including generics and where clauses. Enum parameter fields add a
`ParameterEnum` bound to generated implementations. Tuple structs and data-carrying
enum variants produce explicit diagnostics.

Container attributes:

- `module(id, version, name?, description?, category?)`: exactly one; positive
  version required. Name defaults to the Rust type name; category to `Other`.
- `output(id, index, signal, name?, connections?)`: repeatable; default `fan_out`.
- `group(id, name?, members?, widget?)`: semantic control grouping (example below).
- `crate = path`: optional explicit path to the metadata crate or a re-export.
  Renamed Cargo dependencies are resolved automatically for the new derives.

Field attributes:

- `param(id, default, name?, description?, kind?, range?, unit?, scale?, access?,
  update?, smooth?, setter?)`: one per field.
- `input(id, index, signal, name?, connections?)`: explicit stream sockets;
  default `single`. No implicit input/output is generated.
- `modulation(port, name?, index?, mode?, modes?, mapping?, depth?, domain?, rate?,
  connections?)`: requires a parameter on the same field. Creates a bound control
  input and the parameter's modulation descriptor. Defaults are `add`, `[add]`,
  `native`, depth `1`, `bipolar`, `block`, `single`. An omitted index is allocated
  after explicit input indices. State explicit indices when implementation bindings
  depend on them. All ports are validated for duplicate IDs/indices.
- `context(sample_rate)`: marks an engine context field, excluded from parameters
  and sockets. It cannot also declare a parameter, input or modulation. This is a
  checked declaration only: factories/hosts must initialize the context themselves.

Supported signals: `audio(channels = N)`, `control(domain = bipolar|unipolar,
rate = block|sample)`, `gate`, `note`. Gates and notes describe distinct contracts;
the runtime must support them before these sockets become playable.

Parameters infer float, integer and boolean kinds from primitive field names;
other path types are treated as `ParameterEnum`. `kind = float|int|bool|enumeration`
overrides inference for aliases. Arbitrary compound field types are rejected unless
a compatible explicit kind is supplied. Generated Rust type checking still verifies
that defaults, conversion operations and setter hooks match the field type.

Numeric `default`, range bounds/step, versions, indices and modulation values must
be literals (negative and suffixed literals work). Boolean defaults must be `true`
or `false`. Numeric constants and arbitrary expressions are deliberately rejected
at their span; the macro cannot resolve or evaluate external Rust constants. Enum
defaults are typed Rust expressions, so they may use variant paths or constants;
their dynamic IDs and variant lists must be validated during registry construction.
Handwritten constructors remain responsible for matching descriptor defaults.

`range(min = ..., max = ..., step = ...)` uses inclusive bounds; step is optional.
`unit` and `scale` require a range. Units: `hz`/`hertz`, `seconds`, `milliseconds`,
`decibels`, `ratio`, `q`, `semitones`, `cents`, `percent`, `unitless`. Scales: `linear`
and `logarithmic`. Access: `user` (default) or read-only `engine`. Update: `live`
(default) or `rebuild`. Smoothing only applies to live numeric parameters. Modulation
requires a live user-owned numeric parameter. See treble-meta for mapping formulas
and valid connection combinations.

Portable integer values use `i32`. Wider Rust fields can be read/written only while
the value fits that representation; narrower fields use checked conversions. No
range is silently inferred for a field: declare one to constrain the UI.
For direct `f32` fields, defaults and numeric bounds are quantized through `f32`
before descriptor validation. Consequently a fractional endpoint remains reachable
and getter/setter round-trips stay within the advertised domain. Bounds that become
nonfinite or collapse to one value are rejected. Aliases cannot be resolved by a
procedural macro; authors must validate their generated descriptors explicitly.

## Setter hooks and groups

A custom setter can update coefficients or other cached configuration:

```rust,ignore
#[treble(param(id = "cutoff", default = 1000.0, setter = Self::set_cutoff))]
cutoff: f32,
// Hook signature: fn set_cutoff(&mut self, value: f32) -> Result<(), String>
```

Validation and conversion run before the hook. The hook owns its mutation/error
contract; returning `Err` does not roll back changes the hook already made.

ADSR roles bind stable numeric parameter IDs. Omitting `members` infers them from
the envelope roles; supplying members validates all bindings against that list:

```rust,ignore
#[treble(group(
    id = "amplitude_envelope", name = "Envelope",
    widget = envelope(attack = "attack", decay = "decay", sustain = "sustain", release = "release")
))]
```

Descriptions come only from documentation comments unless overridden explicitly.
Rack positions, widths, cable shapes, registration, factory construction, runtime
smoothing/modulation and automatic compilation remain outside these derives.

## Diagnostics and legacy migration

The new implementation separates attribute parsing with spans, an internal model,
validation and expansion. It rejects unknown/duplicate options, invalid defaults,
IDs, bounds, ports, ownership, modulation and group references. Independent field,
variant and descriptor errors are combined where parsing can recover.
Compile-pass/fail fixtures exercise the messages and highlighted source locations.

`FilterMetaData` keeps its legacy grammar, scalar API, core-bound factory/inventory
registration and legacy token output. Its positional parser now reports `syn`
errors instead of panicking, handles negative/suffixed literals correctly, and
rejects malformed counts, values and ranges. Existing valid declarations retain
their behavior; previously ignored extra arguments now produce errors. Renamed
crate resolution and generic support belong to the new derives; legacy automatic
registration still assumes treble-core's module layout and dependencies.

Release order: published `treble-meta` **0.3.0** → publish this crate **0.3.0** →
update `treble-core` separately. The new derives never emit inventory registration,
core graph types or DSP code. Publish/merge decisions remain with the maintainer.
