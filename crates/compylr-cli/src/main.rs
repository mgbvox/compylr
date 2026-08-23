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
use compylr_core::behavior::{BehaviorRequest, LanguagePair};
use compylr_core::bridge::BuildKey;
use compylr_core::pass::{self, PassConfig};
use compylr_core::verify::verify;
use compylr_core::{Backend, Behavior, Frontend, Source};
// The summary quotes types back in the language of the file being inspected, so it uses the
// frontend's spelling rather than the IR's neutral one.
use compylr_frontend_python::PythonTypeName;
use compylr_registry::{backends, bridges, frontends, passes};

/// The source language a caller gets when it does not name one.
///
/// A default, not an assumption. `--frontend` selects any other, resolved through the same
/// registry `--backend` uses, so neither end of the pipeline is the one that has to be Python.
const DEFAULT_FRONTEND: &str = "python";

/// The target language a caller gets when it does not name one.
const DEFAULT_BACKEND: &str = "rust";

const USAGE: &str = "\
usage: compylr [--emit summary|ir|rust|crate] [--out DIR]
               [--frontend NAME] [--backend NAME] [--behavior SPEC] <file>

  --emit summary   unit fingerprint and each function's signature (default)
  --emit ir        the IR artifact, as JSON
  --emit rust      the translated functions, without performing a build
  --emit crate     every generated file; requires --out
  --out DIR        destination for --emit crate
  --frontend NAME  source language (default: python)
  --backend NAME   target backend (default: rust)
  --behavior SPEC  which language supplies the meaning of each operation:
                   a language name for every axis, or comma-separated
                   axis=language assignments. Unnamed axes take the source
                   language's meaning, which is also the default.
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
    frontend: String,
    backend: String,
    out: Option<PathBuf>,
    /// What the user asked of each axis, before it is resolved against the two languages.
    ///
    /// Held unresolved because resolution needs the components, and parsing must not depend on
    /// which languages happen to be registered — an unknown *axis* is a mistake worth reporting
    /// even for a pair that would itself have been refused.
    behavior: BehaviorRequest,
}

/// Parse arguments by hand.
///
/// Four flags do not justify an argument-parsing dependency, and the crate's dependency surface is
/// currently the vendored ruff tree plus PyO3 and serde.
fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut path: Option<PathBuf> = None;
    let mut emit = Emit::Summary;
    let mut frontend = DEFAULT_FRONTEND.to_string();
    let mut backend = DEFAULT_BACKEND.to_string();
    let mut out: Option<PathBuf> = None;
    let mut behavior = BehaviorRequest::inherit();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--emit" => {
                let value = args.next().ok_or("--emit needs a value")?;
                emit = Emit::parse(&value)?;
            }
            "--frontend" => {
                frontend = args.next().ok_or("--frontend needs a value")?;
            }
            "--backend" => {
                backend = args.next().ok_or("--backend needs a value")?;
            }
            "--out" => {
                out = Some(PathBuf::from(args.next().ok_or("--out needs a value")?));
            }
            "--behavior" => {
                let value = args.next().ok_or("--behavior needs a value")?;
                behavior = parse_behavior(&value)?;
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
        frontend,
        backend,
        out,
        behavior,
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

/// Parse `--behavior`: either a language name, or comma-separated `axis=language` assignments.
///
/// A bare name is expanded to every axis rather than kept as a second shape, so "naming a
/// language means naming it for all six" is true by construction here too.
///
/// An unknown *axis* is rejected at parse time, before any component is resolved. That is
/// deliberate: silently dropping `floor_div=rust` would compile the program the user did not ask
/// for and say nothing about it. An unknown *language* cannot be judged yet — which two are
/// acceptable depends on the pair — so it is left for resolution.
fn parse_behavior(spec: &str) -> Result<BehaviorRequest, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("--behavior needs a language name or axis=language assignments".to_string());
    }

    if !spec.contains('=') {
        return Ok(BehaviorRequest::language(spec));
    }

    let mut pairs = Vec::new();
    for assignment in spec.split(',') {
        let assignment = assignment.trim();
        let (axis, language) = assignment.split_once('=').ok_or_else(|| {
            format!(
                "'{assignment}' is not an axis=language assignment; \
                 --behavior takes a language name or comma-separated axis=language pairs"
            )
        })?;
        pairs.push((axis.trim().to_string(), language.trim().to_string()));
    }
    BehaviorRequest::from_pairs(pairs).map_err(|error| error.to_string())
}

/// Resolve a behavior request against the two languages this run compiles between.
///
/// Both declarations come from the components themselves, and the set of names compylr recognises
/// is the union of the two registries — so a user who names a reserved target is told it is not
/// one of *these* two rather than that it does not exist.
fn resolve_behavior(
    request: &BehaviorRequest,
    frontend: &dyn Frontend,
    backend: &dyn Backend,
) -> Result<Behavior, String> {
    let mut known: Vec<&str> = frontends::names();
    known.extend(backends::names());
    known.sort_unstable();
    known.dedup();

    let pair = LanguagePair {
        source: frontend.name(),
        source_behavior: frontend.behavior(),
        target: backend.name(),
        target_behavior: backend.behavior(),
        known: &known,
    };
    compylr_core::resolve(request, &pair, None).map_err(|error| error.to_string())
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
    // Resolved the same way the backend is, and reporting the same three answers. A reserved
    // source language reads as planned rather than as a typo.
    let frontend = frontends::lookup(&options.frontend).map_err(|error| error.to_string())?;
    // Resolved beside the components and before the file is touched, for the reason they are:
    // a user who both named a behavior that does not exist and mistyped the path should hear
    // about the behavior, not about whichever thing the code happened to reach first.
    let behavior = resolve_behavior(&options.behavior, frontend, backend)?;

    let source = std::fs::read_to_string(&options.path)
        .map_err(|error| format!("could not read {}: {error}", options.path.display()))?;

    // The frontend does the parsing and the assembly. Reading the file is this crate's business
    // because the trait takes text: the decorator's sources come from a live function object and
    // may correspond to no file at all.
    let mut unit = frontend
        .lower(&[Source::new(source, behavior)])
        .map_err(|error| error.to_string())?;
    // Unconditional, and the same check `compile` runs. A CLI with its own idea of what is
    // well formed would become a second source of answers.
    verify(&unit).map_err(|error| error.to_string())?;

    // The same passes a real build runs. A CLI that showed unoptimized source would answer
    // "what does this become?" with something the toolchain never sees, which is the one thing
    // this command exists not to do.
    let directed = passes::for_pair(&options.frontend, &options.backend);
    pass::run(&mut unit, &PassConfig::default(), &directed).map_err(|error| error.to_string())?;

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
            let files =
                backend.post_process(backend.emit(&unit).map_err(|error| error.to_string())?);
            files
                .get(compylr_backend_rust::rust::GENERATED_PATH)
                .cloned()
                .ok_or_else(|| "this backend emits no translated-code file".to_string())
        }
        Emit::Crate => {
            // A buildable crate, which means the host boundary as well as the translation — so
            // this form, unlike `--emit rust`, needs the pair to be bridged.
            let host = bridges::lookup(&options.frontend, &options.backend)
                .map_err(|error| error.to_string())?;
            let key = BuildKey {
                fingerprint: unit.fingerprint(),
                target: options.backend.clone(),
                passes: PassConfig::default().optimization.key(),
            };
            let artifact = host.emit(&unit, &key).map_err(|error| error.to_string())?;
            // Written for a person to read, so it is formatted. Outside emission, which stays a
            // pure function of the unit.
            let files = backend.post_process(artifact.files);
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

    /// Both ends of the pipeline are selectable, and neither is the one that has to be Python.
    #[test]
    fn the_frontend_can_be_selected() {
        assert_eq!(
            parse(&["--frontend", "typescript", "f.ts"])
                .unwrap()
                .frontend,
            "typescript"
        );
    }

    #[test]
    fn both_ends_default_when_unnamed() {
        let options = parse(&["f.py"]).unwrap();
        assert_eq!(options.frontend, DEFAULT_FRONTEND);
        assert_eq!(options.backend, DEFAULT_BACKEND);
    }

    #[test]
    fn a_frontend_without_a_value_is_refused() {
        assert!(parse(&["--frontend"]).is_err());
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
