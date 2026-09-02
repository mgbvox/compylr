# Audit: go-backend dimension (crates/compylr-backend-golang/)

All commands run from /Users/mgb/RustRoverProjects/compylr on branch feature/cpp-backend,
via `cargo run -q -p compylr-cli -- --backend go ...` (read-only CLI invocations) and,
where noted, by hand-assembling the emitted Go into a scratch module under
/private/tmp/.../scratchpad/gocheck/ and running it with the system Go toolchain
(`go version go1.21.5 darwin/arm64`) to observe actual runtime behavior.

## Source files audited
- crates/compylr-backend-golang/src/compat.rs (70 lines)
- crates/compylr-backend-golang/src/emit.rs (441 lines)
- crates/compylr-backend-golang/src/golang.rs (103 lines)
- crates/compylr-backend-golang/src/types.rs (66 lines)
- crates/compylr-backend-golang/tests/emit.rs (43 lines, ONE test)
- compared against crates/compylr-backend-rust/src/rust.rs (2515 lines) for the same axes

## 1. `for i in range(n)` cannot compile — Expr::Range unhandled

emit.rs's `emit_expr` match has a wildcard fallback (line 439):
```rust
_ => "/* unsupported expr */".to_string(),
```
`Expr::Range` (compylr-ir's dedicated form for every counted loop — Python's `range()`,
Go's own three-clause `for`) has no arm and falls through to this. Confirmed:

```
$ cat sumrange.py
def sum_to(n: int) -> int:
    total: int = 0
    for i in range(n):
        total = total + i
    return total

$ cargo run -q -p compylr-cli -- --backend go --emit rust sumrange.py
func sum_to(n int64) int64 {
	var total int64 = int64(0)
	for _, i := range /* unsupported expr */ {
		total = (total + i)
	}
	return total
}
```
This is not valid Go — confirmed by feeding it to `gofmt`:
```
$ gofmt -l /tmp/badrange.go   # (the emitted body above)
/tmp/badrange.go:5:43: expected operand, found '{'
/tmp/badrange.go:8:2: expected '{', found 'return'
/tmp/badrange.go:9:3: expected '}', found 'EOF'
exit: 2
```
3 of the 19 fixtures in frontends/python/fixtures/accepted/ use `range(` directly (many more
use it indirectly/transitively); this is the single most common loop construct in the subset,
and it is completely unimplemented for Go.

## 2. Collection/tuple literals hardcode element types to int64

`emit_expr`'s `ListLit`, `DictLit`, `SetLit`, `TupleLit` arms (emit.rs:378-435) do not consult
the IR type at all — they always emit `[]int64{...}`, `map[int64]int64{...}`,
`map[string]struct{}{...}` (for sets, regardless of element type), and every tuple field typed
`int64`. Confirmed compile-breaking output for every non-int case:

```
$ cat strlist.py
def greet() -> str:
    names: list[str] = ["a", "b", "c"]
    return names[0]
$ cargo run ... --backend go --emit rust strlist.py
	var names []string = []int64{"a", "b", "c"}     # type mismatch, won't compile

$ cat intset.py
def has_two() -> bool:
    s: set[int] = {1, 2, 3}
    return 2 in s
$ cargo run ... --backend go --emit rust intset.py
	var s map[int64]struct{} = map[string]struct{}{int64(1): struct{}{}, ...}  # mismatch

$ cat strdict.py
def lookup() -> str:
    d: dict[str, str] = {"a": "x"}
    return d["a"]
$ cargo run ... --backend go --emit rust strdict.py
	var d map[string]string = map[int64]int64{"a": "x"}   # mismatch, "x" isn't int64

$ cat tup.py
def pair() -> tuple[int, str]:
    return (1, "x")
$ cargo run ... --backend go --emit rust tup.py
func pair() struct { F0 int64; F1 string } {
	return struct { F0 int64; F1 int64 }{F0: int64(1), F1: "x"}   # struct type mismatch,
	                                                                 # "x" into int64 field
}
```
`go_ty()` (types.rs) is fully correct and type-aware for all of these — the emitter simply
never calls it for literals.

## 3. `in`/`not in` (Expr::Contains) only works for maps/sets — breaks list and str

emit.rs:405-411:
```rust
Expr::Contains { value, container } => format!(
    "func() bool {{ _, ok := ({})[{}]; return ok }}()",
    emit_expr(container), emit_expr(value)
),
```
Always uses Go's comma-ok *map* index form, which is a compile error on a slice or a string
(only maps support `v, ok := m[k]`). Per CLAUDE.md, `in`/`not in` must work over list, dict
(keys), set, and str (substrings) — three of those four containers are broken:

```
$ cat listcontains.py
def has_two(xs: list[int]) -> bool:
    return 2 in xs
$ cargo run ... --backend go --emit rust listcontains.py
	return func() bool { _, ok := (xs)[int64(2)]; return ok }()   # invalid: xs is []int64

$ cat strcontains.py
def has_sub(s: str) -> bool:
    return "ab" in s
$ cargo run ... --backend go --emit rust strcontains.py
	return func() bool { _, ok := (s)["ab"]; return ok }()   # invalid: s is string
```
Neither compiles. A correct str-in test needs `strings.Contains`; list containment needs a
loop. Both are unimplemented.

## 4. A missing dict key silently returns the zero value instead of reporting

CLAUDE.md states as a cross-language universal (no mode): "a missing mapping key always
reports." emit.rs's `Subscript` arm (line 356-358) destructures away `checked` with `..` and
always emits plain Go indexing:
```rust
Expr::Subscript { base, index, .. } => format!("({})[{}]", emit_expr(base), emit_expr(index)),
```
Plain Go map indexing on a missing key returns the zero value — it does not panic or report.
Built and ran the emitted code to confirm at runtime (not just read from source):

```go
// generated.go
func get(d map[string]int64, k string) int64 { return (d)[k] }
// main.go
d := map[string]int64{"a": 1}
fmt.Println("missing key result:", get(d, "nope"))
```
```
$ go run .
missing key result: 0
EXIT: 0
```
Python: `d["nope"]` raises `KeyError`. The Go translation of the same program silently
returns 0 — this is exactly the class of defect CLAUDE.md calls out by name for this
guarantee ("always reports", no mode, no exception).

The `GoMapGet` helper defined in compat.rs (`func GoMapGet[K,V](m map[K]V, key K) V { return
m[key] }`) doesn't even fix this — it has the identical bug (no presence check) — but moot
anyway since:

```
$ grep -n "GoMapGet\|GoSubscript\|GoKeys\|GoSetKeys\|GoRuneLen" crates/compylr-backend-golang/src/emit.rs
(no output)
```
None of `GoMapGet`, `GoSubscript`, `GoKeys`, `GoSetKeys`, or `GoRuneLen` — five of the seven
runtime helpers embedded into every generated Go package via `compat.go` — are ever called
from the emitter. They are dead code shipped into every build.

## 5. Negative/from-end indexing (`xs[-1]`) is unimplemented — despite a helper written for it

`GoSubscript` in compat.rs is explicitly documented "resolves positive or negative-from-end
slice indexing" — i.e., it exists to translate Python's `IndexOrigin::FromEitherEnd` into Go,
whose native slices only support `IndexOrigin::FromStart`. As shown in #4's grep, it is never
called. The `Subscript` node's `origin` field is likewise discarded via `..`.

```
$ cat negidx.py
def last(xs: list[int]) -> int:
    return xs[-1]
$ cargo run ... --backend go --emit rust negidx.py
	return (xs)[int64(-1)]
```
Built this into a real Go module and ran it:
```
$ go build .
./generated.go:4:14: invalid argument: index int64(-1) (constant -1 of type int64) must not be negative
```
Even the simplest case (a literal `-1`) fails to compile; a computed negative index would
instead panic at runtime with "index out of range" rather than returning the last element.
Python's `xs[-1]` (last element) has no working translation to Go anywhere in this backend.

## 6. `GO_PRESERVES` claims `DivisionByZeroReported`; exact/float division does not deliver it

golang.rs:20-23:
```rust
const GO_PRESERVES: &[Guarantee] = &[
    Guarantee::DivisionByZeroReported,
    Guarantee::FloatOrderPreserved,
];
```
Python's `exact_division` (plain `/`) is `Checked::Reported` by default (component.rs:61,
"`1.0 / 0.0` raises rather than yielding an infinity") — confirmed on the actual IR:
```
$ cat divzero.py
def half(x: float) -> float:
    return x / 0.0
$ cargo run ... --backend go --emit ir divzero.py | python3 -m json.tool | grep -A2 '"Div"'
    "Div": { "mode": "Exact", "checked": "Reported" }
```
emit.rs's Binary/Div arm for the Exact case discards `checked` and always emits bare Go `/`:
```rust
BinOp::Div { .. } => format!("({} / {})", l, r),
```
Built and ran the emitted code:
```go
func half(x float64) float64 { return (x / float64(0.0)) }
// main: fmt.Println("result:", half(1.0))
```
```
$ go run .
result: +Inf
EXIT: 0
```
No panic, no error — a silent `+Inf`, exactly the "float division does something other than
what Python's `checked: Reported` means" case the guarantee exists to prevent. `GO_PRESERVES`
asserts this is handled; it is not, for the exact-division path.

For comparison, `compylr-backend-rust/src/rust.rs:2360-2382` explicitly branches on `checked`
for the identical `DivMode::Exact` case:
```rust
return Ok(match checked {
    Checked::Reported => format!("div_exact(&({left}), &({right}))?"),
    Checked::Unchecked => format!("(({left}) / ({right}))"),
});
```
The sibling backend gets this right for exactly the reason the Go backend gets it wrong: it
reads `checked` off the node instead of discarding it.

## 7. Text length (`Expr::Len`) ignores `TextUnits`; wrong for any non-ASCII string

emit.rs:360:
```rust
Expr::Len { value, .. } => format!("int64(len({}))", emit_expr(value)),
```
`units` is discarded. Python's `len()` counts code points (`units: CodePoints`), confirmed on
the IR:
```
$ cargo run ... --backend go --emit ir strlen.py | python3 -m json.tool | grep units
"units": "CodePoints"
```
Go's native `len(string)` counts UTF-8 bytes. Built and ran:
```go
func slen(s string) int64 { return int64(len(s)) }
// main: fmt.Println("len:", slen("é"))
```
```
$ go run .
len: 2
```
```
$ python3 -c "print(len('é'))"
1
```
Silently wrong by a factor that grows with non-ASCII content. `GoRuneLen` (compat.rs,
`utf8.RuneCountInString`) exists specifically to produce the correct answer and — per the
grep in #4 — is never called.

## 8. `Rounding` on `DivMode::Integer` is discarded — TowardZero and TowardNegInf emit identically

emit.rs:342-345:
```rust
BinOp::Div { mode: DivMode::Integer(_), .. } => format!("GoFloorDiv({}, {})", l, r),
```
The `Rounding` payload is wildcarded away. `GoFloorDiv` always floors (`TowardNegInf`).
Forcing Go's own native (truncating) integer-division reading via `--behavior
integer_division=go` changes the IR node but not the emitted code:
```
$ cargo run ... --backend go --behavior integer_division=go --emit ir idiv.py | python3 -m json.tool | grep -A4 '"Div"'
"Div": { "mode": { "Integer": "TowardZero" }, "checked": "Reported" }
$ cargo run ... --backend go --behavior integer_division=go --emit rust idiv.py
	return GoFloorDiv(a, b)          # identical to the TowardNegInf/default case
```
`GoFloorDiv(-7, 2)` = -4 (flooring); Go's native `-7 / 2` = -3 (truncating). Requesting Go's
own semantics produces the wrong numeric answer.

## 9. `RemSign` on `BinOp::Rem` is discarded the same way

emit.rs:347: `BinOp::Rem { .. } => format!("GoRem({}, {})", l, r)`. `GoRem` hardcodes
`RemSign::Divisor` (Python's convention). Forcing Go's own native remainder convention via
`--behavior remainder=go`:
```
$ cargo run ... --backend go --behavior remainder=go --emit ir remmode.py | python3 -m json.tool | grep -A3 '"Rem"'
"Rem": { "sign": "Dividend", "checked": "Reported" }
$ cargo run ... --backend go --behavior remainder=go --emit rust remmode.py
	return GoRem(a, b)                # identical to the default Divisor-sign case
```
`GoRem(-7, 2)` = 1 (Divisor sign, Python's `-7 % 2`); Go's native `-7 % 2` = -1 (Dividend
sign). Same defect shape as #8: the mode payload is read only far enough to route to a
helper, never far enough to select which convention that helper implements.

## 10. Integer-overflow `checked` (Add/Sub/Mul/Neg) is discarded; nothing currently enforces it either

emit.rs:339-341, 332: `BinOp::Add { .. } => format!("({} + {})", l, r)` (same for Sub/Mul),
`Expr::Neg { value, .. } => format!("-({})", emit_expr(value))`. Native Go operators are
always used regardless of `Checked::Reported` vs `Checked::Unchecked`. `GO_PRESERVES` does
*not* claim `IntegerOverflowReported`, so this is consistent with what the backend declares —
but nothing in the pipeline currently stops a Python program whose default
`integer_overflow: Checked::Reported` (component.rs:53) from being silently compiled to
wrapping Go arithmetic via the CLI:
```
$ cat addint.py
def add(x: int, y: int) -> int:
    return x + y
$ cargo run ... --backend go --emit rust addint.py
	return (x + y)          # no negotiation error, despite Go not preserving overflow-reported
```
`negotiate()` (compylr-core/src/negotiation.rs) — the mechanism CLAUDE.md and compylr-core's
own docs describe as refusing an incompatible (frontend-requires, backend-preserves)
combination "before any target source exists" — is called in exactly one place in the whole
workspace:
```
$ grep -rn "negotiate(" crates/*/src/*.rs crates/*/src/**/*.rs
crates/compylr-core/src/negotiation.rs:49:pub fn negotiate(...)
crates/compylr-host-python/src/lib.rs:243:    negotiate(&unit, backend).map_err(...)?;
```
`compylr-cli` never calls it (confirmed: `grep -n negotiate crates/compylr-cli/src/*.rs` finds
nothing). And the one real call site (`compylr-host-python`, the `@compyle` decorator path)
resolves the host bridge *before* negotiate ever runs (lib.rs:214: `bridges::lookup(...)`),
and there is no (python, go) bridge registered — so for Go specifically, negotiate() is dead
code in every reachable path: unreachable from `@compyle` (bridge lookup fails first) and
unreachable from the CLI (never called). The "core refuses the combination by name" claim in
CLAUDE.md is true for (python, rust) and false in practice for (python, go).

## 11. gofmt failure is swallowed; broken output is returned as if formatting succeeded

golang.rs:82-102, `format_go_source`:
```rust
match child.wait_with_output() {
    Ok(output) if output.status.success() => String::from_utf8(...)...,
    _ => source.to_string(),   // gofmt failed (or wasn't found) -> return the raw source anyway
}
```
Confirmed gofmt actually detects the #1 defect as a hard parse error:
```
$ gofmt -l /tmp/badrange.go
/tmp/badrange.go:5:43: expected operand, found '{'
...
exit: 2
```
But `GoBackend::post_process` (the only caller of `format_go_source`) has no branch for
failure — every `--emit crate`/`--emit rust` build silently ships syntactically invalid Go
with no diagnostic, no error return, nothing distinguishing it from a clean build.

## 12. Test coverage: one trivial test, no coverage-by-construction check exists

`crates/compylr-backend-golang/tests/emit.rs` contains exactly one test
(`emits_valid_go_package_and_function`), covering a single `Add` with `Checked::Unchecked`.
Nothing in the crate exercises `Range`, `Contains`, any non-`int` `List/Dict/Set/TupleLit`,
negative indexing, or any `Rounding`/`RemSign`/`TextUnits`/`Checked::Reported` variation.
There is no Go analogue of `crates/compylr-host-python/tests/conformance.rs` (which CLAUDE.md
describes as checking `(form, position)` coverage for the Rust backend and crediting it with
catching 4 real defects on its first run) — this whole class of defect (11 of them, above)
shipped with zero automated coverage.

## Comparison to compylr-backend-rust

Every axis the Rust backend gets right by explicitly matching on the IR's carried mode
(confirmed by reading rust.rs:2360-2450: `Checked::Reported`/`Checked::Unchecked` bound in
every arm for Add/Sub/Mul/Div/Rem, `DivMode` distinguishing both `Rounding` values via
separate `PyNum::div_floor`/`PyNum::div_trunc` calls, `RemSign` likewise via
`PyNum::rem_floor`/`PyNum::rem_trunc`) is exactly the axis the Go backend gets wrong by
discarding the same field with `..` or `_`. The Go backend is not an independently-buggy
implementation of the same design — it is missing the design element (matching on modes, not
operation identity) that CLAUDE.md states is the whole point of carrying modes on IR nodes at
all, and that the Rust backend demonstrably implements.
