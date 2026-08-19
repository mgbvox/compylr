## Why

Nothing in this repository shows a person what compylr is for. There are fixtures, which are
one-function files chosen to exercise a rule, and there is a README snippet. Neither is a project
you could copy.

There is also no executable check that the pieces compose. Every capability is tested in isolation;
a demo that compiles and runs is the only thing that fails when they interact badly — and
interaction is where a compiler with control flow, mutation, classes, and a build pipeline is most
likely to be wrong.

## What Changes

- Add **`./demo/`**, a complete uv project depending on compylr, with its own `pyproject.toml`,
  package, and tests. It is a project you could copy, not a snippet.
- Implement **three nth-prime functions that each fully compile**, chosen because they exercise
  different parts of the subset while producing identical answers:
  - **recursive** — self-recursion with a base case; needs branching and calls,
  - **iterative** — loops, reassignment, and a locally built collection,
  - **memoized** — a class holding a mutable cache, needing membership, insertion, and state that
    outlives a call.
- Assert **all three agree**, for every n over a range, and agree with a plain interpreted
  reference. Three implementations that compile but disagree is the failure worth catching.
- Assert **each is genuinely compiled** — not silently falling back to the interpreter, which would
  make the whole demo prove nothing.
- Demonstrate **precompiling**: the demo's documented flow is `compylr compyle ./demo` and then run,
  and its README shows the timing difference.
- The demo SHALL be **verified by the repository's own test suite**, so it cannot rot. A demo that
  stops compiling and nobody notices is worse than no demo.

Explicitly **not** in this change: benchmarking against CPython, packaging the demo for
distribution, or any compiler feature — if the demo needs something the compiler lacks, that is a
finding, and it belongs in a change of its own rather than being smuggled in here.

## Capabilities

### New Capabilities

- `demo`: the demonstration project — what it must contain, that it must compile, that its
  implementations must agree, and that its correctness is checked by this repository rather than
  asserted in prose.

### Modified Capabilities

None. The demo consumes the compiler; it does not change it.

## Impact

- **This is the first executable claim that the features compose.** Control flow, mutation, classes,
  and precompiling are each tested alone. The recursive variant needs branching *and* recursion; the
  iterative needs loops *and* reassignment *and* a built collection; the memoized needs a class
  *and* a mutable attribute *and* membership. If any pairing is wrong, this is where it shows.
- **A rotting demo is a liability.** One that no longer compiles, while the README says it does, is
  worse than none — so the repository's suite builds and runs it, and that check has to be cheap
  enough to keep. It compiles a Rust crate, so it belongs with the other slow tests rather than in
  the fast path.
- **It depends on all four preceding changes.** The recursive variant needs `add-control-flow`; the
  iterative needs control flow and `add-collection-mutation`; the memoized needs those and
  `add-classes`; the documented flow needs `add-cli-precompile`. It cannot be written first, and
  writing it last is what makes it a check rather than a wish.
- **It will find things.** A demo is the first program written to be *useful* rather than to
  exercise a rule, and the gaps it exposes are the ones a user would hit first. Those findings are
  the point; each becomes its own change rather than being absorbed here.
- **Where it lives**: `./demo/` at the repository root, as requested — a sibling of `python/`, not
  inside it, so it is plainly a consumer of the package rather than part of it.
