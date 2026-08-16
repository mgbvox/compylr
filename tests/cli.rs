//! The command line, exercised as a command.
//!
//! Argument parsing is unit-tested inside `src/main.rs`; this file runs the built binary, because
//! the things worth checking here — which stream output goes to, what the exit status is — are
//! properties of the process rather than of a function.

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run the CLI with `args`, returning the completed output.
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_compylr"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("the binary must be built")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const ACCEPTED: &str = "python/fixtures/accepted/inference.py";
const REJECTED: &str = "python/fixtures/rejected/boolean_arithmetic.py";

#[test]
fn a_supported_file_is_reported_and_exits_successfully() {
    let output = cli(&[ACCEPTED]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("unit fingerprint:"), "{out}");
    assert!(out.contains("-> bool"), "{out}");
}

#[test]
fn no_arguments_prints_usage_and_fails() {
    let output = cli(&[]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("usage: compylr"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_missing_file_reports_the_path() {
    let output = cli(&["python/fixtures/accepted/nonesuch.py"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("nonesuch.py"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_rejected_program_reports_its_location_and_fails() {
    let output = cli(&[REJECTED]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("2:12"), "expected a line:column, got: {err}");
    assert!(err.contains("bool"), "{err}");
}

#[test]
fn diagnostics_go_to_stderr_leaving_stdout_clean() {
    // So that redirecting stdout to a file produces a file, not a file with an error in it.
    let output = cli(&["--emit", "rust", REJECTED]);
    assert!(!output.status.success());
    assert!(stdout(&output).is_empty(), "stdout: {}", stdout(&output));
    assert!(!stderr(&output).is_empty());
}

#[test]
fn a_syntax_error_is_reported() {
    let output = cli(&["python/entrypoint.py"]);
    assert!(!output.status.success());
    assert!(!stderr(&output).is_empty());
}

mod emit {
    use super::*;

    #[test]
    fn the_ir_is_emitted_as_json() {
        let output = cli(&["--emit", "ir", ACCEPTED]);
        assert!(output.status.success(), "{}", stderr(&output));

        let artifact: serde_json::Value =
            serde_json::from_str(&stdout(&output)).expect("stdout must be valid JSON");
        assert_eq!(artifact["version"], 1);
        assert!(artifact["functions"].as_array().unwrap().len() >= 3);
    }

    #[test]
    fn only_the_translated_code_is_printed() {
        // The part a reader is after, and the part that stays useful piped into a pager. Printing
        // every file as one stream would produce something that no longer compiles when
        // redirected to a single `.rs`.
        let output = cli(&["--emit", "rust", ACCEPTED]);
        assert!(output.status.success(), "{}", stderr(&output));

        let out = stdout(&output);
        assert!(out.contains("pub fn comparisons"), "{out}");
        assert!(!out.contains("#[pymodule]"), "no boundary code:\n{out}");
        assert!(!out.contains("py_floordiv(lhs"), "no helpers:\n{out}");
        assert!(!out.contains("pub mod compat;"), "no crate root:\n{out}");
    }

    #[test]
    fn summary_is_the_default() {
        assert_eq!(
            stdout(&cli(&[ACCEPTED])),
            stdout(&cli(&["--emit", "summary", ACCEPTED]))
        );
    }

    #[test]
    fn an_unknown_form_lists_the_accepted_ones() {
        let output = cli(&["--emit", "yaml", ACCEPTED]);
        assert!(!output.status.success());
        let err = stderr(&output);
        assert!(err.contains("summary") && err.contains("rust"), "{err}");
    }
}

mod backends {
    use super::*;

    #[test]
    fn a_reserved_backend_reads_as_planned() {
        let output = cli(&["--backend", "typescript", "--emit", "rust", ACCEPTED]);
        assert!(!output.status.success());
        assert!(
            stderr(&output).contains("not implemented yet"),
            "{}",
            stderr(&output)
        );
    }

    #[test]
    fn an_unknown_backend_lists_what_is_available() {
        let output = cli(&["--backend", "nonesuch", ACCEPTED]);
        assert!(!output.status.success());
        let err = stderr(&output);
        assert!(err.contains("rust"), "{err}");
        assert!(
            !err.contains("not implemented yet"),
            "a typo must not read as a planned target: {err}"
        );
    }

    #[test]
    fn the_backend_is_checked_before_the_file_is_read() {
        // The file does not exist; the backend is still the thing to report.
        let output = cli(&["--backend", "nonesuch", "does/not/exist.py"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("nonesuch"), "{}", stderr(&output));
    }
}

mod crate_form {
    use super::*;

    fn out_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("cli-crate")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn every_file_is_written() {
        let dir = out_dir("every-file");
        let output = cli(&["--emit", "crate", "--out", dir.to_str().unwrap(), ACCEPTED]);
        assert!(output.status.success(), "{}", stderr(&output));

        for expected in [
            "src/lib.rs",
            "src/generated.rs",
            "src/bindings.rs",
            "src/compat.rs",
            "Cargo.toml",
        ] {
            assert!(dir.join(expected).exists(), "missing {expected}");
        }
    }

    #[test]
    fn a_missing_directory_is_created() {
        let dir = out_dir("nested").join("deeper").join("still");
        let output = cli(&["--emit", "crate", "--out", dir.to_str().unwrap(), ACCEPTED]);
        assert!(output.status.success(), "{}", stderr(&output));
        assert!(dir.join("src/lib.rs").exists());
    }

    #[test]
    fn no_source_goes_to_the_output_stream() {
        let dir = out_dir("quiet");
        let output = cli(&["--emit", "crate", "--out", dir.to_str().unwrap(), ACCEPTED]);
        let out = stdout(&output);
        assert!(out.contains("wrote"), "a report is fine: {out}");
        assert!(!out.contains("pub fn"), "but not source:\n{out}");
    }

    #[test]
    fn the_destination_is_required() {
        let output = cli(&["--emit", "crate", ACCEPTED]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("--out"), "{}", stderr(&output));
    }

    #[test]
    fn writing_a_crate_invokes_no_toolchain() {
        // No `target/` appears, because nothing was compiled to produce the files.
        let dir = out_dir("no-build");
        let output = cli(&["--emit", "crate", "--out", dir.to_str().unwrap(), ACCEPTED]);
        assert!(output.status.success());
        assert!(
            !dir.join("target").exists(),
            "nothing should have been built"
        );
    }
}
