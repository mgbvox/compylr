## Why

compylr's Rust output is a faithful Python emulator rather than Rust. Every `+` is
`PyAdd::py_add(&(a), &(b))?` — a `checked_add` behind a `Result` behind a `?`; every `xs[i]`
resolves a possibly-negative index against the length at runtime; every `//` corrects Rust's
truncation back to flooring. That fidelity is correct, and today it is the *only* thing on offer.
A user who moved a hot loop to Rust precisely because they want Rust's arithmetic has no way to
say so.

**What this is not.** The shape above reads like a performance problem and is mostly not one.
Measured on the demo at SCALE=4: the trait-call *dispatch* costs almost nothing once it inlines,
and making it inline is one line in the generated manifest — `lto = "fat"`, `codegen-units = 1`,
worth 10–25% across the board — rather than an IR change. The semantics-preserving optimizations
found alongside this change are larger still: an `x = x + y` peephole on strings is worth 4.1x on
one workload, the hasher 1.93x on another, and the boundary conversion is why `binary_search` runs
16x *slower* compiled than interpreted. All of those belong to
`improve-generated-code-performance` and are deliberately not here.

What is left for behavior to buy is the **check itself** — the overflow branch, the index
resolution, the flooring correction — and how much that is worth is a number this change must
produce rather than assume. So the case is made on semantics: a user who wants Rust's arithmetic
has no way to ask for it, and `unchecked-arithmetic` sits declared with nothing behind it. Speed
is a consequence to be measured, not the argument.

The negotiation machinery already anticipates this trade and has nothing to negotiate with. A
frontend declares guarantees, a backend declares what it preserves, and `unchecked-arithmetic`
sits in the Rust backend *declared but not implemented* — reserved for a request no one can make.
Both halves of the conversation exist except the user's.

Now is cheap and later is not. Four constants at the top of `lower.rs` — `PY_TRUE_DIV`,
`PY_FLOOR_DIV`, `PY_MOD`, `PY_INDEX_ORIGIN`, `PY_TEXT_UNITS` — already decide five of the six
axes, and the IR already carries each on its node. Turning constants into a resolved profile is a
contained change while there is one frontend and one backend. Once there are two of either, every
one of them re-derives the same resolution independently, and they will not agree.

## What Changes

- A **behavior** is what a user asks for and what a compilation resolves: for each axis where two
  languages disagree, which of the two languages' meanings applies. `behavior="rust"` in a
  Python→Rust compilation means every axis takes Rust's meaning; `behavior="python"` — the default
  — means every axis takes Python's, which is exactly today's output.

- Six axes, each an operation a programmer writes and two languages read differently:

  | Flag | Python's meaning | Rust's meaning |
  | --- | --- | --- |
  | `overflow` | `a + b` reports a result outside `i64` | native `+`; overflow is not defined by the program |
  | `floor_div` | `-7 // 2` is `-4`; a zero divisor reports | `-7 / 2` is `-3`; a zero divisor traps |
  | `true_div` | `1.0 / 0.0` reports | `1.0 / 0.0` is `inf` |
  | `modulo` | `-7 % 2` is `1` | `-7 % 2` is `-1`; a zero divisor traps |
  | `index` | `xs[-1]` is the last element; out of range reports | `xs[-1]` is out of range; out of range traps |
  | `text_len` | `len("é")` is 1 | `len("é")` is 2 |

- **BREAKING (IR shape).** Operations that can fail gain a `Checked` mode saying whether the
  program defines what happens when they do — the same shape `Rounding` and `RemSign` already
  have, and for the same reason. `BinOp::Add`/`Sub`/`Mul` and `Expr::Neg` carry it for overflow,
  `Div` and `Rem` for a zero divisor, `Expr::Subscript` for an index out of range or a missing
  key. The serialized IR moves to format version 4, so every existing `.compylr` cache rebuilds
  once.

- **A language declares its stance per axis.** `Frontend` and `Backend` each answer "what does my
  language mean by integer division / remainder / indexing / …". Resolution reads the two
  declarations and the user's request; neither side hardcodes the other. Adding a language means
  answering six questions, not editing a resolution table — the N + M property the component model
  already has for everything else.

- **Only the two languages in the compilation are nameable.** In Python→Rust, a flag may be
  `"python"` or `"rust"`. `"haskell"` is rejected as not a language compylr knows; `"go"` is
  rejected with the distinct message that it is a target compylr has reserved but is not one of
  the two languages *here* — the same three-way honesty the registries already use.

- **The Python surface gains one keyword in two places.** `compylr.initialize(behavior=...)` sets
  the project default; `@c.compyle(behavior=...)` overrides it for one member, inheriting every
  flag it does not name. Both accept a bare language name or a `compylr.Behavior(...)` object;
  `behavior="rust"` is exactly `Behavior` with every flag set to `"rust"`. A bad value is rejected
  by the decorator that named it, not by a build much later.

- **Behavior may differ between members of one project.** Unlike `backend`, which is refused when
  mixed because a project compiles to one artifact, behavior rides on the IR nodes of each
  function, so a Python-behavior function calling a Rust-behavior one is well defined and shares
  the same artifact.

- **The Rust backend emits its own operators where a node declares Rust's meaning.** With the
  expected type known, `a + b` on integers emits as `((a) + (b))` and not as a trait call; where
  the expected type is not known — inside a comparison — it emits through an infallible trait so
  the backend still never re-derives the type checker. Generated function signatures stay
  uniformly `Result<T, RuntimeError>`: an unrelated edit must not move a signature, and that
  reason does not weaken here.

- **What a unit requires becomes a property of the program, not of the language.** A unit whose
  arithmetic declares `Unchecked` no longer requires `IntegerOverflowReported`, which is what makes
  `unchecked-arithmetic` a coherent thing to permit rather than a name with nothing behind it.

- `--behavior` on the CLI, alongside the existing `--frontend` and `--backend`, so what a file
  compiles to under a profile is inspectable without a build.

- The demo grows a behavior comparison, so "this is faster" is measured rather than asserted.

## Capabilities

### New Capabilities

- `semantic-behavior`: the behavior model — what an axis is, that a language declares a stance per
  axis, how a user's request resolves against the `(source, target)` pair, the rule that only those
  two languages are nameable, the default of the source language, and the requirement that a
  resolved behavior be fully determined before lowering runs.

### Modified Capabilities

- `ir`: operations that can fail carry a `Checked` mode; the artifact format moves to version 4;
  a unit's recorded requirements are derived from its contents rather than from its frontend.
- `ir-lowering`: lowering takes a resolved behavior and sets every mode from it, rather than from
  fixed Python constants.
- `python-frontend`: declares Python's stance on all six axes; the guarantees it reports become
  those of the resolved behavior rather than a static list.
- `rust-backend`: declares Rust's stance on all six axes; emits native operators for nodes that
  declare Rust's meaning, and the infallible trait shims that make that possible without a type
  checker in the backend.
- `python-api`: `behavior` on `initialize` and on `@c.compyle`, the `Behavior` object, inheritance,
  validation and its messages, and mixed behavior within one project.
- `native-bridge`: `_core` accepts a per-source behavior and exposes behavior validation, so the
  decorator can reject a bad value where it was written.
- `pipeline-architecture`: `Frontend` and `Backend` declare a behavior profile; guarantee
  negotiation reads what the program requires rather than what the language requires.
- `cli`: `--behavior`, accepting a language name or per-axis assignments.
- `demo`: a measured comparison between the default behavior and Rust behavior on the same
  algorithm.

## Impact

**Rust.** `compylr-ir` (`ir.rs` — the `Checked` mode on five node forms, a `Behavior` type and its
axes; `artifact.rs` — version 4). `compylr-core` (`frontend.rs`, `backend.rs` — the profile
declaration; `negotiation.rs` — program-derived requirements; `folding.rs` — folding must read the
new mode, since folding an `Unchecked` overflow to a reported error would be a wrong answer).
`compylr-frontend-python` (`lower.rs` — the five constants become a profile parameter, threaded
through ~40 construction sites; `component.rs` — Python's stance). `compylr-backend-rust`
(`rust.rs` — native emission; `runtime.rs` — the infallible trait shims, embedded into every
generated crate). `compylr-host-python`, `compylr-cli`.

**Python.** `_config.py` (`Behavior`, the `behavior` field, inheritance, validation),
`_manager.py` (`initialize`, `compyle`, per-member behavior reaching `compile_unit`), `__init__.py`
(`Behavior` exported), `_core.pyi`.

**Caches.** Every `.compylr` artifact is invalidated once by the format version. Behavior is part
of the IR, so changing a flag changes the fingerprint and rebuilds without new machinery.

**Docs and tests.** `README.md` gains the axis table (`tests/readme.rs` enforces the mechanical
half). `tests/conformance.rs` must cover each axis in both settings — the `(form, position)`
matrix gains a third dimension, and the honest scope is every *failing* form under both settings
rather than the full cross product. `demo/` and `make demo`.

**Not in scope: the boundary, and it is the larger cost.** How values cross the Python/Rust
boundary is a property of the pair, not something an axis selects — so no flag here touches it.
That exclusion is right on semantics and expensive on performance, and saying so is the honest
version: conversion is per element on every call, roughly 4 ns for a `list[int]` element and
42 ns for a `list[str]` element, which is why `binary_search` over 2000 elements runs **16x
slower** compiled than interpreted. Nothing in this change improves that, and no behavior flag
should. See `improve-generated-code-performance`.

**Not in scope.** No new axis for a missing mapping key's *shape* (Go's `v, ok :=` is a different
expression, not a setting — unchanged from today). No axis for `range`'s zero-step rejection: both
languages refuse it, and the check exists so a non-terminating loop has something to diagnose.
No third value for any flag: the two languages in the compilation are the whole domain.
