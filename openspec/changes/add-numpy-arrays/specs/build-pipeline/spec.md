## ADDED Requirements

### Requirement: The generated crate declares its array dependencies

A generated crate for a program using arrays SHALL declare the array and array-binding
dependencies, and SHALL declare them only when the program uses arrays.

#### Scenario: A program using arrays declares the dependencies

- **WHEN** the manifest for a program using arrays is generated
- **THEN** it declares the array and array-binding dependencies

#### Scenario: A program not using arrays declares neither

- **WHEN** the manifest for a program using no arrays is generated
- **THEN** it is unchanged from before this change

#### Scenario: Versions are pinned

- **WHEN** the generated manifest is inspected
- **THEN** the added dependencies are pinned, so a build is reproducible

### Requirement: A missing numpy at build time is reported as a setup failure

Where building a program that uses arrays requires numpy and it is absent or incompatible, the
failure SHALL be reported as a located setup failure naming what is missing, rather than surfacing
as a compiler error about generated code.

#### Scenario: Absent numpy names itself

- **WHEN** a program using arrays is built in an environment without numpy
- **THEN** the failure names numpy as the missing requirement, alongside the existing requirements
  on the toolchain

#### Scenario: The failure carries a category

- **WHEN** the failure crosses into the host
- **THEN** it carries the machine-readable category used for setup failures, so a caller can branch
  on it

#### Scenario: A program not using arrays does not require numpy

- **WHEN** a program using no arrays is built in an environment without numpy
- **THEN** the build succeeds
