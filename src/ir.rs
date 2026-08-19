//! The compylr intermediate representation.
//!
//! The IR is independent of both Python and any target language. Nothing here names a Rust
//! type, a Go type, or a TypeScript type: a backend maps [`Ty`] onto whatever its target
//! spells, which is what keeps a second backend from requiring a second IR.
//!
//! Two consequences are easy to miss and worth stating up front:
//!
//! * Operators carry **Python** semantics. [`BinOp::FloorDiv`] rounds toward negative infinity
//!   and [`BinOp::Mod`] takes the sign of the divisor. Most targets' native `/` and `%`
//!   truncate toward zero and take the sign of the dividend, so a backend that maps these to
//!   the same-named native operator is wrong for negative operands. Naming the Python
//!   semantic here forces that decision to be made deliberately.
//! * IR values own their data. Nothing borrows from the source text or the parse tree, so an
//!   IR value stays valid after both are dropped — which is what lets a unit accumulate
//!   functions parsed at different times.

use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::error::{ArtifactError, LowerError, LowerErrorKind};
use crate::span::Span;

/// Format version of the on-disk artifact.
///
/// Recorded in every artifact and checked on load, so a file written by a future build fails
/// with an explanation rather than deserializing into a subtly wrong unit.
const ARTIFACT_VERSION: u32 = 1;

/// The on-disk envelope around a unit.
///
/// Carrying the fingerprint alongside the functions makes the artifact self-checking, and makes
/// it possible to answer "does this artifact match the current source?" by reading one field
/// instead of reconstructing the whole unit.
#[derive(Debug, Serialize, Deserialize)]
struct UnitArtifact {
    version: u32,
    fingerprint: String,
    functions: Vec<Function>,
    // Absent from artifacts written before classes existed, so a unit with none still deserializes
    // and still fingerprints the same.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    classes: Vec<Class>,
}

/// A type in the supported subset, described by meaning rather than by any target's spelling.
///
/// Recursive: a collection's parameters are themselves types, to any depth. That is what ends
/// `Copy` — a parameterised type owns its parameters — and the cost is borrows and clones at the
/// three dozen places that previously passed a type by value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Ty {
    /// A 64-bit signed integer.
    Int,
    /// A 64-bit binary floating-point number.
    Float,
    /// A boolean.
    Bool,
    /// A UTF-8 text string.
    Str,
    /// The absence of a value; only valid as a return type.
    Unit,
    /// An ordered sequence of one element type.
    List(Box<Ty>),
    /// A mapping from a key type to a value type.
    ///
    /// The key is restricted by [`Ty::can_key`]: a floating-point key is a hazard in Python, where
    /// `nan` is never equal to itself, and most targets cannot hash a float at all.
    Dict(Box<Ty>, Box<Ty>),
    /// A set of one element type, restricted by [`Ty::can_key`] for the same reason.
    Set(Box<Ty>),
    /// A fixed-length tuple carrying a type per position.
    Tuple(Vec<Ty>),
    /// An instance of a class defined in the same unit.
    ///
    /// **The model's one nominal type.** Every other variant is structural — `list[int]` equals
    /// `list[int]` because of what it contains — and a reader will reasonably assume that holds
    /// throughout. It does not here: two classes with identical attributes are different types,
    /// which is what a user means by writing two classes.
    ///
    /// The consequence worth knowing is that this type is only meaningful relative to the unit
    /// that defines the class, which is why an artifact carries class definitions alongside.
    Instance(String),
}

impl Ty {
    /// The Python annotation this type comes from, useful in diagnostics.
    pub fn python_name(&self) -> String {
        match self {
            Self::Int => "int".to_string(),
            Self::Float => "float".to_string(),
            Self::Bool => "bool".to_string(),
            Self::Str => "str".to_string(),
            Self::Unit => "None".to_string(),
            Self::List(element) => format!("list[{}]", element.python_name()),
            Self::Dict(key, value) => {
                format!("dict[{}, {}]", key.python_name(), value.python_name())
            }
            Self::Set(element) => format!("set[{}]", element.python_name()),
            Self::Tuple(elements) => {
                let inner: Vec<String> = elements.iter().map(Ty::python_name).collect();
                format!("tuple[{}]", inner.join(", "))
            }
            Self::Instance(class) => class.clone(),
        }
    }

    /// Whether this type may be a mapping key or a set element.
    ///
    /// Checked when a type is constructed rather than only when an annotation is parsed, so that
    /// every type the IR can hold is one a backend can render. If `Dict(Float, Int)` were
    /// representable, the failure would surface as a target-language complaint about hashing
    /// rather than as a diagnostic pointing at the user's annotation.
    pub fn can_key(&self) -> bool {
        matches!(self, Self::Int | Self::Str | Self::Bool)
    }

    /// Whether values of this type can be copied freely, or must be cloned where consumed.
    ///
    /// Generalises the rule the backend already applied to strings: a name may be read any number
    /// of times, because Python has no notion of a value being consumed by being used.
    pub fn is_trivially_copyable(&self) -> bool {
        matches!(self, Self::Int | Self::Float | Self::Bool | Self::Unit)
    }

    /// Whether arithmetic is defined on this type.
    ///
    /// Booleans are deliberately excluded even though Python's `bool` subclasses `int`:
    /// accepting `True + 1` would force every backend to decide how a boolean widens, and
    /// would make `a + b` on two booleans mean integer addition, which reads as a bug in the
    /// languages compylr emits.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }
}

/// A literal value appearing directly in the source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Literal {
    /// Integer literal, already checked to fit the supported integer range.
    Int(i64),
    /// Floating-point literal, stored as its IEEE-754 bit pattern.
    ///
    /// `f64` implements neither `Eq` nor `Hash`, and every IR type derives both so that
    /// [`Function::fingerprint`] can hash the structure. Storing the bit pattern keeps those
    /// derives and is also the right comparison here: two source literals should contribute
    /// the same fingerprint exactly when they denote the same value. The usual objections do
    /// not apply — NaN cannot be spelled as a Python literal (it needs a call), and `0.0` and
    /// `-0.0` are genuinely different literals worth distinguishing in a rebuild key.
    Float(u64),
    /// Boolean literal.
    Bool(bool),
    /// String literal contents, unescaped.
    Str(String),
}

impl Literal {
    /// Build a floating-point literal from its value.
    pub fn float(value: f64) -> Self {
        Self::Float(value.to_bits())
    }

    /// The value of a floating-point literal, or `None` for other literal kinds.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        }
    }

    /// Type of this literal.
    pub fn ty(&self) -> Ty {
        match self {
            Self::Int(_) => Ty::Int,
            Self::Float(_) => Ty::Float,
            Self::Bool(_) => Ty::Bool,
            Self::Str(_) => Ty::Str,
        }
    }
}

/// A binary operator, carrying Python's semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinOp {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// True division: always yields a floating-point result, even for two integer operands.
    ///
    /// This is the trap `/` sets for a backend. In Python `7 / 2` is `3.5`; in Rust, Go, and
    /// C++ the same spelling between two integers is integer division yielding `3`. A backend
    /// must convert its operands before dividing.
    TrueDiv,
    /// Floor division: rounds toward negative infinity, unlike most targets' `/`.
    FloorDiv,
    /// Remainder: takes the sign of the divisor, unlike most targets' `%`.
    Mod,
    /// Equality.
    Eq,
    /// Inequality.
    NotEq,
    /// Less than.
    Lt,
    /// Less than or equal.
    LtE,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    GtE,
}

impl BinOp {
    /// Whether this operator yields a boolean regardless of operand types.
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::Eq | Self::NotEq | Self::Lt | Self::LtE | Self::Gt | Self::GtE
        )
    }

    /// The Python spelling of this operator, for diagnostics.
    pub fn python_symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::TrueDiv => "/",
            Self::FloorDiv => "//",
            Self::Mod => "%",
            Self::Eq => "==",
            Self::NotEq => "!=",
            Self::Lt => "<",
            Self::LtE => "<=",
            Self::Gt => ">",
            Self::GtE => ">=",
        }
    }
}

/// An expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expr {
    /// A literal value.
    Literal(Literal),
    /// A reference to a parameter or local.
    Name(String),
    /// Arithmetic negation.
    Neg(Box<Expr>),
    /// Widening of an integer expression to floating-point.
    ///
    /// Inserted by lowering wherever Python's numeric promotion applies, so the conversion is
    /// visible in the tree instead of something each backend has to re-derive. A backend that
    /// emitted operands positionally without this node would produce integer arithmetic where
    /// Python produces float arithmetic.
    ToFloat(Box<Expr>),
    /// A binary operation.
    Binary {
        /// Operator applied.
        op: BinOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// A sequence literal, elements in source order.
    ListLit(Vec<Expr>),
    /// A mapping literal, pairs in source order.
    DictLit(Vec<(Expr, Expr)>),
    /// A set literal.
    SetLit(Vec<Expr>),
    /// A tuple literal, which unlike the others carries a type per position.
    TupleLit(Vec<Expr>),
    /// Reading one element of a collection.
    /// Read a tuple element at a position fixed at compile time.
    ///
    /// Distinct from [`Self::Subscript`] because a tuple is heterogeneous: the type of the result
    /// depends on *which* position, so it cannot be a lookup taking a runtime index the way a
    /// sequence or mapping read can. Lowering has already resolved the position and rejected a
    /// computed one, and recording that here is what lets a backend emit a static field access
    /// rather than search for a lookup operation that cannot exist.
    TupleIndex {
        /// The tuple being read.
        base: Box<Expr>,
        /// Which element, already checked against the tuple's length.
        position: usize,
    },
    /// Read an attribute of an object.
    Attribute {
        /// The object being read.
        object: Box<Expr>,
        /// Which attribute, already checked against the class's declarations.
        name: String,
    },
    /// Construct an instance.
    ///
    /// Its own form rather than a call. Leaving it a call would mean unit validation resolving it
    /// against functions, and the type rules differ enough — arguments check against `__init__`,
    /// the result is an instance type — that one form would make each path carry the other's
    /// cases. The same reasoning already applied to `len` and `range`.
    Construct {
        /// The class being instantiated.
        class: String,
        /// Constructor arguments, already checked against `__init__`.
        args: Vec<Expr>,
    },
    /// Call a method on an object.
    ///
    /// The method resolves against the receiver's class rather than against the unit, which is why
    /// [`Expr::walk_calls`] deliberately does not report it: demanding a free function of that
    /// name would fail on a program the user wrote correctly.
    MethodCall {
        /// The object the method is called on.
        receiver: Box<Expr>,
        /// Which method.
        method: String,
        /// Arguments, already checked against the method's signature.
        args: Vec<Expr>,
    },
    /// Whether a value is present in a container.
    ///
    /// What "present" means is the container's own: a sequence and a set test elements, a mapping
    /// tests **keys**, and a string tests substrings. Each matches Python, and none is what a naive
    /// containment check over the target's native type would do for at least one of them.
    Contains {
        /// The value being looked for.
        value: Box<Expr>,
        /// What is being searched.
        container: Box<Expr>,
    },
    /// Logical negation of a boolean.
    ///
    /// Exists so `not in` can be the negation of a membership test rather than a second spelling of
    /// one. A flag on [`Self::Contains`] would make every consumer responsible for remembering to
    /// honour it, and the one that forgot would silently invert an answer.
    Not(Box<Expr>),
    Subscript {
        /// The collection being read.
        base: Box<Expr>,
        /// The index or key.
        index: Box<Expr>,
    },
    /// The length of a collection or string.
    ///
    /// A distinct node rather than a call: a call is resolved against the unit during validation,
    /// so leaving `len` as one would make its meaning depend on whether someone had decorated a
    /// function of that name.
    Len(Box<Expr>),
    /// A range of integers, as Python's `range` produces.
    ///
    /// All three components are present even when the source omitted them, so a backend never has
    /// to know Python's defaulting rules. A distinct form rather than a call, for the reason
    /// [`Expr::Len`] is: a call is resolved against the unit, so leaving it as one would make its
    /// meaning depend on what else was compiled.
    Range {
        /// First value.
        start: Box<Expr>,
        /// Exclusive bound.
        stop: Box<Expr>,
        /// Amount added each step. May be negative; may not be zero at runtime.
        step: Box<Expr>,
    },
    /// A call to a function by name.
    Call {
        /// Name of the target function.
        callee: String,
        /// Argument expressions, in order.
        args: Vec<Expr>,
    },
}

impl Expr {
    /// Convenience constructor for an integer literal.
    pub fn int(value: i64) -> Self {
        Self::Literal(Literal::Int(value))
    }

    /// Convenience constructor for a floating-point literal.
    pub fn float(value: f64) -> Self {
        Self::Literal(Literal::float(value))
    }

    /// Convenience constructor for a boolean literal.
    pub fn bool(value: bool) -> Self {
        Self::Literal(Literal::Bool(value))
    }

    /// Wrap this expression in an explicit widening to floating-point.
    pub fn to_float(self) -> Self {
        Self::ToFloat(Box::new(self))
    }

    /// Convenience constructor for a string literal.
    pub fn string(value: impl Into<String>) -> Self {
        Self::Literal(Literal::Str(value.into()))
    }

    /// Convenience constructor for a name reference.
    pub fn name(value: impl Into<String>) -> Self {
        Self::Name(value.into())
    }

    /// Convenience constructor for a binary operation.
    pub fn binary(op: BinOp, left: Expr, right: Expr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Visit every call expression in this tree, including nested ones.
    pub fn walk_calls(&self, visit: &mut impl FnMut(&str, usize)) {
        match self {
            Self::Literal(_) | Self::Name(_) => {}
            // ToFloat must descend, or a call wrapped in a promotion would be invisible to
            // Unit::validate and its target would never be checked.
            Self::Neg(inner) | Self::ToFloat(inner) | Self::Len(inner) => inner.walk_calls(visit),
            Self::ListLit(items) | Self::SetLit(items) | Self::TupleLit(items) => {
                for item in items {
                    item.walk_calls(visit);
                }
            }
            Self::DictLit(pairs) => {
                for (key, value) in pairs {
                    key.walk_calls(visit);
                    value.walk_calls(visit);
                }
            }
            Self::TupleIndex { base, .. } => base.walk_calls(visit),
            Self::Not(inner) => inner.walk_calls(visit),
            Self::Attribute { object, .. } => object.walk_calls(visit),
            Self::Construct { args, .. } => {
                for arg in args {
                    arg.walk_calls(visit);
                }
            }
            // The method itself is deliberately not reported: it resolves against the receiver's
            // class, and demanding a free function of that name would reject correct code.
            Self::MethodCall { receiver, args, .. } => {
                receiver.walk_calls(visit);
                for arg in args {
                    arg.walk_calls(visit);
                }
            }
            Self::Contains { value, container } => {
                value.walk_calls(visit);
                container.walk_calls(visit);
            }
            Self::Subscript { base, index } => {
                base.walk_calls(visit);
                index.walk_calls(visit);
            }
            Self::Range { start, stop, step } => {
                start.walk_calls(visit);
                stop.walk_calls(visit);
                step.walk_calls(visit);
            }
            Self::Binary { left, right, .. } => {
                left.walk_calls(visit);
                right.walk_calls(visit);
            }
            Self::Call { callee, args } => {
                visit(callee, args.len());
                for arg in args {
                    arg.walk_calls(visit);
                }
            }
        }
    }
}

/// A statement in a function body.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stmt {
    /// Return a value.
    Return(Expr),
    /// Return no value, or do nothing (`pass`).
    ReturnUnit,
    /// Introduce a new local bound to a value.
    Bind {
        /// Name being introduced.
        name: String,
        /// Type of the binding, declared or inferred from an alias.
        ty: Ty,
        /// Value bound to the name.
        value: Expr,
    },
    /// Assign to a name already bound.
    ///
    /// Distinct from [`Stmt::Bind`] because a backend renders them differently: a binding declares,
    /// an assignment updates. Keeping them apart also means a backend never has to work out which
    /// of two identically-shaped statements introduced the name.
    Assign {
        /// Name being assigned.
        name: String,
        /// The type the name was bound at. Carried so a backend can render the value without
        /// re-deriving what lowering already established.
        ty: Ty,
        /// Value assigned. Its type matches `ty`.
        value: Expr,
    },
    /// Evaluate an expression for its effect, discarding a unit result.
    ///
    /// Lowering only ever puts a unit-returning **method** call here. A free function in this
    /// subset can reach no mutable state, so calling one and discarding the result is dead code and
    /// stays rejected; a method can mutate its receiver, which is the whole point of one.
    Effect(Expr),
    /// Assign to an attribute of an object.
    ///
    /// Distinct from [`Self::SetItem`]: an attribute is declared once with a fixed type, so the
    /// set of them is known from the class rather than growing at runtime.
    SetAttr {
        /// The object being modified.
        object: Expr,
        /// Which attribute.
        name: String,
        /// The type it was declared with, so a backend need not re-derive it.
        ty: Ty,
        /// The new value, already checked against `ty`.
        value: Expr,
    },
    /// Assign to one element of a collection.
    ///
    /// Distinct from [`Self::Assign`], which rebinds a name. Here the name keeps denoting the same
    /// collection and one of its entries changes — and for a mapping the entry may not exist yet,
    /// which is why this cannot be expressed as a read followed by a write.
    SetItem {
        /// The collection being modified.
        collection: Expr,
        /// Which element: an index for a sequence, a key for a mapping.
        index: Expr,
        /// The new value, already checked against what the collection holds.
        value: Expr,
    },
    /// Append a value to a sequence.
    ///
    /// Its own form rather than a general method call. There is exactly one supported method, and a
    /// general form would need a table of method signatures per type before anything consumed it,
    /// plus a decision in every backend about what an unknown method means. An explicit form cannot
    /// be spelled with the wrong name.
    Append {
        /// The sequence being extended.
        sequence: Expr,
        /// The value appended, already checked against the element type.
        value: Expr,
    },
    /// Conditional execution.
    ///
    /// `elif` has no form of its own: it is a conditional in the `otherwise` of another, which is
    /// what it means, and gives a backend one shape to render rather than two.
    If {
        /// The test, which must be a boolean.
        test: Expr,
        /// Statements run when the test holds.
        then: Vec<Stmt>,
        /// Statements run otherwise. Empty when the source had no `else`.
        otherwise: Vec<Stmt>,
    },
    /// Repetition while a test holds.
    While {
        /// The test, which must be a boolean.
        test: Expr,
        /// The body.
        body: Vec<Stmt>,
    },
    /// Repetition over the values of an iterable.
    For {
        /// Name bound to each value. Visible only inside the body.
        name: String,
        /// Type each value takes.
        ty: Ty,
        /// What is iterated: a range, or a collection.
        iter: Expr,
        /// The body.
        body: Vec<Stmt>,
    },
    /// Abandon the nearest enclosing loop.
    Break,
    /// Restart the nearest enclosing loop.
    Continue,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Param {
    /// Parameter name.
    pub name: String,
    /// Parameter type.
    pub ty: Ty,
}

/// A function in the IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Function {
    /// Function name, unique within a unit.
    pub name: String,
    /// Parameters, in declaration order.
    pub params: Vec<Param>,
    /// Declared return type.
    pub ret: Ty,
    /// Body statements, in source order.
    pub body: Vec<Stmt>,
    /// The function's docstring, if it has one.
    ///
    /// Held as a field rather than as a body statement. A `Stmt` variant would put something in
    /// the body that every consumer must remember to skip, and every backend would independently
    /// have to decide it emits nothing; a field is skipped by construction.
    ///
    /// Serialized, because it is useful when reading the artifact, but **not** fingerprinted —
    /// see [`Function::fingerprint`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// Where the function was declared, for diagnostics.
    ///
    /// Deliberately absent from the serialized artifact. A span is a byte offset into a source
    /// text the artifact does not contain, so it is meaningless once written out — and including
    /// it would make two units that differ only in comments and indentation serialize
    /// differently, which is exactly what the artifact must not do. This matches the IR's own
    /// definition of structure: [`Function::fingerprint`] does not hash the span either.
    #[serde(skip)]
    pub span: Span,
}

impl Function {
    /// A fingerprint derived only from this function's structure.
    ///
    /// The span is deliberately excluded: moving a function down a file, or reindenting it,
    /// changes its offsets but not its meaning, and a rebuild triggered by that would be waste.
    /// Hashing the IR rather than the source is what makes comments and formatting free.
    ///
    /// The docstring is excluded for the same reason. It is prose *about* the function rather than
    /// part of what the function computes, so fixing a typo in documentation would otherwise cost
    /// a full crate rebuild — which is exactly the waste this fingerprint exists to avoid.
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.params.hash(&mut hasher);
        self.ret.hash(&mut hasher);
        self.body.hash(&mut hasher);
        hasher.finish()
    }

    /// Visit every call made anywhere in this function's body.
    pub fn walk_calls(&self, visit: &mut impl FnMut(&str, usize)) {
        walk_stmts(&self.body, visit);
    }
}

/// Whether a sequence of statements produces a value on **every** path.
///
/// Shared by lowering, which rejects a function that does not, and by the backend, which uses it to
/// decide whether a trailing value is needed. One implementation rather than two, because the two
/// disagreeing means either a valid program is rejected or generated code fails to compile — and
/// the second surfaces as a complaint about Rust rather than about the user's function.
///
/// A conditional counts only when it has an alternative and **both** branches return. A loop never
/// counts: its body may run zero times, and proving otherwise would mean evaluating the test.
pub fn returns_on_all_paths(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match stmt {
        Stmt::Return(_) | Stmt::ReturnUnit => true,
        Stmt::If {
            then, otherwise, ..
        } => !otherwise.is_empty() && returns_on_all_paths(then) && returns_on_all_paths(otherwise),
        // Deliberately false. `while True:` would be provable and is not worth a special case that
        // only one spelling benefits from.
        Stmt::While { .. } | Stmt::For { .. } => false,
        Stmt::Bind { .. }
        | Stmt::Assign { .. }
        | Stmt::SetItem { .. }
        | Stmt::SetAttr { .. }
        | Stmt::Effect(_)
        | Stmt::Append { .. }
        | Stmt::Break
        | Stmt::Continue => false,
    })
}

/// Visit every call in a sequence of statements, descending into nested bodies.
///
/// Nested bodies are why this is a free function rather than a loop inside `walk_calls`: a call
/// inside a loop inside a branch must still be found, or unit validation would miss it and the
/// backend would emit a call to something that does not exist.
fn walk_stmts(stmts: &[Stmt], visit: &mut impl FnMut(&str, usize)) {
    for stmt in stmts {
        match stmt {
            Stmt::Return(expr) => expr.walk_calls(visit),
            Stmt::Bind { value, .. } | Stmt::Assign { value, .. } => value.walk_calls(visit),
            Stmt::If {
                test,
                then,
                otherwise,
            } => {
                test.walk_calls(visit);
                walk_stmts(then, visit);
                walk_stmts(otherwise, visit);
            }
            Stmt::While { test, body } => {
                test.walk_calls(visit);
                walk_stmts(body, visit);
            }
            Stmt::For { iter, body, .. } => {
                iter.walk_calls(visit);
                walk_stmts(body, visit);
            }
            Stmt::SetItem {
                collection,
                index,
                value,
            } => {
                collection.walk_calls(visit);
                index.walk_calls(visit);
                value.walk_calls(visit);
            }
            Stmt::Effect(expr) => expr.walk_calls(visit),
            Stmt::SetAttr { object, value, .. } => {
                object.walk_calls(visit);
                value.walk_calls(visit);
            }
            Stmt::Append { sequence, value } => {
                sequence.walk_calls(visit);
                value.walk_calls(visit);
            }
            Stmt::ReturnUnit | Stmt::Break | Stmt::Continue => {}
        }
    }
}

/// One attribute of a class, as declared in `__init__`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name, without the `self.` prefix.
    pub name: String,
    /// Declared type. Mandatory, on the same terms as a parameter's.
    pub ty: Ty,
}

/// A class: named state and the methods over it.
///
/// Attributes are held in declaration order rather than sorted, because that order is the class's
/// shape as its author wrote it and a backend emitting fields wants to preserve it. Methods are
/// keyed by name, which gives uniqueness and a deterministic order in one structure — the same
/// reasoning as [`Unit`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Class {
    /// Class name, which is also the name of its instance type.
    pub name: String,
    /// Attributes, in declaration order.
    pub attributes: Vec<Attribute>,
    /// The constructor. Its parameters exclude `self`, and its body initialises the attributes.
    pub init: Function,
    /// Methods other than `__init__`, by name.
    pub methods: BTreeMap<String, Function>,
    /// Docstring, excluded from the fingerprint like a function's.
    pub doc: Option<String>,
    /// Where the class was defined.
    #[serde(skip)]
    pub span: Span,
}

impl Class {
    /// A fingerprint over the class's structure, excluding documentation and spans.
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.attributes.hash(&mut hasher);
        self.init.fingerprint().hash(&mut hasher);
        // Sorted by the map, so declaration order of methods does not move the print.
        for method in self.methods.values() {
            method.fingerprint().hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Every function in the class, constructor first.
    pub fn functions(&self) -> impl Iterator<Item = &Function> {
        std::iter::once(&self.init).chain(self.methods.values())
    }

    /// The declared type of one attribute.
    pub fn attribute(&self, name: &str) -> Option<&Ty> {
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| &a.ty)
    }
}

/// A compilation unit: every function that will share one build artifact.
///
/// Functions reach the compiler independently — in the target design each is decorated
/// separately — but they are emitted into a single shared crate. A unit therefore accumulates
/// functions one at a time and resolves calls across the whole set, not per source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Unit {
    // A BTreeMap keys the unit by name, which gives uniqueness and a deterministic,
    // addition-order-independent iteration order in one structure.
    functions: BTreeMap<String, Function>,
    // Classes share the namespace with functions: they compile into one file and one module, so a
    // collision would surface as a Rust error rather than a diagnostic.
    classes: BTreeMap<String, Class>,
}

impl Unit {
    /// Create an empty unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function, failing if that name is already taken by a function or a class.
    pub fn add_function(&mut self, function: Function) -> Result<(), LowerError> {
        self.reject_taken_name(&function.name, function.span)?;
        self.functions.insert(function.name.clone(), function);
        Ok(())
    }

    /// Add a class, failing if that name is already taken by a class or a function.
    pub fn add_class(&mut self, class: Class) -> Result<(), LowerError> {
        self.reject_taken_name(&class.name, class.span)?;
        self.classes.insert(class.name.clone(), class);
        Ok(())
    }

    /// Refuse a name already used by either kind of member.
    fn reject_taken_name(&self, name: &str, span: Span) -> Result<(), LowerError> {
        let taken = if self.functions.contains_key(name) {
            Some("function")
        } else if self.classes.contains_key(name) {
            Some("class")
        } else {
            None
        };
        match taken {
            None => Ok(()),
            Some(kind) => Err(LowerError::new(
                LowerErrorKind::DuplicateFunction,
                format!("{kind} '{name}' is already defined in this unit"),
                span,
            )),
        }
    }

    /// Classes in deterministic (name) order.
    pub fn classes(&self) -> impl Iterator<Item = &Class> {
        self.classes.values()
    }

    /// Look up a class by name.
    pub fn class(&self, name: &str) -> Option<&Class> {
        self.classes.get(name)
    }

    /// Functions in deterministic (name) order.
    pub fn functions(&self) -> impl Iterator<Item = &Function> {
        self.functions.values()
    }

    /// Look up a function by name.
    pub fn get(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }

    /// Number of functions in the unit.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Whether the unit holds no functions.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// A fingerprint over the whole unit, independent of addition order.
    ///
    /// Member fingerprints are sorted before combining, so decorating functions in a different
    /// order across runs does not invalidate a cached build.
    pub fn fingerprint(&self) -> u64 {
        let mut prints: Vec<u64> = self.functions.values().map(Function::fingerprint).collect();
        prints.sort_unstable();
        let mut hasher = DefaultHasher::new();
        prints.hash(&mut hasher);

        // Classes contribute only when there are some. A unit with none must fingerprint exactly
        // as it did before classes existed, or every cached build in every project invalidates on
        // upgrade with nothing to show for it.
        if !self.classes.is_empty() {
            let mut class_prints: Vec<u64> =
                self.classes.values().map(Class::fingerprint).collect();
            class_prints.sort_unstable();
            class_prints.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Serialize the unit to its on-disk artifact form.
    ///
    /// The artifact is the pipeline's window between lowering and code generation: it belongs to
    /// the IR rather than to any one backend, because every backend consumes the same tree.
    ///
    /// Output is deterministic. Functions are held in a [`BTreeMap`], so they serialize in name
    /// order regardless of the order they were added, and spans are excluded, so reformatting the
    /// source does not change the bytes.
    pub fn to_json(&self) -> Result<String, ArtifactError> {
        let artifact = UnitArtifact {
            version: ARTIFACT_VERSION,
            fingerprint: format!("{:016x}", self.fingerprint()),
            functions: self.functions.values().cloned().collect(),
            classes: self.classes.values().cloned().collect(),
        };
        Ok(serde_json::to_string_pretty(&artifact)?)
    }

    /// Rebuild a unit from its artifact form.
    ///
    /// The recorded fingerprint is recomputed and compared, so a truncated or hand-edited
    /// artifact fails loudly instead of loading as a valid but different unit — which would let
    /// the rebuild cache reuse a build corresponding to no source at all.
    pub fn from_json(json: &str) -> Result<Self, ArtifactError> {
        let artifact: UnitArtifact = serde_json::from_str(json)?;
        if artifact.version != ARTIFACT_VERSION {
            return Err(ArtifactError::UnsupportedVersion {
                found: artifact.version,
                expected: ARTIFACT_VERSION,
            });
        }

        let mut unit = Self::new();
        for function in artifact.functions {
            unit.add_function(function)
                .map_err(|e| ArtifactError::DuplicateFunction(Box::new(e)))?;
        }
        for class in artifact.classes {
            unit.add_class(class)
                .map_err(|e| ArtifactError::DuplicateFunction(Box::new(e)))?;
        }

        let computed = format!("{:016x}", unit.fingerprint());
        if computed != artifact.fingerprint {
            return Err(ArtifactError::FingerprintMismatch {
                recorded: artifact.fingerprint,
                computed,
            });
        }
        Ok(unit)
    }

    /// Check that every call resolves to a function in this unit with matching arity.
    ///
    /// This runs across the assembled unit rather than during lowering because a function may
    /// legitimately call one that has not been added yet — resolving early would make success
    /// depend on the order functions happened to arrive.
    pub fn validate(&self) -> Result<(), LowerError> {
        for class in self.classes.values() {
            for function in class.functions() {
                self.validate_calls(function)?;
            }
        }
        for function in self.functions.values() {
            self.validate_calls(function)?;
        }
        Ok(())
    }

    /// Check one function's calls against the unit.
    fn validate_calls(&self, function: &Function) -> Result<(), LowerError> {
        {
            let mut failure: Option<LowerError> = None;
            function.walk_calls(&mut |callee, argc| {
                if failure.is_some() {
                    return;
                }
                match self.functions.get(callee) {
                    None => {
                        failure = Some(LowerError::new(
                            LowerErrorKind::Unresolved,
                            format!("call to undefined function '{callee}'"),
                            function.span,
                        ));
                    }
                    Some(target) if target.params.len() != argc => {
                        failure = Some(LowerError::new(
                            LowerErrorKind::ArityMismatch,
                            format!(
                                "'{callee}' expects {} argument(s) but {argc} were given",
                                target.params.len()
                            ),
                            function.span,
                        ));
                    }
                    Some(_) => {}
                }
            });
            if let Some(error) = failure {
                return Err(error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn func(name: &str, params: Vec<Param>, ret: Ty, body: Vec<Stmt>) -> Function {
        Function {
            name: name.to_string(),
            params,
            ret,
            body,
            doc: None,
            span: Span::new(0, 1),
        }
    }

    fn param(name: &str, ty: Ty) -> Param {
        Param {
            name: name.to_string(),
            ty,
        }
    }

    #[test]
    fn ty_covers_exactly_the_supported_set() {
        let all = [Ty::Int, Ty::Float, Ty::Bool, Ty::Str, Ty::Unit];
        let names: Vec<String> = all.iter().map(|t| t.python_name()).collect();
        assert_eq!(names, ["int", "float", "bool", "str", "None"]);
        assert_eq!(Ty::Int, Ty::Int);
        assert_ne!(Ty::Int, Ty::Bool);
    }

    #[test]
    fn int_and_float_are_distinct_types() {
        // A backend has to know which representation to emit, so these must never unify.
        assert_ne!(Ty::Int, Ty::Float);
        assert!(Ty::Int.is_numeric() && Ty::Float.is_numeric());
        assert!(
            !Ty::Bool.is_numeric(),
            "booleans are deliberately not numeric"
        );
        assert!(!Ty::Str.is_numeric());
        assert!(!Ty::Unit.is_numeric());
    }

    #[test]
    fn literals_report_their_type() {
        assert_eq!(Literal::Int(1).ty(), Ty::Int);
        assert_eq!(Literal::float(1.5).ty(), Ty::Float);
        assert_eq!(Literal::Bool(true).ty(), Ty::Bool);
        assert_eq!(Literal::Str("x".into()).ty(), Ty::Str);
    }

    #[test]
    fn float_literals_round_trip_and_compare_by_value() {
        let a = Literal::float(1.3);
        assert_eq!(a.as_f64(), Some(1.3));
        assert_eq!(a, Literal::float(1.3));
        assert_ne!(a, Literal::float(1.4));
        assert_eq!(Literal::Int(1).as_f64(), None);
    }

    #[test]
    fn float_literals_can_be_fingerprinted() {
        // The whole reason for storing bits: f64 is neither Eq nor Hash, and Function
        // derives both. A body containing a float must still produce a fingerprint.
        let f = func("f", vec![], Ty::Float, vec![Stmt::Return(Expr::float(1.3))]);
        let same = func("f", vec![], Ty::Float, vec![Stmt::Return(Expr::float(1.3))]);
        let other = func("f", vec![], Ty::Float, vec![Stmt::Return(Expr::float(1.4))]);
        assert_eq!(f.fingerprint(), same.fingerprint());
        assert_ne!(f.fingerprint(), other.fingerprint());
    }

    #[test]
    fn positive_and_negative_zero_are_distinguishable() {
        // Pins the documented bitwise-comparison decision so a later change cannot quietly
        // normalise it away.
        assert_ne!(Literal::float(0.0), Literal::float(-0.0));
        assert_eq!(Literal::float(0.0).as_f64(), Some(0.0));
        assert_eq!(Literal::float(-0.0).as_f64(), Some(-0.0));
    }

    #[test]
    fn true_division_is_distinct_from_floor_division() {
        assert_ne!(BinOp::TrueDiv, BinOp::FloorDiv);
        assert_eq!(BinOp::TrueDiv.python_symbol(), "/");
        assert_eq!(BinOp::FloorDiv.python_symbol(), "//");
        assert!(!BinOp::TrueDiv.is_comparison());
    }

    #[test]
    fn to_float_nests_and_participates_in_equality() {
        let wrapped = Expr::name("a").to_float();
        assert_eq!(wrapped, Expr::ToFloat(Box::new(Expr::name("a"))));
        assert_ne!(wrapped, Expr::name("a"));
        match &wrapped {
            Expr::ToFloat(inner) => assert_eq!(**inner, Expr::name("a")),
            other => panic!("expected ToFloat, got {other:?}"),
        }
    }

    #[test]
    fn walk_calls_descends_through_promotion() {
        // A call hidden under a promotion must still be validated.
        let expr = Expr::Call {
            callee: "helper".into(),
            args: vec![],
        }
        .to_float();
        let mut seen = Vec::new();
        expr.walk_calls(&mut |name, argc| seen.push((name.to_string(), argc)));
        assert_eq!(seen, vec![("helper".to_string(), 0)]);
    }

    #[test]
    fn comparisons_are_flagged_and_arithmetic_is_not() {
        for op in [
            BinOp::Eq,
            BinOp::NotEq,
            BinOp::Lt,
            BinOp::LtE,
            BinOp::Gt,
            BinOp::GtE,
        ] {
            assert!(op.is_comparison(), "{op:?} should be a comparison");
        }
        for op in [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::TrueDiv,
            BinOp::FloorDiv,
            BinOp::Mod,
        ] {
            assert!(!op.is_comparison(), "{op:?} should not be a comparison");
        }
    }

    #[test]
    fn every_operator_has_a_distinct_python_spelling() {
        let ops = [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::TrueDiv,
            BinOp::FloorDiv,
            BinOp::Mod,
            BinOp::Eq,
            BinOp::NotEq,
            BinOp::Lt,
            BinOp::LtE,
            BinOp::Gt,
            BinOp::GtE,
        ];
        let mut symbols: Vec<&str> = ops.iter().map(|op| op.python_symbol()).collect();
        assert_eq!(symbols.len(), 12);
        symbols.sort_unstable();
        symbols.dedup();
        assert_eq!(symbols.len(), 12, "operator spellings must be distinct");
        assert_eq!(BinOp::FloorDiv.python_symbol(), "//");
    }

    #[test]
    fn unit_lookup_finds_present_functions_only() {
        let mut unit = Unit::new();
        unit.add_function(func("present", vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
            .unwrap();
        assert_eq!(
            unit.get("present").map(|f| f.name.as_str()),
            Some("present")
        );
        assert!(unit.get("absent").is_none());
    }

    #[test]
    fn expressions_nest_three_levels_deep() {
        // (a + 1) * -(b)
        let expr = Expr::binary(
            BinOp::Mul,
            Expr::binary(BinOp::Add, Expr::name("a"), Expr::int(1)),
            Expr::Neg(Box::new(Expr::name("b"))),
        );
        match &expr {
            Expr::Binary { op, left, right } => {
                assert_eq!(*op, BinOp::Mul);
                assert!(matches!(**left, Expr::Binary { op: BinOp::Add, .. }));
                assert!(matches!(**right, Expr::Neg(_)));
            }
            other => panic!("expected binary, got {other:?}"),
        }
    }

    #[test]
    fn call_preserves_argument_order() {
        let call = Expr::Call {
            callee: "add".to_string(),
            args: vec![Expr::int(1), Expr::int(2)],
        };
        match call {
            Expr::Call { callee, args } => {
                assert_eq!(callee, "add");
                assert_eq!(args, vec![Expr::int(1), Expr::int(2)]);
            }
            other => panic!("expected call, got {other:?}"),
        }
    }

    #[test]
    fn statements_cover_the_three_supported_forms() {
        let stmts = [
            Stmt::Return(Expr::int(1)),
            Stmt::ReturnUnit,
            Stmt::Bind {
                name: "x".into(),
                ty: Ty::Int,
                value: Expr::int(2),
            },
        ];
        assert_eq!(stmts.len(), 3);
        assert!(matches!(stmts[2], Stmt::Bind { ty: Ty::Int, .. }));
    }

    #[test]
    fn ir_outlives_its_source() {
        let function = {
            let source = String::from("def f(a: int) -> int:\n    return a\n");
            let built = func(
                "f",
                vec![param("a", Ty::Int)],
                Ty::Int,
                vec![Stmt::Return(Expr::name("a"))],
            );
            drop(source); // The IR must not borrow from the text it came from.
            built
        };
        assert_eq!(function.name, "f");
        assert_eq!(function.params[0].ty, Ty::Int);
    }

    #[test]
    fn empty_unit_is_valid() {
        let unit = Unit::new();
        assert!(unit.is_empty());
        assert_eq!(unit.len(), 0);
        assert!(unit.validate().is_ok());
    }

    #[test]
    fn unit_accumulates_functions_from_separate_sources() {
        let mut unit = Unit::new();
        for name in ["a", "b", "c"] {
            unit.add_function(func(name, vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
                .unwrap();
        }
        assert_eq!(unit.len(), 3);

        // Adding a fourth must not disturb the first three.
        let before: Vec<u64> = unit.functions().map(Function::fingerprint).collect();
        unit.add_function(func("d", vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
            .unwrap();
        assert_eq!(unit.len(), 4);
        let after: Vec<u64> = unit
            .functions()
            .filter(|f| f.name != "d")
            .map(Function::fingerprint)
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn duplicate_function_name_is_refused() {
        let mut unit = Unit::new();
        unit.add_function(func("dup", vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
            .unwrap();
        let error = unit
            .add_function(func("dup", vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
            .unwrap_err();
        assert_eq!(error.kind(), LowerErrorKind::DuplicateFunction);
        assert!(error.message().contains("dup"));
        assert_eq!(unit.len(), 1);
    }

    #[test]
    fn unit_order_is_independent_of_addition_order() {
        let make = |names: [&str; 3]| {
            let mut unit = Unit::new();
            for name in names {
                unit.add_function(func(name, vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
                    .unwrap();
            }
            unit
        };
        let forward = make(["alpha", "beta", "gamma"]);
        let backward = make(["gamma", "beta", "alpha"]);
        let names_of = |u: &Unit| u.functions().map(|f| f.name.clone()).collect::<Vec<_>>();
        assert_eq!(names_of(&forward), names_of(&backward));
        assert_eq!(names_of(&forward), vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn identical_structure_fingerprints_identically() {
        let a = func(
            "f",
            vec![param("a", Ty::Int)],
            Ty::Int,
            vec![Stmt::Return(Expr::name("a"))],
        );
        let mut b = a.clone();
        // A different location must not change the fingerprint.
        b.span = Span::new(500, 900);
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn changing_body_signature_or_return_changes_fingerprint() {
        let base = func(
            "f",
            vec![param("a", Ty::Int)],
            Ty::Int,
            vec![Stmt::Return(Expr::name("a"))],
        );

        let mut other_body = base.clone();
        other_body.body = vec![Stmt::Return(Expr::int(0))];
        assert_ne!(base.fingerprint(), other_body.fingerprint());

        let mut other_param = base.clone();
        other_param.params = vec![param("a", Ty::Bool)];
        assert_ne!(base.fingerprint(), other_param.fingerprint());

        let mut other_ret = base.clone();
        other_ret.ret = Ty::Bool;
        assert_ne!(base.fingerprint(), other_ret.fingerprint());
    }

    #[test]
    fn unit_fingerprint_changes_on_add_and_ignores_order() {
        let mut unit = Unit::new();
        for name in ["a", "b", "c"] {
            unit.add_function(func(name, vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
                .unwrap();
        }
        let before = unit.fingerprint();
        unit.add_function(func("d", vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
            .unwrap();
        assert_ne!(before, unit.fingerprint());

        let mut reversed = Unit::new();
        for name in ["d", "c", "b", "a"] {
            reversed
                .add_function(func(name, vec![], Ty::Unit, vec![Stmt::ReturnUnit]))
                .unwrap();
        }
        assert_eq!(unit.fingerprint(), reversed.fingerprint());
    }

    #[test]
    fn validate_resolves_calls_across_sources_in_any_order() {
        let mut unit = Unit::new();
        // Caller added before callee: resolution must not depend on order.
        unit.add_function(func(
            "caller",
            vec![],
            Ty::Int,
            vec![Stmt::Return(Expr::Call {
                callee: "callee".into(),
                args: vec![Expr::int(1)],
            })],
        ))
        .unwrap();
        assert!(unit.validate().is_err(), "callee not present yet");

        unit.add_function(func(
            "callee",
            vec![param("n", Ty::Int)],
            Ty::Int,
            vec![Stmt::Return(Expr::name("n"))],
        ))
        .unwrap();
        assert!(unit.validate().is_ok());
    }

    #[test]
    fn validate_rejects_unknown_callee_and_bad_arity() {
        let mut unit = Unit::new();
        unit.add_function(func(
            "caller",
            vec![],
            Ty::Int,
            vec![Stmt::Return(Expr::Call {
                callee: "nope".into(),
                args: vec![],
            })],
        ))
        .unwrap();
        let error = unit.validate().unwrap_err();
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
        assert!(error.message().contains("nope"));

        let mut unit = Unit::new();
        unit.add_function(func(
            "target",
            vec![param("a", Ty::Int)],
            Ty::Int,
            vec![Stmt::Return(Expr::name("a"))],
        ))
        .unwrap();
        unit.add_function(func(
            "caller",
            vec![],
            Ty::Int,
            vec![Stmt::Return(Expr::Call {
                callee: "target".into(),
                args: vec![Expr::int(1), Expr::int(2)],
            })],
        ))
        .unwrap();
        let error = unit.validate().unwrap_err();
        assert_eq!(error.kind(), LowerErrorKind::ArityMismatch);
        assert!(error.message().contains('1') && error.message().contains('2'));
    }

    #[test]
    fn validate_finds_calls_nested_inside_expressions() {
        let mut unit = Unit::new();
        unit.add_function(func(
            "caller",
            vec![],
            Ty::Int,
            vec![Stmt::Return(Expr::binary(
                BinOp::Add,
                Expr::int(1),
                Expr::Call {
                    callee: "missing".into(),
                    args: vec![],
                },
            ))],
        ))
        .unwrap();
        let error = unit.validate().unwrap_err();
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
    }
}
