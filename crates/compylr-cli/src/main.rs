//! The `compylr` command line.
//!
//! Answers "what does this file actually become?" without a build, an interpreter, or a decorator.
//! It is the fastest way to see generated source — the alternative is running a full toolchain
//! build and finding the file it wrote.
//!
//! Deliberately a thin wrapper over the library. Somebody diagnosing a rejection must get the same
//! diagnostic here as from the decorator; a CLI with its own logic would become a second source of
//! answers and therefore a source of confusion.
//!
//! Emitted output goes to stdout and diagnostics to stderr, so `compylr --emit rust f.py > out.rs`
//! produces a file rather than a file with an error message in it.

use std::path::PathBuf;
use std::process::ExitCode;

use compylr_core::backend::BackendError;
// The summary quotes types back in the language of the file being inspected, so it uses the
// frontend's spelling rather than the IR's neutral one.
use compylr_frontend_python::PythonTypeName;
use compylr_registry::{backends, bridges, frontends};

/// The source language, until there is a second one to choose between.
///
/// A constant rather than a hardcoded call: the name goes through the registry like any other,
/// so adding a `--frontend` flag later is a change to argument parsing rather than to how a
/// frontend is reached.
const DEFAULT_FRONTEND: &str = "python";

const USAGE: &str = "\
usage: compylr [--emit summary|ir|rust|crate] [--out DIR] [--backend NAME] <file.py>

  --emit summary   unit fingerprint and each function's signature (default)
  --emit ir        the IR artifact, as JSON
  --emit rust      the translated functions, without performing a build
  --emit crate     every generated file; requires --out
  --out DIR        destination for --emit crate
  --backend NAME   target backend (default: rust)
";

/// What the CLI should print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Emit {
    Summary,
    Ir,
    /// The translated functions alone — what a reader is usually after, and pipeable.
    Target,
    /// Every generated file, written to a directory.
    Crate,
}

impl Emit {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "summary" => Ok(Self::Summary),
            "ir" => Ok(Self::Ir),
            "rust" | "target" => Ok(Self::Target),
            "crate" => Ok(Self::Crate),
            other => Err(format!(
                "unknown --emit value '{other}'; expected one of: summary, ir, rust, crate"
            )),
        }
    }
}

/// Everything the command line asked for.
#[derive(Debug)]
struct Options {
    path: PathBuf,
    emit: Emit,
    backend: String,
    out: Option<PathBuf>,
}

/// Parse arguments by hand.
///
/// Four flags do not justify an argument-parsing dependency, and the crate's dependency surface is
/// currently the vendored ruff tree plus PyO3 and serde.
fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut path: Option<PathBuf> = None;
    let mut emit = Emit::Summary;
    let mut backend = "rust".to_string();
    let mut out: Option<PathBuf> = None;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--emit" => {
                let value = args.next().ok_or("--emit needs a value")?;
                emit = Emit::parse(&value)?;
            }
            "--backend" => {
                backend = args.next().ok_or("--backend needs a value")?;
            }
            "--out" => {
                out = Some(PathBuf::from(args.next().ok_or("--out needs a value")?));
            }
            "-h" | "--help" => return Err(String::new()),
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'"));
            }
            other => {
                if path.is_some() {
                    return Err("only one file may be given".to_string());
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    let options = Options {
        path: path.ok_or("no input file given")?,
        emit,
        backend,
        out,
    };
    // Required rather than defaulted: writing several files somewhere the user did not name is a
    // side effect a command should not have.
    if options.emit == Emit::Crate && options.out.is_none() {
        return Err("--emit crate needs --out DIR to write to".to_string());
    }
    Ok(options)
}

fn main() -> ExitCode {
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("error: {message}");
            }
            eprint!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    match run(&options) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Compile the file and render the requested form.
fn run(options: &Options) -> Result<String, String> {
    // Resolved first, so asking for an unusable backend reports the backend rather than whichever
    // part of the file happened to be wrong.
    let backend = backends::lookup(&options.backend).map_err(|error: BackendError| {
        // A reserved target reads as planned; an unrecognized one as a typo. Collapsing them would
        // tell someone asking for TypeScript that no such target exists.
        error.to_string()
    })?;
    let frontend = frontends::lookup(DEFAULT_FRONTEND).map_err(|error| error.to_string())?;

    let source = std::fs::read_to_string(&options.path)
        .map_err(|error| format!("could not read {}: {error}", options.path.display()))?;

    // The frontend does the parsing and the assembly. Reading the file is this crate's business
    // because the trait takes text: the decorator's sources come from a live function object and
    // may correspond to no file at all.
    let unit = frontend
        .lower(std::slice::from_ref(&source))
        .map_err(|error| error.to_string())?;
    unit.validate().map_err(|error| error.render(&source))?;

    match options.emit {
        Emit::Summary => {
            let mut out = format!("unit fingerprint: {:016x}\n", unit.fingerprint());
            for function in unit.functions() {
                out.push_str(&format!(
                    "  {} ({} params) -> {}\n",
                    function.name,
                    function.params.len(),
                    function.ret.python_name()
                ));
            }
            Ok(out)
        }
        Emit::Ir => unit
            .to_json()
            .map(|json| format!("{json}\n"))
            .map_err(|error| error.to_string()),
        Emit::Target => {
            // Only the translated functions. Printing every file as one stream would produce
            // something that no longer compiles when redirected to a single `.rs`, quietly
            // breaking the obvious use of the flag.
            //
            // The backend alone, with no bridge: seeing what your Python became is a question
            // about the target, and it stays answerable for a target no host can call yet.
            let files = backend.emit(&unit).map_err(|error| error.to_string())?;
            files
                .get(compylr_backend_rust::rust::GENERATED_PATH)
                .cloned()
                .ok_or_else(|| "this backend emits no translated-code file".to_string())
        }
        Emit::Crate => {
            // A buildable crate, which means the host boundary as well as the translation — so
            // this form, unlike `--emit rust`, needs the pair to be bridged.
            let host = bridges::lookup(DEFAULT_FRONTEND, &options.backend)
                .map_err(|error| error.to_string())?;
            let artifact = host.emit(&unit).map_err(|error| error.to_string())?;
            let files = artifact.files;
            let root = options
                .out
                .as_ref()
                .expect("checked while parsing arguments");
            let mut written = Vec::new();
            for (relative, contents) in &files {
                let path = root.join(relative);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
                }
                std::fs::write(&path, contents)
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                written.push(relative.clone());
            }
            let manifest = artifact.manifest;
            std::fs::write(root.join("Cargo.toml"), manifest)
                .map_err(|e| format!("could not write the manifest: {e}"))?;
            written.push("Cargo.toml".to_string());
            // A report of what was written, never source: the source went to files.
            Ok(format!(
                "wrote {} to {}\n",
                written.join(", "),
                root.display()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn a_bare_path_defaults_to_the_summary() {
        let options = parse(&["f.py"]).unwrap();
        assert_eq!(options.emit, Emit::Summary);
        assert_eq!(options.backend, "rust");
        assert_eq!(options.path, PathBuf::from("f.py"));
    }

    #[test]
    fn emit_values_are_recognised() {
        assert_eq!(parse(&["--emit", "ir", "f.py"]).unwrap().emit, Emit::Ir);
        assert_eq!(
            parse(&["--emit", "rust", "f.py"]).unwrap().emit,
            Emit::Target
        );
        assert_eq!(
            parse(&["--emit", "summary", "f.py"]).unwrap().emit,
            Emit::Summary
        );
    }

    #[test]
    fn an_unknown_emit_value_lists_the_accepted_forms() {
        let error = parse(&["--emit", "yaml", "f.py"]).unwrap_err();
        assert!(error.contains("summary"), "{error}");
        assert!(error.contains("ir"), "{error}");
        assert!(error.contains("rust"), "{error}");
    }

    #[test]
    fn the_backend_can_be_selected() {
        assert_eq!(parse(&["--backend", "go", "f.py"]).unwrap().backend, "go");
    }

    #[test]
    fn no_file_is_an_error() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["--emit", "ir"]).is_err());
    }

    #[test]
    fn a_flag_without_its_value_is_an_error() {
        assert!(parse(&["f.py", "--emit"]).is_err());
        assert!(parse(&["f.py", "--backend"]).is_err());
    }

    #[test]
    fn two_files_are_an_error() {
        assert!(parse(&["a.py", "b.py"]).is_err());
    }

    #[test]
    fn an_unknown_option_is_an_error() {
        assert!(parse(&["--nonesuch", "f.py"]).is_err());
    }

    #[test]
    fn help_exits_without_an_error_message() {
        // An empty message means "print usage, say nothing else" — asking for help is not a
        // mistake to be scolded for.
        assert_eq!(parse(&["--help"]).unwrap_err(), "");
    }
}
