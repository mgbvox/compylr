//! The frontend, run over Python nobody wrote for this compiler.
//!
//! The curated rejection corpus demonstrates that each *known* refusal is located. It cannot
//! demonstrate that an unanticipated construct is refused rather than crashed on, because every
//! program in it was written by someone who already knew the answer. So this walks ordinary
//! Python -- this repository's own, the demo, the scripts, and the standard library of whichever
//! interpreter is installed -- and asserts one property over all of it:
//!
//! **every outcome is a lowered unit or a diagnostic carrying a source position.** A panic fails.
//! A failure without a position fails.
//!
//! It compiles nothing, so it costs seconds rather than minutes. It also reports the proportion
//! of top-level members the frontend accepted, and **deliberately asserts no threshold on it**:
//! the corpus differs between machines, so a threshold would make the suite fail for reasons that
//! have nothing to do with the compiler. The number is there to make growth in the accepted subset
//! a measured quantity rather than an impression.

use std::panic::{AssertUnwindSafe, catch_unwind};

use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source_members;

mod support;
use support::drivers;

/// Python's own stance, which is what an unconfigured compilation resolves to.
fn python_stance() -> compylr_ir::Behavior {
    compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR)
}

/// One member's source, sliced out by the line range Python reported for it.
fn member_source(text: &str, first_line: usize, last_line: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if first_line == 0 || last_line > lines.len() || first_line > last_line {
        return None;
    }
    Some(lines[first_line - 1..last_line].join("\n"))
}

/// What happened to one member.
enum Outcome {
    Lowered,
    Located,
    /// The two failures this test exists to catch, with enough to find them again.
    Unlocated(String),
    Panicked(String),
}

fn classify(source: &str) -> Outcome {
    // A panic is caught rather than allowed to abort, so the report names the input that caused
    // it instead of leaving a stack trace and no file.
    let result = catch_unwind(AssertUnwindSafe(|| {
        let parsed = parse_source(source).ok()?;
        Some(lower_source_members(&parsed, python_stance()))
    }));

    match result {
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_else(|| "a panic with no message".to_string());
            Outcome::Panicked(message)
        }
        // A member that does not parse on its own is a fact about slicing it out of its file,
        // not about lowering; the file already parsed as a whole.
        Ok(None) => Outcome::Located,
        Ok(Some(Ok(_))) => Outcome::Lowered,
        Ok(Some(Err(error))) => {
            let span = error.span();
            let rendered = error.render(source);
            let within = span.end() as usize <= source.len();
            // A zero-width span at the very start is the signature of a span that was never set,
            // rather than of a diagnostic about the first character.
            let set = !(span.start() == 0 && span.end() == 0 && !source.is_empty());
            let positioned = rendered
                .split(':')
                .next()
                .and_then(|line| line.trim().parse::<usize>().ok())
                .is_some_and(|line| line >= 1);

            if within && set && positioned {
                Outcome::Located
            } else {
                Outcome::Unlocated(format!(
                    "{rendered} (span {span:?}, source {} bytes)",
                    source.len()
                ))
            }
        }
    }
}

#[test]
fn arbitrary_python_is_refused_rather_than_crashed_on() {
    let Some((files, unparsed)) = drivers::robustness_corpus() else {
        eprintln!("skipping: no python3 on PATH to locate a corpus with");
        return;
    };
    assert!(
        !files.is_empty(),
        "the robustness corpus is empty; it should include this repository's own Python at least"
    );

    let (mut lowered, mut total) = (0usize, 0usize);
    let mut panics: Vec<String> = Vec::new();
    let mut unlocated: Vec<String> = Vec::new();

    for file in &files {
        let Ok(text) = std::fs::read_to_string(&file.path) else {
            continue;
        };
        for member in &file.members {
            let Some(source) = member_source(&text, member.first_line, member.last_line) else {
                continue;
            };
            total += 1;
            let where_ = format!(
                "{}:{} ({})",
                file.path.display(),
                member.first_line,
                member.name
            );
            match classify(&source) {
                Outcome::Lowered => lowered += 1,
                Outcome::Located => {}
                Outcome::Unlocated(detail) => unlocated.push(format!("{where_}: {detail}")),
                Outcome::Panicked(message) => panics.push(format!("{where_}: {message}")),
            }
        }
    }

    // Reported, never asserted -- the corpus differs between machines. See the module doc.
    let percent = if total == 0 {
        0.0
    } else {
        (lowered as f64 / total as f64) * 100.0
    };
    println!(
        "\n[robustness] {lowered} of {total} top-level members lowered ({percent:.1}%) \
         across {} files; {unparsed} files did not parse on this interpreter",
        files.len()
    );

    assert!(
        panics.is_empty(),
        "lowering panicked on {} of {total} members; a panic is never an acceptable outcome:\n{}",
        panics.len(),
        panics.join("\n")
    );
    assert!(
        unlocated.is_empty(),
        "{} of {total} diagnostics carried no usable source position:\n{}",
        unlocated.len(),
        unlocated.join("\n")
    );
}

#[test]
fn the_corpus_reaches_beyond_this_repository() {
    // Guards the corpus itself. If the standard library stopped being located, this would still
    // pass over the repository's own Python and quietly stop establishing the property that
    // matters -- that the input was not written for this compiler.
    let Some((files, _)) = drivers::robustness_corpus() else {
        eprintln!("skipping: no python3 on PATH to locate a corpus with");
        return;
    };
    let root = drivers::workspace_root();
    let outside = files.iter().filter(|f| !f.path.starts_with(&root)).count();
    assert!(
        outside > 0,
        "every corpus file is inside this repository; the interpreter's standard library was not \
         located, so the walk is only over Python written for this compiler"
    );
}
