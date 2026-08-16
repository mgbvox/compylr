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
}

/// A type in the supported subset, described by meaning rather than by any target's spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
}

impl Ty {
    /// The Python annotation this type comes from, useful in diagnostics.
    pub fn python_name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Str => "str",
            Self::Unit => "None",
        }
    }

    /// Whether arithmetic is defined on this type.
    ///
    /// Booleans are deliberately excluded even though Python's `bool` subclasses `int`:
    /// accepting `True + 1` would force every backend to decide how a boolean widens, and
    /// would make `a + b` on two booleans mean integer addition, which reads as a bug in the
    /// languages compylr emits.
    pub fn is_numeric(self) -> bool {
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
            Self::Neg(inner) | Self::ToFloat(inner) => inner.walk_calls(visit),
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
    ///
    /// A binding always introduces a *new* name; reassignment is rejected during lowering, so a
    /// backend can render this as a plain immutable binding.
    Bind {
        /// Name being introduced.
        name: String,
        /// Type of the binding, declared or inferred from an alias.
        ty: Ty,
        /// Value bound to the name.
        value: Expr,
    },
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
        for stmt in &self.body {
            match stmt {
                Stmt::Return(expr) => expr.walk_calls(visit),
                Stmt::Bind { value, .. } => value.walk_calls(visit),
                Stmt::ReturnUnit => {}
            }
        }
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
}

impl Unit {
    /// Create an empty unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function, failing if one of that name is already present.
    pub fn add_function(&mut self, function: Function) -> Result<(), LowerError> {
        if let Some(existing) = self.functions.get(&function.name) {
            return Err(LowerError::new(
                LowerErrorKind::DuplicateFunction,
                format!(
                    "function '{}' is already defined in this unit",
                    existing.name
                ),
                function.span,
            ));
        }
        self.functions.insert(function.name.clone(), function);
        Ok(())
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
        for function in self.functions.values() {
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
        let names: Vec<&str> = all.iter().map(|t| t.python_name()).collect();
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
