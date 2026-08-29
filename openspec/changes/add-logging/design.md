## Context

See proposal.md — Why. This change deliberately tests whether the previous two were a foundation:
if adding a second effectful module needs an IR form, they were not.

Constraints:

* A module is a namespace, not a value (`add-intrinsic-calls`). A logger object would be a value.
* The backend cannot name Python (`crate_boundaries.rs`), so anything about the host's logging
  configuration is a bridge concern.
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

### Module-level functions only; no logger values

`logging.getLogger(__name__)` is the idiomatic Python spelling, and it returns a **value**.
Supporting it means a logger type in `Ty` that every backend renders and every bridge converts, for
a capability the module-level functions already cover.

What `getLogger(__name__)` is actually *for* — records attributable to the module they came from,
so configuration keyed by name applies — is supplied directly: records carry an origin derived from
the source module. The user gets the benefit without the type.

*Alternative considered: accept `getLogger` and return an opaque handle.* An opaque handle is still
a value with a type, and it would be the first type in the model with no rendering, no equality,
and no meaning to any backend. The nominal-type carve-out `Ty::Instance` already documents how
much a single exception costs.

### Exactly one argument, and multi-argument records are refused

Python's `logging.info("count: %s", n)` defers `%`-formatting until the record is emitted. Two
things follow. First, joining extra arguments with spaces — what `print` does — would produce
*different text* than the interpreted program. Second, in Python `logging.info("done", n)` with no
placeholder is a latent error surfaced at emit time.

Either way, accepting extra arguments means implementing `%`-formatting or producing divergent
text. A format mini-language — widths, precision, `%r`, `%%` — is its own change, and a divergence
inside a *log* is invisible precisely when someone is reading logs to diagnose something else.

Refusing with a diagnostic that names the deferred feature is the honest position. The cost is a
real ergonomic gap, recorded in the proposal rather than hidden.

### The level test wraps the rendering, in the emitted source

The point of levels is that a disabled record is free. If the argument is rendered and then
discarded, `logging.debug(x)` in a loop is too expensive to leave in, and a logging construct that
must be removed before shipping is not one.

So the guard is emitted *around* the rendering, and the spec states it as an observable property:
no allocation attributable to a disabled record. This is testable, and it is the kind of cost
CLAUDE.md records the demo finding three times already — a quadratic clone in `for`, an O(n) clone
per nested read, a full recompile per marked member — each invisible to every correctness test.

### The facade in the backend, the implementation in the bridge

The backend emits calls to the target's standard logging *facade* and the generated crate depends
on the facade alone. The bridge installs an implementation forwarding into the host's logging.

This is the same split as the output sink, and for the same reason: the backend may not name the
host, and the user's handlers and formatters are the thing that should actually write. It also
means a generated crate built outside a host is not broken — a facade with no implementation
installed discards records, which is the defined behavior rather than a failure.

*Alternative considered: emit writes to stderr.* Then host configuration is bypassed entirely,
records from compiled and interpreted code land in different places with different formats, and
turning logging off requires a rebuild.

### Levels map explicitly and totally

Five source levels map onto the facade's levels, both directions, with no default arm. A level
added later fails to compile rather than silently mapping to something adjacent — the same
reasoning `LanguageBehavior::stance` uses for exhaustive matching.

### Comparison is structural, not textual

A formatted log line carries a timestamp, so comparing lines makes the suite fail based on when it
ran. The differential harness captures records structurally — level, message, order — which is what
the program actually determines. This is the same discipline as never asserting on mapping
iteration order: compare what the language promises.

## Risks / Trade-offs

**One argument is a real ergonomic limit** → Acknowledged rather than mitigated. Interpolating a
computed value needs either `%`-formatting or a text conversion, and both are deferred. The
diagnostic names the deferred feature so a user hitting it learns what is coming rather than that
logging is broken.

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

- Whether the origin should be the source module or the compiled function. Module matches what
  `getLogger(__name__)` gives and is the default; per-function origin is strictly finer and could be
  added later without changing the spec, since the spec requires only that an origin derived from
  the source module be present.
