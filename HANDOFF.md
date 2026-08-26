# Handoff

Historical record of the defect that the differential fixture change surfaced. The
`support-class-valued-signatures` change resolves it with borrowed direct parameters, owned
wrapper returns, located escape diagnostics, and decorator deferral. The evidence and rejected
alternatives remain below so the ownership decision is not rediscovered.

---

## Resolved: the Python bridge could not express a class-valued signature

**Resolution:** `support-class-valued-signatures`, stacked above
`apply/add-differential-fixture-testing`.

### Original defect

A free function whose signature names a class generates PyO3 bindings that do not compile.

`crates/compylr-bridge-python-rust/src/bindings.rs` spells every boundary type with the *backend's*
`rust_ty` (lines 92, 110, 183, 211, 233). That is correct inside generated code, where
`Ty::Instance("Tally")` is the struct `Tally`. It is wrong at the Python boundary, where the bridge
has wrapped that struct in a separate `#[pyclass]` type:

```rust
#[pyclass(name = "Tally")]
pub struct __compylr_class_0 { inner: generated::Tally }

#[pyfunction]
#[pyo3(name = "build")]
fn __compylr_export_8(start: i64) -> PyResult<Tally> {      // the INNER struct
    generated::build(start).map_err(__compylr_to_py_err)
}
```

`generated::Tally` is not a `#[pyclass]`, so PyO3 cannot convert it and the crate fails with
`error[E0034]: multiple applicable items in scope` — candidates
`IntoPyObjectConverter<Result<T, E>>` and `IntoPyObjectConverter<T>`. The same applies to a class in
*parameter* position (`read(t: Tally)`).

`grep -n "Instance" crates/compylr-bridge-python-rust/src/*.rs` returns nothing: the bridge has no
`Ty::Instance` handling at all.

### How to see it

```bash
python - <<'PY'
from pathlib import Path
from compylr import _core
from compylr._build import BuildPipeline
from compylr._config import Behavior

source = Path("python/fixtures/accepted/class_valued_signatures.py").read_text()
unit = _core.compile_unit([(source, Behavior.from_language("python").to_core())], "rust")
BuildPipeline(Path("/tmp/bridge-defect/.compylr")).build(unit)   # raises BuildError
PY
```

`python/fixtures/accepted/class_valued_signatures.py` exists precisely to hold this shape, and its
header says why it is kept apart from `classes.py`.

### Why nothing caught it

Two independent gaps hid it, and the differential corpus hit both on its first run:

1. **The decorator never reaches the bridge with this shape.** `Manager.compyle` captures one
   member's source and validates it alone (`python/compylr/_manager.py:241`), so
   `def build(start: int) -> Tally` is rejected at `crates/compylr-frontend-python/src/lower.rs:999`
   — `'Tally' is not a supported type annotation` — before any unit that would resolve it is
   assembled. The manager already defers one such category (an unresolved callee,
   `_DEFERRED_UNTIL_BUILD`); a class-typed annotation is not in it.
2. **Nothing else built one.** The demo's `PrimeCache` is a marked class, but nothing in the demo
   annotates a function with it, and `python/tests/test_end_to_end.py` marks only free functions
   over scalars and collections.

So the subset accepts the shape end to end in the IR and the Rust backend — `tests/differential.rs`
runs it and it agrees with CPython — and it is unreachable from Python.

### What the fix has to decide

- **Return position.** Wrap on the way out: `Ok(__compylr_class_N { inner: generated::build(start)? })`.
  The wrapper's identity is generated (`__compylr_class_N`), so the bridge needs a stable map from
  class name to wrapper ident — it already builds one to emit the classes.
- **Parameter position.** Take the wrapper (`PyRef<'_, __compylr_class_N>`) and hand the inner value
  to the generated function. That means a **clone** of the inner value, which is a semantic
  decision worth stating: `CLAUDE.md` says an instance is *not* converted — the Python object holds
  the Rust value and a method borrows it from there, which is what makes a mutated attribute
  survive. A cloned instance parameter would break that promise, so a function taking a class may
  need to borrow rather than own, or the promise needs qualifying for this position.
  **This is the real design question; the wrapping is mechanical.**
- **Nested positions.** `list[Tally]`, `dict[str, Tally]`. Either support them or reject them with a
  located diagnostic rather than emitting code that does not compile.
- **The decorator gap (1) above** is a separate decision: whether an unknown annotation defers to
  build time the way an unresolved callee does, and how that stays distinguishable from a typo.
  A proposal could take both or leave the decorator to a third.

### Cleanup completed with the fix

- `BOUNDARY_EXCLUDED` in `python/tests/test_differential.py`, and the test
  `test_the_exclusion_stays_one_fixture_wide` that keeps the hole one fixture wide.
- The exclusion note in the header of `python/fixtures/accepted/class_valued_signatures.py`.
- The narrowing recorded in this change's `notes.md`, once the boundary tier covers the whole
  accepted corpus as `openspec/specs/fixture-corpus/spec.md` requires.
