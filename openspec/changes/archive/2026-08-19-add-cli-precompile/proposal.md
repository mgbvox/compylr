## Why

The first run of a compylr project pays for the build. Measured earlier in this project: **8.89
seconds cold, 0.003 seconds cached.** That is the right trade for a development loop, and the wrong
one everywhere else — a container starting, a CI job, a demo someone is watching, or a CLI tool a
user just installed. In each of those the first run *is* the run.

Nothing today can build a project ahead of time. The build is triggered by calling a decorated
function, so the only way to warm the cache is to run the program and wait.

## What Changes

- Add **`compylr compyle <root>`**, which finds every marked function and class under a project
  root, builds the shared artifact once, and exits. A later run finds the cache warm and starts
  immediately.
- **`compylr` becomes a Python console script**, installed by the wheel. The Rust binary cannot
  import Python modules, and discovery works by importing — see Impact. The Rust binary stays a
  development tool, reached through `cargo run`.
- Discovery works by **importing** each module under the root, so the decorators run and register
  exactly as they would at runtime. There is no second definition of what counts as marked.
- The command SHALL **report what it found and what it did**: modules imported, functions and
  classes marked, whether it built or the cache was already warm, and how long it took. A
  precompile that silently does nothing is indistinguishable from one that worked.
- Exit status distinguishes success, a build failure, and finding nothing to compile.

Explicitly **not** in this change: cross-compiling for another platform, building without a Rust
toolchain, distributing a prebuilt artifact, watching for changes, or parallel builds.

## Capabilities

### New Capabilities

None — this widens three existing capabilities.

### Modified Capabilities

- `cli`: gains a command that compiles a whole project, and the surface moves from a Rust binary to
  a Python console script for this purpose.
- `build-pipeline`: gains the ability to be driven without a decorated function having been called.
- `python-api`: gains a programmatic entry point the command is a thin wrapper over, so the two
  cannot drift.

## Impact

- **The Rust binary cannot do this.** Discovery imports Python modules; the `compylr` binary is a
  Rust executable with no interpreter. The options were to spawn a Python subprocess from Rust, to
  scan source statically, or to make the user-facing command a Python console script. The third is
  the only one where the command and the runtime agree by construction, because the same decorator
  registers in both.

  So `compylr` on a user's `PATH` becomes the Python entry point declared in `pyproject.toml`. The
  Rust binary keeps its `--emit` flags and stays useful for compiler development, but it stops being
  the thing users invoke. That is a real change in what the name refers to, and it needs saying in
  the README rather than being discovered.
- **Importing runs module-level code.** That is the cost of exact discovery: a module with a
  side effect at import time will perform it. Stated plainly in the command's own help, because a
  user who expects a compiler to be inert will otherwise be surprised.
- **Discovery must not import the world.** Only modules under the given root, skipping the usual
  non-source directories, and never following installed packages — otherwise precompiling a small
  project could import an arbitrary dependency tree.
- **A module that fails to import is a real outcome, not a crash.** The command reports which
  module and why, and continues to the others, because one broken module should not prevent
  precompiling the rest.
- **Ordering**: fourth of five, and independent of the first three — it compiles whatever the subset
  supports at the time. Placed here because the demo depends on it, and because there is no point
  precompiling a subset too small to write a program in.
