//! Turning Python source into a parse tree.
//!
//! Source *text* is the primary input, not a path. In the target design the compiler is handed
//! the result of `inspect.getsource(fn)` for a decorated function, so a path-only API would
//! force callers to write a temporary file just to satisfy the signature. Reading from disk is
//! a thin convenience on top, used by fixtures and tests.

use std::fs;
use std::path::Path;

use ruff_python_ast::ModModule;
use ruff_python_parser::{Parsed, parse_module};

use crate::error::FrontendError;

/// Parse Python source text into a module parse tree.
pub fn parse_source(source: &str) -> Result<Parsed<ModModule>, FrontendError> {
    // `?` converts ParseError into FrontendError via the From impl in `error`.
    Ok(parse_module(source)?)
}

/// Read a file and parse its contents.
///
/// I/O and syntax failures stay distinguishable, so a caller can tell "I could not find your
/// file" from "your file is not valid Python".
pub fn parse_file(path: &Path) -> Result<Parsed<ModModule>, FrontendError> {
    let source = fs::read_to_string(path).map_err(|error| FrontendError::io(path, error))?;
    parse_source(&source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn parses_valid_source() {
        let parsed = parse_source("def f(a: int) -> int:\n    return a\n").unwrap();
        assert_eq!(parsed.syntax().body.len(), 1);
    }

    #[test]
    fn parses_empty_source_into_empty_body() {
        let parsed = parse_source("").unwrap();
        assert!(parsed.syntax().body.is_empty());
    }

    #[test]
    fn malformed_source_is_a_syntax_failure_with_a_span() {
        let error = parse_source("def (:\n").unwrap_err();
        assert!(error.is_syntax(), "expected syntax failure, got {error}");
        assert!(error.span().is_some());
        assert!(!error.is_io());
    }

    #[test]
    fn caller_can_branch_on_kind_without_reading_messages() {
        let syntax = parse_source("def ??").unwrap_err();
        let io = parse_file(&manifest_dir().join("does-not-exist.py")).unwrap_err();

        // Branching uses the variant, never the rendered text.
        assert!(syntax.is_syntax() && !syntax.is_io());
        assert!(io.is_io() && !io.is_syntax());
    }

    #[test]
    fn missing_file_reports_io_failure_naming_the_path() {
        let path = manifest_dir().join("no-such-file.py");
        let error = parse_file(&path).unwrap_err();
        assert!(error.is_io());
        assert_eq!(error.path(), Some(path.as_path()));
        assert!(error.to_string().contains("no-such-file.py"));
    }

    #[test]
    fn directory_path_reports_io_failure_rather_than_panicking() {
        let path = manifest_dir().join("python");
        let error = parse_file(&path).unwrap_err();
        assert!(error.is_io(), "expected io failure, got {error}");
    }

    #[test]
    fn reads_a_real_fixture_from_disk() {
        let path = manifest_dir().join("python/entrypoint.py");
        let parsed = parse_file(&path).unwrap();
        assert!(!parsed.syntax().body.is_empty());
    }
}
