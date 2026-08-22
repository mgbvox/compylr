## Why

Three of the demo's fifteen workloads run **slower compiled than interpreted**: `text.word_count`
at 0.3x, `graphs.bfs_distances` at 0.7x, `text.joined` at 0.9x. That is the product's central claim
failing on ordinary Python, and it has been true in every benchmark run this repository has
recorded — the demo reports it faithfully and nothing acts on it.

They are not outliers, and they are not one bug. Each is a different systemic cost in the generated
code, and every one is invisible to every correctness test in the repository, because all of them
produce exactly the right answer.

The costs were measured, not inferred. Each candidate below was applied by hand to
`demo/.compylr/crate/` and rebuilt, so the numbers are what that change actually bought rather than
what it ought to buy:

| | change | measured |
| --- | --- | ---: |
| 1 | a `[profile.release]` in the generated manifest | 10–25% across the board |
| 2 | in-place string append instead of rebuild-per-`+` | 4.1x on `text.joined` |
| 3 | a non-cryptographic hasher for generated maps | 1.93x on `graphs.bfs_distances` |
| 4 | iterating a collection by reference | 1.49x on `text.total_length` |
| 5 | moving rather than cloning a returned local | 25 of 25 sites in the demo |
| 6 | the per-call boundary conversion | `binary_search` is **16x slower** compiled |
| 7 | double bounds checks, O(n) `len`, triple map lookups | 2.7x on `word_count`'s body |

Items 1–5 and 7 together take `text.joined` from 0.9x to 3.8x and `graphs.bfs_distances` from 0.7x
to 1.4x — two of the three losing workloads become wins.

**Every one of these is semantics-preserving.** None changes an answer, none is opt-in, and none
needs a user to ask for it. That is what separates this change from `add-behavior-profiles`, which
is about letting a user choose a *different meaning* and whose performance benefit is measurably
the smaller lever. Splitting them keeps a one-line 4.1x fix from shipping behind an IR format bump,
and keeps the question "can this change my program's answer?" answerable per item — yes by design
there, no by construction here.

**Why now.** Item 1 is one line and is the single cheapest thing in the repository. Item 6 is a
design question whose answer constrains the bridge, and the bridge is easier to change before more
users depend on how values cross it. And `add-behavior-profiles` needs the benchmark harness to
resolve differences it currently cannot (item 0 below), so that work is a shared prerequisite.

## What Changes

- **A measurement floor, first.** The benchmark reports a single best-of-five with no spread, and
  `sorting.merge_sort` varies from 160us to 277us across *byte-identical* builds. Every claim below
  is unreadable until the harness reports spread and names its noise floor. This is a prerequisite,
  not a nicety, and it is shared with `add-behavior-profiles`.

- **The generated crate gains an explicit release profile.** `cargo_manifest` emits `[workspace]`,
  `[package]`, `[lib]` and `[dependencies]` and stops, so every user's artifact builds at
  `codegen-units = 16`, `lto = false`. Because the runtime helpers live in a different module from
  their callers, they frequently are not inlined — and every arithmetic operation in the subset is
  a trait call by design. `-C target-cpu=native` was tested and is **rejected**: no row moved
  outside noise, and it would make a copied `.compylr/` fault on another machine.

- **Emission stops making copies it does not need.** An accumulator that reads itself
  (`x = x + y`) updates in place; a loop variable that is only read is borrowed rather than cloned;
  a local returned in tail position is moved rather than deep-copied. Each is a local rule with an
  observable-behavior obligation attached.

- **The hasher for generated maps and sets becomes a choice.** Today it is not one: the runtime's
  impls read `impl<K, V> ... for HashMap<K, V>`, silently pinning `RandomState` across ten trait
  impls. Making them generic over `S` is worth doing whatever hasher wins, and it is what lets a
  non-cryptographic default be selected rather than assumed. A hasher is a **performance** choice
  with no observable semantics — deliberately not a behavior axis.

- **The boundary's per-element cost is confronted.** A collection parameter converts element by
  element on every call: ~4 ns for a `list[int]` element, ~42 ns for a `list[str]` element, ~10 ns
  to return one. A read-only `str` parameter does not need an owned `String`, and the subset
  already guarantees parameters are never mutated — the compiler knows, and the bridge does not use
  it. **This is the largest lever and the one with real design risk**, so it is staged behind the
  rest.

- **The runtime sweep**: `py_index` bounds-checks twice; `py_str_len` decodes the whole string on
  every `len()` under `CodePoints`; `d[k] = d[k] + 1` performs three hash lookups where one would
  do; nothing uses `with_capacity`.

- **A regression guard.** These defects were all invisible to correctness tests. The demo is the
  only instrument that sees them, so the repository gains a check that a known-good speedup does
  not silently regress.

## Capabilities

### New Capabilities

- `generated-code-performance`: what it means for a change to generated code to be an optimization
  — that it preserves observable behavior, that its benefit is measured rather than argued, that a
  claim the harness cannot resolve is not made, and that the demo is the instrument of record.

### Modified Capabilities

- `build-pipeline`: the generated crate is built under an explicit, recorded release profile rather
  than inheriting Cargo's defaults.
- `rust-backend`: emission avoids avoidable copies (in-place accumulation, borrowed iteration,
  moved returns); the runtime's hashed containers are parameterised over their hasher rather than
  pinned to the standard one; the runtime's indexing and length helpers do not repeat work.
- `python-bindings`: a read-only string parameter crosses the boundary without being copied, and
  the per-element cost of crossing is a stated property rather than an accident.
- `demo`: the benchmark reports run-to-run spread and a noise floor, and the repository guards
  against a performance regression in the generated code.

## Impact

**Rust.** `compylr-bridge-python-rust` (`bindings.rs` — `cargo_manifest` gains a release profile;
the generated PyO3 signatures for read-only string parameters). `compylr-backend-rust` (`rust.rs` —
the accumulator peephole, borrowed loop variables, tail-position moves, container literals through
`FromIterator`; `runtime.rs` — the hasher parameter and its self-contained implementation, blanket
impls over `&T`, `py_index`, `py_str_len`).

**Python.** None expected. No API surface changes; no new keyword, no new setting.

**Caches.** None of these change the IR, so `Unit::fingerprint()` does not move and **no cache is
invalidated by correctness**. That is exactly the standing hazard in CLAUDE.md: the rebuild key is
the IR fingerprint, so editing the backend does *not* invalidate a cached build. Every measurement
in this change requires `rm -rf .compylr demo/.compylr` first, or it measures the previous build.

**Docs and tests.** `demo/README.md` (the boundary's per-element cost, which is currently stated
nowhere and which the README's framing implies away). `tests/emit_quality.rs` and
`tests/conformance.rs` for the emission rules. `CLAUDE.md` for the hasher decision and the
boundary's cost.

**Interaction with `add-behavior-profiles`.** No file conflict is expected except
`cargo_manifest` and the demo benchmark. That change's overflow mitigation wants
`overflow-checks = false` in a release profile section this change creates; whichever lands first,
the other should verify rather than duplicate. Its goal of a byte-identical default path must be
diffed against a snapshot taken *after* this change, since this change moves emitted source for
semantics-preserving reasons.

**Not in scope.** No change to the accepted Python subset, no new diagnostics, and no user-facing
setting — if a user has to ask for it, it belongs in `add-behavior-profiles` instead. No
`unchecked-arithmetic`: removing the overflow check changes what a program means, which is that
change's domain and not this one's. No algorithmic rewriting of user code.
