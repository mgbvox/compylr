## Context

See proposal.md — Why. The constraint that shapes everything here:

* The `compylr` binary is Rust. It has no Python interpreter, so it cannot import a project's
  modules, and importing is the only discovery that agrees with the runtime by construction.
* The manager is process-wide and already holds every marked source; `ensure_built` already builds
  the whole project from them. Precompiling is therefore mostly *discovery* plus a call to
  machinery that exists.
* `discover_root` already locates a project's `.compylr/` by walking upward for a marker, so
  building from a different working directory already lands in the right place.

## Goals / Non-Goals

**Goals:**

* One definition of "marked", shared by the command and the runtime.
* A report that makes a precompile that found nothing obviously different from one that worked.

**Non-Goals:**

* Building without a Rust toolchain, or for another platform. Both are distribution problems.
* Watching for changes, or building in parallel.

## Decisions

### D1. `compylr` becomes a Python console script

Declared in `pyproject.toml`, installed by the wheel. The Rust binary keeps its `--emit` surface and
is reached through `cargo run` during compiler development.

This changes what the name refers to for anyone who has been using the Rust binary directly, so the
README has to say it rather than leave it to be discovered. The alternative — the Rust binary
spawning `python -c` — puts a subprocess and an interpreter-discovery problem between the user and
the answer, to end up in the same place.

*Alternative considered:* static scanning. Rejected on the grounds the proposal states: it needs its
own notion of what `@c.compyle` looks like, and that notion drifts from the runtime's on aliases
(`from compylr import initialize as boot`), re-exports, and conditional decoration. A precompiler
that misses a function is worse than none, because the failure is a slow first run rather than an
error.

### D2. Discovery imports, and the cost is stated

Importing runs module-level code. That is inherent: a decorator only registers when it runs.

Two mitigations, neither of which pretends otherwise. The help text says it plainly. And discovery
is bounded — only modules beneath the given root, skipping `.venv`, `__pycache__`, `.git`,
`.compylr`, and build output, and never following installed packages. Precompiling a small project
should not import an arbitrary dependency tree.

A module that raises on import is reported and skipped rather than aborting the run: one broken
module should not prevent precompiling the rest, and the report names it so the omission is visible
rather than silent.

### D3. The command is a thin wrapper over a function

`compylr.precompile(root) -> Report`, and the command formats the report. Any decision made only in
the command is a place the two forms can disagree, and a user debugging a precompile should not have
to work out which they are looking at.

The report carries counts and outcomes rather than formatted text, so the command owns presentation
and the function owns facts.

### D4. Exit status distinguishes three outcomes

Success, build failure, and nothing-found. Nothing-found is not success: a script that precompiles
in a container image and silently compiles nothing has failed at the thing it was there for, and
the symptom would otherwise appear much later as a slow first request.

It is also not a hard error in the programmatic form, where a caller may legitimately precompile a
project that has nothing marked yet — hence a distinct status rather than an exception.

## Risks / Trade-offs

* **Importing executes user code** → Inherent to exact discovery. Stated in help and in the README,
  and bounded to the root. A user who cannot accept it wants static scanning, which cannot be exact;
  that trade is recorded here rather than hidden.
* **Two things are now called `compylr`** → The console script and the Rust binary. Mitigated by the
  binary keeping a distinct, developer-facing surface, and by the README saying which is which.
* **Precompiling in one environment and running in another** → The artifact is built for the
  interpreter that built it. A container that precompiles at build time and runs the same image is
  fine; one that precompiles on a different Python is not. Worth a note, and out of scope to fix,
  since it is the wheel-distribution problem again.
* **A partially-importable project builds partially** → By design, and reported. The risk is a user
  skimming the output and missing that a module was skipped, so the count of failures belongs in the
  summary line rather than only in the detail.

## Migration Plan

Nothing to migrate. Projects that never precompile behave exactly as before; precompiling writes the
same artifact a first call would have, keyed on the same fingerprint, so the two paths converge.
