## ADDED Requirements

### Requirement: Behavior is settable globally and per member

The API SHALL accept a `behavior` setting on initialization and on the decorator. Set on
initialization it SHALL be the project's default for every marked member; set on the decorator it
SHALL apply to that member alone.

`behavior` SHALL accept either a language name — meaning that language's stance on every axis — or
a behavior object naming a language for some axes and leaving the rest to be inherited. A language
name SHALL be exactly equivalent to a behavior object naming that language for every axis.

The default SHALL be the source language, so a project that never mentions `behavior` compiles
exactly as it did before the setting existed.

#### Scenario: A global behavior applies to every member

- **WHEN** initialization sets the behavior to the target language and two functions are marked
  with the bare decorator
- **THEN** both compile under the target language's stance on every axis

#### Scenario: A per-member behavior overrides the global

- **WHEN** initialization sets the behavior to the source language and one function is marked with
  the target language
- **THEN** that function compiles under the target's stance and every other member under the
  source's

#### Scenario: A behavior object inherits the axes it does not name

- **WHEN** a member is marked with a behavior object naming one axis, under a global default of the
  source language
- **THEN** that axis takes the named language's stance and every other axis takes the source
  language's

#### Scenario: A behavior object inherits from a non-default global

- **WHEN** the global behavior is the target language and a member names one axis as the source
  language
- **THEN** that axis takes the source language's stance and every other axis takes the target's

#### Scenario: The two spellings are equivalent

- **WHEN** one function is marked with the target language's name and another with a behavior
  object naming the target for every axis
- **THEN** the two compile to identical code

#### Scenario: Omitting behavior changes nothing

- **WHEN** a project marks members without mentioning behavior anywhere
- **THEN** the generated code is identical to what the same project produced before the setting
  existed

### Requirement: An invalid behavior is rejected where it was written

A behavior naming anything other than the source or the target language of the compilation SHALL be
rejected when the member is marked, or when initialization is called, rather than at a later build.
The error SHALL name the two languages that would have been accepted.

The message SHALL distinguish a name compylr does not know at all from a name that is a registered
or reserved language but is not one of the two in this compilation. An axis name that does not exist
SHALL likewise be rejected, with the valid axis names listed.

#### Scenario: An unknown language is rejected at the decorator

- **WHEN** a function is marked with a behavior naming a language compylr has no component for
- **THEN** an error is raised as the decorator runs, naming the two languages that would have been
  accepted

#### Scenario: A reserved language is rejected distinctly

- **WHEN** a function in a Python-to-Rust project is marked with a behavior naming a language
  compylr has reserved but which is neither Python nor Rust
- **THEN** an error is raised whose message distinguishes it from an unknown name

#### Scenario: An invalid global behavior is rejected at initialization

- **WHEN** initialization is called with a behavior naming a language that is neither the source nor
  the target
- **THEN** an error is raised before any member is marked

#### Scenario: An unknown axis is rejected

- **WHEN** a behavior object is constructed naming an axis that does not exist
- **THEN** an error is raised listing the axes that do

#### Scenario: A per-axis value is validated like a bare name

- **WHEN** a behavior object names a valid axis with an invalid language
- **THEN** an error is raised naming both the axis and the two languages that would have been
  accepted

### Requirement: Members of one project may have different behaviors

Two members of the same project MAY be marked with different behaviors and SHALL compile into the
same shared artifact. This SHALL NOT be refused the way a mixed backend is: a backend decides what
artifact is produced, while a behavior decides what individual operations mean, and operations of
different meanings coexist in one artifact.

A member under one behavior calling a member under another SHALL work, and each SHALL keep its own
meanings.

#### Scenario: Mixed behavior builds one artifact

- **WHEN** one function is marked with the source language's behavior and another with the target's
- **THEN** both are built into the same artifact and both are callable

#### Scenario: A mixed-behavior call keeps each side's meaning

- **WHEN** a function under the source language's behavior calls one under the target's, and both
  compute a floor division of a negative dividend
- **THEN** the caller's result follows the source language's rounding and the callee's follows the
  target's

#### Scenario: A mixed backend is still refused

- **WHEN** two members of one project are marked with different backends
- **THEN** the build is still refused, because a project compiles to one shared artifact

### Requirement: A behavior change rebuilds

Changing a member's behavior SHALL cause the project to rebuild on its next run, without the user
clearing a cache. Behavior determines what the program computes, so it SHALL be part of what the
rebuild key distinguishes.

#### Scenario: Changing a behavior rebuilds

- **WHEN** a project is built, a member's behavior is then changed, and the project is run again
- **THEN** the toolchain runs again and the new behavior is what executes

#### Scenario: An unchanged behavior does not rebuild

- **WHEN** a project is built and run again with nothing changed
- **THEN** the cached artifact is reused

## MODIFIED Requirements

### Requirement: Initialization produces a configured manager

The package SHALL provide an initialization entry point that returns a manager carrying the
project's default settings. It SHALL accept the target backend, the assist mode, and the default
behavior, and all SHALL have defaults so that initialization with no arguments is valid.

#### Scenario: Explicit configuration

- **WHEN** initialization is called with a backend, assist mode, and behavior
- **THEN** a manager carrying those settings is returned

#### Scenario: Defaults

- **WHEN** initialization is called with no arguments
- **THEN** a manager is returned whose backend is the implemented default, whose assist mode
  is disabled, and whose behavior is the source language's on every axis

#### Scenario: One manager per project

- **WHEN** initialization is called a second time with the same settings
- **THEN** the same manager is returned, preserving the one-shared-artifact invariant

#### Scenario: Conflicting reconfiguration is refused

- **WHEN** initialization is called a second time with different settings
- **THEN** an error is raised naming the conflicting setting, rather than silently changing the
  defaults of a project that is already partly configured

#### Scenario: A differing default behavior is a conflicting reconfiguration

- **WHEN** initialization is called a second time with the same backend but a different behavior
- **THEN** an error is raised, because members marked before the change would otherwise compile
  under a behavior their author never chose

### Requirement: Per-function settings override the manager's

A setting given on the decorator SHALL apply to that function only. A setting not given SHALL
be inherited from the manager. Overriding SHALL NOT alter the manager's defaults for any other
function. Where a setting is itself composite — as a behavior is — the parts it does not name
SHALL be inherited individually rather than the whole setting being replaced.

#### Scenario: Override applies to one function only

- **WHEN** one function overrides the backend and another uses the bare decorator
- **THEN** the first uses the override and the second uses the manager's backend

#### Scenario: Unspecified settings are inherited

- **WHEN** the decorator is called specifying only the assist mode
- **THEN** the backend and the behavior are inherited from the manager

#### Scenario: An unnamed axis is inherited, not reset

- **WHEN** the decorator is called with a behavior object naming one axis
- **THEN** the remaining axes keep the manager's stances rather than reverting to any default
