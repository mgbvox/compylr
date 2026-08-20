## Context

See proposal.md — Why. Three facts shape the approach:

`Expr::Subscript` is used for sequences, mappings, and strings; `Expr::Len` for all of those plus
sets. The backend does not know which it has: the IR does not annotate expressions with their types,
so emission produces one call and Rust selects the implementation by type through `PyIndexable` and
`PyLen`. Any mode a node carries therefore reaches the runtime as an *argument*, and reaches
implementations for which it means nothing.

`runtime.rs` is embedded verbatim into every generated crate via `include_str!`, so anything added to
it ships to users, and it may not name anything outside itself.

The arithmetic work set the standard for what earns a mode, and it is worth restating because it is
what keeps this from becoming a configuration surface: *both readings must be somebody's, and the
divergence must be a parameter of one operation rather than a different operation.*

## Goals / Non-Goals

**Goals:**

- Nothing in the IR silently means Python's version of a container operation.
- Every helper in the emitted runtime has a native test, including the ones no Python program can
  reach.
- The conformance corpus fails when a form is untested *in a position*, not only when it is untested
  at all.
- `compylr compyle` imports a package the way Python does.

**Non-Goals:**

- Splitting `Subscript` and `Len` into per-operand nodes. See D2.
- Any behavior change for Python programs. Same accepted subset, same results, same diagnostics.
- A second frontend or backend.

## Decisions

### D1. Two modes, and the three that are deliberately absent

`IndexOrigin::{FromEitherEnd, FromStart}` on `Expr::Subscript`; `TextUnits::{CodePoints, Utf8Bytes,
Utf16Units}` on `Expr::Len`. Both pass the test: Python means one thing, Go and C++ and TypeScript
mean another, and it is the same operation either way.

Three container behaviors that look like candidates and are not:

**A missing mapping key.** Python raises, Go yields the value type's zero, TypeScript yields
`undefined`. Modeling that as a mode would require the IR to know a type's zero value — and
`Ty::Instance` has none. But the deeper objection is that `v, ok := m[k]` is not `m[k]` with a
setting; it is a different expression with a different result type. A frontend that means it lowers
to a different form, exactly as `Expr::Range` is a distinct form rather than a mode on a call.

**Iterating a mapping.** Python yields keys; Go's `range` over a map and TypeScript's `for...in` also
yield keys. No divergence to model.

**String membership.** Substring in all four languages. No divergence to model.

Recording these in the spec matters as much as adding the two modes. A reader who finds
`PyContains` still named for Python needs to know that is a conclusion rather than an omission.

### D2. Modes ride on the existing nodes, inert where they do not apply

`Subscript { base, index, origin }` and `Len { value, units }`. `origin` means nothing for a mapping
and `units` mean nothing for a `Vec`, and both say so in their doc comments.

*Alternative considered: split the nodes, so every field is meaningful* — `Index`/`Lookup` and
`Len`/`TextLen`, with the frontend choosing by operand type. This is the more honest model and has
precedent: `Expr::TupleIndex` was split out of `Subscript` for exactly this reason. Deferred because
it roughly doubles the work — every lowering site must consult the operand's type, `PyIndexable`
splits, and the delta grows — for a gain that is real but smaller than the gain from having the
modes at all. Recorded as the next step if the inert fields prove to mislead anyone.

The inert-field compromise has one concrete cost worth naming: a backend author can pass the wrong
mode to a mapping read and nothing will notice, because nothing reads it. The conformance corpus is
what catches the reverse mistake — a mode being ignored where it *does* apply.

### D3. `Expr::Len` becomes a struct variant, which is the mechanical cost

`Len(Box<Expr>)` is currently grouped with `Neg | ToFloat | Not` in three match arms that share a
single-child traversal. Those arms split. This is the bulk of the diff and none of the risk.

### D4. Runtime helpers take the mode; implementations that do not need it ignore it

`py_index(items, index, origin)`, `PyIndexable::py_get(&self, index, origin)`,
`PyLen::py_len(&self, units)`. The mode enums are defined in `runtime.rs` itself, because it must
stay self-contained, and are therefore duplicated between the IR's copy and the runtime's. That
duplication is deliberate: the IR's enum is a program model and the runtime's is a value that ships
to users, and coupling them would mean the generated crate depending on compylr at build time.

A test asserts the two stay in step, since they are two spellings of one decision.

### D5. Corpus coverage becomes `(form, position)`, checked in Rust rather than by string search

The current check serializes the corpus and looks for variant names, which cannot see position at
all. Replaced with a walk over the corpus units that records where each statement form appears,
against a table of the five positions and which forms are legal in each:

| Position | Notably illegal |
| --- | --- |
| function body | `Break`, `Continue` outside a loop |
| constructor body | `Return(value)` — verification rejects it |
| method body, shared receiver | statements that assign an attribute |
| method body, mutable receiver | — |
| loop body | — |

The string check goes away rather than being kept alongside: two coverage checks that can disagree
is worse than one that is exact.

### D6. Precompile creates ancestors on demand instead of relying on sort order

Two independent defects, one fix each, plus a structural improvement:

- `sys.modules` gets a synthetic `_compylr_precompile` root package before anything is imported.
- `__init__.py` is loaded with `submodule_search_locations`, which is what makes `__package__`
  resolve to the package itself rather than to its parent.
- Every missing ancestor is created on demand rather than assuming `__init__.py` was imported first.

The third is not redundant. `_module_files` walks `sorted(directory.iterdir())`, which places
`__init__.py` before lowercase siblings and *after* an uppercase-named subpackage — `A` sorts before
`_`. Fixing the sort would work today and break the first time someone adds a directory whose name
sorts differently; creating ancestors on demand removes the dependency on order entirely.

*Alternative considered: import each file as a top-level name with no dots.* Rejected — relative
imports would then have no package to resolve against at all, which is a worse version of the bug
being fixed.

## Risks / Trade-offs

- **A second forced rebuild for every project, one change after the last one.** → Unavoidable if the
  modes are to reach the fingerprint, and they must: two programs that index differently are
  different programs. Stated in the proposal; the build state's version check makes it automatic.
  This is intended to be the last: after it, the IR's remaining Python-specific behaviour is
  documented as a conclusion rather than a gap.
- **Inert fields invite misuse.** → D2 names the cost and the escape route. The alternative was
  weighed rather than overlooked.
- **The mode enums are duplicated between the IR and the emitted runtime.** → Deliberate, for the
  reason in D4, and asserted by a test so they cannot drift silently.
- **Rewriting the corpus coverage check could weaken it.** → The new check must fail for the same
  omission the old one caught. Verified by deletion, the way the current one was: remove an entry and
  confirm the specific missing pairs are reported.
- **`compat.rs` grows for every user.** → Two small enums and a parameter. The file is already
  ~17KB of helpers; the readability cost is near zero and the correctness gain is that a second
  frontend does not have to fork it.

## Migration Plan

1. Modes on the IR, with the frontend declaring Python's readings and the backend reading them off
   the node. Fingerprint-changing; artifact version 3.
2. Runtime helpers take the modes; native tests for every helper in the file.
3. Execution tests for the readings no Python program can produce.
4. Corpus coverage by position, verified by deletion.
5. Precompile fix and its tests, including the demo asserting zero import failures.
6. README, `CLAUDE.md`, and the demo artifacts.

Steps 4 and 5 are independent of 1–3 and of each other; either can land first if step 1 stalls.
