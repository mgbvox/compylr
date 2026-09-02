# Audit: python-rust path

## Confirmed defect: `compylr compyle` exit code does not distinguish "failure" as documented

README.md:324-325 claims:
> Three outcomes are distinguishable from the exit status alone: built or reused (`0`), nothing marked (`3`), and failure (`1`, or `2` for a bad root).

The preceding paragraph (README.md:319-320) says import failures are "reported and skipped" — i.e.
a real defect in the user's project (a module that raises on import) is caught, recorded in
`Report.failures`, and precompiling continues.

But `frontends/python/compylr/_precompile.py:312-316`:
```python
quiet = report.found_nothing or report.disabled
stream = sys.stderr if quiet else sys.stdout
print(_describe(report), file=stream)
return 3 if quiet else 0
```
never inspects `report.failures`. Exit code 1 is reachable ONLY via the `except Exception` branch
in `main()` (an exception escaping `precompile()` itself, e.g. a real cargo/maturin build
failure) — never via a module that failed to import.

### Verified by running the actual console-script entry point (`compylr = "compylr._precompile:main"`, pyproject.toml:29)

Case 1 — one good module (marks a function), one module that raises on import:
```
$ compylr compyle <project>
compylr: <project>
  imported 1 module(s); found 1 function(s) and 0 class(es)
  1 module(s) failed to import:
    bad.py: RuntimeError: boom
  built
EXIT CODE: 0
```
A project with a genuinely broken module reports exit code 0 ("built"), indistinguishable from a
fully clean precompile.

Case 2 — every module in the project raises on import (nothing marked):
```
EXIT CODE: 3
```
Two RuntimeErrors that make the whole project unimportable are folded into "nothing marked for
compilation" (exit 3), the same code used for an empty, un-decorated project — not "failure" (1).

Neither scenario can ever produce exit code 1 for an import failure; that path is reserved for a
downstream build/toolchain exception. `report.failures` is surfaced only in the printed text, never
in the exit status, so the CLI's own documented contract ("failure" is exit code 1) is false for
the exact case ("a module raises on import") the same section of the README calls out one sentence
earlier.

### Why this matters
README.md explicitly recommends `compylr compyle` for "a container image, a serverless handler" —
exactly automation that checks exit status, not printed text. A CI/build step that gates on `$?`
after `compylr compyle` will treat a project with a broken (or partially broken) module as a
successful precompile.

### Test coverage confirms the gap was never checked
`frontends/python/tests/test_precompile.py` never calls `_precompile.main()` with a scenario that
has both `report.failures` non-empty AND checks the returned exit code; the only `main()` exit-code
assertions in the repo (`test_precompile.py:246,255,264,270`, `test_disable.py:147`) are for a
missing root (2), nothing marked (3), and full success (0). No test exercises code path 1 at all,
and none combines "some modules failed" with "the run still built something."

### Files
- frontends/python/compylr/_precompile.py:312-316 (the bug)
- README.md:319-325 (the false claim)
- pyproject.toml:29 (confirms this is the real `compylr` console script)
- frontends/python/tests/test_precompile.py (confirms the gap in coverage)

## Areas checked and found sound (not defects)
- `Ty::Tuple`, `Ty::Dict`, `Ty::Set` as top-level parameter/return types: compile via PyO3
  (`cargo check` in context/tuple_crate, context/dictset_crate) AND run correctly
  (`context/tuple_crate`: `swap((5, "hi"))` -> `('hi', 5)` through an actually-built .so).
- Nested class values in collections/tuples, and explicit class-valued params on methods, are
  correctly rejected by the frontend with a location (verified via CLI on
  context/nested_class.py, context/nested_class2.py, context/method_class_param2.py) — matches
  README.md:628-629's "not supported yet" claim.
- `rust_ident` correctly raw-escapes Rust-keyword-but-valid-Python-identifier parameter names
  (crates/compylr-backend-rust/src/rust.rs:90-98), used in emit_function_param/emit_function_arg.
- `RuntimeError` -> Python exception mapping (bindings.rs `__compylr_to_py_err`) is an exhaustive
  match with no wildcard arm — cannot silently swallow a new error variant.
- The python-rust bridge (bindings.rs) has no `is_scalar`-style filter; every function and class in
  the unit is emitted and registered — unlike the (typescript, go) bridge's documented member-drop
  behavior (issue #39).
- `llm_assist=True` is correctly refused at validation time (frontends/python/compylr/_config.py:195-197).
- `_core.pyi` stub matches every export in crates/compylr-host-python/src/lib.rs's `_core` pymodule exactly.
- backends registry three-way answer (implemented/reserved/unknown) verified correct via
  crates/compylr-registry/src/backends.rs.
