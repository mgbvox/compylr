## 1. Split `lower.rs`, changing nothing

- [ ] 1.1 Split `crates/compylr-frontend-python/src/lower.rs` into `scope.rs`, `signatures.rs`,
      `annotations.rs`, `stmt.rs`, and `expr.rs`, with `lower.rs` keeping the entry points — D5.
      Move code only: no renames, no signature changes, no behavior changes
- [ ] 1.2 Keep inference inside `expr.rs` beside expression lowering. It is fused on purpose and
      separating it would recreate the defect class this change closes
- [ ] 1.3 Confirm `cargo test --workspace` output is identical before and after, and that no
      diagnostic message text changed
- [ ] 1.4 Commit this split **on its own**, with no other edit in it

## 2. The typed expression

- [ ] 2.1 Write tests first in `compylr-ir`: a type is readable from any expression; a comparison
      carries boolean rather than its operands' type; a nested subscript and its base carry different
      types; two bodies differing only in one expression's type fingerprint differently; a unit
      round-trips carrying every type
- [ ] 2.2 Write the test that a form and a type cannot be set independently — the property is that
      the raw form is not constructible from outside the crate
- [ ] 2.3 Change `Expr` to carry a form and a `Ty` — D1. Keep `Literal::Float`'s bit-pattern storage
      so `Eq` and `Hash` still derive
- [ ] 2.4 Add the deriving constructors — D2: one per form, computing the type from operands and
      declared modes, with an explicit-type constructor where the expression cannot know (a call)
- [ ] 2.5 Update `Expr::walk` and `walk_calls` for the new shape
- [ ] 2.6 Move the artifact format to version 5, keeping no reader for version 4 — D7. Test that an
      artifact declaring 4 is refused rather than reinterpreted
- [ ] 2.7 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test -p compylr-ir`; commit

## 3. Lowering emits the type

- [ ] 3.1 Write tests first: an inferred type reaches the expression; an annotation supplies what an
      unresolvable call cannot; neither one is the existing undetermined-binding diagnostic,
      unchanged in **category code** and in message; a promoted integer operand and its promoting
      expression carry different types
- [ ] 3.2 Thread the type through `expr.rs`, so every construction uses a deriving constructor and
      `lower_expr`'s `Option<Ty>` stops being discarded
- [ ] 3.3 Resolve the undetermined case per D1: inference, then the required annotation, then the
      existing diagnostic. Add no type meaning *undetermined*
- [ ] 3.4 Confirm every fixture in `python/fixtures/rejected/` is rejected with the same code and
      message as before — this change alters no diagnostic
- [ ] 3.5 `cargo test --workspace`; commit

## 4. Verification

- [ ] 4.1 Write tests first in `compylr-core`, each as hand-built IR: an addition of two integers
      declaring a string result; an argument whose type does not match its parameter; a return whose
      type is not the declared return type; a name read at a type other than the one it was bound at
- [ ] 4.2 Write the test that the verdict does not depend on the producing frontend, matching the
      existing `the_verdict_does_not_depend_on_the_producing_frontend`
- [ ] 4.3 Implement the check in `crates/compylr-core/src/verify.rs`. Keep it to the invariant the
      backend relies on — D4 — not a full type checker
- [ ] 4.4 Write the pass tests: a folded literal carries the type the operation carried, and a unit
      that verified before the pipeline verifies after it
- [ ] 4.5 Update `crates/compylr-core/src/folding.rs` to preserve the replaced expression's type
- [ ] 4.6 `cargo test --workspace`; commit

## 5. Hand-built IR across the suite

- [ ] 5.1 Update `crates/compylr-host-python/tests/conformance.rs` to the deriving constructors, and
      confirm its completeness checks — `ir_variants()`, `(form, position)`, both stances of every
      axis — still pass without being edited. If one needed editing, record why in this change's notes
- [ ] 5.2 Update `execution.rs`, `passes.rs`, and the remaining hand-built IR
- [ ] 5.3 Regenerate the insta snapshots that carry fingerprints, and confirm the only changes are
      fingerprints — a changed emitted shape at this point is a defect
- [ ] 5.4 Run the differential corpus from `add-differential-fixture-testing` and confirm **every**
      fixture still agrees with CPython at both tiers. Nothing else in this change is worth anything
      if this does not hold
- [ ] 5.5 `cargo test --workspace`, `make check`; commit

## 6. Type-directed emission: length

- [ ] 6.1 Write tests first: the length of a collection emits the target's direct length operation;
      the length of a string still honors its declared text units; the result is unchanged for both
- [ ] 6.2 Change `Expr::Len` emission in `crates/compylr-backend-rust/src/rust.rs` to read the
      operand's type instead of dispatching under every mode
- [ ] 6.3 Do the same in `crates/compylr-backend-python/`
- [ ] 6.4 `rm -rf .compylr demo/.compylr`, then `make demo` and record the before/after numbers in
      this change's notes — D6
- [ ] 6.5 `cargo test --workspace`; commit

## 7. Type-directed emission: comparisons and copies

- [ ] 7.1 Write the test first: an arithmetic operation appearing as an operand of a comparison is
      emitted from its own operands' types, and the type-agnostic form is gone
- [ ] 7.2 Delete the third emission path for unchecked arithmetic — the one that exists only because
      the context type under a comparison is `Ty::Unit`. Confirm `native_emission.rs` still passes
- [ ] 7.3 Write the test first for copies: a bound name whose type needs no copy is not copied, and a
      name that is read again after being consumed still is
- [ ] 7.4 Change `Expr::Name` emission to read the expression's own type rather than the context type
- [ ] 7.5 Confirm `a_text_parameter_is_usable_in_every_position` still passes **unchanged**. Do not
      touch it, and do not start on borrowed parameters — that is a separate change
- [ ] 7.6 `rm -rf .compylr demo/.compylr`, `make demo`, record the numbers
- [ ] 7.7 `cargo test --workspace`, `make check`; commit

## 8. Close out

- [ ] 8.1 Update `README.md`: the artifact format is at version 5, and the upgrade note names the
      reason. `tests/readme.rs` enforces the mechanical half — make it pass
- [ ] 8.2 Update `CLAUDE.md` **and its identical `AGENTS.md` copy**: expressions carry their type;
      the backend now asks rather than infers;
      the format version and what it invalidates; `lower.rs` is a directory
- [ ] 8.3 Record in this change's notes what the three removed workarounds were worth, measured, and
      what borrowed parameters would need beyond what this change delivered — that is the input to
      the change that attempts them
- [ ] 8.4 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, `make check`, `make demo`; commit
