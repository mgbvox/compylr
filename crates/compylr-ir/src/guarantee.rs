//! Semantic guarantees a source language needs preserved on the way to a target.
//!
//! Lives with the IR rather than with the component model because a [`crate::Unit`] records what
//! its frontend requires, and the IR cannot depend on the crate that consumes it. `compylr-core`
//! re-exports this type, so a frontend or backend declaring guarantees still names one type.
//!
//! These exist because "compiled" is not the same as "means the same thing". A Python function
//! that reports an overflow and a Rust one that wraps are both valid programs; only one of them
//! is a translation of the other. A frontend states what must survive, a backend states what it
//! preserves, and a combination that would silently change meaning is refused by name rather
//! than discovered by a wrong answer at runtime.
//!
//! The set is small on purpose. Each member is here because a real target option would violate
//! it: overflow checks can be switched off, division by zero is undefined behavior in several
//! targets, and float reassociation is what a fast-math flag buys.

use std::fmt;

use serde::{Deserialize, Serialize};

/// One property that must hold of generated code for it to mean what the source meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Guarantee {
    /// An arithmetic result outside the target's integer range is reported, not wrapped or
    /// truncated.
    IntegerOverflowReported,
    /// Division by zero is reported, rather than trapping the process or being undefined.
    DivisionByZeroReported,
    /// Floating-point arithmetic is evaluated as written, without reassociation.
    FloatOrderPreserved,
}

impl Guarantee {
    /// A stable identifier for this guarantee, for callers that branch on it.
    ///
    /// Separate from this type's `Display` for the same reason error kinds carry a code: prose is
    /// free to be reworded, and anything acting on the value needs something that does not move.
    pub fn code(self) -> &'static str {
        match self {
            Self::IntegerOverflowReported => "integer_overflow_reported",
            Self::DivisionByZeroReported => "division_by_zero_reported",
            Self::FloatOrderPreserved => "float_order_preserved",
        }
    }
}

impl fmt::Display for Guarantee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::IntegerOverflowReported => "integer overflow is reported rather than wrapped",
            Self::DivisionByZeroReported => "division by zero is reported",
            Self::FloatOrderPreserved => "floating-point arithmetic is not reordered",
        };
        f.write_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_guarantee_has_a_distinct_code_and_wording() {
        let all = [
            Guarantee::IntegerOverflowReported,
            Guarantee::DivisionByZeroReported,
            Guarantee::FloatOrderPreserved,
        ];
        let mut codes: Vec<&str> = all.iter().map(|g| g.code()).collect();
        codes.sort_unstable();
        let count = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), count, "codes must be distinct");

        let mut words: Vec<String> = all.iter().map(|g| g.to_string()).collect();
        words.sort();
        let count = words.len();
        words.dedup();
        assert_eq!(words.len(), count, "wordings must be distinct");
    }

    #[test]
    fn a_code_is_not_the_wording() {
        // The point of the code is that rewording the prose does not move it.
        assert_ne!(
            Guarantee::FloatOrderPreserved.code(),
            Guarantee::FloatOrderPreserved.to_string()
        );
    }
}
