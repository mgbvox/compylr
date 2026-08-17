## 1. IR forms

- [ ] 1.1 Write tests asserting each new statement form is representable, that `elif` nests, and that bodies nest to any depth
- [ ] 1.2 Write tests asserting a range carries start, stop, and step even when the source omitted them
- [ ] 1.3 Write tests asserting `walk_calls` descends into branch bodies, loop bodies, loop tests, and range components, so a call nested in one is still validated
- [ ] 1.4 Add `Stmt::If`, `Stmt::While`, `Stmt::For`, `Stmt::Break`, `Stmt::Continue`, and `Expr::Range`
- [ ] 1.5 Extend `walk_calls`
- [ ] 1.6 Write round-trip tests for every new form including nesting, and assert the artifact names no target syntax
- [ ] 1.7 Extend serialization

## 2. Scope becomes a stack

- [ ] 2.1 Write tests asserting a name bound in a branch is not visible after it, and that the diagnostic says the binding may not have happened rather than that the name is unknown
- [ ] 2.2 Write a test asserting a loop variable does not escape its loop
- [ ] 2.3 Write a test asserting an assignment inside a loop updates a counter declared outside it rather than shadowing it, per design.md D2
- [ ] 2.4 Replace the flat `Scope` with a stack of frames, pushing on block entry and popping on exit
- [ ] 2.5 Confirm every existing lowering test passes unchanged, since a flat function body is a single frame

## 3. Reassignment

- [ ] 3.1 Write tests asserting a rebinding keeps the original type, that a different type is rejected reporting both, and that promotion applies
- [ ] 3.2 Write a test asserting an annotation on a rebinding is rejected as a re-declaration
- [ ] 3.3 Write a test asserting a parameter may be reassigned and keeps its declared type
- [ ] 3.4 Replace `ensure_unbound` with the rebinding rule from design.md D3
- [ ] 3.5 Move the existing `rebind_local.py` fixture from `rejected/` to `accepted/`, since it asserts the behaviour this reverses, and update the rejection table and count guard

## 4. Conditionals

- [ ] 4.1 Write tests asserting `if`, `if`/`else`, and `if`/`elif`/`else` lower, with `elif` nesting
- [ ] 4.2 Write a test asserting a non-boolean test is rejected reporting the type — compylr does not infer truthiness from an integer
- [ ] 4.3 Implement conditional lowering

## 5. Reachability

- [ ] 5.1 Write tests for each rule in design.md D1: both branches returning is accepted; one branch is not; no alternative is not; a return after a conditional covers it; a loop never counts
- [ ] 5.2 Write a test asserting nested conditionals are analysed through
- [ ] 5.3 Write a test asserting the diagnostic says a path produces no value, so the fix is obvious
- [ ] 5.4 Replace the structural last-statement check with the recursive analysis
- [ ] 5.5 Confirm every existing accepted fixture still lowers

## 6. Loops

- [ ] 6.1 Write tests asserting `while` lowers and that a non-boolean test is rejected
- [ ] 6.2 Write tests asserting `for` binds an integer over a range, an element type over a sequence, a **key** type over a mapping, and an element type over a set
- [ ] 6.3 Write a test asserting iterating a scalar is rejected
- [ ] 6.4 Write tests asserting `break`/`continue` lower inside a loop, are rejected outside one, and reach the nearest enclosing loop from inside a conditional
- [ ] 6.5 Implement loop lowering with the loop-depth context from design.md D6

## 7. range

- [ ] 7.1 Write tests asserting one, two, and three argument forms fill in start/stop/step
- [ ] 7.2 Write tests asserting a non-integer argument and a wrong arity are rejected
- [ ] 7.3 Write a test asserting a function named `range` is rejected as reserved
- [ ] 7.4 Write a test asserting a bare `range(n)` outside a loop is rejected
- [ ] 7.5 Implement `range` lowering and reserve the name

## 8. Backend: branches and loops

- [ ] 8.1 Write executable tests asserting a conditional runs the matching branch, that one with no alternative continues past it, and that nesting behaves
- [ ] 8.2 Write executable tests asserting a `while` counts, a loop whose test is false at entry does not run, and `break`/`continue` behave as Python's
- [ ] 8.3 Implement branch and loop emission

## 9. Backend: ranges

- [ ] 9.1 Write executable tests for `range(3)`, `range(2,5)`, `range(0,6,2)`, and **`range(3,0,-1)`** — the last is the case Rust's `..` cannot express, per design.md D4
- [ ] 9.2 Write an executable test asserting an empty range does not run its body
- [ ] 9.3 Write a test asserting a zero step returns a recoverable error rather than hanging
- [ ] 9.4 Add `ZeroStep` to the emitted runtime with the range helper, and emit range iteration as a loop

## 10. Backend: iteration and mutability

- [ ] 10.1 Write executable tests asserting sequence order is preserved, a mapping yields keys, and a collection can be iterated then read again
- [ ] 10.2 Write a test asserting mapping and set iteration order is not asserted anywhere, so the suite does not itself become flaky
- [ ] 10.3 Write tests asserting a rebound local compiles, a once-bound local is not marked mutable, and a reassigned parameter compiles
- [ ] 10.4 Implement the assignment-target scan from design.md D3 and emit `mut` accordingly
- [ ] 10.5 Write a test compiling every accepted fixture with warnings denied

## 11. Fixtures and end to end

- [ ] 11.1 Add accepted fixtures covering branching, both loop forms, loop control, ranges including a negative step, and reassignment
- [ ] 11.2 Add rejected fixtures for a non-boolean test, `break` outside a loop, a branch-bound name read after the conditional, a function that returns on only one path, and a function named `range`
- [ ] 11.3 Update the rejection table and fixture-count guard
- [ ] 11.4 Write a pytest comparing compiled and interpreted results for a recursive function with a base case — the shape the demo needs
- [ ] 11.5 Write a pytest comparing an iterative loop with a counter against its interpreted original

## 12. Verification

- [ ] 12.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test` twice, confirming the suite is stable across runs
- [ ] 12.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`; run coverage with the venv deactivated
- [ ] 12.3 Confirm Rust coverage over `src/` still exceeds 80%
- [ ] 12.4 Update the README's supported-subset section and `CLAUDE.md`'s current state
- [ ] 12.5 Run `openspec validate add-control-flow --strict` and confirm every scenario in all three delta specs has a passing test
