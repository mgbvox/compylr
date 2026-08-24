//! The compylr intermediate representation.
//!
//! The IR is independent of both Python and any target language. Nothing here names a Rust
//! type, a Go type, or a TypeScript type: a backend maps [`Ty`] onto whatever its target
//! spells, which is what keeps a second backend from requiring a second IR. Nothing here names a
//! Python construct either: how a type or an operator is spelled *back to the programmer* belongs
//! to the frontend that read it.
//!
//! Two consequences are easy to miss and worth stating up front:
//!
//! * **Operators carry the semantics a frontend declared**, not one language's by default.
//!   [`BinOp::Div`] carries a mode — exact, or integer with a rounding direction — [`BinOp::Rem`]
//!   carries which operand's sign the result takes, [`Expr::Subscript`] carries how a negative
//!   offset resolves, and [`Expr::Len`] carries what a text value is counted in. Python declares
//!   one reading of each; Go, C++, and TypeScript would declare others. A backend that read the
//!   node's *name* instead of its mode would be silently wrong for any frontend meaning the other
//!   thing, on exactly the inputs nobody writes a test for by accident.
//! * **IR values own their data.** Nothing borrows from the source text or the parse tree, so an
//!   IR value stays valid after both are dropped — which is what lets a unit accumulate
//!   functions parsed at different times.
//!
//! Three container behaviours are deliberately **not** carried as modes, and the absence is a
//! conclusion rather than an omission:
//!
//! * **Reading a mapping with an absent key** always reports the failure. Go yields the value
//!   type's zero and TypeScript yields `undefined`, but `v, ok := m[k]` is not `m[k]` with a
//!   setting — it is a different expression with a different result type, and expressing it would
//!   need a notion of a type's zero value this model does not have ([`Ty::Instance`] has none).
//!   A frontend that means it lowers to a different form, the way [`Expr::Range`] is a distinct
//!   form rather than a mode on a call.
//! * **Iterating a mapping** yields keys, which Python, Go's `range`, and TypeScript's `for...in`
//!   all agree on.
//! * **Membership in a string** tests substrings, which `in`, `strings.Contains`, `includes`, and
//!   `find` all agree on.

use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use compylr_diagnostics::error::{LowerError, LowerErrorKind};
use compylr_diagnostics::span::Span;

use crate::artifact::ArtifactError;
use crate::guarantee::Guarantee;

/// Format version of the on-disk artifact.
///
/// Recorded in every artifact and checked on load, so a file written by a future build fails
/// with an explanation rather than deserializing into a subtly wrong unit.
/// Version 4 adds the checking mode to the operations that can fail. **No reader for version 3 is
/// kept**: the only thing a v3 artifact could mean is "every failure reported", and a migration
/// asserting that would be more code than the one rebuild it saves. Every existing cache is
/// refused once and rebuilt, which `_state_is_current` already triggers on the recorded compylr
/// version.
const ARTIFACT_VERSION: u32 = 4;

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
    // Absent for a unit nobody claimed, which is what a hand-built one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin: Option<Origin>,
}

/// Which frontend produced a unit, and what its source language needs preserved.
///
/// Recorded on the unit so that a pass selected by source/target pair, and a backend deciding
/// whether an optimization is permitted, can both answer without re-deriving the source language
/// from the shape of the tree. Guessing it back out of the IR is exactly the coupling the IR
/// exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Origin {
    /// Registry name of the frontend that lowered this unit.
    pub frontend: String,
    /// What that frontend requires a target to preserve.
    ///
    /// Held on the unit rather than looked up from the frontend at use time, because an artifact
    /// read back from disk has no frontend to ask — and the requirements are a property of the
    /// program's meaning, which is what the artifact is for.
    pub requires: Vec<Guarantee>,
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

impl fmt::Display for Ty {
    /// A neutral rendering, in no source language and no target language.
    ///
    /// Deliberately not `int`/`str`: those are Python's spellings, and quoting a type back to a
    /// programmer in the words they wrote is the frontend's job. The IR needs a rendering for
    /// artifacts and debugging, and one that resembled a particular language would be borrowed by
    /// every diagnostic that had no better option — which is how the spellings ended up here in
    /// the first place.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => f.write_str("integer"),
            Self::Float => f.write_str("float"),
            Self::Bool => f.write_str("boolean"),
            Self::Str => f.write_str("string"),
            Self::Unit => f.write_str("unit"),
            Self::List(element) => write!(f, "sequence of {element}"),
            Self::Dict(key, value) => write!(f, "mapping from {key} to {value}"),
            Self::Set(element) => write!(f, "set of {element}"),
            Self::Tuple(elements) => {
                let inner: Vec<String> = elements.iter().map(Ty::to_string).collect();
                write!(f, "tuple of ({})", inner.join(", "))
            }
            Self::Instance(class) => write!(f, "instance of {class}"),
        }
    }
}

impl Ty {
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

/// Which way an integer division rounds a result that is not exact.
///
/// Both members are here because both are somebody's `/`: Python's `//` floors, and C, C++, Go,
/// Rust, and Java truncate. A node that did not say which it meant would be read as whichever
/// the reader's language uses, and the two disagree on exactly the inputs nobody tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rounding {
    /// Round down: `-7 / 2` is `-4`.
    TowardNegInf,
    /// Round toward zero: `-7 / 2` is `-3`.
    TowardZero,
}

/// Whether the program defines what happens when an operation fails.
///
/// **A statement about the program, not about the target.** `Unchecked` does not mean "wrap", or
/// "trap", or "do whatever Rust does" — it means the program declines to define the result, which
/// is a fact about the program that stays true whichever backend reads the unit. One target may
/// trap, another wrap, and a third do something else again; the unit is equally true of all
/// three, which is the property that lets a unit be legible without knowing who will consume it.
///
/// That framing is also what makes Rust's own split expressible at all. A mode named `Wrapping`
/// would be a lie in a debug build, where the same `+` panics; `Unchecked` is true in both.
///
/// Composes with the modes already on a node rather than replacing them. `Div { mode:
/// Integer(TowardNegInf), checked: Unchecked }` is a real combination — a flooring division whose
/// zero divisor is undefined — and a backend must still emit a flooring helper for it, because
/// no target's bare `/` floors *and* leaves the divisor undefined in the same operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Checked {
    /// The failure becomes a value the program can observe and handle.
    Reported,
    /// The program declines to define the result.
    Unchecked,
}

/// What a division does with its operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DivMode {
    /// Operands are promoted to floating point and divided exactly.
    ///
    /// This is the trap `/` sets when a frontend does not say what it means: in Python `7 / 2` is
    /// `3.5`, and in Rust, Go, and C++ the same spelling between two integers is integer division
    /// yielding `3`. Saying `Exact` on the node is what stops a backend from guessing.
    Exact,
    /// Operands divide as integers, rounding as stated.
    Integer(Rounding),
}

/// Which operand's sign a remainder takes.
///
/// Paired with [`Rounding`]: `Divisor` is the companion of [`Rounding::TowardNegInf`] and
/// `Dividend` of [`Rounding::TowardZero`], in the sense that `(a / b) * b + (a % b) == a` holds
/// within a pair and fails across one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemSign {
    /// The result takes the sign of the divisor: `-7 % 2` is `1`.
    Divisor,
    /// The result takes the sign of the dividend: `-7 % 2` is `-1`.
    Dividend,
}

/// How a negative offset into a sequence is resolved.
///
/// Python counts backwards from the end, so `xs[-1]` is the last element. Go, C++, Rust, and
/// TypeScript do not: a negative offset is out of range, undefined, or an enormous positive number.
/// Same operation, two conventions — the shape [`Rounding`] has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexOrigin {
    /// A negative offset counts backwards from the end: `xs[-1]` is the last element.
    FromEitherEnd,
    /// A negative offset is out of range.
    FromStart,
}

/// What the length of a text value counts.
///
/// Three readings, all of them somebody's: Python counts code points, Go's `len` counts UTF-8
/// bytes, and TypeScript's `.length` counts UTF-16 units. They agree on ASCII and disagree on
/// everything else, which is the same class of trap as mapping `//` onto `/` — correct on the
/// inputs a test is likely to use and silently wrong beyond them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextUnits {
    /// Unicode scalar values. `len("é")` is 1; a character outside the basic plane is 1.
    CodePoints,
    /// Bytes of the UTF-8 encoding. `len("é")` is 2; a character outside the basic plane is 4.
    Utf8Bytes,
    /// Units of the UTF-16 encoding. `len("é")` is 1; a character outside the basic plane is 2.
    Utf16Units,
}

/// A binary operator, carrying the semantics a frontend declared for it.
///
/// The operators that admit more than one reasonable reading carry which one they mean. A
/// frontend sets it to whatever its source language means; a backend reproduces exactly what the
/// node says, without knowing which frontend produced it. Anything else makes the IR's meaning a
/// property of the compiler rather than of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinOp {
    /// Addition.
    Add {
        /// Whether the program defines a result outside the integer range.
        checked: Checked,
    },
    /// Subtraction.
    Sub {
        /// Whether the program defines a result outside the integer range.
        checked: Checked,
    },
    /// Multiplication.
    Mul {
        /// Whether the program defines a result outside the integer range.
        checked: Checked,
    },
    /// Division, of the declared kind.
    Div {
        /// Whether this divides exactly or as integers, and how it rounds if it does.
        mode: DivMode,
        /// Whether the program defines a zero divisor.
        checked: Checked,
    },
    /// Remainder, taking the declared operand's sign.
    Rem {
        /// Which operand's sign the result takes.
        sign: RemSign,
        /// Whether the program defines a zero divisor.
        checked: Checked,
    },
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
}

impl fmt::Display for BinOp {
    /// A neutral rendering that states the declared semantics rather than a spelling.
    ///
    /// `//` is Python's way of writing one particular rounding mode; a Go frontend writes the
    /// same mode as `/`. Naming the mode rather than a symbol is what lets one rendering serve
    /// both, and quoting the programmer's own syntax back at them stays the frontend's job.
    ///
    /// The **checking mode is deliberately not rendered**, and it is the one wildcard in this
    /// file that is not an oversight. This rendering names an operation for a diagnostic about
    /// *types* — "cannot apply addition to a string and an integer" — and whether the program
    /// defines an overflow has no bearing on that complaint. The mode stays readable off the
    /// node, which is where anything acting on it should read it; a consumer that decides what to
    /// *emit* must match it, and the backend does.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add { .. } => f.write_str("addition"),
            Self::Sub { .. } => f.write_str("subtraction"),
            Self::Mul { .. } => f.write_str("multiplication"),
            Self::Div {
                mode: DivMode::Exact,
                ..
            } => f.write_str("exact division"),
            Self::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                ..
            } => f.write_str("integer division rounding toward negative infinity"),
            Self::Div {
                mode: DivMode::Integer(Rounding::TowardZero),
                ..
            } => f.write_str("integer division rounding toward zero"),
            Self::Rem {
                sign: RemSign::Divisor,
                ..
            } => f.write_str("remainder taking the sign of the divisor"),
            Self::Rem {
                sign: RemSign::Dividend,
                ..
            } => f.write_str("remainder taking the sign of the dividend"),
            Self::Eq => f.write_str("equality"),
            Self::NotEq => f.write_str("inequality"),
            Self::Lt => f.write_str("less than"),
            Self::LtE => f.write_str("less than or equal"),
            Self::Gt => f.write_str("greater than"),
            Self::GtE => f.write_str("greater than or equal"),
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
    ///
    /// Carries a checking mode for the same reason addition does: negating the least
    /// representable integer is the one input for which the result does not fit.
    Neg {
        /// What is being negated.
        value: Box<Expr>,
        /// Whether the program defines a result outside the integer range.
        checked: Checked,
    },
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
        /// The receiver's class, when lowering could determine it.
        ///
        /// Carried because a backend has to know whether the call mutates the receiver, and the
        /// method name alone does not say: two classes may both define `get`, one mutating and one
        /// not. `None` means the receiver's class was in another source, which lowering resolves at
        /// the unit; a backend must then assume the call mutates rather than guess.
        class: Option<String>,
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
    /// Reading one element of a collection, by offset or by key.
    Subscript {
        /// The collection being read.
        base: Box<Expr>,
        /// The index or key.
        index: Box<Expr>,
        /// How a negative offset is resolved.
        ///
        /// Inert when the index is a **key** rather than an offset: a mapping has no ends to count
        /// from. Carried on the node anyway rather than split into two forms, because indexing a
        /// sequence and looking one up in a mapping share a spelling in every language compylr
        /// accepts, and the type of the base is what already distinguishes them.
        origin: IndexOrigin,
        /// Whether the program defines a read that finds nothing.
        ///
        /// Unlike `origin`, this is **not** inert for a mapping. An offset outside a sequence and
        /// a key a mapping does not hold are the same question — whether the failure is a value
        /// the program can handle — even though only one of the two has ends to count from.
        checked: Checked,
    },
    /// The length of a collection or string.
    ///
    /// A distinct node rather than a call: a call is resolved against the unit during validation,
    /// so leaving `len` as one would make its meaning depend on whether someone had decorated a
    /// function of that name.
    Len {
        /// What is being measured.
        value: Box<Expr>,
        /// What a *text* value is counted in.
        ///
        /// Inert for a collection, whose length is a count of elements under every reading. Only
        /// text admits more than one answer, and it admits three.
        units: TextUnits,
    },
    /// A counted sequence of integers: `start`, then `start + step`, and so on while the value
    /// has not reached `stop`. Half-open — `stop` is excluded — and `step` may be negative but
    /// may not be zero.
    ///
    /// Not a Python feature. Go's three-clause `for` and C++'s `iota` lower to it just as
    /// naturally, and stating the contract here rather than citing one language's defaulting
    /// rules is what makes that true. All three components are always present, so no backend has
    /// to know what any frontend's source language leaves out.
    ///
    /// A distinct form rather than a call, for the reason [`Expr::Len`] is: a call is resolved
    /// against the unit, so leaving it as one would make its meaning depend on what else was
    /// compiled.
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

    /// Visit this expression and every one nested inside it.
    ///
    /// One traversal that everything needing to see the whole tree is built on, rather than a
    /// second hand-written match per question. The two that exist — finding calls, and asking
    /// what a program requires preserved — got the same set of forms wrong in the same way when
    /// they were written separately, because adding a form means remembering every walker.
    pub fn walk(&self, visit: &mut impl FnMut(&Expr)) {
        visit(self);
        match self {
            Self::Literal(_) | Self::Name(_) => {}
            // `ToFloat` must descend, or anything wrapped in a promotion is invisible.
            Self::Neg { value: inner, .. }
            | Self::ToFloat(inner)
            | Self::Not(inner)
            | Self::Len { value: inner, .. }
            | Self::TupleIndex { base: inner, .. }
            | Self::Attribute { object: inner, .. } => inner.walk(visit),
            Self::ListLit(items)
            | Self::SetLit(items)
            | Self::TupleLit(items)
            | Self::Construct { args: items, .. }
            | Self::Call { args: items, .. } => {
                for item in items {
                    item.walk(visit);
                }
            }
            Self::DictLit(pairs) => {
                for (key, value) in pairs {
                    key.walk(visit);
                    value.walk(visit);
                }
            }
            Self::MethodCall { receiver, args, .. } => {
                receiver.walk(visit);
                for arg in args {
                    arg.walk(visit);
                }
            }
            Self::Contains {
                value: left,
                container: right,
            }
            | Self::Subscript {
                base: left,
                index: right,
                ..
            }
            | Self::Binary { left, right, .. } => {
                left.walk(visit);
                right.walk(visit);
            }
            Self::Range { start, stop, step } => {
                start.walk(visit);
                stop.walk(visit);
                step.walk(visit);
            }
        }
    }

    /// Visit every call expression in this tree, including nested ones.
    ///
    /// Note what is **not** reported: a method call. It resolves against the receiver's class
    /// rather than against the unit, so demanding a free function of that name would reject code
    /// the user wrote correctly.
    pub fn walk_calls(&self, visit: &mut impl FnMut(&str, usize)) {
        self.walk(&mut |expr| {
            if let Self::Call { callee, args } = expr {
                visit(callee, args.len());
            }
        });
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

/// Visit every expression in a sequence of statements, descending into nested bodies.
///
/// Nested bodies are the whole reason this is a free function rather than a loop: an operation
/// inside a loop inside a branch is still an operation the program performs, and a walker that
/// stopped at the top level would report that a program requires less than it does — which is the
/// direction that silently permits a transformation the program forbids.
fn walk_stmt_exprs(stmts: &[Stmt], visit: &mut impl FnMut(&Expr)) {
    for stmt in stmts {
        match stmt {
            Stmt::Return(expr) | Stmt::Effect(expr) => expr.walk(visit),
            Stmt::Bind { value, .. } | Stmt::Assign { value, .. } => value.walk(visit),
            Stmt::SetAttr { object, value, .. } => {
                object.walk(visit);
                value.walk(visit);
            }
            Stmt::SetItem {
                collection,
                index,
                value,
            } => {
                collection.walk(visit);
                index.walk(visit);
                value.walk(visit);
            }
            Stmt::Append { sequence, value } => {
                sequence.walk(visit);
                value.walk(visit);
            }
            Stmt::If {
                test,
                then,
                otherwise,
            } => {
                test.walk(visit);
                walk_stmt_exprs(then, visit);
                walk_stmt_exprs(otherwise, visit);
            }
            Stmt::While { test, body } => {
                test.walk(visit);
                walk_stmt_exprs(body, visit);
            }
            Stmt::For { iter, body, .. } => {
                iter.walk(visit);
                walk_stmt_exprs(body, visit);
            }
            Stmt::ReturnUnit | Stmt::Break | Stmt::Continue => {}
        }
    }
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
    // `None` until a frontend claims the unit. A hand-built unit -- a test fixture, a conformance
    // corpus entry -- genuinely has no source language, and pretending it has one would make the
    // record useless for the case it exists to serve.
    origin: Option<Origin>,
}

impl Unit {
    /// Create an empty unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record which frontend produced this unit, and what **this program** requires preserved.
    ///
    /// Set by the frontend at the end of lowering rather than by the caller, so that a unit
    /// cannot claim an origin it does not have.
    ///
    /// The requirements are derived from the unit's own operations rather than copied from the
    /// frontend's list, which is what makes them a property of the program instead of the
    /// language. Two units from one frontend may require different things: the one whose
    /// arithmetic is unchecked does not need overflow reported, and that is exactly what makes a
    /// target option trading overflow a coherent thing to permit for it.
    ///
    /// Derived by walking rather than by mapping the resolved behavior, and the reason is not
    /// stylistic. A unit assembled from members lowered under *different* behaviors has no single
    /// behavior to map; walking asks the only question that always has an answer — what did this
    /// program actually ask for.
    pub fn set_origin(&mut self, frontend: impl Into<String>) {
        let requires = self.derived_requirements();
        self.origin = Some(Origin {
            frontend: frontend.into(),
            requires,
        });
    }

    /// What this unit's own operations ask a target to preserve.
    ///
    /// [`Guarantee::FloatOrderPreserved`] is contributed unconditionally and deliberately: there
    /// is no axis for it, because reassociation is a transformation a *backend* might apply
    /// rather than an operation a programmer wrote. There is nothing for a user to waive, so
    /// nothing to look for on a node.
    fn derived_requirements(&self) -> Vec<Guarantee> {
        let mut requires = vec![Guarantee::FloatOrderPreserved];
        let mut add = |guarantee: Guarantee| {
            if !requires.contains(&guarantee) {
                requires.push(guarantee);
            }
        };

        self.walk_exprs(&mut |expr| match expr {
            Expr::Neg {
                checked: Checked::Reported,
                ..
            } => add(Guarantee::IntegerOverflowReported),
            Expr::Binary { op, .. } => match op {
                BinOp::Add {
                    checked: Checked::Reported,
                }
                | BinOp::Sub {
                    checked: Checked::Reported,
                }
                | BinOp::Mul {
                    checked: Checked::Reported,
                } => add(Guarantee::IntegerOverflowReported),
                BinOp::Div {
                    checked: Checked::Reported,
                    ..
                }
                | BinOp::Rem {
                    checked: Checked::Reported,
                    ..
                } => add(Guarantee::DivisionByZeroReported),
                _ => {}
            },
            _ => {}
        });

        // Sorted so the recorded list does not depend on the order functions were added, for the
        // same reason the artifact holds them in a `BTreeMap`.
        requires.sort_unstable();
        requires
    }

    /// Visit every expression in every function and method this unit holds.
    fn walk_exprs(&self, visit: &mut impl FnMut(&Expr)) {
        for function in self.functions.values() {
            walk_stmt_exprs(&function.body, visit);
        }
        for class in self.classes.values() {
            walk_stmt_exprs(&class.init.body, visit);
            for method in class.methods.values() {
                walk_stmt_exprs(&method.body, visit);
            }
        }
    }

    /// The frontend that produced this unit, if one claimed it.
    pub fn origin(&self) -> Option<&Origin> {
        self.origin.as_ref()
    }

    /// What this unit's source language requires a target to preserve.
    ///
    /// Empty for an unclaimed unit: nothing is known about it, so nothing can be required. A
    /// backend checking against this therefore accepts a hand-built unit, which is what makes a
    /// conformance corpus runnable without inventing a source language for it.
    pub fn requires(&self) -> &[Guarantee] {
        self.origin.as_ref().map_or(&[], |origin| &origin.requires)
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

    /// Apply `edit` to every function in the unit, methods and constructors included.
    ///
    /// Exists for passes. A pass reaching into `functions` and `classes` separately is a pass
    /// that will one day be written against only the first, and a transformation that silently
    /// skipped method bodies would be a bug nothing in the type system catches.
    pub fn map_functions(&mut self, mut edit: impl FnMut(&mut Function)) {
        for function in self.functions.values_mut() {
            edit(function);
        }
        for class in self.classes.values_mut() {
            edit(&mut class.init);
            for method in class.methods.values_mut() {
                edit(method);
            }
        }
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

        // The origin is part of the compilation input, not decoration: two units with identical
        // bodies but different required guarantees can legitimately emit different code, and a
        // cache that could not tell them apart would hand back the wrong build. Contributing only
        // when present keeps a hand-built unit printing as it always did.
        if let Some(origin) = &self.origin {
            origin.hash(&mut hasher);
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
            origin: self.origin.clone(),
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
        unit.origin = artifact.origin;
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
        let names: Vec<String> = all.iter().map(Ty::to_string).collect();
        // Neutral names, not `int`/`str`/`None`: those are Python's, and rendering them here is
        // what let every diagnostic in the compiler borrow one language's vocabulary.
        assert_eq!(names, ["integer", "float", "boolean", "string", "unit"]);
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
    fn a_division_carries_which_kind_it_is() {
        let exact = BinOp::Div {
            mode: DivMode::Exact,
            checked: Checked::Reported,
        };
        let flooring = BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardNegInf),
            checked: Checked::Reported,
        };
        let truncating = BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
            checked: Checked::Reported,
        };

        // All three are `/` in some language. Distinguishable only because the node says which.
        assert_ne!(exact, flooring);
        assert_ne!(flooring, truncating);
        assert!(!exact.is_comparison());
    }

    #[test]
    fn a_remainder_carries_whose_sign_it_takes() {
        let divisor = BinOp::Rem {
            sign: RemSign::Divisor,
            checked: Checked::Reported,
        };
        let dividend = BinOp::Rem {
            sign: RemSign::Dividend,
            checked: Checked::Reported,
        };
        assert_ne!(divisor, dividend);
        assert!(!divisor.is_comparison());
    }

    /// The declared semantics must be readable from the node without any other context.
    #[test]
    fn an_operator_states_its_own_meaning() {
        assert!(
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            }
            .to_string()
            .contains("negative infinity")
        );
        assert!(
            BinOp::Rem {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
            }
            .to_string()
            .contains("divisor")
        );
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
            BinOp::Add {
                checked: Checked::Reported,
            },
            BinOp::Sub {
                checked: Checked::Reported,
            },
            BinOp::Mul {
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Exact,
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
            },
        ] {
            assert!(!op.is_comparison(), "{op:?} should not be a comparison");
        }
    }

    /// Two operators that render alike would be two operators a reader cannot tell apart.
    ///
    /// The list covers every division and remainder *mode*, not just every variant, because the
    /// modes are the whole point: `//` and Go's `/` are the same variant with different meanings,
    /// and a rendering that collapsed them would undo the change.
    #[test]
    fn every_operator_and_mode_renders_distinctly() {
        let ops = [
            BinOp::Add {
                checked: Checked::Reported,
            },
            BinOp::Sub {
                checked: Checked::Reported,
            },
            BinOp::Mul {
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Exact,
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardZero),
                checked: Checked::Reported,
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
            },
            BinOp::Rem {
                sign: RemSign::Dividend,
                checked: Checked::Reported,
            },
            BinOp::Eq,
            BinOp::NotEq,
            BinOp::Lt,
            BinOp::LtE,
            BinOp::Gt,
            BinOp::GtE,
        ];
        let mut rendered: Vec<String> = ops.iter().map(BinOp::to_string).collect();
        assert_eq!(rendered.len(), 14);
        rendered.sort();
        rendered.dedup();
        assert_eq!(rendered.len(), 14, "operator renderings must be distinct");
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
            BinOp::Mul {
                checked: Checked::Reported,
            },
            Expr::binary(
                BinOp::Add {
                    checked: Checked::Reported,
                },
                Expr::name("a"),
                Expr::int(1),
            ),
            Expr::Neg {
                value: Box::new(Expr::name("b")),
                checked: Checked::Reported,
            },
        );
        match &expr {
            Expr::Binary { op, left, right } => {
                assert_eq!(
                    *op,
                    BinOp::Mul {
                        checked: Checked::Reported
                    }
                );
                assert!(matches!(
                    **left,
                    Expr::Binary {
                        op: BinOp::Add {
                            checked: Checked::Reported
                        },
                        ..
                    }
                ));
                assert!(matches!(**right, Expr::Neg { .. }));
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
                BinOp::Add {
                    checked: Checked::Reported,
                },
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
