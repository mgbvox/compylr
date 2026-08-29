## ADDED Requirements

### Requirement: The generated crate declares its array dependencies

A generated crate for a program using arrays SHALL declare the array and array-binding
dependencies, and SHALL declare them only when the program uses arrays. Added dependencies SHALL be
pinned.

#### Scenario: A program using arrays declares the dependencies

- **GIVEN** a program using arrays
- **WHEN** its manifest is generated
- **THEN** the manifest declares the array and array-binding dependencies

#### Scenario: A program not using arrays declares neither

- **GIVEN** a program using no arrays
- **WHEN** its manifest is generated
- **THEN** the manifest is unchanged from before this change

#### Scenario: Versions are pinned

- **GIVEN** a generated manifest declaring the array dependencies
- **WHEN** it is inspected
- **THEN** the added dependencies are pinned, so a build is reproducible

### Requirement: A missing numpy at build time is reported as a setup failure

Where building a program that uses arrays requires numpy and it is absent or incompatible, the
failure SHALL be reported as a located setup failure naming what is missing, rather than surfacing
as a compiler error about generated code. This joins `cargo` and `maturin` on the list of things
compiling needs at runtime.

#### Scenario: Absent numpy names itself

- **GIVEN** an environment without numpy
- **WHEN** a program using arrays is built
- **THEN** the failure names numpy as the missing requirement, alongside the existing requirements
  on the toolchain
- **BUT** it is not a compiler error about generated code

#### Scenario: The failure carries a category

- **GIVEN** a setup failure raised while building a program that uses arrays
- **WHEN** it crosses into the host
- **THEN** it carries the machine-readable category used for setup failures, so a caller can branch
  on it

#### Scenario: A program not using arrays does not require numpy

- **GIVEN** an environment without numpy
- **WHEN** a program using no arrays is built
- **THEN** the build succeeds
