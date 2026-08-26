//! Keeps the demo's coverage claim honest as the IR grows.
//!
//! `demo/demo-python-rust/src/algorithms/ir_coverage.py` walks the IR compylr writes and reports which statement
//! forms, expression forms, types, and operators the demo exercises, and the demo's own suite
//! fails when one stops being covered. That is one half of the guarantee, and on its own it is
//! the weaker half: those tables are written down in Python, so adding a form to the IR would
//! leave the demo reporting full coverage of a subset that had quietly grown underneath it.
//!
//! This is the other half. It reads the variants out of `compylr-ir` and fails when the tables do
//! not match, which is the moment to decide whether the demo should cover the new form or whether
//! the claim in its README has to narrow. Either is fine; not noticing is not.
//!
//! Read out of the source text rather than through the type system because Rust has no reflection
//! over enum variants, and generating the list some other way would be a second thing to keep in
//! sync. The parse is deliberately dumb and the assertions say what to do when it breaks.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate lives at <root>/crates/<name>")
        .to_path_buf()
}

/// The variant names of one `pub enum` in the IR's `ir.rs`.
///
/// A variant is a line at one indent level inside the enum body that starts with an uppercase
/// letter. Doc comments, attributes, and field lines are all indented further or start with
/// something else, which is enough to separate them without parsing Rust.
fn variants_of(enum_name: &str) -> BTreeSet<String> {
    let source = std::fs::read_to_string(repo_root().join("crates/compylr-ir/src/ir.rs"))
        .expect("the IR crate must have an ir.rs");
    let header = format!("pub enum {enum_name} {{");
    let start = source
        .find(&header)
        .unwrap_or_else(|| panic!("{enum_name} must be a `pub enum` in compylr-ir/src/ir.rs"))
        + header.len();

    let mut found = BTreeSet::new();
    let mut depth = 1usize;
    for line in source[start..].lines() {
        // Track braces so a struct variant's fields do not end the scan early.
        let opened = line.matches('{').count();
        let closed = line.matches('}').count();
        if depth == 1
            && let Some(name) = line
                .trim()
                .split(['(', '{', ',', ' '])
                .next()
                .filter(|word| word.starts_with(|c: char| c.is_ascii_uppercase()))
        {
            found.insert(name.to_string());
        }
        depth = depth + opened - closed.min(depth);
        if depth == 0 {
            break;
        }
    }
    assert!(
        !found.is_empty(),
        "no variants were found for {enum_name}; the scan in this test needs updating"
    );
    found
}

/// The entries of one tuple constant in the demo's coverage tables.
fn table(name: &str) -> BTreeSet<String> {
    let source = std::fs::read_to_string(repo_root().join("demo/demo-python-rust/src/algorithms/ir_coverage.py"))
        .expect("the demo must have an ir_coverage.py");
    let header = format!("{name} = (");
    let start = source
        .find(&header)
        .unwrap_or_else(|| panic!("{name} must be a tuple in the demo's ir_coverage.py"))
        + header.len();
    let end = start + source[start..].find(")").expect("the tuple must be closed");
    source[start..end]
        .split(',')
        .map(|entry| entry.trim().trim_matches('"').to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn assert_matches(enum_name: &str, table_name: &str, extra: &[&str]) {
    let mut expected = variants_of(enum_name);
    expected.extend(extra.iter().map(|s| s.to_string()));
    let listed = table(table_name);

    let unlisted: Vec<&String> = expected.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "{enum_name} has variants the demo's {table_name} table does not know about: {unlisted:?}.\n\
         Either add an algorithm to demo/demo-python-rust/src/algorithms/ that uses them and list them there, or \
         narrow the claim in demo/README.md -- but do not leave the demo reporting full coverage \
         of a subset that grew."
    );

    let unknown: Vec<&String> = listed.difference(&expected).collect();
    assert!(
        unknown.is_empty(),
        "the demo's {table_name} table lists forms {enum_name} no longer has: {unknown:?}.\n\
         Remove them from demo/demo-python-rust/src/algorithms/ir_coverage.py."
    );
}

#[test]
fn the_demo_knows_every_statement_form() {
    assert_matches("Stmt", "STATEMENTS", &[]);
}

#[test]
fn the_demo_knows_every_expression_form() {
    assert_matches("Expr", "EXPRESSIONS", &[]);
}

#[test]
fn the_demo_knows_every_operator() {
    assert_matches("BinOp", "OPERATORS", &[]);
}

#[test]
fn the_demo_knows_every_type() {
    // `Ty` is the one table that is not a bare variant list: `Instance` carries a class name and
    // appears in the artifact as `{"Instance": "PrimeCache"}`, which the walk finds by its tag
    // like any other.
    assert_matches("Ty", "TYPES", &[]);
}

#[test]
fn the_demo_knows_every_division_mode_a_python_program_can_produce() {
    // Not derived from `DivMode`, deliberately. `Integer` carries a `Rounding`, and Python only
    // ever means one of the two roundings -- so the demo covering "both modes" is a claim about
    // what a *Python* program can reach, not about what the IR can hold. The compiler's own
    // conformance corpus is what covers the rest, which is why it is authored as IR.
    let listed = table("MODES");
    assert_eq!(
        listed,
        BTreeSet::from(["Exact".to_string(), "Integer".to_string()]),
        "the demo should claim exactly the two division modes Python can produce"
    );
    let modes = variants_of("DivMode");
    assert!(
        modes.is_superset(&listed),
        "DivMode no longer has the modes the demo claims: {modes:?}"
    );
}
