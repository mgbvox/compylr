## 1. The IR carries a docstring

- [x] 1.1 Write a test asserting `Function` can hold a docstring and that its absence is representable
- [x] 1.2 Write tests asserting the fingerprint is unchanged by adding a docstring, and unchanged by editing one, per design.md D2
- [x] 1.3 Write a test asserting the docstring survives an artifact round trip, so the on-disk IR is readable without losing it
- [x] 1.4 Add `doc: Option<String>` to `Function`, excluded from `fingerprint` and included in serialization
- [x] 1.5 Update every `Function` construction site in the crate and tests, and confirm `cargo test` compiles

## 2. Lowering accepts a leading docstring

- [x] 2.1 Write a test asserting a function whose first statement is a string literal lowers
- [x] 2.2 Write a test asserting the docstring does not appear as a statement in the IR body
- [x] 2.3 Write a test asserting the docstring's text is retained on the IR function
- [x] 2.4 Write a test asserting a function annotated `-> None` whose body is only a docstring lowers and produces no value
- [x] 2.5 Write tests asserting the exception stays narrow: a string statement in second position, a non-string bare expression, and a bare call are each still rejected
- [x] 2.6 Write a test asserting a module-level string literal is still rejected, since the exception is body-only
- [x] 2.7 Write a test asserting an f-string in first position is rejected, matching Python, where an f-string is not a docstring
- [x] 2.8 Write a test asserting adjacent concatenated literals in first position are accepted as one docstring
- [x] 2.9 Implement the positional check in body lowering per design.md D3, capturing the text onto the function

## 3. The backend emits it

- [x] 3.1 Write a test asserting a documented function emits a doc comment carrying the text
- [x] 3.2 Write a test asserting a function without a docstring emits no doc comment
- [x] 3.3 Write a test asserting a multi-line docstring emits one comment line per line and still compiles
- [x] 3.4 Write a test asserting a docstring containing a newline, `*/`, and a backslash cannot break out of its comment, and that the result compiles
- [x] 3.5 Write a test asserting emission stays byte-identical across runs for a documented function
- [x] 3.6 Implement doc-comment emission in `src/backend/rust.rs` per design.md D4
- [x] 3.7 Review the regenerated emission snapshots, confirming the comments appear where expected

## 4. End to end

- [x] 4.1 Remove the `strict` xfail from `python/tests/test_api.py::test_a_docstring_does_not_prevent_compilation` and confirm it now passes
- [x] 4.2 Write a pytest asserting a documented function compiles, runs, and returns the same result as the interpreted original
- [x] 4.3 Write a pytest asserting `__doc__` is readable on the marked function
- [x] 4.4 Add an accepted fixture with a documented function, and a rejected fixture for a bare expression statement
- [x] 4.5 Update the rejection table and the fixture-count guard in `tests/fixtures.rs`

## 5. Verification

- [x] 5.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test`
- [x] 5.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`
- [x] 5.3 Confirm Rust coverage over `src/` still exceeds 80%
- [x] 5.4 Update the README's supported-subset section to say docstrings are permitted, and `CLAUDE.md` to drop the docstring entry from its known gaps
- [x] 5.5 Run `openspec validate add-docstring-support --strict` and confirm every scenario in both delta specs has a passing test
