## ADDED Requirements

### Requirement: The behavior is selectable

The CLI SHALL accept a behavior, so that what a file compiles to under a given behavior can be
inspected without a build. It SHALL accept either a language name, meaning that language's stance
on every axis, or per-axis assignments naming a language for some axes and leaving the rest to the
default.

With no behavior given, the CLI SHALL use the source language's stance on every axis, matching the
compiler's own default.

An invalid behavior SHALL be reported with the same distinctions the rest of compylr makes, and
SHALL exit unsuccessfully before any source is parsed.

#### Scenario: The default behavior is the source language's

- **WHEN** the CLI emits generated source with no behavior named
- **THEN** the output is what the source language's stance produces

#### Scenario: A language name sets every axis

- **WHEN** the CLI is given the target language as the behavior
- **THEN** every axis takes the target language's stance, and the emitted source differs from the
  default accordingly

#### Scenario: Per-axis assignments are accepted

- **WHEN** the CLI is given a behavior naming one axis
- **THEN** that axis takes the named language's stance and every other axis takes the source
  language's

#### Scenario: An invalid language is rejected

- **WHEN** the CLI is given a behavior naming a language that is neither the source nor the target
- **THEN** it reports the two languages that would have been accepted and exits unsuccessfully

#### Scenario: An unknown axis is rejected

- **WHEN** the CLI is given a behavior naming an axis that does not exist
- **THEN** it lists the axes that do and exits unsuccessfully

#### Scenario: The behavior is visible in both emitted forms

- **WHEN** the CLI emits the IR and then the target source for the same file under a non-default
  behavior
- **THEN** the declared modes in the IR and the operators in the target source agree with each
  other and with the behavior given
