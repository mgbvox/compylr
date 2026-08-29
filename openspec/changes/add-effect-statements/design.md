## Context

See proposal.md — Why. Four existing facts constrain the approach:

* `Stmt::Effect` means "a unit-returning method call", and its documentation gives the reason. It
  is not a general effect node and must not quietly become one.
* `crate_boundaries.rs` forbids `compylr-backend-rust` from naming Python. Anything about the
  *host's* stream is therefore a bridge concern, not a backend one.
* Mapping and set iteration order is deliberately unspecified, and CLAUDE.md instructs that no test
  assert on it.
* `tests/conformance.rs` checks `(form, position)` coverage, because a statement's emission depends
  on where it is. A new statement form owes four positions.

## Goals / Non-Goals

**Goals:**
- A statement form meaning "an effect on the outside world", distinct from mutation of owned state.
- Compiled output byte-identical to interpreted output, proven by capturing both.
- Correct interleaving with host output, including under redirection.

**Non-Goals:**
- `sep`, `end`, `file`, `flush`. They are keyword arguments and the subset already rejects those.
- Reading input. Output is one direction and one direction is the change.
- Formatted output, f-strings, or `str()` as an expression. A value's text becomes a *value* there,
  which is a different feature with its own type consequences.
- Printing mappings, sets, or instances. Refused, with reasons in the spec.

## Decisions

### Rendering is a declared convention, not the target's default

This is the decision the change turns on. Python prints `True`, Rust prints `true`. Python prints
`1.0` for a whole float; Rust's `{}` prints `1`. Emitting the target's default would make compiled
output differ from interpreted output — and `print` is exactly what a user reaches for when
something is *already* wrong, so a divergence here corrupts the tool they are debugging with.

The convention rides on the operation as a mode, the way `TextUnits` rides on `Expr::Len`, and a
backend matches on the mode. This is the IR's stated rule — "IR operations carry the semantics the
resolved behavior declared, not one language's by default" — applied to output.

*Alternative considered: a seventh behavior axis.* Rejected for the reason
`add-intrinsic-calls` rejected one: an axis costs a field on `LanguageBehavior` and a declared
stance from every language, and nobody wants Python's boolean spelling with Go's float spelling.

*Alternative considered: always render as the source language.* Nearly right, and it is the
default. It is expressed as a mode anyway so a target-native rendering is expressible later without
reopening the IR — the same reason `unchecked-arithmetic` exists as a declined option rather than
as nothing.

### Output goes through a runtime sink the host installs

Rust's `std::io::stdout()` and Python's `sys.stdout` hold **separate buffers**. Line-buffered to a
terminal the difference is usually invisible; block-buffered to a pipe or a file the two orders
scramble outright. Worse, `contextlib.redirect_stdout`, pytest's `capsys`, and notebook capture all
work by replacing `sys.stdout`, and none of them can see a write the Rust side made directly.

So the backend emits a call to a sink in the generated runtime, and the bridge installs one that
writes through the host's stream. Backend emits target code; bridge knows the host. That is the
existing division of labour, not a new one.

*Alternative considered: write to Rust's stdout and flush aggressively.* Flushing fixes neither
redirection nor capture, because the bytes never pass through `sys.stdout` at all.

*Alternative considered: the backend emits PyO3 calls directly.* Forbidden — the backend cannot
name Python, and `crate_boundaries.rs` fails the build if it does.

A default sink writing to the target's own stdout is kept, so a crate built outside a host still
prints. Without it, `cargo run` on generated code would be silent, which reads as a bug.

### Printing a mapping or a set is refused

Their iteration order is unspecified by deliberate choice. A differential test on their printed
form would therefore be **flaky rather than correct** — it would pass locally, fail in CI, and
implicate the compiler in a divergence the language never promised to avoid.

Refusing is the honest answer, and the diagnostic points at an ordered projection so the user has a
way through. This falls straight out of an invariant already recorded in three places; the change
adds no new rule, it declines to break one.

### The carve-out is in the registry, not in the lowering condition

`bare_expression_error` currently special-cases `append` and unit-returning method calls by shape.
Adding `print` by shape would make a third hand-written exception. Instead lowering asks the
registry whether the operation is effectful — so the reason the rejection exists ("its value is
discarded") is tested directly, and every later effectful operation, `logging` included, is covered
without touching this code again.

### Rendering a sequence writes into one buffer

Rendering element-by-element into freshly allocated strings makes printing a sequence quadratic in
allocator traffic. The demo is where cost shows up, and it has already found a quadratic clone in
`for` and an O(n) clone per nested read — both invisible to every correctness test. A linear
rendering is specified so the third one is not found the same way.

## Risks / Trade-offs

**A print costs a boundary crossing** → Accepted deliberately. Correct interleaving is the whole
reason to have the construct, and a fast print in the wrong order is worse than no print. Printing
inside a hot loop is slow; that is true interpreted as well.

**Float rendering may not match CPython in every case** → CPython uses shortest round-tripping
repr; Rust's `{:?}` is close but diverges on exponent formatting. The runtime renderer implements
the source convention explicitly and the corpus compares captured text, so a divergence fails a
test rather than reaching a user. Cases found are fixed in the renderer, not papered over in the
comparison.

**Holding the host's lock while compiled code runs could deadlock** → The sink acquires the host
stream per write and releases it, and never holds it across a call back into user code. Compiled
code cannot call back into the host anyway, which bounds this.

**The artifact version collides with the other in-flight changes** → Same coordination as
`add-intrinsic-calls`; whichever lands first takes the number.

**A future `str()` expression would need the same renderer** → Deliberately shaped for it: the
renderers are runtime functions selected by convention, so making text a value later reuses them
rather than growing a second implementation that could disagree.

## Migration Plan

The artifact version advances; caches are refused once and rebuilt automatically off the recorded
compylr version. Programs that print nothing emit byte-identical code, so the rebuild is the only
observable effect for existing projects.

Rollback is removing the change. Generated crates from it are self-contained, and the sink has a
default, so an already-built artifact keeps working until it is rebuilt.

## Open Questions

- Whether the sink should be line-buffered on the Rust side before handing bytes to the host, to
  reduce crossings when printing in a loop. A measurable optimization that changes neither the
  ordering guarantee nor any spec, so it is decided after the demo can measure it.
