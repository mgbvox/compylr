# Multi-target transpilers: py2many, Haxe, Nim

Research leg `multi-target-transpilers`, run 2026-09-02. Never run before this session — the
decision record lists it among three legs "none would change a decision already made." That
prediction is evaluated at the end.

Question: is compylr's frontend/IR/backend/bridge split unusual among source-to-source compilers
that target multiple output languages, and where do comparable projects break down as targets
multiply? Primary source for the answer is local: `inspiration/py2many`, a git submodule with
thirteen backends, read directly. Haxe and Nim are compared as far as WebFetch allows against their
own manual pages.

## Confidence note on the Haxe/Nim material

WebFetch on this task returns one page at a time, HTML-to-markdown converted and then summarized by
a small fast model, not the raw page. Every Haxe/Nim fetch below came back saying the excerpt did
not contain the specific numeric-semantics detail asked for, even when asking directly about pages
titled for that content (`types-numeric-types.html`, `types-basic-types.html`). That is a genuine
limit of the tool on this task, not evidence the documentation is silent — cppreference 403s
outright, and the Haxe/Nim manuals are large multi-page documents that a single-page fetch samples
thinly. Treat every Haxe/Nim claim below as **low confidence, single-fetch, possibly incomplete**,
and every py2many claim as **high confidence**, since that one is read from source and tests
directly.

## 1. py2many's actual architecture

### There is no IR distinct from Python's own `ast` module

This is the load-bearing finding. compylr's pipeline is `tree → lower → IR → verify → passes →
backend`, with `compylr-ir` an independent crate no backend-specific or frontend-specific code may
touch. py2many has no equivalent stage. Its "frontend" parses Python with the standard library's
`ast` module and then a chain of `ast.NodeVisitor`/`ast.NodeTransformer` passes — living in the
shared `py2many/` package (`context.py`, `scope.py`, `tracer.py`, `inference.py`,
`declaration_extractor.py`, `mutability_transformer.py`, `rewriters.py`) — **mutates that same tree
in place**, attaching Python-specific attributes (`annotation`, `scopes`, `container_type`, and
similar) directly onto the `ast` nodes. Every backend then subclasses `CLikeTranspiler`
(`py2many/clike.py`) and walks that *same* annotated Python AST as an `ast.NodeVisitor`, emitting
target text directly from `visit_FunctionDef`, `visit_BinOp`, etc.

Concretely, from `pycpp/transpiler.py`'s own `transpile()` function:

```python
tree = ast.parse(source)
rewriter = PythonMainRewriter("cpp")
tree = rewriter.visit(tree)
add_variable_context(tree, (tree,))
add_scope_context(tree)
add_list_calls(tree)
add_imports(tree)
transpiler = CppTranspiler()
cpp = transpiler.visit(tree)
```

Every backend does the same shape of thing (confirmed in `pyrs/transpiler.py`, `pygo/transpiler.py`)
— parse Python, run the same shared annotation passes, then hand the *Python* tree to a
backend-specific visitor. There is no data structure in this codebase that is target-language- **and**
source-language-neutral. What compylr calls the IR, py2many does not have; what it has instead is
"Python's AST, with extra fields bolted on by shared passes." That is source-neutral only in the
narrow sense that it is the one frontend's AST — nothing here is what compylr's IR is, a form
several *different* frontends could lower into.

This directly answers "how unusual is compylr's split": py2many, arguably the most direct
multi-target comparator that exists (13 backends, all from one Python subset), does not have an IR
tier at all. It fuses frontend and IR into one artifact — the annotated Python AST — because it only
ever had one frontend to serve. compylr's IR crate having zero dependency on Rust, C++, or Python
concepts (enforced structurally by `tests/crate_boundaries.rs`) is a design decision py2many never
had to make, because py2many never entertained a second *source* language. This is worth stating
plainly: py2many is N-to-1-to-M in name (many backends) but 1-to-1-to-M in fact (one frontend,
fused with the shared representation). compylr's registry already supports M source frontends ×
N backends × a bridge keyed by the pair; nothing in py2many's architecture generalizes to a second
frontend without unfusing the AST from the Python-specific passes first — which is exactly what
compylr's IR crate already is.

### Backend cost is not what "shared IR" would suggest

Line counts per backend package (source + tests, not counting the shared `py2many/` package):

| backend | files | total lines |
| --- | --- | --- |
| pycpp | 6 | 777 (transpiler.py alone) |
| pyrs | 7 | 1185 (transpiler.py alone) |
| pygo | 5 | 1037 (transpiler.py alone) |

Each backend re-implements its own `clike.py` (backend-specific keyword/type mapping),
`plugins.py`/`DISPATCH_MAP` (a per-backend table mapping Python stdlib calls like `math.sqrt`,
`sys.stdout.write`, `range()`, `print()` to target syntax — see `pycpp/plugins.py`'s
`CppTranspilerPlugins`), and in Rust's and Go's case a full miniature AST of their own
(`rust_ast.py`, `cpp_ast.py`) used only to build the string the file becomes. That per-backend
`DISPATCH_MAP` is where compylr's "IR carries the resolved behavior, backend matches on the mode"
discipline (`CLAUDE.md`'s conventions section, `Expr::Subscript`'s index-origin/checked mode,
`BinOp::Rem`'s sign convention) has no counterpart: py2many backends decide Python-semantic
questions (what does `range()` mean, what does `str(x)` do, whether `int + int` needs widening)
independently, per backend, against the raw Python AST, rather than reading a resolved field a
shared earlier stage already decided. `TODO: take into account any imports happening in the file
being parsed and pass them into eval` sitting inside `clike.py`'s `class_for_typename` — used by
every backend — is a live instance of a semantic question (what does this Python name refer to?)
that stayed unresolved at the shared layer and is worked around per call site instead of decided
once.

### Divergent target capability is handled ad hoc, not negotiated

compylr has a structural vocabulary for "a target can't do this": `Checked`/`Unchecked` modes,
`Guarantee`s a backend declares it `PRESERVES`, `TargetOption`s a backend declares and can refuse
by name (`unchecked-arithmetic` on Rust, `cpp26-contracts` on the planned C++ backend), and reserved
frontend/backend names that fail with "planned" rather than silently doing nothing. py2many has none
of this as a structural mechanism. What it has instead, found by grep and read directly:

* **Fail at the point of translation.** `pycpp/transpiler.py:412`: `raise AstNotImplementedError(f"Call
  {fname} ({vargs}) not supported", node)`. `AstNotImplementedError` (`py2many/exceptions.py`) is a
  `NotImplementedError` subclass raised the moment a backend's visitor hits a construct it has no
  case for — there is no upfront capability check, no negotiation before compilation starts. The
  failure surfaces as a Python exception during transpilation of a specific node, not a declared
  refusal.
* **Degrade to a comment.** `tests/test_cli.py` passes `--comment-unsupported` to every case run,
  which (per the CLI, not read in full here but named directly in the flag and exercised by every
  test) turns an unsupported construct into an emitted comment rather than a hard failure — the
  opposite instinct from compylr's stance that a diagnostic should say why and where, not silently
  produce degraded output that still "compiles."
* **Reach for the target's own unstable/nightly features.** `pyrs/transpiler.py` accumulates a
  `self._features` set and emits `#![feature(generators)]`, `#![feature(generator_trait)]`,
  `#![feature(try_blocks)]` (lines 1097–1170) when a Python construct (generators, exception
  handling) needs Rust functionality still gated behind nightly. This is a real strategy — borrow a
  capability a target doesn't stably have yet — but it means the *artifact*, not the registry,
  is where you discover a program now needs nightly Rust. compylr's `TargetOption` mechanism
  exists specifically so this kind of trade is named and can be refused rather than discovered in
  generated output.
* **Document the gap in prose, per backend, by hand.** `doc/langspec.md` ("Supported Features" /
  "Not Supported Features") notes things like `## Functional programming — Algebraic data types via
  sealed classes (rust only)`, `Result[T, E] (rust only)`, `asyncio (rust only)`. There is no
  mechanism forcing this file to stay accurate as backends change — contrast with compylr's
  `scripts/update_subset.py`, which regenerates the README's subset matrix from which fixtures
  actually translated, built, ran, and matched CPython, so the documentation cannot overstate the
  implementation. py2many's own documentation has already drifted: `AGENTS.md` and
  `doc/agent/transpilers.md` both say "`py2many/transpilers/` contains all transpiler
  implementations" and name `tests/test_transpiler.py` as "the main test suite" — **neither exists**.
  The real per-language packages are top-level directories (`pycpp/`, `pyrs/`, `pygo/`, ...) and the
  real test file is `tests/test_cli.py`. This is exactly the failure mode compylr's generated-docs
  discipline (README subset matrix, benchmark tables) exists to prevent, caught here by simply
  trying to follow the docs' own directions.
* **`LANGUAGES.md`'s "Notable failures" column** (`fstring`, `stdio`, `math_func`, `coverage`,
  `global`, `cls`, `sys_argv`, `sys_exit`) is itself evidence divergent capability is tracked as a
  per-backend list of known-broken test categories, not resolved structurally. Rust passes 39/? with
  `math_func, stdio` still failing; the C++ backend fails on `fstring`; SMT (the weakest backend)
  fails on almost everything a real program would need (`hello_world`, `loop`, `global`, `cls`,
  `stdio`...). A backend at 7/64 passing is still listed as "supported."

### Numeric semantics: type-widening heuristics, not declared axes

`doc/langspec.md`: "Overflow protection (i8 + i8 is auto inferred as i16) for addition" — under
"Secure programming." This is the single closest thing py2many has to compylr's `BinOp::Add`
carrying a `checked` mode: instead of a declared, backend-matched axis, the *type inferencer* widens
the inferred type of an addition so the result type has headroom. It is inference-driven and
addition-only ("Underflow protection (for subtraction and possibly other ops)" is listed under TODO,
unimplemented). There is nothing resembling compylr's `Rounding`, `Checked`, index-origin, or
text-units axes — no rounding-mode field on division, no declared stance on what `//` means per
target, no `Guarantee` a backend claims to preserve. Division, indexing, and text length are handled
implicitly by whatever the target language's own operators do, which is precisely the "backend that
read the operation's name would be silently wrong for the other stance" failure compylr's
conventions call out by name as the reason `tests/conformance.rs` exists.

### There is a real compile-and-run differential tier — evidence this is achievable, and evidence of its cost

This directly bears on the add-cpp-backend decision record's #42 finding (the Go path renders
without compiling). py2many's `tests/test_cli.py` is not render-only: `COMPILERS` and `INVOKER`
dicts (lines ~76–125) hold real toolchain invocations per backend — `go build` / `go run`, `nim
compile --nimcache:.` then run, a Rust runner script that both compiles and runs, `dart compile exe`
then invoke, `z3 -smt2` for SMT (declarative, no run step — `is_declarative()` explicitly carves
that case out). `test_generated` actually runs the Python case under CPython first to capture
`expected_output`, generates the target source, and (implied by the compiler/invoker machinery
present, not fully read past line 320) builds and runs it for comparison. A missing toolchain
`pytest.skip`s naming the tool (`if not find_executable(settings.formatter[0]): raise
pytest.skip(...)`), which is exactly the failure-mode compylr's own `add-cpp-backend` design commits
to ("a missing toolchain is reported as *skipped* naming the tool, never as a pass").

The cost side is just as real: this tier requires a **golden expected-output file checked into the
repo per (case, language) pair** — `tests/expected/{case}{ext}` — compared byte-for-byte (module
whitespace/EOL normalization) against freshly generated output, with an `UPDATE_EXPECTED=1` escape
hatch to regenerate them by hand when a backend's emission legitimately changes. That is a second
N × M surface *beyond* the compile-and-run check itself: every one of 13 backends × however many
test cases owns a golden file that a human has to review on `UPDATE_EXPECTED=1`, not just a
pass/fail. `EXPECTED_COMPILE_FAILURES = ["test_dunder.v", "with.v"]` is an explicit allowlist of
known-broken (case, backend) pairs carried in the test file itself — a materialized admission that
some cells of the matrix are known not to work and are tracked by name rather than fixed. compylr's
corpus check works differently and, per `CLAUDE.md`, deliberately: fixtures are derived from a
directory listing (not a hardcoded list) precisely because a hardcoded list "drifted, and hid a real
defect."

### The registry mechanism looks similar on the surface, isn't underneath

`py2many/registry.py`'s `ALL_SETTINGS` dict mapping a target name string to a `LanguageSettings`
constructor is structurally close to `compylr-registry`'s pattern of named, resolvable components.
The difference is what each side of the mapping actually is: `LanguageSettings` bundles a
`CLikeTranspiler` instance, a file extension, formatter/linter command lists, and lists of
`ast.NodeVisitor` rewriter passes — all Python-AST-shaped. `compylr-registry` resolves a
`Frontend`, a `Backend`, or a `(source, target)`-keyed `&'static dyn HostBridge` from
`compylr-registry`'s bridges module, where the frontend and backend sides are IR-typed and the
crate-dependency graph (enforced by `tests/crate_boundaries.rs`) guarantees a backend cannot even
compile against Python-specific code. py2many has no equivalent guarantee — nothing stops a backend
package from importing something Python-specific, because everything downstream of parsing already
*is* Python-specific by construction.

## 2. Haxe (low confidence — see note above)

The fetched pages describe a real per-target divergence mechanism, but not a shared typed IR in the
excerpts retrieved:

* **`#if`/`#end` conditional compilation** tests compiler-set defines, including implicit
  target-name defines (the manual references `-D key=value` and target-specific built-in defines
  surfaced via `--help-defines`) and haxelib version defines. This is source-level, inline
  branching on which target is active — closer to C preprocessor `#ifdef PLATFORM_X` than to
  compylr's `Guarantee`/`TargetOption` negotiation, which happens *before* any target source exists,
  at the unit/backend-pairing level, and is refused by name rather than branched around in emitted
  code.
* **Externs** (`lf-externs.html`, referenced but not fetched) are how Haxe code declares
  target-native functionality it does not itself implement — the manual's own list names "Native
  Metadata" support alongside externs, suggesting the mechanism for "this target can do X and Haxe
  code should call through to it directly" is a declaration attached to the Haxe source, not
  something the compiler negotiates centrally.
* Numeric-type semantics per target (`Int`/`Float` width, precision, overflow behavior across
  JS/C++/PHP) is a real, well-known Haxe topic by title (`types-numeric-types.html`,
  `types-overflow.html` both exist as manual pages) but the single-page fetches returned only the
  type definitions ("Float ... double-precision IEEE 64-bit," "Int ... integral number") without the
  cross-target comparison the page titles promise. **Not established** by this session's fetches
  whether Haxe declares those differences structurally (a per-target capability table) or documents
  them narratively per target page the way py2many's `langspec.md` does. Given what *was* visible —
  unification rules between Int and Float are a language-level typing concern, not obviously backed
  by a compiler-level "guarantee" abstraction — the weight of evidence leans toward narrative/prose
  documentation per target page, matching py2many's pattern more than compylr's, but this is not
  confirmed by primary-source reading the way the py2many claims are.

## 3. Nim (low confidence — see note above)

`nim-lang.org/docs/backends.html` states directly, and this is a direct quote from the fetch, not
inference: "Features or modules that the JavaScript platform does not support are not available.
This includes: manual memory management (alloc, etc.), casting and other unsafe operations, file
management, OS-specific operations, threading, coroutines" plus certain stdlib modules. That is
Nim's own documentation stating a **reserved-capability list per target**, in prose, at the platform
level rather than the individual-construct level py2many hits with `AstNotImplementedError`. It
reads structurally closer to compylr's declared `Guarantee`/reserved-name approach than to py2many's
per-node exception — a target is *told* upfront what it cannot do, rather than discovering it
construct-by-construct at compile time — but it is still prose in a manual page, not a machine-
checked table the way compylr's subset matrix or bridge registry is.

The compilation model itself: Nim compiles to C, C++, Objective-C, or JavaScript by **generating
source in that target and shelling out to its native toolchain** — "transform a .nim file into one
or more .c files" then compile natively. No shared IR was mentioned in the fetched material (and the
fetch explicitly says so). This is architecturally the closest of the three comparators to
compylr's own backend contract (`Backend::emit` producing target source text, someone else compiling
it), but with the C-family targets sharing enough of a semantic model (manual memory, no built-in
GC assumption at the language level) that Nim's "one frontend, few very-similar C-like backends"
problem is smaller than compylr's "Rust, TypeScript/Go, and a GC-less C++ from one IR" problem.

Memory management is the one clearly-stated semantic fork found: JS backend has automatic GC and no
`NimMain()` init step; C-family backends require `NimMain()` initialization and manual care around
object/string lifetime at the boundary. This is the same *shape* of problem `add-cpp-backend`'s
design doc is solving for compylr's C++ target (D3's ownership discussion, D4's handle-release-once
rule) — a GC-vs-no-GC boundary is a recurring hard case across every one of these projects, not
something particular to compylr's choice of C++.

## 4. Answering the question

**Is compylr's frontend/IR/backend/bridge split unusual?** Among the three comparators read here,
yes, and specifically in the way that matters: none of them has a data structure that is
simultaneously **source**-language-neutral and **target**-language-neutral, enforced by a mechanism
independent of anyone's discipline. py2many fuses frontend and "IR" into one artifact (Python's own
`ast`, mutated in place) because it only ever needed to serve one source language — the fusion was
never tested by a second frontend, so nothing in its architecture proves or disproves whether it
would hold up under one. Nim and Haxe are compiler suites for their *own* single source language
targeting many backends — the same shape as py2many, not as compylr, which is explicitly designed
for **M frontends × N backends** with a **bridge keyed by the pair** as a distinct third thing
(`CLAUDE.md`: "bridges cost N × M while frontends and backends cost N + M"). None of the three
projects here has more than one source language, so none of them has had to solve — or could have
exposed a flaw in — the specific problem compylr's crate-boundary tests exist to guarantee: that the
shared representation cannot quietly grow a dependency on any one frontend or backend. The clearest
test of that claim would be a second compylr frontend actually landing (only Python exists today);
this research leg cannot substitute for that test, only note that none of the three comparators ever
faced it either.

**Where do these projects break down as targets multiply?** Three concrete mechanisms, all
observed directly in py2many rather than inferred:

1. **Per-backend duplication of semantic decisions.** Each backend's `DISPATCH_MAP`/`plugins.py`
   independently decides what Python's `range()`, `print()`, `str()`, integer overflow, etc. mean in
   that target — the same question answered N times, independently, rather than once upstream and
   matched on N times downstream. compylr's `BinOp`/`Expr` carrying resolved modes and backends
   matching on the mode (never the operation name) is the structural answer to exactly this failure
   mode, and `tests/conformance.rs` exists because a backend that instead read the operation's name
   would silently be wrong for the other stance — which is precisely what py2many's per-backend
   dispatch tables are exposed to with no test catching it structurally.
2. **Capability divergence tracked as prose and per-node exceptions, not negotiated upfront.**
   `AstNotImplementedError` raised mid-translation, `--comment-unsupported` silently degrading
   output, and a hand-maintained "Notable failures" column are all discovery-at-the-point-of-use.
   compylr's `TargetOption`/`Guarantee`/reserved-name mechanism exists to move that discovery earlier
   and make it a named, queryable refusal rather than an exception thrown partway through emitting a
   file, or a comment silently substituted for real output.
3. **A verification surface that grows as (cases × backends), with real cost.** py2many's
   `tests/expected/` golden-file-per-pair approach genuinely does compile and run generated code
   (unlike compylr's currently-broken Go path per the decision record), which is the right instinct
   — but it pays for that with a second N×M surface (checked-in expected output, hand-reviewed on
   `UPDATE_EXPECTED=1`) and a materialized escape hatch (`EXPECTED_COMPILE_FAILURES`) for pairs that
   just don't work. compylr's directory-derived fixture lists and generated subset matrix are a
   direct answer to the *drift* half of this problem (a hardcoded list "drifted, and hid a real
   defect" — CLAUDE.md, about compylr's own history); py2many's `AGENTS.md`/`doc/agent/` pointing at
   files that don't exist (`py2many/transpilers/`, `tests/test_transpiler.py`) is the same drift
   caught red-handed here, in a project that has no generated-doc discipline to prevent it.

Documentation-per-target as the record of divergence (py2many's `langspec.md`, and by the visible
evidence probably Haxe's per-target manual pages too) is inherently a linear cost per backend with
no cross-check — nothing forces the prose to match what the corpus actually proves, which is the
exact gap `scripts/update_subset.py` closes for compylr by generating the subset matrix from
fixtures that actually translated, built, ran, and matched CPython.

## 5. Does this change a decision?

**No.** Checked against both `research/DECISION.md` and
`openspec/changes/add-cpp-backend/design.md` before writing this section, per instructions.

Nothing here contradicts any of the seven numbered decisions in `design.md` (C++26 targeting,
`std::expected` for fallibility, pairwise nanobind/node-addon-api bridges over a C-ABI hub, flat
binding names, C++'s unchecked-but-preserving stance, the `compat.hpp` header plus refused
`cpp26-contracts` option, or toolchain preflight moving behind the backend). Nor does it touch
anything in `DECISION.md` §§1–3 (the audit findings, the C-ABI-hub cost numbers, nanobind-over-
pybind11, the C++26 floor correction, the Node correction, or "a C-ABI hub is a real pattern and not
free").

What this leg *does* do is retroactively support ground already staked out. `DECISION.md` §3 lists
this leg among three ("`python-native-compilers`, `multi-target-transpilers`, `semantics-mismatch`")
predicted to be "the lowest value of the ten — none would change a decision already made." That
prediction holds. Two places it independently corroborates existing reasoning, without changing
either:

* It is independent evidence for the crate-boundary-test discipline compylr already has (§ "IR
  independent of Python and of any target language" in `CLAUDE.md`) — py2many is the closest real
  comparator and it *doesn't* have that separation, and its lack of it is visibly where its
  per-backend duplication comes from.
* It is independent evidence for the "documentation must be generated, not asserted" pattern
  compylr already committed to (`scripts/update_subset.py`, `scripts/update_benchmarks.py`,
  `tests/readme.rs`) — py2many's own `AGENTS.md` has already drifted from its repo layout in exactly
  the way those scripts exist to prevent, caught by literally trying to follow it during this leg.

Neither observation asks for a new decision; both say an existing one was the right call, seen from
outside compylr's own codebase.
