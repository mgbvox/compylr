//! How a type or an operator is written in Python.
//!
//! These used to be `Ty::python_name` and `BinOp::python_symbol`, on the IR — in a module whose
//! own doc comment said no source language appears there. The point of a diagnostic string is to
//! echo what the programmer wrote, which makes it the frontend's, and the IR having one meant
//! every backend's diagnostics inherited Python's vocabulary for free.
//!
//! Extension traits rather than free functions, so `ty.python_name()` at a hundred call sites in
//! lowering reads exactly as it did.

use compylr_ir::{BinOp, DivMode, RemSign, Rounding, Ty};

/// The Python spelling of a type, for diagnostics.
pub trait PythonTypeName {
    /// The annotation a Python programmer would have written for this type.
    fn python_name(&self) -> String;
}

impl PythonTypeName for Ty {
    fn python_name(&self) -> String {
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
}

/// The Python spelling of an operator, for diagnostics.
pub trait PythonOperator {
    /// The symbol a Python programmer would have written for this operator.
    fn python_symbol(self) -> String;
}

impl PythonOperator for BinOp {
    fn python_symbol(self) -> String {
        match self {
            Self::Add => "+".to_string(),
            Self::Sub => "-".to_string(),
            Self::Mul => "*".to_string(),
            Self::Div {
                mode: DivMode::Exact,
            } => "/".to_string(),
            Self::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
            } => "//".to_string(),
            Self::Eq => "==".to_string(),
            Self::NotEq => "!=".to_string(),
            Self::Lt => "<".to_string(),
            Self::LtE => "<=".to_string(),
            Self::Gt => ">".to_string(),
            Self::GtE => ">=".to_string(),
            Self::Rem {
                sign: RemSign::Divisor,
            } => "%".to_string(),
            // Python has no syntax for these modes and this frontend never produces them, so
            // there is no spelling to give back. Falling through to the IR's own neutral
            // rendering is more honest than inventing a symbol that would send a reader looking
            // for an operator their language does not have.
            other => other.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_are_spelled_the_way_the_programmer_wrote_them() {
        assert_eq!(Ty::Int.python_name(), "int");
        assert_eq!(Ty::Unit.python_name(), "None");
        assert_eq!(
            Ty::Dict(Box::new(Ty::Str), Box::new(Ty::Int)).python_name(),
            "dict[str, int]"
        );
        assert_eq!(
            Ty::List(Box::new(Ty::Instance("Counter".into()))).python_name(),
            "list[Counter]"
        );
    }

    #[test]
    fn the_operators_python_has_are_spelled_as_python_writes_them() {
        assert_eq!(
            BinOp::Div {
                mode: DivMode::Exact,
            }
            .python_symbol(),
            "/"
        );
        assert_eq!(
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
            }
            .python_symbol(),
            "//"
        );
        assert_eq!(
            BinOp::Rem {
                sign: RemSign::Divisor,
            }
            .python_symbol(),
            "%"
        );
    }

    /// A mode Python cannot write must not be given a Python symbol.
    ///
    /// Spelling truncating division `//` would be worse than useless: it names an operator that
    /// exists and means something else.
    #[test]
    fn a_mode_python_cannot_write_falls_back_to_the_neutral_name() {
        let truncating = BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
        };
        assert_ne!(truncating.python_symbol(), "//");
        assert_ne!(truncating.python_symbol(), "/");
        assert!(truncating.python_symbol().contains("toward zero"));
    }
}
