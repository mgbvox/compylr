## Purpose

Defines the array type a compiled program can accept: how its element storage and rank are
declared, what indexing and shape mean, and the guarantee that an array parameter is a view over
the caller's buffer rather than a copy of it.

## ADDED Requirements

### Requirement: An array type carries a declared storage and rank

An array type SHALL carry the storage of its elements and its rank, both declared in the source.
Rank SHALL be part of the type rather than discovered at runtime. An annotation that does not
declare both SHALL be rejected with a located diagnostic naming the accepted spelling.

#### Scenario: A ranked annotation is accepted

- **WHEN** lowering a parameter annotated as an array of a supported storage with a declared rank
- **THEN** lowering succeeds and the parameter's type carries both

#### Scenario: An unranked annotation is refused

- **WHEN** lowering a parameter annotated as an array without a declared rank
- **THEN** lowering fails with a located diagnostic naming the ranked spelling, for the reason a
  bare sequence annotation is refused: a rank that is not written down is not a type

#### Scenario: An unsupported storage is refused as planned

- **WHEN** lowering an array annotated with a storage outside the supported set
- **THEN** lowering fails with a located diagnostic reporting that storage as planned, distinct from
  reporting the annotation as unknown

#### Scenario: Two ranks are two types

- **WHEN** an array of rank one and an array of rank two over the same storage are compared
- **THEN** they are different types, and passing one where the other is declared is refused

### Requirement: Reading an element yields a scalar of the existing model

Reading an array element SHALL yield a value of the existing integer or floating-point type
according to the array's storage, so that the array type introduces no new scalar type and no new
integer width into the model.

#### Scenario: A floating-point element reads as a float

- **WHEN** an element of a floating-point array is read
- **THEN** its type is the model's floating-point type

#### Scenario: An integer element reads as an integer

- **WHEN** an element of an integer array is read
- **THEN** its type is the model's integer type

#### Scenario: An element participates in ordinary arithmetic

- **WHEN** an element read is combined with another numeric expression
- **THEN** the existing operator type rules and numeric promotion apply unchanged

### Requirement: Indexing names one element per rank

Indexing an array SHALL supply exactly one index per rank and SHALL yield an element. Supplying
fewer indices than the rank SHALL be refused, because the result would be a view that outlives the
expression. The declared index origin and checking mode SHALL apply as they do for a sequence.

#### Scenario: A rank-one array is indexed by one index

- **WHEN** an element of a rank-one array is read with a single index
- **THEN** the read succeeds and yields an element

#### Scenario: A rank-two array is indexed by two indices

- **WHEN** an element of a rank-two array is read with two indices in one subscript
- **THEN** the read succeeds and yields an element

#### Scenario: Partial indexing is refused

- **WHEN** a rank-two array is subscripted with a single index
- **THEN** lowering fails with a located diagnostic explaining that every index must be supplied,
  because a partial index would produce a view

#### Scenario: A negative index resolves by the declared origin

- **WHEN** an array is indexed with a negative offset
- **THEN** it resolves according to the declared index origin, as a sequence does

#### Scenario: An out-of-range index honours the declared checking mode

- **WHEN** an array is indexed outside its extent under a reported checking mode
- **THEN** the failure is recoverable and carries a located message, rather than aborting

### Requirement: An array parameter is a view over the caller's buffer

An array parameter SHALL be bound as a view over the memory the caller supplied, and SHALL NOT be
copied at the boundary. A write through a mutably bound array parameter SHALL be visible to the
caller after the call returns.

#### Scenario: No copy is made at the boundary

- **WHEN** a compiled function is called with an array of any size
- **THEN** the time taken before the body runs does not grow with the number of elements

#### Scenario: A write is visible to the caller

- **WHEN** a compiled function writes to an element of a mutably bound array parameter
- **THEN** the caller observes the new value in its own array after the call

#### Scenario: A read-only parameter does not permit writing

- **WHEN** a function only reads an array parameter
- **THEN** the parameter is bound as a shared view, and the emitted code cannot write through it

#### Scenario: A strided array stays a view

- **WHEN** a compiled function is called with a non-contiguous array, such as a strided slice
- **THEN** it is bound as a strided view and is still not copied

#### Scenario: The contrast with collections holds

- **WHEN** a function declares both a sequence parameter and an array parameter and mutates each
- **THEN** mutating the sequence parameter is refused as it is today, and mutating the array
  parameter is accepted and observed by the caller

### Requirement: Overlapping mutable array parameters are refused

Where a function takes more than one array parameter and at least one is mutably bound, the call
SHALL be refused when two of those parameters refer to overlapping memory.

#### Scenario: The same array passed twice is refused

- **WHEN** a compiled function taking two array parameters, one mutably bound, is called with the
  same array for both
- **THEN** the call raises an error naming the overlap, and no compiled code runs

#### Scenario: Overlapping views are refused

- **WHEN** two parameters are different views over overlapping regions of one buffer and one is
  mutably bound
- **THEN** the call raises an error naming the overlap

#### Scenario: Distinct arrays are accepted

- **WHEN** two array parameters refer to separate buffers
- **THEN** the call proceeds

#### Scenario: Shared-only parameters do not need the check

- **WHEN** every array parameter is bound as a shared view
- **THEN** passing the same array for several parameters is accepted

### Requirement: An array may not be returned or constructed

Returning an array, constructing one, or storing one SHALL be refused with a located diagnostic
naming the capability as not yet supported. A function taking array parameters MAY return a scalar.

#### Scenario: Returning an array is refused

- **WHEN** lowering a function declaring an array return type
- **THEN** lowering fails with a located diagnostic naming array creation as not yet supported

#### Scenario: Storing an array is refused

- **WHEN** lowering an assignment of an array parameter to an attribute or a collection
- **THEN** lowering fails, because the view would outlive the call

#### Scenario: A scalar return is accepted

- **WHEN** lowering a function that reduces an array parameter to a scalar
- **THEN** lowering succeeds

#### Scenario: In-place output is the supported idiom

- **WHEN** a function writes its results into a mutably bound array parameter
- **THEN** lowering succeeds and the caller observes the results
