## ADDED Requirements

### Requirement: Class definitions

Lowering SHALL accept a class definition containing an `__init__` and any number of methods, and
SHALL reject a class body containing anything else. Every method SHALL be fully annotated, on the
same terms as a free function, and SHALL take `self` as its first parameter — which SHALL NOT carry
an annotation, since its type is the class being defined.

Inheritance, decorators on methods, class-level attributes, and dunder methods other than `__init__`
SHALL be rejected, each naming what was found.

#### Scenario: A class lowers

- **WHEN** lowering a class with an `__init__` and one method
- **THEN** lowering succeeds and the unit contains the class

#### Scenario: A class without __init__ is rejected

- **WHEN** lowering a class with no `__init__`
- **THEN** lowering fails, because a class's attributes are declared there and nowhere else

#### Scenario: A method must take self

- **WHEN** lowering a method whose first parameter is not `self`
- **THEN** lowering fails naming the method

#### Scenario: self must not be annotated

- **WHEN** lowering a method annotating `self`
- **THEN** lowering fails, because its type is the class being defined

#### Scenario: Method parameters and returns are mandatory

- **WHEN** lowering a method missing a return annotation
- **THEN** lowering fails naming the method

#### Scenario: Inheritance is rejected

- **WHEN** lowering a class declaring a base
- **THEN** lowering fails naming inheritance as unsupported

#### Scenario: A class-level statement is rejected

- **WHEN** lowering a class whose body contains a statement other than a method definition
- **THEN** lowering fails naming the construct

#### Scenario: A dunder other than __init__ is rejected

- **WHEN** lowering a class defining `__eq__`
- **THEN** lowering fails naming the method

#### Scenario: Two methods of the same name are rejected

- **WHEN** lowering a class defining the same method twice
- **THEN** lowering fails reporting the conflict

### Requirement: Attributes are declared in __init__

Every attribute SHALL be declared by an annotated assignment to `self` in `__init__`. An assignment
to an attribute that was not declared there SHALL be rejected, and so SHALL a declaration outside
`__init__`.

Python allows an attribute to appear anywhere, which means an object's shape depends on which
methods have run. A compiled struct's fields cannot depend on that, and requiring the declaration up
front is the same rule the subset already applies to parameters and returns.

#### Scenario: An attribute is declared and typed

- **WHEN** lowering `__init__` containing `self.count: int = 0`
- **THEN** the class carries an attribute `count` of the integer type

#### Scenario: An undeclared attribute is rejected

- **WHEN** lowering a method assigning to an attribute not declared in `__init__`
- **THEN** lowering fails naming the attribute

#### Scenario: An unannotated declaration is rejected

- **WHEN** lowering `__init__` containing `self.count = 0`
- **THEN** lowering fails, because an attribute's type must be written down

#### Scenario: A declaration outside __init__ is rejected

- **WHEN** lowering a method containing an annotated assignment to a new attribute
- **THEN** lowering fails

#### Scenario: An attribute may hold a collection

- **WHEN** lowering `__init__` containing `self._cache: dict[int, int] = {}`
- **THEN** the class carries an attribute of that mapping type

#### Scenario: Every declared attribute must be initialised

- **WHEN** lowering an `__init__` that declares an attribute without a value
- **THEN** lowering fails, because a struct cannot be constructed with a field missing

### Requirement: Attribute access and assignment

Lowering SHALL type an attribute read from the class of the object being read, and SHALL check an
attribute assignment against the declared type, with promotion applying as elsewhere. Reading or
assigning an attribute the class does not declare SHALL be rejected naming it.

Attributes SHALL be mutable: assigning to `self.x` inside a method is permitted, which is what makes
state that outlives a call possible.

#### Scenario: An attribute read is typed

- **WHEN** lowering `self.count` where `count` is an integer attribute
- **THEN** the expression's type is the integer type

#### Scenario: An attribute is assigned

- **WHEN** lowering `self.count = 1`
- **THEN** lowering succeeds

#### Scenario: A wrong type is rejected

- **WHEN** lowering `self.count = "x"` where `count` is an integer
- **THEN** lowering fails reporting both types

#### Scenario: An unknown attribute is rejected

- **WHEN** lowering `self.missing`
- **THEN** lowering fails naming the attribute and the class

#### Scenario: An attribute is read from another object

- **WHEN** lowering `obj.count` where `obj` is an instance parameter
- **THEN** the expression's type is the attribute's type

#### Scenario: A collection attribute may be mutated

- **WHEN** lowering a method that assigns into a mapping attribute
- **THEN** lowering succeeds, unlike the same operation on a collection parameter

### Requirement: Methods and construction

Lowering SHALL type a method call from the method's signature, checking arity and argument types
with promotion, and SHALL type a construction as the class's instance type, checking its arguments
against `__init__`.

Methods and classes SHALL be resolvable across sources on the same terms as functions: a class the
current source does not define leaves a construction's type undetermined rather than failing, and
unit validation catches one that exists nowhere.

#### Scenario: A method call is typed

- **WHEN** lowering `obj.value()` where `value` returns an integer
- **THEN** the expression's type is the integer type

#### Scenario: Construction is typed

- **WHEN** lowering `Counter()` where `Counter` is a class in the source
- **THEN** the expression's type is that class's instance type

#### Scenario: Constructor arguments are checked

- **WHEN** lowering a construction whose arguments do not match `__init__`
- **THEN** lowering fails reporting the mismatch

#### Scenario: Method arity is checked

- **WHEN** lowering a method call with the wrong number of arguments
- **THEN** lowering fails reporting both counts

#### Scenario: An unknown method is rejected

- **WHEN** lowering a call to a method the class does not define
- **THEN** lowering fails naming the method and the class

#### Scenario: A method may call another on the same object

- **WHEN** lowering a method whose body calls `self.other()`
- **THEN** lowering succeeds

#### Scenario: A class in another source leaves construction undetermined

- **WHEN** lowering a construction of a class this source does not define
- **THEN** lowering succeeds with an undetermined type, and unit validation resolves it
