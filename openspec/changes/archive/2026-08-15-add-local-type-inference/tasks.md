## 1. Float in the type model

- [x] 1.1 Write tests asserting `Ty::Float` is distinct from `Ty::Int`, that `python_name()` returns `float`, and that all five types remain comparable
- [x] 1.2 Add `Ty::Float` to `src/ir.rs`, documented as a 64-bit binary floating-point number with no target-language spelling
- [x] 1.3 Write tests for `Literal::float`: round-tripping through `as_f64`, equality of two literals written identically, and that `Literal::Float` participates in `Hash` so a function containing one can be fingerprinted
- [x] 1.4 Write a test asserting `0.0` and `-0.0` are distinguishable, pinning the documented bitwise-comparison decision so a later change cannot silently alter it
- [x] 1.5 Implement `Literal::Float(u64)` storing `f64::to_bits()`, with a `float(f64)` constructor and an `as_f64()` accessor, keeping every `Eq`/`Hash` derive intact
- [x] 1.6 Verify `cargo test` passes and that `Function::fingerprint` still compiles for a body containing a float literal

## 2. True division and the promotion node

- [x] 2.1 Write tests asserting `BinOp::TrueDiv` is distinct from `BinOp::FloorDiv` and that `python_symbol()` returns `/`
- [x] 2.2 Add `BinOp::TrueDiv` to `src/ir.rs`, documenting that it always yields a float even for two integer operands, unlike the same spelling in most target languages
- [x] 2.3 Write tests for `Expr::ToFloat`: it wraps an expression, nests, and takes part in structural equality and fingerprinting
- [x] 2.4 Implement `Expr::ToFloat(Box<Expr>)` as the explicit widening node, and extend `Expr::walk_calls` to descend through it so a call nested under a promotion is still found by `Unit::validate`

## 3. Expression type checker

- [x] 3.1 Write tests for literal typing: integer, float, boolean, and string literals each report their own type
- [x] 3.2 Write tests for name typing: a bound name reports its scope type; an unbound name is still `Unresolved` rather than a type error
- [x] 3.3 Write tests for negation: numeric operands preserve their type; a string or boolean operand is rejected
- [x] 3.4 Write tests for arithmetic typing: int/int is int, float/float is float, `str + str` is str, and `str + int`, boolean arithmetic, and `str - str` are each rejected reporting operand types
- [x] 3.5 Write tests asserting true division yields float for two integer operands, while floor division of the same operands yields int
- [x] 3.6 Write tests for comparison typing: every supported comparison yields bool; comparing a string with an integer is rejected
- [x] 3.7 Write tests for promotion: mixed int/float arithmetic yields float, mixed comparison yields bool, and the integer operand is wrapped in a `ToFloat` node in the resulting IR
- [x] 3.8 Write tests asserting an expression containing a call reports an undetermined type rather than an error, including a call nested inside arithmetic — the case a naive implementation would misreport as a type mismatch
- [x] 3.9 Change `lower_expr` to return `(Expr, Option<Ty>)`, threading the type alongside the lowered node in one traversal
- [x] 3.10 Implement the operator type table (arithmetic, true division, string concatenation, comparisons) with `None` propagating outward through every combining rule
- [x] 3.11 Implement promotion, inserting `ToFloat` around the integer operand whenever one side is float

## 4. Binding inference

- [x] 4.1 Write tests for the target cases: `a = "x"`, `b = 3`, `c = 1.3`, `d = True` each infer without an annotation
- [x] 4.2 Write tests asserting inference composes: `b = a + 1`, `b = a < 10`, `b = -c`, and `b = (a + 1) * 2 - 3` all infer from an annotated parameter
- [x] 4.3 Write tests asserting the existing alias cases still work, including chained aliases, now as a case of the general rule rather than a special path
- [x] 4.4 Write tests asserting an initializer containing a call still requires an annotation, and that aliasing an unbound name still reports `Unresolved`
- [x] 4.5 Replace the alias-only branch in `lower_bare_binding` with general inference: infer when the initializer's type is determined, and emit `MissingAnnotation` naming the variable when it is not
- [x] 4.6 Verify the diagnostic for an undetermined initializer explains that a call's type is not known during lowering, so the message guides toward adding an annotation rather than implying the code is unsupported

## 5. Declared-versus-inferred checking

- [x] 5.1 Write tests asserting `b: str = 1` and `b: str = a` (integer `a`) are both rejected as `TypeMismatch` reporting declared and actual types
- [x] 5.2 Write tests asserting `c: float = 1` is accepted via promotion while `n: int = 1.5` is rejected as narrowing
- [x] 5.3 Write tests asserting `b: int = helper(a)` is accepted, since an undetermined initializer cannot be checked
- [x] 5.4 Implement the declared-versus-inferred check in `lower_annotated_binding`, replacing the alias-only comparison and applying promotion rules
- [x] 5.5 Write tests for return checking: `def f() -> int: return "x"` is rejected; `def f() -> None: return 1` is rejected; `def f() -> float: return 1` is accepted via promotion; a returned call expression is not checked
- [x] 5.6 Implement return-type checking in statement lowering, threading the declared return type into the body walk

## 6. Annotations and subset updates

- [x] 6.1 Write tests asserting `float` is accepted as a parameter, return, and local annotation, and that `complex` is now the example of a rejected scalar annotation
- [x] 6.2 Accept `float` in `lower_annotation` and update the rejection tests that used `float` as their unsupported example
- [x] 6.3 Write tests asserting `/` lowers successfully and that exponentiation and bitwise operators are still rejected
- [x] 6.4 Map ruff's `Operator::Div` to `BinOp::TrueDiv` in expression lowering, removing the rejection branch

## 7. Fixture migration

- [x] 7.1 Move `python/fixtures/rejected/unsupported_type_float.py` and `true_division.py` into `accepted/`, since both now lower, and extend them to exercise float arithmetic and division
- [x] 7.2 Replace `rejected/unannotated_local.py` and `unannotated_local_from_expression.py` with a fixture that is still genuinely rejected — an unannotated binding from a call
- [x] 7.3 Add rejected fixtures for the new ill-typed cases: `str + int`, boolean arithmetic, comparing unrelated types, negating a string, `n: int = 1.5`, and a return conflicting with the declared type
- [x] 7.4 Add an accepted fixture covering the proposal's motivating example (`a = "x"`, `b = 3`, `c = 1.3`) plus promotion and true division
- [x] 7.5 Update the rejection table and the fixture-count guard in `tests/fixtures.rs` so a stale table fails the build rather than silently skipping a rule
- [x] 7.6 Review the regenerated insta snapshots, confirming inferred types and `ToFloat` nodes appear where expected rather than accepting them blindly

## 8. Verification

- [x] 8.1 Run `cargo fmt` and `cargo clippy -p compylr --all-targets -- -D warnings`, resolving all findings
- [x] 8.2 Run `cargo test` and confirm every scenario in both delta specs has a corresponding passing test
- [x] 8.3 Confirm coverage over `src/` still exceeds 80%, adding tests for any uncovered branch in the new type checker
- [x] 8.4 Run `openspec validate add-local-type-inference --strict` and resolve any reported issues
- [x] 8.5 Verify the binary still runs end to end on an accepted fixture and reports a type error with `line:column` on a rejected one
