## Context

See `proposal.md` — Why. The constraints that actually shape this design come from the target
end state in `CLAUDE.md` and the follow-on clarifications:

1. Input arrives as **source text** from `inspect.getsource(fn)`, not as a file path.
2. All decorated functions in a project share **one build artifact**, so the compilation unit
   aggregates functions that were parsed independently.
3. Adding or editing a function must trigger **exactly one rebuild**, so something must decide
   cheaply whether the unit actually changed.
4. Rust is the first backend, but the IR must stay expressible by **Go, C++, and TypeScript**
   backends later.

Current state: `src/main.rs` is a single non-compiling file — `to_ast` propagates `io::Error`
and `ParseError` into a `ToAstError` with no `From` impls, and `compyle` returns `None`. The
vendored ruff tree at `vendored/ruff` supplies the parser via path dependencies. Verified API
surface used below:

- `ruff_python_parser::parse_module(&str) -> Result<Parsed<ModModule>, ParseError>`
- `ParseError { error: ParseErrorType, location: TextRange }`
- `StmtFunctionDef { range, is_async, decorator_list, name: Identifier, type_params,
  parameters: Box<Parameters>, returns: Option<Box<Expr>>, body }`
- `Parameters { posonlyargs, args, vararg, kwonlyargs, kwarg }`, `Parameter { name, annotation }`
- `Operator { Add, Sub, Mult, Div, FloorDiv, Mod, Pow, … }`, `CmpOp { Eq, NotEq, Lt, LtE, Gt, GtE, … }`
- `Int::as_i64() -> Option<i64>` — fallible, hence the out-of-range literal requirement
- `ruff_source_file::LineIndex::from_source_text` / `line_column` for diagnostic rendering

## Goals / Non-Goals

**Goals:**

- A `Unit` that can be built incrementally and validated as a whole.
- Lowering that never panics on parsed input and always reports a located diagnostic.
- An IR carrying no target-language detail, so a backend is a pure function of the IR.
- A fingerprint precise enough to drive rebuild decisions.

**Non-Goals (design level, beyond the proposal's scope list):**

- No backend abstraction. Zero backends exist; a trait with no implementors would be shaped
  by guesswork. The IR staying neutral is what preserves the option.
- No multi-error collection. Lowering stops at the first violation (see Decisions).
- No arena/interning for IR nodes. `Box`/`Vec` ownership is adequate at this size and keeps
  the IR trivially `Clone`, `PartialEq`, and `Hash`.
- No incremental *parsing*. Re-lowering a changed function re-parses its source.

## Decisions

### Source text is the primary input; file reading is a helper

`parse_source(&str)` is the core entry point; `parse_file(&Path)` reads and delegates. The
decorator path never has a file, and tests want fixtures on disk — this satisfies both without
making the core depend on the filesystem.

*Alternative considered:* path-only, matching the existing `to_ast`. Rejected: it forces the
decorator to write `inspect.getsource` output to a temp file purely to satisfy the API.

### Two-phase lowering: per-source lowering, then unit validation

`lower_source(&Parsed<ModModule>) -> Result<Vec<Function>, Diagnostic>` handles everything
decidable from one source: annotations, subset enforcement, local name resolution. Call
*targets* are recorded by name and left unresolved. `Unit::validate()` then resolves calls and
checks arity across the assembled unit.

This falls directly out of constraint 2 — when each function is lowered, the functions it calls
may not have been decorated yet. Deferring only call resolution keeps the per-source phase
independent while still catching unresolved calls before codegen.

*Alternative considered:* resolve calls during lowering against a running registry. Rejected:
it makes lowering order-dependent, so `a` calling `b` would succeed or fail depending on
decoration order — exactly the nondeterminism constraint 3 needs eliminated.

### Fingerprint over IR, not over source text

Each `Function` derives `Hash`; its fingerprint is a hash of the IR structure. The unit's
fingerprint combines member fingerprints **order-independently** (sort function fingerprints,
then hash the sorted sequence).

Keying on source text would rebuild on a comment edit or reformatting; keying on IR rebuilds
only when meaning changes. Order-independence means decoration order and import order don't
churn the key.

*Alternative considered:* hash the rendered Rust output. Rejected: it doesn't exist yet, it
would make the key backend-specific, and it inverts the dependency — the key is needed to
decide whether to *run* the backend.

*Note:* `Hash` gives a 64-bit `u64` key. Collisions are theoretically possible; the
consequence is a skipped rebuild. Acceptable for a local build cache, and the rebuild change
can escalate to a wider digest without touching the IR shape.

### Semantic type model, with Python operator semantics preserved

`Ty` is `{ Int, Bool, Str, Unit }`, documented by semantics (`Int` = 64-bit signed), not by
spelling. Operator variants name *Python* operations: `FloorDiv` means floor-toward-negative-
infinity, `Mod` means sign-of-divisor.

This is the highest-value decision for constraint 4. Rust's `/` truncates toward zero and `%`
takes the sign of the *dividend*, so `-7 // 2` is `-4` in Python but `-7 / 2` is `-3` in Rust.
Naming the IR operator after the Python semantic forces each backend to confront the mismatch
instead of emitting a plausible-looking wrong translation. The Rust backend will need
`div_euclid`-style handling or an explicit floor adjustment; that is its problem, correctly
located.

*Alternative considered:* lower `//` to a target-neutral "divide" and let backends pick.
Rejected: it silently loses the semantic and produces wrong results for negative operands.

### Alias-only inference, and single-assignment locals

A binding whose initializer is a bare `Expr::Name` takes that name's type; everything else
unannotated is rejected. The scope map already holds `name -> Ty` for local resolution, so this
is a lookup, not an inference pass — no unification, no type variables, no ordering constraints.

The line is drawn at "bare name" rather than "any expression whose type we could compute"
because the latter is a slippery slope: once literals are inferred, `b = a + 1` looks arbitrary
to reject, and inferring that requires operator result typing, which requires a real type
checker. Aliasing is the one case where the answer is already written down.

When the binding *is* annotated and the initializer is a bare name, the declared type is checked
against the aliased type and a mismatch is an error. That check is free given the lookup, and
catching `b: str = a` here is strictly better than emitting IR the backend cannot render.

Locals are bound once. Rebinding — including shadowing a parameter — is rejected, so every IR
binding means "introduce a new name" and maps to a plain Rust `let`. Allowing reassignment would
force a decision between `let mut` and shadowing that only matters once mutation is real, and
that decision belongs with the change that adds it.

*Alternative considered:* infer literals too (`x = 1` → int). Rejected: it makes the boundary
arbitrary, and the annotation on a literal binding is the documentation that makes this subset
readable.

### Own `Span`, converted at the boundary

`Span { start: u32, end: u32 }` in byte offsets, converted from ruff's `TextRange` at the
frontend/lowering boundary. Keeps `ruff_text_size` out of the IR's public shape and satisfies
the IR's self-contained requirement. `LineIndex` renders `line:column` at display time, so the
diagnostic itself stays source-free and cheap to compare in tests.

*Alternative considered:* store `TextRange` directly. Rejected: it leaks a ruff type into the
IR contract that other backends and the Python side would have to understand.

### Hand-written error types, no `thiserror`

Two small error enums with hand-written `Display` and `From` impls. This is what the existing
scaffold was reaching for, adds no dependency, and the impls are a few lines each.

*Alternative considered:* `thiserror`. Reasonable and idiomatic; rejected only to keep the
dependency surface at "vendored ruff plus dev-deps". Revisit if error types multiply.

### Fail fast on the first violation

Lowering returns `Result<_, Diagnostic>`, not a diagnostic list. Multi-error reporting needs
error recovery in the walker, which is a real feature with its own design; the specs commit
only to "first violation in source order" so that behavior is deterministic and testable now.

*Trade-off:* a user fixing three errors recompiles three times. Acceptable while the subset is
this small; a later change can widen to `Vec<Diagnostic>` without changing the IR.

## Risks / Trade-offs

- **The decorator's own line appears in `inspect.getsource` output** → This change specs
  decorated functions as *rejected*, which would reject every real input. Deliberate: the
  decorator runtime is responsible for stripping its own decorator before submitting source.
  Recorded here so the later change doesn't discover it as a surprise; the alternative
  (special-casing a `compylr.compyle` decorator inside lowering) would put Python-package
  knowledge into the compiler core.
- **Fingerprint collisions skip a needed rebuild** → 64-bit key over a small unit; mitigated by
  the rebuild change being free to widen the digest.
- **`Ty` is closed and small** → Every later feature (floats, collections, generics) adds a
  variant and touches every `match`. That is the intended trade: exhaustive matching makes the
  compiler tell you where the gaps are, which is worth more than open-ended extensibility here.
- **Source order within a unit is not preserved** → Unit ordering is by name, so codegen output
  won't mirror the user's file. Acceptable and necessary: with functions arriving from many
  sources there is no meaningful global source order, and determinism matters more.
- **Rejecting `pass`-only bodies vs. representing them** → `pass` lowers to a no-value
  statement rather than being dropped, so a `-> None` function with a `pass` body round-trips
  instead of producing an empty body that codegen must special-case.

## Migration Plan

No deployment or data migration — the crate has no users. The existing `to_ast`/`compyle`
free functions and the `test_basic_python_compilation` test are removed rather than deprecated.
`python/entrypoint.py` is retained unchanged and becomes a rejection fixture (its `__main__`
guard is outside the subset), which is the correct behavior to lock in.

Rollback is `git revert`; nothing outside this repo depends on the crate.

## Open Questions

- Should the unit fingerprint incorporate a compiler-version salt, so that upgrading compylr
  invalidates caches built by an older lowering? Deferred safely: it is a one-line change to
  the hash input and belongs with the rebuild machinery that consumes the key, not here.
- Should string literals carry their escape/prefix form (raw, f-string) once more of Python is
  supported? Not relevant while only plain string literals lower successfully.
