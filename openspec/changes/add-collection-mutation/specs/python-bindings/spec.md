## MODIFIED Requirements

### Requirement: Collections cross the boundary by value

A collection passed to a compiled function SHALL be converted into an independent value. A
compiled function therefore SHALL NOT be able to affect a collection its caller still holds.

This differs from calling an interpreted Python function, which receives a reference. Mutation now
exists in the subset, so the difference would be observable — a compiled function mutating a
parameter would leave its caller's collection unchanged where an interpreted one would not, which
is a wrong answer with no error.

Mutation is therefore **confined to locals**: lowering rejects mutating a parameter. The divergence
is unreachable rather than documented, and this requirement records why the restriction exists, so
that relaxing it later has to supply reference semantics first.

#### Scenario: The caller's list is unaffected

- **WHEN** a caller passes a list to a compiled function and inspects it afterwards
- **THEN** the list is unchanged

#### Scenario: A compiled function cannot mutate a parameter at all

- **WHEN** a function attempting to mutate a collection parameter is marked
- **THEN** it is rejected, so no program exists in which the divergence could be observed

#### Scenario: A returned collection is independent

- **WHEN** a compiled function returns a collection and the caller modifies the result
- **THEN** nothing inside the compiled module is affected

#### Scenario: A locally built collection is returned by value

- **WHEN** a compiled function builds a collection and returns it
- **THEN** the caller receives an independent Python object holding the built contents

#### Scenario: Large collections still convert correctly

- **WHEN** a compiled function is called with a sequence of many thousands of elements
- **THEN** it returns the correct result, the conversion cost being proportional to the size
