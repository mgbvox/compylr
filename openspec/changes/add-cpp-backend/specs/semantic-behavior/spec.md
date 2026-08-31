## MODIFIED Requirements

### Requirement: A language declares its own stance on every axis
Each source language and each target language SHALL declare, for every axis, what its own language
means by that operation. A language SHALL NOT declare anything about another language's meaning,
and no component SHALL hold a table mapping one language's stance onto another's. Both TypeScript and Go
SHALL declare their own complete stance on all six behavior axes, and so SHALL C++.

A language's declared stance SHALL be kept distinct from the set of guarantees a target preserves.
A stance says what the language's own operator means; a preserved guarantee says what the target can
be made to do when the resolved behavior asks for it. A target whose native stance leaves a failure
undefined MAY still preserve the guarantee that the failure is reported, and SHALL NOT be assumed
not to. Deriving one from the other would refuse every program whose source language reports a
failure the target's operator does not.

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

#### Scenario: C++ declares its behavior profile
- **GIVEN** the C++ backend
- **WHEN** its behavior is inspected
- **THEN** it returns a complete stance for all six axes
- **AND** the declaration names no language other than C++

#### Scenario: An undefined stance does not by itself withhold a guarantee
- **GIVEN** a target language whose stance on integer overflow leaves the result undefined
- **AND** whose emitted code can report an out-of-range integer result when asked to
- **WHEN** the guarantees it preserves are inspected
- **THEN** the guarantee that integer overflow is reported is present

#### Scenario: A program requiring a guarantee the target withholds is refused before emission
- **GIVEN** a unit whose resolved behavior requires a guarantee
- **WHEN** it is compiled for a target that does not declare that guarantee preserved
- **THEN** compilation fails naming the guarantee
- **AND** no target source is generated
