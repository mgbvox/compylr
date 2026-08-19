## ADDED Requirements

### Requirement: The decorator accepts a class

The manager's decorator SHALL accept a class as well as a function, marking it for compilation
under the same settings and validating it at the point it is marked.

A marked class SHALL keep the identifying attributes callers and tooling read — its name,
docstring, module, and annotations — and SHALL expose the original uncompiled class, so compiled
and interpreted behaviour can be compared.

#### Scenario: A class is marked

- **WHEN** the decorator is applied to a supported class
- **THEN** the class is marked for compilation under the manager's settings

#### Scenario: Both decorator forms work on a class

- **WHEN** the decorator is applied to a class bare and called with settings
- **THEN** both mark it, differing only in the settings in effect

#### Scenario: An unsupported class is rejected when marked

- **WHEN** a class declaring a base is marked
- **THEN** an error is raised naming the unsupported construct and its location

#### Scenario: Identity attributes are preserved

- **WHEN** a marked class's name, docstring, and module are read
- **THEN** they match those of the class as written

#### Scenario: The original class is reachable

- **WHEN** a caller needs the uncompiled implementation
- **THEN** it is accessible from the marked class

#### Scenario: Instantiating a marked class builds the project

- **WHEN** a marked class is instantiated for the first time
- **THEN** the project is built and the compiled type is used, as calling a marked function does

#### Scenario: Classes and functions share one build

- **WHEN** a project marks both classes and functions
- **THEN** one build covers all of them
