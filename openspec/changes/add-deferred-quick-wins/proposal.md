## Why

Four things were deferred across the previous three changes, each for a good reason at the time and
none of them large. They are grouped here because individually they are too small to justify a
change and collectively they remove most of the remaining rough edges.

**Calls do not type their bindings.** The most visible of the four:

```python
@c.compyle
def total(n: int) -> int:
    doubled = double(n)        # MissingAnnotation: `doubled` needs an explicit type
    return doubled + 1
```

`add-local-type-inference` inferred every initializer whose type is determined, and explicitly
excluded calls: typing one during lowering needs the callee's signature, and looking it up there
would reintroduce the decoration-order dependence that moving call resolution into `Unit::validate`
was meant to remove. That was correct, and it left the one initializer form users write most often
requiring an annotation that says nothing new.

**A function that never returns still lowers.** `def f() -> int: pass` passes lowering today. Only
the Rust backend catches it, as a `BackendError` at emit time — a compylr-internal error surfaced
to a user who wrote an ordinary mistake, with no `line:column`.

**The generated Rust cannot be seen without a build.** The CLI stops at the IR. Reading what a
program compiles to means running a full maturin build and finding `.compylr/crate/src/lib.rs`.

**Artifacts follow the working directory.** `.compylr/` is rooted wherever the process starts, so
running the same project from a subdirectory builds it a second time, from scratch.

## What Changes

- **Infer a binding's type from a call.** A two-pass lowering collects every function's signature
  first, then lowers bodies against them, so a call's type is known without depending on the order
  functions arrived. `doubled = double(n)` needs no annotation.
- **BREAKING**: a call to a function that is not in the unit now fails during *lowering* rather
  than during validation, because typing the call requires resolving it. The diagnostic gains a
  `line:column` it did not have, and the failure moves earlier — but a program that used to fail at
  validation may now fail sooner, with a different message.
- **Reject a non-unit function whose body cannot return.** `def f() -> int: pass` becomes a
  lowering error naming the function and its location, instead of a backend error.
- **Add `--emit` to the CLI.** `compylr --emit rust <file>` prints the generated source; the
  default stays the IR summary. `--emit ir` prints the IR artifact as JSON.
- **Find `.compylr/` from the project root.** The manager walks upward for a project marker
  (`pyproject.toml`, or an existing `.compylr/`), falling back to the working directory when there
  is none, so a project has one artifact directory regardless of where it is run from.

Explicitly **not** in this change: inferring parameter or return types (they stay mandatory —
they are the boundary bindings are generated from), recursive or mutually recursive call typing
beyond what a signature pass gives for free, and any new subset feature.

## Capabilities

### New Capabilities

- `cli`: the command-line interface. It exists and is unspecified, which is why adding a flag has
  nothing to check against; this specifies what it already does as well as what it gains.

### Modified Capabilities

- `ir-lowering`: the inference requirement stops excluding calls; a new requirement covers the
  signature pass and the order-independence it must preserve; and a new requirement rejects a
  function whose body cannot produce its declared return type.
- `build-pipeline`: the requirement that artifacts are isolated gains project-root discovery, so
  the directory is a property of the project rather than of the shell.

## Impact

- **Two-pass lowering is a structural change to `src/lower.rs`.** `lower_source` currently walks
  definitions once. It becomes: collect `(name, params, ret)` for every function, then lower each
  body with that table available. The signature pass reads annotations only — which are mandatory —
  so it cannot itself depend on inference, and there is no ordering problem to solve.
- **Order-independence must be re-established, not assumed.** It currently holds because lowering
  resolves nothing. After this change lowering resolves calls, so the property has to be proven
  again: the signature pass sees every function in the unit before any body is lowered, and tests
  must assert that both orderings of a mutually-referencing pair give identical results.
- **`Unit::validate` keeps its job.** Cross-*source* calls still resolve there, because a single
  `lower_source` call sees only one source. Within a source, calls now resolve during lowering.
  This split is worth stating clearly, because the two look like duplicates and are not.
- **A rejection fixture changes meaning.** `python/fixtures/rejected/unannotated_from_call.py`
  exists to assert that a call initializer requires an annotation — the behavior this change
  reverses. It moves to `accepted/`, and the fixture-count guard in `tests/fixtures.rs` updates.
- **Recursion is worth a decision, not an accident.** With signatures available, a self-recursive
  function types fine. Whether the emitted Rust is correct for it is a separate question the tests
  must answer rather than assume.
- **Code**: `src/lower.rs` (the pass split, the never-returns check), `src/main.rs` (the flag),
  `python/compylr/_build.py` and `_manager.py` (root discovery).
- **Ordering**: worth landing before `add-collection-types`. Collections make call-typed
  initializers far more common — `xs = build_list()` is the natural way to write them — so doing
  this first means collections inherit the inference rather than needing annotations everywhere.
