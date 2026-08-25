## MODIFIED Requirements

### Requirement: Function emission

The backend SHALL emit each function in a unit as a Rust function carrying its name, its
parameters in source order with their spelled types, and its declared return type. A direct
instance parameter SHALL be emitted as a borrow of the instance rather than an owned value: shared
when the function only observes it and mutable when the function mutates it directly or through a
mutable method call. Calls between generated functions SHALL pass instance arguments with the same
borrowing convention. Other parameter types SHALL remain owned. The backend SHALL NOT clone a
borrowed instance parameter to satisfy a return, storage operation, rebinding, or other ownership
use; such input SHALL be rejected with a located diagnostic before backend emission. An instance
return SHALL therefore come from an expression that already produces an owned instance.

Every emitted function SHALL be fallible, yielding either the declared return type or a runtime
error. This is uniform rather than decided per function: any body can contain a division or an
arithmetic overflow, including the body of a function that returns nothing, so a signature that
became fallible only when the backend judged failure possible would change shape on an unrelated
edit and force every caller to change with it.

#### Scenario: Function with parameters and a return type

- **WHEN** a function taking two integers and returning an integer is emitted
- **THEN** the Rust signature names both parameters with type `i64`, and the function yields an
  `i64` on success

#### Scenario: Read-only instance parameter is borrowed

- **WHEN** a free function reads an attribute from a direct instance parameter without mutating it
- **THEN** the emitted Rust function accepts a shared borrow of that instance

#### Scenario: Mutated instance parameter is borrowed mutably

- **WHEN** a free function mutates a direct instance parameter or calls a method that does
- **THEN** the emitted Rust function accepts a mutable borrow and changes the original instance

#### Scenario: Borrowed instance forwarding stays borrowed

- **WHEN** a generated function passes its direct instance parameter to another generated
  function with a compatible direct instance parameter
- **THEN** the emitted call passes a shared or mutable borrow and does not clone the instance

#### Scenario: Owned instance return needs no borrowed clone

- **WHEN** a function returns an instance constructed in its body or received as an owned result
  from another function
- **THEN** the emitted Rust returns that owned value directly

#### Scenario: A borrowed return never reaches emission

- **WHEN** source attempts to return a direct instance parameter as an owned instance result
- **THEN** backend emission is not invoked for that invalid unit

#### Scenario: Function returning unit

- **WHEN** a function annotated `-> None` is emitted
- **THEN** the emitted Rust function yields no value on success

#### Scenario: A unit-returning function can still report failure

- **WHEN** a function annotated `-> None` contains a division by zero
- **THEN** its signature is able to carry the failure, rather than the failure being unreportable

#### Scenario: Every function in the unit appears

- **WHEN** a unit holding three functions is emitted
- **THEN** the output contains all three, in the unit's deterministic order
