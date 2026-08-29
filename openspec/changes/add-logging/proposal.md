## Why

A compiled function is invisible. When it produces a wrong answer the user has no way to see inside
it: `print` (once `add-effect-statements` lands) writes unconditionally to a stream, which is a
debugging tool rather than an operational one. Long-running code needs output that carries a
**level**, that can be turned off without editing the source, and that lands in the same place the
rest of the application's logs land.

Today it lands nowhere, because `import logging` fails on line 1 like every other import.

The interesting part is that after the first two changes, almost nothing is missing. `logging.info`
is an effectful operation from a named module — exactly the shape `add-effect-statements` built. So
this change adds **no IR form and no artifact version bump**, which is the point: if supporting a
second effectful module required reopening the IR, the foundation would not have been a foundation.

What it does add is the two things that make logging different from printing: a record is
**suppressed by level before its arguments are rendered**, and it must reach the *host's* logging
configuration rather than a stream, so a user's existing handlers, formatters, and level settings
govern it.

## What Changes

- **`logging` becomes a supported module**, with the module-level functions `debug`, `info`,
  `warning`, `error`, and `critical`. Each is an effectful operation, so it reuses the statement
  form and the renderers that `add-effect-statements` established.

- **`logging.getLogger(...)` is rejected**, with a diagnostic naming the supported module-level
  functions. A logger is a *value*, and a module is not one — accepting it would mean a logger type
  in `Ty` that every backend must render, for a capability the module-level functions already
  provide. The record's origin is supplied instead (below), which is what `getLogger(__name__)` is
  usually for.

- **A record takes exactly one argument**, of any renderable type. Python's logging treats
  additional positional arguments as `%`-format arguments, so accepting them and joining with
  spaces would produce different text from the interpreted program — and in a *log*, a divergence
  is invisible until someone is reading logs to diagnose something else. Multi-argument and
  `%`-style formatting are refused with a diagnostic naming them as deferred, rather than
  half-implemented.

- **A suppressed record costs nothing.** Emission tests the level *before* evaluating or rendering
  the argument, so `logging.debug(expensive)` in a hot loop is a level check when debug logging is
  off. Rendering first and discarding would make the construct too expensive to leave in code,
  which is the same as not having it.

- **Records carry an origin derived from the source module**, so host-side configuration keyed by
  logger name applies to compiled code the way it applies to interpreted code.

- **The host's logging configuration governs.** The backend emits calls to the target's standard
  logging facade; the **bridge** installs an implementation that forwards records into the host's
  logging system, with levels mapped both ways. The user's handlers and formatters are what
  actually write, so compiled and interpreted code produce records in one stream with one format.

- **Level mapping is explicit and total.** The source's five levels map onto the target facade's
  levels, and the host's effective level is what suppresses — so turning logging down in the host
  turns it down in compiled code, without a rebuild.

- **No artifact version change.** No IR form is added.

## Capabilities

### New Capabilities
- `program-logging`: what a compiled program may log, how a level suppresses a record before its
  cost is paid, and how records reach the host's logging configuration.

### Modified Capabilities
- `intrinsics`: an effectful operation may declare a level that gates it.
- `ir-lowering`: `logging` resolves to effectful operations; `getLogger` and multi-argument records
  are refused with reasons.
- `rust-backend`: records emit through the target's logging facade, level-tested before rendering.
- `native-bridge`: the bridge forwards records into the host's logging system and maps levels.
- `fixture-corpus`: logging is exercised by a fixture whose records are compared by level and
  message rather than by formatted line.

## Impact

**Modified**
- `crates/compylr-ir/src/ir.rs` — registry entries only; no form, no version change.
- `crates/compylr-frontend-python/src/lower.rs` — the `getLogger` and arity refusals.
- `crates/compylr-backend-rust/src/rust.rs` — the level-gated emission.
- `crates/compylr-bridge-python-rust/src/bindings.rs` — the forwarding logger and level mapping.
- `crates/compylr-bridge-python-rust/Cargo.toml` and the generated manifest — the logging facade
  dependency.
- `frontends/python/fixtures/` — an accepted fixture with a driver, and rejected fixtures.
- `README.md`, `CLAUDE.md`.

**Unaffected**
- The IR, the artifact format, and every existing cache. Nothing rebuilds because of this change.
- Every existing answer and every existing diagnostic.

**Costs**
- One dependency added to the generated crate: the target's logging facade, which is a facade and
  pulls in no implementation.
- A level check per record on the compiled side, which is the mechanism, not the overhead.
