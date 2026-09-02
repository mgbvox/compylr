# demo-integrity audit evidence

## Finding: demo-ts-go/src/algorithms/ir_coverage.ts is a hardcoded stub, not a measurement

File: demo/demo-ts-go/src/algorithms/ir_coverage.ts (entire file, 15 lines) — every line is a
`console.log` of a literal string. No file read, no JSON parse, no walk of any build artifact.
Contrast with demo/demo-python-rust/src/algorithms/ir_coverage.py, which loads
`.compylr/ir/unit.json` and computes `first_use`/`missing`/`gaps` from the real artifact
(`load_artifact`, `Coverage.covered`/`missing`, `measure`).

Ran it directly:
```
$ node --experimental-strip-types src/algorithms/index.ts
...
statements   — 13/13: Return, ReturnUnit, SetAttr, SetItem, Append, Break, Continue, If, While, For, Var, Effect, Delete
expressions  — 19/19: Literal, Name, Neg, Not, ToFloat, Binary, Subscript, Attribute, Len, Call, MethodCall, SetLit, DictLit, ArrayLit, TupleLit, Construct, Range, Has, Is
...
Every IR form a TypeScript program can produce is exercised by this demo package.
```

Checked the real IR artifact this build actually wrote (`.compylr/ir/unit.json`) by walking every
serde tag under `functions`/`classes` (same externally-tagged-enum walk `ir_coverage.py` uses;
confirmed the TS-targeted artifact uses the identical JSON shape by inspecting
`.compylr/ir/unit.json`'s first function):

```
missing stmts: ['Var', 'Delete']
missing exprs: ['ToFloat', 'ArrayLit', 'Range', 'Has', 'Is']
```

Cross-checked against the actual `compylr-ir` source (`crates/compylr-ir/src/ir.rs`):
- `enum Stmt` (line 728) has exactly 13 variants: Return, ReturnUnit, Bind, Assign, Effect,
  SetAttr, SetItem, Append, If, While, For, Break, Continue. **There is no `Var` or `Delete`
  variant anywhere in the IR** — ir_coverage.ts's printed statement list names two forms that do
  not exist in the compiler at all, while omitting the two real variants `Bind` and `Assign`.
- `enum Expr` (line 441) has exactly 19 variants, and the real names are `ListLit` (not
  `ArrayLit`), `Contains` (not `Has`); **there is no `Is` variant anywhere in `compylr-ir`**.
  `TupleIndex` is a real variant the TS list omits.

So `npm start`'s "IR construct coverage" display: (a) never reads any artifact, (b) is byte-
identical no matter what the demo actually compiles, (c) currently claims full coverage of two
statement forms and one expression form that do not exist in the compiler, while failing to
mention that the real build is missing 2 real statement forms and 5 real expression forms
(Bind/Assign statements aren't even named; ToFloat, ArrayLit[real:ListLit], Range, Has[real:
Contains], Is[nonexistent] are all absent from the real artifact).

## Finding: demo-ts-go/tests/test_coverage.test.ts does not test coverage

File: demo/demo-ts-go/tests/test_coverage.test.ts (14 lines). Its only assertion is
`registered.size >= 25` plus 4 `.has(name)` checks against the `CompylrManager`'s registration
map — i.e. it tests that the `compyle()` decorator (a pure passthrough, see `_compylr.ts`) was
called on ~25 names. It never touches `ir_coverage.ts`, never reads `.compylr/ir/unit.json`, and
cannot fail if IR coverage regresses, contradicting the file name and the README's framing
alongside the Python project's `test_coverage.py` (which does 6 real parametrized assertions over
`Coverage.missing()`/`.gaps()`/`.first_use`, including a "coverage check that cannot fail is worse
than none" deletion test-suite).

## demo-ts-go/.compylr/go/* is dead code, unreferenced by src/ or tests/

```
$ grep -rln "compylr/go\|index.js\|koffi\|dlopen\|ffi" src/ tests/ package.json
(no output)
```
Nothing in the TS source or its tests ever loads the generated Go bridge output. Consistent with
already-filed issue #38/#39 (compyle() is a no-op passthrough; nothing benchmarked or tested ever
executes compiled Go).

## demo-python-rust — verified genuine (fresh eyes), no new defect found

- `PYTHONPATH=src .venv/bin/python -m algorithms.benchmark --scale 1` actually ran two real
  subprocesses (`_timing.measure_in_child`, `subprocess.run([sys.executable, "-m", module, ...])`)
  with `COMPYLR_DISABLE` set/unset, produced real varying timings including a genuine
  `not resolvable` for workloads under the noise floor, and an `!` instability marker — output
  varies run to run (spread values differed from the numbers baked into the committed README),
  which is what real measurement looks like.
- `.venv/bin/python -m pytest -q` → `1833 passed in 3.97s`, including `tests/test_benchmark.py`
  (explicitly tests the harness's *honesty*: that compiled/interpreted are really different modes,
  answers agree, timing-a-batch works) and `tests/test_coverage.py` (real parametrized assertions
  against `ir_coverage.measure(...)` of the real build artifact).
- `PYTHONPATH=src .venv/bin/python -m algorithms` printed a coverage table computed live against
  the just-built artifact (`compylr_generated_ef5b5f0fa0049bdd_a0cdd1db`), naming real member
  attributions (e.g. `Break  binary_search`, `Continue  bfs_distances`) that match the actual
  algorithm modules — not a static string.

No fabrication found on the Python side.
