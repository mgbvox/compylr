# Notes

Measurements and decisions taken while implementing, recorded because the proposal and design
asked for them by name.

## What the boundary tier costs

**11.8s**, measured on this machine as the module-scoped build in
`python/tests/test_differential.py`, printed by the fixture itself on every run.

That is **one** build for the whole corpus — 18 fixtures compiled into a single unit and a single
extension, as a real project is built. The alternative the design warned about, one build per
fixture, would have been eighteen of these.

The proposal said: *if it is intolerable the tier becomes its own target rather than being
dropped.* Twelve seconds is not intolerable, so it stays inside `make check`, marked `slow` and
toolchain-gated like `test_end_to_end.py`. Revisit if the corpus grows by an order of magnitude.

## A narrowing, recorded

`openspec/specs/fixture-corpus/spec.md` requires the boundary tier to cover the **whole** accepted
corpus. It currently covers all of it **except `class_valued_signatures.py`**, which is named in
`BOUNDARY_EXCLUDED` and guarded by `test_the_exclusion_stays_one_fixture_wide` so the hole cannot
widen quietly.

The reason is a defect this tier found on its first run: the Python bridge has no `Ty::Instance`
handling, so a function whose signature names a class emits bindings that do not compile. The
translation tier covers that fixture in full, so what goes untested is the *conversion* and
nothing else — which is exactly the split the two tiers exist to make visible.

Full detail, reproduction, and the design question the fix has to settle are in `HANDOFF.md` at
the repository root. The follow-up is to be proposed on a branch stacked above this one; when it
lands, the exclusion and this note come out together.

## Two smaller findings, already fixed here

* **Only the declared type decides how a value renders.** `def widen(n: int) -> float: return n`
  answers the integer 3 interpreted and 3.0 translated. Those are the same answer; the tier was
  reporting Python's runtime typing as a compiler difference.
* **Popping `__builtins__` from a fixture's namespace silently breaks annotations.** Python
  evaluates them lazily against the defining module's globals, so without builtins `-> float`
  cannot resolve `float`, every annotation degrades to its own spelling, and the coercion above
  becomes a no-op that looks like it is working.

## `make check` blocked by a pre-existing `doc` failure

`make check` runs `fmt-check lint doc test python`. Every stage passes except `doc`, which fails
on this branch for reasons this change cannot reach: `cargo doc --workspace` compiles three crates
against a `compylr_ir` that predates behavior profiles, while `cargo build --workspace --lib`,
`cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` all succeed
on the same source, and each crate documents fine on its own.

This change modifies no `crates/*/src/` file and no manifest, and `cargo doc --lib` does not build
tests, so it is not reachable from here. Diagnosis and the ruled-out causes are in `HANDOFF.md`.

## What the robustness walk found

Nothing, which is the good outcome and is worth recording as a measurement rather than an
impression.

**8,121 top-level members across 959 files**, including the standard library of the interpreter on
this machine, located by asking it. **Zero panics. Zero diagnostics without a usable source
position. Zero files that failed to parse.** 6.6s.

**76 members lowered — 0.9%.** That number is reported and never asserted: the corpus is whatever
Python the machine has, so a threshold would make the suite fail for reasons unrelated to the
compiler. It is low because the subset requires complete annotations and ordinary Python does not
carry them; what makes it useful is watching it move as the subset grows.

Task 6.5 asked for whatever panics or unlocated errors this found to be fixed. There were none, so
nothing was changed — the frontend already held the property over inputs nobody wrote for it.
