//! Lowering class definitions, attributes, methods, and construction.
//!
//! The rule that will surprise people is that every attribute must be declared, with an annotation,
//! in `__init__`. Python lets an attribute appear anywhere; without this rule a struct's fields
//! would depend on which methods happened to be called. It is the same rule the subset already
//! applies to parameters and returns, and the diagnostic has to say *where* to declare it — a
//! refusal that only says the attribute is unknown leaves the user guessing.
//!
//! The other contrast worth holding onto: a collection **attribute** may be mutated, where a
//! collection **parameter** may not. An instance is not converted at the boundary — Python holds
//! the Rust value itself — so a mutated attribute is exactly what the caller does observe.

use compylr_diagnostics::error::LowerErrorKind;
use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source_members;
use compylr_ir::{Class, Stmt, Ty};

fn classes(source: &str) -> Vec<Class> {
    let parsed = parse_source(source).expect("fixture must parse");
    lower_source_members(&parsed, python_stance())
        .unwrap_or_else(|e| panic!("should lower: {}", e.render(source)))
        .1
}

fn one_class(source: &str) -> Class {
    let mut found = classes(source);
    assert_eq!(found.len(), 1, "expected exactly one class");
    found.remove(0)
}

fn accepts(source: &str) {
    let _ = classes(source);
}

fn reject(source: &str) -> LowerErrorKind {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source_members(&parsed, python_stance()) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.kind(),
    }
}

fn message(source: &str) -> String {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source_members(&parsed, python_stance()) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.to_string(),
    }
}

const COUNTER: &str = "class Counter:\n\
    \x20   def __init__(self) -> None:\n\
    \x20       self.count: int = 0\n\
    \n\
    \x20   def bump(self, by: int) -> None:\n\
    \x20       self.count = self.count + by\n\
    \n\
    \x20   def get(self) -> int:\n\
    \x20       return self.count\n";

// ---------------------------------------------------------------------------
// Class definitions
// ---------------------------------------------------------------------------

#[test]
fn a_class_lowers_with_its_attributes_and_methods() {
    let class = one_class(COUNTER);
    assert_eq!(class.name, "Counter");
    assert_eq!(class.attributes.len(), 1);
    assert_eq!(class.attributes[0].name, "count");
    assert_eq!(class.attributes[0].ty, Ty::Int);
    assert_eq!(class.methods.len(), 2, "__init__ is not among the methods");
    assert!(class.methods.contains_key("bump"));
    assert!(class.methods.contains_key("get"));
}

#[test]
fn a_class_without_init_is_rejected() {
    // Without it there is nowhere for attributes to be declared, so the struct would have no
    // defined shape.
    assert_eq!(
        reject("class C:\n    def get(self) -> int:\n        return 1\n"),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn a_method_must_take_self() {
    let text = message(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         \x20   def get() -> int:\n\
         \x20       return 1\n",
    );
    assert!(
        text.contains("self"),
        "diagnostic should say the method needs self, got: {text}"
    );
}

#[test]
fn self_must_not_be_annotated() {
    // Its type is the class, and letting it be written invites it being written differently.
    assert_eq!(
        reject(
            "class C:\n\
             \x20   def __init__(self: int) -> None:\n\
             \x20       self.x: int = 0\n"
        ),
        LowerErrorKind::UnsupportedConstruct
    );
}

#[test]
fn method_parameters_and_returns_stay_mandatory() {
    assert_eq!(
        reject(
            "class C:\n\
             \x20   def __init__(self) -> None:\n\
             \x20       self.x: int = 0\n\
             \n\
             \x20   def get(self, n) -> int:\n\
             \x20       return n\n"
        ),
        LowerErrorKind::MissingAnnotation
    );
    assert_eq!(
        reject(
            "class C:\n\
             \x20   def __init__(self) -> None:\n\
             \x20       self.x: int = 0\n\
             \n\
             \x20   def get(self):\n\
             \x20       return 1\n"
        ),
        LowerErrorKind::MissingAnnotation
    );
}

#[test]
fn inheritance_is_rejected() {
    let text = message(
        "class C(Base):\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n",
    );
    assert!(
        text.contains("inherit") || text.contains("base"),
        "diagnostic should name what was found, got: {text}"
    );
}

#[test]
fn a_class_level_statement_is_rejected() {
    // A class attribute is shared state with no instance, which is a different thing entirely.
    let text = message(
        "class C:\n\
         \x20   shared: int = 0\n\
         \n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n",
    );
    assert!(
        !text.is_empty(),
        "a class body may only contain method definitions"
    );
}

#[test]
fn a_dunder_other_than_init_is_rejected() {
    let text = message(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         \x20   def __repr__(self) -> str:\n\
         \x20       return \"C\"\n",
    );
    assert!(
        text.contains("__repr__"),
        "diagnostic should name the method, got: {text}"
    );
}

#[test]
fn two_methods_of_the_same_name_are_rejected() {
    assert_eq!(
        reject(
            "class C:\n\
             \x20   def __init__(self) -> None:\n\
             \x20       self.x: int = 0\n\
             \n\
             \x20   def get(self) -> int:\n\
             \x20       return 1\n\
             \n\
             \x20   def get(self) -> int:\n\
             \x20       return 2\n"
        ),
        LowerErrorKind::DuplicateFunction
    );
}

// ---------------------------------------------------------------------------
// Attributes
// ---------------------------------------------------------------------------

#[test]
fn an_attribute_may_hold_a_collection() {
    let class = one_class(
        "class Cache:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.entries: dict[int, int] = {}\n",
    );
    assert_eq!(
        class.attributes[0].ty,
        Ty::Dict(Box::new(Ty::Int), Box::new(Ty::Int))
    );
}

#[test]
fn an_undeclared_attribute_is_rejected_and_says_where_to_declare_it() {
    // The rule is stricter than Python in a way users notice, so a refusal that only says the
    // attribute is unknown leaves them guessing at the fix.
    let text = message(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         \x20   def set(self, n: int) -> None:\n\
         \x20       self.y = n\n",
    );
    assert!(
        text.contains('y'),
        "diagnostic should name the attribute, got: {text}"
    );
    assert!(
        text.contains("__init__"),
        "diagnostic should say where to declare it, got: {text}"
    );
}

#[test]
fn an_unannotated_declaration_is_rejected() {
    assert_eq!(
        reject(
            "class C:\n\
             \x20   def __init__(self) -> None:\n\
             \x20       self.x = 0\n"
        ),
        LowerErrorKind::MissingAnnotation
    );
}

#[test]
fn a_declaration_outside_init_is_rejected() {
    // Otherwise the struct's fields would depend on which methods happened to be called.
    let text = message(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         \x20   def late(self) -> None:\n\
         \x20       self.y: int = 1\n",
    );
    assert!(
        text.contains("__init__"),
        "diagnostic should point at __init__, got: {text}"
    );
}

// ---------------------------------------------------------------------------
// Access and assignment
// ---------------------------------------------------------------------------

#[test]
fn an_attribute_read_is_typed() {
    let class = one_class(COUNTER);
    let get = &class.methods["get"];
    assert_eq!(get.ret, Ty::Int);
}

#[test]
fn an_attribute_is_assigned() {
    let class = one_class(COUNTER);
    let bump = &class.methods["bump"];
    assert!(matches!(bump.body[0], Stmt::SetAttr { ty: Ty::Int, .. }));
}

#[test]
fn a_wrong_attribute_type_is_rejected() {
    let text = message(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         \x20   def set(self, s: str) -> None:\n\
         \x20       self.x = s\n",
    );
    assert!(
        text.contains("int") && text.contains("str"),
        "diagnostic should report both types, got: {text}"
    );
}

#[test]
fn an_unknown_attribute_read_is_rejected() {
    assert_eq!(
        reject(
            "class C:\n\
             \x20   def __init__(self) -> None:\n\
             \x20       self.x: int = 0\n\
             \n\
             \x20   def get(self) -> int:\n\
             \x20       return self.missing\n"
        ),
        LowerErrorKind::Unresolved
    );
}

#[test]
fn an_attribute_is_read_from_another_object() {
    accepts(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         def peek(c: C) -> int:\n\
         \x20   return c.x\n",
    );
}

#[test]
fn a_collection_attribute_may_be_mutated() {
    // The contrast that matters: a collection *parameter* may not be, because it is a copy. An
    // instance is not converted at the boundary, so a mutated attribute is what the caller sees.
    accepts(
        "class Cache:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.entries: dict[int, int] = {}\n\
         \n\
         \x20   def put(self, k: int, v: int) -> None:\n\
         \x20       self.entries[k] = v\n\
         \n\
         \x20   def has(self, k: int) -> bool:\n\
         \x20       return k in self.entries\n",
    );
}

// ---------------------------------------------------------------------------
// Methods and construction
// ---------------------------------------------------------------------------

#[test]
fn construction_and_method_calls_are_typed() {
    accepts(
        "class Counter:\n\
         \x20   def __init__(self, start: int) -> None:\n\
         \x20       self.count: int = start\n\
         \n\
         \x20   def get(self) -> int:\n\
         \x20       return self.count\n\
         \n\
         def use() -> int:\n\
         \x20   c = Counter(1)\n\
         \x20   return c.get()\n",
    );
}

#[test]
fn constructor_arguments_are_checked() {
    assert_eq!(
        reject(
            "class Counter:\n\
             \x20   def __init__(self, start: int) -> None:\n\
             \x20       self.count: int = start\n\
             \n\
             def use() -> int:\n\
             \x20   c = Counter(\"a\")\n\
             \x20   return 1\n"
        ),
        LowerErrorKind::TypeMismatch
    );
    assert_eq!(
        reject(
            "class Counter:\n\
             \x20   def __init__(self, start: int) -> None:\n\
             \x20       self.count: int = start\n\
             \n\
             def use() -> int:\n\
             \x20   c = Counter()\n\
             \x20   return 1\n"
        ),
        LowerErrorKind::ArityMismatch
    );
}

#[test]
fn method_arity_is_checked() {
    assert_eq!(
        reject(
            "class C:\n\
             \x20   def __init__(self) -> None:\n\
             \x20       self.x: int = 0\n\
             \n\
             \x20   def get(self) -> int:\n\
             \x20       return self.x\n\
             \n\
             def use(c: C) -> int:\n\
             \x20   return c.get(1)\n"
        ),
        LowerErrorKind::ArityMismatch
    );
}

#[test]
fn an_unknown_method_is_rejected_naming_it_and_the_class() {
    let text = message(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         def use(c: C) -> int:\n\
         \x20   return c.missing()\n",
    );
    assert!(
        text.contains("missing") && text.contains('C'),
        "diagnostic should name the method and the class, got: {text}"
    );
}

#[test]
fn a_method_may_call_another_on_the_same_object() {
    accepts(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         \x20   def bump(self) -> None:\n\
         \x20       self.x = self.x + 1\n\
         \n\
         \x20   def bump_twice(self) -> None:\n\
         \x20       self.bump()\n\
         \x20       self.bump()\n",
    );
}

#[test]
fn a_class_in_another_source_leaves_construction_undetermined() {
    // Matching how functions behave: each decorated member is validated on its own, so rejecting
    // an unseen class would make acceptance depend on decoration order.
    assert_eq!(
        reject("def use() -> int:\n    c = Elsewhere()\n    return 1\n"),
        LowerErrorKind::UndeterminedBinding
    );
}

/// Python's own stance, which is what an unconfigured compilation resolves to.
///
/// Read from the frontend's declaration rather than rebuilt here, so these tests lower under the
/// same bundle the pipeline uses.
fn python_stance() -> compylr_ir::Behavior {
    compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR)
}
