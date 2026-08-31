## Purpose

Translates compylr IR into deterministic, standalone C++26 source and a build manifest. Owns the
IR-to-C++ type spellings, the fallible-call convention generated code uses in place of exceptions,
the runtime helpers that implement each behavior axis, C++'s own declaration of what it means on
those axes, and which guarantees it preserves.

## ADDED Requirements

### Requirement: The backend targets C++26 and says so in its manifest

The backend SHALL emit source valid under **C++26** and SHALL emit a build manifest that selects
that standard, so that a reader of the generated tree can build it without being told which
standard to ask for. The emitted manifest SHALL name the minimum compiler versions the generated
source requires.

Support for C++26 is partial and uneven across shipping compilers. The backend SHALL therefore not
rely on a language or library feature whose absence would produce a compile error a user cannot
act on; where a relied-upon feature is unavailable, the failure SHALL name the feature and the
compiler versions that provide it.

#### Scenario: The generated manifest selects the standard

- **GIVEN** a unit with at least one function
- **WHEN** the unit is emitted for the `cpp` backend
- **THEN** the emitted files include a build manifest
- **AND** that manifest selects the C++26 standard

#### Scenario: An unsupported compiler is named, not merely failed against

- **GIVEN** a generated C++ tree
- **WHEN** it is built with a compiler that does not provide the standard features it uses
- **THEN** the failure names the missing feature and the compiler versions that provide it

### Requirement: Concrete C++ type spellings

The backend SHALL map each IR type to a C++ type. The mapping SHALL live in the C++ backend alone,
SHALL be derived from IR types, and SHALL be independent of which frontend produced the unit.

| IR type | C++ type |
| --- | --- |
| integer | `int64_t` |
| float | `double` |
| bool | `bool` |
| string | `std::string` |
| unit | `void` |
| sequence of `T` | `std::vector<T>` |
| mapping from `K` to `V` | `std::unordered_map<K, V>` |
| set of `T` | `std::unordered_set<T>` |
| tuple of `T1..Tn` | `std::tuple<T1, .., Tn>` |
| instance of `Class` | `ClassName` |

#### Scenario: Scalar types are spelled

- **GIVEN** a unit whose functions take integer, float, bool, and string parameters
- **WHEN** the unit is emitted for the `cpp` backend
- **THEN** the emitted signatures spell those parameters `int64_t`, `double`, `bool`, and
  `std::string`

#### Scenario: Collection types are spelled recursively

- **GIVEN** a unit with a parameter that is a sequence of mappings from string to integer
- **WHEN** the unit is emitted for the `cpp` backend
- **THEN** the emitted parameter type is `std::vector<std::unordered_map<std::string, int64_t>>`

#### Scenario: A spelling does not depend on the producing frontend

- **GIVEN** two units holding the same IR, one lowered from Python and one from TypeScript
- **WHEN** each is emitted for the `cpp` backend
- **THEN** the emitted source is byte-identical

### Requirement: A fallible operation returns a value, and nothing throws across a boundary

An operation whose resolved behavior can report a failure SHALL be emitted so that the failure is
returned as a value rather than thrown. A generated function whose body contains such an operation
SHALL return `std::expected<T, compylr::Error>`, and a failure SHALL propagate out of the function
rather than being discarded.

No generated function SHALL let an exception escape across the boundary a host bridge exports,
because the calling runtime has no way to observe one.

#### Scenario: A checked operation makes its function fallible

- **GIVEN** a unit whose function divides two integers under a resolved behavior that reports
  division by zero
- **WHEN** the unit is emitted for the `cpp` backend
- **THEN** the emitted function returns `std::expected` over its declared return type

#### Scenario: A function with no fallible operation stays plain

- **GIVEN** a unit whose function only adds two integers under a resolved behavior that leaves
  overflow undefined
- **WHEN** the unit is emitted for the `cpp` backend
- **THEN** the emitted function returns its declared return type directly

#### Scenario: A failure propagates rather than being dropped

- **GIVEN** a unit whose function calls another function that can fail
- **WHEN** the unit is emitted for the `cpp` backend and the inner call fails at run time
- **THEN** the outer function returns the same failure
- **AND** the outer function does not continue past the failing call

#### Scenario: Nothing escapes as an exception

- **GIVEN** a generated C++ tree for any unit
- **WHEN** any exported entry point is called with any argument
- **THEN** no exception propagates out of it

### Requirement: Runtime helpers implement the behavior axes the unit resolved

The backend SHALL emit a compatibility header holding the helpers that implement each behavior
axis. Emitted code SHALL select a helper by matching on the **modes** an IR node carries — the
rounding and checking of a division, the sign convention and checking of a remainder, the origin
and checking of a subscript, the units of a text length — and SHALL NOT select one by the
operation's name.

Reading the name would be silently wrong for the other stance, which is what
[`Axis`](../../../../../crates/compylr-ir/src/behavior.rs#L36) exists to prevent.

#### Scenario Outline: Integer division is emitted from its resolved modes

- **GIVEN** a unit whose behavior resolves integer division to `<rounding>` with `<checking>`
- **WHEN** the expression `-7 // 2` is emitted for the `cpp` backend and run
- **THEN** the result is `<result>`

**Examples:**

| rounding        | checking    | result                     |
| --------------- | ----------- | -------------------------- |
| TowardNegInf    | Reported    | `-4`                       |
| TowardZero      | Reported    | `-3`                       |
| TowardNegInf    | Unchecked   | `-4`                       |

#### Scenario: A checked division by zero reports rather than trapping

- **GIVEN** a unit whose behavior resolves integer division to a checking mode that reports
- **WHEN** the emitted code divides by zero
- **THEN** the call returns a failure naming division by zero
- **AND** the process does not trap

#### Scenario: An index from the end is resolved by the helper

- **GIVEN** a unit whose behavior resolves sequence indexing to an origin that counts from the end
- **WHEN** a sequence is indexed with `-1` in emitted code and run
- **THEN** the last element is returned

#### Scenario: Text length is counted in the units the unit resolved

- **GIVEN** a unit whose behavior resolves text length to Unicode code points
- **WHEN** the length of a string holding one non-ASCII character is taken in emitted code and run
- **THEN** the result is `1`

### Requirement: C++ declares its own stance, and separately what it preserves

The C++ backend SHALL declare, for every behavior axis, what C++ **itself** means by that
operation, and SHALL separately declare which semantic guarantees it **preserves**. It SHALL
declare nothing about any other language's meaning.

These are different questions and SHALL NOT be conflated. C++'s native stance leaves signed integer
overflow undefined, truncates integer division toward zero, takes the dividend's sign for a
remainder, indexes from the start without checking, and counts text in UTF-8 bytes. What the
backend *preserves* is wider than that, because the compatibility helpers implement each checked
mode: the backend SHALL preserve that integer overflow is reported, that division by zero is
reported, and that floating-point arithmetic is not reordered.

Deriving the preserved set from the native stance would refuse every program whose source language
reports overflow, which is every default Python program. This is the same separation the Rust
backend already draws — its native stance is unchecked on every axis and it preserves all three
guarantees.

#### Scenario: The declaration is complete

- **WHEN** the C++ backend's behavior is inspected
- **THEN** it answers for all six axes

#### Scenario: The declaration names only C++

- **WHEN** the C++ backend's behavior declaration is inspected
- **THEN** it names no language other than C++

#### Scenario: The native stance leaves overflow undefined

- **WHEN** the C++ backend's stance on integer overflow is inspected
- **THEN** it declares the result undefined by the program

#### Scenario: A program requiring overflow reporting still compiles

- **GIVEN** a unit whose resolved behavior requires that integer overflow be reported
- **WHEN** it is negotiated against the `cpp` backend
- **THEN** negotiation succeeds
- **AND** the emitted code reports an integer result outside the target's range rather than
  wrapping it

#### Scenario: A default Python program reaches C++ unchanged

- **GIVEN** a unit lowered from Python with no behavior requested
- **WHEN** it is compiled for the `cpp` backend
- **THEN** compilation succeeds
- **AND** every answer the compiled code gives is the one the Python source gives

### Requirement: A C++26 transformation is declared and refused rather than silently absent

The backend SHALL declare a target option covering the C++26 contract facilities, which would let a
checked mode be expressed as a precondition rather than as a branch, and SHALL report it as
reserved when it is permitted. Permitting a reserved option SHALL fail saying so rather than
silently doing nothing.

This is the same three-way honesty the registries use, and the reason the Rust backend declares
[`unchecked-arithmetic`](../../../../../crates/compylr-backend-rust/src/rust.rs#L202).

#### Scenario: The option is listed

- **WHEN** the C++ backend's target options are inspected
- **THEN** the contract option appears among them

#### Scenario: Permitting a reserved option is refused

- **GIVEN** a unit that may be compiled for the `cpp` backend
- **WHEN** the contract option is explicitly permitted
- **THEN** the request fails saying the option is reserved
- **AND** emission does not silently proceed as if it had been applied

### Requirement: Emission is pure and produces a buildable tree

Emission SHALL be a pure function of the unit: no filesystem access, no environment inspection, and
no invocation of a compiler. The emitted files SHALL together form a tree that builds without any
file being added by hand.

Formatting SHALL be applied as post-processing rather than during emission, SHALL change only how
the source reads, and SHALL fall back to the unformatted text when the formatter is unavailable.

#### Scenario: The same unit emits the same bytes

- **GIVEN** the same unit
- **WHEN** it is emitted for the `cpp` backend twice in different environments
- **THEN** the emitted source is byte-identical

#### Scenario: Emission touches no file

- **GIVEN** a unit
- **WHEN** it is emitted for the `cpp` backend
- **THEN** no file is read or written by emission

#### Scenario: The emitted tree is complete

- **GIVEN** a unit with functions and classes
- **WHEN** it is emitted for the `cpp` backend
- **THEN** the emitted files include the build manifest, the compatibility header, and the
  translated source
- **AND** building that tree with no file added succeeds

#### Scenario: A missing formatter costs readability and nothing else

- **GIVEN** a machine with no C++ formatter installed
- **WHEN** emitted files are post-processed
- **THEN** the files are returned unformatted
- **AND** they still build

### Requirement: A class emits a type whose state outlives a call

An IR class SHALL emit a C++ type holding one member per declared attribute, a constructor
initializing every one of them, and one method per IR method. A method that mutates an attribute
SHALL be emitted so that the mutation is observable to the next call on the same instance.

#### Scenario: An instance keeps its state between calls

- **GIVEN** a unit with a class whose method increments an integer attribute
- **WHEN** the emitted code constructs an instance and calls that method twice
- **THEN** the attribute's value reflects both calls

#### Scenario: A constructor initializes every attribute

- **GIVEN** a unit with a class declaring three attributes
- **WHEN** the unit is emitted for the `cpp` backend
- **THEN** the emitted constructor assigns all three
