## 0. Prerequisite

- [ ] 0.1 Confirm `add-typed-ir-expressions` is complete and archived. **Do not start before it.**
      Deciding whether an argument may be borrowed requires an expression's type, which the backend
      does not know until that change lands.
- [ ] 0.2 Run `a_text_parameter_is_usable_in_every_position` and record that it passes, as the
      before-state of the gate.

## 1. The mode in the IR

- [ ] 1.1 Write the failing tests: a parameter carries a mode; the mode round-trips; two units
      differing only in a mode fingerprint differently.
- [ ] 1.2 Add the passing mode — owned, shared borrow, mutable borrow — to `Param`.
- [ ] 1.3 Default every constructor to owned, so a hand-built unit is conservative by construction.
- [ ] 1.4 Advance `ARTIFACT_VERSION` and extend the fingerprint over the mode.
- [ ] 1.5 Confirm the mode carries no target spelling.

## 2. The analysis

- [ ] 2.1 Write the failing tests first, one per forcing shape: returned, appended, stored as a
      mapping value, stored in an attribute, ordering-compared, membership-tested, passed to an
      owning callee, passed to an unseen callee.
- [ ] 2.2 Write the failing tests for the borrowing cases: read-only, read several times, forwarded
      to a compatible borrow.
- [ ] 2.3 Extend the existing mutability fixpoint to decide ownership in the same pass; do not add a
      second analysis.
- [ ] 2.4 Make every parameter start owned and move toward a borrow only when proven, so the lattice
      is monotone and termination is structural.
- [ ] 2.5 Cover mutual recursion, and assert the result is independent of analysis order.
- [ ] 2.6 Assert every existing receiver-mutability conclusion in the corpus is unchanged.
- [ ] 2.7 Assert no new diagnostic is produced for any program, accepted or rejected.

## 3. Emission

- [ ] 3.1 Write the failing tests: owned emits owned, shared emits a shared reference, mutable emits
      a mutable reference, and a never-mutated owned parameter still emits owned.
- [ ] 3.2 Emit signatures from the mode, never from the type or from whether the parameter is
      mutated.
- [ ] 3.3 Remove the clone on reading a borrowed parameter.
- [ ] 3.4 Build every accepted fixture and confirm the generated crates compile — especially the
      four forcing shapes.
- [ ] 3.5 Confirm emission stays byte-reproducible.

## 4. The boundary

- [ ] 4.1 Write the failing tests: a borrowed text argument is not copied; an owned argument
      converts as before; a borrow does not outlive the call.
- [ ] 4.2 Convert each argument by its mode, borrowing the host's buffer for text where possible.
- [ ] 4.3 Confirm a collection argument is still converted element by element under every mode, and
      state that in the code where a reader would otherwise assume the borrow made it free.
- [ ] 4.4 Confirm every driver's answers are identical before and after.

## 5. The gate

- [ ] 5.1 Run `a_text_parameter_is_usable_in_every_position` **unchanged**. If it needs modifying,
      stop: the design is wrong.
- [ ] 5.2 Add a case per forcing shape asserting the parameter's mode is owned, inspecting the unit
      rather than emitted text.
- [ ] 5.3 Add the coverage check that fails when a shape's case asserts only the answer and not the
      mode.
- [ ] 5.4 Add the check that fails when a previously owned shape becomes borrowed, so such a change
      is deliberate.

## 6. Measurement

- [ ] 6.1 Remove `.compylr` and `demo/.compylr` before measuring anything.
- [ ] 6.2 Measure the per-element text conversion cost borrowed against owned, and record it.
- [ ] 6.3 Confirm no algorithm in the demo regresses beyond noise.
- [ ] 6.4 Confirm forwarding a collection between compiled functions no longer clones.
- [ ] 6.5 If the text saving does not materialise, record the finding before `add-numpy-arrays`
      begins, since that change depends on this mechanism.

## 7. Documentation and checks

- [ ] 7.1 Rewrite the CLAUDE.md note about the reverted `&str` work so it states the rule — a
      parameter is owned when it escapes — rather than warning against trying.
- [ ] 7.2 Update README prose covering how arguments cross, including that collections still convert
      element by element.
- [ ] 7.3 Regenerate the README benchmark tables with `scripts/update_benchmarks.py`; never hand-edit.
- [ ] 7.4 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
      `cargo test --workspace`.
- [ ] 7.5 Run `make check`.
