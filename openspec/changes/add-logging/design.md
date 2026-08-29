## Context

See proposal.md — Why. This change deliberately tests whether the previous two were a foundation:
if adding a second effectful module needs an IR form, they were not.

Constraints:

* A module is a namespace, not a value (`add-intrinsic-calls`, decision 4). A logger object would
  be a value.
* [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs) forbids the
  backend from naming Python, so anything about the host's logging configuration is a bridge
  concern.
* Mapping and set iteration order is unspecified, so the same rendering restriction output has
  applies here.
* The demo is where cost shows up — a construct users are told to leave in production code must be
  free when disabled.

## Goals / Non-Goals

**Goals:**
- Logging that reaches the host's existing handlers, formatters, and levels.
- A disabled record that costs a level test and nothing else.
- No IR change and no artifact version bump.

**Non-Goals:**
- Logger objects, `getLogger`, and per-logger configuration from compiled code.
- `%`-style formatting and multi-argument records. Refused explicitly, named as deferred.
- Structured or key-value logging fields.
- Configuring logging from compiled code. That belongs to the host.
- Exception logging and tracebacks. The subset has no exceptions.

## Decisions

### 1. No IR change, and that is the deliverable

**Decision.** Registry entries only. There is no snippet of an IR delta here, because there is no
IR delta — `logging.info(x)` produces the same statement form `print(x)` does:

```rust
// the form add-effect-statements added, reused unchanged
Perform { module: "logging", operation: "info", args: vec![x], convention: Convention::Python }
```

**Why.** This is the test of whether the first two changes were a foundation. Because the IR is
untouched, [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) does not move, no
fingerprint changes, no cache is invalidated, and
[`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs) is not tripped —
none of the five IR questions apply, and that is the point rather than an omission.

**Alternatives considered.** *A dedicated logging statement form carrying a level.* It would make
the level a first-class IR concept for one module's benefit, and the level is already expressible
as the operation's identity. The registry knows `debug` from `info`; the IR does not need to.

### 2. Module-level functions only; no logger values

**Decision.** `getLogger` is refused with a diagnostic naming the supported spelling:

```python
log = logging.getLogger(__name__)   # error: a logger is a value; use logging.info(...) directly
logging.info(count)                 # the supported form
```

**Why.** `logging.getLogger(__name__)` is the idiomatic Python spelling, and it returns a **value**.
Supporting it means a logger type in [`Ty`](../../../crates/compylr-ir/src/ir.rs#L103) that every
backend renders and every bridge converts, for a capability the module-level functions already
cover.

**Alternatives considered.** *Accept `getLogger` and return an opaque handle.* An opaque handle is
still a value with a type, and it would be the first type in the model with no rendering, no
equality, and no meaning to any backend. The nominal-type carve-out `Ty::Instance` already documents
how much a single exception costs.

### 3. Exactly one argument, and multi-argument records are refused

**Decision.** A second positional argument is a diagnostic naming the deferred feature:

```python
logging.info(count)                  # accepted
logging.info("count: %s", count)     # error: %-style formatting is not supported yet
```

**Why.** Python's `logging.info("count: %s", n)` defers `%`-formatting until the record is emitted.
Two things follow. First, joining extra arguments with spaces — what `print` does — would produce
*different text* than the interpreted program. Second, in Python `logging.info("done", n)` with no
placeholder is a latent error surfaced at emit time. Either way, accepting extra arguments means
implementing `%`-formatting or producing divergent text, and a divergence inside a *log* is
invisible precisely when someone is reading logs to diagnose something else.

**Alternatives considered.** *Join with spaces like `print`.* It is the one option that silently
produces different text in the two modes, on the construct people read when something else is
already broken.

### 4. The level test wraps the rendering, in the emitted source

**Decision.** The guard is emitted *around* the rendering, not inside the logging call:

```rust
// emitted — the argument is never rendered when the level is off
if log::log_enabled!(log::Level::Debug) {
    log::debug!("{}", render_int(middle, Convention::Python));
}
```

**Why.** The point of levels is that a disabled record is free. If the argument is rendered and
then discarded, `logging.debug(x)` in a loop is too expensive to leave in, and a logging construct
that must be removed before shipping is not one. The spec states this as an observable property —
no allocation attributable to a disabled record — so it is testable. This is the kind of cost
[`CLAUDE.md`](../../../CLAUDE.md) records the demo finding three times already, each invisible to
every correctness test.

**Alternatives considered.** *Let the facade's macro do the gating.* It does gate, but only after
the argument expression is evaluated, which is exactly the allocation this decision exists to avoid.

### 5. The facade in the backend, the implementation in the bridge

**Decision.** The generated crate depends on the facade alone; the bridge installs the implementation:

```rust
// backend emits this — it names no host
log::info!("{}", rendered);
// bridge installs this — it names Python, and only the bridge may
log::set_boxed_logger(Box::new(HostForwardingLogger::new()))
```

**Why.** The same split as the output sink, and for the same reason: the backend may not name the
host, and the user's handlers and formatters are the thing that should actually write. It also means
a generated crate built outside a host is not broken — a facade with no implementation installed
discards records, which is the defined behavior rather than a failure.

**Alternatives considered.** *Emit writes to stderr.* Then host configuration is bypassed entirely,
records from compiled and interpreted code land in different places with different formats, and
turning logging off requires a rebuild.

### 6. A record is attributed to the root logger, matching the interpreted program

**Decision.** Records forward to the host's **root** logger, because that is where the module-level
functions send them in CPython:

```text
INFO:root:3      # CPython, running the worked example
INFO:root:3      # compiled, required to match
```

**Why.** An earlier draft of this change said records should "carry an origin derived from the
source module", reasoning that this is what `getLogger(__name__)` is for. Running the worked example
showed that is wrong: module-level `logging.info` records against the **root** logger, so attributing
a compiled record to its source module would make the same source produce `INFO:root:` interpreted
and `INFO:searching:` compiled. Host configuration keyed by logger name would then apply to one and
not the other — the precise failure this change exists to prevent, introduced by the feature meant
to help.

The structural comparison in decision 7 would not have caught it, because it compares level,
message, and order and not the logger name. The fixture therefore compares the name too.

**Alternatives considered.** *Attribute records to the source module.* Strictly more useful, and it
is what a user writing `getLogger(__name__)` would get — but the source here does not say that, and
inventing an attribution the source did not ask for is a divergence. If per-module attribution is
wanted it belongs with `getLogger` support, where the user asks for it explicitly.

### 7. Levels map explicitly and totally

**Decision.** Five source levels, both directions, no default arm:

```rust
match level {
    Level::Debug => /* ... */, Level::Info => /* ... */, Level::Warning => /* ... */,
    Level::Error => /* ... */, Level::Critical => /* ... */,
    // no `_ =>` arm: a level added later fails to compile
}
```

**Why.** A level added later fails to compile rather than silently mapping to something adjacent —
the same reasoning [`stance`](../../../crates/compylr-ir/src/behavior.rs#L199) uses for exhaustive
matching.

**Alternatives considered.** *A default arm mapping unknown levels to `info`.* It turns a
compile-time error into a silently wrong severity in production logs.

### 8. Comparison is structural, not textual

**Decision.** The differential harness captures records as level, name, message, and order — not as
formatted lines. A decision about test methodology rather than about a type, so it carries no
snippet.

**Why.** A formatted log line carries a timestamp, so comparing lines makes the suite fail based on
when it ran. Capturing structurally compares what the program actually determines. This is the same
discipline as never asserting on mapping iteration order: compare what the language promises.

## Risks / Trade-offs

**One argument is a real ergonomic limit** → Acknowledged rather than mitigated. Interpolating a
computed value needs either `%`-formatting or a text conversion, and both are deferred. The
diagnostic names the deferred feature so a user hitting it learns what is coming rather than that
logging is broken.

**Logger attribution was nearly got wrong** → Recorded as decision 6 rather than left as a risk,
because it was found by running the example rather than by reasoning about it. The lesson
generalizes: the structural comparison must include the logger name, or a whole class of
attribution divergence passes the suite.

**Forwarding a record acquires the host's runtime lock** → Per record, released immediately, never
held across a call into user code. A record at a disabled level never reaches the bridge at all,
because the level test is on the compiled side — which is why the guard placement is specified
rather than left to the implementation.

**The host's level is read per record** → Cached on the compiled side and invalidated when the host
changes it, so the common case is a comparison. If the cache proves wrong under host
reconfiguration, correctness wins and the read stays; the spec requires that a host level change
take effect without a rebuild, not that it be free.

**A logging implementation installed by another library could conflict** → Installation is
idempotent and installs once per process. If the host application has already installed its own,
the bridge does not displace it.

**Recursion through the host's logging** → A handler that itself calls a compiled function that
records could recurse. Forwarding contains failures and does not re-enter, so the failure mode is a
dropped record rather than a stack overflow.

## Migration Plan

None required. No IR form, no artifact version change, no cache invalidation: existing projects are
untouched and nothing rebuilds. The generated crate gains one facade dependency, which appears on
the next build for a project that uses logging and not otherwise.

Rollback is removing the change.

## Open Questions

None. The attribution question that was open in the earlier draft is resolved as decision 6:
running the worked example settled it, and leaving it open would have changed both the spec and the
fixture's comparison.
