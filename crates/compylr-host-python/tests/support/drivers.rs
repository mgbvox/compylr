//! Reading the fixture drivers from Rust.
//!
//! A driver is literal data so that both differential tiers can consume the same declaration. The
//! declaration's *meaning* -- what shapes are legal, which members a call reaches -- is defined
//! once, in `python/fixtures/drivers/_runner.py`. Rather than restate it here in a second parser
//! that would be free to disagree, this asks that module to read the corpus and hand back JSON.
//! D1 sanctions exactly this: "the Rust harness reads it with `ast.literal_eval` via one `python3`
//! invocation".

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

/// One driver: the calls it declares, and the fixture members those calls reach.
#[derive(Debug, Clone)]
pub struct Driver {
    /// The declared calls, as JSON, in the order they must run.
    pub calls: Vec<Value>,
    /// Every fixture member named, including classes constructed to pass as arguments.
    pub members: Vec<String>,
}

/// The workspace root, which the fixture tree hangs off.
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate lives at <root>/crates/<name>")
        .to_path_buf()
}

pub fn accepted_dir() -> PathBuf {
    workspace_root().join("python/fixtures/accepted")
}

pub fn drivers_dir() -> PathBuf {
    workspace_root().join("python/fixtures/drivers")
}

/// Every accepted fixture's stem, read from the directory rather than listed.
///
/// Derived, never hardcoded: a literal list drifted once already and hid a real defect, so the
/// corpus is always whatever is on disk.
pub fn accepted_stems() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(accepted_dir())
        .expect("accepted fixtures directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".py"))
        .map(|name| name.trim_end_matches(".py").to_string())
        .collect();
    names.sort();
    names
}

/// Every driver's stem. Files beginning with `_` are the shared runner, not drivers.
pub fn driver_stems() -> Vec<String> {
    let dir = drivers_dir();
    if !dir.exists() {
        return Vec::new();
    }
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("drivers directory must be readable")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".py") && !name.starts_with('_'))
        .map(|name| name.trim_end_matches(".py").to_string())
        .collect();
    names.sort();
    names
}

/// Whether a Python interpreter can be found. Its absence is a fact about the machine.
pub fn python() -> Option<&'static str> {
    for candidate in ["python3", "python"] {
        if Command::new(candidate).arg("--version").output().is_ok() {
            return Some(candidate);
        }
    }
    None
}

/// Load every driver by asking `_runner` to read them.
///
/// Returns `None` when no interpreter is installed, which callers report as a skip naming the
/// missing tool rather than as a failure.
pub fn load_all() -> Option<BTreeMap<String, Driver>> {
    let interpreter = python()?;
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import _runner

out = {}
for path in sorted(__import__("pathlib").Path(sys.argv[1]).glob("*.py")):
    if path.name.startswith("_"):
        continue
    calls = _runner.load_calls(path)
    out[path.stem] = {
        "calls": _runner.encode_calls(calls),
        "members": sorted(_runner.members_named(calls)),
    }
print(json.dumps(out))
"#;
    let output = Command::new(interpreter)
        .arg("-c")
        .arg(script)
        .arg(drivers_dir())
        .output()
        .expect("running the interpreter must not fail once it has been located");
    assert!(
        output.status.success(),
        "reading the drivers failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: BTreeMap<String, Value> =
        serde_json::from_slice(&output.stdout).expect("the driver reader emits JSON");

    Some(
        parsed
            .into_iter()
            .map(|(stem, value)| {
                let calls = value["calls"]
                    .as_array()
                    .expect("calls is an array")
                    .to_vec();
                let members = value["members"]
                    .as_array()
                    .expect("members is an array")
                    .iter()
                    .map(|m| m.as_str().expect("a member name is a string").to_string())
                    .collect();
                (stem, Driver { calls, members })
            })
            .collect(),
    )
}
