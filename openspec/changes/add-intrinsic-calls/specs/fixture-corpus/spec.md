## ADDED Requirements

### Requirement: Every supported module is exercised by a driven fixture

Each module the registry supports SHALL have at least one accepted fixture exercising every
operation the registry lists for it, and that fixture SHALL have a driver naming the calls that
exercise it. An operation with no fixture SHALL fail the suite, so a registry entry cannot claim
support that was never translated, built, run, and agreed with CPython.

#### Scenario: An unexercised operation fails the suite

- **WHEN** the registry lists an operation that no accepted fixture calls
- **THEN** the fixture suite fails naming the operation, rather than passing silently

#### Scenario: CPython remains the oracle

- **WHEN** a driver calls a fixture function that uses a supported module
- **THEN** the expected answer is what CPython produces for the same call, and no expected value is
  written into the driver

#### Scenario: Floating-point agreement is compared within a tolerance

- **WHEN** a driver compares a floating-point answer from a mathematical operation
- **THEN** agreement is checked within a stated tolerance rather than by exact equality, because
  the two runtimes may round a transcendental function's last bit differently

#### Scenario: A non-finite answer is compared by classification

- **WHEN** an operation produces a non-finite result under an unchecked mode
- **THEN** agreement is checked by classification, since a not-a-number value is never equal to
  itself

### Requirement: A refused module or operation cannot start compiling

A program importing an unsupported module, or naming an unsupported operation of a supported
module, SHALL appear in the rejected corpus and SHALL fail before lowering produces any IR. Such a
program that begins lowering SHALL fail the suite.

#### Scenario: An unsupported module is refused before lowering

- **WHEN** the rejected corpus contains a program importing an unsupported module
- **THEN** the suite confirms lowering never starts, and the diagnostic is located

#### Scenario: Clearing a refusal means supporting the module

- **WHEN** a module in the rejected corpus becomes supported
- **THEN** its program moves into the accepted corpus with a driver, and no allowance is added to
  the rejection guard
