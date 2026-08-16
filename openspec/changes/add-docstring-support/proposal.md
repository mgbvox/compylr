## Why

`@c.compyle` cannot be applied to a documented function:

```python
@c.compyle
def add(a: int, b: int) -> int:
    """Return the sum."""       # UnsupportedProgramError: 2:5: unsupported statement
    return a + b
```

A docstring is a bare string expression statement, and the subset permits only `return`, `pass`,
and bindings. So the rule that rejects it is working exactly as written — the rule is simply
wrong about this case.

The cost is out of all proportion to the cause. Most code worth compiling is documented, and
house style in this very repository requires docstrings on public items. A user's first attempt
on real code fails, and the diagnostic points at a line they will not think of as a statement at
all. This is the single largest thing standing between the compiler and its first real use, and
the fix is to ignore a string that was never going to do anything.

## What Changes

- **Accept a docstring** — a bare string-literal expression statement in the first position of a
  function body — and carry no runtime meaning for it. Python evaluates and discards it too; the
  interpreter stores it on `__doc__` from the code object, not by executing the statement.
- Keep **every other bare expression statement rejected**. `x + 1` alone on a line still fails.
  A statement whose value is discarded is either dead code or a call made for a side effect the
  subset cannot express, and neither should compile silently. The exception is narrow on purpose:
  first position, string literal, nothing else.
- **Emit the docstring into the generated Rust** as a doc comment on the translated function. It
  costs a few lines and makes the generated source readable, which is the point of writing it to
  disk at all.
- The decorator already preserves `__doc__` on the marked function, so nothing changes there.
  This change removes the rejection; it does not add a new way to reach the docstring.

Explicitly **not** in this change: module-level docstrings (top-level statements other than
function definitions stay rejected), class and attribute docstrings (there are no classes), and
any other bare expression statement.

## Capabilities

### New Capabilities

None — this narrows one existing rejection rule.

### Modified Capabilities

- `ir-lowering`: the "reject constructs outside the subset" rule gains a stated exception for a
  leading docstring, and a new requirement defines exactly what qualifies and what it means.
- `rust-backend`: function emission gains the docstring as a doc comment on the emitted function.

## Impact

- **Code**: `src/lower.rs` (one branch in body lowering), `src/ir.rs` (`Function` gains an
  optional doc string — it is part of the function's identity as written, and the alternative is
  threading it around outside the IR), `src/backend/rust.rs` (emit it).
- **Fingerprints move.** `Function::fingerprint` hashes the function's structure, and adding a
  field means deciding whether a docstring is structure. It is not: editing a comment must not
  trigger a rebuild, and a docstring is a comment that happens to be addressable. Excluding it
  keeps the existing "reformatting does not recompile" guarantee, and matches the decision
  already made for spans.
- **An existing xfail flips.** `python/tests/test_api.py::test_a_docstring_does_not_prevent_compilation`
  is marked `strict=True`, so it fails the suite once the behavior lands and must be unmarked in
  the same change. That is the intended signal, not an obstacle.
- **The rejection fixture count changes.** `tests/fixtures.rs` asserts an exact number of files
  in `python/fixtures/rejected/`, so adding a fixture for the still-rejected bare expression
  statement requires updating that guard.
- **Ordering**: independent of the other two proposed changes. It touches one lowering branch and
  one emission site, and can land first.
