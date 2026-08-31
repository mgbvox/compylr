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

- **GIVEN** a compylr build
- **WHEN** the set of behavior axes is requested
- **THEN** exactly the six named axes are returned
- **AND** each carries a stable identifier a diagnostic and a test can both refer to

#### Scenario: An axis names an operation, not an implementation

- **GIVEN** one of the six axes
- **WHEN** it is described to a user
- **THEN** it is described by the operation it governs
- **AND** by what each language means by that operation

### Requirement: A language declares its own stance on every axis

Each source language and each target language SHALL declare, for every axis, what its own language
means by that operation. A language SHALL NOT declare anything about another language's meaning,
and no component SHALL hold a table mapping one language's stance onto another's.

#### Scenario: Both endpoints declare a stance

- **GIVEN** a source language and a target language resolved for a compilation
- **WHEN** each is asked what it means
- **THEN** each answers for every axis
- **BUT** neither answers on behalf of the other

#### Scenario: A language's declaration is complete

- **GIVEN** a language offered for registration
- **WHEN** its behavior declaration is read
- **THEN** it covers every axis
- **AND** a language omitting one could not be registered

#### Scenario Outline: Every implemented language declares a complete stance

- **GIVEN** the implemented language `<language>` in its role as `<role>`
- **WHEN** its behavior is inspected
- **THEN** it returns a stance for all six axes
- **BUT** it names no other language

  **Examples:**

  | language | role |
  | --- | --- |
  | `python` | frontend |
  | `typescript` | frontend |
  | `rust` | backend |
  | `go` | backend |

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

- **GIVEN** a compilation whose request names the target language and nothing else
- **WHEN** the request is resolved
- **THEN** every axis resolves to the target language's stance

#### Scenario: An unspecified axis inherits

- **GIVEN** a request naming one axis, with the source language as the enclosing default
- **WHEN** the request is resolved
- **THEN** that axis takes the named language's stance
- **AND** every other axis takes the source language's
- **BUT** naming the one axis has not reset the others

#### Scenario: The two spellings agree

- **GIVEN** a bare language name and a per-axis selection naming that language for every axis
- **WHEN** each is resolved against the same pair
- **THEN** the two resolved behaviors are identical

#### Scenario: A resolved behavior is total

- **GIVEN** any behavior request
- **WHEN** it is resolved
- **THEN** every axis has exactly one stance
- **AND** no axis is left to be decided later

### Requirement: Only the two languages in the compilation may be named

A behavior request SHALL name only the source language or the target language of the compilation
it applies to. Any other name SHALL be rejected before compilation begins, with a message that
names the two languages that would have been accepted.

The rejection SHALL distinguish a name compylr does not know from a name that is a registered or
reserved language but is not one of the two in this compilation, because the two are different
mistakes and the second is the one a user is likely to make.

#### Scenario: An unknown language is rejected

- **GIVEN** a behavior request naming a language compylr has no frontend or backend for
- **WHEN** the request is resolved
- **THEN** it is rejected
- **AND** the message says the name is not a language compylr knows
- **AND** it names the two languages that would have been accepted

#### Scenario: A known but absent language is rejected distinctly

- **GIVEN** a Python-to-Rust compilation whose request names a registered or reserved language
  that is neither Python nor Rust
- **WHEN** the request is resolved
- **THEN** it is rejected
- **AND** the message distinguishes this from an unknown name
- **AND** it names the two languages of this compilation

#### Scenario: An unknown axis is rejected

- **GIVEN** a per-axis selection naming an axis that does not exist
- **WHEN** the request is resolved
- **THEN** it is rejected
- **AND** the message lists the axes that do exist

#### Scenario: Rejection precedes compilation

- **GIVEN** an invalid behavior request
- **WHEN** the compilation is attempted
- **THEN** it is rejected before any source is lowered
- **AND** before any target source exists

### Requirement: The source language is the default

When no behavior is requested, every axis SHALL resolve to the **source** language's stance. A
compilation with no behavior request SHALL therefore produce exactly what it produced before
behavior existed.

#### Scenario: No request means the source language

- **GIVEN** a compilation with no behavior request
- **WHEN** the behavior is resolved
- **THEN** every axis resolves to the source language's stance

#### Scenario: The default is not the target's

- **GIVEN** a source and target that disagree on an axis, and no behavior request
- **WHEN** the program runs
- **THEN** the source language's meaning is what the generated code produces

### Requirement: A resolved behavior determines what the program requires preserved

A resolved behavior SHALL determine which semantic guarantees the compiled program requires. An
axis resolved to a stance under which an operation's failure is not defined by the program SHALL
NOT contribute the guarantee that the failure be reported.

The guarantees a program requires SHALL therefore be a property of the program rather than of its
source language, so that a target transformation forbidden for one program may be permitted for
another written in the same language.

#### Scenario: Requirements shrink with behavior

- **GIVEN** a program whose every arithmetic axis resolves to a stance leaving overflow undefined
- **WHEN** the guarantees it requires are computed
- **THEN** it does not require that integer overflow be reported

#### Scenario: Requirements are per program

- **GIVEN** two programs in the same source language resolving different behaviors
- **WHEN** the guarantees each requires are computed
- **THEN** they may differ
- **AND** a target option refused for one may be permitted for the other

#### Scenario: A default behavior requires what the language requires

- **GIVEN** a program resolving the source language's stance on every axis
- **WHEN** the guarantees it requires are computed
- **THEN** they are exactly the guarantees that source language requires
