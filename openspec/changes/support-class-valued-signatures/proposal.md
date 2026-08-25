## Why

Free functions whose signatures name a compiled class already lower and translate, but their
generated Python↔Rust bindings name the inner Rust struct instead of its `#[pyclass]` wrapper and
therefore do not compile. The decorator also rejects the annotation before the whole project can
resolve the class, leaving a supported IR shape unreachable through compylr's primary API and
forcing the differential boundary suite to exclude its only fixture for this case.

## What Changes

- Accept a compiled class as a direct parameter or return annotation on a top-level free function.
- Resolve potentially class-valued annotations with the whole project's class table; defer only
  annotations that can become resolvable, while reporting a located error at build time for an
  unknown or misspelled class.
- Reject nested class-valued boundary annotations such as `list[Tally]` with a located diagnostic
  before Rust source is emitted; nested instance conversion remains out of scope.
- Emit direct instance parameters as borrows and have the Python bridge borrow the stable wrapper's
  inner value, preserving the identity and persistent state of the Python-held instance rather than
  cloning it.
- Wrap class-valued returns in the stable generated `#[pyclass]` wrapper before they cross back to
  Python.
- Extend unit, bridge, decorator, and end-to-end differential coverage, then remove the
  `class_valued_signatures` boundary exclusion, its one-fixture guard, the fixture's exclusion note,
  and the temporary narrowing recorded by the differential-fixture change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ir-lowering`: Direct class annotations in free-function signatures resolve across the complete
  unit, while unknown and nested class-valued boundary annotations fail with located diagnostics.
- `rust-backend`: Direct instance parameters are emitted as shared or mutable borrows so calls do
  not copy persistent instance state.
- `python-bindings`: The Python↔Rust bridge accepts direct class-valued parameters through the
  stable wrapper and wraps direct class-valued returns, while refusing nested instance conversion.
- `python-api`: Decorator-time validation defers potentially resolvable class annotations until the
  whole-project build without deferring unrelated subset violations.

## Impact

The implementation will touch Python lowering and diagnostic categorisation, project-level source
assembly, Rust signature/call emission, generated PyO3 bindings, and the manager's narrow deferral
policy. Tests will cover shared and mutable instance parameters, returned instances, marking order,
unknown and nested annotations, generated-crate compilation, and compiled-versus-CPython behavior.
No new runtime dependency or public configuration is introduced. On the intended stacked branch,
the existing `fixture-corpus` requirement is not changed; removing its temporary exclusion restores
the promised whole-corpus boundary coverage.
