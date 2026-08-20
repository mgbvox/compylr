//! The pair-directed pass table.
//!
//! Empty, and that is a claim rather than an omission: no transformation has yet been found that
//! is correct for one `(source, target)` pair and wrong for another. The selection rule is tested
//! in [`compylr_core::pass`] against a table with entries in it, because a rule tested only
//! against this table would pass for the wrong reason.

use compylr_core::pass::{DirectedPass, Pass, select_directed};

/// Every registered pair-directed pass.
const REGISTRY: &[DirectedPass] = &[];

/// The passes registered for one pair, in registration order.
pub fn for_pair(source: &str, target: &str) -> Vec<&'static dyn Pass> {
    select_directed(REGISTRY, source, target)
}

/// Every pair that has at least one directed pass.
pub fn pairs() -> Vec<(&'static str, &'static str)> {
    REGISTRY
        .iter()
        .map(|entry| (entry.source, entry.target))
        .collect()
}
