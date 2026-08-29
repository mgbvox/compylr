## Why

A compiled function is invisible. When it produces a wrong answer the user has no way to see inside
it: `print` (once `add-effect-statements` lands) writes unconditionally to a stream, which is a
debugging tool rather than an operational one. Long-running code needs output that carries a
**level**, that can be turned off without editing the source, and that lands in the same place the
rest of the application's logs land.

Today it lands nowhere, because `import logging` fails on line 1 like every other import — see
[`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs#L585).

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
  in [`Ty`](../../../crates/compylr-ir/src/ir.rs#L103) that every backend must render, for a
  capability the module-level functions already provide.

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

- **The host's logging configuration governs.**
  [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs) emits calls to the target's standard
  logging facade; the **bridge**
  ([`compylr-bridge-python-rust`](../../../crates/compylr-bridge-python-rust/src/lib.rs)) installs
  an implementation that forwards records into the host's logging system, with levels mapped both
  ways. The user's handlers and formatters are what actually write, so compiled and interpreted code
  produce records in one stream with one format.

- **Level mapping is explicit and total.** The source's five levels map onto the target facade's
  levels, and the host's effective level is what suppresses — so turning logging down in the host
  turns it down in compiled code, without a rebuild.

- **Records are attributed to the same logger the interpreted program uses.** In CPython the
  module-level `logging.info` records against the **root** logger, not against the calling module;
  a compiled record must therefore do the same, or the same source would produce `INFO:root:` in
  one mode and `INFO:<module>:` in the other. See design.md — decision 6, which is where this was
  nearly got wrong.

- **No artifact version change.** No IR form is added.

## Worked Example

Binary search is already inside the subset — only the visibility is missing — so the example is one
function with one record in the loop and one at the end. That reaches everything this change adds:
a suppressed level, an emitted level, and the origin question.

**Input** — `searching.py`:

```python
import logging


def bisect(values: list[int], target: int) -> int:
    low = 0
    high = len(values)
    while low < high:
        middle = (low + high) // 2
        logging.debug(middle)
        if values[middle] < target:
            low = middle + 1
        else:
            high = middle
    logging.info(low)
    return low
```

**Today** — the import stops it at line 1, and nothing about the algorithm is the problem. Both
transcripts below are real runs against the CLI at the tip of this branch:

```text
$ cargo run -p compylr-cli -- searching.py
error: 1:1: imports are not supported; only function definitions may appear at top level

$ cargo run -p compylr-cli -- searching.py    # with the two logging lines deleted
unit fingerprint: d6f4df7e125cde7e
  bisect (2 params) -> int
```

That second transcript is the whole argument for this change: the function compiles today, and the
only thing it cannot do is say what it is doing.

**After** — the record is emitted with its level test outside the rendering, so a suppressed
`debug` in the loop costs a comparison:

```rust
// expected — the mechanism does not exist yet
if log::log_enabled!(log::Level::Debug) {
    log::debug!("{}", render_int(middle, Convention::Python));
}
```

Rendering the argument and then discarding it is what makes a logging construct too expensive to
leave in shipped code, which is the same as not having it.

**At the boundary** — run under CPython at the default `INFO` level, so the loop's `debug` records
are suppressed and one `info` record survives:

```pycon
>>> import logging
>>> logging.basicConfig(level=logging.INFO, format="%(levelname)s:%(name)s:%(message)s")
>>> import searching
>>> searching.bisect([1, 3, 5, 7, 9], 7)
INFO:root:3
3
```

That transcript is CPython's actual output, run while writing this proposal — not expected output.
The `root` in it is the detail that matters: module-level `logging.info` records against the root
logger, so a compiled build attributing records to `searching` would produce a different line for
the same source.

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
- [`ir.rs`](../../../crates/compylr-ir/src/ir.rs) — registry entries only; no form, no version
  change.
- [`lower.rs`](../../../crates/compylr-frontend-python/src/lower.rs#L585) — the `getLogger` and
  arity refusals.
- [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs) — the level-gated emission.
- [`compylr-bridge-python-rust`](../../../crates/compylr-bridge-python-rust/src/lib.rs) — the
  forwarding logger and level mapping, plus the logging facade dependency in the generated manifest.
- [`frontends/python/fixtures/`](../../../frontends/python/fixtures/) — an accepted fixture with a
  driver, and rejected fixtures.
- [`README.md`](../../../README.md), [`CLAUDE.md`](../../../CLAUDE.md).

**Unaffected**
- The IR, the artifact format, and every existing cache. Nothing rebuilds because of this change.
- Every existing answer and every existing diagnostic.

**Costs**
- One dependency added to the generated crate: the target's logging facade, which is a facade and
  pulls in no implementation.
- A level check per record on the compiled side, which is the mechanism, not the overhead.
