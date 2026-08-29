## Why

`print` is the first thing anyone writes and the subset cannot express it. `lower.rs:1677` refuses
it in as many words — *"this statement computes a value and discards it, which is either dead code
or a side effect the subset cannot express"* — and the wording is precise: the problem is not that
`print` is missing from a table, it is that the IR has no way to say **a statement performed an
effect on the outside world**.

`Stmt::Effect` exists but does not mean that. Its documentation is explicit that it holds one thing:

> Lowering only ever puts a unit-returning **method** call here. A free function in this subset can
> reach no mutable state, so calling one and discarding the result is dead code and stays rejected;
> a method can mutate its receiver, which is the whole point of one.

Every effect the subset admits today is a mutation of something the program already owns. Output
leaves the program entirely, and nothing in the IR distinguishes that from dead code.

The practical cost is larger than one builtin. `COMPYLR_DISABLE=1` exists so a marked function can
be measured interpreted, and `make demo` compares the two — but a user debugging why the compiled
answer differs from the interpreted one cannot add a print to find out. The tool that makes the
compiler legible to its users is the one construct it refuses.

**Why after `add-intrinsic-calls`.** `print` is an operation the program did not compile, so it
needs the namespace and registry that change builds. This change adds the one thing that change
deliberately left out: an intrinsic that is performed rather than evaluated.

## What Changes

- **An effectful intrinsic is a statement form.** The registry gains a notion of an operation with
  no result, performed for what it does rather than for what it returns, and the IR gains a
  statement carrying one. `Stmt::Effect`'s existing meaning — a unit-returning method call — is
  unchanged and untouched.

- **`print` is the proving operation**, accepted with positional arguments only. Multiple arguments
  are separated by a single space and the line is terminated by a newline, matching Python's
  defaults. `sep`, `end`, `file`, and `flush` are keyword arguments, which the subset already
  rejects, so no new refusal is needed and none is added.

- **A printed value is rendered by the source language's convention, carried as a mode.** This is
  the change's substance. Python prints `True`; Rust prints `true`. Python prints `1.0`; Rust's
  `{}` prints `1`. Output is *observable*, so a backend emitting its own spelling would make a
  compiled program produce different text from the interpreted one — the exact divergence the
  differential corpus exists to catch, on the one construct users reach for when something is
  already wrong. The convention rides on the operation the way `TextUnits` rides on `Expr::Len`,
  and a backend matches on the mode rather than on the operation's name.

- **Printing a mapping or a set is rejected**, with a diagnostic saying why. Their iteration order
  is deliberately unspecified — CLAUDE.md: *"never assert on mapping or set iteration order"* — so
  their printed form is not a value CPython and a compiled build can be required to agree on.
  Accepting it would produce a differential test that is flaky rather than a compiler that is
  wrong. Sequences and tuples are ordered and print.

- **Output goes through a sink the host installs, not directly to the target's stdout.** Generated
  Rust writing to `std::io::stdout()` interleaves incorrectly with Python's separately buffered
  `sys.stdout`: piped to a file, the two orders scramble, and `redirect_stdout`, `capsys`, and
  notebook capture see nothing at all. The backend emits a call to a runtime sink; the **bridge**
  installs one that writes through the host's own stream. That split is what keeps
  `compylr-backend-rust` from naming Python, which `crate_boundaries.rs` enforces.

- **BREAKING (artifact format).** A statement form is added, so the artifact version advances and
  every cache rebuilds once, automatically.

## Capabilities

### New Capabilities
- `program-output`: what a compiled program may write, how a value is rendered as text, and how
  output reaches the host's stream in the right order.

### Modified Capabilities
- `intrinsics`: an operation may be effectful, having no result and being performed as a statement.
- `ir`: a statement form carrying an effectful intrinsic, and a rendering convention on it.
- `ir-lowering`: the bare-expression rejection carves out an effectful intrinsic; printing an
  unordered container is refused with its reason.
- `rust-backend`: output emits through the runtime sink, rendering each value by the declared
  convention.
- `native-bridge`: the Python bridge installs a sink writing through the host's stream, and
  ordering is preserved across the boundary.
- `fixture-corpus`: output is compared against CPython's, captured from both.

## Impact

**Modified**
- `crates/compylr-ir/src/ir.rs` — the effect statement form, the rendering mode, the artifact
  version, the fingerprint.
- `crates/compylr-frontend-python/src/lower.rs` — `bare_expression_error` at `:1677` gains one
  carve-out; the unordered-container refusal.
- `crates/compylr-backend-rust/src/rust.rs` and `runtime.rs` — the sink and the rendering functions.
- `crates/compylr-bridge-python-rust/src/bindings.rs` — installing the host sink.
- `frontends/python/fixtures/` — an accepted fixture with a driver, and rejected fixtures.
- `README.md`, `CLAUDE.md`.

**Unaffected**
- `Stmt::Effect` and every program using it.
- Every existing answer. Programs that print nothing emit byte-identical code.

**Costs**
- One rebuild per project.
- A print costs a boundary crossing rather than a bare `write`. Deliberate: correct interleaving is
  the reason the construct is worth having, and a fast print in the wrong order is not a feature.
