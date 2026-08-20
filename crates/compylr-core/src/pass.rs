//! Transformations over the IR, between verification and emission.
//!
//! Two kinds, running in this order:
//!
//! * **target-agnostic** passes, which hold for any source and any target;
//! * **pair-directed** passes, selected by `(source, target)` and still operating on the IR
//!   rather than on target source text.
//!
//! The second kind exists because some transformations are only correct, or only worth doing,
//! for a particular combination — and doing them on the IR keeps them reviewable as
//! transformations of a program rather than as string surgery on generated code.
//!
//! A pass must derive every decision from what the IR declares. Reading the unit's origin to
//! decide *what an operator means* would put the source language back into the middle of the
//! compiler, which is the thing this design removes. Selecting a directed pass by origin is
//! different and fine: that is choosing which transformation to apply, not what a node means.

use std::error::Error;
use std::fmt;

use compylr_ir::Unit;

/// One named transformation of a unit.
pub trait Pass: fmt::Debug + Send + Sync {
    /// The name this pass is selected and reported by.
    fn name(&self) -> &'static str;

    /// Transform the unit in place.
    ///
    /// A pass that cannot establish that a transformation is safe must leave the unit unchanged
    /// and return `Ok`. Returning an error means the *pass* is broken, not the program: a
    /// program that is wrong was already rejected by verification.
    fn run(&self, unit: &mut Unit) -> Result<(), PassError>;
}

/// A pass registered for one `(source, target)` pair.
#[derive(Debug, Clone, Copy)]
pub struct DirectedPass {
    /// Source language this pass applies to.
    pub source: &'static str,
    /// Target language this pass applies to.
    pub target: &'static str,
    /// The pass itself.
    pub pass: &'static dyn Pass,
}

/// A pass failed, which is a compiler bug rather than a rejected program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassError {
    pass: String,
    detail: String,
}

impl PassError {
    /// Build a failure attributed to `pass`.
    pub fn new(pass: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            pass: pass.into(),
            detail: detail.into(),
        }
    }

    /// Which pass failed.
    pub fn pass(&self) -> &str {
        &self.pass
    }
}

impl fmt::Display for PassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the '{}' pass failed: {}", self.pass, self.detail)
    }
}

impl Error for PassError {}

/// Which optimization passes to run.
///
/// `None` is a first-class choice rather than an absence: turning optimization off must produce a
/// program that behaves identically, and a configuration that cannot express "off" gives nothing
/// to compare an optimized build against when one of them is wrong.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Optimization {
    /// The documented default set.
    #[default]
    Default,
    /// No optimization passes. Verification still runs.
    None,
    /// Only the named passes, in the pipeline's own order.
    ///
    /// Names that match nothing are ignored rather than refused: the set of passes changes
    /// between releases, and a build that failed because a pass was renamed would be a worse
    /// outcome than one that ran fewer passes.
    Only(Vec<String>),
}

impl Optimization {
    /// A short stable key identifying this configuration.
    ///
    /// Recorded in build state beside the compiler version, because the same program built under
    /// two configurations is two different artifacts and reusing one for the other would hand
    /// back code nobody asked for.
    pub fn key(&self) -> String {
        match self {
            Self::Default => "default".to_string(),
            Self::None => "none".to_string(),
            Self::Only(names) => {
                let mut sorted = names.clone();
                sorted.sort();
                format!("only:{}", sorted.join(","))
            }
        }
    }
}

/// How the pipeline should run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PassConfig {
    /// Which optimization passes to run.
    pub optimization: Optimization,
}

/// What the pipeline actually did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PipelineReport {
    /// Names of the passes that ran, in order.
    ///
    /// Reported so a build can be explained. "Why is this generated code different?" is
    /// unanswerable without it, and answering it by reading the pass list in the source is
    /// answering a question about *this* build with a fact about the compiler.
    pub passes: Vec<&'static str>,
}

/// The target-agnostic passes, in the order they run.
///
/// Empty until there is something correct to put in it. An optimizer that exists because the
/// architecture has a slot for one is how a compiler acquires transformations nobody can justify.
const AGNOSTIC: &[&dyn Pass] = &[];

/// Run the pipeline over `unit`.
///
/// `directed` is supplied by the caller rather than looked up here, because selecting it needs
/// the registry and this crate must not depend on the crates it registers.
pub fn run(
    unit: &mut Unit,
    config: &PassConfig,
    directed: &[&'static dyn Pass],
) -> Result<PipelineReport, PassError> {
    let mut report = PipelineReport::default();
    if config.optimization == Optimization::None {
        return Ok(report);
    }

    for pass in AGNOSTIC.iter().chain(directed.iter()) {
        if !selected(&config.optimization, pass.name()) {
            continue;
        }
        pass.run(unit)?;
        report.passes.push(pass.name());
    }
    Ok(report)
}

fn selected(optimization: &Optimization, name: &str) -> bool {
    match optimization {
        Optimization::Default => true,
        Optimization::None => false,
        Optimization::Only(names) => names.iter().any(|wanted| wanted == name),
    }
}

/// The passes registered for one pair, in registration order.
///
/// Selection lives here rather than in the registry so that it can be tested against a table with
/// entries in it. A registry whose table is empty would make any test of the selection rule pass
/// for the wrong reason.
pub fn select_directed(
    entries: &[DirectedPass],
    source: &str,
    target: &str,
) -> Vec<&'static dyn Pass> {
    entries
        .iter()
        .filter(|entry| entry.source == source && entry.target == target)
        .map(|entry| entry.pass)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use compylr_diagnostics::span::Span;
    use compylr_ir::{Expr, Function, Literal, Stmt, Ty};

    /// A pass that records that it ran by appending a function.
    #[derive(Debug)]
    struct Marker(&'static str);

    impl Pass for Marker {
        fn name(&self) -> &'static str {
            self.0
        }

        fn run(&self, unit: &mut Unit) -> Result<(), PassError> {
            unit.add_function(Function {
                name: self.0.replace('-', "_"),
                params: vec![],
                ret: Ty::Int,
                body: vec![Stmt::Return(Expr::Literal(Literal::Int(1)))],
                doc: None,
                span: Span::default(),
            })
            .map_err(|error| PassError::new(self.0, error.message()))
        }
    }

    static FIRST: Marker = Marker("first");
    static SECOND: Marker = Marker("second");

    fn directed_table() -> Vec<DirectedPass> {
        vec![
            DirectedPass {
                source: "python",
                target: "rust",
                pass: &FIRST,
            },
            DirectedPass {
                source: "python",
                target: "go",
                pass: &SECOND,
            },
        ]
    }

    #[test]
    fn a_directed_pass_runs_only_for_its_own_pair() {
        let table = directed_table();
        let for_rust = select_directed(&table, "python", "rust");
        assert_eq!(for_rust.len(), 1);
        assert_eq!(for_rust[0].name(), "first");

        let for_go = select_directed(&table, "python", "go");
        assert_eq!(for_go.len(), 1);
        assert_eq!(for_go[0].name(), "second");
    }

    #[test]
    fn an_unregistered_pair_selects_nothing() {
        let table = directed_table();
        assert!(select_directed(&table, "typescript", "rust").is_empty());
        assert!(select_directed(&table, "python", "cpp").is_empty());
    }

    #[test]
    fn the_report_names_the_passes_that_ran_in_order() {
        let mut unit = Unit::new();
        let report = run(
            &mut unit,
            &PassConfig::default(),
            &[&FIRST as &dyn Pass, &SECOND],
        )
        .unwrap();
        assert_eq!(report.passes, ["first", "second"]);
        assert_eq!(unit.functions().count(), 2);
    }

    #[test]
    fn optimization_off_runs_nothing() {
        let mut unit = Unit::new();
        let report = run(
            &mut unit,
            &PassConfig {
                optimization: Optimization::None,
            },
            &[&FIRST as &dyn Pass],
        )
        .unwrap();
        assert!(report.passes.is_empty());
        assert_eq!(unit.functions().count(), 0);
    }

    #[test]
    fn only_runs_the_named_passes() {
        let mut unit = Unit::new();
        let report = run(
            &mut unit,
            &PassConfig {
                optimization: Optimization::Only(vec!["second".to_string()]),
            },
            &[&FIRST as &dyn Pass, &SECOND],
        )
        .unwrap();
        assert_eq!(report.passes, ["second"]);
    }

    /// A name that matches nothing must not fail the build.
    #[test]
    fn an_unknown_pass_name_is_ignored() {
        let mut unit = Unit::new();
        let report = run(
            &mut unit,
            &PassConfig {
                optimization: Optimization::Only(vec!["retired-in-a-later-release".to_string()]),
            },
            &[&FIRST as &dyn Pass],
        )
        .unwrap();
        assert!(report.passes.is_empty());
    }

    #[test]
    fn a_configuration_key_distinguishes_the_choices() {
        assert_eq!(Optimization::Default.key(), "default");
        assert_eq!(Optimization::None.key(), "none");
        assert_ne!(Optimization::Default.key(), Optimization::None.key());
        // Order of the named set must not change the key, or the same build would look different.
        assert_eq!(
            Optimization::Only(vec!["b".into(), "a".into()]).key(),
            Optimization::Only(vec!["a".into(), "b".into()]).key()
        );
    }
}
