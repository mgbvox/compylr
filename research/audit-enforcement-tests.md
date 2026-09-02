# Enforcement-tests dimension audit

## Finding A (critical): Go backend output is never handed to `go build`/`go vet` anywhere
in the Rust enforcement suite, and the one test whose entire premise is "rendering is not
enough, the result has to build" (conformance.rs) is hardcoded to the Rust backend only.

### The corpus already contains the exact case that breaks
crates/compylr-host-python/tests/conformance.rs has several `Stmt::For { iter: Expr::Range { .. }, .. }`
corpus entries (grep hits at lines 351-354, 619-622, 658-661, 729-732, 886-889).

### `every_implemented_backend_renders_the_whole_corpus` (line 970) only checks emit() doesn't
error and returns non-empty files:
```
970:fn every_implemented_backend_renders_the_whole_corpus() {
971:    let backends = compylr_registry::backends::implemented_names();
...
979:            let files = backend.emit(&unit).unwrap_or_else(|error| { panic!(...) });
981:            assert!(!files.is_empty(), "'{backend_name}' rendered '{name}' as no files at all");
```
No check that the returned text is syntactically valid target source.

### The only "does it build" check exists solely for Rust:
```
993:/// output to the target's own compiler settles it.
995:fn every_corpus_entry_compiles_for_the_rust_backend() {
998:    let backend = compylr_registry::backends::lookup("rust").expect("the shipped backend");
...
1010:        let output = Command::new("rustc")
```
There is no `every_corpus_entry_compiles_for_the_go_backend` (or cpp) anywhere in
crates/compylr-host-python/tests/, nor in crates/compylr-backend-golang/tests/emit.rs (43 lines,
no `go build` invocation, confirmed by `grep -n "go build\|fn main" crates/compylr-backend-golang/tests/emit.rs` -> no hits).

### Root cause in the backend itself, confirmed by reading + running
crates/compylr-backend-golang/src/emit.rs:439 — `emit_expr`'s match has no arm for `Expr::Range`
and falls through to:
```
439:        _ => "/* unsupported expr */".to_string(),
```
`Stmt::For` (emit.rs:263-296) always calls `emit_expr(iter)` inside a Go `range` clause, so any
`for x in range(...)` loop emits:
```go
for _, i := range /* unsupported expr */ {
```
which is a Go syntax error.

### Reproduced end-to-end against real accepted fixtures (not just the hand-built IR corpus)
```
$ cargo run -q -p compylr-cli -- --backend go --emit rust \
    frontends/python/fixtures/accepted/classes.py frontends/python/fixtures/accepted/collections.py \
    frontends/python/fixtures/accepted/branching.py frontends/python/fixtures/accepted/loops.py \
    frontends/python/fixtures/accepted/arithmetic.py frontends/python/fixtures/accepted/mutation.py \
    frontends/python/fixtures/accepted/nested_mutation.py frontends/python/fixtures/accepted/division.py \
    frontends/python/fixtures/accepted/comparisons.py > context/go_check2/generated.go
```
produced (excerpt):
```go
func countdown(n int64) int64 {
	var steps int64 = int64(0)
	for _, i := range /* unsupported expr */ {
		steps = (steps + int64(1))
	}
	return steps
}
```
Reconstructed the crate (go.mod = `module compylr\n\ngo 1.20\n` per
crates/compylr-backend-golang/src/golang.rs:72-74, and compat.go extracted verbatim from
crates/compylr-backend-golang/src/compat.rs's `GO_COMPAT_SOURCE`), then:
```
$ cd context/go_check2 && go build ./...
# compylr
./generated.go:110:43: syntax error: unexpected {, expected expression
./generated.go:111:27: syntax error: unexpected ), expected { after for clause
./generated.go:138:43: syntax error: unexpected {, expected expression
./generated.go:139:9: syntax error: unexpected ), expected { after for clause
./generated.go:154:43: syntax error: unexpected {, expected expression
./generated.go:155:3: syntax error: unexpected if, expected { after for clause
./generated.go:307:43: syntax error: unexpected {, expected expression
./generated.go:308:19: syntax error: unexpected ), expected { after for clause
./generated.go:315:43: syntax error: unexpected {, expected expression
./generated.go:316:25: syntax error: unexpected ), expected { after for clause
```
3 of the ~19 accepted Python fixtures use `range(...)` (loops.py, mutation.py, nested_mutation.py;
`grep -rl "range(" frontends/python/fixtures/accepted/*.py`), and `for ... in range(...)` is one
of the most heavily documented constructs in the project's own CLAUDE.md ("range is a reserved
name... the loop is written out against a cursor the body cannot disturb...").

### Proof the enforcement test currently passes despite this
```
$ cargo test -p compylr --test conformance every_implemented_backend_renders_the_whole_corpus -- --nocapture
running 1 test
test every_implemented_backend_renders_the_whole_corpus ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out
```
This is the same test/corpus that already contains `Expr::Range` for-loop entries and is fed to
the "go" backend by name (`implemented_names()` includes "go" per
crates/compylr-host-python/tests/registry.rs:12-17,
`the_rust_and_go_backends_are_implemented`). It passes only because the assertion checks
non-emptiness, not validity.

### emit_quality.rs has the identical scoping problem
crates/compylr-host-python/tests/emit_quality.rs's module doc ("Whether emitted source is fit to
be compiled and read... checked against every accepted fixture") reads as general, but every test
in the file hardcodes `lookup("rust")` (grep: `let backend = lookup("rust").unwrap();` at lines 84,
128, 154, 186 approx). No backend parameter, no loop over `implemented_names()`. Same for
differential.rs, whose only backend/frontend references are
`compylr_backend_rust::{...}` / `compylr_frontend_python::...` imports and one
`lookup("rust").unwrap()` call (line 491) — the module doc is honestly scoped to
"generated Rust must answer what CPython answers", so this is not a false claim, just confirms
the (typescript, go) pair has zero differential/build coverage anywhere in this dimension.

## Finding B (medium): readme.rs's backend-status check (`readme_status_matches_reality`,
line ~215) only ever checks the Rust backend's presence/absence against the text "no backend" /
"Rust source". There is no equivalent check for the Go or TypeScript backend claims.
`grep -ni "golang\|\"go\"\|typescript" crates/compylr-host-python/tests/readme.rs` -> no hits at all.
Confirmed the gap is live: README.md:185-186 currently reads
"Not built yet: `llm_assist` ..., and the TypeScript, Go, and C++ backends (reserved names that
fail with a message saying so)." — false for Go, which is fully implemented and registered
(`compylr-backend-golang` exists; `cargo run -q -p compylr-cli -- --backend go --emit rust
frontends/python/fixtures/accepted/arithmetic.py` emits real Go, not a "reserved name" refusal
message). This matches previously-filed issue #40; the new evidence here is that readme.rs's own
mechanism, whose whole purpose is "so cargo test fails when the code and the README disagree",
was never extended to check this specific claim, which is why it survives.
