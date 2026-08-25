//! The TypeScript frontend as a registered component.

use compylr_core::{Frontend, Guarantee, LanguageBehavior, LoweringError, Source};
use compylr_ir::{
    Checked, IndexOrigin, IntegerDivision, RemSign, Remainder, Rounding, SequenceIndex, TextUnits,
    Unit,
};

use crate::lower::lower_typescript_sources;

/// The TypeScript frontend.
#[derive(Debug)]
pub struct TypeScriptFrontend;

/// What TypeScript needs a target to preserve.
const TYPESCRIPT_REQUIRES: &[Guarantee] = &[
    Guarantee::DivisionByZeroReported,
    Guarantee::FloatOrderPreserved,
];

/// What TypeScript means, on every behavior axis.
pub const TYPESCRIPT_BEHAVIOR: LanguageBehavior = LanguageBehavior {
    integer_overflow: Checked::Unchecked,
    integer_division: IntegerDivision {
        rounding: Rounding::TowardZero,
        checked: Checked::Reported,
    },
    exact_division: Checked::Reported,
    remainder: Remainder {
        sign: RemSign::Dividend,
        checked: Checked::Reported,
    },
    sequence_index: SequenceIndex {
        origin: IndexOrigin::FromEitherEnd,
        checked: Checked::Reported,
    },
    text_length: TextUnits::Utf16Units,
};

impl Frontend for TypeScriptFrontend {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn requires(&self) -> &'static [Guarantee] {
        TYPESCRIPT_REQUIRES
    }

    fn behavior(&self) -> &'static LanguageBehavior {
        &TYPESCRIPT_BEHAVIOR
    }

    fn lower(&self, sources: &[Source]) -> Result<Unit, LoweringError> {
        lower_typescript_sources(sources)
    }
}
