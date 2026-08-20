//! What a frontend requires, what a backend preserves, and what happens when they disagree.
//!
//! "Compiled" is not the same as "means the same thing". A Python function that reports an
//! overflow and a Rust one that wraps are both valid programs; only one of them is a translation
//! of the other. The check here is what turns that from something a user discovers as a wrong
//! answer into something compylr refuses by name, before any target source exists.
//!
//! It also gates the other direction. A backend may offer transformations that trade a guarantee
//! for speed; each declares which guarantees it breaks, and one that would break a guarantee the
//! frontend requires is withheld — with a reason, so that "why is this not faster?" has an
//! answer.

use std::error::Error;
use std::fmt;

use compylr_ir::{Guarantee, Unit};

use crate::backend::Backend;

/// A frontend requirement the chosen backend does not meet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmetGuarantee {
    /// The guarantee that is not covered.
    pub guarantee: Guarantee,
    /// The source language that requires it.
    pub frontend: String,
    /// The target language that does not preserve it.
    pub backend: String,
}

impl fmt::Display for UnmetGuarantee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the '{}' backend does not preserve a guarantee '{}' requires: {}",
            self.backend, self.frontend, self.guarantee
        )
    }
}

impl Error for UnmetGuarantee {}

/// Check a unit's requirements against a backend's declaration.
///
/// A unit with no origin requires nothing and passes. That is deliberate rather than lax: a
/// hand-built unit — a test fixture, a conformance corpus entry — has no source language, and
/// inventing requirements for it would make the corpus unrunnable to satisfy a check that has
/// nothing to check.
pub fn negotiate(unit: &Unit, backend: &dyn Backend) -> Result<(), UnmetGuarantee> {
    let preserved = backend.preserves();
    for required in unit.requires() {
        if !preserved.contains(required) {
            return Err(UnmetGuarantee {
                guarantee: *required,
                frontend: unit
                    .origin()
                    .map_or_else(|| "unknown".to_string(), |o| o.frontend.clone()),
                backend: backend.name().to_string(),
            });
        }
    }
    Ok(())
}

/// A transformation a target offers that costs a guarantee.
///
/// Declared rather than applied. A target's fast path is a real thing someone may want, and the
/// design's answer is not to forbid it but to make the trade explicit and refusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetOption {
    /// The name this option is permitted by.
    pub name: &'static str,
    /// The guarantees applying it would break.
    pub breaks: &'static [Guarantee],
    /// Whether the backend can actually apply it.
    ///
    /// The same three-way honesty the registries use: a name a backend has reserved for a
    /// transformation it intends to offer is not the same as a name that means nothing, and
    /// permitting one should say so rather than silently do nothing.
    pub implemented: bool,
}

/// Why an option was not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithheldOption {
    /// The option that was not applied.
    pub option: &'static str,
    /// The guarantee that stopped it.
    pub guarantee: Guarantee,
}

impl fmt::Display for WithheldOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "'{}' was not applied because the source language requires that {}",
            self.option, self.guarantee
        )
    }
}

/// Asking for an option a backend does not implement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableOption {
    /// The name that was requested.
    pub option: String,
    /// Whether the backend has reserved that name for a planned transformation.
    pub reserved: bool,
}

impl fmt::Display for UnavailableOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.reserved {
            write!(
                f,
                "'{}' is a transformation this backend has reserved but does not implement yet",
                self.option
            )
        } else {
            write!(
                f,
                "'{}' is not a transformation this backend offers",
                self.option
            )
        }
    }
}

impl Error for UnavailableOption {}

/// Decide which of a backend's options may run.
///
/// Returns the options to apply and the ones withheld with their reasons. An option the caller
/// did not ask for is simply absent from both: silence about a transformation nobody wanted is
/// not a report, it is noise.
pub fn resolve_options(
    unit: &Unit,
    backend: &dyn Backend,
    requested: &[String],
) -> Result<(Vec<&'static str>, Vec<WithheldOption>), UnavailableOption> {
    let mut applied = Vec::new();
    let mut withheld = Vec::new();

    for name in requested {
        let Some(option) = backend.options().iter().find(|o| o.name == *name) else {
            return Err(UnavailableOption {
                option: name.clone(),
                reserved: false,
            });
        };
        if let Some(guarantee) = option
            .breaks
            .iter()
            .find(|breaks| unit.requires().contains(breaks))
        {
            withheld.push(WithheldOption {
                option: option.name,
                guarantee: *guarantee,
            });
            continue;
        }
        if !option.implemented {
            return Err(UnavailableOption {
                option: name.clone(),
                reserved: true,
            });
        }
        applied.push(option.name);
    }
    Ok((applied, withheld))
}

/// Every option a backend would withhold for this unit, whether or not it was asked for.
///
/// The reportable half of "why is this not faster?". Answering that requires knowing what was
/// available and refused, which is not visible from what ran.
pub fn withheld_by_default(unit: &Unit, backend: &dyn Backend) -> Vec<WithheldOption> {
    backend
        .options()
        .iter()
        .filter_map(|option| {
            option
                .breaks
                .iter()
                .find(|breaks| unit.requires().contains(breaks))
                .map(|guarantee| WithheldOption {
                    option: option.name,
                    guarantee: *guarantee,
                })
        })
        .collect()
}
