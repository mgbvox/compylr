# Semantic mismatch across transpiler backends: prior art, and whether compylr's axes are novel

Primary source: `/Users/mgb/RustRoverProjects/compylr/inspiration/py2many` (git submodule, read
directly — no fetch needed). Thirteen backends from one Python AST: Rust, C++, Go, D, Dart, Kotlin,
Nim, Zig, V, Julia, Mojo, Lean, SMT. Secondary: compylr's own `crates/compylr-ir/src/behavior.rs`
and `crates/compylr-backend-rust/src/runtime.rs`, read as the contrasting design. External web
sources were **not** used — WebSearch is exhausted for this session and cppreference.com 403s to
WebFetch — so anything not sourced from local code is marked with its confidence and the reason it
is unverified this session.

## The question

Does prior art make semantic axes (rounding, sign, overflow, indexing origin, length units)
**explicit and configurable per operation**, the way compylr's IR carries a `Rounding`, a `RemSign`,
a `Checked`, an `IndexOrigin`, and a `TextUnits` on the operations that need them
(`crates/compylr-ir/src/behavior.rs:36-51` for the `Axis` enum, `:179-191` for the
`LanguageBehavior` struct that carries each axis's resolved mode)? Or does everyone else pick one meaning per target
language, bake it into the code generator, and either document it as a limitation or not mention it
at all?

**Finding: py2many does the latter, uniformly, across all six axes checked, and the choice is not
even documented as a limitation — it does not appear in `LANGUAGES.md`, `README.md`, or any test
case.** This is a live, thirteen-backend, general-purpose transpiler, not a strawman.

## What py2many actually does, read from source

### Integer division and remainder: one symbol table, no rounding logic anywhere

`py2many/clike.py:64-65` (the base `CLikeTranspiler` every backend inherits from):

```python
symbols = {
    ...
    ast.Div: "/",
    ast.FloorDiv: "/",
    ast.Mod: "%",
    ...
}
```

Python's `//` (floor division — rounds toward negative infinity) and `/` (true division) are mapped
to the **same** target symbol, `/`. Every backend checked (`pyrs/clike.py:89`, `pygo/clike.py:132`,
`pyd/clike.py:102`, `pykt/clike.py:133`, `pydart/clike.py:112`) either uses this table unmodified or
only special-cases `ast.Mult`/`ast.Div` for spacing, never for rounding. `pycpp/clike.py` has no
override at all.

C's `/` (and Rust's, Go's, Kotlin's, Dart's, C++'s) truncates toward zero. Python's `//` floors. They
agree for positive operands and **disagree for any negative operand**: `-7 // 2` is `-4` in Python,
but the emitted `-7 / 2` is `-3` in every one of these target languages. This is not a rare edge
case — any Python function doing modular arithmetic, hashing, or array wrap-around with signed
operands hits it silently. `%` has the matching problem: Python's `%` takes the sign of the
**divisor**; C/Go/Rust/Kotlin/Dart's `%` takes the sign of the **dividend**. Same table, same
one-symbol mapping, same silent divergence.

Confirmed no backend overrides this for correctness: `grep -rn "FloorDiv"` across the whole tree
turns up only the symbol-table entries above, in six `clike.py` files, and never a rounding
correction. **Confirmed no test exercises it either** — `grep` for negative literals near `//`/`%`
in `tests/cases/*.py` returns nothing, and there is no `div`/`mod`/`negative`-named test file. The
gap is not merely unhandled in code; it is untested, which is consistent with it being unknown to
the maintainers rather than a documented, deliberate simplification.

### Negative indexing: same story, and it gets worse per-language

`py2many/clike.py:192-202`, `_slice_value`, explicitly raises `AstNotImplementedError` for an
`ast.Slice` node (`a[1:5]`, `a[-3:]`) — so *range* slicing is a known, guarded gap. But a **single**
negative index, `a[-1]`, is a plain expression under `ast.Constant`/`ast.UnaryOp`, not `ast.Slice`,
so it sails through the guard.

What each backend then emits:

- **C++** (`pycpp/transpiler.py:633-645`, `visit_Subscript`): `return f"{value}[{index}]"` — emits
  `value[-1]` verbatim. `operator[]` on `std::vector`/`std::string` takes an unsigned `size_type`;
  passing a negative `int` converts it to a huge unsigned value. Out-of-bounds read, undefined
  behavior, no exception, no diagnostic.
- **Rust** (`pyrs/transpiler.py:898-921`, `visit_Subscript`): for a `List`-typed value, the index is
  cast — `index = self._cast(index, "usize")`, i.e. `(-1) as usize`. That wraps to
  `18446744073709551615`, so `value[(-1) as usize]` panics with an out-of-bounds index at runtime —
  not a translation of "last element," a crash with a confusing message.
- **Go** (`pygo/transpiler.py:741-750`, `visit_Subscript`): `return f"{value}[{index}]"`, same as
  C++. Go slice indexing with a negative `int` panics at runtime with "index out of range."

Three different failure shapes (silent UB, a wrapped-index panic, a range panic) for the same one
Python idiom, across three backends of the same project, none of them a translation of what
`a[-1]` means. compylr's own runtime code names this exact bug class and rejects it by construction
— see below.

### String length: units never distinguished from list length

`pyrs/plugins.py:203`: `"len": lambda n, vargs: f"{vargs[0]}.len() as i32"`.
`pycpp/plugins.py:88`: `"len": lambda n, vargs: f"static_cast<int>({vargs[0]}.size())"`.

One dispatch entry for the Python builtin `len`, used for both lists and strings. Rust's
`String::len()` and C++'s `std::string::size()` both count **UTF-8 bytes**; Python's `len(str)`
counts **Unicode code points**. `len("café")` is `4` in Python and `5` in the emitted Rust/C++ for
any string containing a character outside the ASCII range. There is no branch anywhere in either
`plugins.py` that asks whether the receiver is a string versus a list before choosing the target
method — the single lambda is applied uniformly, so there is no place a units distinction *could*
live even if someone wanted to add it later without restructuring the dispatch table.

### Integer overflow: type-widening heuristic, not a reported failure mode

`py2many/inference.py:439-462`, `_handle_overflow`, is the only "overflow" handling in the tree
(confirmed by `grep -rn overflow` across every backend's `inference.py` and `pycpp/transpiler.py` —
all other hits are unrelated uses of the English word). It is a **compile-time type-widening
heuristic**: `i8 + i8` infers as `i16`, based on a fixed rank table
(`self.FIXED_WIDTH_INTS_LIST`), one step wider for `Add`/`Mult`. It is explicitly heuristic and the
authors know it — the code carries its own doubt in a comment right above the widening branch:

```python
# Does this hold across all languages?
if left_id == "int":
    left_id = "c_int32"
```

This is a mitigation that reduces *how often* overflow is silently wrong, not a semantic contract:
it has no runtime check, no way to opt into a reported failure, no per-operation control, and no
interaction with the sign-and-magnitude questions above. Python integers are unbounded; every target
language's are not; py2many's answer is "guess a wider fixed type and hope," which is a real
technique (compylr's Rust backend's own emitted arithmetic is unchecked by default — see
`CLAUDE.md`'s `RUST_BEHAVIOR`) but is not a *declared, resolved, per-axis* choice the way compylr's
`Checked::Checked | Unchecked` is. There is nothing to turn on if you want overflow reported; the
capability does not exist in generated code at all.

### The project's own scorecard doesn't test for any of this

`LANGUAGES.md` is py2many's conformance matrix — 64 Python test cases, pass/fail per backend
("stdio", "fstring", "math_func" etc. named as "Notable failures" per language). None of the six
axes above appears as a tracked category. The matrix answers "did this syntax transpile and run,"
never "did it answer the same thing CPython answers for a negative operand or a non-ASCII string" —
which is exactly the differential-correctness question compylr's own conformance and demo tiers ask
(`CLAUDE.md`: "never assert on... iteration order" is the one place compylr documents an
*intentional* semantic gap, and it is documented specifically because everything else is not
supposed to have one).

## Is there prior art for making axes explicit and configurable per operation?

**Not found in py2many, at any of the six axes checked, across any of the thirteen backends.** The
uniform pattern is: pick the target language's native operator/method, emit it, and either don't
notice the mismatch (division, remainder, string length) or half-guard it in a way that changes the
failure mode without fixing the semantics (negative indexing, overflow).

Two adjacent, **not independently verified this session** data points from training data, offered
at reduced confidence because WebSearch is exhausted and I could not re-check them against a primary
source:

- **Cython's `cdivision` directive** (medium confidence, not re-verified live): Cython — a real,
  widely used Python-to-C compiler, distinct from py2many — is reported to default `//`/`%` on C
  integers to Python's floor/sign-of-divisor semantics with a runtime zero-check, and lets a user
  opt into raw C division/remainder semantics via `# cython: cdivision=True`, at the level of a
  module or function. If accurate, this is the closest real prior art to "explicit and
  configurable" I am aware of — but it is a **coarse, opt-in flag per compilation unit**, not a
  per-operation mode carried on each expression the way compylr's `BinOp::Div` carries a `Rounding`
  and `Checked` value in the IR itself (`crates/compylr-ir/src/behavior.rs:179-191`). It also only
  covers one axis (division/remainder rounding); I have no comparable recollection of a
  per-operation length-units or indexing-origin control in any transpiler.
- **WebAssembly's instruction set** (higher confidence — stable, well-documented spec, but likewise
  not re-fetched this session): Wasm keeps `i32.div_s`/`i32.div_u` as genuinely distinct
  instructions and specifies trapping on divide-by-zero and on `INT_MIN / -1`, rather than folding
  signed/unsigned division into one opcode with implementation-defined behavior. That is evidence a
  language-*design* team, not a transpiler team, found this exact ambiguity important enough to
  make structurally unavoidable — which is closer in spirit to compylr's axis-per-operation IR
  field than anything found in py2many, but it is a virtual-machine ISA decision, not a
  source-to-source transpiler's answer to a *mismatch between two other languages*.

Neither is the thing being asked about — an explicit, per-operation, resolved-and-carried-in-the-IR
axis spanning rounding, sign, overflow, indexing origin, *and* text units together, checked against
what py2many actually ships. On the narrow question "does prior art make these choices explicit and
configurable per operation, the way compylr's `Axis` enum does" — **no instance of that was found**,
in the one real multi-backend transpiler read directly, and I was not able to search further this
session.

## Concrete bug-report-shaped evidence

py2many itself does not have a public issue tracker read in this session (no web access), so "what
the bug reports looked like" is answered by **reading the defects directly out of the source and
the absent test coverage**, which is the strongest evidence available without web access — these are
not reported-and-fixed bugs, they are **latent, unreported, unfixed defects visible in the current
tree**:

1. `pyrs/transpiler.py:898-921` — `a[-1]` on a Rust-typed list emits a cast to `usize` that panics.
   No corresponding Python semantics preserved; a Python program that reads the last element of a
   list crashes instead when transpiled.
2. `pycpp/transpiler.py:633-645` — the same idiom emitted to C++ is undefined behavior rather than a
   crash, which is worse: it may silently read adjacent memory rather than fail loudly.
3. `pyrs/plugins.py:203`, `pycpp/plugins.py:88` — `len()` on non-ASCII text is silently wrong by a
   language-dependent amount (bytes-per-codepoint), with no path to notice unless a test happens to
   use non-ASCII input, which `tests/cases/` does not (not confirmed exhaustively, but no `café`-
   style literal was found in a targeted grep).
4. `py2many/clike.py:64-65` — every negative floor-division or modulo result is wrong across every
   one of thirteen backends, for any operand pair with mismatched signs.

## Contrast: what compylr already does, read from source

This is the part that matters for the design decision, and it is why this leg does not change one:
compylr's IR and Rust backend already treat every one of py2many's four gaps as a named, resolved
axis, and in two cases the code contains a comment that reads like a direct rebuttal of the exact
bug found above.

- **Negative indexing.** `crates/compylr-backend-rust/src/runtime.rs:477-491`, `resolve_index`:

  ```rust
  IndexOrigin::FromEitherEnd if index < 0 => index.saturating_add(length as i64),
  // Left as it is, so it fails the range check rather than wrapping into an enormous
  // positive index — which is what a target's native indexing would do with it.
  IndexOrigin::FromEitherEnd | IndexOrigin::FromStart => index,
  ```

  "what a target's native indexing would do with it" is precisely the `(-1) as usize` wrap found
  live in `pyrs/transpiler.py:898-921` above. compylr's runtime resolves the negative offset against
  the collection's length *before* ever converting to `usize`, and rejects it as a bounds error
  rather than wrapping. The IR carries `IndexOrigin` as a mode on `Expr::Subscript`
  (`crates/compylr-ir/src/ir.rs:307`, referenced from `behavior.rs:44-46`) precisely so the backend
  never has to guess which of Python's or Rust's indexing rules a given subscript means.

- **Text length units.** `crates/compylr-backend-rust/src/runtime.rs:641-645`:

  ```rust
  TextUnits::CodePoints | TextUnits::Utf16Units if value.is_ascii() => value.len() as i64,
  TextUnits::CodePoints => value.chars().count() as i64,
  TextUnits::Utf8Bytes => value.len() as i64,
  ```

  This is the exact fork py2many's single `len` lambda cannot express: Rust's native `.len()`
  (UTF-8 bytes) is used only when the resolved axis actually calls for byte counting; when the
  Python frontend's `TextUnits::CodePoints` (`crates/compylr-frontend-python/src/component.rs:73`)
  is what was asked for, it decodes and counts code points instead, with an ASCII fast path noted in
  a comment as an explicit performance/correctness tradeoff rather than an accident. This is the
  `"café".len()` bug from `pyrs/plugins.py:203` and `pycpp/plugins.py:88`, resolved by construction.

- **Division rounding and remainder sign.** `crates/compylr-ir/src/behavior.rs:179-191`,
  `LanguageBehavior`, carries `integer_division: IntegerDivision` and `remainder: Remainder` as
  distinct struct fields (each in turn holding a rounding/sign plus a `Checked` mode, per
  `crates/compylr-ir/src/ir.rs`), not a single shared operator symbol. This is structurally what
  py2many's shared `ast.FloorDiv: "/"` / `ast.Mod: "%"` table cannot do even in principle — the axis
  exists as data the backend must consult, rather than a syntax-level substitution.

- **Overflow.** `Axis::IntegerOverflow` (`crates/compylr-ir/src/behavior.rs:37-39`) resolves to a
  `Checked` stance on the `integer_overflow` field of `LanguageBehavior` (`:181`), distinctly from py2many's
  compile-time type-widening guess — compylr's Rust backend is `Unchecked` on this axis by design
  (`CLAUDE.md`, `RUST_BEHAVIOR`) but that is a *stated* stance with a documented reason
  (`Checked::Unchecked` is "a statement about the program, not a promised machine result"), not an
  unexamined default.

## Does this change a decision already recorded?

**No — checked against both documents as instructed.**

- `research/DECISION.md`'s own gap table already anticipated this outcome: "`semantics-mismatch`...
  Never run. Lowest value of the ten — none would change a decision already made" and "Whether
  compylr's behavior axes are novel... Folded into the above; interesting, not load-bearing." This
  leg confirms that framing rather than overturning it: the axes are not proven *novel* in any
  absolute sense (Cython's coarser `cdivision` flag and Wasm's split opcodes are adjacent, if
  unverified this session), but no prior art was found that does what compylr does — per-operation,
  IR-carried, resolved-and-checked axes spanning rounding, sign, overflow, indexing origin, and text
  units together — and the one real multi-backend transpiler read in full (py2many) has live,
  unfixed, untested defects in exactly the shape compylr's design already guards against.
- `openspec/changes/add-cpp-backend/design.md` §"Decisions" #5 ("C++'s stance is unchecked; the
  backend preserves all three guarantees anyway") and #2 (`std::expected` propagation) are about
  *how the C++ backend resolves and preserves* these same axes, not about *whether axes should
  exist*. Nothing found here bears on the C++26 standard selection, the nanobind/node-addon-api
  bridge split, the `std::expected` error channel, or the toolchain-preflight decisions in that
  document. No contradiction; this leg is orthogonal to it, exactly as the decision record
  predicted.

## Confidence summary

| Claim | Confidence | Basis |
| --- | --- | --- |
| py2many maps `FloorDiv`/`Div` to one target symbol across all six backends checked, no rounding correction | High | Direct source read, `py2many/clike.py:64-65` + six backend `clike.py`/`transpiler.py` files |
| py2many emits negative single-index subscripts unguarded, with three different failure shapes across Rust/C++/Go | High | Direct source read, `pyrs/transpiler.py:898-921`, `pycpp/transpiler.py:633-645`, `pygo/transpiler.py:741-750` |
| py2many's `len()` dispatch does not distinguish string byte-length from codepoint-length | High | Direct source read, `pyrs/plugins.py:203`, `pycpp/plugins.py:88` |
| py2many's overflow handling is compile-time type-widening, not a runtime-reportable axis | High | Direct source read, `py2many/inference.py:439-462`, confirmed no other `overflow` hits in the tree |
| None of the above is tracked in py2many's own test matrix or docs | High | `LANGUAGES.md` read in full; targeted greps of `tests/cases/` for negative-operand and non-ASCII literals found nothing |
| Cython's `cdivision` directive behaves as described | Medium | Training data only, not re-verified this session (WebSearch exhausted, cppreference blocked) |
| WebAssembly's `div_s`/`div_u` split and trap semantics as described | Medium-high | Training data only, not re-fetched this session, but stable long-published spec content |
| compylr's `resolve_index`, text-units dispatch, and `Axis` design solve exactly these four gaps | High | Direct source read, `crates/compylr-backend-rust/src/runtime.rs`, `crates/compylr-ir/src/behavior.rs` |
| No broader prior art beyond py2many, Cython, and Wasm exists for per-operation configurable axes | Low | This session had no web access; only what was in reach (one submodule + training-data recollection) was checked. A real literature/prior-art search (LLVM's `-ftrapv`, Zig's `@divFloor`/`@divTrunc`/`@divExact` family, Ada's explicit `mod`/`rem`, Swift's `&+`/`&-` overflow operators) was not performed and could well surface closer analogues. |
