## Purpose

The surface a user actually touches: creating a configured manager, marking functions for
compilation with a decorator, resolving global against per-function settings, and having calls
land on compiled code without the calling code changing.

## Requirements

### Requirement: Initialization produces a configured manager

The package SHALL provide an initialization entry point that returns a manager carrying the
project's default settings. It SHALL accept the target backend, the assist mode, and the default
behavior, and all SHALL have defaults so that initialization with no arguments is valid.

#### Scenario: Explicit configuration

- **WHEN** initialization is called with a backend, assist mode, and behavior
- **THEN** a manager carrying those settings is returned

#### Scenario: Defaults

- **WHEN** initialization is called with no arguments
- **THEN** a manager is returned whose backend is the implemented default, whose assist mode
  is disabled, and whose behavior is the source language's on every axis

#### Scenario: One manager per project

- **WHEN** initialization is called a second time with the same settings
- **THEN** the same manager is returned, preserving the one-shared-artifact invariant

#### Scenario: Conflicting reconfiguration is refused

- **WHEN** initialization is called a second time with different settings
- **THEN** an error is raised naming the conflicting setting, rather than silently changing the
  defaults of a project that is already partly configured

#### Scenario: A differing default behavior is a conflicting reconfiguration

- **WHEN** initialization is called a second time with the same backend but a different behavior
- **THEN** an error is raised, because members marked before the change would otherwise compile
  under a behavior their author never chose

### Requirement: Both decorator forms are supported

The manager SHALL provide a decorator usable bare, applied directly to a function, and called
with settings before being applied. Both SHALL mark the function for compilation identically
apart from the settings in effect.

#### Scenario: Bare form

- **WHEN** the decorator is applied directly to a supported function
- **THEN** the function is marked for compilation under the manager's settings

#### Scenario: Called form with no arguments

- **WHEN** the decorator is called with no arguments and then applied
- **THEN** the result is identical to the bare form

#### Scenario: Called form with settings

- **WHEN** the decorator is called with settings and then applied
- **THEN** the function is marked for compilation under those settings

### Requirement: Per-function settings override the manager's

A setting given on the decorator SHALL apply to that function only. A setting not given SHALL
be inherited from the manager. Overriding SHALL NOT alter the manager's defaults for any other
function. Where a setting is itself composite — as a behavior is — the parts it does not name
SHALL be inherited individually rather than the whole setting being replaced.

#### Scenario: Override applies to one function only

- **WHEN** one function overrides the backend and another uses the bare decorator
- **THEN** the first uses the override and the second uses the manager's backend

#### Scenario: Unspecified settings are inherited

- **WHEN** the decorator is called specifying only the assist mode
- **THEN** the backend and the behavior are inherited from the manager

#### Scenario: An unnamed axis is inherited, not reset

- **WHEN** the decorator is called with a behavior object naming one axis
- **THEN** the remaining axes keep the manager's stances rather than reverting to any default

### Requirement: Requesting an unimplemented target fails clearly

A backend that is reserved but not implemented SHALL be accepted as a valid name and SHALL fail
with an error saying it is not implemented yet. A name not in the registry SHALL fail with an
error naming the available backends. Both SHALL be reported when the function is marked, not at
first call, so that the failure points at the decorator that caused it.

#### Scenario: Reserved backend

- **WHEN** a function is marked with a reserved but unimplemented backend
- **THEN** an error is raised stating that the backend is not implemented yet

#### Scenario: Unknown backend

- **WHEN** a function is marked with a backend name that is not in the registry
- **THEN** an error is raised naming the available backends

### Requirement: The assist mode is declared but not implemented

The API SHALL accept the assist mode setting so that the surface does not change when it is
implemented, and SHALL raise a clear error when it is enabled. Disabling it SHALL be the
default and SHALL be accepted silently.

#### Scenario: Enabled globally

- **WHEN** initialization enables the assist mode
- **THEN** an error is raised stating it is not implemented yet

#### Scenario: Enabled for one function

- **WHEN** the decorator enables the assist mode for a single function
- **THEN** an error is raised stating it is not implemented yet

#### Scenario: Disabled is accepted

- **WHEN** the assist mode is disabled or omitted
- **THEN** no error is raised

### Requirement: Unsupported functions are rejected when marked

A function outside the supported subset SHALL be rejected at the point it is marked, carrying
the diagnostic and its `line:column`. It SHALL NOT be accepted and then fail at first call, and
it SHALL NOT silently fall back to interpreted execution: the user asked for compilation, so
being told immediately that it cannot happen is the useful outcome.

**One category is deferred.** A binding whose initializer calls a function the marked source does
not define cannot be typed at this point, because each marked function is captured as its own
source and its callees live in others. Rejecting it here would demand an annotation for
`doubled = double(n)` in exactly the arrangement this API always produces. That single category
SHALL be deferred to the build, where every source is present and it can be typed — and SHALL be
reported there if it still cannot be.

Every other violation SHALL still be reported when the function is marked.

#### Scenario: Missing annotation

- **WHEN** a function with an unannotated parameter is marked
- **THEN** an error is raised naming the parameter and its location

#### Scenario: Unsupported construct

- **WHEN** a function containing a loop is marked
- **THEN** an error is raised naming the unsupported construct and its location

#### Scenario: Failure is immediate

- **WHEN** an unsupported function is marked
- **THEN** the error is raised at that point, before the function is ever called

#### Scenario: A call to another marked function needs no annotation

- **WHEN** a function is marked whose body binds the result of calling another marked function,
  without an annotation
- **THEN** marking succeeds, and the binding is typed when the project is built

#### Scenario: Marking order does not matter

- **WHEN** the calling function is marked before the function it calls
- **THEN** both are accepted, since the check that needs both is deferred to the build

#### Scenario: Only that category is deferred

- **WHEN** a marked function contains any other violation
- **THEN** it is still reported at the point of marking

#### Scenario: A callee that is never marked is still reported

- **WHEN** a deferred binding's callee is never marked and the project is built
- **THEN** the build fails, since deferring a check is not the same as skipping it

#### Scenario: No silent fallback

- **WHEN** a function is rejected
- **THEN** it is not left silently interpreted

### Requirement: Source is captured from the live function

The manager SHALL obtain each marked function's source by introspecting the function object,
so that marking works regardless of the file layout. Indentation from the surrounding context
SHALL NOT cause a rejection.

#### Scenario: Source obtained by introspection

- **WHEN** a function defined in a module is marked
- **THEN** its source is captured without the user supplying a path

#### Scenario: Decorator line is not part of the compiled source

- **WHEN** a marked function's source is captured
- **THEN** the decorator line itself is not treated as part of the function to compile

### Requirement: Calls reach the compiled implementation

Once the artifact is built, calling a marked function SHALL execute the compiled
implementation. The build SHALL be triggered once for the whole project rather than once per
function, so that marking N functions does not cost N builds.

#### Scenario: A call returns the compiled result

- **WHEN** a marked function is called
- **THEN** the compiled implementation runs and returns its result

#### Scenario: One build for many functions

- **WHEN** several functions are marked and one of them is called
- **THEN** a single build covering all of them is performed

#### Scenario: Later calls do not rebuild

- **WHEN** a marked function is called repeatedly
- **THEN** the build happens at most once per process

#### Scenario: Results match the interpreted function

- **WHEN** a marked function is called with arguments inside the supported ranges
- **THEN** the result equals what the original Python function would have returned

### Requirement: Marked functions remain ordinary Python objects

A marked function SHALL keep the identifying attributes callers and tooling depend on — its
name, its docstring, its module, and its annotations — and SHALL expose the original
uncompiled function, so that a compiled function can still be introspected, documented, and
compared against its source implementation.

#### Scenario: Identity attributes are preserved

- **WHEN** a marked function's name, docstring, module, and annotations are read
- **THEN** they match those of the function as written

#### Scenario: The original function is reachable

- **WHEN** a caller needs the uncompiled implementation
- **THEN** it is accessible from the marked function

#### Scenario: Usable as a normal callable

- **WHEN** a marked function is passed to code that accepts any callable
- **THEN** it behaves as a callable

### Requirement: The decorator accepts a class

The manager's decorator SHALL accept a class as well as a function, marking it for compilation
under the same settings and validating it at the point it is marked.

A marked class SHALL keep the identifying attributes callers and tooling read — its name,
docstring, module, and annotations — and SHALL expose the original uncompiled class, so compiled
and interpreted behaviour can be compared.

#### Scenario: A class is marked

- **WHEN** the decorator is applied to a supported class
- **THEN** the class is marked for compilation under the manager's settings

#### Scenario: Both decorator forms work on a class

- **WHEN** the decorator is applied to a class bare and called with settings
- **THEN** both mark it, differing only in the settings in effect

#### Scenario: An unsupported class is rejected when marked

- **WHEN** a class declaring a base is marked
- **THEN** an error is raised naming the unsupported construct and its location

#### Scenario: Identity attributes are preserved

- **WHEN** a marked class's name, docstring, and module are read
- **THEN** they match those of the class as written

#### Scenario: The original class is reachable

- **WHEN** a caller needs the uncompiled implementation
- **THEN** it is accessible from the marked class

#### Scenario: Instantiating a marked class builds the project

- **WHEN** a marked class is instantiated for the first time
- **THEN** the project is built and the compiled type is used, as calling a marked function does

#### Scenario: Classes and functions share one build

- **WHEN** a project marks both classes and functions
- **THEN** one build covers all of them

### Requirement: A project can be compiled programmatically

The package SHALL expose an entry point that takes a project root, discovers everything marked
beneath it, builds once, and reports what it found and did.

The command-line form SHALL be a thin wrapper over it. Anything the command decides that the
programmatic form does not is a place the two can disagree, and a user debugging a precompile
should not have to work out which one they are looking at.

#### Scenario: A root is compiled programmatically

- **WHEN** the entry point is called with a project root containing marked functions
- **THEN** the artifact is built and a report is returned

#### Scenario: The report names what was found

- **WHEN** the entry point returns
- **THEN** the report carries the modules imported, the functions and classes found, and whether a
  build occurred

#### Scenario: An empty project is not an error

- **WHEN** the root contains nothing marked
- **THEN** the report says so and no build is attempted

#### Scenario: Import failures are reported rather than raised

- **WHEN** one module cannot be imported
- **THEN** the report carries the failure and the others are still processed

#### Scenario: A build failure raises

- **WHEN** the toolchain fails
- **THEN** the same error is raised as when a call triggers the build, carrying the toolchain output

#### Scenario: The command adds no behaviour of its own

- **WHEN** the same root is compiled through the command and through the entry point
- **THEN** both produce the same artifact and the same outcome

### Requirement: Behavior is settable globally and per member

The API SHALL accept a `behavior` setting on initialization and on the decorator. Set on
initialization it SHALL be the project's default for every marked member; set on the decorator it
SHALL apply to that member alone.

`behavior` SHALL accept either a language name — meaning that language's stance on every axis — or
a behavior object naming a language for some axes and leaving the rest to be inherited. A language
name SHALL be exactly equivalent to a behavior object naming that language for every axis.

The default SHALL be the source language, so a project that never mentions `behavior` compiles
exactly as it did before the setting existed.

#### Scenario: A global behavior applies to every member

- **WHEN** initialization sets the behavior to the target language and two functions are marked
  with the bare decorator
- **THEN** both compile under the target language's stance on every axis

#### Scenario: A per-member behavior overrides the global

- **WHEN** initialization sets the behavior to the source language and one function is marked with
  the target language
- **THEN** that function compiles under the target's stance and every other member under the
  source's

#### Scenario: A behavior object inherits the axes it does not name

- **WHEN** a member is marked with a behavior object naming one axis, under a global default of the
  source language
- **THEN** that axis takes the named language's stance and every other axis takes the source
  language's

#### Scenario: A behavior object inherits from a non-default global

- **WHEN** the global behavior is the target language and a member names one axis as the source
  language
- **THEN** that axis takes the source language's stance and every other axis takes the target's

#### Scenario: The two spellings are equivalent

- **WHEN** one function is marked with the target language's name and another with a behavior
  object naming the target for every axis
- **THEN** the two compile to identical code

#### Scenario: Omitting behavior changes nothing

- **WHEN** a project marks members without mentioning behavior anywhere
- **THEN** the generated code is identical to what the same project produced before the setting
  existed

### Requirement: An invalid behavior is rejected where it was written

A behavior naming anything other than the source or the target language of the compilation SHALL be
rejected when the member is marked, or when initialization is called, rather than at a later build.
The error SHALL name the two languages that would have been accepted.

The message SHALL distinguish a name compylr does not know at all from a name that is a registered
or reserved language but is not one of the two in this compilation. An axis name that does not exist
SHALL likewise be rejected, with the valid axis names listed.

#### Scenario: An unknown language is rejected at the decorator

- **WHEN** a function is marked with a behavior naming a language compylr has no component for
- **THEN** an error is raised as the decorator runs, naming the two languages that would have been
  accepted

#### Scenario: A reserved language is rejected distinctly

- **WHEN** a function in a Python-to-Rust project is marked with a behavior naming a language
  compylr has reserved but which is neither Python nor Rust
- **THEN** an error is raised whose message distinguishes it from an unknown name

#### Scenario: An invalid global behavior is rejected at initialization

- **WHEN** initialization is called with a behavior naming a language that is neither the source nor
  the target
- **THEN** an error is raised before any member is marked

#### Scenario: An unknown axis is rejected

- **WHEN** a behavior object is constructed naming an axis that does not exist
- **THEN** an error is raised listing the axes that do

#### Scenario: A per-axis value is validated like a bare name

- **WHEN** a behavior object names a valid axis with an invalid language
- **THEN** an error is raised naming both the axis and the two languages that would have been
  accepted

### Requirement: Members of one project may have different behaviors

Two members of the same project MAY be marked with different behaviors and SHALL compile into the
same shared artifact. This SHALL NOT be refused the way a mixed backend is: a backend decides what
artifact is produced, while a behavior decides what individual operations mean, and operations of
different meanings coexist in one artifact.

A member under one behavior calling a member under another SHALL work, and each SHALL keep its own
meanings.

#### Scenario: Mixed behavior builds one artifact

- **WHEN** one function is marked with the source language's behavior and another with the target's
- **THEN** both are built into the same artifact and both are callable

#### Scenario: A mixed-behavior call keeps each side's meaning

- **WHEN** a function under the source language's behavior calls one under the target's, and both
  compute a floor division of a negative dividend
- **THEN** the caller's result follows the source language's rounding and the callee's follows the
  target's

#### Scenario: A mixed backend is still refused

- **WHEN** two members of one project are marked with different backends
- **THEN** the build is still refused, because a project compiles to one shared artifact

### Requirement: A behavior change rebuilds

Changing a member's behavior SHALL cause the project to rebuild on its next run, without the user
clearing a cache. Behavior determines what the program computes, so it SHALL be part of what the
rebuild key distinguishes.

#### Scenario: Changing a behavior rebuilds

- **WHEN** a project is built, a member's behavior is then changed, and the project is run again
- **THEN** the toolchain runs again and the new behavior is what executes

#### Scenario: An unchanged behavior does not rebuild

- **WHEN** a project is built and run again with nothing changed
- **THEN** the cached artifact is reused
