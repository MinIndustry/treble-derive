# Changelog

## 0.3.0

- Depend on published treble-meta 0.3.0, with no local metadata/core patches.
- Add `ModuleMetadata`, `ModuleParameters` and `ParameterEnum` derives using named
  nested `#[treble(...)]` attributes and stable explicit identities.
- Separate typed parameters, explicit audio/control/gate/note ports, context fields,
  modulation capabilities, ADSR groups and optional checked setter hooks.
- Generate typed inherent parameter accessors with shared validation errors and
  live/rebuild hints; no runtime compilation or registration is generated.
- Resolve renamed metadata dependencies, support explicit crate overrides and
  generics, and preserve Rust type checking for defaults and enum contracts.
- Validate with source spans and accumulated diagnostics. Quantize f32 domains
  consistently with field representation; reject overflowing/collapsed bounds.
- Replace the legacy positional panic-based parser with structured errors while
  preserving its scalar API and core-bound registration behavior.
- Add runtime contract tests, renamed-dependency fixture and compile-pass/fail
  diagnostics snapshots. Numeric constant expressions are explicitly unsupported.
