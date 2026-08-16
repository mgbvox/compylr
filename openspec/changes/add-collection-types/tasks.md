## 1. The type model becomes recursive

- [ ] 1.1 Write tests asserting sequence, mapping, set, and tuple types are constructible, nest to any depth, and compare unequal when their parameters differ
- [ ] 1.2 Write tests asserting a mapping keyed by float and a set of float cannot be constructed, per design.md D2
- [ ] 1.3 Write a test asserting a mapping *value* of float is fine, since only keys and set elements need hashing
- [ ] 1.4 Add the four parameterised variants to `Ty`, dropping `Copy` and keeping `Eq`, `Hash`, and `Ord`
- [ ] 1.5 Update every site that passed `Ty` by value to borrow or clone; land this as its own commit so the mechanical churn stays separable from the feature
- [ ] 1.6 Confirm `cargo test` passes with no behavior change before any new syntax is accepted

## 2. IR expression forms

- [ ] 2.1 Write tests for the new expression forms: sequence, mapping, set, and tuple literals; subscript; and length
- [ ] 2.2 Write a test asserting `Expr::Len` is distinct from a call, so validation does not try to resolve it as a callee
- [ ] 2.3 Write tests asserting `walk_calls` descends into collection literals, subscripts, and length operands, so a call nested inside one is still validated
- [ ] 2.4 Add the new `Expr` variants and extend `walk_calls`
- [ ] 2.5 Write round-trip tests for every new type and expression form, including nested types, and assert the artifact names no Rust spelling
- [ ] 2.6 Write a test asserting serialization of a unit using collections stays byte-identical across runs
- [ ] 2.7 Extend serialization to cover the new forms

## 3. Annotations

- [ ] 3.1 Write tests asserting `list[int]`, `dict[str, int]`, `set[int]`, and `tuple[int, str]` are accepted as parameter, return, and local annotations
- [ ] 3.2 Write a test asserting `dict[str, list[int]]` nests correctly
- [ ] 3.3 Write tests asserting bare `list`, `dict[str]`, and `list[complex]` are each rejected with a diagnostic naming the problem
- [ ] 3.4 Write tests asserting `dict[float, int]` and `set[float]` are rejected explaining that a float cannot be a key or set element
- [ ] 3.5 Write a test asserting `frozenset[int]` is still rejected as an unsupported generic
- [ ] 3.6 Implement subscripted-annotation parsing in `lower_annotation`, recursing into parameters

## 4. Literal typing

- [ ] 4.1 Write tests asserting sequence, mapping, set, and tuple literals infer their types
- [ ] 4.2 Write tests asserting mismatched sequence elements and mismatched mapping values are rejected reporting both types
- [ ] 4.3 Write a test asserting a set literal de-duplicates like Python's, so `{1, 2, 2}` has two elements
- [ ] 4.4 Write a test asserting numeric promotion applies inside a literal, with the integer elements carrying explicit conversion nodes
- [ ] 4.5 Write tests asserting an empty literal requires an annotation and that an annotated one is accepted, per design.md D8
- [ ] 4.6 Write a test asserting a literal conflicting with its annotation is rejected
- [ ] 4.7 Implement literal lowering and element unification

## 5. Subscript and length typing

- [ ] 5.1 Write tests asserting a sequence subscript yields the element type, a mapping subscript the value type, and a tuple subscript the type at that position
- [ ] 5.2 Write tests asserting a wrong key type, a non-integer sequence index, and subscripting a set are each rejected
- [ ] 5.3 Write tests asserting a computed tuple index and an out-of-range tuple index are rejected, per design.md D4
- [ ] 5.4 Write a test asserting slicing is rejected
- [ ] 5.5 Write a test asserting subscripts compose, so `d["a"][0]` types correctly
- [ ] 5.6 Write tests asserting `len` types as an integer for every collection and for a string, and is rejected for a number and for the wrong argument count
- [ ] 5.7 Write tests asserting a unit function named `len` is rejected, and that `len(xs)` validates without a `len` function present
- [ ] 5.8 Implement subscript and length lowering, and reserve the `len` name

## 6. Backend: spellings and literals

- [ ] 6.1 Write tests asserting each collection type emits its Rust spelling, and that nested types spell recursively
- [ ] 6.2 Write executable tests asserting each literal constructs the expected value, including a set literal de-duplicating
- [ ] 6.3 Write an executable test asserting an empty literal is constructed from its declared type
- [ ] 6.4 Write an executable test asserting nested literals preserve their nesting
- [ ] 6.5 Implement collection spellings and literal emission

## 7. Backend: indexing, length, and failures

- [ ] 7.1 Write executable tests for negative indices: `xs[-1]` is the last element and `xs[-3]` the first of three
- [ ] 7.2 Write executable tests asserting an index past either end returns a recoverable error rather than panicking
- [ ] 7.3 Write an executable test asserting a missing mapping key returns a recoverable error naming the key
- [ ] 7.4 Write a test asserting a tuple index is resolved at emission and cannot fail at runtime
- [ ] 7.5 Write executable tests asserting `len` matches Python for each collection, **and for a non-ASCII string**, per design.md D5 — this is the case that catches a byte count
- [ ] 7.6 Write a test asserting an index failure propagates through a nested call
- [ ] 7.7 Add `IndexOutOfRange` and `MissingKey` to the emitted runtime, with the indexing helper from design.md D4
- [ ] 7.8 Implement subscript and length emission

## 8. Backend: move safety

- [ ] 8.1 Write tests asserting a sequence parameter read twice, passed to a call and then read, and read then returned, all compile
- [ ] 8.2 Generalise the clone rule from "expected type is `Str`" to "type is not trivially copyable", per design.md D6
- [ ] 8.3 Write a test lowering and emitting every accepted fixture and compiling the result with warnings denied
- [ ] 8.4 Review the regenerated emission snapshots

## 9. Bindings

- [ ] 9.1 Write tests asserting each collection type round-trips across the boundary, including nested collections
- [ ] 9.2 Write a test asserting a `tuple` return arrives as a Python `tuple` rather than a `list`
- [ ] 9.3 Write tests asserting a wrong collection kind, a wrong element type, and a wrong tuple length each raise `TypeError`
- [ ] 9.4 Write tests asserting an out-of-range index raises `IndexError` and a missing key raises `KeyError`, that the process survives both, and that they propagate through nested calls
- [ ] 9.5 Write a test asserting a caller's list is unchanged after being passed to a compiled function
- [ ] 9.6 Write a test asserting a returned mapping has the right contents, deliberately **not** asserting key order, per design.md D7
- [ ] 9.7 Write a test asserting a returned sequence and tuple DO preserve order, so the ordering caveat is scoped to mappings and sets
- [ ] 9.8 Map the two new runtime errors onto `IndexError` and `KeyError` in the binding layer

## 10. End to end

- [ ] 10.1 Add accepted fixtures covering each collection type, literals, subscripting, and `len`
- [ ] 10.2 Add rejected fixtures for the new rules: bare `list`, `dict[float, int]`, mismatched literal elements, a computed tuple index, slicing, and a function named `len`
- [ ] 10.3 Update the rejection table and fixture-count guard in `tests/fixtures.rs`
- [ ] 10.4 Write a pytest comparing compiled and interpreted results over a table of collection inputs, including negative indices and a non-ASCII string
- [ ] 10.5 Measure and record the boundary conversion cost for a large list, so the O(n) caveat in design.md is a number

## 11. Verification

- [ ] 11.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test`
- [ ] 11.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`
- [ ] 11.3 Confirm Rust coverage over `src/` still exceeds 80%
- [ ] 11.4 Update the README's supported-subset table and type list, and `CLAUDE.md`'s current state
- [ ] 11.5 Document the dict-ordering divergence in the README, where a user will find it before it surprises them
- [ ] 11.6 Run `openspec validate add-collection-types --strict` and confirm every scenario in all four delta specs has a passing test
