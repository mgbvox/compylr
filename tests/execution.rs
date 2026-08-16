//! Python operator semantics, verified by running the emitted code.
//!
//! Reading emitted text cannot catch the failure that matters here. A floor-division helper that
//! adjusts the quotient in the wrong direction still *looks* correct in a string comparison, and
//! a snapshot of it would be just as wrong as the code. So every test in this file lowers Python,
//! emits Rust, compiles it with `rustc`, runs the binary, and asserts on what it printed.
//!
//! That is slower than a string assertion, and it is the point: these are precisely the cases
//! where Rust's native operators disagree with Python's, so the only convincing evidence is a
//! number produced by executing the result.

use std::path::PathBuf;
use std::process::Command;

use compylr::backend::lookup;
use compylr::frontend::parse_source;
use compylr::ir::Unit;
use compylr::lower::lower_source;

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

/// Emit `source`, append `main_body`, compile, run, and return stdout.
///
/// `label` only has to be unique across tests so parallel runs do not fight over a path.
fn run(label: &str, source: &str, main_body: &str) -> String {
    let unit = unit_from(source);
    let emitted = lookup("rust").unwrap().emit(&unit).expect("must emit");

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(label);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    let source_path = dir.join("main.rs");
    let binary_path = dir.join("program");

    let program = format!("{emitted}\nuse generated::*;\nfn main() {{\n{main_body}\n}}\n");
    std::fs::write(&source_path, &program).expect("write generated source");

    let compile = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-o")
        .arg(&binary_path)
        .arg(&source_path)
        .output()
        .expect("rustc must be available; it ships with the toolchain that runs these tests");

    assert!(
        compile.status.success(),
        "generated Rust did not compile:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&compile.stderr),
        program
    );

    let output = Command::new(&binary_path)
        .output()
        .expect("run the program");
    assert!(
        output.status.success(),
        "generated program aborted, which means a failure escaped as a panic:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("output must be UTF-8")
}

#[test]
fn floor_division_rounds_toward_negative_infinity() {
    // Rust's `/` truncates toward zero, so it answers -3, 3, and -3 for the first three.
    let out = run(
        "floordiv",
        concat!(
            "def fdiv(a: int, b: int) -> int:\n    return a // b\n\n",
            "def ffdiv(a: float, b: float) -> float:\n    return a // b\n",
        ),
        r#"
    println!("{}", fdiv(-7, 2).unwrap());
    println!("{}", fdiv(7, -2).unwrap());
    println!("{}", fdiv(-6, 2).unwrap());
    println!("{}", fdiv(7, 2).unwrap());
    println!("{:?}", ffdiv(-7.0, 2.0).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0], "-4",
        "-7 // 2 must floor to -4, not truncate to -3"
    );
    assert_eq!(lines[1], "-4", "7 // -2 must floor to -4");
    assert_eq!(lines[2], "-3", "exact division is unaffected");
    assert_eq!(lines[3], "3", "positive operands agree with Rust");
    assert_eq!(lines[4], "-4.0", "float floor division floors too");
}

#[test]
fn remainder_takes_the_sign_of_the_divisor() {
    // Rust's `%` takes the sign of the dividend, so it answers -1 and 1 for the first two.
    let out = run(
        "remainder",
        concat!(
            "def m(a: int, b: int) -> int:\n    return a % b\n\n",
            "def fm(a: float, b: float) -> float:\n    return a % b\n",
        ),
        r#"
    println!("{}", m(-7, 2).unwrap());
    println!("{}", m(7, -2).unwrap());
    println!("{}", m(7, 2).unwrap());
    println!("{}", m(-6, 3).unwrap());
    println!("{:?}", fm(-7.0, 2.0).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "1", "-7 % 2 must be 1, not -1");
    assert_eq!(lines[1], "-1", "7 % -2 must be -1");
    assert_eq!(lines[2], "1");
    assert_eq!(lines[3], "0");
    assert_eq!(
        lines[4], "1.0",
        "float remainder follows the divisor's sign"
    );
}

#[test]
fn floor_division_and_remainder_stay_consistent() {
    // The invariant that ties the two together. If either helper is wrong for some sign
    // combination, this catches it without needing to know which.
    let out = run(
        "identity",
        concat!(
            "def fdiv(a: int, b: int) -> int:\n    return a // b\n\n",
            "def m(a: int, b: int) -> int:\n    return a % b\n",
        ),
        r#"
    let mut bad = Vec::new();
    for a in [-13i64, -7, -1, 0, 1, 7, 13, 100] {
        for b in [-7i64, -3, -1, 1, 3, 7] {
            let q = fdiv(a, b).unwrap();
            let r = m(a, b).unwrap();
            if q * b + r != a {
                bad.push(format!("a={a} b={b} q={q} r={r}"));
            }
            // Python also guarantees the remainder lies in [0, b) or (b, 0].
            let in_range = if b > 0 { r >= 0 && r < b } else { r <= 0 && r > b };
            if !in_range {
                bad.push(format!("range a={a} b={b} r={r}"));
            }
        }
    }
    println!("{}", bad.len());
    for line in bad { println!("{line}"); }
"#,
    );
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        "0",
        "(a // b) * b + (a % b) == a failed for: {}",
        lines.collect::<Vec<_>>().join("; ")
    );
}

#[test]
fn true_division_always_yields_a_float() {
    let out = run(
        "truediv",
        "def ratio(a: int, b: int) -> float:\n    return a / b\n",
        r#"
    println!("{:?}", ratio(7, 2).unwrap());
    println!("{:?}", ratio(-7, 2).unwrap());
    println!("{:?}", ratio(6, 3).unwrap());
    // The declared Rust return type must be f64, not i64.
    let value: f64 = ratio(1, 4).unwrap();
    println!("{value:?}");
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "3.5", "7 / 2 must be 3.5, not Rust's 3");
    assert_eq!(lines[1], "-3.5");
    assert_eq!(lines[2], "2.0", "even an exact result stays a float");
    assert_eq!(lines[3], "0.25");
}

#[test]
fn remaining_operators_behave() {
    let out = run(
        "operators",
        concat!(
            "def cat(a: str, b: str) -> str:\n    return a + b\n\n",
            "def eq(a: int, b: int) -> bool:\n    return a == b\n\n",
            "def ne(a: int, b: int) -> bool:\n    return a != b\n\n",
            "def lt(a: int, b: int) -> bool:\n    return a < b\n\n",
            "def le(a: int, b: int) -> bool:\n    return a <= b\n\n",
            "def gt(a: int, b: int) -> bool:\n    return a > b\n\n",
            "def ge(a: int, b: int) -> bool:\n    return a >= b\n\n",
            "def streq(a: str, b: str) -> bool:\n    return a == b\n",
        ),
        r#"
    println!("{}", cat(String::from("a"), String::from("b")).unwrap());
    println!("{}", eq(1, 1).unwrap());
    println!("{}", ne(1, 2).unwrap());
    println!("{}", lt(1, 2).unwrap());
    println!("{}", le(2, 2).unwrap());
    println!("{}", gt(3, 2).unwrap());
    println!("{}", ge(2, 2).unwrap());
    println!("{}", streq(String::from("x"), String::from("y")).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "ab", "string concatenation");
    assert_eq!(
        &lines[1..7],
        ["true", "true", "true", "true", "true", "true"]
    );
    assert_eq!(lines[7], "false");
}

#[test]
fn a_string_used_twice_is_not_moved_on_first_use() {
    // Python has no notion of a value being consumed by being read. If emission moved a `String`
    // parameter, this would fail to compile rather than fail an assertion.
    let out = run(
        "strings",
        "def twice(s: str) -> str:\n    t = s + s\n    return t + s\n",
        r#"
    println!("{}", twice(String::from("ab")).unwrap());
"#,
    );
    assert_eq!(out.trim(), "ababab");
}

#[test]
fn division_by_zero_is_recoverable_for_integers_and_floats() {
    // Python raises where Rust would panic on integers and hand back infinity on floats. Neither
    // native behavior is acceptable, so both must produce the same recoverable error.
    let out = run(
        "divzero",
        concat!(
            "def fdiv(a: int, b: int) -> int:\n    return a // b\n\n",
            "def m(a: int, b: int) -> int:\n    return a % b\n\n",
            "def ratio(a: int, b: int) -> float:\n    return a / b\n\n",
            "def ffdiv(a: float, b: float) -> float:\n    return a // b\n\n",
            "def fm(a: float, b: float) -> float:\n    return a % b\n",
        ),
        r#"
    println!("{:?}", fdiv(1, 0));
    println!("{:?}", m(1, 0));
    println!("{:?}", ratio(1, 0));
    println!("{:?}", ffdiv(1.0, 0.0));
    println!("{:?}", fm(1.0, 0.0));
    // The process must still be running and usable afterwards.
    println!("{}", fdiv(9, 2).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    for (index, line) in lines[..5].iter().enumerate() {
        assert!(
            line.contains("DivisionByZero"),
            "case {index} must report division by zero, got: {line}"
        );
    }
    assert_eq!(
        lines[5], "4",
        "execution continues after a recovered failure"
    );
}

#[test]
fn overflow_is_detected_rather_than_wrapped() {
    let out = run(
        "overflow",
        concat!(
            "def add(a: int, b: int) -> int:\n    return a + b\n\n",
            "def mul(a: int, b: int) -> int:\n    return a * b\n\n",
            "def sub(a: int, b: int) -> int:\n    return a - b\n\n",
            "def neg(a: int) -> int:\n    return -a\n\n",
            "def fdiv(a: int, b: int) -> int:\n    return a // b\n\n",
            "def m(a: int, b: int) -> int:\n    return a % b\n",
        ),
        r#"
    println!("{:?}", add(i64::MAX, 1));
    println!("{:?}", mul(i64::MAX, 2));
    println!("{:?}", sub(i64::MIN, 1));
    println!("{:?}", neg(i64::MIN));
    // i64::MIN / -1 is the one division whose true quotient is out of range.
    println!("{:?}", fdiv(i64::MIN, -1));
    // i64::MIN % -1 is 0, which IS representable, so Python's answer is available.
    println!("{:?}", m(i64::MIN, -1));
    println!("{:?}", add(2, 3));
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    for (index, line) in lines[..5].iter().enumerate() {
        assert!(
            line.contains("Overflow"),
            "case {index} must report overflow rather than wrapping, got: {line}"
        );
    }
    assert!(
        lines[5].contains("Ok(0)"),
        "i64::MIN % -1 is 0 in Python and representable in i64, so it must not error: {}",
        lines[5]
    );
    assert!(lines[6].contains("Ok(5)"));
}

#[test]
fn a_failure_inside_a_called_function_propagates_to_the_outermost_caller() {
    let out = run(
        "propagate",
        concat!(
            "def inner(a: int, b: int) -> int:\n    return a // b\n\n",
            "def middle(a: int, b: int) -> int:\n    return inner(a, b) + 1\n\n",
            "def outer(a: int, b: int) -> int:\n    return middle(a, b) * 2\n",
        ),
        r#"
    println!("{:?}", outer(10, 0));
    println!("{:?}", outer(10, 3));
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].contains("DivisionByZero"),
        "a failure two calls deep must reach the top, got: {}",
        lines[0]
    );
    assert_eq!(lines[1], "Ok(8)", "(10 // 3 + 1) * 2 == 8");
}

#[test]
fn compiled_results_match_the_interpreted_originals() {
    // The broadest check available at this stage: a table of operations over signed operands,
    // with the expected values taken from Python's own semantics.
    let out = run(
        "parity",
        concat!(
            "def fdiv(a: int, b: int) -> int:\n    return a // b\n\n",
            "def m(a: int, b: int) -> int:\n    return a % b\n\n",
            "def ratio(a: int, b: int) -> float:\n    return a / b\n",
        ),
        r#"
    for (a, b) in [(-7i64, 2i64), (7, -2), (-7, -2), (7, 2), (-1, 5), (1, -5)] {
        println!("{} {} {:?}", fdiv(a, b).unwrap(), m(a, b).unwrap(), ratio(a, b).unwrap());
    }
"#,
    );
    // Produced by CPython: [(a // b, a % b, a / b) for a, b in ...]
    let expected = [
        "-4 1 -3.5",
        "-4 -1 -3.5",
        "3 -1 3.5",
        "3 1 3.5",
        "-1 4 -0.2",
        "-1 -4 -0.2",
    ];
    assert_eq!(out.lines().collect::<Vec<_>>(), expected);
}
