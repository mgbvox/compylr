# The rejection corpus

Every `.py` file here is a program compylr **refuses**, and each one records *what* it is refused
for. `crates/compylr-host-python/tests/fixtures.rs` holds that record in `REJECTIONS` and checks
it three ways, all derived from this directory rather than from a list beside it:

- every file lowers to a failure, with the recorded diagnostic kind;
- every file has an entry in `REJECTIONS` — a fixture with no recorded rejection is one whose
  refusal nothing is asserting;
- **no file lowers successfully.**

## If a fixture here starts lowering

That is the inverted guard, and the failure is deliberate. Growing the accepted subset is a
decision, and this is what makes it one rather than something that happens quietly.

**Clear it by moving the program into `../accepted/` and giving it a driver in `../drivers/`** —
so the construct that just became supported is exercised against CPython from the moment it is
supported, at both differential tiers. Then remove its row from `REJECTIONS`.

**Never clear it by adding an allowance**, loosening the assertion, or deleting the fixture. Each
of those turns a change in the language into a change in a test, and the corpus stops recording
what the subset refuses.

## These files are not linted, and must not be

They are compiler *input*, and many are deliberately invalid Python. `pyproject.toml` excludes this
directory from ruff and ty, and sets `force-exclude` so that naming a file here on the command line
does not lint it either. That is not tidiness: `ruff check --fix` once deleted the `import os` from
`import_statement.py` — the single line that fixture exists to test.

Drivers live in `../drivers/`, outside both corpora, precisely so that the checks which enumerate
these directories do not find them.
