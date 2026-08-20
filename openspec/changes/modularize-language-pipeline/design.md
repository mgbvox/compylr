## Context

Today `compylr` is one crate of ~8,500 lines with `lower.rs` (3,406) and `ir.rs` (1,454) at its
centre, plus `backend/rust.rs` (1,285), `backend/runtime.rs` (424) and `backend/bindings.rs` (292) on
the target side. The `Backend` trait and its three-way registry already exist and work well — that
part of the shape is right and this design keeps it. What does not exist is a symmetric notion of a
frontend, a place for a source/target *pair*, or any way for a node to say what it means.

Three constraints shape everything below.

**The IR is the only thing every component shares**, so it is the one place where a leak is fatal.
`BinOp::FloorDiv`, `Ty::python_name`, `BinOp::python_symbol`, and `Expr::Range` ("as Python's `range`
produces") are all leaks in the source direction; the type model is already clean in the target
direction.

**compylr is not LLVM.** LLVM emits object code and never calls back into the source language, so
frontends and backends compose N + M. compylr's whole purpose is that Python calls the result, and a
calling convention is a property of the *pair*. Pretending otherwise is how the N×M cost gets hidden
somewhere it cannot be seen.

**The Python surface must not move.** `compylr.initialize`, `@c.compyle`, `COMPYLR_DISABLE`, the
`compylr compyle` console script, and the `compylr._core` function signatures are all in use, and the
demo project and its benchmark depend on them.

See proposal.md — Why, for motivation, and the delta specs for the behavior being contracted.

## Goals / Non-Goals

**Goals:**

- Adding a source language means adding one crate and one registry entry. Adding a target language
  means the same. Neither requires editing the IR, the passes, or the other side.
- Every meaning a node can have is readable from the node. No component infers a source language.
- The N×M cost is confined to one thin, optional, individually-registered layer, and is *visible* —
  a missing pair is a specific, reportable answer rather than a crash or a wrong assumption.
- The Python→Rust path behaves exactly as it does today: same accepted subset, same rejections, same
  diagnostics, same runtime results.
- The seams are load-bearing rather than aspirational, proven by the crate graph (a backend that
  cannot build a Python parser) and by a conformance corpus enumerated from the registry.

**Non-Goals:**

- No second frontend and no second backend. `typescript`, `go`, and `cpp` stay reserved.
- No optimizer. One real pass (constant folding) exists to prove the pass interface and the
  semantics carrier; the rest of the pipeline is infrastructure.
- No change to the supported Python subset, and no new diagnostics beyond those the new failure
  modes require.
- Not a rewrite of `lower.rs`. It moves and its imports change; its logic does not.

## Decisions

### D1. Semantics ride on the node, not on a unit-level "language mode"

`BinOp::FloorDiv` becomes `BinOp::IntDiv { rounding: Rounding }` with `Rounding::{TowardNegInf,
TowardZero}`; `Mod` becomes `Rem { sign: RemSign }` with `RemSign::{Divisor, Dividend}`; `TrueDiv`
becomes `Div { promote: Promotion }`. A frontend sets them; a backend matches on them.

*Alternative considered: a `SemanticsProfile::Python` on the unit, with operators staying bare.*
Rejected — it recreates the problem one level up. A pass would have to consult the profile to fold
`7 // -2`, so every pass would grow a source-language switch, and a language that is Python-like in
division but C-like in remainder would need a profile of its own. Flags on the node are how LLVM
handles the same tension (`sdiv`/`udiv`, `nsw`/`nuw`), and they compose.

*Alternative considered: leave it, and let each frontend normalize into Python's meanings.* Rejected
— it forces every future frontend to emit a correction expression for its own native operator, which
is both slower and a place for each frontend to get it subtly wrong independently.

The enums are deliberately small and closed. Two rounding modes and two remainder conventions cover
Python, Go, C++, Rust, and TypeScript; a language needing a third adds a variant to the IR, which is
a reviewable event rather than a silent frontend convention.

### D2. Two axes, three registries

`Frontend` and `Backend` are the two axes; `HostBridge` is the pair. All three resolve by name (or
name pair) through the same three-way `Entry { name, impl: Option<...> }` shape the backend registry
already uses, so "reserved" stays a first-class answer everywhere.

```
Frontend:   &str            -> impl Frontend    (source text -> Unit)
Backend:    &str            -> impl Backend     (Unit -> GeneratedFiles)
HostBridge: (&str, &str)    -> impl HostBridge  (Unit + BuildKey -> callable artifact)
```

The `BuildKey` is not decoration. A bridge knows the pair but not the pass configuration, and the
name a host loads the artifact under has to distinguish builds a process might hold at once —
otherwise two builds of one source under different settings collide and the second silently *is*
the first.

*Alternative considered: fold bridging into `Backend::emit_python_extension`, which is where it lives
today.* Rejected — that method's name already says what is wrong with it. A Go backend would need
`emit_python_extension` and `emit_typescript_extension` and so on, so the N×M matrix would live as
methods on the backend trait, growing every time a *frontend* is added.

*Alternative considered: a canonical C ABI hub, so every language bridges to C and N×M collapses to
N + M.* Genuinely attractive and explicitly deferred, not rejected: it costs a marshalling layer for
collections that today cross by direct PyO3 conversion, and it would change the observable
performance the demo benchmark measures. The `HostBridge` trait is the seam a C-ABI hub would later
be implemented *behind* — a single bridge registered for many pairs — so choosing it later costs no
rework. Recorded as the intended escape hatch if the matrix ever exceeds a handful of entries.

### D3. Guarantee negotiation is a declared set, checked before emission

A frontend returns the guarantees it requires; a backend returns those it preserves; core
intersects them and fails with the missing member's name. The initial set is
`{ IntegerOverflowReported, DivisionByZeroReported, FloatOrderPreserved }`.

This exists for the note's last line — "Y: post-generation Y-specific optimizations (if compatible
with expectations from X or explicitly allowed)". It is not hypothetical for compylr: the generated
crate's profile is a place where someone will eventually want `overflow-checks = false` or a
fast-math equivalent, and each would silently stop the compiled function from meaning what the Python
meant. Making it a declaration turns that into a refusal with a name in it.

*Alternative considered: a boolean `allow_unsafe_optimizations`.* Rejected — it cannot say *which*
guarantee was traded, which is the only information the person reading the failure needs.

### D4. Passes are a pipeline of named, IR→IR functions, with verification first

```
lower -> verify -> [agnostic passes] -> [pair-directed passes] -> emit
```

Verification is unconditional and is the piece with immediate value: it is what catches a *new*
frontend emitting a tree that lowering's own invariants would have caught for Python, without
requiring each frontend to re-derive those checks. `returns_on_all_paths` is the model here — it is
already shared between lowering and the backend for exactly this reason.

Constant folding is the one optimization, chosen because it *must* read the semantics flags to be
correct. A folder that gets `7 // -2` right is a working test of D1; a folder that leaves
division-by-zero and overflow alone is a working test of "a pass does not optimize away an error".

The fingerprint is taken on the pre-optimization IR, and the pass configuration is recorded in build
state next to the compiler version. Otherwise enabling a pass would look like the user edited their
code, and disabling one would silently reuse an optimized artifact.

### D5. The workspace, and what its edges enforce

```
compylr-diagnostics   spans, located-error scaffolding          (no deps)
compylr-ir            types, nodes, semantics, guarantees,      -> diagnostics
                      fingerprint, serialization
compylr-core          traits, passes, pipeline, verification,   -> ir, diagnostics
                      guarantee negotiation
compylr-registry      the tables: which frontends, backends,    -> core + every implementation
                      bridges, and directed passes exist
compylr-frontend-python  ruff parse + lowering                  -> core, ir, diagnostics, ruff
compylr-backend-rust     rust emission + emitted runtime        -> core, ir, diagnostics
compylr-bridge-python-rust  PyO3 generation onto user code      -> core, ir  (+ backend-rust)
compylr-cli              the `compylr` binary, --emit           -> registry and below; no PyO3
compylr (cdylib)         `compylr._core`, PyO3 bridge           -> all of the above
```

The registry is a crate of its own for a reason that only becomes visible once you try to write the
table: a crate that *defines* what a backend is cannot name the crates implementing that trait, or
the dependency is a cycle. So core holds the interfaces and knows no implementation, and one crate
above it is allowed to know them all. That crate is also the single place a new language is
registered, which is the property the whole design is for.

The graph is the enforcement mechanism, not a convention: `compylr-backend-rust` cannot mention
Python because it does not depend on anything Python, and `compylr-ir` cannot grow a `python_name`
because ruff is not among its dependencies. This is why the split is into crates rather than modules
— module boundaries in one crate are advisory, and this exact leak already happened once inside them.

`compylr-bridge-python-rust` depending on `compylr-backend-rust` is a deliberate asymmetry: the
bridge needs the Rust backend's type spellings to write conversions, but the backend must not need
the bridge. The dependency points from the pair-specific crate to the general one, which is the
direction that keeps `cargo build -p compylr-backend-rust` free of PyO3.

*Alternative considered: keep one crate with stricter module discipline.* Rejected on evidence — the
Python spellings on `Ty` are in the current tree despite a module comment saying the IR names no
language.

### D6. `Ty::python_name` and `BinOp::python_symbol` move to the Python frontend

The IR keeps a neutral `Display` for debugging and artifacts. Diagnostics that quote a type or
operator to the user get the spelling from the frontend that read the source, because the whole point
of those strings is to echo what the programmer wrote.

This is a real cost: `LowerError` currently formats messages using `python_name` from anywhere. The
mitigation is that lowering *is* the Python frontend after the split, so the function lands in the
crate that already uses it almost exclusively.

### D7. `Expr::Range` stays, and is renamed

`Range` is a counted-iteration form, not a Python feature; Go's three-clause `for` and C++'s
`iota` lower to it just as naturally. It keeps its zero-step rejection and its cursor-based emission.
Its doc comment stops citing Python's defaulting rules and states the form's own contract: start,
stop, step, half-open, step non-zero.

*Alternative considered: replace it with a `while` and an explicit counter at lowering time.*
Rejected — it would lose the zero-step diagnostic (a hang has nothing to diagnose from) and hand every
backend a loop it cannot recognize well enough to emit idiomatically.

### D8. `compylr._core` keeps its name, its module path, and its signatures

Only what it links against changes. `python/compylr/*.py` is untouched apart from build state gaining
the pass configuration; `_core.pyi` is unchanged. The demo project and its benchmark are the
regression test for this: they are built from the user-facing API and would break loudly.

## Risks / Trade-offs

- **The IR shape changes, so every user's cached build is invalidated once.** → Build state already
  records the compiler version, so the rebuild is automatic rather than a stale-artifact bug. Stated
  in the proposal's Impact; the cost is one rebuild, which the demo measures at ~8s.

- **A 3,406-line file moving between crates is where a silent behavior change hides.** → The move is
  mechanical and is done as its own task with no logic edits, so `git` reports pure relocation; the
  accepted/rejected fixture suites and the execution tests run unchanged before and after. Any
  diagnostic wording change is a test failure, not a review question.

- **Splitting the crate slows the edit-compile loop, since a change to `compylr-ir` rebuilds
  everything.** → Accepted. `compylr-ir` is the smallest crate and the least frequently edited; the
  common case (editing lowering, or editing emission) now rebuilds *less* than today.

- **`maturin develop` and `cargo llvm-cov` both need workspace-aware invocation, and getting one wrong
  fails in a way that looks like a code problem.** → Both commands are in `CLAUDE.md` and the README;
  updating them is a task, and the llvm-cov ignore regex is updated in the same step. The known
  venv/llvm-cov interaction is already documented and unaffected.

- **The guarantee mechanism could become ceremony — three names checked in one place, satisfied by
  construction.** → Its value is real only when the first violating option appears. It is kept
  minimal now (a set intersection, no configuration DSL) so it costs little while unused, and the
  Rust backend's declaration is asserted against the Python frontend's requirement by test, so the
  check is exercised rather than dead.

- **Constant folding changes the emitted Rust for existing fixtures, so snapshot tests move.** →
  Expected and reviewed as part of the folding task. Execution tests assert on *values*, not emitted
  text, and are the check that folding is correct; snapshot churn is cosmetic.

- **N×M is confined but not solved.** With one bridge it is invisible; at three sources and three
  targets it is nine crates. → D2's C-ABI hub is the planned answer and the trait is shaped to accept
  it. The trigger to reconsider is the second bridge, not the ninth.

## Migration Plan

The workspace split lands before any semantic change, so that each step is separately reviewable and
separately revertable:

1. **Move, don't change.** Create the workspace and relocate modules verbatim. The full suite must
   pass with no test edits beyond import paths. `git` should show renames.
2. **Introduce the traits and registries** with the existing Python frontend and Rust backend as
   their first implementations, and the existing `emit_python_extension` re-homed as the first
   `HostBridge`. Still no behavior change.
3. **Carry semantics on nodes** (D1), updating the Python frontend to declare and the Rust backend to
   read. This is the fingerprint-changing step and the one that forces a rebuild.
4. **Add verification and the pass pipeline**, with an empty optimization set. Behavior unchanged.
5. **Add constant folding**, guarantee negotiation, and the conformance corpus.
6. **Update `README.md`, `CLAUDE.md`, and the commands** in the same change, since `tests/readme.rs`
   enforces the mechanical half and will fail until they agree.

Rollback is per-step: steps 1, 2, and 4 are behavior-preserving and revert cleanly. Step 3 is the
irreversible one for caches, but reverting it only causes one more rebuild.

## Open Questions

- Should the CLI grow a `--frontend` flag now that frontends are named? It has no second frontend to
  select and the default is unambiguous, so this is deferred; it does not affect the specs or the
  crate boundaries either way.
- The `Instance(String)` nominal type is the one place the type model is not structural. A second
  frontend with different class semantics may need it parameterized, but nothing in this change
  depends on the answer.

## What changed during implementation

The decisions above are as written before the work started. Four of them moved, and the reasons are
worth keeping — each was forced by the dependency graph rather than chosen, which is itself evidence
that the crate split does the job it was introduced to do.

**The registry became its own crate (D5).** The design put registries in `compylr-core`. Writing the
table showed that impossible: core defines what a backend *is*, so it cannot name the crates that
implement the trait. `compylr-registry` sits above core and below nothing else, and is now the
single place a language is registered — a better outcome than planned, but not the planned one.

**`LowerError` went to `compylr-diagnostics`, not to the Python frontend.** The task list assumed
the whole of `error.rs` was Python's. It is not: every kind — missing annotation, unresolved name,
arity mismatch, type mismatch — is a category any frontend for an annotated subset produces, and
`Unit::validate` raises two of them itself. Putting them in the frontend would have pointed the IR
at Python, which is the exact edge the split exists to remove. Only `SourceError`, which wraps
ruff's `ParseError`, is genuinely Python's.

**`Span` lost its parser dependency.** It used ruff's `LineIndex` to resolve a line and column.
Diagnostics sits below the IR, so anything it pulls in reaches every crate in the workspace —
including a Python parser, which would have made "a backend cannot name Python" untestable.
`line_column` is a dozen lines of plain Rust now, and the parser conversion lives in the frontend as
`span_of`.

**`Guarantee` lives in `compylr-ir`, not `compylr-core` (D3).** A unit records what its frontend
requires, so the type has to sit at or below the IR. Core re-exports it, so a frontend or backend
declaring guarantees still names one type.

**`emit_python_extension` came off the `Backend` trait in step 1 rather than step 3.** The bridge
crate sits downstream of the backend, so the call could not point upward; the removal happened where
the graph forced it, and D2's `HostBridge` was built around the already-removed method rather than
removing it itself.

**`typescript`, `go`, and `cpp` are reserved as frontends too.** The task list said "no reserved
names yet" for the frontend registry. That left the spec's three-way-resolution scenario with no
reserved name to exercise, and a language compylr supports in one direction and not the other is not
a language compylr supports.
