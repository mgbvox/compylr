# Python-native compilers: integer semantics and the CPython boundary

Research leg `python-native-compilers`, run 2026-09-02. Scope per the brief: for each of Nuitka,
Cython, mypyc, Codon, Numba, Pythran, and the CPython 3.13+ JIT — what subset of Python it accepts,
and specifically how it handles the four integer-semantics gaps between Python and machine
arithmetic (arbitrary precision, floor vs. truncating division, overflow, remainder sign), plus
how each crosses back into CPython. Then: did anyone else land on compylr's answer (`int` is `i64`,
overflow is a declared behavior axis), and what did the ones who chose differently pay?

Sourcing note: WebSearch was exhausted for the session; every citation below is a `WebFetch` against
a primary source (official docs or the project's own GitHub-hosted source), quoted verbatim where
the tool returned exact text. `cppreference.com` was not touched (known to 403 WebFetch, out of
scope anyway). Nuitka's and Pythran's *published* doc pages were thin on this topic — noted inline
— so those two answers lean more on the projects' own developer-facing source than on user docs;
confidence is marked down accordingly.

## Summary table

| project | int representation | overflow | division (`//`, `%`) | subset strategy | CPython boundary |
| --- | --- | --- | --- | --- | --- |
| **Nuitka** | `PyObject*` (real `PyLongObject`, arbitrary precision) by default; native `C long` fast path with automatic fallback to the boxed form on overflow is **planned, not shipped** | not applicable by default — it's still a real Python bignum | unchanged — it's still CPython's operator, executed by CPython's own code | none: whole-program compile of full CPython semantics, not a subset | not a boundary at all — compiled code *is* the interpreter's replacement, same `PyObject*`s throughout |
| **Cython** | programmer's choice per variable: Python `int` object, or a declared C type (`cdef int`, `long`, …) | Python object: bignum, no overflow. C type: wraps per the C compiler, no default check | `cdivision=False` (default) rewrites the C `/`/`%` to match Python's floor-division/sign convention and raises `ZeroDivisionError`; `cdivision=True` uses raw C `/`/`%` (truncating, UB on zero) | subset only at annotation boundaries — untyped code is exactly Python | automatic marshalling at every typed function/parameter boundary (CPython C-API calls) |
| **mypyc** | `int` = arbitrary precision via a **tagged-pointer** representation (small values inline, falls back to a real bignum) by default; opt-in native `i64`/`i32`/`i16`/`u8` | default `int`: none, real bignum. Native signed types: **explicitly undefined** on overflow; unsigned `u8` wraps | not documented on the fetched page for either type | subset via annotation (`mypy`-checked Python), safe-by-default with an explicit unsafe opt-in | compiles to CPython C-extension modules; native types have no runtime tag/bounds check, `int` still does |
| **Codon** | `int` is **always** a 64-bit signed integer — not opt-in, no bignum fallback. `Int[N]` gives other fixed widths | C semantics ("some numeric operations use C semantics rather than Python semantics"); the `-numerics=py` flag restores Python-like checks (e.g. division-by-zero raising) but **does not** turn `int` back into arbitrary precision | not separately documented beyond the numerics-flag note | whole language recompiled to a stricter, statically-typed subset of Python | separate runtime; crosses into CPython through an explicit interop layer (not directly probed this leg) |
| **Numba** | fixed machine-word size via type inference in `@njit`/nopython mode; no bignum | "arithmetic operations can wraparound or produce undefined results or overflow" | not covered on the fetched reference page | subset: nopython mode compiles a restricted, typed core; object mode falls back to the interpreter per-operation | nopython-mode functions unbox Python arguments to native values at entry and rebox the return; object mode interleaves native and interpreted code directly |
| **Pythran** | `int` maps to `np.int_` → C `long`, i.e. whatever width the platform's `long` is (32-bit on some systems, 64-bit on others) | not discussed in the manual; implicitly C's (silent) | not documented | subset: no BigInt support at all, typed numeric core only | not probed this leg beyond the type-mapping quote |
| **CPython 3.13+ JIT** | unchanged — still `PyLongObject`, full arbitrary precision | unchanged — impossible, same as always | unchanged — same bytecode semantics | none — it JITs the *existing* Tier 2 IR to machine code (copy-and-patch), it does not retype or restrict the language | not a boundary — same interpreter, same objects, before and after |

## Detail and quotes, by project

### Nuitka

The published user manual (`nuitka.net/doc/user-manual.html` and its index) does not discuss integer
representation at all — confirmed by two separate fetches of different manual pages that came back
empty on this topic. **Confidence: medium**, resting on the project's own developer manual rather
than user-facing docs.

The `Developer_Manual.rst` (fetched from `raw.githubusercontent.com/Nuitka/Nuitka/develop/`) is
explicit that the current code generation is conservative:

> "Types are always `PyObject *`, and only a few C types, e.g. `nuitka_bool` and `nuitka_void` and
> more are coming."

and that a faster path is designed but not built:

> "The expansion with more C types is currently in progress, and there will also be alternative C
> types, where e.g. `PyObject *` and `C long` are in an enum that indicates which value is valid,
> and where special code will be available that can avoid creating the `PyObject *` unless the
> later overflows."

Read together: today, an `int` inside Nuitka-compiled code is a real `PyLongObject` behind a
`PyObject*`, exactly as it would be in the unmodified interpreter — arbitrary precision, Python's
floor division, no overflow because there's no fixed width to overflow. The performance story is
"compile the *interpreter's own operations* into a faster call sequence," not "give ints a smaller,
faster representation." The `C long`-with-fallback design described above is precisely the
tagged/boxed-hybrid idea mypyc ships today (see below) — Nuitka has designed the same shape and, per
its own developer manual, has not yet shipped it. Because Nuitka is a whole-program compiler
producing a standalone executable or extension that *is* the program, there is no separate
"boundary" to cross back into CPython — the compiled code already speaks in `PyObject*` throughout.

### Cython

`cython.readthedocs.io/en/latest/src/userguide/language_basics.html`:

> "While these C types can be vastly faster, they have C semantics. Specifically, the integer types
> overflow."

Left implicit but load-bearing: an untyped Cython variable is a plain Python object with plain
Python (bignum) semantics, and the "C semantics" sentence above only fires once you write `cdef int`
or similar. So Cython's answer is per-variable, not per-program: safe by default, opt-in-unsafe with
a type annotation — the same shape mypyc's `int` vs. native types takes, and unlike compylr and
Codon, which fix the answer for the whole language.

The `cdivision` directive (`source_files_and_compilation.html`) is the sharpest number in this
whole leg:

> "If set to False, Cython will adjust the remainder and quotient operators C types to match those
> of Python ints (which differ when the operands have opposite signs) and raise a
> `ZeroDivisionError` when the right operand is 0." ... "This has up to a 35% speed penalty." ...
> "If set to True, no checks are performed."

That is a direct, quoted price for exactly the guarantee compylr's C++ backend also chose to
preserve by default (`Guarantee::IntegerOverflowReported`-style safety, generalized here to
division/modulo correctness) — Cython's mechanism is a *global* directive flip, not a per-operation
mode threaded through an IR the way compylr's `BinOp::Rem`/`Expr::Subscript` carry their checking
mode as data. **Confidence: high** — this is an exact quote from the primary docs.

### mypyc

`mypyc.readthedocs.io/en/latest/int_operations.html` is the single most directly comparable design
to compylr's found this leg, because it names the same trade-off compylr names, using almost the
same vocabulary:

> "int (arbitrary-precision integer)" — the default, using "a more efficient runtime representation
> (tagged pointer)" — alongside four native types, "i64 (64-bit signed integer), i32 (32-bit signed
> integer), i16 (16-bit signed integer), u8 (8-bit unsigned integer)."

> "If one of the above native integer operations overflows or underflows with signed operands, the
> behavior is undefined." ... "Operations on unsigned integers (u8) wrap around on overflow."

> "Signed native integer types should only be used if all possible values are small enough for the
> type. For this reason, the arbitrary-precision int type is recommended for signed values unless
> the performance of integer operations is critical."

So mypyc ships *both* answers compylr had to choose between, as two distinct types the programmer
picks explicitly per variable: a safe, bignum-backed default `int`, and an `i64` that is exactly
compylr's chosen width but with overflow left **undefined** rather than **checked** — the opposite
resolution of the same axis compylr made a first-class, declared guarantee. mypyc's own guidance
("recommended... unless performance is critical") reads as tacit acknowledgment that `i64` is a
correctness trap for anyone who reaches for it without re-deriving the overflow analysis by hand —
which is exactly the failure mode compylr's `Checked`/`Guarantee` machinery exists to make
impossible to reach silently. **Confidence: high**, direct quotes from the primary reference page.

mypyc compiles to ordinary CPython C-extension modules (same PyO3-adjacent territory as compylr's
Rust bridge, but hand-rolled C-API rather than a binding library); this leg did not fetch the page
that would confirm marshalling costs at that boundary specifically.

### Codon

The doc site's rendered pages (`docs.exaloop.io/codon/...`) all came back as client-side redirect
stubs to WebFetch — the tool does not execute the JS redirect. The underlying markdown source,
fetched directly from `raw.githubusercontent.com/exaloop/codon/develop/docs/language/overview.md`,
resolved this cleanly. **Confidence: high** on the quotes below (primary source, exact text); note
that this is the doc source rather than the rendered site, though for a static-site generator the
content is identical.

> "Codon's `int` is a 64-bit signed integer, whereas Python's (after version 3) can be arbitrarily
> large." ... "Codon does support larger integers via `Int[N]` where `N` is the bit width."

> "For performance reasons, some numeric operations use C semantics rather than Python semantics.
> This includes, for example, raising an exception when dividing by zero, or other checks done by
> `math` functions. Strict adherence to Python semantics can be enforced by using the `-numerics=py`
> flag of the `codon` CLI. Note that this does *not* change `int`s from 64-bit."

This is the closest sibling to compylr's actual decision found this leg: `int` is unconditionally
`i64` — not opt-in, not a programmer's choice the way Cython's and mypyc's is — matching compylr's
"`int` is `i64`" exactly. Where the two diverge is the overflow axis specifically: Codon's own docs
name division-by-zero and `math`-function checks as what `-numerics=py` restores, and say nothing
about integer *overflow* being covered by that flag one way or the other. **This leg could not
establish whether Codon's strict-numerics mode traps integer overflow** — the fetched text is silent
on it by omission, not by explicit denial, so treat "Codon leaves overflow as C-style wraparound
even under `-numerics=py`" as a plausible but unconfirmed reading, **confidence: low** on that
specific sub-claim only. What is confirmed at high confidence is that Codon's fixed-width `int` is
not a global toggle away from bignum the way compylr's isn't either — both projects made `i64` the
one and only integer, then negotiated *other* semantics (division-by-zero, and in compylr's case
overflow too) as a separate, explicit axis rather than folding it into "is this Python-like or not."

### Numba

`numba.readthedocs.io/en/stable/reference/pysemantics.html`:

> "While Python has arbitrary-sized integers, integers in Numba-compiled functions get a fixed size
> through type inference (usually, the size of a machine integer). This means that arithmetic
> operations can wraparound or produce undefined results or overflow." ... "Type inference can be
> overridden by an explicit type specification, if fine-grained control of integer width is
> desired."

No default checking, no opt-in checked mode documented on this page, and no separate word on
floor-division/modulo semantics specifically (the page's other sections cover bounds-checking,
exception behavior, and global-variable semantics, not `//`/`%`). **Confidence: medium-high** on the
overflow quote (exact primary-source text); **could not establish** Numba's `//`/`%` stance from this
page — a gap, not a claim. Numba's nopython functions unbox arguments and rebox the return at the
Python/native boundary; object mode instead falls back to running unsupported operations through the
interpreter inline. This leg did not find a page confirming per-call marshalling cost figures for
Numba specifically (contrast: `research/python-call-overhead.md`, if it exists, would be the place
for that on the Rust/PyO3 side of compylr's own boundary).

### Pythran

The rendered docs site gave nothing usable (generic project description only); the manual source
fetched from GitHub (`serge-sans-paille/pythran/master/docs/MANUAL.rst`) had the one load-bearing
sentence:

> "There is no BigInt support. All integer operations are performed on `np.int_`, which maps to C
> `long` type. Beware that as a consequence, the size of this type is system-dependent."

This is a strictly worse position than compylr's on the axis compylr cared most about controlling:
compylr chose a *fixed* width (`i64`, same everywhere) specifically so a program's behavior does not
depend on where it runs; Pythran's `int` is `C long`, which is 64-bit on Linux/macOS and 32-bit on
Windows (LLP64) — the manual's own "system-dependent" warning names that exact footgun. No overflow
handling, checked mode, or division-semantics discussion was found in the manual beyond this.
**Confidence: medium** — one clear quote, but the surrounding manual is thin and this leg did not
find Pythran-specific pages on division or the CPython call boundary to corroborate or extend it.

### CPython 3.13+ JIT

`docs.python.org/3/whatsnew/3.13.html`, the experimental JIT section:

> "When CPython is configured and built using the `--enable-experimental-jit` option, a just-in-time
> (JIT) compiler is added which may speed up some Python programs." The mechanism: "the optimized
> Tier 2 IR is translated to machine code" via "copy-and-patch," with "no runtime dependencies" but
> "a new build-time dependency on LLVM."

This is the one entry in the table that isn't a Python-to-native-code *transpiler* in compylr's
sense at all: it JITs CPython's own existing Tier 2 bytecode IR into machine code, in-process, and
that IR already encodes full CPython object semantics (bignum ints, floor division, no overflow).
There is no representation change, no subset, and therefore nothing to trap, wrap, or promote —
"crossing back into CPython" is a category error here because the JIT never leaves it. **Confidence:
high** for the mechanism description (exact quotes); this is more a control than a data point, but
it's the one honest "no compromise, no restriction" answer in the set, purchased by staying inside
the interpreter rather than generating an external artifact.

## Answering the brief's question

**Did anyone else reach compylr's answer — `int` is `i64`, and overflow is a declared behavior
axis?**

Half of it, split across two different projects, and nobody found this leg has both halves at once:

- **The width half** (`int` is unconditionally `i64`, not a per-variable choice) matches **Codon**
  exactly, and nothing else surveyed: Cython, mypyc, and Numba all keep a bignum-or-native choice
  the programmer makes per annotation; Pythran fixes a *width* but not the *value* (`C long` varies
  by platform); Nuitka and the CPython JIT don't restrict the type at all.
- **The "overflow is a declared, checked axis rather than a silent stance" half** matches nothing
  found this leg with the same shape compylr uses. The closest analog is mypyc's `i64`, but mypyc
  resolves the axis the *other* way — signed native overflow is explicitly **undefined**, and safety
  is bought by not using `i64` at all (defaulting to bignum `int` instead). Cython's `cdivision` and
  Codon's `-numerics=py` are the only other projects that make *any* part of this configurable, and
  both are global compiler flags scoped to division/modulo/zero-checks, not a value carried on the
  operation itself the way compylr's `Checked` mode is carried on `BinOp::Rem`/`Expr::Subscript` and
  threaded through the IR, verifier, and backend per `CLAUDE.md`'s description of the behavior axes.

**What did the ones who chose differently pay?**

- **Cython's safe default has a quoted, specific price**: "up to a 35% speed penalty" for
  `cdivision=False`, i.e., for exactly the floor-division/modulo/zero-check correctness compylr's
  C++ backend also preserves by default. Cython's is a coarser instrument (one global flag covering
  every `/`/`%` in the module) than compylr's per-operation mode, but the number is a real,
  quotable cost for buying the same guarantee.
- **Nuitka's safe-by-construction default pays in the other currency**: not a division penalty, but
  a standing architectural one — every int stays a full `PyObject*`/`PyLongObject`, and the project's
  own developer manual describes the `C long`-with-overflow-fallback design (the same tagged/boxed
  hybrid mypyc ships) as still "in progress," not shipped. Read plainly: the design that would let
  Nuitka have both correctness and machine-int speed is known and wanted, and hasn't landed yet.
  That's evidence the hybrid is nontrivial to build well, which is relevant context for anyone
  tempted to propose it for compylr later.
- **mypyc's unsafe opt-in pays in documented risk, not measured slowdown**: choosing `i64` buys
  speed and buys "undefined behavior" on overflow as an explicit, named trade its own docs warn
  against reaching for casually. compylr's C++ backend, choosing the same width, paid a different
  price instead — `compat.hpp`'s overflow builtins, i.e., a runtime check on every checked-mode
  arithmetic op — to keep the guarantee rather than disclaim it.
- **Numba and Pythran pay in silence**: neither project's fetched docs describe any built-in checked
  mode for overflow at all; "wraparound or produce undefined results" (Numba) and an unremarked
  `C long` mapping (Pythran) are the whole story on the pages found. The cost there isn't a
  benchmarked number, it's an absent feature — a Numba or Pythran program that overflows its machine
  int does so silently, with no equivalent of compylr's `Checked::Checked` to opt back into a report.
- **The CPython JIT pays nothing on this axis**, because it isn't playing this game — it never
  changes the representation, so there's no trade to make. Useful as the control case, not as a
  point of comparison for backend design.

## What this changes for `add-cpp-backend`

**`changes_a_decision: false`.** Nothing here contradicts a decision recorded in
`research/DECISION.md` or `openspec/changes/add-cpp-backend/design.md`. `DECISION.md` §3 listed
"Whether compylr's behavior axes are novel" as an open, non-load-bearing gap — explicitly *not* a
recorded decision — and this leg answers that gap rather than overturning anything: the axis-based
design (a `Checked` mode carried as IR data, independent of any one target language's native stance)
has no full match among the seven projects surveyed. The nearest partial matches (mypyc's `i64`,
Codon's fixed-width `int`) each solve half the problem and leave the other half either unsafe
(mypyc: UB on overflow) or unspecified (Codon: silent on whether overflow is covered by
`-numerics=py`). That is corroborating evidence for, not a challenge to, design decision 5 in
`design.md` ("C++'s stance is unchecked; the backend preserves all three guarantees anyway") — the
project surveyed that comes closest to compylr's posture (Cython's `cdivision=False`) pays a
quoted, real cost (35%) for a coarser version of the same guarantee, which is a useful data point if
`compat.hpp`'s overflow-checked path is ever benchmarked against an unchecked build: a double-digit
percentage overhead for checked arithmetic would not be an outlier among comparable projects, it
would be roughly in line with the one comparable number that exists.

One concrete, low-cost follow-up worth flagging rather than acting on now: Pythran's "`int` maps to
system-dependent `C long`" is a real, named footgun in a project chasing exactly compylr's kind of
numeric-performance goal, and it's evidence *for* the choice already made — compylr's `int` is `i64`
everywhere, not "whatever the host's native word size is" — but this was not an open question this
leg was asked to resolve, so it is filed here rather than turned into a task.
