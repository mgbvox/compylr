//! Error reporting and diagnostic conversion for TypeScript lowering.

use compylr_core::frontend::LoweringError;

/// A category of construct the TypeScript frontend rejects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Syntax,
    UnsupportedType,
    UnsupportedStatement,
    UnsupportedExpression,
    MissingAnnotation,
    MissingReturn,
    ParameterMutation,
    TypeMismatch,
    ControlFlow,
}

impl Category {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::UnsupportedType => "unsupported_type",
            Self::UnsupportedStatement => "unsupported_statement",
            Self::UnsupportedExpression => "unsupported_expression",
            Self::MissingAnnotation => "missing_annotation",
            Self::MissingReturn => "missing_return",
            Self::ParameterMutation => "parameter_mutation",
            Self::TypeMismatch => "type_mismatch",
            Self::ControlFlow => "control_flow",
        }
    }
}

pub fn unsupported(
    category: Category,
    message: impl Into<String>,
    line: usize,
    column: usize,
) -> LoweringError {
    LoweringError::Unsupported {
        message: message.into(),
        code: category.as_code(),
        line,
        column,
    }
}

pub fn syntax(message: impl Into<String>, line: usize, column: usize) -> LoweringError {
    LoweringError::Syntax {
        message: message.into(),
        line,
        column,
    }
}
