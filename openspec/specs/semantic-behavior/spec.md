## Purpose

Defines how a user chooses, per operation, which of the two languages in a compilation supplies
the meaning: what an axis is, how each language declares its stance on one, how a request resolves
against the `(source, target)` pair, and what makes a request invalid.

## Requirements

### Requirement: A behavior axis is one operation two languages read differently

A **behavior axis** SHALL name one operation a programmer writes for which the source and target
languages can disagree about the result or about what happens when it fails. The set of axes SHALL
be fixed and enumerable, so that a user, a diagnostic, and a test can all refer to the same list.

The axes SHALL be: integer overflow (addition, subtraction, multiplication, and negation of
integers), integer division, exact division, remainder, sequence indexing, and text length.

An operation on which every supported language agrees SHALL NOT be an axis. In particular, the
*shape* of a missing-mapping-key lookup is not an axis — a language whose lookup yields a presence
flag alongside a value writes a different operation, not the same one configured differently — and
neither is a range with a zero step, which every supported language refuses and which exists to
keep a non-terminating loop diagnosable.

#### Scenario: The axes are enumerable

- **WHEN** the set of behavior axes is requested
- **THEN** exactly the six named axes are returned, each with a stable identifier

#### Scenario: An axis names an operation, not an implementation

- **WHEN** an axis is described to a user
- **THEN** it is described by the operation it governs and by what each language means by it

### Requirement: A language declares its own stance on every axis
Each source language and each target language SHALL declare, for every axis, what its own language
means by that operation. A language SHALL NOT declare anything about another language's meaning,
and no component SHALL hold a table mapping one language's stance onto another's. Both TypeScript and Go
SHALL declare their own complete stance on all six behavior axes.

#### Scenario: Both endpoints declare a stance
- **WHEN** a source language and a target language are resolved for a compilation
- **THEN** each answers, for every axis, what its own language means

#### Scenario: A language's declaration is complete
- **WHEN** a language declares its behavior
- **THEN** it covers every axis, and a language that omitted one could not be registered

#### Scenario: TypeScript declares its behavior profile
- **WHEN** TypeScript frontend behavior is inspected
- **THEN** it returns a complete stance for all 6 axes

#### Scenario: Go backend declares its behavior profile
- **WHEN** Go backend behavior is inspected
- **THEN** it returns a complete stance for all 6 axes

### Requirement: A behavior request resolves per axis to one of the two languages

A **behavior request** SHALL be either a single language name, meaning that language's stance on
every axis, or a per-axis selection naming a language for some axes and leaving the rest
unspecified. Resolving a request against a `(source, target)` pair SHALL produce a **resolved
behavior**: exactly one stance for every axis, taken from the language named for that axis.

An axis left unspecified SHALL take the stance of the enclosing default rather than of any fixed
language, so that naming one axis does not silently reset the others.

A single language name SHALL be exactly equivalent to a per-axis selection naming that language
for every axis.

#### Scenario: A bare language name sets every axis

- **WHEN** a request names the target language and nothing else
- **THEN** every axis resolves to the target language's stance

#### Scenario: An unspecified axis inherits

- **WHEN** a request names one axis and the enclosing default is the source language
- **THEN** that axis takes the named language's stance and every other axis takes the source
  language's

#### Scenario: The two spellings agree

- **WHEN** a bare language name and a per-axis selection naming that language for every axis are
  each resolved against the same pair
- **THEN** the two resolved behaviors are identical

#### Scenario: A resolved behavior is total

- **WHEN** a behavior is resolved
- **THEN** every axis has exactly one stance, and no axis is left to be decided later

### Requirement: Only the two languages in the compilation may be named

A behavior request SHALL name only the source language or the target language of the compilation
it applies to. Any other name SHALL be rejected before compilation begins, with a message that
names the two languages that would have been accepted.

The rejection SHALL distinguish a name compylr does not know from a name that is a registered or
reserved language but is not one of the two in this compilation, because the two are different
mistakes and the second is the one a user is likely to make.

#### Scenario: An unknown language is rejected

- **WHEN** a behavior names a language compylr has no frontend or backend for
- **THEN** the request is rejected, and the message says it is not a language compylr knows and
  names the two that would have been accepted

#### Scenario: A known but absent language is rejected distinctly

- **WHEN** a behavior in a Python-to-Rust compilation names a language compylr has registered or
  reserved but which is neither Python nor Rust
- **THEN** the request is rejected with a message distinguishing it from an unknown name, and
  naming the two languages of this compilation

#### Scenario: An unknown axis is rejected

- **WHEN** a per-axis selection names an axis that does not exist
- **THEN** the request is rejected and the message lists the axes that do

#### Scenario: Rejection precedes compilation

- **WHEN** a behavior request is invalid
- **THEN** it is rejected before any source is lowered and before any target source exists

### Requirement: The source language is the default

When no behavior is requested, every axis SHALL resolve to the **source** language's stance. A
compilation with no behavior request SHALL therefore produce exactly what it produced before
behavior existed.

#### Scenario: No request means the source language

- **WHEN** a compilation is run with no behavior request
- **THEN** every axis resolves to the source language's stance

#### Scenario: The default is not the target's

- **WHEN** the source and target disagree on an axis and no behavior is requested
- **THEN** the source language's meaning is what the generated code produces

### Requirement: A resolved behavior determines what the program requires preserved

A resolved behavior SHALL determine which semantic guarantees the compiled program requires. An
axis resolved to a stance under which an operation's failure is not defined by the program SHALL
NOT contribute the guarantee that the failure be reported.

The guarantees a program requires SHALL therefore be a property of the program rather than of its
source language, so that a target transformation forbidden for one program may be permitted for
another written in the same language.

#### Scenario: Requirements shrink with behavior

- **WHEN** every arithmetic axis resolves to a stance under which overflow is undefined by the
  program
- **THEN** the program does not require that integer overflow be reported

#### Scenario: Requirements are per program

- **WHEN** two programs in the same source language resolve different behaviors
- **THEN** they may require different guarantees, and a target option refused for one may be
  permitted for the other

#### Scenario: A default behavior requires what the language requires

- **WHEN** a program resolves the source language's stance on every axis
- **THEN** it requires exactly the guarantees that source language requires
