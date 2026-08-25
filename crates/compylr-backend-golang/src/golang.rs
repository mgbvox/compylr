//! The Go backend as a registered component.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use compylr_core::backend::{Backend, BackendError, GeneratedFiles};
use compylr_ir::{
    Checked, Guarantee, IndexOrigin, IntegerDivision, LanguageBehavior, RemSign, Remainder,
    Rounding, SequenceIndex, TextUnits, Unit,
};

use crate::compat::GO_COMPAT_SOURCE;
use crate::emit::emit_go_unit;

/// The Go backend.
#[derive(Debug)]
pub struct GoBackend;

/// What Go preserves.
const GO_PRESERVES: &[Guarantee] = &[
    Guarantee::DivisionByZeroReported,
    Guarantee::FloatOrderPreserved,
];

/// What Go means natively on every behavior axis.
pub const GO_BEHAVIOR: LanguageBehavior = LanguageBehavior {
    integer_overflow: Checked::Unchecked,
    integer_division: IntegerDivision {
        rounding: Rounding::TowardZero,
        checked: Checked::Reported,
    },
    exact_division: Checked::Reported,
    remainder: Remainder {
        sign: RemSign::Dividend,
        checked: Checked::Reported,
    },
    sequence_index: SequenceIndex {
        origin: IndexOrigin::FromStart,
        checked: Checked::Reported,
    },
    text_length: TextUnits::Utf8Bytes,
};

impl Backend for GoBackend {
    fn name(&self) -> &'static str {
        "go"
    }

    fn preserves(&self) -> &'static [Guarantee] {
        GO_PRESERVES
    }

    fn behavior(&self) -> &'static LanguageBehavior {
        &GO_BEHAVIOR
    }

    fn post_process(&self, files: GeneratedFiles) -> GeneratedFiles {
        let mut formatted = GeneratedFiles::new();
        for (path, content) in files {
            if path.ends_with(".go") {
                formatted.insert(path, format_go_source(&content));
            } else {
                formatted.insert(path, content);
            }
        }
        formatted
    }

    fn emit(&self, unit: &Unit) -> Result<GeneratedFiles, BackendError> {
        let mut files = BTreeMap::new();
        files.insert(
            "go.mod".to_string(),
            "module compylr\n\ngo 1.20\n".to_string(),
        );
        files.insert("compat.go".to_string(), GO_COMPAT_SOURCE.to_string());
        files.insert("generated.go".to_string(), emit_go_unit(unit));
        Ok(files)
    }
}

/// Format Go source with gofmt if available.
pub fn format_go_source(source: &str) -> String {
    use std::io::Write as _;
    let Ok(mut child) = Command::new("gofmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return source.to_string();
    };

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(source.as_bytes());
    }

    match child.wait_with_output() {
        Ok(output) if output.status.success() => {
            String::from_utf8(output.stdout).unwrap_or_else(|_| source.to_string())
        }
        _ => source.to_string(),
    }
}
