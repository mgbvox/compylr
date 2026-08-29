## 1. The registry and the IR form

- [ ] 1.1 Write the failing tests first: an intrinsic resolves without a matching unit member; a
      user function of the same name does not shadow it; resolution does not vary with compilation
      order.
- [ ] 1.2 Add the intrinsic signature registry to [`compylr-ir`](../../../crates/compylr-ir/src/lib.rs),
      keyed by module and operation, carrying parameter types, result type, and whether the
      operation is fallible.
- [ ] 1.3 Populate the `math` entries: `sqrt`, `floor`, `ceil`, `fabs`, `exp`, `log`, `log2`,
      `log10`, `pow`, `sin`, `cos`, `tan`, `atan2`, `hypot`, `isnan`, `isinf`, `isfinite`, `trunc`,
      and the constants `pi`, `e`, `tau`, `inf`, `nan`.
- [ ] 1.4 Add the namespaced intrinsic form to [`Expr`](../../../crates/compylr-ir/src/ir.rs#L441),
      carrying module, operation, arguments, and an optional checking mode.
- [ ] 1.5 Advance [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) from 4 to 5 and
      confirm no reader for the previous version is kept.
- [ ] 1.6 Extend [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) over the new
      form; assert two units differing only in module, in operation, or in checking mode fingerprint
      differently.
- [ ] 1.7 Round-trip an intrinsic through the artifact and assert the recovered unit matches.
- [ ] 1.8 Confirm `cargo test -p compylr-ir` passes and
      [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs) still
      holds — the registry must not have pulled a dependency into `compylr-ir`.

## 2. Imports and namespaces in the frontend

- [ ] 2.1 Write the failing tests: a supported import is accepted; an unsupported one lists the
      supported modules; a from-import names the supported form.
- [ ] 2.2 Replace the blanket import rejection in
      [`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs#L585) with resolution
      against the registry, keeping the rejection for unsupported modules and for from-imports.
- [ ] 2.3 Bind imported module names — and aliases — into a per-source namespace scope, and confirm
      an alias does not leak the original name.
- [ ] 2.4 Confirm a module namespace does not cross sources within a unit.
- [ ] 2.5 Reject a module name in every non-receiver position with the "a module is not a value"
      diagnostic: bound, passed, returned, stored, compared.
- [ ] 2.6 Lower an attribute access on a module namespace to the intrinsic form, and report an
      attribute the registry does not list as a located diagnostic naming module and attribute.

## 3. Typing an intrinsic call

- [ ] 3.1 Write the failing tests: wrong arity, wrong argument type, inferred binding type, integer
      promotion, and a result that violates a declared return type.
- [ ] 3.2 Type-check arguments against the registry signature and report a mismatch with the
      operation, expected type, and supplied type.
- [ ] 3.3 Apply the existing numeric promotion so an integer argument to a float operation is
      widened by the same path everything else uses.
- [ ] 3.4 Make an intrinsic result determine an unannotated binding's type.
- [ ] 3.5 Take the checking mode for a fallible intrinsic from the resolved behavior, never from the
      operation name.
- [ ] 3.6 Add the frontend spellings so a diagnostic quotes the module and operation the way the
      user wrote them, in [`spelling.rs`](../../../crates/compylr-frontend-python/src/spelling.rs#L16)
      and not in the IR.

## 4. Verification and folding

- [ ] 4.1 Make [`Unit::validate`](../../../crates/compylr-ir/src/ir.rs#L1384) resolve intrinsics
      against the registry and leave `Expr::Call` resolution unchanged.
- [ ] 4.2 Confirm constant folding in [`folding.rs`](../../../crates/compylr-core/src/folding.rs)
      either folds an intrinsic over literal arguments or leaves it untouched — and that whichever
      it does, it preserves the checking mode.
- [ ] 4.3 Assert the fingerprint is still taken before the optimization passes.

## 5. Rust emission

- [ ] 5.1 Write the failing tests: an operation emits an inherent method, a constant emits a target
      constant, an unchecked intrinsic emits no domain test, and a checked one returns an error.
- [ ] 5.2 Add the `math` emission table in [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs),
      mapping each operation onto its `f64` method or `std::f64::consts` constant.
- [ ] 5.3 Emit the domain test for a reported checking mode, returning the same recoverable error
      type checked arithmetic uses, carrying the operation's name.
- [ ] 5.4 Assert the generated signature is identical under both modes.
- [ ] 5.5 Extend [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs) so an
      intrinsic is covered in every position it is legal in — free function body, method body,
      constructor body, and loop body.
- [ ] 5.6 Confirm emission is byte-reproducible for a unit containing intrinsics.

## 6. Go reserved

- [ ] 6.1 Write the failing test: a program using `math` compiled for Go fails reporting the mapping
      as planned.
- [ ] 6.2 Add the refusal on the (module, backend) pair in
      [`golang.rs`](../../../crates/compylr-backend-golang/src/golang.rs), distinct from the backend
      being unknown or unimplemented.
- [ ] 6.3 Confirm a Go compilation of a program using no module still succeeds unchanged.

## 7. Corpus

- [ ] 7.1 Move the proposal's worked example into
      [`accepted/`](../../../frontends/python/fixtures/accepted/) as `math_module.py`, extended to
      exercise every registry operation, with member names unique across the whole accepted corpus.
- [ ] 7.2 Add its driver in [`drivers/`](../../../frontends/python/fixtures/drivers/), naming calls
      as literal data and carrying no expected values.
- [ ] 7.3 Compare floating-point answers within a stated tolerance and non-finite answers by
      classification.
- [ ] 7.4 Add rejected fixtures in [`rejected/`](../../../frontends/python/fixtures/rejected/):
      unsupported module, unsupported operation of a supported module, from-import, module as a
      value, and exponentiation still refused.
- [ ] 7.5 Add the derived coverage check that fails when a registry operation has no fixture calling
      it — derived from the registry, never a hardcoded list.

## 8. Bridge and host

- [ ] 8.1 Confirm a checked domain failure crosses
      [`compylr-bridge-python-rust`](../../../crates/compylr-bridge-python-rust/src/lib.rs) as an
      exception, not an abort.
- [ ] 8.2 Confirm the generated crate needs no new dependency for `math`.

## 9. Demo coverage

- [ ] 9.1 Add an algorithm to [`demo/demo-python-rust/src/algorithms/`](../../../demo/demo-python-rust/src/algorithms/)
      that uses `math`, so the added `Expr` form is covered rather than the claim narrowed.
- [ ] 9.2 Confirm [`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs)
      passes, and that `ir_coverage.py` sees the new form.

## 10. Documentation and checks

- [ ] 10.1 Regenerate the README subset matrix with
      [`update_subset.py`](../../../scripts/update_subset.py); confirm `--check` passes, and never
      hand-edit the block.
- [ ] 10.2 Update the README prose half: imports, the supported module list, and the
      module-is-not-a-value rule.
- [ ] 10.3 Update [`CLAUDE.md`](../../../CLAUDE.md): the new artifact version, the intrinsic form and
      why it is not a call, the from-import refusal, and the Go reservation.
- [ ] 10.4 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
      `cargo test --workspace`.
- [ ] 10.5 Run `make check` and confirm CI, the Makefile, and the hooks all still agree.
- [ ] 10.6 Remove `.compylr/` and `demo/demo-python-rust/.compylr/` before any measurement, then run
      `make demo` and confirm no regression on programs that use no module.
