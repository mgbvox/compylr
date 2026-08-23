//! Lowering a parsed Python module into compylr IR.
//!
//! Lowering is where the "strict annotated subset" is enforced. Anything outside it is rejected
//! with a located diagnostic rather than guessed at, because a transpiler that quietly picks a
//! meaning produces code that compiles and does the wrong thing.
//!
//! Two decisions shape the structure:
//!
//! * **Call targets are resolved only as far as one source allows.** Signatures are collected in
//!   a first pass, so a call within the source is typed and its arguments checked, and a function
//!   may call one defined below it. A callee this source cannot see is *not* an error: lowering
//!   handles one source at a time, and a decorated function may legitimately call one in a module
//!   that has not been marked yet, so rejecting here would make success depend on arrival order.
//!   Such a call is recorded by name and checked by [`crate::ir::Unit::validate`] once every
//!   source is assembled.
//! * **Inference covers whatever is determined.** A binding may omit its annotation when its
//!   initializer's type follows from literals, already-typed names, negation, arithmetic,
//!   comparisons, and calls to functions this source can see. Each has exactly one possible
//!   result given its operands, so this computes an answer that was already fixed rather than
//!   choosing among candidates. An expression containing an unseen call is *undetermined* — not
//!   an error — and such a binding still needs an annotation.
//!
//! Lowering is therefore also a small type checker: [`lower_expr`] returns an expression and
//! its type together, so shape and type can never be derived from separate traversals and
//! disagree.

use std::collections::{BTreeMap, HashMap, HashSet};

use ruff_python_ast::{
    CmpOp, ElifElseClause, Expr as PyExpr, ModModule, Number, Operator, Parameters, Stmt as PyStmt,
    StmtFunctionDef, UnaryOp,
};
use ruff_python_parser::Parsed;
use ruff_text_size::Ranged;

use crate::span_of;
use crate::spelling::{PythonOperator, PythonTypeName};

use compylr_diagnostics::error::{LowerError, LowerErrorKind};
use compylr_ir::{
    Attribute, Behavior, BinOp, Class, DivMode, Expr, Function, Literal, Param, Stmt, Ty,
    returns_on_all_paths,
};

/// Remove a trailing bare `return` from a constructor, and reject one anywhere else.
///
/// An instance is built from the *whole* constructor: every attribute becomes a field, and the
/// struct is assembled once the body has run. An early return would leave the attributes below it
/// unassigned, which the model has no way to represent — there is no such thing as a half-built
/// instance here.
///
/// A trailing `return` is a different matter. It does nothing in Python either, so it is dropped
/// rather than refused; refusing it would reject a program whose meaning is unambiguous over a
/// stylistic habit.
///
/// Caught here rather than left to the backend, which emitted `return Ok(())` from a function
/// returning `Self` — generated code that does not compile, reported as a complaint about Rust
/// rather than about the class.
fn strip_constructor_return(body: &mut Vec<Stmt>, node: &impl Ranged) -> Result<(), LowerError> {
    if matches!(body.last(), Some(Stmt::ReturnUnit)) {
        body.pop();
    }
    if body.iter().any(returns_anywhere) {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "'__init__' cannot return early: every attribute becomes a field of the instance, so \
             a return before the end would leave part of it unassigned",
            node,
        ));
    }
    Ok(())
}

/// Whether a statement, or anything nested in it, returns.
fn returns_anywhere(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) | Stmt::ReturnUnit => true,
        Stmt::If {
            then, otherwise, ..
        } => then.iter().any(returns_anywhere) || otherwise.iter().any(returns_anywhere),
        Stmt::While { body, .. } | Stmt::For { body, .. } => body.iter().any(returns_anywhere),
        _ => false,
    }
}

/// Names visible inside a function body, with the type each was bound at.
///
/// A stack of frames rather than one map, because a block is a scope: a name bound inside a branch
/// or a loop is gone when it ends. That is stricter than Python, which leaks such a name into the
/// enclosing function and fails at runtime if the branch did not run. Rejecting at compile time is
/// the point of the subset — the alternative is admitting names whose existence depends on a
/// runtime test, and then either rejecting reads of them anyway or emitting code that does not
/// compile.
///
/// Lookup walks outward and binding writes to the innermost frame, which together are what make
/// `i = i + 1` inside a loop update the counter declared outside it rather than shadow it.
/// What a name denotes, and where its value came from.
#[derive(Debug, Clone)]
struct Binding {
    /// The type it was bound at.
    ty: Ty,
    /// The collection parameter this value ultimately came from, if any.
    ///
    /// Recorded so mutating an alias can be refused on the same terms as mutating the parameter.
    /// In Python `copied = xs` binds a second name to one object; under compylr's value semantics
    /// it copies. Mutating either is observable to the caller in the first reading and in neither
    /// under the second, so a rule that stopped at the parameter would be blind to one spelling of
    /// the same hazard.
    origin: Option<String>,
}

#[derive(Debug)]
struct Scope {
    /// Innermost frame last.
    frames: Vec<HashMap<String, Binding>>,
    /// Names that were bound in a block that has since ended.
    ///
    /// Kept only so that reading one afterwards can say the binding may not have happened, rather
    /// than that the name is unknown. The name *is* in the source a few lines up, so "not defined"
    /// reads as a compiler bug; what is actually wrong is that whether it was bound depends on a
    /// test compylr will not evaluate.
    departed: HashSet<String>,
}

impl Scope {
    /// A function's outermost scope, holding its parameters.
    fn function(params: &[Param]) -> Self {
        Self {
            frames: vec![
                params
                    .iter()
                    .map(|param| {
                        (
                            param.name.clone(),
                            Binding {
                                ty: param.ty.clone(),
                                origin: None,
                            },
                        )
                    })
                    .collect(),
            ],
            departed: HashSet::new(),
        }
    }

    /// Enter a nested block.
    fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    /// Leave a nested block, discarding everything it bound.
    fn pop(&mut self) {
        if let Some(frame) = self.frames.pop() {
            self.departed.extend(frame.into_keys());
        }
        debug_assert!(
            !self.frames.is_empty(),
            "the function frame is never popped"
        );
    }

    /// Whether a name was bound in a block that has ended.
    fn was_bound_in_a_departed_block(&self, name: &str) -> bool {
        self.departed.contains(name)
    }

    /// The type a visible name was bound at, searching innermost frame outward.
    fn get(&self, name: &str) -> Option<&Ty> {
        self.binding(name).map(|binding| &binding.ty)
    }

    /// The whole binding a visible name refers to.
    fn binding(&self, name: &str) -> Option<&Binding> {
        self.frames.iter().rev().find_map(|frame| frame.get(name))
    }

    /// The collection parameter a name's value ultimately came from, if any.
    fn origin(&self, name: &str) -> Option<&str> {
        self.binding(name).and_then(|b| b.origin.as_deref())
    }

    /// Update the origin of an existing binding, in whichever frame owns it.
    ///
    /// Reassignment changes where a name's value came from. `working = xs` then `working = []`
    /// leaves `working` holding a fresh collection, and mutating it afterwards is safe — which
    /// matters because building a fresh collection is exactly the workaround the alias diagnostic
    /// recommends.
    fn set_origin(&mut self, name: &str, origin: Option<String>) {
        for frame in self.frames.iter_mut().rev() {
            if let Some(binding) = frame.get_mut(name) {
                binding.origin = origin;
                return;
            }
        }
    }

    /// Whether a name is visible here.
    fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Introduce a name in the innermost frame.
    fn declare(&mut self, name: String, ty: Ty, origin: Option<String>) {
        self.frames
            .last_mut()
            .expect("the function frame is never popped")
            .insert(name, Binding { ty, origin });
    }
}

/// What lowering a statement needs to know beyond the statement itself.
///
/// `in_loop` is the whole reason this is a struct rather than two arguments: `break` outside a loop
/// has to be a located diagnostic, and the alternative is letting the backend discover it as an
/// unexplained failure to generate code.
#[derive(Clone, Copy)]
struct Ctx<'a> {
    /// The enclosing function's declared return type.
    ret: &'a Ty,
    /// Everything nameable while lowering: functions and classes.
    lowering: Lowering<'a>,
    /// Whether the body being lowered is a constructor, where attributes may be declared.
    in_init: bool,
    /// Names that arrived as parameters.
    ///
    /// Needed because mutation is confined to locals, and a parameter shares a scope frame with
    /// the body's own top-level bindings — so the scope alone cannot tell them apart.
    params: &'a HashSet<String>,
    /// Whether a loop encloses this statement. Conditionals do not reset it, so `break` inside an
    /// `if` inside a loop is fine — which is the common case.
    in_loop: bool,
}

impl<'a> Ctx<'a> {
    /// The same context, inside a loop body.
    fn inside_loop(self) -> Self {
        Self {
            in_loop: true,
            ..self
        }
    }
}

/// The builtin sequence-of-integers form, recognised in iterable position.
const RANGE: &str = "range";

fn err(kind: LowerErrorKind, message: impl Into<String>, node: &impl Ranged) -> LowerError {
    LowerError::new(kind, message, span_of(node.range()))
}

/// A function's declared interface, as written in its annotations.
///
/// Collected before any body is lowered, so that a call can be typed without depending on which
/// function was defined first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// Parameter types, in declaration order.
    pub params: Vec<Ty>,
    /// Declared return type.
    pub ret: Ty,
}

/// Every signature visible while lowering one source.
pub type Signatures = HashMap<String, Signature>;

/// What a class offers, without any body having been lowered.
///
/// Collected in the same first pass as function signatures and for the same reason: a function may
/// construct a class defined below it, and acceptance must not depend on definition order.
#[derive(Debug, Clone, Default)]
pub struct ClassSignature {
    /// Attributes in declaration order, with their declared types.
    pub attributes: Vec<Attribute>,
    /// The constructor's parameters, excluding `self`.
    pub init: Vec<Ty>,
    /// Method signatures by name, excluding `__init__`.
    pub methods: HashMap<String, Signature>,
}

/// Every class visible while lowering one source.
pub type ClassSignatures = HashMap<String, ClassSignature>;

/// The names of every class in scope.
///
/// Separate from [`ClassSignatures`] and gathered before it: an annotation may name a class whose
/// own attributes are not yet collected, including its own. Names are all an annotation needs.
pub type ClassNames = HashSet<String>;

/// Scan a source for class names, before anything else is collected.
pub fn collect_class_names(parsed: &Parsed<ModModule>) -> ClassNames {
    parsed
        .syntax()
        .body
        .iter()
        .filter_map(|stmt| match stmt {
            PyStmt::ClassDef(def) => Some(def.name.to_string()),
            _ => None,
        })
        .collect()
}

/// Everything nameable while lowering: the functions and the classes.
///
/// Bundled because expression lowering needs both — a call resolves against one and an attribute
/// read against the other — and threading a second reference through every helper would obscure
/// the two things that actually vary between them.
#[derive(Clone, Copy)]
pub struct Names<'a> {
    /// Function signatures.
    pub sigs: &'a Signatures,
    /// Class signatures.
    pub classes: &'a ClassSignatures,
    /// Just the class names, which is all an annotation needs.
    pub class_names: &'a ClassNames,
}

/// Everything lowering an expression needs beyond the expression itself.
///
/// [`Names`] wrapped rather than extended, because a behavior is not a name and a struct called
/// "everything nameable" should not quietly start carrying one. Wrapped rather than passed
/// alongside, because [`lower_expr`] takes this and not [`Ctx`], and a second parameter would
/// have meant editing forty call sites to thread a value none of them look at.
///
/// `Copy`, like the `Names` it holds, so passing it down costs nothing and no call site has to
/// decide whether to borrow.
#[derive(Clone, Copy)]
pub struct Lowering<'a> {
    /// Functions and classes in scope.
    pub names: Names<'a>,
    /// Which language supplies the meaning of each operation this source lowers.
    ///
    /// Held here rather than looked up, because it is per *source*: one unit may hold members
    /// lowered under different behaviors, and a call between two of them is an ordinary call.
    pub behavior: Behavior,
}

/// The name a method's receiver must carry.
const SELF: &str = "self";

/// The constructor's name.
const INIT: &str = "__init__";

/// Collect the signature of every function in a source, without lowering any body.
///
/// This reads annotations only. Parameters and returns are mandatory, so nothing here needs
/// inference — which is what makes the pass immune to definition order and safe to run first.
///
/// Malformed signatures are left for `lower_function` to report. Failing here would produce the
/// same diagnostics from a different place, and the body pass reports them in source order.
pub fn collect_signatures(parsed: &Parsed<ModModule>, classes: &ClassNames) -> Signatures {
    let mut signatures = Signatures::new();
    for stmt in &parsed.syntax().body {
        let PyStmt::FunctionDef(def) = stmt else {
            continue;
        };
        let Ok(params) = lower_parameters(&def.parameters, def.name.as_str(), classes) else {
            continue;
        };
        let Some(annotation) = def.returns.as_deref() else {
            continue;
        };
        let Ok(ret) = lower_annotation(annotation, true, classes) else {
            continue;
        };
        signatures.insert(
            def.name.to_string(),
            Signature {
                params: params.into_iter().map(|p| p.ty).collect(),
                ret,
            },
        );
    }
    signatures
}

/// Collect what every class in a source offers, without lowering any body.
///
/// Attribute types come from the annotated assignments in `__init__`, which is the only place they
/// may be declared. Malformed classes are skipped rather than reported, exactly as malformed
/// function signatures are: the body pass reports them in source order.
pub fn collect_class_signatures(
    parsed: &Parsed<ModModule>,
    classes: &ClassNames,
) -> ClassSignatures {
    let mut collected = ClassSignatures::new();
    for stmt in &parsed.syntax().body {
        let PyStmt::ClassDef(def) = stmt else {
            continue;
        };
        let mut signature = ClassSignature::default();
        for member in &def.body {
            let PyStmt::FunctionDef(method) = member else {
                continue;
            };
            let Ok(params) = method_parameters(method, classes) else {
                continue;
            };
            if method.name.as_str() == INIT {
                signature.init = params.iter().map(|p| p.ty.clone()).collect();
                signature.attributes = collect_attributes(&method.body, classes);
                continue;
            }
            let Some(annotation) = method.returns.as_deref() else {
                continue;
            };
            let Ok(ret) = lower_annotation(annotation, true, classes) else {
                continue;
            };
            signature.methods.insert(
                method.name.to_string(),
                Signature {
                    params: params.into_iter().map(|p| p.ty).collect(),
                    ret,
                },
            );
        }
        collected.insert(def.name.to_string(), signature);
    }
    collected
}

/// The attributes an `__init__` body declares, reading annotations only.
fn collect_attributes(body: &[PyStmt], classes: &ClassNames) -> Vec<Attribute> {
    let mut attributes = Vec::new();
    for stmt in body {
        let PyStmt::AnnAssign(assign) = stmt else {
            continue;
        };
        let Some(name) = self_attribute_name(&assign.target) else {
            continue;
        };
        let Ok(ty) = lower_annotation(&assign.annotation, false, classes) else {
            continue;
        };
        attributes.push(Attribute {
            name: name.to_string(),
            ty,
        });
    }
    attributes
}

/// The attribute name in `self.<name>`, or `None` for anything else.
fn self_attribute_name(target: &PyExpr) -> Option<&str> {
    let PyExpr::Attribute(attribute) = target else {
        return None;
    };
    let PyExpr::Name(object) = attribute.value.as_ref() else {
        return None;
    };
    (object.id.as_str() == SELF).then(|| attribute.attr.as_str())
}

/// A method's parameters, with `self` removed and checked.
fn method_parameters(
    def: &StmtFunctionDef,
    classes: &ClassNames,
) -> Result<Vec<Param>, LowerError> {
    let receiver = def.parameters.args.first().ok_or_else(|| {
        err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "method '{}' must take 'self' as its first parameter",
                def.name
            ),
            def,
        )
    })?;
    if receiver.parameter.name.as_str() != SELF {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "method '{}' must take 'self' as its first parameter, not '{}'",
                def.name, receiver.parameter.name
            ),
            def,
        ));
    }
    if receiver.parameter.annotation.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "'self' must not be annotated: its type is the class it is defined in, and writing it \
             invites writing it differently",
            def,
        ));
    }

    // The receiver is stripped before the usual parameter rules run, so every remaining parameter
    // is checked exactly as a free function's would be.
    let mut without_self = def.parameters.clone();
    without_self.args.remove(0);
    lower_parameters(&without_self, def.name.as_str(), classes)
}

/// Lower every top-level function definition in a parsed source.
///
/// Only function definitions are permitted at top level; a module-level statement such as an
/// `if __name__ == '__main__':` guard has no meaning once the function is compiled into a
/// shared artifact, so it is rejected rather than silently dropped.
pub fn lower_source(
    parsed: &Parsed<ModModule>,
    behavior: Behavior,
) -> Result<Vec<Function>, LowerError> {
    lower_source_with(parsed, &Signatures::new(), behavior)
}

/// Lower a source into its functions **and** its classes.
///
/// [`lower_source`] remains for callers that only want functions; this is what the pipeline uses,
/// since a unit holds both.
pub fn lower_source_members(
    parsed: &Parsed<ModModule>,
    behavior: Behavior,
) -> Result<(Vec<Function>, Vec<Class>), LowerError> {
    lower_source_members_with(
        parsed,
        &Signatures::new(),
        &ClassSignatures::new(),
        behavior,
    )
}

/// Lower a source with signatures from elsewhere already known.
///
/// The decorator submits each function as its own source, so a call between two decorated
/// functions is a call across sources. Supplying the signatures gathered from every source lets
/// those calls be typed, which is the difference between the decorator inferring
/// `doubled = double(n)` and demanding an annotation for it.
///
/// Signatures found in `parsed` take precedence, so a source is always typed against its own
/// definitions first.
pub fn lower_source_with(
    parsed: &Parsed<ModModule>,
    external: &Signatures,
    behavior: Behavior,
) -> Result<Vec<Function>, LowerError> {
    Ok(lower_source_members_with(parsed, external, &ClassSignatures::new(), behavior)?.0)
}

/// Lower a source into both kinds of member, with names from elsewhere already known.
pub fn lower_source_members_with(
    parsed: &Parsed<ModModule>,
    external: &Signatures,
    external_classes: &ClassSignatures,
    behavior: Behavior,
) -> Result<(Vec<Function>, Vec<Class>), LowerError> {
    // Pass one: every signature in the source, so a call to a function or class defined later
    // types the same as one defined earlier.
    // Class names come first: an annotation may name a class whose attributes are collected in the
    // very same pass, including its own.
    let mut class_names: ClassNames = external_classes.keys().cloned().collect();
    class_names.extend(collect_class_names(parsed));
    let mut signatures = external.clone();
    signatures.extend(collect_signatures(parsed, &class_names));
    let mut class_signatures = external_classes.clone();
    class_signatures.extend(collect_class_signatures(parsed, &class_names));
    let lowering = Lowering {
        names: Names {
            sigs: &signatures,
            classes: &class_signatures,
            class_names: &class_names,
        },
        behavior,
    };

    let mut functions = Vec::new();
    let mut classes = Vec::new();
    for stmt in &parsed.syntax().body {
        match stmt {
            PyStmt::FunctionDef(def) => {
                functions.push(lower_function_in(def, lowering, None, false)?)
            }
            PyStmt::ClassDef(def) => classes.push(lower_class(def, lowering)?),
            PyStmt::Import(_) | PyStmt::ImportFrom(_) => {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "imports are not supported; only function definitions may appear at top level",
                    stmt,
                ));
            }
            other => {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "only function and class definitions are permitted at top level",
                    other,
                ));
            }
        }
    }
    Ok((functions, classes))
}

/// Lower a class definition.
pub fn lower_class(
    def: &ruff_python_ast::StmtClassDef,
    lowering: Lowering<'_>,
) -> Result<Class, LowerError> {
    if !def.decorator_list.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "class '{}' carries a decorator, which is not supported",
                def.name
            ),
            def,
        ));
    }
    if def.type_params.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedType,
            format!(
                "class '{}' declares type parameters, which are not yet supported",
                def.name
            ),
            def,
        ));
    }
    if def
        .arguments
        .as_ref()
        .is_some_and(|a| !a.args.is_empty() || !a.keywords.is_empty())
    {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "class '{}' declares a base class; inheritance is not supported",
                def.name
            ),
            def,
        ));
    }

    let (doc, body) = split_docstring(&def.body);

    let mut init: Option<&StmtFunctionDef> = None;
    let mut method_defs: Vec<&StmtFunctionDef> = Vec::new();
    for member in body {
        let PyStmt::FunctionDef(method) = member else {
            return Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "a class body may only contain method definitions: a class-level assignment is \
                 state shared by every instance, which is a different thing than an attribute",
                member,
            ));
        };
        let name = method.name.as_str();
        if name == INIT {
            init = Some(method);
            continue;
        }
        // Every other dunder would need a decision about what the target language does with it,
        // and nothing depends on one yet.
        if name.starts_with("__") && name.ends_with("__") {
            return Err(err(
                LowerErrorKind::UnsupportedConstruct,
                format!(
                    "'{name}' is not supported; '__init__' is the only special method in the subset"
                ),
                method,
            ));
        }
        if method_defs.iter().any(|m| m.name.as_str() == name) {
            return Err(err(
                LowerErrorKind::DuplicateFunction,
                format!("method '{name}' is defined twice in class '{}'", def.name),
                method,
            ));
        }
        method_defs.push(method);
    }

    let Some(init_def) = init else {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "class '{}' has no '__init__'; attributes are declared there, so without one the \
                 class has no defined shape",
                def.name
            ),
            def,
        ));
    };

    let name = def.name.to_string();
    let mut init = lower_function_in(init_def, lowering, Some(&name), true)?;
    if init.ret != Ty::Unit {
        return Err(err(
            LowerErrorKind::TypeMismatch,
            "'__init__' must be annotated '-> None'",
            init_def,
        ));
    }
    strip_constructor_return(&mut init.body, init_def)?;

    // Read back from the lowered constructor rather than from the annotations again, so the
    // declared shape and the code that initialises it cannot disagree.
    let attributes: Vec<Attribute> = init
        .body
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::SetAttr { name, ty, .. } => Some(Attribute {
                name: name.clone(),
                ty: ty.clone(),
            }),
            _ => None,
        })
        .collect();

    let mut methods = BTreeMap::new();
    for method in method_defs {
        let lowered = lower_function_in(method, lowering, Some(&name), false)?;
        methods.insert(lowered.name.clone(), lowered);
    }

    Ok(Class {
        name,
        attributes,
        init,
        methods,
        doc,
        span: span_of(def.range()),
    })
}

/// Lower a single top-level function definition.
pub fn lower_function(
    def: &StmtFunctionDef,
    sigs: &Signatures,
    behavior: Behavior,
) -> Result<Function, LowerError> {
    lower_function_in(
        def,
        Lowering {
            names: Names {
                sigs,
                classes: &ClassSignatures::new(),
                class_names: &ClassNames::new(),
            },
            behavior,
        },
        None,
        false,
    )
}

/// Lower a function or method.
///
/// `enclosing` names the class when this is a method, which is what gives `self` a type;
/// `in_init` permits attribute declarations, which are legal in exactly one place.
pub fn lower_function_in(
    def: &StmtFunctionDef,
    lowering: Lowering<'_>,
    enclosing: Option<&str>,
    in_init: bool,
) -> Result<Function, LowerError> {
    if def.is_async {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "'{}' is an async function, which is not supported",
                def.name
            ),
            def,
        ));
    }
    if !def.decorator_list.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!("'{}' carries a decorator, which is not supported", def.name),
            def,
        ));
    }
    if def.type_params.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedType,
            format!(
                "'{}' declares type parameters, which are not yet supported",
                def.name
            ),
            def,
        ));
    }

    // `self` is the receiver and nothing else. A free function taking a parameter of that name
    // would be legal Python, and would make the emitted code ambiguous about what `self` denotes.
    if enclosing.is_none()
        && def
            .parameters
            .args
            .iter()
            .any(|arg| arg.parameter.name.as_str() == SELF)
    {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "'{}' takes a parameter named 'self', which is reserved for a method's receiver",
                def.name
            ),
            def,
        ));
    }

    if def.name.as_str() == RANGE {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "'range' is reserved: it is a builtin, and a function of that name would make \
             `range(n)` mean different things depending on what else was marked for compilation",
            def,
        ));
    }

    if def.name.as_str() == "len" {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "'len' is reserved: it is a builtin, and a function of that name would make \
             `len(x)` mean different things depending on what else was marked for compilation",
            def,
        ));
    }

    // A method's receiver is stripped and re-introduced as a typed name below, so every remaining
    // parameter goes through exactly the rules a free function's would.
    let params = match enclosing {
        Some(_) => method_parameters(def, lowering.names.class_names)?,
        None => lower_parameters(
            &def.parameters,
            def.name.as_str(),
            lowering.names.class_names,
        )?,
    };

    let ret = match def.returns.as_deref() {
        Some(annotation) => lower_annotation(annotation, true, lowering.names.class_names)?,
        None => {
            return Err(err(
                LowerErrorKind::MissingAnnotation,
                format!("function '{}' needs a return type annotation", def.name),
                def,
            ));
        }
    };

    let mut scope = Scope::function(&params);
    let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
    // `self` is visible but is not a parameter for the mutation rule: an instance is not converted
    // at the boundary, so mutating through it is exactly what the caller observes.
    if let Some(class) = enclosing {
        scope.declare(SELF.to_string(), Ty::Instance(class.to_string()), None);
    }
    let ctx = Ctx {
        ret: &ret,
        lowering,
        in_init,
        params: &param_names,
        in_loop: false,
    };
    let (doc, rest) = split_docstring(&def.body);
    let body = lower_body(rest, &mut scope, ctx)?;

    // A function that declares a value must produce one on every path. With branching this is no
    // longer the structural question of whether the last statement is a `return`, so it defers to
    // the same analysis the backend uses — two implementations disagreeing would mean either
    // rejecting a valid program or emitting code that does not compile.
    //
    // Catching it here makes it an ordinary located diagnostic; left to the backend it surfaces as
    // an internal code-generation error describing the compiler\'s difficulty rather than the
    // user\'s mistake.
    if ret != Ty::Unit && !returns_on_all_paths(&body) {
        return Err(err(
            LowerErrorKind::MissingReturn,
            format!(
                "function '{}' declares a return type of '{}', but there is a path through its \
                 body that produces no value",
                def.name,
                ret.python_name()
            ),
            def,
        ));
    }

    Ok(Function {
        name: def.name.to_string(),
        params,
        ret,
        body,
        doc,
        span: span_of(def.range()),
    })
}

/// Split a leading docstring off a function body.
///
/// Python treats a bare string literal in first position as documentation: the interpreter records
/// it from the code object rather than by executing the statement, so it contributes nothing to
/// what the function does. Removing it here means the rest of lowering never sees it, and the
/// catch-all that rejects discarded expression statements keeps working everywhere else — a string
/// in *second* position is still an error, because there it really is a value thrown away.
///
/// Adjacent literals (`"a" "b"`) are concatenated by the parser into one node, so they are covered
/// without special handling. An f-string is a different node and is not matched, which is correct:
/// Python does not treat an f-string as a docstring either.
fn split_docstring(body: &[PyStmt]) -> (Option<String>, &[PyStmt]) {
    let Some(PyStmt::Expr(statement)) = body.first() else {
        return (None, body);
    };
    let PyExpr::StringLiteral(literal) = statement.value.as_ref() else {
        return (None, body);
    };
    (Some(literal.value.to_str().to_string()), &body[1..])
}

fn lower_parameters(
    parameters: &Parameters,
    owner: &str,
    classes: &ClassNames,
) -> Result<Vec<Param>, LowerError> {
    // Only plain positional parameters are in the subset. Each of the other forms would need a
    // calling-convention decision on the target side that nothing depends on yet.
    if !parameters.posonlyargs.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "positional-only parameters are not supported",
            parameters,
        ));
    }
    if !parameters.kwonlyargs.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "keyword-only parameters are not supported",
            parameters,
        ));
    }
    if parameters.vararg.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "variadic '*args' parameters are not supported",
            parameters,
        ));
    }
    if parameters.kwarg.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "variadic '**kwargs' parameters are not supported",
            parameters,
        ));
    }

    let mut params = Vec::with_capacity(parameters.args.len());
    for arg in &parameters.args {
        if arg.default.is_some() {
            return Err(err(
                LowerErrorKind::UnsupportedConstruct,
                format!(
                    "parameter '{}' has a default value, which is not supported",
                    arg.parameter.name
                ),
                arg,
            ));
        }
        let Some(annotation) = arg.parameter.annotation.as_deref() else {
            return Err(err(
                LowerErrorKind::MissingAnnotation,
                format!(
                    "parameter '{}' of '{owner}' needs a type annotation",
                    arg.parameter.name
                ),
                &arg.parameter,
            ));
        };
        params.push(Param {
            name: arg.parameter.name.to_string(),
            ty: lower_annotation(annotation, false, classes)?,
        });
    }
    Ok(params)
}

/// Convert a Python annotation expression into an IR type.
///
/// `allow_unit` is true only for return annotations: `None` describes "returns nothing", which
/// is meaningless for a parameter.
fn lower_annotation(
    annotation: &PyExpr,
    allow_unit: bool,
    classes: &ClassNames,
) -> Result<Ty, LowerError> {
    match annotation {
        PyExpr::Name(name) => match name.id.as_str() {
            "int" => Ok(Ty::Int),
            "float" => Ok(Ty::Float),
            "bool" => Ok(Ty::Bool),
            "str" => Ok(Ty::Str),
            // A class defined in the same source names its instance type. Unknown names are
            // still rejected, so a typo does not become a phantom type.
            other if classes.contains(other) => Ok(Ty::Instance(other.to_string())),
            other => Err(err(
                LowerErrorKind::UnsupportedType,
                format!("'{other}' is not a supported type annotation"),
                annotation,
            )),
        },
        PyExpr::NoneLiteral(_) => {
            if allow_unit {
                Ok(Ty::Unit)
            } else {
                Err(err(
                    LowerErrorKind::UnsupportedType,
                    "'None' is only supported as a return annotation",
                    annotation,
                ))
            }
        }
        PyExpr::Subscript(subscript) => lower_generic_annotation(subscript, annotation, classes),
        other => Err(err(
            LowerErrorKind::UnsupportedType,
            "unsupported type annotation",
            other,
        )),
    }
}

/// Lower every element of a literal and unify their types.
///
/// Elements must agree. A literal whose elements disagree is a type error rather than a union:
/// the IR has no union type, and inventing one here would put a decision in the compiler that the
/// user should be making in the annotation.
fn unify_elements(
    elements: &[PyExpr],
    scope: &Scope,
    lowering: Lowering<'_>,
    node: &PyExpr,
    what: &str,
) -> Result<(Vec<Expr>, Option<Ty>), LowerError> {
    let mut lowered = Vec::with_capacity(elements.len());
    let mut types = Vec::with_capacity(elements.len());
    for element in elements {
        let (expr, ty) = lower_expr(element, scope, lowering)?;
        lowered.push(expr);
        types.push(ty);
    }
    let unified = agree(&types, node, &format!("{what} element"))?;

    // Promotion inside a literal, matching promotion everywhere else: mixing integers and floats
    // yields floats, and each integer element carries an explicit conversion.
    if unified.as_ref() == Some(&Ty::Float) {
        for (expr, ty) in lowered.iter_mut().zip(&types) {
            if ty.as_ref() == Some(&Ty::Int) {
                let taken = std::mem::replace(expr, Expr::Name(String::new()));
                *expr = Expr::to_float(taken);
            }
        }
    }
    // An empty literal has nothing to infer from, so its type is undetermined and the binding
    // rule demands an annotation -- the same sentence that already governs a call initializer.
    if lowered.is_empty() {
        return Ok((lowered, None));
    }
    Ok((lowered, unified))
}

/// The single type a list of maybe-determined types agrees on.
///
/// Returns `None` when any is undetermined, which propagates outward exactly as it does through
/// arithmetic. Integers and floats agree on float, matching numeric promotion.
fn agree(types: &[Option<Ty>], node: &PyExpr, what: &str) -> Result<Option<Ty>, LowerError> {
    let mut settled: Option<Ty> = None;
    for ty in types {
        let Some(ty) = ty else { return Ok(None) };
        settled = Some(match settled {
            None => ty.clone(),
            Some(current) if current == *ty => current,
            Some(current) if current.is_numeric() && ty.is_numeric() => Ty::Float,
            Some(current) => {
                return Err(err(
                    LowerErrorKind::TypeMismatch,
                    format!(
                        "every {what} must have the same type, but found '{}' and '{}'",
                        current.python_name(),
                        ty.python_name()
                    ),
                    node,
                ));
            }
        });
    }
    Ok(settled)
}

/// Lower a subscript, typing it from the collection being read.
fn lower_subscript(
    subscript: &ruff_python_ast::ExprSubscript,
    scope: &Scope,
    lowering: Lowering<'_>,
    node: &PyExpr,
) -> Result<(Expr, TyResult), LowerError> {
    if matches!(subscript.slice.as_ref(), PyExpr::Slice(_)) {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "slicing is not supported",
            node,
        ));
    }

    let (base, base_ty) = lower_expr(&subscript.value, scope, lowering)?;
    let (index, index_ty) = lower_expr(&subscript.slice, scope, lowering)?;

    let Some(base_ty) = base_ty else {
        // The collection's own type is undetermined, so the element's is too.
        return Ok((
            Expr::Subscript {
                base: Box::new(base),
                index: Box::new(index),
                origin: lowering.behavior.index_origin(),
                checked: lowering.behavior.index_checked(),
            },
            None,
        ));
    };

    let result = match &base_ty {
        Ty::List(element) => {
            expect_index(&index_ty, &Ty::Int, node, "a sequence index")?;
            Some((**element).clone())
        }
        Ty::Dict(key, value) => {
            expect_index(&index_ty, key, node, "a mapping key")?;
            Some((**value).clone())
        }
        Ty::Tuple(elements) => {
            // Each position has its own type, so a computed index has no single answer.
            let Expr::Literal(Literal::Int(position)) = &index else {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "a tuple index must be a literal, because each position has its own type",
                    node,
                ));
            };
            let position = *position;
            if position < 0 || position as usize >= elements.len() {
                return Err(err(
                    LowerErrorKind::TypeMismatch,
                    format!(
                        "index {position} is outside a tuple of {} element(s)",
                        elements.len()
                    ),
                    node,
                ));
            }
            // Returned directly rather than falling through to the generic subscript below: a
            // tuple read is a static field access, not a lookup.
            return Ok((
                Expr::TupleIndex {
                    base: Box::new(base),
                    position: position as usize,
                },
                Some(elements[position as usize].clone()),
            ));
        }
        other => {
            return Err(err(
                LowerErrorKind::TypeMismatch,
                format!("'{}' cannot be subscripted", other.python_name()),
                node,
            ));
        }
    };

    Ok((
        Expr::Subscript {
            base: Box::new(base),
            index: Box::new(index),
            origin: lowering.behavior.index_origin(),
            checked: lowering.behavior.index_checked(),
        },
        result,
    ))
}

/// Check an index's type against what the collection expects.
fn expect_index(
    actual: &TyResult,
    expected: &Ty,
    node: &PyExpr,
    what: &str,
) -> Result<(), LowerError> {
    let Some(actual) = actual else { return Ok(()) };
    if actual == expected {
        return Ok(());
    }
    Err(err(
        LowerErrorKind::TypeMismatch,
        format!(
            "{what} must be '{}', but found '{}'",
            expected.python_name(),
            actual.python_name()
        ),
        node,
    ))
}

/// Lower a parameterised annotation such as `list[int]` or `dict[str, int]`.
///
/// A bare `list` is rejected: an element type that is not written down is not a type compylr can
/// compile against, and guessing one would put a decision in the compiler that belongs in the
/// user's annotation.
fn lower_generic_annotation(
    subscript: &ruff_python_ast::ExprSubscript,
    node: &PyExpr,
    classes: &ClassNames,
) -> Result<Ty, LowerError> {
    let PyExpr::Name(name) = subscript.value.as_ref() else {
        return Err(err(
            LowerErrorKind::UnsupportedType,
            "unsupported generic type annotation",
            node,
        ));
    };

    // `dict[str, int]` puts a tuple in the slice; `list[int]` puts the element directly.
    let parameters: Vec<&PyExpr> = match subscript.slice.as_ref() {
        PyExpr::Tuple(tuple) => tuple.elts.iter().collect(),
        single => vec![single],
    };

    let kind = name.id.as_str();
    let lowered = |exprs: &[&PyExpr]| -> Result<Vec<Ty>, LowerError> {
        exprs
            .iter()
            .map(|p| lower_annotation(p, false, classes))
            .collect::<Result<Vec<_>, _>>()
    };

    let wrong_arity = |wanted: &str| {
        err(
            LowerErrorKind::UnsupportedType,
            format!(
                "'{kind}' takes {wanted}, but {} were given",
                parameters.len()
            ),
            node,
        )
    };

    // Keys and set elements must be comparable and hashable. A floating-point key can never be
    // retrieved once it is `nan`, and most targets cannot hash a float at all — so this is
    // refused where the user wrote it, rather than surfacing later as a target-language
    // complaint about a trait bound.
    let must_key = |ty: Ty, what: &str| -> Result<Ty, LowerError> {
        if ty.can_key() {
            Ok(ty)
        } else {
            Err(err(
                LowerErrorKind::UnsupportedType,
                format!(
                    "'{}' cannot be a {what}: only int, str, and bool can be compared and hashed",
                    ty.python_name()
                ),
                node,
            ))
        }
    };

    match kind {
        "list" => {
            let mut types = lowered(&parameters)?;
            if types.len() != 1 {
                return Err(wrong_arity("one element type"));
            }
            Ok(Ty::List(Box::new(types.remove(0))))
        }
        "set" => {
            let mut types = lowered(&parameters)?;
            if types.len() != 1 {
                return Err(wrong_arity("one element type"));
            }
            Ok(Ty::Set(Box::new(must_key(types.remove(0), "set element")?)))
        }
        "dict" => {
            let mut types = lowered(&parameters)?;
            if types.len() != 2 {
                return Err(wrong_arity("a key type and a value type"));
            }
            let value = types.remove(1);
            let key = must_key(types.remove(0), "mapping key")?;
            Ok(Ty::Dict(Box::new(key), Box::new(value)))
        }
        "tuple" => {
            let types = lowered(&parameters)?;
            if types.is_empty() {
                return Err(wrong_arity("at least one element type"));
            }
            Ok(Ty::Tuple(types))
        }
        other => Err(err(
            LowerErrorKind::UnsupportedType,
            format!("'{other}[...]' is not a supported type annotation"),
            node,
        )),
    }
}

/// The type of an expression, or `None` when it is not determined during lowering.
///
/// `None` is not an error. A call's type comes from the callee's signature, and lowering
/// deliberately does not resolve callees — doing so would make results depend on which
/// function was submitted first. So an expression containing a call anywhere inside it is
/// simply undetermined, and the *binding* decides what that means: infer when `Some`, demand
/// an annotation when `None`.
///
/// Keeping the uncertainty in an `Option` rather than a `Ty::Unknown` variant confines it to
/// lowering; no backend ever has to match on a state that must not reach codegen.
type TyResult = Option<Ty>;

fn lower_body(body: &[PyStmt], scope: &mut Scope, ctx: Ctx<'_>) -> Result<Vec<Stmt>, LowerError> {
    let mut lowered = Vec::with_capacity(body.len());
    for stmt in body {
        // `pass` carries no meaning, so it produces no statement. Lowering it to anything at all
        // would give `for i in range(n): pass` a body that does something.
        if matches!(stmt, PyStmt::Pass(_)) {
            continue;
        }
        lowered.push(lower_stmt(stmt, scope, ctx)?);
    }
    Ok(lowered)
}

/// Lower a nested block in its own scope.
fn lower_block(body: &[PyStmt], scope: &mut Scope, ctx: Ctx<'_>) -> Result<Vec<Stmt>, LowerError> {
    scope.push();
    let lowered = lower_body(body, scope, ctx);
    scope.pop();
    lowered
}

/// Lower a conditional or loop test, which must be a boolean.
///
/// Python treats many values as truthy; compylr does not. A subset whose annotations are mandatory
/// everywhere should not then infer that an integer means a condition — requiring a boolean keeps
/// the meaning of a test written down rather than guessed.
fn lower_test(test: &PyExpr, scope: &Scope, ctx: Ctx<'_>) -> Result<Expr, LowerError> {
    let (lowered, ty) = lower_expr(test, scope, ctx.lowering)?;
    match ty {
        // Undetermined means the test calls a function this source does not define; taken on
        // trust here exactly as a returned call is.
        None | Some(Ty::Bool) => Ok(lowered),
        Some(other) => Err(err(
            LowerErrorKind::TypeMismatch,
            format!(
                "a test must be a 'bool', but this expression is '{}'; compylr does not treat \
                 other types as true or false",
                other.python_name()
            ),
            test,
        )),
    }
}

/// Lower the `elif`/`else` tail of a conditional into the alternative of the one before it.
///
/// Python's `elif` is a flattened list in the syntax tree, but it means a conditional nested in the
/// previous alternative. Nesting it here means every consumer of the IR sees one shape rather than
/// having to know that a list of clauses is really a chain.
fn lower_clauses(
    clauses: &[ElifElseClause],
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Vec<Stmt>, LowerError> {
    let Some((first, rest)) = clauses.split_first() else {
        return Ok(Vec::new());
    };
    match first.test.as_ref() {
        None => lower_block(&first.body, scope, ctx),
        Some(test) => {
            let test = lower_test(test, scope, ctx)?;
            let then = lower_block(&first.body, scope, ctx)?;
            let otherwise = lower_clauses(rest, scope, ctx)?;
            Ok(vec![Stmt::If {
                test,
                then,
                otherwise,
            }])
        }
    }
}

/// Reject `for`/`else` and `while`/`else`.
///
/// The alternative runs when the loop finished without a `break`, which most readers misremember
/// as the opposite. A construct that reliably misleads is worse than no construct.
fn reject_loop_else(orelse: &[PyStmt], node: &impl Ranged) -> Result<(), LowerError> {
    if orelse.is_empty() {
        return Ok(());
    }
    Err(err(
        LowerErrorKind::UnsupportedConstruct,
        "an 'else' on a loop is not supported: it runs when the loop ended without a 'break', \
         which reads as the opposite of what it does",
        node,
    ))
}

/// Reject `break` or `continue` with no enclosing loop.
fn reject_outside_loop(keyword: &str, ctx: Ctx<'_>, node: &impl Ranged) -> Result<(), LowerError> {
    if ctx.in_loop {
        return Ok(());
    }
    Err(err(
        LowerErrorKind::LoopControlOutsideLoop,
        format!("'{keyword}' is not inside a loop"),
        node,
    ))
}

/// Lower `for <name> in <iterable>`.
fn lower_for(
    stmt: &PyStmt,
    node: &ruff_python_ast::StmtFor,
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Stmt, LowerError> {
    if node.is_async {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "'async for' is not supported",
            stmt,
        ));
    }
    reject_loop_else(&node.orelse, stmt)?;
    let name = binding_target(&node.target, stmt)?.to_string();

    let (iter, element) = lower_iterable(&node.iter, scope, ctx)?;

    // The loop variable belongs to the loop's own frame, so it is gone afterwards — a read of it
    // after the loop would otherwise depend on the collection having been non-empty.
    scope.push();
    // The loop variable holds one element, cloned out of the container, so it is never an alias.
    scope.declare(name.clone(), element.clone(), None);
    let body = lower_body(&node.body, scope, ctx.inside_loop());
    scope.pop();

    Ok(Stmt::For {
        name,
        ty: element,
        iter,
        body: body?,
    })
}

/// Lower what a `for` iterates, returning it alongside the type each step yields.
fn lower_iterable(iter: &PyExpr, scope: &Scope, ctx: Ctx<'_>) -> Result<(Expr, Ty), LowerError> {
    if let Some(range) = lower_range(iter, scope, ctx)? {
        return Ok((range, Ty::Int));
    }
    let (lowered, ty) = lower_expr(iter, scope, ctx.lowering)?;
    let Some(ty) = ty else {
        return Err(err(
            LowerErrorKind::UndeterminedBinding,
            "the type of what this loop iterates cannot be determined here: it calls a function \
             this source does not define",
            iter,
        ));
    };
    // A mapping yields its keys, matching Python. Anything else would be a silent divergence in
    // the one construct where a user is most likely to assume the languages agree.
    let element = match &ty {
        Ty::List(element) | Ty::Set(element) => (**element).clone(),
        Ty::Dict(key, _) => (**key).clone(),
        other => {
            return Err(err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "'{}' cannot be iterated; a loop needs a list, dict, set, or range",
                    other.python_name()
                ),
                iter,
            ));
        }
    };
    Ok((lowered, element))
}

/// Recognise `range(...)` in iterable position, filling in the arguments Python defaults.
///
/// Returns `Ok(None)` for anything that is not a call to `range`, so the caller can go on to treat
/// it as an ordinary expression.
fn lower_range(iter: &PyExpr, scope: &Scope, ctx: Ctx<'_>) -> Result<Option<Expr>, LowerError> {
    let PyExpr::Call(call) = iter else {
        return Ok(None);
    };
    let PyExpr::Name(callee) = call.func.as_ref() else {
        return Ok(None);
    };
    if callee.id.as_str() != RANGE {
        return Ok(None);
    }
    if !call.arguments.keywords.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "'range' does not take keyword arguments",
            iter,
        ));
    }

    let args = &call.arguments.args;
    if args.is_empty() || args.len() > 3 {
        return Err(err(
            LowerErrorKind::ArityMismatch,
            format!(
                "'range' takes one, two, or three arguments, but {} were given",
                args.len()
            ),
            iter,
        ));
    }

    let mut lowered = Vec::with_capacity(args.len());
    for arg in args {
        let (expr, ty) = lower_expr(arg, scope, ctx.lowering)?;
        match ty {
            // Undetermined is taken on trust, as everywhere else a cross-source call appears.
            None | Some(Ty::Int) => lowered.push(expr),
            Some(other) => {
                return Err(err(
                    LowerErrorKind::TypeMismatch,
                    format!(
                        "'range' takes integers, but this argument is '{}'",
                        other.python_name()
                    ),
                    arg,
                ));
            }
        }
    }

    // `range(stop)` and `range(start, stop)` are spelled with the defaults Python supplies, so the
    // IR carries three components however the source was written and no consumer has to know the
    // defaults.
    let (start, stop, step) = match lowered.len() {
        1 => (Expr::int(0), lowered.remove(0), Expr::int(1)),
        2 => {
            let stop = lowered.remove(1);
            (lowered.remove(0), stop, Expr::int(1))
        }
        _ => {
            let step = lowered.remove(2);
            let stop = lowered.remove(1);
            (lowered.remove(0), stop, step)
        }
    };
    Ok(Some(Expr::Range {
        start: Box::new(start),
        stop: Box::new(stop),
        step: Box::new(step),
    }))
}

/// The one supported method.
const APPEND: &str = "append";

/// Where a binding's value came from, if it is a collection that a caller still holds.
///
/// Only a bare name can alias: every other initializer — a literal, a call, an expression — has
/// produced a fresh value. That is why the whole analysis is a lookup rather than a dataflow pass;
/// the subset has no other way to make two names denote one object.
///
/// Only collection parameters are tracked. A scalar has no mutation to observe, so a user who
/// writes `total = count` must never see a word about aliasing.
fn alias_origin(initializer: &Expr, ty: &Ty, scope: &Scope, ctx: Ctx<'_>) -> Option<String> {
    if ty.is_trivially_copyable() {
        return None;
    }
    let Expr::Name(source) = initializer else {
        return None;
    };
    // Either the source is the parameter, or it already carries one — which is what makes the
    // relation transitive without a second pass.
    if ctx.params.contains(source) {
        return Some(source.clone());
    }
    scope.origin(source).map(str::to_string)
}

/// The rejection for a statement that computes a value and discards it.
///
/// Still the default for a bare expression. `append` is carved out because its whole purpose is
/// the side effect; anything else is either dead code or a side effect the subset cannot express.
fn bare_expression_error(node: &impl Ranged) -> LowerError {
    err(
        LowerErrorKind::UnsupportedConstruct,
        "this statement computes a value and discards it, which is either dead code or a side \
         effect the subset cannot express",
        node,
    )
}

/// Reject mutating a collection that arrived as a parameter.
///
/// Collections cross the boundary by value, so a compiled function mutating a parameter would
/// leave its caller's collection unchanged where the interpreted original would have modified it.
/// Nothing raises; the caller simply gets the wrong answer. Rejecting makes that program not exist.
///
/// The diagnostic explains the copy rather than merely refusing, because the workaround — build a
/// local and return it — is not guessable from a bare "not supported".
fn reject_mutating_a_parameter(
    target: &Expr,
    scope: &Scope,
    ctx: Ctx<'_>,
    node: &impl Ranged,
) -> Result<(), LowerError> {
    let Expr::Name(name) = target else {
        return Ok(());
    };

    if ctx.params.contains(name) {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "'{name}' is a parameter, and a collection parameter is a copy — this mutation \
                 could not be observed by the caller. Build a local collection and return it \
                 instead"
            ),
            node,
        ));
    }

    // An alias is the same hazard at one remove: in Python `copied = xs` leaves both names denoting
    // one object, so the caller would have seen this. Naming the parameter is the load-bearing part
    // of the diagnostic — pointing only at a local the user just wrote gives them no reason to look
    // at the signature.
    let Some(origin) = scope.origin(name) else {
        return Ok(());
    };
    Err(err(
        LowerErrorKind::UnsupportedConstruct,
        format!(
            "'{name}' holds the parameter '{origin}', and a collection parameter is a copy — this \
             mutation could not be observed by the caller. Build a fresh collection and fill it \
             from '{origin}' instead"
        ),
        node,
    ))
}

/// Lower `collection[index] = value`.
fn lower_set_item(
    stmt: &PyStmt,
    target: &ruff_python_ast::ExprSubscript,
    value: &PyExpr,
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Stmt, LowerError> {
    if matches!(target.slice.as_ref(), PyExpr::Slice(_)) {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "slice assignment is not supported",
            stmt,
        ));
    }

    let (collection, collection_ty) = lower_expr(&target.value, scope, ctx.lowering)?;
    reject_mutating_a_parameter(&collection, scope, ctx, stmt)?;

    let (index, index_ty) = lower_expr(&target.slice, scope, ctx.lowering)?;
    let (value, value_ty) = lower_expr(value, scope, ctx.lowering)?;

    let Some(collection_ty) = collection_ty else {
        return Err(err(
            LowerErrorKind::UndeterminedBinding,
            "the type of what is being assigned into cannot be determined here",
            stmt,
        ));
    };

    // Only a sequence and a mapping have an assignable element. A set has no positions, and a
    // tuple is immutable in Python.
    let (expected_index, expected_value) = match &collection_ty {
        Ty::List(element) => (Ty::Int, (**element).clone()),
        Ty::Dict(key, entry) => ((**key).clone(), (**entry).clone()),
        other => {
            return Err(err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "'{}' does not support element assignment; only a list or a dict does",
                    other.python_name()
                ),
                stmt,
            ));
        }
    };

    expect_index(&index_ty, &expected_index, &target.slice, "an index")?;
    let value = match value_ty {
        Some(actual) => coerce(value, &actual, &expected_value).ok_or_else(|| {
            err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "this collection holds '{}', but the assigned value is '{}'",
                    expected_value.python_name(),
                    actual.python_name()
                ),
                stmt,
            )
        })?,
        None => value,
    };

    Ok(Stmt::SetItem {
        collection,
        index,
        value,
    })
}

/// Lower a statement that is a bare expression.
///
/// Only one shape is accepted: a call to `append` on a local sequence. Every other bare expression
/// is still rejected, because a value computed and discarded is either dead code or a side effect
/// the subset cannot express.
fn lower_method_statement(
    stmt: &PyStmt,
    value: &PyExpr,
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Stmt, LowerError> {
    let PyExpr::Call(call) = value else {
        return Err(bare_expression_error(stmt));
    };
    let PyExpr::Attribute(attribute) = call.func.as_ref() else {
        return Err(bare_expression_error(stmt));
    };

    let method = attribute.attr.as_str();

    // A method call on an instance, made for its effect. Accepted only when the method returns
    // nothing, so no value is actually discarded.
    let (_, receiver_ty) = lower_expr(&attribute.value, scope, ctx.lowering)?;
    if let Some(Ty::Instance(class)) = &receiver_ty
        && ctx
            .lowering
            .names
            .classes
            .get(class)
            .is_some_and(|signature| signature.methods.contains_key(method))
    {
        let (lowered, ty) = lower_expr(value, scope, ctx.lowering)?;
        if ty != Some(Ty::Unit) {
            return Err(err(
                LowerErrorKind::UnsupportedConstruct,
                format!(
                    "'{class}.{method}' returns a value, and discarding it is either dead code or \
                     a side effect the subset cannot express"
                ),
                stmt,
            ));
        }
        return Ok(Stmt::Effect(lowered));
    }

    if method != APPEND {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "'{method}' is not supported; 'append' is the only collection method in the subset"
            ),
            stmt,
        ));
    }
    if !call.arguments.keywords.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "'append' does not take keyword arguments",
            stmt,
        ));
    }
    if call.arguments.args.len() != 1 {
        return Err(err(
            LowerErrorKind::ArityMismatch,
            format!(
                "'append' takes exactly one argument, but {} were given",
                call.arguments.args.len()
            ),
            stmt,
        ));
    }

    let (sequence, sequence_ty) = lower_expr(&attribute.value, scope, ctx.lowering)?;
    reject_mutating_a_parameter(&sequence, scope, ctx, stmt)?;

    let (value, value_ty) = lower_expr(&call.arguments.args[0], scope, ctx.lowering)?;

    let Some(sequence_ty) = sequence_ty else {
        return Err(err(
            LowerErrorKind::UndeterminedBinding,
            "the type of what is being appended to cannot be determined here",
            stmt,
        ));
    };
    let Ty::List(element) = &sequence_ty else {
        return Err(err(
            LowerErrorKind::TypeMismatch,
            format!(
                "'append' is defined on a list, but this is '{}'",
                sequence_ty.python_name()
            ),
            stmt,
        ));
    };

    let value = match value_ty {
        Some(actual) => coerce(value, &actual, element).ok_or_else(|| {
            err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "this list holds '{}', but the appended value is '{}'",
                    element.python_name(),
                    actual.python_name()
                ),
                stmt,
            )
        })?,
        None => value,
    };

    Ok(Stmt::Append { sequence, value })
}

/// Lower a membership test, or its negation.
///
/// What membership means is the container's own: a sequence and a set test elements, a mapping
/// tests **keys**, and a string tests substrings. All three match Python, and the last two are not
/// what a reader expecting element membership would predict.
fn lower_membership(
    value: &PyExpr,
    container: &PyExpr,
    negated: bool,
    scope: &Scope,
    lowering: Lowering<'_>,
    node: &PyExpr,
) -> Result<(Expr, TyResult), LowerError> {
    let (value, value_ty) = lower_expr(value, scope, lowering)?;
    let (container, container_ty) = lower_expr(container, scope, lowering)?;

    if let Some(container_ty) = container_ty {
        let expected = match &container_ty {
            Ty::List(element) | Ty::Set(element) => (**element).clone(),
            Ty::Dict(key, _) => (**key).clone(),
            Ty::Str => Ty::Str,
            other => {
                return Err(err(
                    LowerErrorKind::TypeMismatch,
                    format!(
                        "'{}' does not support membership; only a list, dict, set, or str does",
                        other.python_name()
                    ),
                    node,
                ));
            }
        };
        if let Some(actual) = &value_ty
            && *actual != expected
        {
            return Err(err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "membership in this container tests '{}', but the value is '{}'",
                    expected.python_name(),
                    actual.python_name()
                ),
                node,
            ));
        }
    }

    let test = Expr::Contains {
        value: Box::new(value),
        container: Box::new(container),
    };
    // `not in` is the negation of a membership test rather than a second form, so nothing
    // consuming the IR has to remember to honour a flag.
    let node = if negated {
        Expr::Not(Box::new(test))
    } else {
        test
    };
    Ok((node, Some(Ty::Bool)))
}

fn lower_stmt(stmt: &PyStmt, scope: &mut Scope, ctx: Ctx<'_>) -> Result<Stmt, LowerError> {
    match stmt {
        PyStmt::Return(node) => match node.value.as_deref() {
            Some(value) => {
                let ret = ctx.ret;
                let (lowered, ty) = lower_expr(value, scope, ctx.lowering)?;
                if *ret == Ty::Unit {
                    return Err(err(
                        LowerErrorKind::TypeMismatch,
                        "function is declared to return no value, but a value is returned here",
                        stmt,
                    ));
                }
                // Only check when the type is determined; a returned call cannot be checked.
                match ty {
                    Some(actual) => Ok(Stmt::Return(coerce(lowered, &actual, ret).ok_or_else(
                        || {
                            err(
                                LowerErrorKind::TypeMismatch,
                                format!(
                                    "function returns '{}' but this expression is '{}'",
                                    ret.python_name(),
                                    actual.python_name()
                                ),
                                stmt,
                            )
                        },
                    )?)),
                    None => Ok(Stmt::Return(lowered)),
                }
            }
            None => Ok(Stmt::ReturnUnit),
        },
        // Filtered out before reaching here; a `pass` produces no statement at all.
        PyStmt::Pass(_) => unreachable!("`pass` is dropped by lower_body"),
        PyStmt::AnnAssign(assign) => lower_annotated_binding(stmt, assign, scope, ctx),
        PyStmt::Assign(assign) => {
            // `self.x = v` assigns an attribute the class already declares.
            if assign.targets.len() == 1
                && let PyExpr::Attribute(target) = &assign.targets[0]
            {
                return lower_set_attr(stmt, target, &assign.value, scope, ctx);
            }
            // A subscripted target is an element assignment, not a binding: the name keeps
            // denoting the same collection and one of its entries changes.
            if assign.targets.len() == 1
                && let PyExpr::Subscript(target) = &assign.targets[0]
            {
                return lower_set_item(stmt, target, &assign.value, scope, ctx);
            }
            lower_bare_binding(stmt, assign, scope, ctx)
        }
        PyStmt::Expr(statement) => lower_method_statement(stmt, &statement.value, scope, ctx),
        PyStmt::If(node) => {
            let test = lower_test(&node.test, scope, ctx)?;
            let then = lower_block(&node.body, scope, ctx)?;
            let otherwise = lower_clauses(&node.elif_else_clauses, scope, ctx)?;
            Ok(Stmt::If {
                test,
                then,
                otherwise,
            })
        }
        PyStmt::While(node) => {
            reject_loop_else(&node.orelse, stmt)?;
            let test = lower_test(&node.test, scope, ctx)?;
            let body = lower_block(&node.body, scope, ctx.inside_loop())?;
            Ok(Stmt::While { test, body })
        }
        PyStmt::For(node) => lower_for(stmt, node, scope, ctx),
        PyStmt::Break(_) => {
            reject_outside_loop("break", ctx, stmt)?;
            Ok(Stmt::Break)
        }
        PyStmt::Continue(_) => {
            reject_outside_loop("continue", ctx, stmt)?;
            Ok(Stmt::Continue)
        }
        PyStmt::AugAssign(_) => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "augmented assignment is not supported",
            stmt,
        )),
        other => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "unsupported statement",
            other,
        )),
    }
}

/// Adapt `expr` of type `actual` to the expected type, or `None` if it cannot be adapted.
///
/// The only adaptation is widening an integer to a float, which is Python's own promotion.
/// Narrowing a float to an integer is deliberately not offered: it would lose information
/// silently, which is exactly the kind of quiet wrongness this compiler is meant to avoid.
fn coerce(expr: Expr, actual: &Ty, expected: &Ty) -> Option<Expr> {
    if actual == expected {
        return Some(expr);
    }
    if *actual == Ty::Int && *expected == Ty::Float {
        return Some(expr.to_float());
    }
    None
}

/// Reject re-declaring a name that already exists.
///
/// Assigning to a name again is fine and lowers to [`Stmt::Assign`]; *annotating* it again is not.
/// `i: int = 1` after `i = 0` is a second declaration, and accepting it would raise the question of
/// whether the two annotations may differ — a question with no good answer, since a name whose type
/// changes partway through is one a reader has to simulate the program to follow.
fn ensure_undeclared(name: &str, scope: &Scope, node: &impl Ranged) -> Result<(), LowerError> {
    if scope.contains_key(name) {
        return Err(err(
            LowerErrorKind::Reassignment,
            format!(
                "'{name}' is already bound, so this annotation re-declares it; drop the \
                 annotation to assign to it instead"
            ),
            node,
        ));
    }
    Ok(())
}

fn binding_target<'a>(target: &'a PyExpr, node: &impl Ranged) -> Result<&'a str, LowerError> {
    match target {
        PyExpr::Name(name) => Ok(name.id.as_str()),
        _ => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "only simple name targets are supported in assignments",
            node,
        )),
    }
}

/// Check one argument against a declared parameter type, promoting where Python would.
fn check_argument(
    arg: Expr,
    actual: TyResult,
    expected: &Ty,
    callee: &str,
    node: &PyExpr,
) -> Result<Expr, LowerError> {
    match actual {
        // Undetermined is taken on trust; the unit checks it once every source is assembled.
        None => Ok(arg),
        Some(actual) => coerce(arg, &actual, expected).ok_or_else(|| {
            err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "'{callee}' expects '{}' here but was given '{}'",
                    expected.python_name(),
                    actual.python_name()
                ),
                node,
            )
        }),
    }
}

/// Lower `<receiver>.<method>(...)`.
fn lower_method_call(
    call: &ruff_python_ast::ExprCall,
    receiver: &ruff_python_ast::ExprAttribute,
    scope: &Scope,
    lowering: Lowering<'_>,
    expr: &PyExpr,
) -> Result<(Expr, TyResult), LowerError> {
    let method = receiver.attr.as_str();
    let (object, object_ty) = lower_expr(&receiver.value, scope, lowering)?;

    let mut args = Vec::with_capacity(call.arguments.args.len());
    let mut arg_types = Vec::with_capacity(call.arguments.args.len());
    for arg in &call.arguments.args {
        let (lowered, ty) = lower_expr(arg, scope, lowering)?;
        args.push(lowered);
        arg_types.push(ty);
    }

    let class_of = match &object_ty {
        Some(Ty::Instance(class)) => Some(class.clone()),
        _ => None,
    };
    let node = |args| Expr::MethodCall {
        receiver: Box::new(object.clone()),
        class: class_of.clone(),
        method: method.to_string(),
        args,
    };

    // Undetermined propagates: the receiver may be an instance of a class in another source.
    let Some(Ty::Instance(class)) = object_ty else {
        return Ok((node(args), None));
    };
    let Some(signature) = lowering.names.classes.get(&class) else {
        return Ok((node(args), None));
    };
    let Some(method_sig) = signature.methods.get(method) else {
        return Err(err(
            LowerErrorKind::Unresolved,
            format!("'{class}' has no method '{method}'"),
            expr,
        ));
    };
    if method_sig.params.len() != args.len() {
        return Err(err(
            LowerErrorKind::ArityMismatch,
            format!(
                "'{class}.{method}' expects {} argument(s) but {} were given",
                method_sig.params.len(),
                args.len()
            ),
            expr,
        ));
    }
    let mut checked = Vec::with_capacity(args.len());
    for ((arg, actual), expected) in args.into_iter().zip(arg_types).zip(&method_sig.params) {
        checked.push(check_argument(arg, actual, expected, method, expr)?);
    }
    Ok((node(checked), Some(method_sig.ret.clone())))
}

/// Lower `<object>.<name> = <value>`.
///
/// In `__init__` an annotated form declares the attribute; anywhere else the attribute must already
/// exist. A struct's fields cannot depend on which methods happened to run, which is why the
/// declaration has exactly one legal home.
fn lower_set_attr(
    stmt: &PyStmt,
    target: &ruff_python_ast::ExprAttribute,
    value: &PyExpr,
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Stmt, LowerError> {
    let (object, object_ty) = lower_expr(&target.value, scope, ctx.lowering)?;
    let attribute = target.attr.as_str();

    let Some(Ty::Instance(class)) = object_ty else {
        return Err(err(
            LowerErrorKind::TypeMismatch,
            "only an attribute of a class instance can be assigned",
            stmt,
        ));
    };

    let declared = ctx
        .lowering
        .names
        .classes
        .get(&class)
        .and_then(|signature| {
            signature
                .attributes
                .iter()
                .find(|a| a.name == attribute)
                .map(|a| a.ty.clone())
        })
        .ok_or_else(|| {
            // Inside the constructor an undeclared attribute is almost always a missing
            // annotation rather than a typo, and saying so points at the actual fix.
            let kind = if ctx.in_init {
                LowerErrorKind::MissingAnnotation
            } else {
                LowerErrorKind::Unresolved
            };
            err(
                kind,
                format!(
                    "'{class}' has no attribute '{attribute}'; declare it with an annotation in \
                     '__init__', which is the only place an attribute may be introduced"
                ),
                stmt,
            )
        })?;

    let (value, actual) = lower_expr(value, scope, ctx.lowering)?;
    let value = match actual {
        Some(actual) => coerce(value, &actual, &declared).ok_or_else(|| {
            err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "attribute '{attribute}' is '{}', but this assigns a value of type '{}'",
                    declared.python_name(),
                    actual.python_name()
                ),
                stmt,
            )
        })?,
        None => value,
    };

    Ok(Stmt::SetAttr {
        object,
        name: attribute.to_string(),
        ty: declared,
        value,
    })
}

/// Lower `self.<name>: T = <value>`, which declares an attribute.
fn lower_attribute_declaration(
    stmt: &PyStmt,
    assign: &ruff_python_ast::StmtAnnAssign,
    attribute: &str,
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Stmt, LowerError> {
    if !ctx.in_init {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "attribute '{attribute}' must be declared in '__init__'; declaring it elsewhere \
                 would make the class's shape depend on which methods happened to run"
            ),
            stmt,
        ));
    }
    let declared = lower_annotation(&assign.annotation, false, ctx.lowering.names.class_names)?;
    let Some(value) = assign.value.as_deref() else {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!("attribute '{attribute}' is declared without a value, which is not supported"),
            stmt,
        ));
    };
    let (value, actual) = lower_expr(value, scope, ctx.lowering)?;
    let value = match actual {
        Some(actual) => coerce(value, &actual, &declared).ok_or_else(|| {
            err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "attribute '{attribute}' is declared as '{}' but the value is '{}'",
                    declared.python_name(),
                    actual.python_name()
                ),
                stmt,
            )
        })?,
        None => value,
    };
    Ok(Stmt::SetAttr {
        object: Expr::name(SELF),
        name: attribute.to_string(),
        ty: declared,
        value,
    })
}

fn lower_annotated_binding(
    stmt: &PyStmt,
    assign: &ruff_python_ast::StmtAnnAssign,
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Stmt, LowerError> {
    if let Some(attribute) = self_attribute_name(&assign.target) {
        let attribute = attribute.to_string();
        return lower_attribute_declaration(stmt, assign, &attribute, scope, ctx);
    }
    let name = binding_target(&assign.target, stmt)?.to_string();
    ensure_undeclared(&name, scope, stmt)?;

    let declared = lower_annotation(&assign.annotation, false, ctx.lowering.names.class_names)?;
    let Some(value) = assign.value.as_deref() else {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!("'{name}' is declared without a value, which is not supported"),
            stmt,
        ));
    };
    let (lowered, actual) = lower_expr(value, scope, ctx.lowering)?;

    // An undetermined initializer cannot be checked; the declared type is taken on trust.
    let value = match actual {
        Some(actual) => coerce(lowered, &actual, &declared).ok_or_else(|| {
            err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "'{name}' is declared as '{}' but the value is '{}'",
                    declared.python_name(),
                    actual.python_name()
                ),
                stmt,
            )
        })?,
        None => lowered,
    };

    let origin = alias_origin(&value, &declared, scope, ctx);
    scope.declare(name.clone(), declared.clone(), origin);
    Ok(Stmt::Bind {
        name,
        ty: declared,
        value,
    })
}

fn lower_bare_binding(
    stmt: &PyStmt,
    assign: &ruff_python_ast::StmtAssign,
    scope: &mut Scope,
    ctx: Ctx<'_>,
) -> Result<Stmt, LowerError> {
    if assign.targets.len() != 1 {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "chained assignment is not supported",
            stmt,
        ));
    }
    let name = binding_target(&assign.targets[0], stmt)?.to_string();

    // A name already visible is being *re*assigned. Its type was fixed where it was first bound,
    // so the value is checked against that rather than inferring a fresh one — the alternative is
    // a name that denotes different things at different points in the same function.
    if let Some(existing) = scope.get(&name).cloned() {
        let (value, actual) = lower_expr(&assign.value, scope, ctx.lowering)?;
        let value = match actual {
            Some(actual) => coerce(value, &actual, &existing).ok_or_else(|| {
                err(
                    LowerErrorKind::TypeMismatch,
                    format!(
                        "'{name}' is '{}', but this assigns a value of type '{}'",
                        existing.python_name(),
                        actual.python_name()
                    ),
                    stmt,
                )
            })?,
            None => value,
        };
        // Reassignment changes where the name's value came from, so the origin moves with it.
        let origin = alias_origin(&value, &existing, scope, ctx);
        scope.set_origin(&name, origin);
        return Ok(Stmt::Assign {
            name,
            ty: existing,
            value,
        });
    }

    let (value, inferred) = lower_expr(&assign.value, scope, ctx.lowering)?;

    // Infer when the initializer's type is determined; otherwise the answer is genuinely
    // unknown here and an annotation is the only way to supply it.
    let Some(ty) = inferred else {
        return Err(err(
            LowerErrorKind::UndeterminedBinding,
            format!(
                "'{name}' needs an explicit type annotation: its value contains a call to a \
                 function this source does not define"
            ),
            stmt,
        ));
    };

    let origin = alias_origin(&value, &ty, scope, ctx);
    scope.declare(name.clone(), ty.clone(), origin);
    Ok(Stmt::Bind { name, ty, value })
}

/// Result type of an arithmetic operator applied to two determined operand types.
fn arithmetic_result(op: BinOp, left: &Ty, right: &Ty) -> Option<Ty> {
    // Python's `+` is overloaded on strings; every other arithmetic operator is numeric only.
    if matches!(op, BinOp::Add { .. }) && *left == Ty::Str && *right == Ty::Str {
        return Some(Ty::Str);
    }
    if !left.is_numeric() || !right.is_numeric() {
        return None;
    }
    // Exact division always yields a float, even for two integers. This is the single most
    // likely place for a backend to be accidentally wrong, which is why the node says so rather
    // than leaving a backend to infer it from the operator's name.
    if matches!(
        op,
        BinOp::Div {
            mode: DivMode::Exact,
            ..
        }
    ) {
        return Some(Ty::Float);
    }
    if *left == Ty::Float || *right == Ty::Float {
        Some(Ty::Float)
    } else {
        Some(Ty::Int)
    }
}

/// Build a typed binary expression, applying promotion and rejecting invalid operand types.
fn build_binary(
    op: BinOp,
    left: Expr,
    left_ty: &Ty,
    right: Expr,
    right_ty: &Ty,
    node: &impl Ranged,
) -> Result<(Expr, Ty), LowerError> {
    let mismatch = |extra: &str| {
        err(
            LowerErrorKind::TypeMismatch,
            format!(
                "operator '{}' is not defined for '{}' and '{}'{extra}",
                op.python_symbol(),
                left_ty.python_name(),
                right_ty.python_name()
            ),
            node,
        )
    };

    if op.is_comparison() {
        // Comparison operands must agree, except that numbers compare across int and float.
        let operand = if left_ty == right_ty {
            left_ty.clone()
        } else if left_ty.is_numeric() && right_ty.is_numeric() {
            Ty::Float
        } else {
            return Err(mismatch(""));
        };
        let left = coerce(left, left_ty, &operand).ok_or_else(|| mismatch(""))?;
        let right = coerce(right, right_ty, &operand).ok_or_else(|| mismatch(""))?;
        return Ok((Expr::binary(op, left, right), Ty::Bool));
    }

    let result = arithmetic_result(op, left_ty, right_ty).ok_or_else(|| {
        if *left_ty == Ty::Bool || *right_ty == Ty::Bool {
            mismatch("; booleans are not numbers in compylr")
        } else {
            mismatch("")
        }
    })?;

    // Operands are widened to the result type so a backend can emit them positionally.
    let operand = if result == Ty::Str {
        Ty::Str
    } else {
        result.clone()
    };
    let left = coerce(left, left_ty, &operand).ok_or_else(|| mismatch(""))?;
    let right = coerce(right, right_ty, &operand).ok_or_else(|| mismatch(""))?;
    Ok((Expr::binary(op, left, right), result))
}

/// Lower an expression and determine its type in one traversal.
///
/// Shape and type are produced together so they cannot be computed from different traversals
/// and disagree about what an expression means.
fn lower_expr(
    expr: &PyExpr,
    scope: &Scope,
    lowering: Lowering<'_>,
) -> Result<(Expr, TyResult), LowerError> {
    match expr {
        PyExpr::NumberLiteral(literal) => match &literal.value {
            Number::Int(value) => match value.as_i64() {
                Some(int) => Ok((Expr::int(int), Some(Ty::Int))),
                // `Int::as_i64` returns None beyond the 64-bit range. Truncating would
                // silently change the program's meaning, so this is an error.
                None => Err(err(
                    LowerErrorKind::LiteralOutOfRange,
                    "integer literal is too large for a 64-bit signed integer",
                    expr,
                )),
            },
            Number::Float(value) => Ok((Expr::float(*value), Some(Ty::Float))),
            Number::Complex { .. } => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "complex literals are not supported",
                expr,
            )),
        },
        PyExpr::BooleanLiteral(literal) => Ok((Expr::bool(literal.value), Some(Ty::Bool))),
        PyExpr::StringLiteral(literal) => Ok((Expr::string(literal.value.to_str()), Some(Ty::Str))),
        PyExpr::FString(_) => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "f-strings are not supported",
            expr,
        )),
        PyExpr::Name(name) => {
            let id = name.id.as_str();
            match scope.get(id) {
                Some(ty) => Ok((Expr::name(id), Some(ty.clone()))),
                None if scope.was_bound_in_a_departed_block(id) => Err(err(
                    LowerErrorKind::Unresolved,
                    format!(
                        "'{id}' is bound inside a branch or loop, so it may not have been bound \
                         by the time it is read here; bind it before the block instead"
                    ),
                    expr,
                )),
                None => Err(err(
                    LowerErrorKind::Unresolved,
                    format!("'{id}' is not defined"),
                    expr,
                )),
            }
        }
        PyExpr::UnaryOp(unary) => match unary.op {
            UnaryOp::USub => {
                let (operand, ty) = lower_expr(&unary.operand, scope, lowering)?;
                match ty {
                    Some(ty) if ty.is_numeric() => Ok((
                        Expr::Neg {
                            value: Box::new(operand),
                            checked: lowering.behavior.arithmetic(),
                        },
                        Some(ty),
                    )),
                    Some(ty) => Err(err(
                        LowerErrorKind::TypeMismatch,
                        format!("cannot negate a value of type '{}'", ty.python_name()),
                        expr,
                    )),
                    None => Ok((
                        Expr::Neg {
                            value: Box::new(operand),
                            checked: lowering.behavior.arithmetic(),
                        },
                        None,
                    )),
                }
            }
            UnaryOp::UAdd => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "unary '+' is not supported",
                expr,
            )),
            UnaryOp::Not => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "'not' is not supported",
                expr,
            )),
            UnaryOp::Invert => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "bitwise inversion is not supported",
                expr,
            )),
        },
        PyExpr::BinOp(binary) => {
            let op = match binary.op {
                Operator::Add => BinOp::Add {
                    checked: lowering.behavior.arithmetic(),
                },
                Operator::Sub => BinOp::Sub {
                    checked: lowering.behavior.arithmetic(),
                },
                Operator::Mult => BinOp::Mul {
                    checked: lowering.behavior.arithmetic(),
                },
                Operator::Div => lowering.behavior.exact_division(),
                Operator::FloorDiv => lowering.behavior.integer_division(),
                Operator::Mod => lowering.behavior.remainder(),
                other => {
                    return Err(err(
                        LowerErrorKind::UnsupportedConstruct,
                        format!("operator '{}' is not supported", other.as_str()),
                        expr,
                    ));
                }
            };
            let (left, left_ty) = lower_expr(&binary.left, scope, lowering)?;
            let (right, right_ty) = lower_expr(&binary.right, scope, lowering)?;
            match (left_ty, right_ty) {
                (Some(l), Some(r)) => {
                    let (node, ty) = build_binary(op, left, &l, right, &r, expr)?;
                    Ok((node, Some(ty)))
                }
                // Undetermined propagates outward rather than becoming a type error.
                _ => Ok((Expr::binary(op, left, right), None)),
            }
        }
        PyExpr::Compare(compare) => {
            if compare.ops.len() != 1 || compare.comparators.len() != 1 {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "chained comparisons are not supported",
                    expr,
                ));
            }
            if matches!(compare.ops[0], CmpOp::In | CmpOp::NotIn) {
                return lower_membership(
                    &compare.left,
                    &compare.comparators[0],
                    matches!(compare.ops[0], CmpOp::NotIn),
                    scope,
                    lowering,
                    expr,
                );
            }
            let op = match compare.ops[0] {
                CmpOp::Eq => BinOp::Eq,
                CmpOp::NotEq => BinOp::NotEq,
                CmpOp::Lt => BinOp::Lt,
                CmpOp::LtE => BinOp::LtE,
                CmpOp::Gt => BinOp::Gt,
                CmpOp::GtE => BinOp::GtE,
                other => {
                    return Err(err(
                        LowerErrorKind::UnsupportedConstruct,
                        format!("comparison '{}' is not supported", other.as_str()),
                        expr,
                    ));
                }
            };
            let (left, left_ty) = lower_expr(&compare.left, scope, lowering)?;
            let (right, right_ty) = lower_expr(&compare.comparators[0], scope, lowering)?;
            match (left_ty, right_ty) {
                (Some(l), Some(r)) => {
                    let (node, ty) = build_binary(op, left, &l, right, &r, expr)?;
                    Ok((node, Some(ty)))
                }
                _ => Ok((Expr::binary(op, left, right), None)),
            }
        }
        PyExpr::List(list) => {
            let (items, element) = unify_elements(&list.elts, scope, lowering, expr, "list")?;
            Ok((
                Expr::ListLit(items),
                element.map(|ty| Ty::List(Box::new(ty))),
            ))
        }
        PyExpr::Set(set) => {
            let (items, element) = unify_elements(&set.elts, scope, lowering, expr, "set")?;
            let element = match element {
                Some(ty) if !ty.can_key() => {
                    return Err(err(
                        LowerErrorKind::UnsupportedType,
                        format!(
                            "'{}' cannot be a set element: only int, str, and bool can be \
                             compared and hashed",
                            ty.python_name()
                        ),
                        expr,
                    ));
                }
                other => other,
            };
            Ok((Expr::SetLit(items), element.map(|ty| Ty::Set(Box::new(ty)))))
        }
        PyExpr::Tuple(tuple) => {
            // A type per position, so nothing is unified: elements need not agree.
            let mut items = Vec::with_capacity(tuple.elts.len());
            let mut types = Vec::with_capacity(tuple.elts.len());
            let mut determined = true;
            for element in &tuple.elts {
                let (lowered, ty) = lower_expr(element, scope, lowering)?;
                items.push(lowered);
                match ty {
                    Some(ty) => types.push(ty),
                    None => determined = false,
                }
            }
            let ty = if determined && !items.is_empty() {
                Some(Ty::Tuple(types))
            } else {
                None
            };
            Ok((Expr::TupleLit(items), ty))
        }
        PyExpr::Dict(dict) => {
            let mut pairs = Vec::with_capacity(dict.items.len());
            let mut keys = Vec::with_capacity(dict.items.len());
            let mut values = Vec::with_capacity(dict.items.len());
            for item in &dict.items {
                let Some(key_expr) = item.key.as_ref() else {
                    return Err(err(
                        LowerErrorKind::UnsupportedConstruct,
                        "dictionary unpacking is not supported",
                        expr,
                    ));
                };
                let (key, key_ty) = lower_expr(key_expr, scope, lowering)?;
                let (value, value_ty) = lower_expr(&item.value, scope, lowering)?;
                pairs.push((key, value));
                keys.push(key_ty);
                values.push(value_ty);
            }
            let key_ty = agree(&keys, expr, "mapping key")?;
            let value_ty = agree(&values, expr, "mapping value")?;
            if let Some(key) = &key_ty
                && !key.can_key()
            {
                return Err(err(
                    LowerErrorKind::UnsupportedType,
                    format!(
                        "'{}' cannot be a mapping key: only int, str, and bool can be compared \
                         and hashed",
                        key.python_name()
                    ),
                    expr,
                ));
            }
            let ty = match (key_ty, value_ty) {
                (Some(key), Some(value)) if !pairs.is_empty() => {
                    Some(Ty::Dict(Box::new(key), Box::new(value)))
                }
                _ => None,
            };
            Ok((Expr::DictLit(pairs), ty))
        }
        PyExpr::Subscript(subscript) => lower_subscript(subscript, scope, lowering, expr),
        PyExpr::Attribute(attribute) => {
            let (object, object_ty) = lower_expr(&attribute.value, scope, lowering)?;
            let node = Expr::Attribute {
                object: Box::new(object),
                name: attribute.attr.to_string(),
            };
            // Undetermined propagates: the object may be an instance of a class in another source.
            let Some(Ty::Instance(class)) = object_ty else {
                return Ok((node, None));
            };
            let Some(signature) = lowering.names.classes.get(&class) else {
                return Ok((node, None));
            };
            let ty = signature
                .attributes
                .iter()
                .find(|a| a.name == attribute.attr.as_str())
                .map(|a| a.ty.clone())
                .ok_or_else(|| {
                    err(
                        LowerErrorKind::Unresolved,
                        format!(
                            "'{class}' has no attribute '{}'; every attribute must be declared \
                             with an annotation in '__init__'",
                            attribute.attr
                        ),
                        expr,
                    )
                })?;
            Ok((node, Some(ty)))
        }
        PyExpr::Call(call) => {
            if !call.arguments.keywords.is_empty() {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "keyword arguments are not supported",
                    expr,
                ));
            }
            if let PyExpr::Attribute(receiver) = call.func.as_ref() {
                return lower_method_call(call, receiver, scope, lowering, expr);
            }
            let PyExpr::Name(callee) = call.func.as_ref() else {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "only calls to plain function names are supported",
                    expr,
                ));
            };
            let mut args = Vec::with_capacity(call.arguments.args.len());
            let mut arg_types = Vec::with_capacity(call.arguments.args.len());
            for arg in &call.arguments.args {
                let (lowered, ty) = lower_expr(arg, scope, lowering)?;
                args.push(lowered);
                arg_types.push(ty);
            }

            let name = callee.id.as_str();

            // A class name is a construction, not a call. Unit validation would otherwise try to
            // resolve it against functions, and the type rules differ enough that one form would
            // make each path carry the other's cases.
            if let Some(signature) = lowering.names.classes.get(name) {
                if signature.init.len() != args.len() {
                    return Err(err(
                        LowerErrorKind::ArityMismatch,
                        format!(
                            "'{name}' takes {} constructor argument(s) but {} were given",
                            signature.init.len(),
                            args.len()
                        ),
                        expr,
                    ));
                }
                let mut checked = Vec::with_capacity(args.len());
                for ((arg, actual), expected) in
                    args.into_iter().zip(arg_types).zip(signature.init.iter())
                {
                    checked.push(check_argument(arg, actual, expected, name, expr)?);
                }
                return Ok((
                    Expr::Construct {
                        class: name.to_string(),
                        args: checked,
                    },
                    Some(Ty::Instance(name.to_string())),
                ));
            }

            // A range is only meaningful as something to iterate: there is no range value in the
            // subset, so `r = range(n)` has nothing to bind. Caught here rather than left to
            // resolve as an unknown function, so the diagnostic says what is actually wrong.
            if name == RANGE {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "'range' can only be used as what a 'for' loop iterates",
                    expr,
                ));
            }

            // `len` is a builtin, lowered to its own node rather than resolved against the unit.
            // Left as a call it would mean different things depending on whether someone had
            // decorated a function of that name, which is the order-dependence the unit's design
            // exists to prevent. The name is reserved to make that impossible.
            if name == "len" {
                if args.len() != 1 {
                    return Err(err(
                        LowerErrorKind::ArityMismatch,
                        format!(
                            "'len' takes exactly one argument but {} were given",
                            args.len()
                        ),
                        expr,
                    ));
                }
                let operand = args.remove(0);
                let ty = arg_types.remove(0);
                return match ty {
                    // A tuple's length is known here, so it is folded to a literal and never
                    // reaches the backend as a runtime query.
                    Some(Ty::Tuple(elements)) => {
                        Ok((Expr::int(elements.len() as i64), Some(Ty::Int)))
                    }
                    Some(Ty::List(_) | Ty::Dict(_, _) | Ty::Set(_) | Ty::Str) | None => Ok((
                        Expr::Len {
                            value: Box::new(operand),
                            units: lowering.behavior.text_units(),
                        },
                        Some(Ty::Int),
                    )),
                    Some(other) => Err(err(
                        LowerErrorKind::TypeMismatch,
                        format!("'len' is not defined for '{}'", other.python_name()),
                        expr,
                    )),
                };
            }

            let Some(signature) = lowering.names.sigs.get(name) else {
                // The callee is defined in another source, which lowering cannot see: it handles
                // one source at a time, and a decorated function may legitimately call one in a
                // module that has not been marked yet. Rejecting here would make acceptance
                // depend on decoration order. The type stays undetermined, and
                // `Unit::validate` catches a callee that exists nowhere at all.
                return Ok((
                    Expr::Call {
                        callee: name.to_string(),
                        args,
                    },
                    None,
                ));
            };

            if signature.params.len() != args.len() {
                return Err(err(
                    LowerErrorKind::ArityMismatch,
                    format!(
                        "'{name}' takes {} argument{} but {} {} given",
                        signature.params.len(),
                        if signature.params.len() == 1 { "" } else { "s" },
                        args.len(),
                        if args.len() == 1 { "was" } else { "were" }
                    ),
                    expr,
                ));
            }

            // Each argument is checked against the declared parameter type, with promotion, so an
            // integer passed where a float is declared carries an explicit conversion rather than
            // leaving a backend to notice. An undetermined argument cannot be checked.
            for (index, (declared, actual)) in signature.params.iter().zip(&arg_types).enumerate() {
                let Some(actual) = actual else { continue };
                let taken = std::mem::replace(&mut args[index], Expr::Name(String::new()));
                args[index] = coerce(taken, actual, declared).ok_or_else(|| {
                    err(
                        LowerErrorKind::TypeMismatch,
                        format!(
                            "argument {} of '{name}' is declared as '{}' but the value is '{}'",
                            index + 1,
                            declared.python_name(),
                            actual.python_name()
                        ),
                        expr,
                    )
                })?;
            }

            Ok((
                Expr::Call {
                    callee: name.to_string(),
                    args,
                },
                Some(signature.ret.clone()),
            ))
        }
        other => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "unsupported expression",
            other,
        )),
    }
}
#[cfg(test)]
mod tests {
    use compylr_ir::Checked;

    use super::*;
    use crate::frontend::parse_source;

    /// Python's own stance, which is what an unconfigured compilation resolves to.
    ///
    /// Read from the frontend's declaration rather than rebuilt here, so these tests exercise the
    /// same bundle the pipeline uses. A local copy would keep passing after the declaration
    /// changed, which is the whole failure the declaration exists to prevent.
    fn python() -> Behavior {
        Behavior::of(&crate::component::PYTHON_BEHAVIOR)
    }

    fn lower(source: &str) -> Result<Vec<Function>, LowerError> {
        let parsed = parse_source(source).expect("fixture must parse");
        lower_source(&parsed, python())
    }

    fn lower_one(source: &str) -> Function {
        let mut functions = lower(source).expect("expected lowering to succeed");
        assert_eq!(functions.len(), 1);
        functions.remove(0)
    }

    fn error_for(source: &str) -> LowerError {
        lower(source).expect_err("expected lowering to fail")
    }

    // ---- happy path -------------------------------------------------------

    #[test]
    fn lowers_a_simple_annotated_function() {
        let f = lower_one("def add(a: int, b: int) -> int:\n    return a + b\n");
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].ty, Ty::Int);
        assert_eq!(f.ret, Ty::Int);
        assert_eq!(
            f.body,
            vec![Stmt::Return(Expr::binary(
                BinOp::Add {
                    checked: Checked::Reported
                },
                Expr::name("a"),
                Expr::name("b")
            ))]
        );
    }

    #[test]
    fn preserves_function_order_within_a_source() {
        let functions = lower(
            "def a() -> None:\n    pass\ndef b() -> None:\n    pass\ndef c() -> None:\n    pass\n",
        )
        .unwrap();
        let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn empty_source_lowers_to_no_functions() {
        assert!(lower("").unwrap().is_empty());
    }

    #[test]
    fn covers_bindings_arithmetic_comparison_strings_and_calls() {
        let f = lower_one(
            "def f(a: int) -> bool:\n\
             \x20   x: int = a * 2\n\
             \x20   y: str = \"hi\"\n\
             \x20   z: int = helper(x)\n\
             \x20   return z >= 0\n",
        );
        assert_eq!(f.body.len(), 4);
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
        assert!(matches!(f.body[1], Stmt::Bind { ty: Ty::Str, .. }));
        assert!(matches!(f.body[2], Stmt::Bind { ty: Ty::Int, .. }));
        assert!(matches!(
            f.body[3],
            Stmt::Return(Expr::Binary { op: BinOp::GtE, .. })
        ));
    }

    #[test]
    fn bare_return_lowers_to_a_unit_statement() {
        let g = lower_one("def g() -> None:\n    return\n");
        assert_eq!(g.body, vec![Stmt::ReturnUnit]);
    }

    #[test]
    fn pass_produces_no_statement() {
        // It carries no meaning, and lowering it to a return would give `for i in ...: pass` a
        // body that exits the function.
        let f = lower_one("def f() -> None:\n    pass\n");
        assert!(f.body.is_empty());

        let loop_body = lower_one("def f(n: int) -> None:\n    for i in range(n):\n        pass\n");
        let Stmt::For { body, .. } = &loop_body.body[0] else {
            panic!("expected a loop");
        };
        assert!(body.is_empty());
    }

    #[test]
    fn all_supported_operators_lower() {
        for (symbol, expected) in [
            (
                "+",
                BinOp::Add {
                    checked: Checked::Reported,
                },
            ),
            (
                "-",
                BinOp::Sub {
                    checked: Checked::Reported,
                },
            ),
            (
                "*",
                BinOp::Mul {
                    checked: Checked::Reported,
                },
            ),
            ("//", python().integer_division()),
            ("%", python().remainder()),
        ] {
            let f = lower_one(&format!(
                "def f(a: int, b: int) -> int:\n    return a {symbol} b\n"
            ));
            match &f.body[0] {
                Stmt::Return(Expr::Binary { op, .. }) => assert_eq!(*op, expected),
                other => panic!("expected binary for {symbol}, got {other:?}"),
            }
        }
        for (symbol, expected) in [
            ("==", BinOp::Eq),
            ("!=", BinOp::NotEq),
            ("<", BinOp::Lt),
            ("<=", BinOp::LtE),
            (">", BinOp::Gt),
            (">=", BinOp::GtE),
        ] {
            let f = lower_one(&format!(
                "def f(a: int, b: int) -> bool:\n    return a {symbol} b\n"
            ));
            match &f.body[0] {
                Stmt::Return(Expr::Binary { op, .. }) => assert_eq!(*op, expected),
                other => panic!("expected comparison for {symbol}, got {other:?}"),
            }
        }
    }

    #[test]
    fn literals_and_negation_lower() {
        let f = lower_one("def f() -> int:\n    return -7\n");
        assert_eq!(
            f.body[0],
            Stmt::Return(Expr::Neg {
                value: Box::new(Expr::int(7)),
                checked: Checked::Reported,
            })
        );

        let g = lower_one("def g() -> bool:\n    return True\n");
        assert_eq!(g.body[0], Stmt::Return(Expr::bool(true)));

        let h = lower_one("def h() -> str:\n    return \"hello\"\n");
        assert_eq!(h.body[0], Stmt::Return(Expr::string("hello")));
    }

    #[test]
    fn none_is_accepted_as_a_return_annotation() {
        assert_eq!(lower_one("def f() -> None:\n    pass\n").ret, Ty::Unit);
    }

    // ---- annotations ------------------------------------------------------

    #[test]
    fn unannotated_parameter_is_rejected() {
        let error = error_for("def add(a, b: int) -> int:\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
        assert!(error.message().contains('a'));
    }

    #[test]
    fn missing_return_annotation_is_rejected() {
        let error = error_for("def add(a: int, b: int):\n    return a\n");
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
        assert!(error.message().contains("add"));
    }

    #[test]
    fn unsupported_annotations_are_rejected() {
        let complex = error_for("def f(a: complex) -> int:\n    return 1\n");
        assert_eq!(complex.kind(), LowerErrorKind::UnsupportedType);
        assert!(complex.message().contains("complex"));

        // `list[int]` is supported now; an unsupported *parameter* is still rejected, and so is
        // a generic compylr does not model.
        let bad_parameter = error_for("def f(a: list[complex]) -> int:\n    return 1\n");
        assert_eq!(bad_parameter.kind(), LowerErrorKind::UnsupportedType);

        let unknown_generic = error_for("def f(a: frozenset[int]) -> int:\n    return 1\n");
        assert_eq!(unknown_generic.kind(), LowerErrorKind::UnsupportedType);

        let none_param = error_for("def f(a: None) -> int:\n    return 1\n");
        assert_eq!(none_param.kind(), LowerErrorKind::UnsupportedType);
        assert!(none_param.message().contains("return"));
    }

    #[test]
    fn type_parameters_are_rejected() {
        let error = error_for("def f[T](a: T) -> T:\n    return a\n");
        assert_eq!(error.kind(), LowerErrorKind::UnsupportedType);
        assert!(error.message().contains("type parameters"));
    }

    #[test]
    fn non_simple_parameter_forms_are_rejected() {
        for source in [
            "def f(*args: int) -> int:\n    return 1\n",
            "def f(**kwargs: int) -> int:\n    return 1\n",
            "def f(*, a: int) -> int:\n    return a\n",
            "def f(a: int, /) -> int:\n    return a\n",
            "def f(a: int = 1) -> int:\n    return a\n",
        ] {
            let error = error_for(source);
            assert_eq!(
                error.kind(),
                LowerErrorKind::UnsupportedConstruct,
                "source should be rejected: {source}"
            );
        }
    }

    #[test]
    fn decorated_and_async_functions_are_rejected() {
        let decorated = error_for("@cache\ndef f() -> None:\n    pass\n");
        assert_eq!(decorated.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(decorated.message().contains("decorator"));

        let asynchronous = error_for("async def f() -> None:\n    pass\n");
        assert_eq!(asynchronous.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(asynchronous.message().contains("async"));
    }

    // ---- constructs outside the subset ------------------------------------

    #[test]
    fn control_flow_needs_a_boolean_test() {
        // Control flow itself lowers now; what is still rejected is inferring truthiness from a
        // value that is not a boolean.
        let conditional =
            error_for("def f(a: int) -> int:\n    if a:\n        return 1\n    return 0\n");
        assert_eq!(conditional.kind(), LowerErrorKind::TypeMismatch);

        let loop_stmt =
            error_for("def f(a: int) -> int:\n    while a:\n        pass\n    return 0\n");
        assert_eq!(loop_stmt.kind(), LowerErrorKind::TypeMismatch);
    }

    #[test]
    fn top_level_statements_are_rejected() {
        let guard =
            error_for("def main() -> None:\n    pass\nif __name__ == '__main__':\n    main()\n");
        assert_eq!(guard.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(guard.message().contains("top level"));

        let import = error_for("import os\n");
        assert_eq!(import.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(import.message().contains("import"));

        let class = error_for("class C:\n    pass\n");
        assert_eq!(class.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(class.message().contains("class"));
    }

    #[test]
    fn unsupported_operators_are_rejected() {
        let power = error_for("def f(a: int, b: int) -> int:\n    return a ** b\n");
        assert_eq!(power.kind(), LowerErrorKind::UnsupportedConstruct);
    }

    #[test]
    fn out_of_range_integer_literal_is_rejected() {
        let error = error_for(&format!("def f() -> int:\n    return {}\n", "9".repeat(40)));
        assert_eq!(error.kind(), LowerErrorKind::LiteralOutOfRange);
    }

    // ---- name resolution --------------------------------------------------

    #[test]
    fn parameters_and_prior_locals_resolve() {
        let f = lower_one("def f(a: int) -> int:\n    x: int = a + 1\n    return x\n");
        assert_eq!(f.body.len(), 2);
    }

    #[test]
    fn unbound_name_is_rejected() {
        let error = error_for("def f() -> int:\n    return q\n");
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
        assert!(error.message().contains('q'));
    }

    #[test]
    fn reference_before_binding_is_rejected() {
        let error = error_for("def f() -> int:\n    y: int = x\n    x: int = 1\n    return y\n");
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
    }

    #[test]
    fn rebinding_a_local_or_parameter_is_rejected() {
        let local = error_for("def f() -> int:\n    x: int = 1\n    x: int = 2\n    return x\n");
        assert_eq!(local.kind(), LowerErrorKind::Reassignment);

        let parameter = error_for("def f(a: int) -> int:\n    a: int = 2\n    return a\n");
        assert_eq!(parameter.kind(), LowerErrorKind::Reassignment);
    }

    // ---- alias inference --------------------------------------------------

    #[test]
    fn alias_of_a_parameter_is_inferred() {
        let f = lower_one("def foo(a: int) -> int:\n    b = a\n    return b\n");
        assert_eq!(
            f.body[0],
            Stmt::Bind {
                name: "b".into(),
                ty: Ty::Int,
                value: Expr::name("a"),
            }
        );
    }

    #[test]
    fn alias_of_a_prior_local_is_inferred() {
        let f = lower_one("def f() -> str:\n    x: str = \"hi\"\n    y = x\n    return y\n");
        assert!(matches!(f.body[1], Stmt::Bind { ty: Ty::Str, .. }));
    }

    #[test]
    fn chained_aliases_are_inferred() {
        let f = lower_one("def f(a: bool) -> bool:\n    b = a\n    c = b\n    return c\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Bool, .. }));
        assert!(matches!(f.body[1], Stmt::Bind { ty: Ty::Bool, .. }));
    }

    #[test]
    fn unannotated_binding_from_a_literal_is_inferred() {
        let f = lower_one("def f() -> int:\n    x = 1\n    return x\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    #[test]
    fn unannotated_binding_from_an_expression_is_inferred() {
        let f = lower_one("def f(a: int) -> int:\n    b = a + 1\n    return b\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    #[test]
    fn unannotated_binding_from_a_call_is_rejected() {
        let error = error_for("def f(a: int) -> int:\n    b = helper(a)\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::UndeterminedBinding);
    }

    #[test]
    fn alias_of_an_unbound_name_reports_unresolved_not_missing_annotation() {
        let error = error_for("def f() -> int:\n    b = q\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
        assert!(error.message().contains('q'));
    }

    #[test]
    fn explicit_annotation_still_wins_over_inference() {
        let f = lower_one("def f(a: int) -> int:\n    b: int = a\n    return b\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    #[test]
    fn annotation_conflicting_with_the_aliased_type_is_rejected() {
        let error = error_for("def f(a: int) -> str:\n    b: str = a\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::TypeMismatch);
        assert!(error.message().contains("str") && error.message().contains("int"));
    }

    // ---- literal and expression inference ---------------------------------

    /// Type a binding by name in the first function of a source.
    fn bound_ty(source: &str, want: &str) -> Ty {
        let f = lower_one(source);
        for stmt in &f.body {
            if let Stmt::Bind { name, ty, .. } = stmt
                && name == want
            {
                return ty.clone();
            }
        }
        panic!("no binding named {want}");
    }

    #[test]
    fn literal_initializers_are_inferred() {
        // The motivating cases from the proposal.
        assert_eq!(
            bound_ty("def f() -> str:\n    a = \"x\"\n    return a\n", "a"),
            Ty::Str
        );
        assert_eq!(
            bound_ty("def f() -> int:\n    b = 3\n    return b\n", "b"),
            Ty::Int
        );
        assert_eq!(
            bound_ty("def f() -> float:\n    c = 1.3\n    return c\n", "c"),
            Ty::Float
        );
        assert_eq!(
            bound_ty("def f() -> bool:\n    d = True\n    return d\n", "d"),
            Ty::Bool
        );
    }

    #[test]
    fn expression_initializers_are_inferred() {
        assert_eq!(
            bound_ty("def f(a: int) -> int:\n    b = a + 1\n    return b\n", "b"),
            Ty::Int
        );
        assert_eq!(
            bound_ty(
                "def f(a: int) -> bool:\n    b = a < 10\n    return b\n",
                "b"
            ),
            Ty::Bool
        );
        assert_eq!(
            bound_ty("def f(c: float) -> float:\n    b = -c\n    return b\n", "b"),
            Ty::Float
        );
        assert_eq!(
            bound_ty(
                "def f(a: int) -> int:\n    b = (a + 1) * 2 - 3\n    return b\n",
                "b"
            ),
            Ty::Int
        );
    }

    #[test]
    fn true_division_yields_float_while_floor_division_stays_int() {
        assert_eq!(
            bound_ty(
                "def f(a: int, b: int) -> float:\n    q = a / b\n    return q\n",
                "q"
            ),
            Ty::Float
        );
        assert_eq!(
            bound_ty(
                "def f(a: int, b: int) -> int:\n    q = a // b\n    return q\n",
                "q"
            ),
            Ty::Int
        );
    }

    #[test]
    fn string_concatenation_is_inferred() {
        assert_eq!(
            bound_ty(
                "def f(a: str, b: str) -> str:\n    c = a + b\n    return c\n",
                "c"
            ),
            Ty::Str
        );
    }

    #[test]
    fn mixed_arithmetic_promotes_and_records_the_conversion() {
        let f = lower_one("def f(a: int, b: float) -> float:\n    c = a + b\n    return c\n");
        match &f.body[0] {
            Stmt::Bind { ty, value, .. } => {
                assert_eq!(*ty, Ty::Float);
                // The integer operand must be wrapped, or a backend emitting operands
                // positionally would produce integer arithmetic.
                match value {
                    Expr::Binary { left, right, .. } => {
                        assert!(
                            matches!(**left, Expr::ToFloat(_)),
                            "int operand should be promoted, got {left:?}"
                        );
                        assert!(matches!(**right, Expr::Name(_)));
                    }
                    other => panic!("expected binary, got {other:?}"),
                }
            }
            other => panic!("expected bind, got {other:?}"),
        }
    }

    #[test]
    fn true_division_of_two_ints_promotes_both_operands() {
        let f = lower_one("def f(a: int, b: int) -> float:\n    q = a / b\n    return q\n");
        match &f.body[0] {
            Stmt::Bind {
                value: Expr::Binary { op, left, right },
                ..
            } => {
                assert_eq!(*op, python().exact_division());
                assert!(matches!(**left, Expr::ToFloat(_)));
                assert!(matches!(**right, Expr::ToFloat(_)));
            }
            other => panic!("expected binary bind, got {other:?}"),
        }
    }

    #[test]
    fn mixed_comparison_is_permitted_and_yields_bool() {
        assert_eq!(
            bound_ty(
                "def f(a: int, b: float) -> bool:\n    c = a < b\n    return c\n",
                "c"
            ),
            Ty::Bool
        );
    }

    #[test]
    fn ill_typed_operands_are_rejected() {
        for (source, note) in [
            (
                "def f(a: str, b: int) -> str:\n    c = a + b\n    return c\n",
                "str + int",
            ),
            (
                "def f(a: bool, b: bool) -> int:\n    c = a + b\n    return c\n",
                "bool arithmetic",
            ),
            (
                "def f(a: str) -> str:\n    c = -a\n    return c\n",
                "negate str",
            ),
            (
                "def f(a: str, b: int) -> bool:\n    c = a < b\n    return c\n",
                "str < int",
            ),
        ] {
            let error = error_for(source);
            assert_eq!(
                error.kind(),
                LowerErrorKind::TypeMismatch,
                "{note} should be a type mismatch"
            );
        }
    }

    #[test]
    fn boolean_arithmetic_explains_itself() {
        let error = error_for("def f(a: bool, b: bool) -> int:\n    c = a + b\n    return c\n");
        assert!(
            error.message().contains("booleans are not numbers"),
            "message should explain the deliberate divergence, got: {}",
            error.message()
        );
    }

    #[test]
    fn call_makes_an_expression_undetermined_rather_than_ill_typed() {
        // The case a naive implementation gets wrong: it must demand an annotation, not
        // report a type error, when a call is buried inside arithmetic.
        let error = error_for("def f(a: int) -> int:\n    b = helper(a) + 1\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::UndeterminedBinding);
        assert!(error.message().contains("call"));

        // ...and with an annotation it lowers fine, unchecked.
        let f = lower_one("def f(a: int) -> int:\n    b: int = helper(a) + 1\n    return b\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    // ---- declared versus inferred -----------------------------------------

    #[test]
    fn annotation_conflicting_with_the_initializer_is_rejected() {
        for source in [
            "def f() -> str:\n    b: str = 1\n    return b\n",
            "def f(a: int) -> str:\n    b: str = a\n    return b\n",
        ] {
            let error = error_for(source);
            assert_eq!(error.kind(), LowerErrorKind::TypeMismatch);
        }
    }

    #[test]
    fn widening_is_accepted_but_narrowing_is_not() {
        let f = lower_one("def f() -> float:\n    c: float = 1\n    return c\n");
        match &f.body[0] {
            Stmt::Bind { ty, value, .. } => {
                assert_eq!(*ty, Ty::Float);
                assert!(
                    matches!(value, Expr::ToFloat(_)),
                    "int should widen to float"
                );
            }
            other => panic!("expected bind, got {other:?}"),
        }

        let error = error_for("def f() -> int:\n    n: int = 1.5\n    return n\n");
        assert_eq!(error.kind(), LowerErrorKind::TypeMismatch);
    }

    #[test]
    fn float_annotations_are_accepted_everywhere() {
        let f = lower_one("def f(a: float) -> float:\n    b: float = a\n    return b\n");
        assert_eq!(f.params[0].ty, Ty::Float);
        assert_eq!(f.ret, Ty::Float);
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Float, .. }));
    }

    #[test]
    fn returned_value_is_checked_against_the_declared_type() {
        let wrong = error_for("def f() -> int:\n    return \"x\"\n");
        assert_eq!(wrong.kind(), LowerErrorKind::TypeMismatch);

        let from_unit = error_for("def f() -> None:\n    return 1\n");
        assert_eq!(from_unit.kind(), LowerErrorKind::TypeMismatch);

        // Widening applies to returns too.
        let widened = lower_one("def f() -> float:\n    return 1\n");
        assert_eq!(widened.body[0], Stmt::Return(Expr::int(1).to_float()));

        // A returned call is undetermined, so it is not checked.
        let unchecked = lower_one("def f(a: int) -> int:\n    return helper(a)\n");
        assert!(matches!(unchecked.body[0], Stmt::Return(Expr::Call { .. })));
    }

    // ---- diagnostics ------------------------------------------------------

    #[test]
    fn diagnostics_carry_a_useful_span() {
        let source = "def f(a: int) -> int:\n    x = a ** 2\n    return x\n";
        let error = lower(source).unwrap_err();
        let rendered = error.render(source);
        assert!(
            rendered.starts_with("2:"),
            "expected line 2, got {rendered}"
        );
    }

    #[test]
    fn first_violation_in_source_order_is_reported() {
        // Two violations: the unannotated call binding on line 2, the `if` on line 3.
        let source =
            "def f(a: int) -> int:\n    x = helper(a)\n    if a:\n        pass\n    return a\n";
        let error = lower(source).unwrap_err();
        assert_eq!(error.kind(), LowerErrorKind::UndeterminedBinding);
    }

    #[test]
    fn lowering_never_panics_on_parsed_input() {
        for source in [
            "def f(a) -> int:\n    return a\n",
            "class C:\n    pass\n",
            "def f() -> int:\n    return undefined_name\n",
            "x = 1\n",
            "def f() -> float:\n    return 1.5\n",
        ] {
            let _ = lower(source);
        }
    }
}
