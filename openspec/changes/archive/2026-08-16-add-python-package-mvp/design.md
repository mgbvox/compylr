## Context

See proposal.md — Why. The relevant current state is narrow:

* The crate is a `bin` plus a `lib` with no target-language code anywhere in it.
* `Unit` already aggregates functions from independently-parsed sources, orders them
  deterministically by name, and fingerprints them order-independently.
* Lowering already resolves the two things a backend would otherwise have to re-derive:
  **operands arrive pre-widened**. `a / b` on two integers lowers to
  `TrueDiv(ToFloat(a), ToFloat(b))`, and mixed-type arithmetic likewise carries explicit
  `ToFloat` nodes. `build_binary` states this outright — *"operands are widened to the result
  type so a backend can emit them positionally"*. The backend therefore never implements
  promotion; it emits operands as it finds them.
* Call resolution is deliberately **not** part of lowering. `lower_function` validates one
  function against the subset; `Unit::validate` resolves callees. That split is what makes
  results independent of decoration order, and it dictates the two-stage validation below.

Verified toolchain on this machine: Python 3.14.5, uv 0.11.21, maturin 1.9.6, cargo 1.97.1.

## Goals / Non-Goals

**Goals:**

* One coherent story for the two different PyO3 roles, so they never get confused.
* Keep `rust-backend` free of any Python awareness, so the Rust backend stays usable by a
  future non-Python consumer and the binding layer stays swappable.
* Make the failure modes loud: no silent interpretation, no silent wrapping, no silent skip.
* Pay the build cost once per project, not once per decorated function.

**Non-Goals:**

* Making the *first* call fast. It compiles a Rust crate; it will take seconds.
* Removing the local Rust toolchain requirement (that is a wheel-distribution change).
* Any optimization of emitted Rust. Correct and readable beats clever.

## Decisions

### D1. Two crates, two PyO3 roles

| | Built when | Contains | Module |
| --- | --- | --- | --- |
| this repo | at package build | frontend, lowering, IR, backends | `compylr._core` |
| generated | at runtime, first call | the user's compiled functions | `compylr_generated` |

*Alternatives:* reimplement the frontend and lowering in Python — rejected, it creates a second
source of truth for the supported subset, and the two would drift the first time a rule
changed. Shell out to the CLI — rejected: it still requires shipping a binary, so it defers the
packaging problem rather than solving it, and diagnostics would have to be re-parsed out of
formatted text after having been structured data moments earlier.

### D2. Emit a pure-Rust layer and a binding layer separately

The generated `lib.rs` has three parts:

```rust
mod runtime   { /* helpers + RuntimeError; no PyO3 */ }
mod generated { /* fn add(a: i64, b: i64) -> Result<i64, RuntimeError>  */ }
              /* #[pyfunction] wrappers + #[pymodule], mapping RuntimeError -> PyErr */
```

This is what lets `rust-backend` own translation and `python-bindings` own the boundary, matching
the capability split rather than cutting across it. Generated functions call each other through
the inner layer with `?`, so a nested failure propagates without crossing the Python boundary
twice.

*Alternative:* emit a single `#[pyfunction]` layer. Rejected — it welds PyO3 into the Rust
backend, so a Go backend or a plain-Rust consumer would inherit Python types.

### D3. Inner functions uniformly return `Result<T, RuntimeError>`

Not "only when the body can fail". A conditional return type creates two emission paths and a
trap: a function that becomes fallible after an edit changes signature, and every caller must
change with it. Uniformity costs one `Ok(...)` and some `?`.

### D4. Runtime helpers are emitted inline, not depended on

The generated crate must build on a user's machine where this repo does not exist, so a path
dependency is impossible and a published `compylr-runtime` crate would mean release-managing a
second crate for roughly forty lines. The helpers are emitted from a single `const` in the
backend, so there is still one source of truth in *this* repo.

Python semantics, one rule serving both `//` and `%`, integers and floats alike — adjust when
the remainder is non-zero and its sign disagrees with the divisor:

```rust
// floor division
let q = a.checked_div(b).ok_or(RuntimeError::Overflow)?;   // also catches i64::MIN / -1
let r = a % b;
if r != 0 && ((r < 0) != (b < 0)) { q - 1 } else { q }
// remainder
if r != 0 && ((r < 0) != (b < 0)) { r + b } else { r }
```

Zero divisors are checked before dividing, for floats too: Python raises `ZeroDivisionError`
where IEEE-754 would produce infinity, so `1.0 / 0.0` must not be allowed to return `inf`. This
applies to `TrueDiv` as well, which is otherwise the one operator the backend emits natively.

Overflow uses `checked_*`, because release builds wrap silently and a silently negative result
is the worst possible outcome for a tool whose entire pitch is "same semantics, faster".

### D5. Fully parenthesized emission, then best-effort `rustfmt`

The spec requires IR grouping to survive regardless of Rust's precedence table. Emitting
parentheses around every binary node makes that true by construction instead of by a
precedence table that has to be kept correct. The output is then piped through `rustfmt`, which
is present wherever `cargo` is; if it is missing, the unformatted source is written and
correctness is unaffected.

### D6. Backend registry with a three-way answer

```rust
enum Entry { Implemented(&'static dyn Backend), Reserved }
```

The spec distinguishes implemented / reserved-but-unimplemented / unknown. A registry expresses
that directly; an enum of implemented backends could not represent `Reserved` without a second
list. `rust` is implemented; `typescript`, `go`, and `cpp` are reserved.

### D7. Serialization omits spans

`Span` is a byte offset into a source text that the artifact does not contain, so it is
meaningless once serialized — and including it would make the artifact differ for sources that
differ only in comments and indentation, violating the determinism requirement. This is
consistent with the IR's existing definition of structure: the fingerprint requirement already
enumerates structure as *name, parameter names and types, return type, and body*, and spans are
not in it. Round-tripping therefore restores default spans, and equality is asserted over that
same structural content.

`Literal::Float` already stores `f64::to_bits()`, so JSON carries an integer and round-trips
bit-exactly, including `-0.0` — a nice payoff from a decision made two changes ago for
unrelated reasons (`f64` implements neither `Eq` nor `Hash`).

### D8. Artifact layout, in the project, not a cache directory

```
.compylr/
  ir/unit.json         the IR artifact
  crate/               the shared generated crate
    Cargo.toml  pyproject.toml  src/lib.rs
  state.json           {"fingerprint": ..., "module": ..., "built": ...}
```

A platform cache directory would be invisible, and the spec's point is that the intermediates
are *inspectable*. Rooted at the working directory when `initialize()` runs. `.compylr/` is added
to this repo's `.gitignore` and the README tells users to do the same.

### D9. Build with maturin, install with uv

`maturin build --release --out <dir>`, then `uv pip install --force-reinstall <wheel>`, falling
back to `pip` when uv is absent, then `importlib.invalidate_caches()`.

*Alternative:* `maturin develop`. Rejected — it requires `VIRTUAL_ENV` to be set and mutates the
environment implicitly, which behaves unpredictably under uv-managed projects. `--release`
rather than a debug build: a debug build compiles faster but runs several times slower, which
defeats the purpose of the tool.

### D10. Two-stage validation, matching the compiler's existing split

* **At decoration** — capture source, lower *that function alone*. Subset violations raise here,
  with `line:column`, before the function is ever called. No codegen, no build; this is
  microseconds.
* **At first call** — assemble every registered function into one `Unit`, validate (this is where
  cross-function calls resolve), emit, build, install, import, swap.

The consequence is worth stating because it will surprise someone: a call to a function that was
never decorated fails at **build** time, not at decoration, because callee resolution lives in
`Unit::validate`. That is not an accident of the implementation — it is the same property that
makes results independent of decoration order.

### D11. The build is lazy and per-process, keyed on the fingerprint

Building at decoration would mean N builds for N functions. Building at interpreter exit would be
too late to call anything. First call is the only point where "all module-level decorators have
run" and "a result is needed" coincide.

If a function is decorated *after* a build has happened, the unit fingerprint no longer matches
the built module; the next call through an unbuilt function triggers one rebuild covering
everything. Correct, and the cost is bounded by how often that pattern actually occurs.

### D12. Decorator returns a wrapper object, not a replaced function

`@c.compyle` returns a callable carrying `__name__`, `__doc__`, `__module__`,
`__annotations__`, `__qualname__`, and `__wrapped__`. On call: ensure the build, resolve the
compiled callable once, cache it, dispatch. Keeping the original reachable through `__wrapped__`
is what makes "compiled and interpreted agree" testable at all.

### D13. The generated module's name carries the fingerprint

Module `compylr_generated_<fingerprint>`, inside a wheel whose *distribution* name is the stable
`compylr-generated`.

CPython cannot reliably re-import an extension module under a name already in `sys.modules` —
the `.so` is already mapped, and reload semantics for extension modules are not supported. A
stable module name would therefore make in-process rebuilds (D11) impossible: the second build
would succeed and then be unloadable. Putting the fingerprint in the module name makes each
build a distinct module, so it imports cleanly alongside its predecessor.

The distribution name stays stable so that installing a rebuild *uninstalls* the previous one
rather than accumulating dead modules in the venv. A module already loaded in the current
process keeps working from memory after its file is removed, which is what makes the swap safe
mid-process.

This is only viable because the name is not user-facing — the `python-bindings` spec requires
callers to reach compiled functions through the objects they marked, never by importing the
module.

### D14. Package layout

Root `pyproject.toml` with the maturin backend, `python-source = "python"`,
`module-name = "compylr._core"`, and the package at `python/compylr/`. `python/fixtures/` is not
a package and is not picked up, so the existing fixture paths and `tests/fixtures.rs` are
untouched. `[lib] crate-type = ["cdylib", "rlib"]` — `cdylib` for the extension, `rlib` so the
existing binary and the integration tests still link.

### D15. Exception taxonomy

```
CompylrError
├── CompilationError    (line, column, message)
│   ├── SourceSyntaxError
│   └── UnsupportedProgramError
├── BackendError        (unknown or reserved-but-unimplemented)
├── BuildError          (carries the toolchain's own stdout/stderr)
├── ToolchainMissingError
└── ConfigurationError  (assist mode enabled; conflicting re-initialization)
```

One catchable base, because the spec requires a caller to be able to handle "any compylr
compilation failure" without enumerating subclasses. `SourceSyntaxError` does not subclass
Python's built-in `SyntaxError`: it describes a *string being compiled*, not the module being
executed, and inheriting would let it be swallowed by handlers that meant the built-in.

## Risks / Trade-offs

* **PyO3 must support CPython 3.14** → **Resolved during implementation: `pyo3 = "0.29.2"` with
  `abi3-py311`**, verified building against CPython 3.14.5. One wheel spans 3.11+ instead of
  needing one per interpreter version. `extension-module` is kept behind an optional cargo
  feature: it tells PyO3 not to link libpython, which a wheel needs and `cargo test` cannot
  tolerate, since the test binary must resolve those symbols.
* **First call blocks for seconds** → Unavoidable for a compile-on-demand tool. Mitigated by the
  fingerprint cache making it strictly once per meaningful change. **Measured on this machine
  (M-series, CPython 3.14.5): first build 8.89 s, cached run 0.003 s — roughly 3400×.** The cost
  is real but paid once per meaningful change, which is the property that matters.
* **The user needs cargo and maturin** → Diagnosed explicitly before any build is attempted, with
  install instructions. Called out in the README rather than discovered at first run.
* **Emitted helpers can drift from the spec** → They are generated from one `const` and tested
  through executed code, not by string comparison, so a drift shows up as a wrong answer in a
  test rather than as a passing snapshot.
* **`i64` is not Python's integer** → Overflow raises `OverflowError` instead of silently
  wrapping, which is the honest failure. Arbitrary precision is out of scope and stays a
  documented limitation, consistent with the existing `LiteralOutOfRange` rejection.
* **Installing into the running interpreter's environment** → `--force-reinstall` plus
  `importlib.invalidate_caches()`, with the fingerprinted module name from D13 making the newly
  installed module importable rather than shadowed by the one already loaded.
* **`.compylr/` in the working directory** → Depends on where the process starts, so a project run
  from two different directories builds twice. Acceptable for an MVP, and visible enough to be
  understood when it happens; a project-root discovery rule can come later.

## Migration Plan

Nothing to migrate — no released package and no users. The one ordering constraint is internal:
`add-local-type-inference` should be archived before implementation starts, so the spec base
that the Rust backend is written against actually describes `float`, `/`, and `ToFloat`.

## Open Questions

* ~~Which exact PyO3 version and `abi3` floor to pin.~~ **Resolved:** `0.29.2` with `abi3-py311`,
  verified against CPython 3.14.5. See Risks.
* Whether `.compylr/` should be discovered by walking up to a project root rather than rooted at
  the working directory. Deferrable — it changes where one path is computed, and every
  requirement is stated in terms of "one directory", not a specific location.
