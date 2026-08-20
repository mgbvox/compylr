//! Well-formedness checking, between lowering and everything downstream.
//!
//! Runs unconditionally and knows no source language. That is the point: the Python frontend
//! enforces these invariants itself while lowering, so for Python this stage never fires. It
//! exists for the *second* frontend, which will not have re-derived them — and whose mistakes
//! would otherwise surface as a backend complaining about the target language rather than as a
//! diagnostic about the program.
//!
//! The checks are deliberately the ones whose absence produces generated code that does not
//! compile. A tree that merely computes something odd is the program's business; a tree that
//! cannot be rendered is the compiler's.

use std::error::Error;
use std::fmt;

use compylr_ir::{Unit, returns_on_all_paths};

/// A unit that cannot be rendered by any backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationError {
    /// Which member the problem is in.
    member: String,
    /// What is wrong with it.
    detail: String,
}

impl VerificationError {
    /// The unit member the problem is in.
    pub fn member(&self) -> &str {
        &self.member
    }

    /// What is wrong with it.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' is not well formed: {}", self.member, self.detail)
    }
}

impl Error for VerificationError {}

/// Check that a unit is renderable.
///
/// Two checks today, both chosen because failing them produces target source that does not build:
///
/// * every call resolves, with matching arity — an unresolved call has no name to emit;
/// * every function declaring a value returns one on every path — otherwise the emitted function
///   falls off its end.
///
/// Neither consults the unit's origin. A malformed unit is malformed whichever frontend produced
/// it, and a verifier that answered differently per language would be a second place for the
/// subset to be defined.
pub fn verify(unit: &Unit) -> Result<(), VerificationError> {
    unit.validate().map_err(|error| VerificationError {
        member: "unit".to_string(),
        detail: error.message().to_string(),
    })?;

    for function in unit
        .functions()
        .chain(unit.classes().flat_map(|c| c.functions()))
    {
        if function.ret != compylr_ir::Ty::Unit && !returns_on_all_paths(&function.body) {
            return Err(VerificationError {
                member: function.name.clone(),
                detail: format!(
                    "it declares a return type of {} but does not return a value on every path",
                    function.ret
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use compylr_diagnostics::span::Span;
    use compylr_ir::{Expr, Function, Literal, Param, Stmt, Ty};

    fn function(name: &str, ret: Ty, body: Vec<Stmt>) -> Function {
        Function {
            name: name.to_string(),
            params: vec![Param {
                name: "n".to_string(),
                ty: Ty::Int,
            }],
            ret,
            body,
            doc: None,
            span: Span::default(),
        }
    }

    #[test]
    fn a_well_formed_unit_passes() {
        let mut unit = Unit::new();
        unit.add_function(function(
            "answer",
            Ty::Int,
            vec![Stmt::Return(Expr::Literal(Literal::Int(42)))],
        ))
        .unwrap();
        assert!(verify(&unit).is_ok());
    }

    #[test]
    fn an_unresolved_call_is_rejected() {
        let mut unit = Unit::new();
        unit.add_function(function(
            "caller",
            Ty::Int,
            vec![Stmt::Return(Expr::Call {
                callee: "nowhere".to_string(),
                args: vec![],
            })],
        ))
        .unwrap();

        let error = verify(&unit).expect_err("the callee is in no unit");
        assert!(error.to_string().contains("nowhere"), "{error}");
    }

    #[test]
    fn a_function_that_does_not_return_is_rejected() {
        let mut unit = Unit::new();
        unit.add_function(function("silent", Ty::Int, vec![]))
            .unwrap();

        let error = verify(&unit).expect_err("nothing is returned");
        assert_eq!(error.member(), "silent");
        assert!(error.detail().contains("every path"), "{error}");
    }

    #[test]
    fn a_unit_returning_function_needs_no_value() {
        let mut unit = Unit::new();
        unit.add_function(function("nothing", Ty::Unit, vec![]))
            .unwrap();
        assert!(verify(&unit).is_ok());
    }

    /// The same malformed unit must fail the same way whoever produced it.
    ///
    /// A verifier that consulted the origin would be a second place the accepted subset is
    /// defined, and the two would drift.
    #[test]
    fn the_verdict_does_not_depend_on_the_producing_frontend() {
        let mut base = Unit::new();
        base.add_function(function("silent", Ty::Int, vec![]))
            .unwrap();

        let mut python = base.clone();
        python.set_origin("python", &[]);
        let mut other = base.clone();
        other.set_origin("some-other-language", &[]);

        assert_eq!(verify(&base), verify(&python));
        assert_eq!(verify(&python), verify(&other));
        assert!(verify(&other).is_err());
    }
}
