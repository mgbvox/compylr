//! The Python frontend as a registered component.
//!
//! Everything below this is the same lowering that has always run; what changes is that it is
//! reached through [`compylr_core::Frontend`] rather than by a caller knowing which functions to
//! call in which order. That assembly — parse everything, union the names, gather the signatures,
//! then lower — is Python's typing rules, and it belongs to Python rather than to whoever asked
//! for a compilation.

use compylr_core::{Frontend, Guarantee, LanguageBehavior, LoweringError, Source};
use compylr_diagnostics::error::LowerError;
use compylr_ir::{
    Checked, IndexOrigin, IntegerDivision, RemSign, Remainder, Rounding, SequenceIndex, TextUnits,
    Unit,
};

use crate::error::SourceError;
use crate::frontend::parse_source;
use crate::lower::{
    ClassNames, ClassSignatures, Signatures, collect_class_names, collect_class_signatures,
    collect_signatures, lower_source_members_with,
};

/// The Python frontend.
#[derive(Debug)]
pub struct PythonFrontend;

/// What Python needs a target to preserve.
///
/// Each of these is a place a target option would silently change what a compiled function
/// means: wrapping arithmetic instead of reporting overflow, treating division by zero as
/// undefined, or reassociating floating-point arithmetic. Python does none of those, so a
/// backend that does is not translating Python.
const PYTHON_REQUIRES: &[Guarantee] = &[
    Guarantee::IntegerOverflowReported,
    Guarantee::DivisionByZeroReported,
    Guarantee::FloatOrderPreserved,
];

/// What Python means, on every behavior axis.
///
/// The **source of truth** for how Python reads each of these operations. Lowering sets a node's
/// modes from the resolved behavior, and the resolved behavior takes these when an axis resolves
/// to Python — so there is exactly one place that says `-7 // 2` is `-4`.
///
/// Describes Python and nothing else. That is not a stylistic constraint: a stance mentioning
/// Rust here would be the first row of a table that costs one entry per language *pair*, and the
/// point of two separate declarations is that a third language costs one declaration.
///
/// Every axis is the reporting or Python-flavoured answer, which is why `PY_CHECKED` in `lower.rs`
/// could be one constant rather than five. A language whose answers differed per axis — and Rust
/// is not one either — would still be expressible, because the axes are carried separately.
pub const PYTHON_BEHAVIOR: LanguageBehavior = LanguageBehavior {
    // `i64::MAX + 1` raises rather than wrapping.
    integer_overflow: Checked::Reported,
    // `-7 // 2` is `-4`, and `1 // 0` raises.
    integer_division: IntegerDivision {
        rounding: Rounding::TowardNegInf,
        checked: Checked::Reported,
    },
    // `1.0 / 0.0` raises rather than yielding an infinity.
    exact_division: Checked::Reported,
    // `-7 % 2` is `1`, and `1 % 0` raises.
    remainder: Remainder {
        sign: RemSign::Divisor,
        checked: Checked::Reported,
    },
    // `xs[-1]` is the last element, and reading past the end raises.
    sequence_index: SequenceIndex {
        origin: IndexOrigin::FromEitherEnd,
        checked: Checked::Reported,
    },
    // `len("é")` is 1.
    text_length: TextUnits::CodePoints,
};

impl Frontend for PythonFrontend {
    fn name(&self) -> &'static str {
        "python"
    }

    fn requires(&self) -> &'static [Guarantee] {
        PYTHON_REQUIRES
    }

    fn behavior(&self) -> &'static LanguageBehavior {
        &PYTHON_BEHAVIOR
    }

    fn lower(&self, sources: &[Source]) -> Result<Unit, LoweringError> {
        // Every source is parsed before any is lowered, so signatures can be gathered across all
        // of them. The decorator submits each function as its own source, which makes a call
        // between two decorated functions a call *across* sources; without this, such a call
        // could never be typed and `doubled = double(n)` would demand an annotation in exactly
        // the arrangement the decorator always produces.
        let mut parsed_sources = Vec::with_capacity(sources.len());
        for source in sources {
            let parsed =
                parse_source(&source.text).map_err(|error| syntax_error(&error, &source.text))?;
            parsed_sources.push((source, parsed));
        }

        // Names are unioned across every source first, for the same reason: a construction of a
        // class defined in another source has to resolve.
        let mut class_names = ClassNames::new();
        for (_, parsed) in &parsed_sources {
            class_names.extend(collect_class_names(parsed));
        }
        let mut signatures = Signatures::new();
        let mut class_signatures = ClassSignatures::new();
        for (_, parsed) in &parsed_sources {
            signatures.extend(collect_signatures(parsed, &class_names));
            class_signatures.extend(collect_class_signatures(parsed, &class_names));
        }

        // Each source is lowered under **its own** behavior. Two members of one project may
        // disagree about what `-7 // 2` means, and a call between them is still an ordinary call:
        // the meanings ride on the nodes, so one unit holds both without any of it being special.
        let mut unit = Unit::new();
        for (source, parsed) in &parsed_sources {
            let (functions, classes) =
                lower_source_members_with(parsed, &signatures, &class_signatures, source.behavior)
                    .map_err(|error| unsupported(&error, &source.text))?;
            for function in functions {
                unit.add_function(function)
                    .map_err(|error| unsupported(&error, &source.text))?;
            }
            for class in classes {
                unit.add_class(class)
                    .map_err(|error| unsupported(&error, &source.text))?;
            }
        }

        // Claimed last, so a unit that failed to lower never carries an origin it did not earn.
        unit.set_origin(self.name());
        Ok(unit)
    }
}

/// Resolve a parse failure against the source it came from.
///
/// The location is resolved here rather than handed on as a byte offset, because this is the
/// last place that holds the text. Downstream there is a `Unit` and no way to turn an offset
/// into anything a person can act on.
fn syntax_error(error: &SourceError, source: &str) -> LoweringError {
    match error {
        SourceError::Syntax { message, span } => {
            let at = span.line_column(source);
            LoweringError::Syntax {
                message: message.clone(),
                line: at.line,
                column: at.column,
            }
        }
        // `parse_source` reads no files, so an I/O variant cannot arise from it. Reported rather
        // than unreachable-panicked: a compiler that aborts while reporting an error is worse
        // than one that reports it slightly oddly.
        SourceError::Io { path, source } => LoweringError::Syntax {
            message: format!("could not read {}: {source}", path.display()),
            line: 1,
            column: 1,
        },
    }
}

fn unsupported(error: &LowerError, source: &str) -> LoweringError {
    let at = error.span().line_column(source);
    LoweringError::Unsupported {
        message: error.message().to_string(),
        code: error.kind().code(),
        line: at.line,
        column: at.column,
    }
}
