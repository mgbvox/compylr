## Context

See proposal.md — Why. Three facts from the current code shape the approach:

* Body lowering walks `&[PyStmt]` and matches on statement kind. A bare string literal arrives as
  `Stmt::Expr` holding a `StringLiteral`, and falls into the catch-all that reports "unsupported
  statement".
* `Function::fingerprint` hashes name, parameters, return type, and body. It deliberately excludes
  `span`, and the serialization work established the precedent: *structure is what the fingerprint
  covers, and a byte offset is not structure.*
* The decorator already carries `__doc__` onto the wrapper through `functools.update_wrapper`, so
  nothing on the Python side needs to change to make `f.__doc__` work.

## Goals / Non-Goals

**Goals:**

* Make the narrowest possible exception, so "a discarded expression statement is an error" stays
  true everywhere it should.
* Keep the docstring reachable from the IR, so a backend can emit it without a side channel.
* Leave rebuild behavior unchanged: editing prose must not recompile.

**Non-Goals:**

* Module, class, or attribute docstrings.
* Making `__doc__` on a compiled function come from the compiled module rather than the wrapper.
  It already works through the wrapper; routing it differently would be churn.

## Decisions

### D1. The docstring lives on `Function`, not in the body

`Function` gains `doc: Option<String>`. The alternative — a `Stmt::Docstring` variant — would put
a statement in the body that every consumer must then remember to skip, and every backend would
have to decide independently that it emits nothing. A field is skipped by construction.

### D2. It is excluded from the fingerprint

`Function::fingerprint` does not hash `doc`. A docstring is prose about the function, and the
project's rebuild guarantee is that changes which do not alter meaning do not cost a recompile.
Including it would mean fixing a typo in documentation triggers a full crate build.

This follows the same reasoning as spans, and the same consequence applies: the serialized
artifact and the fingerprint agree about what "structure" means, so `doc` is serialized (it is
useful when reading the artifact) but excluded from the fingerprint, exactly as the round-trip
tests already assert for other non-structural data.

*Alternative considered:* hash it, on the grounds that the emitted Rust changes when the docstring
changes and a cached build would then carry stale documentation. Rejected — the staleness is
invisible in behavior, and paying a full rebuild for a comment is the worse trade. The generated
source is regenerated whenever anything else changes.

### D3. Recognised positionally, during lowering, not by a pre-pass

The check is: this is the first statement in the body, and it is an expression statement whose
expression is a string literal. Both conditions are available where the body walk already stands,
so there is no separate scan.

Concatenated adjacent literals (`"a" "b"`) parse as a single `StringLiteral`, so they are covered
without special handling. An f-string does not — it parses as a different node — and is therefore
rejected, which is correct: an f-string docstring is not a docstring to Python either.

### D4. Emitted as `///` lines, with the text made comment-safe

Each line of the docstring becomes a `///` line. Two hazards, both handled by escaping rather
than by trusting input:

* A line containing `*/` is harmless in `///` comments but would matter if the emission ever moved
  to block comments; escaping is done at the source so the choice of comment style stays free.
* A docstring is arbitrary user text and must not be able to terminate the comment or introduce
  code. Every line is prefixed, and any carriage returns are normalised, so the emitted comment
  cannot span into code.

PyO3 lifts a `///` doc comment onto the generated function's `__doc__`, so the compiled function
gains its documentation for free. That is a side benefit rather than the reason: `__doc__` already
works through the wrapper.

## Risks / Trade-offs

* **The narrow rule surprises someone eventually** → A user writing a second string statement, or
  a string after a binding, gets a rejection. The diagnostic should say the statement is
  unsupported rather than mentioning docstrings, because the problem is the discarded expression,
  not the position.
* **`Function` gains a field that most code ignores** → Every construction site must supply it.
  Tests build `Function` literally in several files, so this is mechanical churn, caught at compile
  time rather than at runtime.
* **A strict xfail flips to a failure** → Intended. `test_a_docstring_does_not_prevent_compilation`
  is `strict=True` precisely so that fixing this cannot go unnoticed; unmarking it is part of the
  work, not a surprise.

## Migration Plan

Nothing to migrate. The change only accepts programs that were previously rejected, so no existing
program's meaning changes and no artifact needs regenerating. Fingerprints are unaffected by
construction (D2), so caches stay valid across the upgrade.
