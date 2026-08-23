//! Resolving a user's behavior request against the two languages in a compilation.
//!
//! The model itself — what an axis is, what a language declares — lives in `compylr-ir`, beside
//! the guarantees, because a unit holds the modes it resolves to. Resolution lives *here* because
//! it is the one operation that needs **two** components at once, which is what this crate is for.
//!
//! Nothing below names a concrete language. The two names arrive as strings on a
//! [`LanguagePair`], and the set of names compylr recognises arrives alongside them, because the
//! registry that knows every language depends on this crate rather than the other way round.
//!
//! The rejection is three-way, matching the registries':
//!
//! * a name compylr does not know at all — most likely a typo,
//! * a name it knows, registered or reserved, that is simply not one of the two *here*,
//! * an axis that does not exist.
//!
//! Folding the middle case into the first would tell a user who asked for Go in a Python-to-Rust
//! compilation that no such language exists, which is false and unhelpful. It is also the mistake
//! a user is most likely to make.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use compylr_ir::{Axis, Behavior, LanguageBehavior};

/// What a user asked for: a language per axis, with anything unnamed left to inherit.
///
/// A bare language name is not a separate shape — [`BehaviorRequest::language`] expands it to
/// every axis. That is what makes "`behavior='rust'` is every axis set to Rust" true by
/// construction rather than by two code paths agreeing, which is the same reasoning the Python
/// surface uses for normalising a bare string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BehaviorRequest {
    axes: BTreeMap<Axis, String>,
}

impl BehaviorRequest {
    /// A request naming nothing: every axis inherits.
    pub fn inherit() -> Self {
        Self::default()
    }

    /// A request naming one language for every axis.
    pub fn language(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            axes: Axis::ALL
                .into_iter()
                .map(|axis| (axis, name.clone()))
                .collect(),
        }
    }

    /// The same request with one more axis named.
    pub fn with(mut self, axis: Axis, language: impl Into<String>) -> Self {
        self.axes.insert(axis, language.into());
        self
    }

    /// A request from axis identifiers, as a CLI flag or a host binding supplies them.
    ///
    /// Fallible because the identifiers are text a user typed. An axis that does not exist is
    /// rejected here rather than ignored: silently dropping `floor_div=rust` would compile the
    /// program the user did not ask for and say nothing.
    pub fn from_pairs<I, K, V>(pairs: I) -> Result<Self, BehaviorError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        let mut request = Self::inherit();
        for (axis, language) in pairs {
            let axis = axis.as_ref();
            let Some(axis) = Axis::from_code(axis) else {
                return Err(BehaviorError::UnknownAxis {
                    name: axis.to_string(),
                    available: Axis::codes(),
                });
            };
            request = request.with(axis, language);
        }
        Ok(request)
    }

    /// Whether this request names nothing at all.
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }

    /// The language named for one axis, if any.
    pub fn language_for(&self, axis: Axis) -> Option<&str> {
        self.axes.get(&axis).map(String::as_str)
    }
}

/// The two languages a compilation runs between, and what each means.
///
/// Carries the names as well as the declarations because the rejection has to say which two would
/// have been accepted, and a declaration does not know its own name — deliberately, since a
/// [`LanguageBehavior`] describes meanings and naming itself would be the first step toward
/// naming somebody else.
#[derive(Debug, Clone, Copy)]
pub struct LanguagePair<'a> {
    /// Registry name of the source language.
    pub source: &'a str,
    /// What the source language means, on every axis.
    pub source_behavior: &'a LanguageBehavior,
    /// Registry name of the target language.
    pub target: &'a str,
    /// What the target language means, on every axis.
    pub target_behavior: &'a LanguageBehavior,
    /// Every language name compylr recognises, implemented or reserved.
    ///
    /// Supplied rather than looked up, because the registry that knows them depends on this
    /// crate. It is also what separates "not a language" from "not a language *here*".
    pub known: &'a [&'a str],
}

impl<'a> LanguagePair<'a> {
    /// This pair's declaration for one language name, or `None` if it is neither of the two.
    fn declaration(&self, name: &str) -> Option<&'a LanguageBehavior> {
        if name == self.source {
            Some(self.source_behavior)
        } else if name == self.target {
            Some(self.target_behavior)
        } else {
            None
        }
    }
}

/// Why a behavior request could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorError {
    /// A name compylr has no component for at all.
    UnknownLanguage {
        /// The name that was asked for.
        name: String,
        /// The source language of this compilation.
        source: String,
        /// The target language of this compilation.
        target: String,
    },
    /// A name compylr knows, but which is neither language of this compilation.
    NotInPair {
        /// The name that was asked for.
        name: String,
        /// The source language of this compilation.
        source: String,
        /// The target language of this compilation.
        target: String,
    },
    /// An axis that does not exist.
    UnknownAxis {
        /// The identifier that was asked for.
        name: String,
        /// The identifiers that would have worked.
        available: Vec<&'static str>,
    },
}

impl BehaviorError {
    /// A stable identifier for the category, so a caller branches without matching prose.
    ///
    /// The Python surface turns each of these into a different sentence, and it must not do so by
    /// reading the message it is about to replace.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownLanguage { .. } => "unknown_language",
            Self::NotInPair { .. } => "language_not_in_pair",
            Self::UnknownAxis { .. } => "unknown_axis",
        }
    }
}

impl fmt::Display for BehaviorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownLanguage {
                name,
                source,
                target,
            } => write!(
                f,
                "'{name}' is not a language compylr knows; this compilation goes from '{source}' \
                 to '{target}', and a behavior may name either of those"
            ),
            Self::NotInPair {
                name,
                source,
                target,
            } => write!(
                f,
                "'{name}' is a language compylr knows, but it is not one of the two in this \
                 compilation; this compilation goes from '{source}' to '{target}', and a behavior \
                 may name either of those"
            ),
            Self::UnknownAxis { name, available } => write!(
                f,
                "'{name}' is not a behavior axis; the axes are: {}",
                available.join(", ")
            ),
        }
    }
}

impl Error for BehaviorError {}

/// Resolve a request against the two languages, producing one stance per axis.
///
/// `default` is what an unnamed axis takes. `None` means the **source** language's stance, which
/// is the rule that makes a project mentioning no behavior compile exactly as it did before the
/// setting existed. A caller with an enclosing default — a decorator inheriting from its manager —
/// passes that instead, so naming one axis does not silently reset the other five.
pub fn resolve(
    request: &BehaviorRequest,
    pair: &LanguagePair<'_>,
    default: Option<&Behavior>,
) -> Result<Behavior, BehaviorError> {
    let mut resolved = default
        .copied()
        .unwrap_or_else(|| Behavior::of(pair.source_behavior));

    for axis in Axis::ALL {
        let Some(name) = request.language_for(axis) else {
            continue;
        };
        let Some(declaration) = pair.declaration(name) else {
            return Err(rejection(name, pair));
        };
        resolved = resolved.with(declaration.stance(axis));
    }
    Ok(resolved)
}

/// Which of the two rejections a name earns.
fn rejection(name: &str, pair: &LanguagePair<'_>) -> BehaviorError {
    let (name, source, target) = (
        name.to_string(),
        pair.source.to_string(),
        pair.target.to_string(),
    );
    if pair.known.contains(&name.as_str()) {
        BehaviorError::NotInPair {
            name,
            source,
            target,
        }
    } else {
        BehaviorError::UnknownLanguage {
            name,
            source,
            target,
        }
    }
}

#[cfg(test)]
mod tests {
    use compylr_ir::{
        Checked, IndexOrigin, IntegerDivision, RemSign, Remainder, Rounding, SequenceIndex,
        TextUnits,
    };

    use super::*;

    /// Two made-up languages that disagree on every axis.
    ///
    /// Deliberately not Python's and Rust's. This crate names no concrete language, and a test
    /// that reached for the real declarations would be asserting the frontend's behavior through
    /// the resolver — and would keep passing if resolution stopped consulting the declarations at
    /// all, because it would agree with them by coincidence.
    fn strict() -> LanguageBehavior {
        LanguageBehavior {
            integer_overflow: Checked::Reported,
            integer_division: IntegerDivision {
                rounding: Rounding::TowardNegInf,
                checked: Checked::Reported,
            },
            exact_division: Checked::Reported,
            remainder: Remainder {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
            },
            sequence_index: SequenceIndex {
                origin: IndexOrigin::FromEitherEnd,
                checked: Checked::Reported,
            },
            text_length: TextUnits::CodePoints,
        }
    }

    fn loose() -> LanguageBehavior {
        LanguageBehavior {
            integer_overflow: Checked::Unchecked,
            integer_division: IntegerDivision {
                rounding: Rounding::TowardZero,
                checked: Checked::Unchecked,
            },
            exact_division: Checked::Unchecked,
            remainder: Remainder {
                sign: RemSign::Dividend,
                checked: Checked::Unchecked,
            },
            sequence_index: SequenceIndex {
                origin: IndexOrigin::FromStart,
                checked: Checked::Unchecked,
            },
            text_length: TextUnits::Utf8Bytes,
        }
    }

    const KNOWN: &[&str] = &["strict", "loose", "elsewhere"];

    fn pair<'a>(source: &'a LanguageBehavior, target: &'a LanguageBehavior) -> LanguagePair<'a> {
        LanguagePair {
            source: "strict",
            source_behavior: source,
            target: "loose",
            target_behavior: target,
            known: KNOWN,
        }
    }

    fn resolved(request: &BehaviorRequest) -> Behavior {
        let (source, target) = (strict(), loose());
        resolve(request, &pair(&source, &target), None).expect("should resolve")
    }

    #[test]
    fn no_request_is_the_source_languages_stance() {
        let behavior = resolved(&BehaviorRequest::inherit());
        assert_eq!(behavior.axes(), &strict());
    }

    #[test]
    fn a_bare_language_name_sets_every_axis() {
        let behavior = resolved(&BehaviorRequest::language("loose"));
        assert_eq!(behavior.axes(), &loose());

        // And naming the source language is equally a full selection, not a no-op path.
        assert_eq!(
            resolved(&BehaviorRequest::language("strict")).axes(),
            &strict()
        );
    }

    /// A bare name and a full per-axis selection are the same request, by construction.
    #[test]
    fn the_two_spellings_agree() {
        let bare = resolved(&BehaviorRequest::language("loose"));

        let mut spelled_out = BehaviorRequest::inherit();
        for axis in Axis::ALL {
            spelled_out = spelled_out.with(axis, "loose");
        }
        assert_eq!(bare, resolved(&spelled_out));
    }

    #[test]
    fn an_unnamed_axis_inherits_the_enclosing_default() {
        let behavior = resolved(&BehaviorRequest::inherit().with(Axis::TextLength, "loose"));

        assert_eq!(behavior.text_units(), TextUnits::Utf8Bytes);
        for axis in Axis::ALL {
            if axis == Axis::TextLength {
                continue;
            }
            assert_eq!(
                behavior.stance(axis),
                strict().stance(axis),
                "{axis:?} should have kept the enclosing default"
            );
        }
    }

    /// The enclosing default is whatever was passed, not a fixed language.
    ///
    /// This is the case a decorator inheriting from its manager exercises: the manager's behavior
    /// is the target language, one axis is overridden back to the source, and the other five must
    /// stay the manager's rather than reverting to the source's.
    #[test]
    fn inheriting_reads_the_default_it_was_given_rather_than_the_source() {
        let (source, target) = (strict(), loose());
        let enclosing = Behavior::of(&target);

        let behavior = resolve(
            &BehaviorRequest::inherit().with(Axis::IntegerOverflow, "strict"),
            &pair(&source, &target),
            Some(&enclosing),
        )
        .expect("should resolve");

        assert_eq!(behavior.arithmetic(), Checked::Reported);
        for axis in Axis::ALL {
            if axis == Axis::IntegerOverflow {
                continue;
            }
            assert_eq!(
                behavior.stance(axis),
                loose().stance(axis),
                "{axis:?} should have kept the enclosing default, not reverted to the source"
            );
        }
    }

    /// Every axis is decided, whatever was asked for.
    #[test]
    fn a_resolved_behavior_is_total() {
        for request in [
            BehaviorRequest::inherit(),
            BehaviorRequest::language("loose"),
            BehaviorRequest::inherit().with(Axis::Remainder, "loose"),
        ] {
            let behavior = resolved(&request);
            for axis in Axis::ALL {
                assert_eq!(behavior.stance(axis).axis(), axis);
            }
        }
    }

    /// Mixing the two languages across axes is a resolvable request, not an error.
    #[test]
    fn axes_may_take_different_languages() {
        let behavior = resolved(
            &BehaviorRequest::inherit()
                .with(Axis::IntegerOverflow, "loose")
                .with(Axis::SequenceIndex, "strict"),
        );

        assert_eq!(behavior.arithmetic(), Checked::Unchecked);
        assert_eq!(behavior.index_origin(), IndexOrigin::FromEitherEnd);
        assert_eq!(behavior.text_units(), TextUnits::CodePoints);
    }

    fn rejected(request: &BehaviorRequest) -> BehaviorError {
        let (source, target) = (strict(), loose());
        resolve(request, &pair(&source, &target), None).expect_err("should be rejected")
    }

    #[test]
    fn a_language_compylr_does_not_know_is_rejected() {
        let error = rejected(&BehaviorRequest::language("haskell"));
        assert_eq!(error.code(), "unknown_language");
    }

    /// A language compylr knows that is not one of these two is a different mistake.
    ///
    /// It is also the likelier one: a user who has read that `go` is a reserved target may
    /// reasonably try to name it, and telling them no such language exists would be false.
    #[test]
    fn a_known_language_outside_the_pair_is_rejected_distinctly() {
        let error = rejected(&BehaviorRequest::language("elsewhere"));
        assert_eq!(error.code(), "language_not_in_pair");
        assert_ne!(
            error.code(),
            rejected(&BehaviorRequest::language("haskell")).code(),
            "the two rejections must be distinguishable without reading prose"
        );
    }

    #[test]
    fn an_unknown_axis_is_rejected_listing_the_axes_that_exist() {
        let error = BehaviorRequest::from_pairs([("floor_div", "loose")])
            .expect_err("a Python spelling is not an axis identifier");
        assert_eq!(error.code(), "unknown_axis");

        let message = error.to_string();
        for axis in Axis::ALL {
            assert!(
                message.contains(axis.code()),
                "the message must list '{}'; got: {message}",
                axis.code()
            );
        }
    }

    /// Every rejection names both languages that would have been accepted.
    ///
    /// Without them the message says what is wrong and not what to write instead, which for a
    /// setting with exactly two valid values is most of the answer.
    #[test]
    fn every_language_rejection_names_the_pair() {
        for name in ["haskell", "elsewhere"] {
            let message = rejected(&BehaviorRequest::language(name)).to_string();
            assert!(
                message.contains("strict"),
                "should name the source: {message}"
            );
            assert!(
                message.contains("loose"),
                "should name the target: {message}"
            );
            assert!(
                message.contains(name),
                "should name what was asked for: {message}"
            );
        }
    }

    /// An axis identifier is validated when the request is built, before any pair exists.
    #[test]
    fn an_axis_is_rejected_without_a_compilation_to_check_it_against() {
        assert!(BehaviorRequest::from_pairs([("integer_overflow", "anything")]).is_ok());
        assert!(BehaviorRequest::from_pairs([("", "anything")]).is_err());
        assert!(BehaviorRequest::from_pairs([("overflow", "anything")]).is_err());
    }
}
