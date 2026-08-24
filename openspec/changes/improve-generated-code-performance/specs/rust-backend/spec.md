## ADDED Requirements

### Requirement: An accumulator that reads itself updates in place

Where a statement assigns to a name from an expression that reads that same name as the left
operand of an addition — the shape `x = x + y` — the backend SHALL emit an in-place update rather
than building a new value and rebinding it.

This is not a micro-optimization for text. Building a fresh value per iteration makes accumulation
quadratic, and CPython resizes in place when the target holds the only reference, so the current
emission is asymptotically *worse* than the interpreter it replaces. Measured on `text.joined`:
343.76us to 83.08us, a 4.1x difference that moves the workload from losing to the interpreter to
beating it.

The emission SHALL stay type-directed. The backend does not know an expression's type and must not
learn it here; the in-place form is selected through a trait whose implementations differ per type,
exactly as the existing addition is.

#### Scenario: String accumulation appends in place

- **WHEN** a `str` local is assigned from itself plus another value
- **THEN** the emitted code appends to the existing value rather than allocating a new one

#### Scenario: Numeric accumulation keeps its checking

- **WHEN** an `int` local is assigned from itself plus another value
- **THEN** the emitted code performs the same checked addition it does today, and still reports
  overflow

#### Scenario: The name must be the left operand

- **WHEN** the assigned name appears somewhere other than as the left operand of the addition
- **THEN** the ordinary emission is used, because the in-place form would read a value that has
  already been modified

### Requirement: A loop variable that is only read is borrowed

Where a `for` iterates a collection and the loop body never assigns to, moves, or mutates the loop
variable, the backend SHALL bind it by reference rather than cloning each element.

For a collection of scalars this costs nothing either way; for a collection of owned values it is
an allocation and a copy per element per loop. Measured on `text.total_length`, whose body is a
single length read per element: 88.52us to 59.43us.

Whether the body assigns to the loop variable is already computed, because it decides whether the
binding is emitted as mutable. The same answer decides this.

#### Scenario: A read-only loop variable is not cloned

- **WHEN** a loop body only reads its loop variable
- **THEN** the emitted loop binds it by reference

#### Scenario: A written loop variable is still owned

- **WHEN** a loop body assigns to its loop variable
- **THEN** the emitted loop binds an owned value, so the assignment is legal and does not affect
  what is iterated

#### Scenario: The runtime accepts a borrowed value wherever an owned one works

- **WHEN** a borrowed loop variable is passed to a runtime helper
- **THEN** the helper accepts it, so borrowing a loop variable never turns a working program into
  one that does not compile

### Requirement: A local returned in tail position is moved

Where a function's final statement returns a bare local name, the backend SHALL move that value
rather than cloning it. The function is ending and the original is about to be dropped, so the copy
has no reader.

The restriction to tail position is deliberate and load-bearing: a `return` nested inside a loop
that iterates the same name would move out of a value the loop borrows. Tail position is the last
statement at the top level of the body and therefore cannot sit inside any loop, which makes the
move safe by construction rather than by analysis.

#### Scenario: A returned collection is not copied

- **WHEN** a function's last statement returns a local holding a collection
- **THEN** the emitted code moves it into the result

#### Scenario: A return inside a loop is unchanged

- **WHEN** a `return` of a local appears anywhere other than tail position
- **THEN** the existing emission is used

#### Scenario: Returning a field still copies

- **WHEN** a function returns an attribute rather than a local
- **THEN** it is copied, because the instance outlives the call and must not be emptied

### Requirement: Generated maps and sets are parameterised over their hasher

The runtime's implementations for mapping and set types SHALL be generic over the hasher rather
than written against the standard library's default, and generated code SHALL select the hasher it
uses rather than inheriting one.

Today the hasher is not a choice at all: the implementations are written against the two-parameter
form of the container types, which silently pins the default hasher across every one of them. That
is a defect independent of which hasher is preferred — it means the decision cannot be expressed.

The selected default SHALL be a non-cryptographic hasher. Keys in generated code come from the
user's own program rather than from an untrusted source, and the interpreter being compared
against hashes small integers to themselves and caches a string's hash in the string. Measured:
`graphs.bfs_distances` 159.36us to 82.49us, which moves it from 0.7x to 1.4x against interpreted;
`graphs.topological_order` 421.48us to 271.33us.

A hasher has no observable semantics, so this is a performance choice and not a behavior axis. It
SHALL NOT be exposed as one.

#### Scenario: The runtime accepts any hasher

- **WHEN** the runtime's mapping and set implementations are compiled
- **THEN** they are generic over the hasher, and a container using a non-default hasher satisfies
  every one of them

#### Scenario: Container literals build with the selected hasher

- **WHEN** a mapping or set literal is emitted
- **THEN** it constructs a container using the selected hasher rather than a form available only
  for the default one

#### Scenario: Iteration order remains unguaranteed

- **WHEN** a mapping or set is iterated in generated code
- **THEN** no order is guaranteed, exactly as before, and no test may depend on one

### Requirement: The runtime does not repeat work it has already done

Runtime helpers SHALL NOT perform work a caller or an earlier step has already performed.

Three instances are known and measured as a group at 2.7x on `text.word_count`'s body: resolving an
index validates the offset and then indexes through a checked operation that validates it again;
computing a text length under a code-point reading decodes the entire string on every call, where
the common case admits an exact shortcut; and the read-modify-write of a mapping entry performs
three separate lookups of the same key.

#### Scenario: An index is validated once

- **WHEN** a sequence element is read through the runtime
- **THEN** the offset is checked once, and an out-of-range index is still reported rather than
  panicking

#### Scenario: Text length keeps its declared units

- **WHEN** a text length is computed under any units the IR declares
- **THEN** the answer is exactly what it is today for every input, including non-ASCII text

#### Scenario: A mapping read-modify-write is not three lookups

- **WHEN** generated code reads a mapping entry, derives a new value, and stores it back under the
  same key
- **THEN** the emitted code does not hash that key three separate times

#### Scenario: A missing key still reports

- **WHEN** a mapping entry that is absent is read
- **THEN** it is reported exactly as it is today, and the fused form does not create it
