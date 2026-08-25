## MODIFIED Requirements

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
