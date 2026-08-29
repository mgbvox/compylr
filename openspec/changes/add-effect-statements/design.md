## Context

See proposal.md — Why. Four existing facts constrain the approach:

* [`Stmt::Effect`](../../../crates/compylr-ir/src/ir.rs#L761) means "a unit-returning method call",
  and its documentation gives the reason. It is not a general effect node and must not quietly
  become one.
* [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs) forbids
  `compylr-backend-rust` from naming Python. Anything about the *host's* stream is therefore a
  bridge concern, not a backend one.
* Mapping and set iteration order is deliberately unspecified, and
  [`CLAUDE.md`](../../../CLAUDE.md) instructs that no test assert on it.
* [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs) checks
  `(form, position)` coverage, because a statement's emission depends on where it is. A new
  statement form owes four positions.

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

### 1. An effect on the outside world is a new `Stmt` form, not a widened `Stmt::Effect`

**Decision.** Add a statement variant beside the existing one rather than redefining it:

```rust
// before — the only effect the subset admits, and its doc says so
Effect(Expr),
// after — an effect that leaves the program, carrying the operation and how values render
Perform {
    module: String,
    operation: String,
    args: Vec<Expr>,
    convention: Convention,
},
```

**Why.** `Stmt::Effect` documents that lowering only ever puts a unit-returning *method* call there,
and the reason is load-bearing: a free function in this subset reaches no mutable state, so
discarding its result is dead code. Widening the variant to mean "any effect" would delete that
reasoning from the type and make the dead-code rejection unenforceable.

**Alternatives considered.** *Reuse `Stmt::Effect` and let the registry decide.* The IR would then
carry a form whose legality depends on a lookup, and a reader of the enum could no longer tell what
is admissible. *An expression form returning unit.* `None` is a return type in this subset, not a
value; an expression of no type has nowhere to go.

#### The IR, in both faces

The definition delta is above. The value, for the worked example's first `print`, as the JSON
`--emit ir` writes. The envelope is real output from the tip of this branch; the `Perform` node is
`expected`:

```json
{
  "version": 5,
  "functions": [
    {
      "name": "describe",
      "params": [{ "name": "label", "ty": "Text" }, { "name": "values", "ty": { "List": "Int" } }],
      "ret": "Float",
      "body": [
        {
          "Perform": {
            "module": "builtins",
            "operation": "print",
            "args": [{ "Name": "label" }, { "Name": "total" }],
            "convention": "Python"
          }
        }
      ]
    }
  ],
  "origin": { "frontend": "python", "requires": ["IntegerOverflowReported", "FloatOrderPreserved"] }
}
```

The five questions:

- **Neutrality.** `Convention` names a *rendering stance*, not a language's formatter. `Python` is
  the name of a convention the way `Rounding::TowardNegInf` is the name of a rounding — a Go
  frontend declaring the same stance gets the same rendering. Nothing in the form reaches a
  formatter, so `crate_boundaries.rs` is unaffected.
- **Mode or form?** Both, at two levels, and keeping them straight is the decision. *Performing* an
  effect differs from evaluating an expression in **shape**, so it is a new statement form.
  *Rendering* differs in the **semantics** of one operation, so it is a mode on that form — exactly
  as `units` is a mode on `Expr::Len` rather than two length forms.
- **Format version.** [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) advances. Every
  cached build is invalidated once; see the Migration Plan.
- **Fingerprint.** [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) must cover the
  operation, the arguments, and the convention. The convention changes the program's *observable
  output*, so it is squarely on the covered side of the pre-pass line — two units differing only in
  it must not share a cached build.
- **Coverage.** A new `Stmt` form trips both
  [`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs) and the
  `(form, position)` matrix in `conformance.rs`. Paid with a demo algorithm that prints and with
  the new form covered in all four positions — free function body, method body, constructor body,
  and loop body — both scheduled in tasks.

### 2. Rendering is a declared convention, not the target's default

**Decision.** The convention rides on the operation as a mode, and the backend matches on it:

```rust
match convention {
    Convention::Python => render_python(value),  // True, 5.0
    // a target-native rendering is expressible without reopening the IR
}
```

**Why.** This is the decision the change turns on. Python prints `True`, Rust prints `true`. Python
prints `5.0` for a whole float; Rust's `{}` prints `5`. Emitting the target's default would make
compiled output differ from interpreted output — and `print` is exactly what a user reaches for
when something is *already* wrong, so a divergence here corrupts the tool they are debugging with.
This is the IR's stated rule — "IR operations carry the semantics the resolved behavior declared,
not one language's by default" — applied to output.

**Alternatives considered.** *A seventh behavior axis.* Rejected for the reason
`add-intrinsic-calls` rejected one: an axis costs a field on
[`LanguageBehavior`](../../../crates/compylr-ir/src/behavior.rs#L179) and a declared stance from
every language, and nobody wants Python's boolean spelling with Go's float spelling. *Always render
as the source language.* Nearly right, and it is the default. It is expressed as a mode anyway so a
target-native rendering is expressible later without reopening the IR — the same reason
`unchecked-arithmetic` exists as a declined option rather than as nothing.

### 3. Output goes through a runtime sink the host installs

**Decision.** The backend emits a sink call; the bridge installs the sink:

```rust
// backend emits this — it names no host
compylr_sink::write_line(&parts);
// bridge installs this — it names Python, and only the bridge may
compylr_sink::install(|bytes| py_stdout_write(bytes));
```

**Why.** Rust's `std::io::stdout()` and Python's `sys.stdout` hold **separate buffers**.
Line-buffered to a terminal the difference is usually invisible; block-buffered to a pipe or a file
the two orders scramble outright. Worse, `contextlib.redirect_stdout`, pytest's `capsys`, and
notebook capture all work by replacing `sys.stdout`, and none of them can see a write the Rust side
made directly. Backend emits target code; bridge knows the host. That is the existing division of
labour, not a new one. A default sink writing to the target's own stdout is kept, so a crate built
outside a host still prints — without it, `cargo run` on generated code would be silent, which
reads as a bug.

**Alternatives considered.** *Write to Rust's stdout and flush aggressively.* Flushing fixes neither
redirection nor capture, because the bytes never pass through `sys.stdout` at all. *The backend
emits PyO3 calls directly.* Forbidden — the backend cannot name Python, and `crate_boundaries.rs`
fails the build if it does.

### 4. Printing a mapping or a set is refused

**Decision.** A located diagnostic pointing at an ordered projection:

```python
print(counts)          # error: a mapping has no guaranteed order, so its printed form is not
                       #        a value the compiled and interpreted builds must agree on
print(sorted(counts))  # the suggested way through
```

**Why.** Their iteration order is unspecified by deliberate choice. A differential test on their
printed form would therefore be **flaky rather than correct** — it would pass locally, fail in CI,
and implicate the compiler in a divergence the language never promised to avoid. This falls straight
out of an invariant already recorded in three places; the change adds no new rule, it declines to
break one.

**Alternatives considered.** *Print them in insertion order.* That is a promise about mapping order,
made through the back door, on a language that deliberately declines to make it.

### 5. The carve-out is in the registry, not in the lowering condition

**Decision.** Lowering asks whether the operation is effectful rather than matching its shape:

```rust
// before — a hand-written shape test, twice
if !is_unit_returning_method_call(&expr) { return Err(bare_expression_error(stmt)); }
// after — the registry answers, and every later effectful operation is covered
if !is_unit_returning_method_call(&expr) && !registry::is_effectful(&expr) {
    return Err(bare_expression_error(stmt));
}
```

**Why.** [`bare_expression_error`](../../../crates/compylr-frontend-python/src/lower.rs#L1677) is
raised from two call sites that special-case `append` and unit-returning method calls by shape.
Adding `print` by shape would make a third hand-written exception. Asking the registry tests the
reason the rejection exists — that the value is discarded — directly, and `add-logging` then needs
no change here at all.

**Alternatives considered.** *A third shape test.* It works and it is the version that has to be
edited again for every effectful operation ever added.

### 6. Rendering a sequence writes into one buffer

**Decision.** The renderer takes the output buffer rather than returning a string:

```rust
fn render_into(buffer: &mut String, value: &Value, convention: Convention);
```

**Why.** Rendering element-by-element into freshly allocated strings makes printing a sequence
quadratic in allocator traffic. The demo is where cost shows up, and it has already found a
quadratic clone in `for` and an O(n) clone per nested read — both invisible to every correctness
test. A linear rendering is specified so the third one is not found the same way.

**Alternatives considered.** *Return `String` and join.* Simpler to read and allocates once per
element plus once per join, which is the shape of the two defects already found this way.

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
