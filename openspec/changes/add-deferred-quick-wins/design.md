## Context

See proposal.md — Why. The four items are independent except that the first is by far the largest,
and it is the one with a real design question.

What the current code establishes:

* `lower_source` walks a module's statements once, lowering each `def` in isolation. Nothing
  resolves anything across functions.
* `Unit::validate` resolves callees and checks arity, deliberately, so that adding functions to a
  unit in any order gives the same result. That property is load-bearing: functions arrive from
  separately decorated sources in whatever order the interpreter imports them.
* `lower_expr` returns `(Expr, Option<Ty>)`, where `None` means *undetermined*. A call currently
  produces `None`, and that single fact is what forces the annotation.

## Goals / Non-Goals

**Goals:**

* Type calls without giving up order-independence — the property, not just the current tests.
* Keep the two resolution sites distinguishable, because after this change there are two and they
  look redundant.
* Leave the CLI and artifact-root items genuinely small.

**Non-Goals:**

* Inferring parameter or return types. They stay mandatory: they are the boundary the bindings are
  generated from, and inferring them needs whole-program analysis.
* Any change to what the subset accepts, beyond the two rules stated.

## Decisions

### D1. Two passes over a source: signatures, then bodies

```
lower_source(parsed):
    signatures = collect_signatures(parsed)   # annotations only
    for def in parsed:  lower_function(def, &signatures)
```

`collect_signatures` reads parameter and return annotations, which are **mandatory**, so it never
needs inference and cannot be order-sensitive. Every body is then lowered against a table that
already holds every function in the source, so a call to a function defined later types exactly as
one defined earlier.

*Alternative considered:* lower bodies lazily, resolving a callee by lowering it on demand. Rejected
— mutual recursion turns it into a cycle needing memoisation and cycle detection, to produce the
same answer the signature pass gets in one linear walk.

*Alternative considered:* keep calls undetermined and infer at the unit level, after assembly.
Rejected — the diagnostic would lose its source location, since a unit spans sources and a span
indexes into one text. Locations on type errors are worth more than the symmetry.

### D2. Two resolution sites, and why that is not duplication

After this change:

| | resolves | reports |
| --- | --- | --- |
| lowering | calls **within one source** | with `line:column` |
| `Unit::validate` | calls **across sources** | without a location |

A single `lower_source` call sees one source. Two decorated functions in different modules can call
each other, and neither source can resolve the other at lowering time — that is precisely why unit
validation exists and it must stay.

The observable consequence is worth stating: a call to a function that exists in **no** source now
fails earlier and with a location, while a call to one in **another** source still fails at
validation without one. Both are correct; a reader who sees only one of them will think the other
is dead code.

### D3. "Cannot return" is a structural check, not a flow analysis

The subset has no branching, so a body either ends in a `return` or it does not. The check is: if
the declared return type is not unit, the last statement must be a return. No reachability
analysis, no CFG — and when control flow eventually lands, this rule is where it will have to grow
into one, which is the right place for that to happen.

### D4. The CLI takes `--emit`, and stays a thin wrapper

`--emit summary|ir|rust`, defaulting to `summary`, plus `--backend`. Emitted output goes to stdout
and diagnostics to stderr, so `compylr --emit rust f.py > out.rs` produces a file rather than a
file with an error message in it.

It stays a thin wrapper over the library: no logic that could disagree with what the decorator
does. A user diagnosing a rejection must get the same message from both, or the CLI becomes a
source of confusion rather than a way out of it.

*Alternative considered:* an argument-parsing dependency. Rejected — four flags do not justify one,
and the crate's dependency surface is currently "the vendored ruff tree plus PyO3 and serde".

### D5. Project root discovery walks upward for a marker

Markers, in order: an existing `.compylr/`, then `pyproject.toml`. An existing artifact directory
wins because a project that has been built once should keep using what it built, even if it also
has a `pyproject.toml` further up.

The walk stops at the filesystem root and falls back to the working directory. An explicit root
passed by a caller skips discovery entirely — tests depend on that, and so does anyone who wants
artifacts somewhere specific.

*Alternative considered:* `.git` as a marker. Rejected — a monorepo holding several projects would
collapse them into one artifact directory, which is worse than the problem being fixed.

## Risks / Trade-offs

* **Order-independence could regress silently** → It is currently guaranteed by lowering resolving
  nothing; afterwards it is guaranteed by an invariant of the signature pass. Tests must assert it
  directly — both orderings of a mutually-referencing pair producing identical IR — rather than
  relying on it holding by construction.
* **A previously-passing program can now fail earlier** → A call to a nonexistent function moves
  from validation to lowering. The message improves and gains a location, but anything asserting on
  the old failure point changes. Called out as BREAKING in the proposal.
* **Recursion types but may not emit correctly** → With signatures available, a self-recursive
  function passes lowering. Whether the generated Rust compiles and terminates is a separate
  question; a test must answer it rather than assuming the backend copes.
* **Root discovery changes where artifacts appear** → For anyone who has built in a subdirectory,
  the next run uses a different directory and rebuilds once. Harmless, and worth a line in the
  README so it is not mistaken for a cache bug.

## Migration Plan

No artifact format changes and no fingerprints move, so existing caches stay valid. The one
behavioral reversal — call initializers no longer needing an annotation — only accepts programs
that were previously rejected. Annotations that are now redundant keep working; nothing has to be
rewritten.

`python/fixtures/rejected/unannotated_from_call.py` asserts the behavior being reversed and moves
to `accepted/` as part of the change.
