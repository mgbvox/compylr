//! Docstrings: accepted, inert, and not part of a function's structure.
//!
//! The exception this file exercises is deliberately narrow — first position, string literal — so
//! most of these tests are about what is *still* rejected. A rule that accepted any discarded
//! expression statement would let dead code and inexpressible side effects through silently, which
//! is the failure this narrowness exists to prevent.

use compylr_diagnostics::error::LowerErrorKind;
use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source;
use compylr_ir::{Function, Stmt, Unit};
use compylr_registry::backends::lookup;

fn lower(source: &str) -> Vec<Function> {
    let parsed = parse_source(source).expect("fixture must parse");
    lower_source(&parsed, python_stance())
        .unwrap_or_else(|e| panic!("should lower: {}", e.render(source)))
}

fn reject(source: &str) -> LowerErrorKind {
    let parsed = parse_source(source).expect("fixture must parse");
    match lower_source(&parsed, python_stance()) {
        Ok(_) => panic!("should have been rejected but lowered:\n{source}"),
        Err(error) => error.kind(),
    }
}

fn unit_from(source: &str) -> Unit {
    let mut unit = Unit::new();
    for function in lower(source) {
        unit.add_function(function).unwrap();
    }
    unit
}

const DOCUMENTED: &str = concat!(
    "def add(a: int, b: int) -> int:\n",
    "    \"\"\"Return the sum.\"\"\"\n",
    "    return a + b\n",
);

#[test]
fn a_documented_function_lowers() {
    let functions = lower(DOCUMENTED);
    assert_eq!(functions.len(), 1);
}

#[test]
fn the_docstring_does_not_become_a_statement() {
    let functions = lower(DOCUMENTED);
    assert_eq!(
        functions[0].body.len(),
        1,
        "the body should hold only the return; got {:?}",
        functions[0].body
    );
    assert!(matches!(functions[0].body[0], Stmt::Return(_)));
}

#[test]
fn the_docstring_is_retained_on_the_function() {
    let functions = lower(DOCUMENTED);
    assert_eq!(functions[0].doc.as_deref(), Some("Return the sum."));
}

#[test]
fn an_undocumented_function_carries_no_docstring() {
    let functions = lower("def add(a: int, b: int) -> int:\n    return a + b\n");
    assert_eq!(functions[0].doc, None);
}

#[test]
fn a_function_of_only_a_docstring_lowers() {
    // The docstring is removed, leaving an empty body — valid only for a unit return, which is
    // exactly what Python's own `def f(): "doc"` amounts to.
    let functions = lower("def noop() -> None:\n    \"\"\"Does nothing.\"\"\"\n");
    assert_eq!(functions[0].doc.as_deref(), Some("Does nothing."));
    assert!(functions[0].body.is_empty());
}

#[test]
fn a_multi_line_docstring_is_retained_whole() {
    let functions = lower(concat!(
        "def add(a: int, b: int) -> int:\n",
        "    \"\"\"Return the sum.\n",
        "\n",
        "    A longer explanation.\n",
        "    \"\"\"\n",
        "    return a + b\n",
    ));
    let doc = functions[0].doc.as_deref().expect("docstring");
    assert!(doc.contains("Return the sum."));
    assert!(doc.contains("A longer explanation."));
}

#[test]
fn adjacent_literals_are_one_docstring() {
    // The parser concatenates them into a single node, so this needs no special handling — but it
    // would silently break if the check ever moved to matching source text.
    let functions = lower(concat!(
        "def add(a: int, b: int) -> int:\n",
        "    \"first \" \"second\"\n",
        "    return a + b\n",
    ));
    assert_eq!(functions[0].doc.as_deref(), Some("first second"));
}

mod fingerprints {
    use super::*;

    fn fingerprint(source: &str) -> u64 {
        lower(source)[0].fingerprint()
    }

    #[test]
    fn adding_a_docstring_does_not_change_the_fingerprint() {
        // The guarantee at stake: documenting a function must not trigger a crate rebuild.
        let bare = fingerprint("def add(a: int, b: int) -> int:\n    return a + b\n");
        assert_eq!(bare, fingerprint(DOCUMENTED));
    }

    #[test]
    fn editing_a_docstring_does_not_change_the_fingerprint() {
        let other = concat!(
            "def add(a: int, b: int) -> int:\n",
            "    \"\"\"Completely different prose.\"\"\"\n",
            "    return a + b\n",
        );
        assert_eq!(fingerprint(DOCUMENTED), fingerprint(other));
    }

    #[test]
    fn the_unit_fingerprint_is_also_unaffected() {
        let bare = unit_from("def add(a: int, b: int) -> int:\n    return a + b\n");
        assert_eq!(bare.fingerprint(), unit_from(DOCUMENTED).fingerprint());
    }

    #[test]
    fn a_real_change_still_moves_the_fingerprint() {
        // The mirror: excluding the docstring must not have made the fingerprint insensitive.
        let changed = concat!(
            "def add(a: int, b: int) -> int:\n",
            "    \"\"\"Return the sum.\"\"\"\n",
            "    return a - b\n",
        );
        assert_ne!(fingerprint(DOCUMENTED), fingerprint(changed));
    }
}

mod artifact {
    use super::*;

    #[test]
    fn the_docstring_survives_a_round_trip() {
        let unit = unit_from(DOCUMENTED);
        let restored = Unit::from_json(&unit.to_json().unwrap()).unwrap();
        assert_eq!(
            restored.get("add").unwrap().doc.as_deref(),
            Some("Return the sum."),
            "the artifact is for reading, and a function stripped of its documentation is harder \
             to check against the original"
        );
    }

    #[test]
    fn an_absent_docstring_round_trips_as_absent() {
        let unit = unit_from("def add(a: int, b: int) -> int:\n    return a + b\n");
        let restored = Unit::from_json(&unit.to_json().unwrap()).unwrap();
        assert_eq!(restored.get("add").unwrap().doc, None);
    }

    #[test]
    fn the_recorded_fingerprint_still_verifies() {
        // `from_json` recomputes the fingerprint and rejects a mismatch. A field that is
        // serialized but not hashed must not break that check.
        let unit = unit_from(DOCUMENTED);
        assert!(Unit::from_json(&unit.to_json().unwrap()).is_ok());
    }
}

mod still_rejected {
    use super::*;

    #[test]
    fn a_string_statement_after_the_first_is_rejected() {
        assert_eq!(
            reject(concat!(
                "def add(a: int, b: int) -> int:\n",
                "    c = a + b\n",
                "    \"not a docstring\"\n",
                "    return c\n",
            )),
            LowerErrorKind::UnsupportedConstruct
        );
    }

    #[test]
    fn a_second_string_in_a_documented_function_is_rejected() {
        assert_eq!(
            reject(concat!(
                "def add(a: int, b: int) -> int:\n",
                "    \"\"\"Real docstring.\"\"\"\n",
                "    \"stray string\"\n",
                "    return a + b\n",
            )),
            LowerErrorKind::UnsupportedConstruct
        );
    }

    #[test]
    fn a_non_string_expression_statement_is_rejected() {
        assert_eq!(
            reject("def add(a: int, b: int) -> int:\n    a + b\n    return a\n"),
            LowerErrorKind::UnsupportedConstruct
        );
    }

    #[test]
    fn a_bare_call_statement_is_rejected() {
        // The subset cannot express a call made for a side effect, so accepting it would compile
        // something whose whole purpose is invisible to the compiler.
        assert_eq!(
            reject(concat!(
                "def helper(a: int) -> int:\n    return a\n\n",
                "def add(a: int, b: int) -> int:\n",
                "    helper(a)\n",
                "    return a + b\n",
            )),
            LowerErrorKind::UnsupportedConstruct
        );
    }

    #[test]
    fn a_module_level_docstring_is_rejected() {
        // The exception is body-only. A module docstring is a top-level statement that is not a
        // function definition, and that rule is unchanged.
        assert_eq!(
            reject("\"\"\"Module docs.\"\"\"\ndef add(a: int) -> int:\n    return a\n"),
            LowerErrorKind::UnsupportedConstruct
        );
    }

    #[test]
    fn an_f_string_in_first_position_is_rejected() {
        // Python does not treat an f-string as a docstring either: it is a runtime expression,
        // and `__doc__` is None for such a function.
        assert_eq!(
            reject(concat!(
                "def add(a: int, b: int) -> int:\n",
                "    f\"not a docstring {a}\"\n",
                "    return a + b\n",
            )),
            LowerErrorKind::UnsupportedConstruct
        );
    }

    #[test]
    fn a_non_returning_documented_function_is_rejected() {
        // Stripping the docstring must not make `-> int` with no return look acceptable. This was
        // caught by the backend until lowering gained the check; lowering is the better place,
        // since it reports the function and its location rather than an internal codegen error.
        assert_eq!(
            reject("def f() -> int:\n    \"\"\"Docs.\"\"\"\n"),
            LowerErrorKind::MissingReturn
        );
    }
}

mod emission {
    use super::*;

    /// The translated functions.
    ///
    /// The helpers are themselves documented, so asserting against the whole crate would find
    /// `///` whether or not the function under test carried a docstring. Now they are a
    /// different file, so this is a lookup rather than string surgery.
    fn emit(source: &str) -> String {
        lookup("rust")
            .unwrap()
            .emit(&unit_from(source))
            .expect("must emit")
            .remove("src/generated.rs")
            .expect("a translated-code file must be emitted")
    }

    /// Every emitted file, as the crate it describes.
    fn whole_crate(source: &str) -> std::collections::BTreeMap<String, String> {
        lookup("rust")
            .unwrap()
            .emit(&unit_from(source))
            .expect("must emit")
    }

    #[test]
    fn a_docstring_reaches_the_generated_source() {
        let emitted = emit(DOCUMENTED);
        assert!(
            emitted.contains("/// Return the sum."),
            "expected a doc comment in:\n{emitted}"
        );
    }

    #[test]
    fn a_function_without_a_docstring_emits_none() {
        let emitted = emit("def add(a: int, b: int) -> int:\n    return a + b\n");
        assert!(
            !emitted.contains("///"),
            "no doc comment should be emitted:\n{emitted}"
        );
    }

    #[test]
    fn a_multi_line_docstring_emits_one_line_each() {
        let emitted = emit(concat!(
            "def add(a: int, b: int) -> int:\n",
            "    \"\"\"First line.\n",
            "\n",
            "    Second line.\n",
            "    \"\"\"\n",
            "    return a + b\n",
        ));
        assert!(emitted.contains("/// First line."), "{emitted}");
        assert!(emitted.contains("Second line."), "{emitted}");
    }

    #[test]
    fn emission_stays_deterministic() {
        assert_eq!(emit(DOCUMENTED), emit(DOCUMENTED));
    }

    /// Compile emitted source as a library, returning rustc's complaint on failure.
    ///
    /// `label` must be unique per test. `cargo test` runs these in parallel, and sharing one
    /// scratch path makes them race — which surfaces as a suite that passes on one run and fails
    /// on the next, the worst kind of failure to chase.
    fn compiles(
        label: &str,
        files: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), String> {
        use std::path::PathBuf;
        use std::process::Command;

        // Written out and compiled from the crate root, which is what the build pipeline does.
        // Concatenating the files would not work: `lib.rs` opens with inner attributes, which are
        // only valid at the top of a file.
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("docstrings")
            .join(label);
        let _ = std::fs::remove_dir_all(&dir);
        for (relative, contents) in files {
            let path = dir.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, contents).unwrap();
        }
        let path = dir.join("src/lib.rs");

        let output = Command::new("rustc")
            .args([
                "--edition",
                "2024",
                "--crate-type",
                "lib",
                "--emit",
                "metadata",
                "-o",
            ])
            .arg(dir.join("lib.rmeta"))
            .arg(&path)
            .output()
            .expect("rustc must be available");
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    #[test]
    fn a_docstring_cannot_break_out_of_its_comment() {
        // Arbitrary user text reaches the generated source. If it could terminate the comment,
        // whatever followed would be read as code -- so the interesting assertion is that the
        // result still compiles, not merely that the characters survive.
        let source = concat!(
            "def risky(a: int) -> int:\n",
            "    \"\"\"closes */ a block, has a \\\\ backslash\n",
            "    and spans lines\"\"\"\n",
            "    return a\n",
        );
        let whole = whole_crate(source);

        assert!(
            whole["src/generated.rs"].contains("/// closes */ a block"),
            "{}",
            emit(source)
        );
        if let Err(stderr) = compiles("risky", &whole) {
            panic!("emitted source did not compile:\n{stderr}");
        }
    }

    #[test]
    fn a_documented_unit_still_compiles() {
        if let Err(stderr) = compiles("documented", &whole_crate(DOCUMENTED)) {
            panic!("emitted source did not compile:\n{stderr}");
        }
    }
}

/// Python's own stance, which is what an unconfigured compilation resolves to.
///
/// Read from the frontend's declaration rather than rebuilt here, so these tests lower under the
/// same bundle the pipeline uses.
fn python_stance() -> compylr_ir::Behavior {
    compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR)
}
