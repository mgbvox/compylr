# Audit: `ts-go-bridge` — evidence transcript

All commands run 2026-08-31 on darwin/arm64, go1.21.5, node v24.11.0.
Scratch dir `$S = /private/tmp/claude-501/.../scratchpad`.

## 1. Export census on the checked-in demo artifact

```
$ cd demo/demo-ts-go/.compylr/go
$ echo "d.ts declared: $(grep -c '^export function' index.d.ts)"
d.ts declared: 56
$ echo "js exports:    $(grep -c '^exports\.' index.js)"
js exports:    56
$ echo "cgo exports:   $(grep -c '^//export' bindings.go)"
cgo exports:   18
$ grep -c '^func [a-z]' generated.go   # free functions
56
$ grep -c '^func (self' generated.go   # methods
15
$ grep -c '^func New' generated.go     # constructors
4
```

75 Go members generated; 18 C-exported. **24.0%**.
Zero of the 19 class members (IntStack, PrimeCache, RunningStats, UnionFind) are exported;
`emit_cgo_bindings` / `emit_dts` / `emit_js_loader` all iterate `unit.functions()` only
(`crates/compylr-bridge-typescript-golang/src/bridge.rs:61,126,162`); `Unit::classes()` exists
(`crates/compylr-ir/src/ir.rs:1266`) and is never called by the bridge.

## 2. The library builds, and its symbol table matches bindings.go, not index.js

```
$ cp demo/demo-ts-go/.compylr/go/*.go go.mod $S/gobuild/ && cd $S/gobuild
$ go build -buildmode=c-shared -o compylr_generated_319b223ac9755cf2_e6769e80.so .
(exit 0)
$ nm -gU *.so | grep -c 'Call_'
18
```

```
$ python3 -c 'ctypes probe'
Call_gcd(48,18) = 6
Call_averageOfCounts MISSING
Call_mean MISSING
Call_mergeSort MISSING
Call_divide MISSING
Call_NewIntStack MISSING
Call_IntStack_push MISSING
Call_push MISSING
```

`index.js` calls `lib.func('Call_averageOfCounts', ...)` at line 8 — top level, before any
user code runs — for a symbol that is not in the library.

## 3. The loader cannot be loaded by Node at all

```
$ cd demo/demo-ts-go && node .compylr/go/index.js
file:///.../.compylr/go/index.js:2
const path = require('path');
             ^
ReferenceError: require is not defined in ES module scope, you can use import instead
This file is being treated as an ES module because it has a '.js' file extension and
'/Users/mgb/RustRoverProjects/compylr/demo/demo-ts-go/package.json' contains "type": "module".
```

`koffi` is not a dependency of `demo/demo-ts-go/package.json`, nor of anything else in the repo
(`grep -rn koffi` outside node_modules hits only the bridge source, its test, one archived design
doc, and the generated file itself).

## 4. Float ABI mismatch — proven numerically

Probe source (`$S/probe.ts`), emitted with:
`cargo run -q -p compylr-cli -- --frontend typescript --backend go --emit crate --out $S/probecrate $S/probe.ts`

```
export function half(x: float): float { return x * 0.5; }
export function sumAll(xs: Array<number>): number { ... }
export function shout(s: string): string { return s; }
export class Counter { n: number; constructor(){...} bump(): void {...} value(): number {...} }
```

bindings.go (whole file):
```go
//export Call_half
func Call_half(x C.double) C.double {
	res := half(float64(x))
	return C.double(res)
}
func main() {}
```
index.js:
```js
const native_half = lib.func('Call_half', 'int64', ['int64']);
const native_shout = lib.func('Call_shout', 'int64', ['int64']);
const native_sumAll = lib.func('Call_sumAll', 'int64', ['int64']);
```
index.d.ts:
```ts
export function half(x: number): number;
export function shout(s: number): number;
export function sumAll(xs: number): number;
```

Built and called both ways:
```
correct ABI  Call_half(9.0) = 4.5
index.js ABI Call_half(9)   = 0
Call_shout MISSING
Call_sumAll MISSING
Call_NewCounter MISSING
Call_Counter_bump MISSING
```

## 5. A Go panic kills the host process; no error is translated

```
$ python3 -c 'ctypes: Call_floorDivide(1, 0)'
about to call Call_floorDivide(1, 0)
panic: division by zero

goroutine 17 [running, locked to thread]:
main.GoFloorDiv(...)        compat.go:10
main.floorDivide(...)       generated.go:464
main.Call_floorDivide(...)  bindings.go:22
PYTHON EXIT=134            # SIGABRT
```

`openspec/specs/typescript-go-bridge/spec.md:26,32-34` requires the JS wrapper to translate a
non-nil Go `error` into a thrown JS `Error("division by zero")`. Grep of the bridge crate for
`error|Error|throw`: only `BackendError` in the `emit` signature. No wrapper error handling exists.

## 6. Name collisions in both emitted languages

Probe (`$S/collide.ts`): `main`, `path`, `lib` as exported function names.

```
$ go build -buildmode=c-shared -o collide.so .
# compylr
./generated.go:8:6: func main must have no arguments and no return values
./bindings.go:26:6: main redeclared in this block
	./generated.go:8:6: other declaration of main

$ cp index.js check.cjs && node --check check.cjs
function lib(n) { return native_lib(n); }
^
SyntaxError: Identifier 'lib' has already been declared
```
(`emit_js_loader` hardcodes `const path = require('path')` at bridge.rs:152 and
`const lib = koffi.load(libPath)` at bridge.rs:160.)

## 7. Test coverage of the bridge

`crates/compylr-bridge-typescript-golang/tests/bridge.rs` is the only test that reads the emitted
files (`grep -rn 'bindings.go|index.d.ts|index.js' --include='*.rs'` finds nothing else). It builds
one function, `multiply(x: Int, y: Int) -> Int` — all scalar — and asserts `files.len() == 6` plus
three substring checks. No non-scalar signature, no class, and no assertion relating the
`//export` set to the `exports.` set.

`crates/compylr-host-python/tests/bridges.rs:51` (`a_bridged_pair_produces_a_loadable_artifact`)
tests only `("python","rust")`; line 98 still says "only the (python, rust) pair is bridged today".

## 8. README status prose vs. the shipped registry

`README.md:49` — "a **backend** turns IR into target source (`rust`; the same three reserved)"
`README.md:51` — "belongs to the `(source, target)` **pair** — `(python, rust)` today."
`README.md:186` — "Not built yet: ... the TypeScript, Go, and C++ backends (reserved names that
fail with a message saying so)."

```
$ cargo run -q -p compylr-cli -- --backend go --frontend typescript --emit summary $S/collide.ts
unit fingerprint: 9893abe24b5c7167
  lib (1 params) -> number
  main (0 params) -> number
  path (1 params) -> number
```
No message; it works. `README.md:86-90` simultaneously lists `typescript-frontend`,
`golang-backend`, `typescript-go-bridge`, `typescript-bindings`, `typescript-api` as capabilities.

## 9. The demo benchmark table names workloads the bridge cannot export

`demo/demo-ts-go/README.md` benchmark block rows vs. the 18 exported symbols:
exported = collatzLength, digitSum, floorDivide, gcd, integerSqrt, isPrime, iterativeNotDivisible,
iterativeNthPrime, larger, lcm, power, recursiveIsPrime, recursiveNextPrime, recursiveNthPrime,
recursiveNthPrimeFrom, remainder, smaller, squareRoot.

Of the 14 non-reference rows, only `arithmetic.collatz_length` names an exported function.
knapsack, matrixMultiply, sieve, standardDeviation, mergeSort, insertionSort, editDistance,
normalize, topologicalSort, matrixTranspose, isPalindrome, bfsDistances, wordCount all have
non-scalar signatures and no `Call_` symbol.

## 10. `compylr-host-typescript` is a stub

`crates/compylr-host-typescript/src/lib.rs` in full (8 lines):
```rust
//! Node-API host extension module for compylr.
use napi_derive::napi;
#[napi]
pub fn version() -> String {
    "0.1.0".to_string()
}
```
It declares dependencies on the TS frontend, Go backend, TS-Go bridge and registry and uses none
of them. `README.md:89` calls it "The Node-API addon exposing the compiler to Node".

---

# Follow-up pass — pushing past the prior audit

Re-verified §1-§7 above by re-running the export census and the `go build -buildmode=c-shared`
command from this session; counts (75 members / 18 exported / 56 in index.d.ts+index.js) and the
`node .compylr/go/index.js` `ReferenceError: require is not defined` all reproduced unchanged.
New ground below.

## 11. The Go backend drops constructor logic entirely — CRITICAL, and it is not a bridge bug

`crates/compylr-backend-golang/src/emit.rs:38-73` (`emit_class`) never reads `class.init.body` at
all. The constructor is synthesized purely from `class.init.params` matched **by name** against
`class.attributes`:

```rust
for attr in &class.attributes {
    let name = go_ident(&attr.name);
    if class.init.params.iter().any(|p| p.name == attr.name) {
        writeln!(out, "\t\t{}: {},", name, name).unwrap();   // only same-name aliasing
    } else {
        let zero = match &attr.ty { Ty::Int => "0", ... };    // everything else: hardcoded zero
        writeln!(out, "\t\t{}: {},", name, zero).unwrap();
    }
}
```

There is no loop, no conditional, no computed assignment, and no case for an attribute whose name
differs from the constructor parameter that initializes it. Any constructor doing more than
`this.x = x` for identically-named fields is silently discarded.

`demo/demo-ts-go/src/algorithms/structures.ts:62-77` (`UnionFind`) has exactly such a constructor:

```ts
constructor(size: number) {
    this.parent = [];
    this.rank = [];
    this.components = size;      // name differs from param "size"
    let i: number = 0;
    while (i < size) {           // loop body: never emitted
        this.parent.push(i);
        this.rank.push(0);
        i = i + 1;
    }
}
```

The checked-in generated Go (`demo/demo-ts-go/.compylr/go/generated.go:147-153`):

```go
func NewUnionFind(size int64) *UnionFind {
	inst := &UnionFind{
		components: 0,               // should be `size`
		parent:     make([]int64, 0), // should have `size` elements
		rank:       make([]int64, 0),
	}
	return inst
}
```

`size` is accepted as a parameter and never referenced anywhere in the function body — Go doesn't
even warn (unused *function* params are legal). Confirmed by compiling the checked-in files
standalone and calling it as plain Go (no cgo, no bridge involved at all):

```
$ cp demo/demo-ts-go/.compylr/go/*.go demo/demo-ts-go/.compylr/go/go.mod $S/gocheck/
$ cat >> $S/gocheck/probe_test.go   # TestClasses: NewUnionFind(5); union(0,1); union(1,2); connected(0,2)
$ go test -run TestClasses -v .
--- FAIL: TestClasses (0.00s)
panic: runtime error: index out of range [0] with length 0
	compylr.(*UnionFind).find(...)      generated.go:162
	compylr.(*UnionFind).union(...)     generated.go:179
```

`self.parent` and `self.rank` are permanently empty slices, so the first `self.parent[root]` read
inside `find` panics. This is a correctness defect in `compylr-backend-golang` itself — it fires
for pure Go execution with the cgo/bridge/koffi layer entirely out of the picture. It is squarely
one of the "57 members that never reach the boundary" this assignment asked about: reaching the
boundary is not the only way this pair is broken; the Go the backend hands the bridge for a
non-trivial class is already wrong before the bridge does anything.

The other three demo classes (`IntStack`, `PrimeCache`, `RunningStats`) happen to work only
because their constructors are trivial by coincidence — `IntStack`/`PrimeCache`/`RunningStats`
take no constructor parameters and initialize every field to a literal that matches the backend's
hardcoded zero value (`0`, `0.0`, empty collection). Verified the same way:

```
$ go test -run TestNonScalar -v .    # mergeSort, wordCount, standardDeviation, isPalindrome: all correct
$ go test -run TestClasses -v .      # IntStack push/pop/peek/depth: correct, BEFORE hitting UnionFind
```

So 3 of 4 demo classes mask the defect; 1 of 4 (the one with a non-trivial constructor) exposes
it. `crates/compylr-backend-golang/tests/emit.rs` has **zero** tests involving `Class`/constructors
at all (grepped the whole file — only one free-function test, reproduced above in full). Nothing
in the repository's test suite could have caught this: there is no differential-testing tier for
`(typescript, go)` at all. `frontends/python/tests/test_differential.py` and
`crates/compylr-host-python/tests/differential.rs` exist; grepping the whole tree for
`differential` outside `worktrees/` finds no TypeScript/Go analog. The mechanism that would run
generated Go and compare it against the TS source's own output — the thing that would have caught
this in one run — does not exist for this pair.

## 12. `loaded_as` names a file that nothing in the repository ever builds

`crates/compylr-bridge-typescript-golang/src/bridge.rs:25-29` computes
`loaded_as = compylr_generated_<fingerprint:016x>_<variant_tag>`, and `emit_js_loader` embeds that
exact string as the `.so` filename `index.js` will `require`/`koffi.load` at
(`bridge.rs:156: path.join(__dirname, '{module_name}.so')`). But grepping the whole repository
(`grep -rn "buildmode=c-shared" .` outside `target/`/`node_modules/`) finds exactly three build
invocations, all in CI, and all three hardcode a different, static name:

```
.github/workflows/golang.yml:54:      run: go build -buildmode=c-shared -o ../lib/compylr_generated_demo.so .
.github/workflows/golang.yml:87:      run: go build -buildmode=c-shared -o ../lib/compylr_generated_demo.so .
.github/workflows/typescript.yml:92:  run: go build -buildmode=c-shared -o ../lib/compylr_generated_demo.so .
```

`compylr_generated_demo.so` ignores both the fingerprint and the variant tag. There is no code
path anywhere in the repo — Rust, Python, TS, or CI — that runs `go build -buildmode=c-shared`
using the `loaded_as` value the bridge computed for the same unit. Contrast with the other bridge:
`crates/compylr-host-python/src/lib.rs:273` actually threads `artifact.loaded_as` into
`module_name` for the PyO3 build. No Node-side equivalent exists (`compylr-host-typescript` is the
8-line stub already reported as part of #39) — `grep -rn loaded_as` across the tree turns up only
the five lines already listed in §7 of the earlier pass, none of them a Go build driver. So even
setting aside the ESM/`require` and missing-`koffi`-dependency defects already filed, if those were
fixed today the loader would ask for `compylr_generated_<hash>_<tag>.so`, and the only artifact CI
ever produces on disk is `compylr_generated_demo.so` — a guaranteed file-not-found, and the two
names would only ever coincidentally match.

Separately, `HostArtifact.manifest` for this bridge (`bridge.rs:43`,
`files.get("go.mod").cloned().unwrap_or_default()`) is **not** a defect: `go.mod` is always the
fixed string `"module compylr\n\ngo 1.20\n"` (`compylr-backend-golang/src/golang.rs:72-73`,
confirmed by reading the file), which is a legitimate (if content-invariant) build manifest for a
Go module — nothing about it contradicts the `HostArtifact::manifest` doc comment in
`compylr-core/src/bridge.rs:71-72`. Flagging its invariance was considered and dropped: real
`go.mod` files for a fixed dependency set are legitimately static, unlike `loaded_as`/`.so` naming
above, which is actively contradicted by the CI build step.

## 13. `Str` parameters/returns: confirms and sharpens the earlier ABI finding

`is_scalar` (`bridge.rs:69-71`) is `Int | Float | Bool | Unit` only — `Ty::Str` (and every
collection/class type) is non-scalar, so **any** function taking or returning a string is skipped
by `emit_cgo_function` in full (no `//export`, no Go wrapper at all — confirmed already in §4's
`shout` probe: `Call_shout MISSING` from the built `.so`'s symbol table). What's new here: the two
*other* emitters do not agree with that skip. `emit_dts` (`bridge.rs:124-147`) and `emit_js_loader`
(`bridge.rs:149-189`) iterate `unit.functions()` unconditionally and unconditionally render every
parameter and return as `number`/`'int64'` regardless of `Ty`, string included — so `index.d.ts`
asserts a string-taking function has a fully numeric signature, and `index.js` would (if it ever
loaded) call `lib.func('Call_shout', 'int64', ['int64'])` against a symbol the library does not
contain. There is no gate anywhere that keeps the three emitters' member sets in sync; each walks
`unit.functions()` independently and only one of the three (`emit_cgo_function`) applies the
scalar filter.

## 14. Net judgment on this dimension

The `(typescript, go)` pair is not "partially working" — it is inert twice over. Even bracketing
the already-filed loader/runtime failures (ESM `require`, missing `koffi` dep, unhandled Go panics,
name collisions, 24% export coverage, zero classes exported), the code the Go backend itself
produces for a class with a non-trivial constructor is wrong before any bridge concern applies
(§11), and the one filename convention that would let a fixed loader find its library is
contradicted by the only build automation that exists for it (§12). None of this is bridge-layer
polish; recovering it needs, at minimum: (a) rewriting `emit_class`'s constructor emission to
actually lower `class.init.body` statements the way the Rust backend does (CLAUDE.md's own account
of "A constructor has no `self`" describes solving this exact problem for `compylr-backend-rust`;
`compylr-backend-golang` never did the equivalent work), (b) a real type-directed ABI in the cgo/JS
emitters for `Str`/collections/classes (not just widening `is_scalar`), (c) a differential test
tier for this pair so defects like §11 are caught the way `frontends/python/tests/test_differential.py`
catches them for `(python, rust)`, and (d) reconciling `loaded_as` with whatever actually invokes
`go build`. That is a rewrite of the bridge's marshalling layer plus a real gap in the backend's
class support, not an incremental patch to the existing 189-line bridge file.
