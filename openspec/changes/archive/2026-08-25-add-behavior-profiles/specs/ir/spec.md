## ADDED Requirements

### Requirement: Fallible operations declare whether the program defines their failure

Every IR operation that can fail on some inputs SHALL carry a **checking mode** stating whether the
program defines what happens when it does. The two modes SHALL be *reported*, where the failure
becomes a value the program can observe and handle, and *unchecked*, where the program declines to
define the result and whatever the target does is what happens.

The operations that SHALL carry it are: integer addition, subtraction, multiplication, and negation
(for a result outside the integer range); division and remainder (for a zero divisor); and
subscripting (for an index out of range or a key that is absent).

*Unchecked* is a statement about the program, not about the target. It says the program does not
define the result, which is why it is legible without knowing which backend will consume the unit —
one target may trap, another may wrap, and a third may do something else, and the unit is equally
true of all three.

#### Scenario: The checking mode is readable from the node

- **WHEN** an addition, division, remainder, negation, or subscript node is inspected
- **THEN** its checking mode is readable from the node itself

#### Scenario: The same operator can mean either mode

- **WHEN** two addition nodes declare different checking modes
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: The mode composes with an existing mode

- **WHEN** an integer division node is inspected
- **THEN** its rounding mode and its checking mode are both readable, and the two are independent

#### Scenario: A checking mode survives the artifact

- **WHEN** a unit containing both modes is serialized and read back
- **THEN** every declared checking mode is unchanged

#### Scenario: A checking mode reaches the fingerprint

- **WHEN** two units differ only in a declared checking mode
- **THEN** their fingerprints differ, because the mode is part of what the program computes

#### Scenario: An unchecked operation is not folded into a reported failure

- **WHEN** a pass folds a constant expression whose operation is declared unchecked and whose
  result would overflow or divide by zero
- **THEN** the pass leaves the expression alone rather than turning it into a reported failure,
  because the program did not ask for one

## MODIFIED Requirements

### Requirement: Operators carry declared semantics

Every arithmetic operator in the IR that admits more than one reasonable interpretation SHALL carry
its interpretation explicitly on the node, rather than relying on a convention inherited from one
source language. Specifically: integer division SHALL carry a rounding mode, remainder SHALL carry a
sign convention, division that promotes its operands SHALL say so, and every operator that can fail
SHALL carry a checking mode. A frontend SHALL set these to whatever the resolved behavior says; a
backend SHALL reproduce exactly what the node declares, without knowing which frontend produced it.

The two rounding modes SHALL be *toward negative infinity* and *toward zero*. The two remainder sign
conventions SHALL be *sign of the divisor* and *sign of the dividend*. These pairs cover the
behavior of the languages in compylr's supported list; a source language needing a third SHALL add
it to the IR rather than encode it in its frontend.

#### Scenario: Rounding mode is explicit

- **WHEN** an integer division node is inspected
- **THEN** its rounding mode is readable from the node itself

#### Scenario: The same operator can mean either rounding

- **WHEN** two integer division nodes declare different rounding modes
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: Remainder sign convention is explicit

- **WHEN** a remainder node is inspected
- **THEN** its sign convention is readable from the node itself

#### Scenario: Promotion is explicit

- **WHEN** a division node that yields a floating-point result from integer operands is inspected
- **THEN** the promotion is declared on the node rather than implied by the operator's name

#### Scenario: No node's meaning depends on the source language

- **WHEN** a unit is interpreted without knowing which frontend produced it
- **THEN** every operator's meaning is fully determined by the unit

#### Scenario: Failure handling is explicit

- **WHEN** an operator that can fail is inspected
- **THEN** whether the program defines its failure is readable from the node, independently of the
  operator's other declared modes

### Requirement: A unit records the frontend that produced it

A unit SHALL record the name of the frontend that lowered it, and the semantic guarantees **the
program** requires be preserved. This is what allows a pair-directed pass to be selected and a
backend's post-processing to be gated without any component re-deriving the source language from the
shape of the tree.

The recorded guarantees SHALL be derived from what the unit's own operations declare, not from a
fixed list belonging to the frontend. Two units produced by the same frontend MAY therefore record
different guarantees, because the guarantees describe what this program needs preserved rather than
what its language usually needs.

#### Scenario: The producing frontend is recorded

- **WHEN** a unit is produced by lowering source with a named frontend
- **THEN** the unit reports that frontend's name

#### Scenario: Required guarantees travel with the unit

- **WHEN** a unit is inspected
- **THEN** the guarantees the program requires preserved are readable from it

#### Scenario: The record survives the artifact

- **WHEN** a unit is serialized and read back
- **THEN** the producing frontend and its required guarantees are unchanged

#### Scenario: Guarantees follow the program, not the language

- **WHEN** two units from the same frontend declare different checking modes on their arithmetic
- **THEN** the one whose arithmetic is unchecked does not record that integer overflow must be
  reported, and the other does

### Requirement: Container operations carry declared semantics

Reading an element of a sequence and measuring the length of a value each admit more than one
reasonable interpretation across the languages compylr supports, so each SHALL carry its
interpretation on the node rather than inherit one from whichever frontend happens to exist.

Specifically: a subscript SHALL carry an **index origin** and a **checking mode**, and a length
SHALL carry the **text units** it counts in. A frontend sets these to whatever the resolved behavior
says; a backend reproduces exactly what the node says.

The index origins SHALL be *from either end*, where a negative index counts backwards from the end,
and *from the start*, where a negative index is out of range. The text units SHALL be *code points*,
*UTF-8 bytes*, and *UTF-16 units*. These cover Python, Go, C++, and TypeScript; a language needing
another SHALL add it to the IR rather than encode it in its frontend.

Each mode describes one operand kind and SHALL be inert for the others: an index origin says nothing
about a mapping, whose index is a key rather than an offset, and text units say nothing about a
sequence, whose length is a count of elements. A subscript's checking mode is the exception: it
applies to every operand kind, because a sequence offset out of range and a mapping key that is
absent are the same question — whether the failure is a value the program handles.

#### Scenario: Index origin is explicit

- **WHEN** a subscript node is inspected
- **THEN** its index origin is readable from the node itself

#### Scenario: The same subscript can mean either origin

- **WHEN** two subscript nodes declare different index origins
- **THEN** they are distinguishable, and a backend renders each differently

#### Scenario: Text units are explicit

- **WHEN** a length node is inspected
- **THEN** the units it counts in are readable from the node itself

#### Scenario: All three unit readings are distinguishable

- **WHEN** three length nodes declare code points, UTF-8 bytes, and UTF-16 units
- **THEN** each is distinct from the others

#### Scenario: A declared container mode survives the artifact

- **WHEN** a unit containing subscripts and lengths is serialized and read back
- **THEN** every declared mode is unchanged

#### Scenario: A declared container mode reaches the fingerprint

- **WHEN** two units differ only in a declared index origin, only in declared text units, or only
  in a subscript's checking mode
- **THEN** their fingerprints differ, because the mode is part of what the program computes

#### Scenario: A subscript's checking mode applies to mappings too

- **WHEN** a mapping subscript declares that its failure is unchecked
- **THEN** the node says so, and a backend renders it differently from one that reports

### Requirement: Container behavior that is not a mode is not parameterized

Where languages differ in the **shape** of an operation rather than in a setting on it, the IR SHALL
model the difference as a distinct form and SHALL NOT add a mode. In particular, reading a mapping
with a key that is absent SHALL always be an operation that *fails*: a language whose lookup instead
yields a default value alongside a presence flag is performing a different operation, one that
requires a notion of a type's zero value the IR does not model, and its frontend SHALL lower it to a
different form rather than set a flag.

Whether that failure is reported to the program or left undefined is a separate question, answered
by the subscript's checking mode. The two are not in tension: the mode says how a failure surfaces,
and this requirement says that a missing key is a failure at all.

#### Scenario: A missing mapping key is reported

- **WHEN** a mapping is read with a key it does not contain, from a node declaring the failure
  reported
- **THEN** the operation reports the missing key, whichever frontend produced the unit

#### Scenario: A missing key never yields a default value

- **WHEN** a mapping is read with a key it does not contain, under either checking mode
- **THEN** the operation fails, and never yields the value type's zero in place of one

#### Scenario: No mode exists for behavior compylr's languages agree on

- **WHEN** the IR's node definitions are inspected
- **THEN** no mode is carried for iterating a mapping, testing membership, or assigning a mapping
  key, because the languages in the supported list agree on all three

#### Scenario: No mode exists for a range with a zero step

- **WHEN** the IR's node definitions are inspected
- **THEN** a range carries no mode for a zero step, because every supported language refuses one
  and the refusal exists so that a non-terminating loop stays diagnosable

### Requirement: A unit serializes to a durable artifact

The IR SHALL be serializable to a durable, self-describing artifact and SHALL be reconstructible
from it. This belongs to the IR rather than to any one backend: the IR is the stage every
backend consumes, so an on-disk form of it is what makes the pipeline inspectable between
lowering and code generation regardless of which target is being emitted.

The artifact SHALL carry a format version, and a reader SHALL refuse an artifact whose version it
does not understand, naming both the version found and the version expected. Adding a mode to a node
changes the serialized shape, so the version SHALL advance whenever it does.

#### Scenario: A unit is written and read back

- **WHEN** a unit is serialized and then deserialized
- **THEN** the result compares structurally equal to the original

#### Scenario: The artifact describes every construct

- **WHEN** a unit containing every supported type, statement form, and expression form is
  serialized
- **THEN** each construct is represented in the artifact and survives a round trip

#### Scenario: Fingerprint survives a round trip

- **WHEN** a unit is serialized, deserialized, and its fingerprint recomputed
- **THEN** the fingerprint equals that of the original unit

#### Scenario: Float literals survive exactly

- **WHEN** a unit containing float literals, including negative zero, is round-tripped
- **THEN** each literal is bit-for-bit identical to the original, consistent with the IR's rule
  that float literals compare by bit pattern

#### Scenario: The artifact carries no target-language information

- **WHEN** an artifact is inspected
- **THEN** it names IR types and operators only, containing no Rust or other target spellings

#### Scenario: An artifact written before checking modes is refused

- **WHEN** an artifact written under the previous format version is read
- **THEN** it is refused with a message naming the version found and the version expected, rather
  than being read as though every operation reported its failures
