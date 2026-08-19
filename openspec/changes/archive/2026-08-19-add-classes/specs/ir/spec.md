## ADDED Requirements

### Requirement: A unit holds classes as well as functions

The IR SHALL model a class as a member of a compilation unit, carrying its name, its attributes
with their types in declaration order, and its methods. Classes and functions SHALL share one
namespace: a unit SHALL refuse a class whose name is already taken by a function, and the reverse.

A unit's ordering and fingerprint guarantees SHALL extend to classes: members SHALL be exposed in
an order determined by content rather than by addition order, and a unit's fingerprint SHALL cover
each class's structure.

#### Scenario: A class is a unit member

- **WHEN** a class is added to a unit
- **THEN** the unit contains it, alongside any functions

#### Scenario: Names are shared across kinds

- **WHEN** a class is added to a unit already containing a function of that name
- **THEN** the unit refuses the addition and reports the conflicting name

#### Scenario: Ordering is content-determined

- **WHEN** the same classes and functions are added to two units in different orders
- **THEN** both expose their members in the same order

#### Scenario: A class contributes to the fingerprint

- **WHEN** a method body is changed
- **THEN** the unit's fingerprint differs from its previous value

#### Scenario: A unit without classes fingerprints unchanged

- **WHEN** a unit containing only functions is fingerprinted
- **THEN** the value is what it was before classes existed, so existing caches stay valid

#### Scenario: Attribute order follows declaration

- **WHEN** a class declaring three attributes is represented in the IR
- **THEN** they appear in the order declared

### Requirement: Instance types

The type model SHALL gain an instance type naming a class. It SHALL be usable wherever a type is,
including as a collection's parameter, and SHALL be distinct from every scalar and from every other
class's instance type.

An instance type SHALL NOT be usable as a mapping key or set element: the type model restricts
those to what can be compared and hashed, and an instance has no defined ordering or hash here.

#### Scenario: A class name is a type

- **WHEN** a value is declared with a class's name as its annotation
- **THEN** its IR type is that class's instance type

#### Scenario: Two classes are distinct types

- **WHEN** the instance types of two different classes are compared
- **THEN** they are different types

#### Scenario: Instances nest in collections

- **WHEN** a value is declared as a sequence of a class
- **THEN** its IR type is a sequence whose element type is that instance type

#### Scenario: An instance cannot be a key

- **WHEN** a mapping keyed by an instance type is considered
- **THEN** the type model provides no IR type for it

#### Scenario: An instance is not trivially copyable

- **WHEN** the copyability of an instance type is considered
- **THEN** it is treated as a type that must be cloned where consumed, like a collection

### Requirement: Attribute and construction forms

The IR SHALL support reading an attribute, assigning an attribute, and constructing an instance.
Construction SHALL carry the class name and its arguments, distinct from a call to a function.

#### Scenario: Attribute read

- **WHEN** an attribute is read
- **THEN** the IR carries the object expression and the attribute name

#### Scenario: Attribute assignment

- **WHEN** an attribute is assigned
- **THEN** the IR carries the object expression, the attribute name, and the value

#### Scenario: Construction is distinct from a call

- **WHEN** a class is constructed
- **THEN** the IR represents it as a construction carrying the class name, not as a function call

#### Scenario: The new forms survive the artifact

- **WHEN** a unit containing a class, attribute access, attribute assignment, and construction is
  round-tripped
- **THEN** the result compares structurally equal to the original

#### Scenario: The artifact stays target-neutral

- **WHEN** an artifact describing a class is inspected
- **THEN** it names IR forms only, containing no target-language struct or trait syntax
