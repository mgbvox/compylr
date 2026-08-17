## ADDED Requirements

### Requirement: A class emits a struct and an implementation

The backend SHALL emit each class as a data type carrying its attributes as fields in declaration
order, and an implementation block carrying its methods. Attribute types SHALL use the same
spellings every other type does.

#### Scenario: Attributes become fields

- **WHEN** a class declaring three attributes is emitted
- **THEN** the emitted type carries three fields with the corresponding spellings

#### Scenario: Methods become an implementation

- **WHEN** a class with two methods is emitted
- **THEN** both appear in one implementation block for that type

#### Scenario: __init__ becomes a constructor

- **WHEN** a class is emitted
- **THEN** it carries a constructor initialising every field

#### Scenario: Methods are fallible

- **WHEN** a method is emitted
- **THEN** it yields either its declared return type or a runtime error, on the same terms as every
  free function

#### Scenario: Emission is deterministic

- **WHEN** the same unit containing classes is emitted twice
- **THEN** the two outputs are byte-identical

#### Scenario: Classes and functions are emitted into the same file

- **WHEN** a unit holding both is emitted
- **THEN** the translated file holds both, with nothing else added to the crate root

### Requirement: A method takes a mutable receiver only when it needs one

The backend SHALL emit a method that assigns to an attribute, or mutates a collection attribute,
with a mutable receiver, and every other method with a shared one.

Emitting a mutable receiver everywhere would make two methods unusable on the same object at once,
and the failure would be a borrow-checker complaint about generated code rather than a diagnostic
about the user's program.

#### Scenario: A mutating method compiles

- **WHEN** a method that assigns to an attribute is emitted
- **THEN** the emitted Rust compiles

#### Scenario: A reading method takes a shared receiver

- **WHEN** a method that only reads attributes is emitted
- **THEN** its receiver is shared, so it can be called while another borrow is held

#### Scenario: A method mutating a collection attribute is mutating

- **WHEN** a method that inserts into a mapping attribute is emitted
- **THEN** it takes a mutable receiver and the emitted Rust compiles

#### Scenario: A method calling a mutating method is mutating

- **WHEN** a method whose body calls another method that mutates is emitted
- **THEN** it also takes a mutable receiver, since it mutates transitively

#### Scenario: Reading and mutating compose

- **WHEN** a method reads an attribute, calls a mutating method, and reads again
- **THEN** the emitted Rust compiles

### Requirement: Attribute access and construction are emitted

The backend SHALL emit attribute reads, attribute assignments, and constructions. A collection or
instance attribute SHALL be read without being moved out of the object.

#### Scenario: An attribute read yields its value

- **WHEN** a method reading an integer attribute is emitted and executed
- **THEN** the value is the attribute's

#### Scenario: An attribute assignment persists

- **WHEN** a method assigns an attribute and a later call reads it
- **THEN** the later call observes the assigned value

#### Scenario: A collection attribute is not moved by a read

- **WHEN** a method reads a mapping attribute twice
- **THEN** the emitted Rust compiles

#### Scenario: Construction initialises every field

- **WHEN** a construction is emitted and executed
- **THEN** the resulting object's attributes hold what `__init__` assigned

#### Scenario: State outlives a call

- **WHEN** a method mutates an attribute and is called twice
- **THEN** the second call observes the first call's effect — which is what makes a cache possible
