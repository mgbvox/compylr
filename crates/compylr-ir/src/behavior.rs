//! Which language supplies the meaning, for each operation the two disagree about.
//!
//! Lives beside [`crate::guarantee`] and for the reason recorded there: a [`crate::Unit`] holds
//! the modes an axis resolves to, and the IR cannot depend on the crate that consumes it.
//! Resolution itself is deliberately *not* here — it needs two components, their names, and the
//! set of languages compylr knows, which is `compylr-core`'s business rather than the tree's.
//!
//! An **axis** is one operation a programmer writes for which a source language and a target
//! language read different meanings. Not every difference between two languages is an axis: a
//! difference in the *shape* of an operation is a different form in the IR rather than a setting
//! on one, which is why reading a mapping with an absent key has none. See the module doc on
//! [`crate::ir`] for the container behaviours that deliberately have no mode.
//!
//! The identifiers here are **neutral**, and that is load-bearing. `integer_division` rather than
//! `floor_div`, `text_length` rather than `len`: how an axis is spelled back to a programmer
//! belongs to the frontend that read their source, exactly as the Python spelling of a [`crate::Ty`]
//! does. A TypeScript host would name these same axes after `/`, `%`, and `.length` and resolve
//! against the same identifiers underneath.
//!
//! A [`LanguageBehavior`] describes **one language only**. Nothing here maps one language's stance
//! onto another's, and nothing may: that mapping is what would cost N x M as languages are added,
//! and avoiding it is the same property the component registries already have.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ir::{BinOp, Checked, DivMode, IndexOrigin, RemSign, Rounding, TextUnits};

/// One operation two languages can read differently.
///
/// The set is fixed and enumerable so that a user, a diagnostic, and a test all refer to the same
/// list. Adding one means adding a field to [`LanguageBehavior`] and an arm to
/// [`LanguageBehavior::stance`], neither of which compiles until it is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Axis {
    /// Addition, subtraction, multiplication, and negation of integers, when the result falls
    /// outside the integer range.
    IntegerOverflow,
    /// Division of two integers yielding an integer: which way it rounds, and what a zero divisor
    /// means.
    IntegerDivision,
    /// Division that promotes its operands and divides exactly: what a zero divisor means.
    ExactDivision,
    /// Remainder: which operand's sign the result takes, and what a zero divisor means.
    Remainder,
    /// Reading a collection at an index: how a negative offset resolves, and what an index outside
    /// the collection means.
    SequenceIndex,
    /// The length of a text value: what it counts.
    TextLength,
}

impl Axis {
    /// Every axis, in a fixed order.
    pub const ALL: [Axis; 6] = [
        Self::IntegerOverflow,
        Self::IntegerDivision,
        Self::ExactDivision,
        Self::Remainder,
        Self::SequenceIndex,
        Self::TextLength,
    ];

    /// A stable identifier, for callers that branch on the axis or accept it as text.
    ///
    /// Separate from this type's `Display` for the reason [`crate::Guarantee::code`] is: prose is
    /// free to be reworded, and a CLI flag or a host binding needs something that does not move.
    pub fn code(self) -> &'static str {
        match self {
            Self::IntegerOverflow => "integer_overflow",
            Self::IntegerDivision => "integer_division",
            Self::ExactDivision => "exact_division",
            Self::Remainder => "remainder",
            Self::SequenceIndex => "sequence_index",
            Self::TextLength => "text_length",
        }
    }

    /// The axis with this identifier, or `None` if there is none.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|axis| axis.code() == code)
    }

    /// Every identifier, for a diagnostic that has to list what would have been accepted.
    pub fn codes() -> Vec<&'static str> {
        Self::ALL.into_iter().map(Self::code).collect()
    }
}

impl fmt::Display for Axis {
    /// The operation, described rather than named.
    ///
    /// An axis is only meaningful as "the thing two languages disagree about", so the rendering
    /// names the operation and leaves the disagreement to whoever holds both stances.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::IntegerOverflow => "integer arithmetic outside the integer range",
            Self::IntegerDivision => "integer division",
            Self::ExactDivision => "exact division",
            Self::Remainder => "the remainder of a division",
            Self::SequenceIndex => "reading a collection at an index",
            Self::TextLength => "the length of a text value",
        };
        f.write_str(text)
    }
}

/// What a language means by integer division.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntegerDivision {
    /// Which way a result that is not exact rounds.
    pub rounding: Rounding,
    /// Whether the program defines what a zero divisor does.
    pub checked: Checked,
}

/// What a language means by remainder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Remainder {
    /// Which operand's sign the result takes.
    pub sign: RemSign,
    /// Whether the program defines what a zero divisor does.
    pub checked: Checked,
}

/// What a language means by reading a collection at an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SequenceIndex {
    /// How a negative offset resolves.
    pub origin: IndexOrigin,
    /// Whether the program defines what an index outside the collection does.
    pub checked: Checked,
}

/// One axis's stance, carried without knowing which axis it belongs to.
///
/// Exists so that resolution is a loop over [`Axis::ALL`] rather than six hand-written copies of
/// the same choice. Six copies would work and would be six places to forget an axis in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stance {
    /// Stance on [`Axis::IntegerOverflow`].
    IntegerOverflow(Checked),
    /// Stance on [`Axis::IntegerDivision`].
    IntegerDivision(IntegerDivision),
    /// Stance on [`Axis::ExactDivision`].
    ExactDivision(Checked),
    /// Stance on [`Axis::Remainder`].
    Remainder(Remainder),
    /// Stance on [`Axis::SequenceIndex`].
    SequenceIndex(SequenceIndex),
    /// Stance on [`Axis::TextLength`].
    TextLength(TextUnits),
}

impl Stance {
    /// Which axis this stance is about.
    pub fn axis(self) -> Axis {
        match self {
            Self::IntegerOverflow(_) => Axis::IntegerOverflow,
            Self::IntegerDivision(_) => Axis::IntegerDivision,
            Self::ExactDivision(_) => Axis::ExactDivision,
            Self::Remainder(_) => Axis::Remainder,
            Self::SequenceIndex(_) => Axis::SequenceIndex,
            Self::TextLength(_) => Axis::TextLength,
        }
    }
}

/// What one language means, on every axis.
///
/// Complete by construction: every field is required, so a language that answered five of the six
/// questions is not a value this type can hold. That is the point — a partial declaration would
/// have to be completed by somebody, and whoever completed it would be writing one language's
/// assumptions into another's.
///
/// Describes **one language**. Nothing here refers to another language's meanings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LanguageBehavior {
    /// What arithmetic outside the integer range means.
    pub integer_overflow: Checked,
    /// What integer division means.
    pub integer_division: IntegerDivision,
    /// What a zero divisor means for exact division.
    pub exact_division: Checked,
    /// What remainder means.
    pub remainder: Remainder,
    /// What reading a collection at an index means.
    pub sequence_index: SequenceIndex,
    /// What the length of a text value counts.
    pub text_length: TextUnits,
}

impl LanguageBehavior {
    /// This language's stance on one axis.
    ///
    /// Exhaustively matched rather than wildcarded, so that adding an [`Axis`] variant fails to
    /// compile here instead of silently returning some other axis's stance.
    pub fn stance(&self, axis: Axis) -> Stance {
        match axis {
            Axis::IntegerOverflow => Stance::IntegerOverflow(self.integer_overflow),
            Axis::IntegerDivision => Stance::IntegerDivision(self.integer_division),
            Axis::ExactDivision => Stance::ExactDivision(self.exact_division),
            Axis::Remainder => Stance::Remainder(self.remainder),
            Axis::SequenceIndex => Stance::SequenceIndex(self.sequence_index),
            Axis::TextLength => Stance::TextLength(self.text_length),
        }
    }
}

/// What one compilation means, on every axis.
///
/// The result of resolving a user's request against two languages' declarations. Distinct from
/// [`LanguageBehavior`] despite holding the same fields, because the two are different claims:
/// one says "this is what Rust means", the other says "this is what *this program* means".
/// Lowering takes the second, and a shared type could not catch the mistake of handing it the
/// first.
///
/// Total by construction, which is what lets every node's mode come from here: an axis left
/// undecided would be a node with no meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Behavior(LanguageBehavior);

impl Behavior {
    /// Every axis taking one language's stance.
    ///
    /// This is what "no request" resolves to, given the *source* language's declaration: a
    /// compilation nobody configured means what the source it was written in means.
    pub fn of(language: &LanguageBehavior) -> Self {
        Self(*language)
    }

    /// The same behavior with one axis replaced.
    pub fn with(mut self, stance: Stance) -> Self {
        match stance {
            Stance::IntegerOverflow(value) => self.0.integer_overflow = value,
            Stance::IntegerDivision(value) => self.0.integer_division = value,
            Stance::ExactDivision(value) => self.0.exact_division = value,
            Stance::Remainder(value) => self.0.remainder = value,
            Stance::SequenceIndex(value) => self.0.sequence_index = value,
            Stance::TextLength(value) => self.0.text_length = value,
        }
        self
    }

    /// This behavior's stance on one axis.
    pub fn stance(&self, axis: Axis) -> Stance {
        self.0.stance(axis)
    }

    /// The stances, as the same bundle a language declares.
    pub fn axes(&self) -> &LanguageBehavior {
        &self.0
    }

    /// The checking mode for addition, subtraction, multiplication, and negation of integers.
    pub fn arithmetic(&self) -> Checked {
        self.0.integer_overflow
    }

    /// The operator for a division that promotes its operands and divides exactly.
    ///
    /// The mode is always [`DivMode::Exact`]. What a zero divisor means is a behavior question;
    /// whether `/` promotes is a *typing* question, and that answer does not move. A behavior that
    /// changed it would make the same annotated source type-check under one setting and fail under
    /// another, and the annotations are the one thing this subset insists on.
    pub fn exact_division(&self) -> BinOp {
        BinOp::Div {
            mode: DivMode::Exact,
            checked: self.0.exact_division,
        }
    }

    /// The operator for a division of two integers yielding an integer.
    pub fn integer_division(&self) -> BinOp {
        BinOp::Div {
            mode: DivMode::Integer(self.0.integer_division.rounding),
            checked: self.0.integer_division.checked,
        }
    }

    /// The operator for a remainder.
    pub fn remainder(&self) -> BinOp {
        BinOp::Rem {
            sign: self.0.remainder.sign,
            checked: self.0.remainder.checked,
        }
    }

    /// How a negative offset into a collection resolves.
    pub fn index_origin(&self) -> IndexOrigin {
        self.0.sequence_index.origin
    }

    /// Whether the program defines what reading outside a collection does.
    pub fn index_checked(&self) -> Checked {
        self.0.sequence_index.checked
    }

    /// What the length of a text value counts.
    pub fn text_units(&self) -> TextUnits {
        self.0.text_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bundle with a stance on every axis, for tests that only need *a* behavior.
    ///
    /// Deliberately not any real language's: a fixture that happened to be Python's would let a
    /// test pass for the wrong reason.
    fn sample() -> LanguageBehavior {
        LanguageBehavior {
            integer_overflow: Checked::Reported,
            integer_division: IntegerDivision {
                rounding: Rounding::TowardNegInf,
                checked: Checked::Reported,
            },
            exact_division: Checked::Unchecked,
            remainder: Remainder {
                sign: RemSign::Dividend,
                checked: Checked::Reported,
            },
            sequence_index: SequenceIndex {
                origin: IndexOrigin::FromStart,
                checked: Checked::Unchecked,
            },
            text_length: TextUnits::Utf16Units,
        }
    }

    #[test]
    fn there_are_exactly_six_axes() {
        assert_eq!(Axis::ALL.len(), 6);
        assert_eq!(Axis::codes().len(), 6);
    }

    #[test]
    fn every_axis_has_a_distinct_stable_identifier() {
        let mut codes = Axis::codes();
        let count = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), count, "identifiers must be distinct");

        for axis in Axis::ALL {
            assert_eq!(Axis::from_code(axis.code()), Some(axis));
        }
        assert_eq!(Axis::from_code("floor_div"), None);
        assert_eq!(Axis::from_code(""), None);
    }

    /// The identifier is what a flag and a host binding key on; the prose is free to be reworded.
    #[test]
    fn an_identifier_is_not_the_prose() {
        for axis in Axis::ALL {
            assert_ne!(axis.code(), axis.to_string());
        }

        let mut words: Vec<String> = Axis::ALL.iter().map(Axis::to_string).collect();
        let count = words.len();
        words.sort();
        words.dedup();
        assert_eq!(words.len(), count, "wordings must be distinct");
    }

    /// Neutral identifiers, not one language's names for its own operators.
    ///
    /// `floor_div` and `text_len` are Python's spellings and belong to the Python frontend. If
    /// they appear here, the split design D4 draws has been lost.
    #[test]
    fn the_identifiers_name_no_language() {
        for code in Axis::codes() {
            for borrowed in [
                "floor_div",
                "true_div",
                "modulo",
                "text_len",
                "len",
                "index",
            ] {
                assert_ne!(code, borrowed, "'{code}' is a language's own spelling");
            }
        }
    }

    /// A stance bundle covers every axis, and cannot be constructed covering fewer.
    ///
    /// Destructured rather than checked field by field: that is the only way to state "complete by
    /// construction" as a test, because a bundle missing a field is a compile error rather than a
    /// value a test could be handed. Adding an axis without adding a field here — or the reverse —
    /// fails to compile.
    #[test]
    fn a_stance_bundle_is_complete_by_construction() {
        let LanguageBehavior {
            integer_overflow: _,
            integer_division: _,
            exact_division: _,
            remainder: _,
            sequence_index: _,
            text_length: _,
        } = sample();

        // And every axis is answerable off it, which is the same completeness stated from the
        // other side: `stance` is an exhaustive match, so an axis with no field cannot compile.
        for axis in Axis::ALL {
            assert_eq!(sample().stance(axis).axis(), axis);
        }
    }

    #[test]
    fn a_resolved_behavior_starts_as_one_languages_stance() {
        let behavior = Behavior::of(&sample());
        for axis in Axis::ALL {
            assert_eq!(behavior.stance(axis), sample().stance(axis));
        }
        assert_eq!(behavior.axes(), &sample());
    }

    #[test]
    fn replacing_one_axis_leaves_the_others_alone() {
        let behavior = Behavior::of(&sample()).with(Stance::TextLength(TextUnits::Utf8Bytes));

        assert_eq!(behavior.text_units(), TextUnits::Utf8Bytes);
        for axis in Axis::ALL {
            if axis == Axis::TextLength {
                continue;
            }
            assert_eq!(behavior.stance(axis), sample().stance(axis));
        }
    }

    /// The accessors lowering reads are assembled from the axes, not from a second table.
    #[test]
    fn the_lowering_accessors_report_what_the_axes_say() {
        let behavior = Behavior::of(&sample());

        assert_eq!(behavior.arithmetic(), Checked::Reported);
        assert_eq!(
            behavior.exact_division(),
            BinOp::Div {
                mode: DivMode::Exact,
                checked: Checked::Unchecked,
            }
        );
        assert_eq!(
            behavior.integer_division(),
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            }
        );
        assert_eq!(
            behavior.remainder(),
            BinOp::Rem {
                sign: RemSign::Dividend,
                checked: Checked::Reported,
            }
        );
        assert_eq!(behavior.index_origin(), IndexOrigin::FromStart);
        assert_eq!(behavior.index_checked(), Checked::Unchecked);
        assert_eq!(behavior.text_units(), TextUnits::Utf16Units);
    }

    /// Exact division promotes under every behavior; only its zero divisor is an axis.
    ///
    /// Design D10: `/` keeping its float result type is what makes acceptance independent of
    /// behavior. If the mode ever moved off `Exact`, the same annotated source would type-check
    /// under one behavior and fail under another.
    #[test]
    fn exact_division_stays_exact_whichever_stance_it_takes() {
        for checked in [Checked::Reported, Checked::Unchecked] {
            let language = LanguageBehavior {
                exact_division: checked,
                ..sample()
            };
            let BinOp::Div { mode, checked: got } = Behavior::of(&language).exact_division() else {
                panic!("exact division must lower to a division");
            };
            assert_eq!(mode, DivMode::Exact);
            assert_eq!(got, checked);
        }
    }
}
