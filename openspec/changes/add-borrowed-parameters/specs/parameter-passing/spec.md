## Purpose

Defines what it means for a parameter to be passed owned or borrowed, how the compiler decides
which from the function's body, and how far a borrowed value may travel before ownership is
required.

## ADDED Requirements

### Requirement: A parameter is owned unless a borrow is proven safe

Every parameter SHALL carry a passing mode of owned, shared borrow, or mutable borrow. Owned SHALL
be the default and the result whenever the analysis cannot prove a borrow safe. A parameter that
cannot be borrowed SHALL NOT produce a diagnostic, because the program is correct either way.

#### Scenario: An unanalyzable body yields ownership

- **WHEN** a parameter's use cannot be proven not to escape
- **THEN** the parameter is owned, and compilation succeeds with no diagnostic

#### Scenario: A read-only parameter is borrowed

- **WHEN** a parameter is only read within the call
- **THEN** it carries a shared borrow

#### Scenario: Ownership is never requested by the user

- **WHEN** any supported source program is compiled
- **THEN** no syntax expresses a passing mode, and no diagnostic mentions one

#### Scenario: The mode does not change what the program computes

- **WHEN** the same program is compiled with every parameter forced owned and again with modes
  inferred
- **THEN** both produce identical answers for every call

### Requirement: A parameter that escapes the call is owned

A parameter SHALL be owned whenever the body lets its value outlive the call: returning it, storing
it in a collection, an attribute, or any binding that outlives the call, appending it to a
sequence, using it as a mapping key or value that is retained, or passing it to anything that
requires ownership.

#### Scenario: A returned parameter is owned

- **WHEN** a function returns its parameter
- **THEN** the parameter is owned

#### Scenario: An appended parameter is owned

- **WHEN** a function appends its parameter to a sequence
- **THEN** the parameter is owned

#### Scenario: A parameter stored under a mapping key is owned

- **WHEN** a function assigns its parameter as a mapping value
- **THEN** the parameter is owned

#### Scenario: A parameter stored in an attribute is owned

- **WHEN** a method assigns its parameter to an attribute of the receiver
- **THEN** the parameter is owned

#### Scenario: A parameter compared across representations is owned

- **WHEN** a function compares its text parameter with a text literal using an ordering comparison
- **THEN** the parameter is owned, because the comparison is not available between the borrowed and
  owned representations

#### Scenario: A parameter tested for membership is owned

- **WHEN** a function tests whether a sequence contains its parameter
- **THEN** the parameter is owned

#### Scenario: A parameter passed to a function needing ownership is owned

- **WHEN** a function passes its parameter to another function whose corresponding parameter is
  owned
- **THEN** the caller's parameter is also owned

### Requirement: Ownership and mutability are decided by one analysis

The decision of whether a parameter is borrowed SHALL be made by the same fixpoint that decides
whether a receiver or an instance parameter is mutable, and SHALL reach the same conclusion for a
given program on every run.

#### Scenario: One analysis decides both

- **WHEN** a function both mutates an instance parameter and forwards a text parameter
- **THEN** the mutability of the first and the ownership of the second are decided together, and
  neither is re-derived separately

#### Scenario: Transitive requirements propagate

- **WHEN** a function passes a parameter to a function that stores it, which is only discovered
  after that callee is analyzed
- **THEN** the fixpoint re-runs and the caller's parameter becomes owned

#### Scenario: Mutual recursion terminates

- **WHEN** two functions pass parameters to one another
- **THEN** the analysis terminates and produces the same modes regardless of which was analyzed
  first

#### Scenario: The result does not depend on declaration order

- **WHEN** the same functions are analyzed in a different order
- **THEN** every parameter receives the same mode

### Requirement: A borrow does not reach beyond the call

A borrowed value MAY be read, mutated where its mode permits, and forwarded to a parameter that
borrows it compatibly. It SHALL NOT be returned as an owned value, stored, or forwarded to a
parameter requiring ownership. Where a program requires any of those, the parameter is owned rather
than the program refused.

#### Scenario: A borrowed value may be forwarded compatibly

- **WHEN** a function passes a borrowed parameter to another function that also borrows it
- **THEN** both remain borrowed and no copy is made

#### Scenario: Reading a borrowed value does not copy it

- **WHEN** a borrowed parameter is read several times in a body
- **THEN** no copy is made for any read

#### Scenario: The existing instance rules are unchanged

- **WHEN** a program tries to return a borrowed instance, or a field of one
- **THEN** it is refused by the existing diagnostics, which this change does not relax
