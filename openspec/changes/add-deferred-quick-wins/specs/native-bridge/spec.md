## MODIFIED Requirements

### Requirement: Sources are assembled into one unit

The bridge SHALL combine every supplied source into a single compilation unit before emitting,
so that a call from a function in one source to a function in another resolves. Resolution
SHALL NOT depend on the order the sources are supplied.

Signatures SHALL be gathered from **every** source before any body is lowered, so that a call
across sources is typed rather than left undetermined. This is not an optimisation: the decorator
captures each function as its own source, so a call between two decorated functions is always a
cross-source call, and without this the inference the compiler offers would work everywhere except
through its primary interface.

#### Scenario: Call across two sources

- **WHEN** two sources are compiled together and a function in the first calls a function in
  the second
- **THEN** compilation succeeds

#### Scenario: A cross-source call is typed

- **WHEN** a binding in one source is initialised by calling a function defined in another
- **THEN** the binding takes the callee's return type and needs no annotation

#### Scenario: Order independence

- **WHEN** the same two sources are compiled in both orders
- **THEN** both succeed and report the same fingerprint

#### Scenario: A callee in no source is still reported

- **WHEN** every source has been supplied and a binding's initializer still cannot be typed
- **THEN** compilation fails, since deferring a check is not the same as skipping it

#### Scenario: Duplicate function names across sources

- **WHEN** two sources each define a function of the same name
- **THEN** compilation fails reporting the conflicting name

## ADDED Requirements

### Requirement: Failures carry a machine-readable category

A compilation failure SHALL carry a stable identifier for what kind of rule was broken, alongside
its message and location. Callers that act differently on different failures SHALL be able to
branch on that identifier.

The identifier SHALL be distinct from the human-readable message. A caller matching on message
text is broken by any rewording, which makes the message unimprovable — and one caller, the
decorator, needs to recognise exactly one category in order to defer it, without recognising any
other.

#### Scenario: A subset violation reports its category

- **WHEN** a program is rejected for an unsupported construct
- **THEN** the failure carries an identifier naming that category

#### Scenario: Categories are distinguishable

- **WHEN** two programs are rejected for different reasons
- **THEN** their identifiers differ

#### Scenario: The identifier is not the message

- **WHEN** a failure's identifier and message are compared
- **THEN** the identifier is a stable token rather than the prose shown to a user

#### Scenario: A binding that cannot yet be typed has its own category

- **WHEN** a binding's initializer calls a function the supplied sources do not define
- **THEN** the failure's category distinguishes it from an annotation the user simply omitted,
  because one may become resolvable with more sources and the other never will

#### Scenario: A syntax error needs no category

- **WHEN** a source fails to parse
- **THEN** the failure is identifiable as a syntax error without carrying a subset category
