## 1. Crate scaffolding and dependencies

- [x] 1.1 Add path dependencies on `ruff_text_size` and `ruff_source_file` to `Cargo.toml`, and `insta` as a dev-dependency; confirm `cargo build` resolves against the vendored tree
- [x] 1.2 Convert the crate to a library plus binary: create `src/lib.rs` declaring the `span`, `error`, `frontend`, `ir`, and `lower` modules, and reduce `src/main.rs` to a thin binary that compiles
- [x] 1.3 Delete the obsolete `to_ast`, `compyle`, and `test_basic_python_compilation` scaffolding from `src/main.rs`, along with its teaching comments
- [x] 1.4 Verify `cargo build` and `cargo test` both succeed on the empty skeleton before any behavior is added

## 2. Spans and diagnostics

- [x] 2.1 Write tests for `Span`: construction from a `TextRange`, byte-offset accessors, equality
- [x] 2.2 Implement `Span { start: u32, end: u32 }` in `src/span.rs` with `From<TextRange>`
- [x] 2.3 Write tests for rendering a span as `line:column` via `LineIndex`, covering the first line, a later line, and a position after a multi-byte character
- [x] 2.4 Implement span-to-`line:column` rendering that takes the source text as an argument, keeping `Span` itself source-free

## 3. Frontend

- [x] 3.1 Write tests for `parse_source`: valid module, empty source, and malformed syntax (asserting a syntax-kind failure carrying a span) — covers spec scenarios "Valid Python file is parsed", "Empty file is parsed", "Malformed Python source"
- [x] 3.2 Write tests for `parse_file`: nonexistent path and a path pointing at a directory, each asserting an I/O-kind failure naming the path, and asserting neither panics
- [x] 3.3 Implement `FrontendError` in `src/error.rs` as an enum distinguishing I/O from syntax failures, with hand-written `Display`, `std::error::Error`, and `From<std::io::Error>` / `From<ParseError>` impls
- [x] 3.4 Implement `parse_source(&str) -> Result<Parsed<ModModule>, FrontendError>` and `parse_file(&Path)` delegating to it
- [x] 3.5 Verify the caller can branch on failure kind without matching on message text — covers spec scenario "Caller can distinguish failure kinds"

## 4. IR data model

- [x] 4.1 Write tests asserting `Ty` covers exactly `Int`, `Bool`, `Str`, `Unit` and that each is constructible and comparable
- [x] 4.2 Implement `Ty` in `src/ir.rs`, documenting each variant by semantics (`Int` = 64-bit signed) with no target-language spellings — covers spec requirement "Target-language independence"
- [x] 4.3 Write tests for `Expr` construction and nesting, including a three-level nested arithmetic expression and a two-argument call
- [x] 4.4 Implement `Expr` with literal, name-reference, unary-negation, binary, and call variants; implement `BinOp` naming Python semantics, documenting that `FloorDiv` rounds toward negative infinity and `Mod` takes the divisor's sign
- [x] 4.5 Write tests for `Stmt` covering value return, bare return, and typed local binding
- [x] 4.6 Implement `Stmt` and `Function { name, params, ret, body }` with `Param { name, ty }`; derive `Debug`, `Clone`, `PartialEq`, `Eq`, and `Hash` on all IR types
- [x] 4.7 Write a test asserting an IR value remains usable after its source `String` and `Parsed` are dropped — covers spec scenario "IR outlives its source"

## 5. Unit assembly

- [x] 5.1 Write tests for `Unit`: empty unit, adding three functions from separate sources, adding a fourth to an existing unit, and rejecting a duplicate name — covers all "Unit aggregates functions incrementally" scenarios
- [x] 5.2 Implement `Unit` with an `add_function` that returns an error on duplicate names
- [x] 5.3 Write a test asserting two units built from the same functions in different addition orders expose them in identical order — covers spec scenario "Addition order does not affect unit order"
- [x] 5.4 Implement deterministic name-ordered iteration over the unit's functions

## 6. Fingerprinting

- [x] 6.1 Write tests asserting identical structure yields identical fingerprints, and that changing a body, a parameter type, or a return type each changes the fingerprint
- [x] 6.2 Implement `Function::fingerprint()` derived from the IR structure only
- [x] 6.3 Write tests asserting the unit fingerprint changes when a function is added, is unchanged for existing functions, and is identical across differing addition orders
- [x] 6.4 Implement `Unit::fingerprint()` by sorting member fingerprints before combining, making it order-independent

## 7. Lowering: signatures and types

- [x] 7.1 Write tests for annotation lowering: each of `int`, `bool`, `str` accepted; `None` accepted only as a return type; `float`, `list[int]`, and a bare type variable rejected with the annotation named in the diagnostic
- [x] 7.2 Implement annotation lowering from `Expr` to `Ty`, rejecting anything outside the supported set
- [x] 7.3 Write tests for signature rejection: unannotated parameter, missing return annotation, `*args`/`**kwargs`, keyword-only and positional-only parameters, a parameter with a default, a decorated function, an `async def`, and a function with PEP 695 type params
- [x] 7.4 Implement function-signature lowering enforcing all of the above, reading `posonlyargs`/`vararg`/`kwonlyargs`/`kwarg`/`decorator_list`/`is_async`/`type_params` from `StmtFunctionDef`

## 8. Lowering: statements and expressions

- [x] 8.1 Write tests for expression lowering covering each literal kind, name references, unary minus, every supported arithmetic and comparison operator, and calls
- [x] 8.2 Write a test asserting an integer literal exceeding `i64` is rejected rather than truncated, exercising the `Int::as_i64()` `None` path
- [x] 8.3 Implement expression lowering, mapping ruff `Operator`/`CmpOp` to the IR's `BinOp` and rejecting unsupported operators such as true division and `**`
- [x] 8.4 Write tests for statement lowering: `return <expr>`, bare `return`, `pass`, annotated assignment accepted; unannotated assignment and `if`/`while`/`for` rejected
- [x] 8.5 Implement statement lowering for the three supported forms
- [x] 8.6 Write tests for local name resolution: parameter and prior-local references resolve; unbound names and references before binding are rejected
- [x] 8.7 Implement scope tracking during body lowering so names resolve against parameters plus previously bound locals
- [x] 8.8 Write tests for alias inference: `b = a` from a parameter, from a prior local, and chained (`b = a; c = b`) all infer; literal, arithmetic, and call initializers are still rejected as needing an annotation; aliasing an unbound name reports unresolved rather than missing annotation
- [x] 8.9 Implement alias inference for bare-name initializers by looking the name up in the scope map
- [x] 8.10 Write tests asserting an explicit annotation still wins (`b: int = a`) and that a conflicting annotation (`b: str = a` with integer `a`) is rejected reporting declared and actual types
- [x] 8.11 Implement the declared-vs-aliased type check for annotated bare-name bindings
- [x] 8.12 Write tests asserting rebinding an existing local, and binding over a parameter name, are both rejected as unsupported reassignment
- [x] 8.13 Implement single-assignment enforcement in the scope map
- [x] 8.14 Write tests asserting top-level rejection of imports, class definitions, and the `if __name__ == '__main__':` guard
- [x] 8.15 Implement `lower_source` walking top-level statements, accepting only function definitions and recording call targets by name without resolving them

## 9. Unit validation

- [x] 9.1 Write tests for `Unit::validate`: a call across two separately-lowered sources resolves; resolution succeeds regardless of which function was added first; an unknown callee is rejected; an argument-count mismatch reports expected and actual counts
- [x] 9.2 Implement `Unit::validate` resolving every call target against the assembled unit and checking arity

## 10. Diagnostics coverage

- [x] 10.1 Write a test asserting every diagnostic carries a span locating the offending construct
- [x] 10.2 Write a test asserting that a source containing multiple violations reports the first in source order
- [x] 10.3 Implement `Diagnostic` with a message and span, plus `Display` rendering problem and location, and confirm no lowering path panics on parsed input

## 11. Fixtures and snapshot tests

- [x] 11.1 Create `python/fixtures/accepted/` with samples covering the full supported subset, including a cross-source call pair
- [x] 11.2 Create `python/fixtures/rejected/` with one sample per rejection rule, named after the rule it triggers
- [x] 11.3 Add `insta` snapshot tests rendering the lowered IR for each accepted fixture, and review the generated snapshots for correctness
- [x] 11.4 Add a test asserting `python/entrypoint.py` is rejected for its `__main__` guard, locking in the documented behavior
- [x] 11.5 Add a snapshot test asserting two sources differing only in comments, blank lines, and indentation produce identical fingerprints — covers spec scenario "Identical functions fingerprint identically"

## 12. Verification

- [x] 12.1 Run `cargo fmt` and `cargo clippy -- -D warnings`, resolving all findings
- [x] 12.2 Run `cargo test` and confirm every spec scenario across the three capabilities has a corresponding passing test
- [x] 12.3 Measure coverage and confirm it exceeds 80%, adding tests for any uncovered branch
- [x] 12.4 Run `openspec validate add-python-ir-lowering --strict` and resolve any reported issues
