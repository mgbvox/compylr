## 1. IR forms

- [ ] 1.1 Write tests asserting element assignment, membership, and append are representable, and that `not in` is the negation of a membership test rather than its own form
- [ ] 1.2 Write a test asserting `walk_calls` descends into every new form
- [ ] 1.3 Write a test asserting validation does not try to resolve `append` as a function in the unit
- [ ] 1.4 Add `Stmt::SetItem`, `Stmt::Append`, and `Expr::Contains`, and extend `walk_calls`
- [ ] 1.5 Write round-trip tests for the new forms and extend serialization

## 2. Element assignment

- [ ] 2.1 Write tests asserting sequence and mapping element assignment lower
- [ ] 2.2 Write tests asserting a wrong value type and a wrong index type are each rejected reporting both types
- [ ] 2.3 Write a test asserting promotion applies to the assigned value
- [ ] 2.4 Write tests asserting assignment into a tuple and into a set are rejected
- [ ] 2.5 Implement element-assignment lowering

## 3. Mutation is confined to locals

- [ ] 3.1 Write tests asserting a local collection may be appended to and assigned into
- [ ] 3.2 Write tests asserting appending to, and assigning into, a **parameter** are rejected
- [ ] 3.3 Write a test asserting the diagnostic explains the parameter is a copy and the caller would not observe it, per design.md D1 — a refusal without the reason leaves the user no workaround
- [ ] 3.4 Write a test asserting reading a parameter is unaffected
- [ ] 3.5 Write a test asserting a local bound from a parameter may be mutated, since it is the function's own value
- [ ] 3.6 Implement the parameter rule

## 4. Append

- [ ] 4.1 Write tests asserting append lowers, and that a wrong element type, wrong arity, and a non-sequence receiver are each rejected
- [ ] 4.2 Write a test asserting another method is rejected with a diagnostic naming it
- [ ] 4.3 Implement append lowering per design.md D5

## 5. Membership

- [ ] 5.1 Write tests asserting membership over a sequence, mapping, set, and string each yields a boolean
- [ ] 5.2 Write a test asserting mapping membership tests **keys**, matching Python
- [ ] 5.3 Write tests asserting `not in` yields a boolean, a mismatched value type is rejected, and membership in a scalar is rejected
- [ ] 5.4 Implement membership lowering

## 6. Backend: mutation

- [ ] 6.1 Write executable tests asserting appending in a loop accumulates, and that an element assignment is observed by a later read — assert on **values**, never on emitted text, per design.md's second risk
- [ ] 6.2 Write tests asserting a mutated local is bound mutably and an unmutated one is not
- [ ] 6.3 Write an executable test asserting mutation and reading compose, so a mutated collection can still be measured
- [ ] 6.4 Extend the assignment-target scan to cover element assignment and append, and suppress the clone for those names, per design.md D2
- [ ] 6.5 Implement mutation emission

## 7. Backend: insertion and membership

- [ ] 7.1 Write executable tests asserting assigning a new key creates it, assigning an existing key replaces it, and reading a missing key still fails
- [ ] 7.2 Write executable tests for membership over each container, including a mapping testing keys and a string testing substrings
- [ ] 7.3 Write an executable test asserting `not in` is the negation, and that membership does not consume the container
- [ ] 7.4 Add `PyContains` to the emitted runtime per design.md D4 and emit insertion separately from the checked read

## 8. Fixtures and end to end

- [ ] 8.1 Add accepted fixtures covering building a sequence in a loop, mapping insertion, and membership over each container
- [ ] 8.2 Add rejected fixtures for mutating a parameter, appending to a mapping, an unsupported method, and membership with a mismatched type
- [ ] 8.3 Update the rejection table and fixture-count guard
- [ ] 8.4 Write a pytest asserting a caller's list is unchanged after being passed to a compiled function
- [ ] 8.5 Write a pytest asserting a compiled function that builds a collection returns its contents correctly
- [ ] 8.6 Write a pytest exercising a cache-shaped function: membership, read, and insert over a local mapping — the shape the memoized demo needs

## 9. Verification

- [ ] 9.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test` twice
- [ ] 9.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`; coverage with the venv deactivated
- [ ] 9.3 Confirm Rust coverage over `src/` still exceeds 80%
- [ ] 9.4 Update the README's supported-subset section, stating the parameter rule and why it exists
- [ ] 9.5 Update `CLAUDE.md`'s current state and known gaps, recording that mutating while iterating is not yet rejected
- [ ] 9.6 Run `openspec validate add-collection-mutation --strict` and confirm every scenario in all four delta specs has a passing test
