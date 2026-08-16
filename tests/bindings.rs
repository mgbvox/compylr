//! The Python boundary, verified by importing a real extension module.
//!
//! Nothing here inspects emitted text. Whether `bool` survives as a Python `bool` rather than an
//! `int`, whether a keyword call binds correctly, and whether dividing by zero raises
//! `ZeroDivisionError` are all facts about a compiled artifact, and the only way to establish them
//! is to build one and call into it.
//!
//! One extension is built for the whole file and reused, because compiling a crate that depends on
//! PyO3 is the expensive part. The builds share a target directory so PyO3 itself is compiled once
//! across every test binary rather than once per crate.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use compylr::backend::bindings::{cargo_manifest, emit_extension, module_name};
use compylr::frontend::parse_source;
use compylr::ir::Unit;
use compylr::lower::lower_source;

/// PyO3 version the generated crate depends on. Matches this crate's own pin.
const PYO3_VERSION: &str = "0.29.2";

fn unit_from(source: &str) -> Unit {
    let parsed = parse_source(source).expect("fixture must parse");
    let functions =
        lower_source(&parsed).unwrap_or_else(|e| panic!("should lower: {}", e.render(source)));
    let mut unit = Unit::new();
    for function in functions {
        unit.add_function(function).unwrap();
    }
    unit.validate().expect("calls must resolve");
    unit
}

/// Build `source` into an importable extension module.
///
/// Returns the directory the module can be imported from, and the module's name.
fn build_extension(label: &str, source: &str) -> (PathBuf, String) {
    let unit = unit_from(source);
    let name = module_name(&unit);
    let emitted = emit_extension(&unit).expect("must emit");

    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(label);
    let src = root.join("src");
    std::fs::create_dir_all(&src).expect("crate directory");
    std::fs::write(root.join("Cargo.toml"), cargo_manifest(&unit, PYO3_VERSION)).unwrap();
    std::fs::write(src.join("lib.rs"), &emitted).unwrap();

    // An extension module resolves Python's symbols from the interpreter that loads it rather
    // than linking libpython, which is what `extension-module` asks for. On macOS the linker has
    // to be told those symbols are allowed to be missing at link time.
    let cargo_dir = root.join(".cargo");
    std::fs::create_dir_all(&cargo_dir).unwrap();
    std::fs::write(
        cargo_dir.join("config.toml"),
        "[target.aarch64-apple-darwin]\n\
         rustflags = [\"-C\", \"link-arg=-undefined\", \"-C\", \"link-arg=dynamic_lookup\"]\n\
         [target.x86_64-apple-darwin]\n\
         rustflags = [\"-C\", \"link-arg=-undefined\", \"-C\", \"link-arg=dynamic_lookup\"]\n",
    )
    .unwrap();

    let shared_target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("shared-cargo-target");
    let mut command = Command::new("cargo");
    command
        .current_dir(&root)
        .arg("build")
        .arg("--release")
        .env("CARGO_TARGET_DIR", &shared_target);

    // Under `cargo llvm-cov` the parent build exports coverage instrumentation flags. Cargo would
    // apply them to this nested build too, and the generated cdylib has no profiler runtime to
    // link against, so it would fail for a reason that has nothing to do with the code under
    // test. The generated crate is the subject here, not something whose coverage is measured.
    for leaked in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTDOCFLAGS",
        "CARGO_ENCODED_RUSTDOCFLAGS",
        "LLVM_PROFILE_FILE",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTFLAGS",
    ] {
        command.env_remove(leaked);
    }

    let output = command.output().expect("cargo must be available");
    assert!(
        output.status.success(),
        "generated crate did not build:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        emitted
    );

    // Python imports `<name>.so`; cargo produces `lib<name>.dylib` or `lib<name>.so`.
    let release = shared_target.join("release");
    let built = ["dylib", "so"]
        .iter()
        .map(|ext| release.join(format!("lib{name}.{ext}")))
        .find(|path| path.exists())
        .unwrap_or_else(|| panic!("no shared library produced in {}", release.display()));

    let importable = root.join(format!("{name}.so"));
    std::fs::copy(&built, &importable).expect("stage the module for import");
    (root, name)
}

/// Run a Python snippet with `dir` on `sys.path`, returning stdout.
fn python(dir: &Path, script: &str) -> String {
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .env("PYTHONPATH", dir)
        .output()
        .expect("python3 must be available");
    assert!(
        output.status.success(),
        "python failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("output must be UTF-8")
}

/// A unit covering every binding concern in one build.
const SOURCE: &str = concat!(
    "def add(a: int, b: int) -> int:\n    return a + b\n\n",
    "def ratio(a: int, b: int) -> float:\n    return a / b\n\n",
    "def is_big(n: int) -> bool:\n    return n > 100\n\n",
    "def shout(s: str) -> str:\n    return s + \"!\"\n\n",
    "def nothing(n: int) -> None:\n    pass\n\n",
    "def half(n: int) -> int:\n    return n // 0\n\n",
    "def outer(n: int) -> int:\n    return half(n) + 1\n\n",
    "def grow(n: int) -> int:\n    return n * 2\n",
);

/// The shared extension, built exactly once.
///
/// Every test in this file needs the same module. Building per test would race: `cargo test` runs
/// them in parallel and they would write the same crate directory and the same target directory at
/// once. That produces a suite which passes on a warm cache and fails on a cold one, which is
/// worse than a suite that simply fails.
static EXTENSION: LazyLock<(PathBuf, String)> =
    LazyLock::new(|| build_extension("bindings", SOURCE));

fn built() -> (PathBuf, String) {
    EXTENSION.clone()
}

#[test]
fn every_function_is_exposed_and_nothing_else_is() {
    let (dir, name) = built();
    let out = python(
        &dir,
        &format!(
            r#"
import {name} as m
names = sorted(n for n in dir(m) if not n.startswith("__"))
print(",".join(names))
"#
        ),
    );
    assert_eq!(
        out.trim(),
        "add,grow,half,is_big,nothing,outer,ratio,shout",
        "the module must expose exactly the unit's functions, with no backend helpers leaking"
    );
}

#[test]
fn arguments_bind_positionally_and_by_keyword() {
    let (dir, name) = built();
    let out = python(
        &dir,
        &format!(
            r#"
import {name} as m
print(m.add(1, 2))
print(m.add(a=1, b=2))
print(m.add(b=2, a=1))
print(m.add(1, b=2))
"#
        ),
    );
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        ["3", "3", "3", "3"],
        "a caller replacing an interpreted function must not have to change how it calls"
    );
}

#[test]
fn each_type_crosses_the_boundary_intact() {
    let (dir, name) = built();
    let out = python(
        &dir,
        &format!(
            r#"
import {name} as m
print(repr(m.add(2, 3)))
print(repr(m.ratio(7, 2)))
print(repr(m.shout("hi")))
print(repr(m.nothing(1)))
print(repr(m.is_big(500)))
print(type(m.is_big(500)).__name__)
"#
        ),
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "5");
    assert_eq!(lines[1], "3.5", "true division must arrive as a float");
    assert_eq!(lines[2], "'hi!'");
    assert_eq!(lines[3], "None", "a unit return is None");
    assert_eq!(lines[4], "True");
    assert_eq!(
        lines[5], "bool",
        "a bool return must be a Python bool, not an int, matching the IR's rule that booleans \
         are not numbers"
    );
}

#[test]
fn wrong_argument_types_and_counts_raise_type_error() {
    let (dir, name) = built();
    let out = python(
        &dir,
        &format!(
            r#"
import {name} as m
for call in [lambda: m.add("x", 1), lambda: m.add(1), lambda: m.add(1, 2, 3), lambda: m.shout(5)]:
    try:
        call()
        print("NO ERROR")
    except TypeError:
        print("TypeError")
    except Exception as e:
        print(type(e).__name__)
"#
        ),
    );
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        ["TypeError"; 4],
        "the compiled function's contract is exactly the annotations the user wrote"
    );
}

#[test]
fn arithmetic_failures_raise_the_exception_python_would() {
    let (dir, name) = built();
    let out = python(
        &dir,
        &format!(
            r#"
import {name} as m
try:
    m.half(10)
except ZeroDivisionError:
    print("ZeroDivisionError")
try:
    m.ratio(1, 0)
except ZeroDivisionError:
    print("ZeroDivisionError")
try:
    m.grow(2**62)
except OverflowError:
    print("OverflowError")
# A failure two calls deep must still surface as the same exception.
try:
    m.outer(10)
except ZeroDivisionError:
    print("nested ZeroDivisionError")
# And the interpreter must still be usable afterwards.
print(m.add(40, 2))
"#
        ),
    );
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            "ZeroDivisionError",
            "ZeroDivisionError",
            "OverflowError",
            "nested ZeroDivisionError",
            "42",
        ],
        "existing error handling around a function must keep working once it is compiled"
    );
}

#[test]
fn the_module_name_carries_the_fingerprint() {
    let unit = unit_from("def add(a: int, b: int) -> int:\n    return a + b\n");
    let name = module_name(&unit);
    assert!(
        name.contains(&format!("{:016x}", unit.fingerprint())),
        "the module name must encode build identity so a rebuild can load beside its predecessor"
    );

    // A different unit compiles to a differently named module...
    let other = unit_from("def add(a: int, b: int) -> int:\n    return a - b\n");
    assert_ne!(name, module_name(&other));

    // ...while a cosmetic change does not, because the fingerprint is over the IR.
    let reformatted =
        unit_from("# a comment\ndef add(a: int, b: int) -> int:\n\n        return a + b\n");
    assert_eq!(name, module_name(&reformatted));
}
