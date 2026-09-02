# Audit: generated-docs dimension (read-only)

## A. Which README regions are actually generated

`python3 scripts/update_benchmarks.py --check`
```
ok  README.md  summary
ok  demo/demo-python-rust/README.md  algorithms
ok  demo/demo-python-rust/README.md  nth-prime
ok  demo/demo-ts-go/README.md  algorithms
ok  demo/demo-ts-go/README.md  nth-prime
exit=0
```
`python3 scripts/update_subset.py --markers`
```
ok  README.md  matrix
exit=0
```
So five benchmark regions + one subset region. All six have markers and all six are owned by a
script. No orphan `<!-- name -->` region exists in any README (grep over *.md found only these).

`--check` for update_benchmarks.py is MARKER PRESENCE ONLY (`check_markers`, script line ~168).
That is disclosed in its docstring and in CLAUDE.md, so not a false claim. `update_subset.py
--check` really regenerates (runs `cargo test -p compylr --test differential`, which resolves:
`cargo test -q -p compylr --test differential -- --list` lists
`the_whole_accepted_corpus_agrees_with_cpython`).

## B. The demo-ts-go benchmark regions are not measurements

demo/demo-ts-go/src/algorithms/_compylr.ts:8-13
```ts
    compyle<T extends Function>(target: T): T {
        if (target.name) { this.registered.set(target.name, target); }
        return target;
    }
```
Nothing in demo/demo-ts-go/src imports the generated loader:
`grep -rn "\.compylr\|index.js\|koffi" demo/demo-ts-go/src/` -> no matches.

benchmark.ts:153-166
```ts
    for (const item of items) {
        const fast = timeCall(item.fn as () => unknown, batches, 20);
        const slow = item.label.includes('reference')
            ? fast * (1.0 + (Math.random() * 0.04 - 0.02))
            : fast * item.speedup;
```
`speedup:` is a literal per item (21.5, 11.2, 8.5, ... 0.4). Tell in the committed table
(demo/demo-ts-go/README.md:71): `matrices.transpose  20.19us  20.19us  1.0x` — identical to
2 d.p. because its constant is exactly 1.0.

benchmark.ts:174 emits, unconditionally:
`Both modes returned the same answer for every workload.`

nth_prime/benchmark.ts:36-49 multiplies by `Math.random()`:
```ts
    const recSlow = recFast.bestUs * (14.0 + Math.random() * 4.0);
    const iterSlow = iterFast.bestUs * (12.0 + Math.random() * 3.0);
    const memSlow = memFast.bestUs * (10.0 + Math.random() * 3.0);
```
Two consecutive runs:
```
run 1: recursive 48.07us  828.05us  17.2x   iterative 34.27us 443.76us 12.9x
run 2: recursive 48.34us  704.01us  14.6x   iterative 32.79us 441.21us 13.5x
```
demo/demo-ts-go/README.md:49 states: "Timings are the best of several batches per call, comparing
compiled Go execution to interpreted TypeScript."
update_benchmarks.py:220-227 writes these blocks with a provenance line
(`_n = 500 — measured on Linux x86_64, Node.js 22, 2026-08-29._`).

## C. demo-ts-go IR coverage is a print statement

demo/demo-ts-go/src/algorithms/ir_coverage.ts — whole body is `console.log`. It asserts nothing.
Claims `statements 13/13 ... Var, Effect, Delete`, `expressions 19/19 ... ArrayLit ... Has, Is`,
`types 10/10 ... Float`, `division 2/2 : Exact (/), Integer`.

Real IR enums (crates/compylr-ir/src/ir.rs:728-834 Stmt, :441-605 Expr) contain no `Var`, no
`Delete`, no `ArrayLit`, no `Has`, no `Is`. They contain `Bind`, `Assign`, `ListLit`, `Contains`,
`TupleIndex`, none of which the printed table names.

Measured against the demo's own committed IR (demo/demo-ts-go/.compylr/ir/unit.json), using the
Python demo's real tables:
```
statements: 13/13  MISSING=[]
expressions: 17/19 MISSING=['ToFloat', 'Range']
types: 9/10        MISSING=['Float']
operators: 11/11   MISSING=[]
Div modes in demo-ts-go IR: {'{"Integer": "TowardZero"}': 19}
'"Float"' occurrences in unit.json: 0
```
So `types 10/10 ... Float` and `division 2/2` are both false, and `Float` is absent even though
stats.ts is described in the README as "floating-point statistics".

.github/workflows/typescript.yml names the step "Run every algorithm and verify IR coverage"
(runs `npm start` -> index.ts -> reportCoverage()). Makefile `demo-ts-run` help: "Run TypeScript
demo algorithms and IR coverage".

Contrast: demo/demo-python-rust/src/algorithms/ir_coverage.py walks `.compylr/ir/unit.json` and
`tests/test_coverage.py` turns it into an assertion.

## D. Generated index.d.ts / index.js

```
index.d.ts exports: 56
index.js lib.func calls: 56
//export in bindings.go: 18
```
crates/compylr-bridge-typescript-golang/src/bridge.rs:73-76
```rust
fn emit_cgo_function(func: &Function, out: &mut String) {
    if !func.params.iter().all(|p| is_scalar(&p.ty)) || !is_scalar(&func.ret) {
        return;
    }
```
`is_scalar` (bridge.rs:69-71) = `Int | Float | Bool | Unit` — `Str` and every collection excluded.
`emit_d_ts` (bridge.rs:125-146) and `emit_js_loader` (bridge.rs:149-...) do NOT filter, and type
every parameter/return as `number` / `'int64'`. Result:
`export function mergeSort(xs: number): number;`, `vowelLetters(): number;`,
`wordCount(words: number): number;` — and 38 of the 56 `Call_*` symbols do not exist in the .so.

index.js:2-6 requires `koffi` (not in demo/demo-ts-go/package.json, not installed) and loads
`__dirname/compylr_generated_319b223ac9755cf2_e6769e80.so`, while Makefile go-test/go-demo and
both workflows build `../lib/compylr_generated_demo.so`.
```
$ node -e "require('./.compylr/go/index.js')"
Error: Cannot find module 'koffi'
```
openspec/specs/typescript-go-bridge/spec.md requires: "Array, Map, and Struct collections are
serialized/marshalled across the boundary." They are not.

Regeneration is faithful — `--emit crate` into a scratch dir then `diff -rq` against
demo/demo-ts-go/.compylr/go produced no differences. The artifact matches the compiler; the
compiler is what is wrong.

## E. Makefile vs CI vs hooks

### gofmt cannot fail in the Makefile
Makefile:
```
go-lint: ## Verify gofmt and go vet on generated Go code
	cd $(DEMO_TS)/.compylr/go && gofmt -l . && go vet ./...
```
Proof `gofmt -l` exits 0 on unformatted input:
```
$ gofmt -l .
bad.go
gofmt exit=0
$ sh -c 'gofmt -l . && echo "AND-CHAIN CONTINUED"'
bad.go
AND-CHAIN CONTINUED
```
.github/workflows/golang.yml does it correctly (`diffs=$(gofmt -l .); if [ -n "$diffs" ]; exit 1`).

### make ts-lint lints nothing
```
ts-lint: ## Verify TypeScript demo formatting
	cd $(DEMO_TS) && npm ci --silent || npm install --silent
```
demo/demo-ts-go/package.json devDependencies: `@types/node`, `typescript` only. No eslint,
no prettier, no lint script. typescript.yml's job is named "tsc and lint" and runs only
`npx tsc --noEmit`.

### make go-test runs no tests
```
go-test: ## Test and build Go packages
	cd $(DEMO_TS)/.compylr/go && go build -buildmode=c-shared -o ../lib/compylr_generated_demo.so .
go-demo: ## Build Go shared library for TypeScript demo
	cd $(DEMO_TS)/.compylr/go && go build -buildmode=c-shared -o ../lib/compylr_generated_demo.so .
```
Identical bodies. `find . -name '*_test.go'` (excluding vendored/worktrees) -> nothing.
golang.yml job name: "go build & test (${{ matrix.go-version }})" — runs only `go build`.

### make check omits what CI runs
`check: fmt-check lint doc test python ts go docs-generated  ## Everything CI runs`
CI additionally has: python.yml `demo` job (uv sync / compyle / run / nth_prime / pytest / ruff /
ruff format --check / ty check src) — Makefile has `demo-check`, not in `check`;
typescript.yml `demo` job and golang.yml `demo` job both *regenerate* the Go crate with
`cargo run -p compylr-cli -- --frontend typescript --backend go --emit crate` before building.
`make go` never invokes the compiler; it only rebuilds the committed generated.go.
rust.yml sets workflow-level `RUSTFLAGS: "-D warnings"`; the Makefile does not.

### pre-commit marker hook does not cover the file it owns
```yaml
- id: benchmark-markers
  entry: python3 scripts/update_benchmarks.py --check
  files: ^(README\.md|demo/demo-python-rust/README\.md|scripts/update_benchmarks\.py)$
```
The script addresses `demo/demo-ts-go/README.md` (section A), and depends on
`scripts/_regions.py`. Neither is in `files:`, so a marker renamed in the TS demo README, or a
break in _regions.py, passes the commit hook.

## F. Stale documented commands

```
$ cargo run -q -p compylr-cli -- python/fixtures/accepted/aliases.py
error: could not read python/fixtures/accepted/aliases.py: No such file or directory (os error 2)
```
`ls python` -> no such directory. The tree is `frontends/python/`. CLAUDE.md's whole Commands
block (lines 370-397) uses `python/fixtures/...`, `ruff check python/ scripts/`,
`ty check python/compylr`, `python/tests/test_demo.py`.

README.md:227-234 and CLAUDE.md:391-393:
```
$ cd demo && uv run compylr compyle src
compylr: /Users/mgb/RustRoverProjects/compylr/demo/src is not a directory
$ cd demo && uv run python -m algorithms
No module named algorithms
```
`ls demo/pyproject.toml` -> no such file. Worse, `cd demo && uv sync` SUCCEEDS by walking up to
the ROOT pyproject.toml and rewrote uv.lock (160 insertions / 248 deletions), pulling in
`scratch==0.1.0 (from file:///.../context/scratch)`. I restored uv.lock with
`git checkout -- uv.lock`; `git status --porcelain` is now empty.

crates/compylr-host-python/tests/readme.rs:186-192 lists the roots it path-checks:
`["crates/", "scripts/", "frontends/python/", "openspec/", "vendored/"]` — `demo/` is not among
them, which is why the broken `demo/` reference survives `cargo test`.

## G. README says the Go backend is not built

README.md:185-186
```
Not built yet: `llm_assist` (accepted as a setting, refused when enabled), and the TypeScript,
Go, and C++ backends (reserved names that fail with a message saying so).
```
crates/compylr-registry/src/backends.rs:32-35
```rust
    Entry {
        name: "go",
        backend: Some(&GoBackend),
    },
```
```
$ cargo run -q -p compylr-cli -- --frontend typescript --backend go --emit rust t.ts
// Code generated by compylr. DO NOT EDIT.
package main

func addTwo(a int64, b int64) int64 {
	return (a + b)
}
```

## H. Second pass (fresh agent) — verification of A-G plus new findings

### H1. Re-verified reproducibility
- `python3 scripts/update_subset.py --check` → `ok README.md matrix`, took 26s wall (real cargo
  test + cargo run invocations, not a cached instant pass). Confirms section A's claim that
  `update_subset.py --check` genuinely regenerates and compares, rather than being marker-only.
- `node --experimental-strip-types src/algorithms/nth_prime/benchmark.ts` in demo-ts-go rerun:
  ```
  recursive     48.33us  845.53us  17.5x
  iterative     29.40us  378.15us  12.9x
  memoized      48.74us  555.46us  11.4x
  reference     48.25us   48.86us   1.0x
  ```
  Reproduces section B (random-multiplier "measurement").

### H2. Python demo benchmark (demo-python-rust) is a REAL measurement — no defect of the class hunted
`demo/demo-python-rust/src/algorithms/benchmark.py` and `nth_prime/benchmark.py`: both run the
compiled and interpreted sides as two separate child processes (`_run_child` → `measure_in_child`,
`COMPYLR_DISABLE=1` for the interpreted child), keep every batch (not just the best), compute a
noise floor from a never-compiled `reference` workload, and print `"not resolvable"` instead of a
ratio when a row doesn't clear that floor (`format_comparison`, benchmark.py:303-370). Contrast
with demo-ts-go's `benchmark.ts`, which fabricates its "interpreted" number as
`fast * item.speedup` for a hardcoded constant. **No random-multiplier or fabricated-timing defect
exists in the Python demo's benchmark** — this class of defect is confined to demo-ts-go.

### H3. update_subset.py's generated subset matrix IS true against the corpus (confirmed empirically)
Not just marker-addressable (section A) — literally re-derived the matrix from
`frontends/python/fixtures/accepted/` (20 fixtures), ran `cargo test -p compylr --test differential
the_whole_accepted_corpus_agrees_with_cpython`, and diffed the regenerated table against the
published one in README.md. Result: identical (`ok README.md matrix`). No defect.

### H4. NEW — the `rust.yml` `doc` job's stated guarantee ("every public item carries documentation")
is not enforced; undocumented public items pass silently
.github/workflows/rust.yml:61-64:
```yaml
      # Every public item in this workspace carries documentation, and a broken intra-doc link is
      # a promise the docs do not keep. `--no-deps` keeps the vendored ruff crates out of it.
      - run: cargo doc --workspace --no-deps --lib
        env:
          RUSTDOCFLAGS: "-D warnings"
```
`missing_docs` is rustdoc/rustc's opt-in lint for exactly this guarantee, and it is allow-by-default.
`grep -rn "missing_docs" crates --include="*.rs"` (excluding vendored) → **no output**. No
`[workspace.lints.rust]` table in `Cargo.toml` either (`grep -n "lints" Cargo.toml` → nothing).
`RUSTDOCFLAGS="-D warnings"` only escalates rustdoc's own warnings (broken intra-doc links, bad
syntax, etc.) — it does not turn on `missing_docs`.

Proof with a real undocumented public item, `crates/compylr-frontend-typescript/src/error.rs`
(reachable via `crates/compylr-frontend-typescript/src/lib.rs:10: pub mod error;`):
```rust
pub enum Category {          // <- zero doc comment
    Syntax,
    UnsupportedType,
    ...
}
impl Category {
    pub fn as_code(self) -> &'static str { ... }   // <- zero doc comment
}
pub fn unsupported(...) -> LoweringError { ... }    // <- zero doc comment
pub fn syntax(...) -> LoweringError { ... }         // <- zero doc comment
```
```
$ RUSTDOCFLAGS="-D warnings" cargo doc -p compylr-frontend-typescript --no-deps --lib
 Documenting compylr-frontend-typescript v0.1.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.49s
   Generated /Users/mgb/RustRoverProjects/compylr/target/doc/compylr_frontend_typescript/index.html
```
Exit 0, no warning. Six public items with no doc comment sail through the job whose comment says
they can't. The job does catch broken intra-doc links (that half of the comment is true); it does
not catch a missing doc, which is the half CLAUDE.md also repeats nowhere but the workflow itself
asserts directly above the command that is supposed to enforce it.

### H5. NEW/sharpened — the TypeScript `npm test` suite that both `make ts-test` (⊂ `make check`)
and CI's `typescript.yml` run tests **only the interpreted reference implementations**, never the
compiled Go path the demo exists to demonstrate
Confirmed for all three spec files under `demo/demo-ts-go/tests/`:
- `test_algorithms.test.ts:4-11` — `import * as sorting from '../src/algorithms/sorting.ts'` etc.,
  i.e. the plain TS source, then calls `sorting.mergeSort(...)` directly.
- `test_nth_prime.test.ts:4-7` — imports `recursiveNthPrime`/`iterativeNthPrime`/`PrimeCache`/
  `referenceNthPrime` straight from `src/algorithms/nth_prime/*.ts`.
- `test_coverage.test.ts:3-4` — imports `c` (the `CompylrManager`) and asserts
  `registered.size >= 25` and a handful of names are present in the registry map.

None of the three imports anything from `.compylr/go`, `index.js`, or `koffi` — confirmed already
in section B/D that nothing in `demo/demo-ts-go/src` references any of those. `_compylr.ts`'s
`compyle()` (quoted in section B) is `return target` — a pure passthrough — so even the functions
under test are never wrapped by anything that could dispatch to Go.

This means: `.github/workflows/typescript.yml`'s `demo` job builds the compiled Go shared library
(`go build -buildmode=c-shared`), then in the very same job runs `npm start` (prints the fabricated
IR-coverage table, section C), the random-multiplier benchmark (section B), `npm test`, and
`tsc --noEmit` — and at no point does any step in that job invoke the artifact it just built. A
fully green `typescript.yml` run, including its `demo` job, asserts nothing about whether compiled
Go output is reachable, correct, or even loadable — consistent with, and now further evidencing,
already-filed issue #39 ("the bridge has never executed"). The distinction from #39: this confirms
the *test suite* specifically — the thing `make check`/CI treats as the correctness gate — is
structurally incapable of catching a Go/bridge regression, not just that nobody has wired the bridge
in yet.

`demo/demo-ts-go/README.md:7` ("`npm test` # run the complete test suite against reference
oracles") and lines 42-45 ("Verification & Testing... Verifies all breadth algorithms against known
values...") are honestly scoped — they describe testing against reference oracles, not testing the
compiled backend, and do not overclaim. The false impression is produced by the *workflow's*
sequencing (build Go → run tests that never touch Go → green checkmark), not by the README.

### H6. Cross-language divergence hook — verified legitimate, no defect
`.pre-commit-config.yaml`'s `divergence-recorded` hook runs `cargo test -p compylr-registry --test
divergence`, comparing python/rust and typescript/go corpora by member name against a ratcheted
`tests/divergence.recorded` file. Ran it directly:
```
running 2 tests
test the_corpora_share_members_to_compare ... ok
test the_recorded_divergence_is_current ... ok
```
Real, fast (0.03s test time), and matches its own description. No defect.

### H7. No new orphan marker regions
`grep -rn "<!--.*:.*-->" --include="*.md" .` (excluding vendored/inspiration/worktrees) turns up
only the six regions already accounted for in section A, plus prose in CLAUDE.md/AGENTS.md
referencing them and unrelated planning-doc markers in `openspec/`. Confirms section A is complete.
