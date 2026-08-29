## ADDED Requirements

### Requirement: Every supported module is exercised by a driven fixture

Each module the registry supports SHALL have at least one accepted fixture in
[`accepted/`](../../../../../frontends/python/fixtures/accepted/) exercising every operation the
registry lists for it, and that fixture SHALL have a driver in
[`drivers/`](../../../../../frontends/python/fixtures/drivers/) naming the calls that exercise it.
An operation with no fixture SHALL fail the suite, so a registry entry cannot claim support that was
never translated, built, run, and agreed with CPython. The coverage check SHALL be derived from the
registry rather than written as a list.

#### Scenario: An unexercised operation fails the suite

- **GIVEN** a registry listing an operation that no accepted fixture calls
- **WHEN** the fixture suite runs
- **THEN** the suite fails naming the operation
- **BUT** it does not pass silently

#### Scenario: CPython remains the oracle

- **GIVEN** a driver calling a fixture function that uses a supported module
- **WHEN** the driver runs against the compiled and the interpreted implementation
- **THEN** the expected answer is what CPython produces for the same call
- **AND** no expected value is written into the driver

#### Scenario: Floating-point agreement is compared within a tolerance

- **GIVEN** a driver comparing a floating-point answer from a mathematical operation
- **WHEN** agreement is checked
- **THEN** it is checked within a stated tolerance rather than by exact equality, because the two
  runtimes may round a transcendental function's last bit differently

#### Scenario: A non-finite answer is compared by classification

- **GIVEN** an operation producing a non-finite result under the unchecked mode
- **WHEN** agreement is checked
- **THEN** it is checked by classification, since a not-a-number value is never equal to itself

### Requirement: A refused module or operation cannot start compiling

A program importing an unsupported module, or naming an unsupported operation of a supported
module, SHALL appear in [`rejected/`](../../../../../frontends/python/fixtures/rejected/) and SHALL
fail before lowering produces any IR. Such a program that begins lowering SHALL fail the suite,
under the inverted guard the rejection corpus already carries.

#### Scenario: An unsupported module is refused before lowering

- **GIVEN** a rejected-corpus program importing an unsupported module
- **WHEN** the corpus suite runs
- **THEN** the suite confirms lowering never starts
- **AND** the diagnostic is located

#### Scenario: Clearing a refusal means supporting the module

- **GIVEN** a module in the rejected corpus that becomes supported
- **WHEN** the change that supports it lands
- **THEN** its program moves into the accepted corpus with a driver
- **BUT** no allowance is added to the rejection guard
