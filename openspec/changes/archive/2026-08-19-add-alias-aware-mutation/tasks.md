## 1. Origin tracking

- [x] 1.1 Write tests asserting a local bound to a collection parameter records that parameter as its origin, and that a local bound to a literal, a call, or an expression records none
- [x] 1.2 Write a test asserting origin propagates through a second binding, so the relation is transitive
- [x] 1.3 Write a test asserting a local bound to a **scalar** parameter records no origin, per design.md D3
- [x] 1.4 Write a test asserting reassigning a local to a fresh collection clears its origin
- [x] 1.5 Store the origin beside the type in the scope frame, per design.md D2

## 2. The widened rejection

- [x] 2.1 Write tests asserting appending to, and assigning into, a local that aliases a parameter are both rejected
- [x] 2.2 Write a test asserting the rejection survives a chain of two bindings
- [x] 2.3 Write a test asserting the diagnostic names **both** the local and the parameter it came from — a refusal pointing only at the local gives the user no reason to look at the signature
- [x] 2.4 Write a test asserting a local built fresh and filled from a parameter may be mutated
- [x] 2.5 Write a test asserting a local that has been rebound away from a parameter may be mutated
- [x] 2.6 Write a test asserting reading through an alias is unaffected
- [x] 2.7 Extend the mutation check to consult the origin

## 3. Reversing the previous behaviour

- [x] 3.1 Replace the `a_local_bound_from_a_parameter_may_be_mutated` test in `tests/mutation_lowering.rs` with its inverse
- [x] 3.2 Flip `python/tests/test_mutation.py`'s divergence tests: the compiled and interpreted forms must now agree, because the program that made them differ no longer compiles
- [x] 3.3 Replace `_replace_first` in that suite with a shape that is still accepted, so the by-value property is still exercised
- [x] 3.4 Add a rejected fixture for mutating an alias, and update the rejection table and count guard

## 4. Verification

- [x] 4.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test` twice
- [x] 4.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`; coverage with the venv deactivated
- [x] 4.3 Confirm Rust coverage over `src/` still exceeds 80%
- [x] 4.4 Update the README's mutation section: the rule covers aliases, and the workaround is an explicit copy
- [x] 4.5 Remove the alias hole from `CLAUDE.md`'s known gaps, since it is no longer one
- [x] 4.6 Run `openspec validate add-alias-aware-mutation --strict` and confirm every scenario has a passing test
