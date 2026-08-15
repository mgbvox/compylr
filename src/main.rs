//! Thin binary entry point.
//!
//! The pipeline lives in the library so it can be tested without a user-facing surface. A real
//! CLI is a later change; for now this reports what a file lowers to, which makes the crate
//! runnable without pretending to be finished.

use std::path::PathBuf;
use std::process::ExitCode;

use compylr::frontend::parse_file;
use compylr::ir::Unit;
use compylr::lower::lower_source;

fn main() -> ExitCode {
    let Some(arg) = std::env::args().nth(1) else {
        eprintln!("usage: compylr <file.py>");
        return ExitCode::FAILURE;
    };

    let path = PathBuf::from(arg);
    let parsed = match parse_file(&path) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let functions = match lower_source(&parsed) {
        Ok(functions) => functions,
        Err(error) => {
            eprintln!("error: {}", error.render(&source));
            return ExitCode::FAILURE;
        }
    };

    let mut unit = Unit::new();
    for function in functions {
        if let Err(error) = unit.add_function(function) {
            eprintln!("error: {}", error.render(&source));
            return ExitCode::FAILURE;
        }
    }

    if let Err(error) = unit.validate() {
        eprintln!("error: {}", error.render(&source));
        return ExitCode::FAILURE;
    }

    println!("unit fingerprint: {:016x}", unit.fingerprint());
    for function in unit.functions() {
        println!(
            "  {} ({} params) -> {}",
            function.name,
            function.params.len(),
            function.ret.python_name()
        );
    }
    ExitCode::SUCCESS
}
