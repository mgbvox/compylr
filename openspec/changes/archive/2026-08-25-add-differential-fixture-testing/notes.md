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

## Two smaller findings, already fixed here

* **Only the declared type decides how a value renders.** `def widen(n: int) -> float: return n`
  answers the integer 3 interpreted and 3.0 translated. Those are the same answer; the tier was
  reporting Python's runtime typing as a compiler difference.
* **Popping `__builtins__` from a fixture's namespace silently breaks annotations.** Python
  evaluates them lazily against the defining module's globals, so without builtins `-> float`
  cannot resolve `float`, every annotation degrades to its own spelling, and the coercion above
  becomes a no-op that looks like it is working.

## A `doc` failure that was not real

While implementing, `make check` failed at `doc`: `cargo doc --workspace` reported three crates
compiling against a `compylr_ir` that predated behavior profiles, while `cargo build`, `cargo
test`, and `cargo clippy` all passed on the same source and each crate documented fine alone.

It was a **stale local build cache**, left by the rebase that put this branch on current `main`.
`cargo clean --doc` does not clear it; cleaning the four packages does, after which
`cargo doc --workspace` passes. CI never saw it -- `rustdoc` was green on this branch throughout,
which is what prompted looking again.

Recorded because two commit messages in this branch describe it as a pre-existing branch failure.
It was not. The trap is now noted in `CLAUDE.md`.

## The demo did not move

`make demo` reports **"Both modes returned the same answer for every workload"** and all three
behavior modes still answer 118 for `arithmetic.collatz_length(97)`.

That was the check worth running: this change alters no compiler code, so any movement in a demo
answer would have been a defect in the change itself rather than a measurement.
