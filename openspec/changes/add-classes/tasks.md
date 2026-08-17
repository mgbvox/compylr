## 1. Units hold classes

- [ ] 1.1 Write tests asserting a class can be added to a unit, that a name colliding with a function is refused in both directions, and that member order is content-determined
- [ ] 1.2 Write a test asserting a unit containing **no** classes fingerprints exactly as it did before — every cached build depends on it, per design.md D5
- [ ] 1.3 Write a test asserting changing a method body moves the unit fingerprint
- [ ] 1.4 Add a `Class` type and a second map on `Unit`, extending ordering, fingerprint, and validation over both
- [ ] 1.5 Update every consumer that iterates a unit's functions

## 2. Instance types

- [ ] 2.1 Write tests asserting a class name is a type, that two classes are distinct types, and that instances nest in collections
- [ ] 2.2 Write tests asserting an instance cannot be a mapping key or set element, and is not trivially copyable
- [ ] 2.3 Add `Ty::Instance`, documenting at the definition that it is the model's one **nominal** type, per design.md D1
- [ ] 2.4 Write round-trip tests covering classes, instance types, attribute access, assignment, and construction

## 3. IR forms

- [ ] 3.1 Write tests asserting attribute read, attribute assignment, and construction are representable and that construction is distinct from a call
- [ ] 3.2 Write a test asserting `walk_calls` descends into attribute objects, construction arguments, and method-call receivers and arguments
- [ ] 3.3 Add the new expression and statement forms, extend `walk_calls`, and extend serialization

## 4. Class definitions

- [ ] 4.1 Write tests asserting a class with `__init__` and a method lowers, and that a class without `__init__` is rejected
- [ ] 4.2 Write tests asserting `self` is required, must not be annotated, and that method parameters and returns stay mandatory
- [ ] 4.3 Write tests asserting inheritance, a class-level statement, a dunder other than `__init__`, and a duplicate method are each rejected naming what was found
- [ ] 4.4 Implement class-definition lowering

## 5. Attributes

- [ ] 5.1 Write tests asserting an annotated assignment in `__init__` declares a typed attribute, including a collection attribute
- [ ] 5.2 Write tests asserting an undeclared attribute, an unannotated declaration, and a declaration outside `__init__` are each rejected
- [ ] 5.3 Write a test asserting the undeclared-attribute diagnostic says where to declare it, per design.md D2 — a refusal that does not say where leaves the user guessing
- [ ] 5.4 Write a test asserting a declared attribute must be initialised
- [ ] 5.5 Write tests for attribute read and assignment typing, promotion, an unknown attribute, and reading from another object
- [ ] 5.6 Write a test asserting a collection **attribute** may be mutated, unlike a collection parameter
- [ ] 5.7 Implement attribute declaration, access, and assignment

## 6. Methods and construction

- [ ] 6.1 Write tests asserting a method call is typed from its signature, with arity and argument checks and promotion
- [ ] 6.2 Write tests asserting an unknown method is rejected naming it and the class, and that a method may call another on the same object
- [ ] 6.3 Write tests asserting construction is typed as the instance type and its arguments are checked against `__init__`
- [ ] 6.4 Write a test asserting a class in another source leaves construction undetermined and resolves at the unit, matching how functions behave
- [ ] 6.5 Implement method-call and construction lowering

## 7. Backend: structs and receivers

- [ ] 7.1 Write tests asserting attributes become fields in declaration order, methods become one implementation block, and `__init__` becomes a constructor
- [ ] 7.2 Write a test asserting a mutating method compiles and a reading method takes a shared receiver
- [ ] 7.3 Write a test asserting a method that **calls** a mutating method also takes a mutable receiver — the transitive case, and the likeliest bug, per design.md D3
- [ ] 7.4 Write a test asserting reading and mutating compose in one method
- [ ] 7.5 Implement the mutable-receiver fixpoint and struct emission

## 8. Backend: access, construction, and persistence

- [ ] 8.1 Write executable tests asserting an attribute read yields its value and an assignment is observed by a later call
- [ ] 8.2 Write a test asserting a collection attribute read twice compiles, so reading a field does not move it
- [ ] 8.3 Write an executable test asserting construction initialises every field
- [ ] 8.4 Write an executable test asserting a mutation is observed by a second call — the property the change exists for
- [ ] 8.5 Implement attribute and construction emission

## 9. Bindings

- [ ] 9.1 Write tests asserting the type is exposed, constructible, and its methods callable
- [ ] 9.2 Write tests asserting arguments convert as they do for functions, wrong types raise `TypeError`, and failures raise the same exceptions
- [ ] 9.3 Write tests asserting instance state persists across calls, two instances are independent, and a cache hits on the second call
- [ ] 9.4 Write a test asserting an instance stored by the caller keeps its state
- [ ] 9.5 Emit `#[pyclass]` and `#[pymethods]` per design.md D4

## 10. The decorator

- [ ] 10.1 Write tests asserting the decorator accepts a class in both forms and validates it when marked
- [ ] 10.2 Write tests asserting identity attributes are preserved and the original class is reachable
- [ ] 10.3 Write a test asserting instantiating a marked class builds the project, and that classes and functions share one build
- [ ] 10.4 Implement class support in the manager

## 11. Fixtures and end to end

- [ ] 11.1 Add accepted fixtures for a counter class, a class holding a collection attribute, and a method calling another method
- [ ] 11.2 Add rejected fixtures for inheritance, a missing `__init__`, an undeclared attribute, an unannotated attribute, and a method without `self`
- [ ] 11.3 Update the rejection table and fixture-count guard
- [ ] 11.4 Write a pytest building a memoized class end to end and asserting the second call hits the cache — the demo's third variant, proven before the demo depends on it

## 12. Verification

- [ ] 12.1 Run `cargo fmt`, `cargo clippy -p compylr --all-targets -- -D warnings`, and `cargo test` twice
- [ ] 12.2 Run `pytest`, `ruff check python/`, and `mypy python/compylr`; coverage with the venv deactivated
- [ ] 12.3 Confirm Rust coverage over `src/` still exceeds 80%
- [ ] 12.4 Update the README and `CLAUDE.md`, including that instance state persists while collection parameters are copies — the contrast is the thing people will get wrong
- [ ] 12.5 Run `openspec validate add-classes --strict` and confirm every scenario in all five delta specs has a passing test
