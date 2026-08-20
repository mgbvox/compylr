//! Declared semantics, verified by running the emitted code.
//!
//! Reading emitted text cannot catch the failure that matters here. A flooring-division helper
//! that adjusts the quotient in the wrong direction still *looks* correct in a string comparison,
//! and a snapshot of it would be just as wrong as the code. So every test in this file emits
//! Rust, compiles it with `rustc`, runs the binary, and asserts on what it printed.
//!
//! That is slower than a string assertion, and it is the point: these are precisely the cases
//! where Rust's native operators disagree with what the IR declared, so the only convincing
//! evidence is a number produced by executing the result.
//!
//! Most tests start from Python source, because that is the path users take. The ones that do not
//! build IR by hand, because Python has no syntax for truncating division or a dividend-signed
//! remainder — and a mode no test can reach is a mode the backend can get wrong indefinitely.

use std::path::PathBuf;
use std::process::Command;

use compylr_frontend_python::frontend::parse_source;
use compylr_frontend_python::lower::lower_source_members;
use compylr_ir::Unit;
use compylr_registry::backends::lookup;

fn unit_from(source: &str) -> Unit {
    let parsed = parse_source(source).expect("fixture must parse");
    let (functions, classes) = lower_source_members(&parsed)
        .unwrap_or_else(|e| panic!("should lower: {}", e.render(source)));
    let mut unit = Unit::new();
    for function in functions {
        unit.add_function(function).unwrap();
    }
    for class in classes {
        unit.add_class(class).unwrap();
    }
    unit.validate().expect("calls must resolve");
    unit
}

/// Emit `source`, append `main_body`, compile, run, and return stdout.
///
/// `label` only has to be unique across tests so parallel runs do not fight over a path.
fn run(label: &str, source: &str, main_body: &str) -> String {
    run_unit(label, &unit_from(source), main_body)
}

/// The same, starting from a unit rather than from source.
///
/// Separate so that a mode no source language in this repo can produce still gets executed.
fn run_unit(label: &str, unit: &Unit, main_body: &str) -> String {
    let emitted = lookup("rust").unwrap().emit(unit).expect("must emit");

    // The crate is written out and a `main.rs` added beside it, so the code under test is
    // compiled exactly as it ships rather than concatenated into a shape it never takes.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(label);
    let _ = std::fs::remove_dir_all(&dir);
    for (relative, contents) in &emitted {
        let path = dir.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).expect("scratch directory");
        std::fs::write(&path, contents).expect("write generated source");
    }
    let source_path = dir.join("src/main.rs");
    let binary_path = dir.join("program");

    let program = format!(
        "#![allow(unused_parens, non_snake_case, unused_variables, dead_code, unused_imports)]\n\
         mod compat;\n\
         mod generated;\n\
         use generated::*;\n\
         fn main() {{\n{main_body}\n}}\n"
    );
    std::fs::write(&source_path, &program).expect("write the harness");

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
        "generated Rust did not compile:\n{}\n--- translated ---\n{}",
        String::from_utf8_lossy(&compile.stderr),
        emitted["src/generated.rs"]
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

/// Collection semantics, executed rather than inspected.
///
/// Two of these are the same class of trap as `//` and `%`: Python indexes from the end for a
/// negative index and counts a string's *characters*, where Rust does neither. Both are correct
/// for the easy cases and silently wrong for the ones a test written in English would miss.
mod collections {
    use super::*;

    #[test]
    fn a_negative_index_counts_from_the_end() {
        let out = run(
            "neg_index",
            concat!(
                "def last(xs: list[int]) -> int:\n    return xs[-1]\n\n",
                "def first(xs: list[int]) -> int:\n    return xs[-3]\n\n",
                "def front(xs: list[int]) -> int:\n    return xs[0]\n",
            ),
            r#"
    let xs = vec![10i64, 20, 30];
    println!("{}", last(xs.clone()).unwrap());
    println!("{}", first(xs.clone()).unwrap());
    println!("{}", front(xs.clone()).unwrap());
"#,
        );
        assert_eq!(out.lines().collect::<Vec<_>>(), ["30", "10", "10"]);
    }

    #[test]
    fn an_index_past_either_end_is_recoverable() {
        let out = run(
            "index_range",
            "def at(xs: list[int], i: int) -> int:\n    return xs[i]\n",
            r#"
    let xs = vec![1i64, 2, 3];
    println!("{:?}", at(xs.clone(), 5).is_err());
    println!("{:?}", at(xs.clone(), -5).is_err());
    println!("{}", at(xs.clone(), 1).unwrap());
"#,
        );
        assert_eq!(out.lines().collect::<Vec<_>>(), ["true", "true", "2"]);
    }

    #[test]
    fn a_missing_key_is_recoverable_and_names_the_key() {
        let out = run(
            "missing_key",
            "def get(d: dict[str, int], k: str) -> int:\n    return d[k]\n",
            r#"
    let mut d = std::collections::HashMap::new();
    d.insert(String::from("a"), 1i64);
    println!("{}", get(d.clone(), String::from("a")).unwrap());
    match get(d.clone(), String::from("zzz")) {
        Err(e) => println!("{e}"),
        Ok(v) => println!("unexpected {v}"),
    }
"#,
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "1");
        assert!(
            lines[1].contains("zzz"),
            "the key must be named: {}",
            lines[1]
        );
    }

    #[test]
    fn len_counts_characters_not_bytes() {
        // The case that catches a byte count. `len("é")` is 1 in Python and 2 in Rust.
        let out = run(
            "len_chars",
            concat!(
                "def size(s: str) -> int:\n    return len(s)\n\n",
                "def items(xs: list[int]) -> int:\n    return len(xs)\n",
            ),
            r#"
    println!("{}", size(String::from("abc")).unwrap());
    println!("{}", size(String::from("é")).unwrap());
    println!("{}", size(String::from("héllo")).unwrap());
    println!("{}", items(vec![1i64, 2, 3]).unwrap());
"#,
        );
        assert_eq!(out.lines().collect::<Vec<_>>(), ["3", "1", "5", "3"]);
    }

    #[test]
    fn literals_construct_what_python_would() {
        let out = run(
            "literals",
            concat!(
                "def list_lit() -> list[int]:\n    return [1, 2, 3]\n\n",
                "def set_lit() -> set[int]:\n    return {1, 2, 2}\n\n",
                "def dict_lit() -> dict[str, int]:\n    return {\"a\": 1, \"b\": 2}\n\n",
                "def tuple_lit() -> tuple[int, str]:\n    return (1, \"a\")\n",
            ),
            r#"
    println!("{:?}", list_lit().unwrap());
    println!("{}", set_lit().unwrap().len());
    println!("{}", dict_lit().unwrap()["a"]);
    let (n, s) = tuple_lit().unwrap();
    println!("{n} {s}");
"#,
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "[1, 2, 3]", "sequence order is preserved");
        assert_eq!(
            lines[1], "2",
            "a set literal de-duplicates, as Python's does"
        );
        assert_eq!(lines[2], "1");
        assert_eq!(lines[3], "1 a");
    }

    #[test]
    fn a_collection_read_twice_is_not_moved() {
        // Python has no notion of a value being consumed by being read. If emission moved the
        // collection, this would fail to compile rather than fail an assertion.
        let out = run(
            "no_move",
            "def both(xs: list[int]) -> int:\n    a = xs[0]\n    b = len(xs)\n    return a + b\n",
            r#"
    println!("{}", both(vec![7i64, 8, 9]).unwrap());
"#,
        );
        assert_eq!(out.trim(), "10");
    }

    #[test]
    fn nested_collections_work() {
        let out = run(
            "nested",
            "def inner(d: dict[str, list[int]], k: str) -> int:\n    xs = d[k]\n    return xs[0]\n",
            r#"
    let mut d = std::collections::HashMap::new();
    d.insert(String::from("k"), vec![42i64, 43]);
    println!("{}", inner(d, String::from("k")).unwrap());
"#,
        );
        assert_eq!(out.trim(), "42");
    }
}

// ---------------------------------------------------------------------------
// Control flow
//
// Branches and loops are where a wrong answer is quietest: a loop that runs one
// iteration too few still produces a number, and a range with a negative step
// that never runs produces the zero the caller might have expected anyway. So
// these run the code and check the values.
// ---------------------------------------------------------------------------

#[test]
fn a_conditional_runs_the_matching_branch() {
    let out = run(
        "cf_branches",
        concat!(
            "def sign(n: int) -> int:\n",
            "    if n > 0:\n        return 1\n    elif n < 0:\n        return -1\n    else:\n        return 0\n\n",
            "def bump(n: int) -> int:\n",
            "    label = n\n    if n > 10:\n        label = 100\n    return label\n\n",
            "def nested(a: int, b: int) -> int:\n",
            "    if a > 0:\n        if b > 0:\n            return 3\n        return 2\n    return 1\n",
        ),
        r#"
    println!("{}", sign(5).unwrap());
    println!("{}", sign(-5).unwrap());
    println!("{}", sign(0).unwrap());
    println!("{}", bump(3).unwrap());
    println!("{}", bump(20).unwrap());
    println!("{}", nested(1, 1).unwrap());
    println!("{}", nested(1, -1).unwrap());
    println!("{}", nested(-1, 1).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0..3],
        ["1", "-1", "0"],
        "each branch of an if/elif/else"
    );
    assert_eq!(lines[3], "3", "an if with no else continues past it");
    assert_eq!(lines[4], "100", "and takes the branch when the test holds");
    assert_eq!(lines[5..8], ["3", "2", "1"], "nesting reaches each depth");
}

#[test]
fn a_while_loop_counts_and_loop_control_behaves() {
    let out = run(
        "cf_while",
        concat!(
            "def count_to(n: int) -> int:\n",
            "    i = 0\n    while i < n:\n        i = i + 1\n    return i\n\n",
            "def stop_at_five(n: int) -> int:\n",
            "    i = 0\n    while i < n:\n        if i == 5:\n            break\n        i = i + 1\n    return i\n\n",
            "def count_odds(n: int) -> int:\n",
            "    i = 0\n    odds = 0\n    while i < n:\n        i = i + 1\n        if i % 2 == 0:\n            continue\n        odds = odds + 1\n    return odds\n",
        ),
        r#"
    println!("{}", count_to(4).unwrap());
    println!("{}", count_to(0).unwrap());
    println!("{}", count_to(-3).unwrap());
    println!("{}", stop_at_five(100).unwrap());
    println!("{}", count_odds(10).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "4");
    assert_eq!(
        lines[1], "0",
        "a test false at entry runs the body zero times"
    );
    assert_eq!(lines[2], "0", "and stays zero when it is false by a margin");
    assert_eq!(lines[3], "5", "break leaves at the iteration it fires on");
    assert_eq!(
        lines[4], "5",
        "continue skips the rest of one iteration only"
    );
}

#[test]
fn ranges_count_the_way_python_counts() {
    // `range(3, 0, -1)` is the case Rust's `..` cannot express: it counts up by one, `step_by`
    // takes an unsigned step, and `rev()` does not compose with a computed step.
    let out = run(
        "cf_ranges",
        concat!(
            "def one(n: int) -> int:\n",
            "    acc = 0\n    for i in range(n):\n        acc = acc * 10 + i\n    return acc\n\n",
            "def two(a: int, b: int) -> int:\n",
            "    acc = 0\n    for i in range(a, b):\n        acc = acc * 10 + i\n    return acc\n\n",
            "def three(a: int, b: int, c: int) -> int:\n",
            "    acc = 0\n    for i in range(a, b, c):\n        acc = acc * 10 + i\n    return acc\n\n",
            "def counted(a: int, b: int, c: int) -> int:\n",
            "    seen = 0\n    for i in range(a, b, c):\n        seen = seen + 1\n    return seen\n",
        ),
        r#"
    println!("{}", one(3).unwrap());
    println!("{}", two(2, 5).unwrap());
    println!("{}", three(0, 6, 2).unwrap());
    println!("{}", three(3, 0, -1).unwrap());
    println!("{}", counted(0, 0, 1).unwrap());
    println!("{}", counted(5, 0, 1).unwrap());
    println!("{}", counted(0, 5, -1).unwrap());
    println!("{}", one(-4).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    // Each digit is one value, in order: the accumulator is order-sensitive on purpose.
    assert_eq!(lines[0], "12", "range(3) yields 0, 1, 2");
    assert_eq!(lines[1], "234", "range(2, 5) yields 2, 3, 4");
    assert_eq!(lines[2], "24", "range(0, 6, 2) yields 0, 2, 4");
    assert_eq!(lines[3], "321", "range(3, 0, -1) counts down: 3, 2, 1");
    assert_eq!(lines[4], "0", "an empty range does not run its body");
    assert_eq!(
        lines[5], "0",
        "nor does one whose start is already past its stop"
    );
    assert_eq!(lines[6], "0", "nor one stepping away from its stop");
    assert_eq!(lines[7], "0", "range of a negative count is empty");
}

#[test]
fn a_zero_step_fails_rather_than_hanging() {
    // The one failure worse than a wrong answer: with a zero step the condition never changes, so
    // without this check the program produces nothing at all to diagnose from.
    let out = run(
        "cf_zero_step",
        concat!(
            "def walk(a: int, b: int, c: int) -> int:\n",
            "    seen = 0\n    for i in range(a, b, c):\n        seen = seen + 1\n    return seen\n",
        ),
        r#"
    println!("{:?}", walk(0, 10, 0).is_err());
    println!("{}", walk(0, 10, 1).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "true", "a zero step is a recoverable error");
    assert_eq!(lines[1], "10", "and a valid step still works afterwards");
}

#[test]
fn iterating_a_collection_preserves_order_and_leaves_it_readable() {
    let out = run(
        "cf_iteration",
        concat!(
            "def digits(xs: list[int]) -> int:\n",
            "    acc = 0\n    for x in xs:\n        acc = acc * 10 + x\n    return acc\n\n",
            "def twice(xs: list[int]) -> int:\n",
            "    total = 0\n    for x in xs:\n        total = total + x\n",
            "    for x in xs:\n        total = total + x\n    return total + len(xs)\n\n",
            // Summing key lengths rather than concatenating: a mapping's order is not defined, so
            // asserting on it would make this suite itself flaky.
            "def key_chars(d: dict[str, int]) -> int:\n",
            "    n = 0\n    for k in d:\n        n = n + len(k)\n    return n\n\n",
            "def set_total(s: set[int]) -> int:\n",
            "    n = 0\n    for v in s:\n        n = n + v\n    return n\n",
        ),
        r#"
    println!("{}", digits(vec![1, 2, 3]).unwrap());
    println!("{}", twice(vec![1, 2, 3]).unwrap());
    let mut d = std::collections::HashMap::new();
    d.insert(String::from("ab"), 1i64);
    d.insert(String::from("cde"), 2i64);
    println!("{}", key_chars(d).unwrap());
    println!("{}", set_total(std::collections::HashSet::from([1i64, 2, 3])).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "123", "a sequence is iterated in order");
    assert_eq!(
        lines[1], "15",
        "iterating twice sees every element both times"
    );
    assert_eq!(lines[2], "5", "a mapping yields its keys, as Python does");
    assert_eq!(lines[3], "6", "a set yields its elements");
}

#[test]
fn only_reassigned_bindings_are_marked_mutable() {
    // A spurious `mut` is a warning in code that must compile clean, and the mutability scan is a
    // second traversal that could disagree with emission.
    let unit = unit_from(
        "def f(fixed: int, moved: int) -> int:\n\
         \x20   stable = 1\n\
         \x20   counter = 0\n\
         \x20   counter = counter + fixed\n\
         \x20   moved = moved + 1\n\
         \x20   return stable + counter + moved\n",
    );
    let emitted = lookup("rust").unwrap().emit(&unit).expect("must emit");
    let source = &emitted["src/generated.rs"];
    assert!(
        source.contains("let stable: i64"),
        "a once-bound local is not mutable:\n{source}"
    );
    assert!(
        source.contains("let mut counter: i64"),
        "a reassigned local is mutable:\n{source}"
    );
    assert!(
        source.contains("mut moved: i64"),
        "a reassigned parameter is mutable:\n{source}"
    );
    assert!(
        !source.contains("mut fixed"),
        "an untouched parameter is not mutable:\n{source}"
    );
}

// ---------------------------------------------------------------------------
// Mutation and membership
//
// These assert on **values after mutation**, never on emitted text. The failure
// mode that matters here is a mutation applied to a clone: it compiles, it runs,
// and it silently does nothing. Emitted text would look right.
// ---------------------------------------------------------------------------

#[test]
fn appending_in_a_loop_accumulates() {
    let out = run(
        "mut_append",
        concat!(
            "def evens(n: int) -> list[int]:\n",
            "    found: list[int] = []\n",
            "    for i in range(n):\n",
            "        if i % 2 == 0:\n",
            "            found.append(i)\n",
            "    return found\n\n",
            "def counted(n: int) -> int:\n",
            "    found: list[int] = []\n",
            "    for i in range(n):\n",
            "        found.append(i)\n",
            "    return len(found)\n",
        ),
        r#"
    println!("{:?}", evens(7).unwrap());
    println!("{:?}", evens(0).unwrap());
    println!("{}", counted(5).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    // If the append landed on a clone, this would be `[]` and nothing would report an error.
    assert_eq!(
        lines[0], "[0, 2, 4, 6]",
        "each append must survive the loop"
    );
    assert_eq!(lines[1], "[]", "nothing to append leaves it empty");
    assert_eq!(lines[2], "5", "mutation and reading compose");
}

#[test]
fn an_element_assignment_is_observed_by_a_later_read() {
    let out = run(
        "mut_setitem",
        concat!(
            "def replaced(n: int) -> int:\n",
            "    xs: list[int] = [1, 2, 3]\n",
            "    xs[1] = n\n",
            "    return xs[1]\n\n",
            "def from_the_end(n: int) -> list[int]:\n",
            "    xs: list[int] = [1, 2, 3]\n",
            "    xs[-1] = n\n",
            "    return xs\n\n",
            "def out_of_range(n: int) -> int:\n",
            "    xs: list[int] = [1]\n",
            "    xs[n] = 0\n",
            "    return xs[0]\n",
        ),
        r#"
    println!("{}", replaced(9).unwrap());
    println!("{:?}", from_the_end(9).unwrap());
    println!("{}", out_of_range(5).is_err());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "9", "the write is visible to a later read");
    assert_eq!(
        lines[1], "[1, 2, 9]",
        "a negative index counts from the end"
    );
    assert_eq!(
        lines[2], "true",
        "a sequence has no element to create, so out of range still fails"
    );
}

#[test]
fn assigning_a_mapping_key_creates_or_replaces_it() {
    // Reading a missing key is a KeyError; assigning to one is not. Routing assignment through the
    // checked read would make every insertion of a new key fail.
    let out = run(
        "mut_insert",
        concat!(
            "def inserted(k: str, v: int) -> int:\n",
            "    d: dict[str, int] = {}\n",
            "    d[k] = v\n",
            "    return d[k]\n\n",
            "def replaced(v: int) -> int:\n",
            "    d: dict[str, int] = {}\n",
            "    d[\"a\"] = 1\n",
            "    d[\"a\"] = v\n",
            "    return d[\"a\"]\n\n",
            "def sized(k: str) -> int:\n",
            "    d: dict[str, int] = {}\n",
            "    d[k] = 1\n",
            "    d[k] = 2\n",
            "    return len(d)\n\n",
            "def missing(k: str) -> int:\n",
            "    d: dict[str, int] = {}\n",
            "    d[\"a\"] = 1\n",
            "    return d[k]\n",
        ),
        r#"
    println!("{}", inserted(String::from("k"), 7).unwrap());
    println!("{}", replaced(5).unwrap());
    println!("{}", sized(String::from("k")).unwrap());
    println!("{}", missing(String::from("absent")).is_err());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "7", "assigning a new key creates it");
    assert_eq!(lines[1], "5", "assigning an existing key replaces it");
    assert_eq!(lines[2], "1", "replacing does not add a second entry");
    assert_eq!(lines[3], "true", "reading a missing key still fails");
}

#[test]
fn membership_means_what_each_container_means_by_it() {
    let out = run(
        "mut_contains",
        concat!(
            "def in_list(xs: list[int], x: int) -> bool:\n    return x in xs\n\n",
            "def in_set(s: set[int], x: int) -> bool:\n    return x in s\n\n",
            // A mapping tests keys. A naive containment check over the values would answer the
            // opposite for both of the cases below.
            "def in_map(d: dict[str, int], k: str) -> bool:\n    return k in d\n\n",
            // A string tests substrings, not characters -- `\"ab\" in \"cab\"` is true in Python.
            "def in_str(hay: str, needle: str) -> bool:\n    return needle in hay\n\n",
            "def not_in_list(xs: list[int], x: int) -> bool:\n    return x not in xs\n\n",
            "def tested_then_measured(xs: list[int], x: int) -> int:\n",
            "    if x in xs:\n        return len(xs)\n",
            "    return 0 - len(xs)\n",
        ),
        r#"
    println!("{}", in_list(vec![1, 2, 3], 2).unwrap());
    println!("{}", in_list(vec![1, 2, 3], 9).unwrap());
    println!("{}", in_set(std::collections::HashSet::from([1i64, 2]), 2).unwrap());
    let mut d = std::collections::HashMap::new();
    d.insert(String::from("a"), 7i64);
    println!("{}", in_map(d.clone(), String::from("a")).unwrap());
    println!("{}", in_map(d, String::from("7")).unwrap());
    println!("{}", in_str(String::from("cab"), String::from("ab")).unwrap());
    println!("{}", in_str(String::from("cab"), String::from("ba")).unwrap());
    println!("{}", not_in_list(vec![1, 2], 9).unwrap());
    println!("{}", not_in_list(vec![1, 2], 1).unwrap());
    println!("{}", tested_then_measured(vec![1, 2, 3], 2).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0..2], ["true", "false"], "sequence membership");
    assert_eq!(lines[2], "true", "set membership");
    assert_eq!(lines[3], "true", "a mapping tests its keys");
    assert_eq!(
        lines[4], "false",
        "and not its values -- 7 is a value, not a key"
    );
    assert_eq!(
        lines[5], "true",
        "a string tests substrings: 'ab' is in 'cab'"
    );
    assert_eq!(lines[6], "false", "and 'ba' is not");
    assert_eq!(lines[7..9], ["true", "false"], "`not in` is the negation");
    assert_eq!(
        lines[9], "3",
        "membership does not consume the container, which is still measurable"
    );
}

#[test]
fn only_mutated_collections_are_bound_mutably() {
    // The one text assertion in this group, and it is about `mut` rather than about mutation
    // working: a spurious `mut` is a warning in code that must compile clean, and a missing one
    // fails to compile.
    let unit = unit_from(
        "def f() -> int:\n\
         \x20   read_only: list[int] = [1, 2]\n\
         \x20   appended: list[int] = []\n\
         \x20   written: list[int] = [0]\n\
         \x20   appended.append(1)\n\
         \x20   written[0] = 1\n\
         \x20   return read_only[0] + len(appended) + written[0]\n",
    );
    let emitted = lookup("rust").unwrap().emit(&unit).expect("must emit");
    let source = &emitted["src/generated.rs"];
    assert!(
        source.contains("let read_only: Vec<i64>"),
        "an unmutated collection is not mutable:\n{source}"
    );
    assert!(
        source.contains("let mut appended: Vec<i64>"),
        "an appended-to collection is mutable:\n{source}"
    );
    assert!(
        source.contains("let mut written: Vec<i64>"),
        "an assigned-into collection is mutable:\n{source}"
    );
}

// ---------------------------------------------------------------------------
// Classes
//
// The property these exist for is that state survives a call. A struct that
// compiled but whose methods took a copy of the receiver would pass every
// type check and lose every mutation.
// ---------------------------------------------------------------------------

#[test]
fn instance_state_survives_between_calls() {
    let out = run(
        "cls_state",
        concat!(
            "class Counter:\n",
            "    def __init__(self, start: int) -> None:\n",
            "        self.count: int = start\n",
            "\n",
            "    def bump(self, by: int) -> None:\n",
            "        self.count = self.count + by\n",
            "\n",
            "    def get(self) -> int:\n",
            "        return self.count\n",
        ),
        r#"
    let mut c = Counter::__compylr_new(10).unwrap();
    println!("{}", c.get().unwrap());
    c.bump(5).unwrap();
    c.bump(5).unwrap();
    println!("{}", c.get().unwrap());
    let other = Counter::__compylr_new(0).unwrap();
    println!("{}", other.get().unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "10", "construction initialises every field");
    assert_eq!(lines[1], "20", "a mutation is observed by a later call");
    assert_eq!(lines[2], "0", "two instances are independent");
}

#[test]
fn a_method_calling_a_mutating_method_takes_a_mutable_receiver() {
    // The transitive case, and the likeliest bug: a method whose body is only `self.bump()`
    // mutates through the call, and a shared receiver there produces a borrow-checker error about
    // generated code rather than a diagnostic about the user's program.
    let out = run(
        "cls_transitive",
        concat!(
            "class Counter:\n",
            "    def __init__(self) -> None:\n",
            "        self.count: int = 0\n",
            "\n",
            "    def bump(self) -> None:\n",
            "        self.count = self.count + 1\n",
            "\n",
            "    def bump_twice(self) -> None:\n",
            "        self.bump()\n",
            "        self.bump()\n",
            "\n",
            "    def bump_four_times(self) -> None:\n",
            "        self.bump_twice()\n",
            "        self.bump_twice()\n",
            "\n",
            "    def get(self) -> int:\n",
            "        return self.count\n",
        ),
        r#"
    let mut c = Counter::__compylr_new().unwrap();
    c.bump_four_times().unwrap();
    println!("{}", c.get().unwrap());
"#,
    );
    assert_eq!(
        out.trim(),
        "4",
        "mutation must reach through two levels of method call"
    );
}

#[test]
fn a_collection_attribute_is_a_cache() {
    // The shape the memoized demo needs: membership, read, and insert over an attribute that
    // outlives the call. A collection *parameter* could not do this, because it is a copy.
    let out = run(
        "cls_cache",
        concat!(
            "class Cache:\n",
            "    def __init__(self) -> None:\n",
            "        self.entries: dict[int, int] = {}\n",
            "        self.hits: int = 0\n",
            "\n",
            "    def square(self, n: int) -> int:\n",
            "        if n in self.entries:\n",
            "            self.hits = self.hits + 1\n",
            "            return self.entries[n]\n",
            "        computed = n * n\n",
            "        self.entries[n] = computed\n",
            "        return computed\n",
            "\n",
            "    def hit_count(self) -> int:\n",
            "        return self.hits\n",
            "\n",
            "    def size(self) -> int:\n",
            "        return len(self.entries)\n",
        ),
        r#"
    let mut c = Cache::__compylr_new().unwrap();
    println!("{}", c.square(4).unwrap());
    println!("{}", c.square(4).unwrap());
    println!("{}", c.hit_count().unwrap());
    println!("{}", c.square(5).unwrap());
    println!("{}", c.size().unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "16", "a miss computes");
    assert_eq!(lines[1], "16", "a hit returns the stored value");
    assert_eq!(
        lines[2], "1",
        "and the hit was counted, so the cache was consulted"
    );
    assert_eq!(lines[3], "25");
    assert_eq!(
        lines[4], "2",
        "reading a collection attribute does not move it"
    );
}

#[test]
fn a_reading_method_takes_a_shared_receiver() {
    // Two methods must be usable on one object. `&mut self` everywhere would make that a borrow
    // error about the compiler's output rather than about the user's program.
    let unit = unit_from(
        "class C:\n\
         \x20   def __init__(self) -> None:\n\
         \x20       self.x: int = 0\n\
         \n\
         \x20   def get(self) -> int:\n\
         \x20       return self.x\n\
         \n\
         \x20   def set(self, n: int) -> None:\n\
         \x20       self.x = n\n",
    );
    let emitted = lookup("rust").unwrap().emit(&unit).expect("must emit");
    let source = &emitted["src/generated.rs"];
    assert!(
        source.contains("fn get(&self)"),
        "a reading method takes a shared receiver:\n{source}"
    );
    assert!(
        source.contains("fn set(&mut self"),
        "a mutating method takes a mutable receiver:\n{source}"
    );
}

/// Folded programs must still run, and still produce what the unfolded ones did.
///
/// Folding rewrites the tree the backend receives, so it can produce a literal the backend has
/// never had to emit before — a float that arrived as a division, say. Checking the pass in
/// isolation cannot catch that; only compiling and running the result can.
mod folding {
    use super::*;
    use compylr_core::pass::{self, Optimization, PassConfig};

    fn compiled(label: &str, source: &str, main_body: &str, optimization: Optimization) -> String {
        let mut unit = unit_from(source);
        pass::run(&mut unit, &PassConfig { optimization }, &[])
            .expect("passes must not fail on an accepted program");
        run_unit(label, &unit, main_body)
    }

    /// The same program, with and without the pass, must print the same thing.
    #[test]
    fn folding_does_not_change_what_a_program_computes() {
        // Every case is one where a wrong fold gives a different answer: flooring on mixed signs,
        // divisor-signed remainder, and a division that promotes.
        let source = "def quotient() -> int:\n    return 7 // -2\n\n\
                      def remainder() -> int:\n    return -7 % 2\n\n\
                      def ratio() -> float:\n    return 7 / 2\n\n\
                      def joined() -> str:\n    return \"a\" + \"b\"\n";
        let main = "    println!(\"{} {} {} {}\", quotient().unwrap(), remainder().unwrap(), \
                    ratio().unwrap(), joined().unwrap());";

        let folded = compiled("folding_on", source, main, Optimization::Default);
        let plain = compiled("folding_off", source, main, Optimization::None);

        assert_eq!(folded.trim(), "-4 1 3.5 ab");
        assert_eq!(folded, plain, "folding must not change the result");
    }

    /// A failure the program would have reported must survive the pass.
    #[test]
    fn a_folded_program_still_reports_the_failures_it_would_have() {
        let source = "def by_zero() -> int:\n    return 1 // 0\n";
        let out = compiled(
            "folding_keeps_errors",
            source,
            "    println!(\"{:?}\", by_zero().is_err());",
            Optimization::Default,
        );
        assert_eq!(
            out.trim(),
            "true",
            "the division must still fail at runtime"
        );
    }
}

/// The modes the Python frontend never produces, executed anyway.
///
/// Truncating division and a dividend-signed remainder are what C, Go, Rust, and Java mean by `/`
/// and `%`. No Python program can reach them, so without hand-built IR the backend could emit
/// flooring for both modes and every test in this repo would still pass — which is precisely the
/// bug the change is meant to make impossible.
mod modes_python_cannot_write {
    use super::*;
    use compylr_diagnostics::span::Span;
    use compylr_ir::{BinOp, DivMode, Expr, Function, Param, RemSign, Rounding, Stmt, Ty};

    /// A unit holding one function `op(a, b) -> int` applying `op` to its two parameters.
    fn binary_unit(op: BinOp) -> Unit {
        let mut unit = Unit::new();
        unit.add_function(Function {
            name: "op".to_string(),
            params: vec![
                Param {
                    name: "a".to_string(),
                    ty: Ty::Int,
                },
                Param {
                    name: "b".to_string(),
                    ty: Ty::Int,
                },
            ],
            ret: Ty::Int,
            body: vec![Stmt::Return(Expr::Binary {
                op,
                left: Box::new(Expr::name("a")),
                right: Box::new(Expr::name("b")),
            })],
            doc: None,
            span: Span::default(),
        })
        .unwrap();
        unit
    }

    #[test]
    fn division_rounding_toward_zero_truncates() {
        let unit = binary_unit(BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
        });
        let out = run_unit(
            "mode_div_trunc",
            &unit,
            "    println!(\"{}\", op(-7, 2).unwrap());\n\
             \x20   println!(\"{}\", op(7, -2).unwrap());\n\
             \x20   println!(\"{}\", op(-6, 2).unwrap());",
        );
        // Truncation, not flooring: -3 rather than -4 on the first two.
        assert_eq!(out.lines().collect::<Vec<_>>(), ["-3", "-3", "-3"]);
    }

    #[test]
    fn division_rounding_toward_negative_infinity_floors() {
        let unit = binary_unit(BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardNegInf),
        });
        let out = run_unit(
            "mode_div_floor",
            &unit,
            "    println!(\"{}\", op(-7, 2).unwrap());\n\
             \x20   println!(\"{}\", op(7, -2).unwrap());\n\
             \x20   println!(\"{}\", op(-6, 2).unwrap());",
        );
        assert_eq!(out.lines().collect::<Vec<_>>(), ["-4", "-4", "-3"]);
    }

    #[test]
    fn remainder_taking_the_sign_of_the_dividend() {
        let unit = binary_unit(BinOp::Rem {
            sign: RemSign::Dividend,
        });
        let out = run_unit(
            "mode_rem_trunc",
            &unit,
            "    println!(\"{}\", op(-7, 2).unwrap());\n\
             \x20   println!(\"{}\", op(7, -2).unwrap());",
        );
        // The sign follows the dividend, so these are the mirror image of Python's.
        assert_eq!(out.lines().collect::<Vec<_>>(), ["-1", "1"]);
    }

    #[test]
    fn remainder_taking_the_sign_of_the_divisor() {
        let unit = binary_unit(BinOp::Rem {
            sign: RemSign::Divisor,
        });
        let out = run_unit(
            "mode_rem_floor",
            &unit,
            "    println!(\"{}\", op(-7, 2).unwrap());\n\
             \x20   println!(\"{}\", op(7, -2).unwrap());",
        );
        assert_eq!(out.lines().collect::<Vec<_>>(), ["1", "-1"]);
    }

    /// A sequence read, under each declared origin, executed.
    #[test]
    fn indexing_from_the_start_refuses_a_negative_index() {
        use compylr_ir::IndexOrigin;

        for (label, origin, expected) in [
            ("mode_index_either", IndexOrigin::FromEitherEnd, "ok 30"),
            ("mode_index_start", IndexOrigin::FromStart, "out of range"),
        ] {
            let mut unit = Unit::new();
            unit.add_function(Function {
                name: "read".to_string(),
                params: vec![
                    Param {
                        name: "xs".to_string(),
                        ty: Ty::List(Box::new(Ty::Int)),
                    },
                    Param {
                        name: "i".to_string(),
                        ty: Ty::Int,
                    },
                ],
                ret: Ty::Int,
                body: vec![Stmt::Return(Expr::Subscript {
                    base: Box::new(Expr::name("xs")),
                    index: Box::new(Expr::name("i")),
                    origin,
                })],
                doc: None,
                span: Span::default(),
            })
            .unwrap();

            let out = run_unit(
                label,
                &unit,
                "    let xs = vec![10i64, 20, 30];\n\
                 \x20   match read(xs, -1) {\n\
                 \x20       Ok(value) => println!(\"ok {value}\"),\n\
                 \x20       Err(_) => println!(\"out of range\"),\n\
                 \x20   }",
            );
            assert_eq!(out.trim(), expected, "{label}");
        }
    }

    /// A length, under each declared reading, executed.
    ///
    /// The same string measured three ways gives three answers. A backend that ignored the units
    /// would return one of them for all three and pass every Python-driven test in this repo,
    /// because Python only ever declares code points.
    #[test]
    fn each_text_unit_reading_measures_differently() {
        use compylr_ir::TextUnits;

        let mut unit = Unit::new();
        for (name, units) in [
            ("code_points", TextUnits::CodePoints),
            ("utf8_bytes", TextUnits::Utf8Bytes),
            ("utf16_units", TextUnits::Utf16Units),
        ] {
            unit.add_function(Function {
                name: name.to_string(),
                params: vec![Param {
                    name: "s".to_string(),
                    ty: Ty::Str,
                }],
                ret: Ty::Int,
                body: vec![Stmt::Return(Expr::Len {
                    value: Box::new(Expr::name("s")),
                    units,
                })],
                doc: None,
                span: Span::default(),
            })
            .unwrap();
        }

        let out = run_unit(
            "mode_text_units",
            &unit,
            "    let s = \"\u{1f980}\".to_string();\n\
             \x20   println!(\n\
             \x20       \"{} {} {}\",\n\
             \x20       code_points(s.clone()).unwrap(),\n\
             \x20       utf8_bytes(s.clone()).unwrap(),\n\
             \x20       utf16_units(s).unwrap(),\n\
             \x20   );",
        );
        // One character outside the basic plane is the only input that separates all three.
        assert_eq!(out.trim(), "1 4 2");
    }

    /// Each pair must satisfy `(a / b) * b + (a % b) == a`; mixing halves must not.
    ///
    /// This is the property that makes the pairing real rather than a naming convention. A
    /// backend that emitted flooring division beside a dividend-signed remainder would pass every
    /// single-operation test above and still compute nonsense.
    #[test]
    fn each_division_and_remainder_pair_reconstructs_the_dividend() {
        for (label, rounding, sign) in [
            ("pair_floor", Rounding::TowardNegInf, RemSign::Divisor),
            ("pair_trunc", Rounding::TowardZero, RemSign::Dividend),
        ] {
            let mut unit = Unit::new();
            for (name, op) in [
                (
                    "quotient",
                    BinOp::Div {
                        mode: DivMode::Integer(rounding),
                    },
                ),
                ("remainder", BinOp::Rem { sign }),
            ] {
                unit.add_function(Function {
                    name: name.to_string(),
                    params: vec![
                        Param {
                            name: "a".to_string(),
                            ty: Ty::Int,
                        },
                        Param {
                            name: "b".to_string(),
                            ty: Ty::Int,
                        },
                    ],
                    ret: Ty::Int,
                    body: vec![Stmt::Return(Expr::Binary {
                        op,
                        left: Box::new(Expr::name("a")),
                        right: Box::new(Expr::name("b")),
                    })],
                    doc: None,
                    span: Span::default(),
                })
                .unwrap();
            }

            let out = run_unit(
                label,
                &unit,
                "    for a in [-7i64, -6, 7, 6, -1, 1] {\n\
                 \x20       for b in [2i64, -2, 3, -3] {\n\
                 \x20           let q = quotient(a, b).unwrap();\n\
                 \x20           let r = remainder(a, b).unwrap();\n\
                 \x20           assert_eq!(q * b + r, a, \"a={a} b={b} q={q} r={r}\");\n\
                 \x20       }\n\
                 \x20   }\n\
                 \x20   println!(\"consistent\");",
            );
            assert_eq!(out.trim(), "consistent", "{label}");
        }
    }
}

/// Programs the conformance corpus found the backend rendering wrongly.
///
/// Every case here is ordinary Python that produced generated Rust which did not compile, or in
/// one case a program that ran forever. None was reachable from `python/fixtures/accepted/`,
/// because a fixture only covers a form *somewhere* and every one of these is about a form's
/// behaviour in a particular **position**. They are executed rather than emitted, because for the
/// loop case reading the text is what missed it in the first place.
mod positions_the_backend_rendered_wrongly {
    use super::*;

    /// `continue` inside `for i in range(...)` used to skip the cursor increment.
    ///
    /// Not a wrong answer — a hang. The increment was emitted after the body, and `continue` jumps
    /// straight to the loop condition, so the cursor stayed where it was and the same iteration
    /// repeated forever.
    #[test]
    fn continue_in_a_range_loop_still_advances() {
        let out = run(
            "position_continue_range",
            "def count_odd(n: int) -> int:\n\
             \x20   total = 0\n\
             \x20   for i in range(n):\n\
             \x20       if i % 2 == 0:\n\
             \x20           continue\n\
             \x20       total = total + 1\n\
             \x20   return total\n",
            "    println!(\"{}\", count_odd(10).unwrap());",
        );
        assert_eq!(out.trim(), "5");
    }

    /// An attribute assigned inside an `if` in `__init__` used to emit `(self).count`.
    ///
    /// The instance does not exist inside its own constructor, so that never compiled. Only
    /// top-level attribute assignments were rewritten into locals.
    #[test]
    fn an_attribute_assigned_in_a_branch_of_a_constructor() {
        let out = run(
            "position_attr_in_branch",
            "class Gate:\n\
             \x20   def __init__(self, n: int) -> None:\n\
             \x20       self.count: int = 0\n\
             \x20       if n > 0:\n\
             \x20           self.count = n\n\
             \n\
             def build(n: int) -> int:\n\
             \x20   g = Gate(n)\n\
             \x20   return g.count\n",
            "    println!(\"{} {}\", build(7).unwrap(), build(-1).unwrap());",
        );
        assert_eq!(out.trim(), "7 0");
    }

    #[test]
    fn an_attribute_assigned_in_a_loop_of_a_constructor() {
        let out = run(
            "position_attr_in_loop",
            "class Counter:\n\
             \x20   def __init__(self, n: int) -> None:\n\
             \x20       self.count: int = 0\n\
             \x20       for i in range(n):\n\
             \x20           self.count = i\n\
             \n\
             def build(n: int) -> int:\n\
             \x20   c = Counter(n)\n\
             \x20   return c.count\n",
            "    println!(\"{}\", build(4).unwrap());",
        );
        assert_eq!(out.trim(), "3");
    }

    /// A collection attribute mutated inside a constructor's loop.
    #[test]
    fn an_attribute_collection_appended_to_in_a_constructor() {
        let out = run(
            "position_append_in_loop",
            "class Log:\n\
             \x20   def __init__(self, n: int) -> None:\n\
             \x20       self.seen: list[int] = []\n\
             \x20       for i in range(n):\n\
             \x20           self.seen.append(i)\n\
             \n\
             def size(n: int) -> int:\n\
             \x20   log = Log(n)\n\
             \x20   return len(log.seen)\n",
            "    println!(\"{}\", size(3).unwrap());",
        );
        assert_eq!(out.trim(), "3");
    }

    /// A local reassigned inside a constructor used to be emitted without `mut`.
    ///
    /// The constructor fed the emitter one statement at a time, and `Stmt::Bind` decides
    /// mutability by looking for a later assignment in the slice it is handed — which was always
    /// a slice of one.
    #[test]
    fn a_local_reassigned_inside_a_constructor() {
        let out = run(
            "position_local_reassigned",
            "class Total:\n\
             \x20   def __init__(self, n: int) -> None:\n\
             \x20       running = 0\n\
             \x20       running = running + n\n\
             \x20       self.value: int = running\n\
             \n\
             def build(n: int) -> int:\n\
             \x20   t = Total(n)\n\
             \x20   return t.value\n",
            "    println!(\"{}\", build(5).unwrap());",
        );
        assert_eq!(out.trim(), "5");
    }

    /// A trailing bare `return` in `__init__` means nothing and is dropped, not refused.
    #[test]
    fn a_trailing_return_in_a_constructor_is_accepted() {
        let out = run(
            "position_trailing_return",
            "class Thing:\n\
             \x20   def __init__(self, n: int) -> None:\n\
             \x20       self.count: int = n\n\
             \x20       return\n\
             \n\
             def build(n: int) -> int:\n\
             \x20   t = Thing(n)\n\
             \x20   return t.count\n",
            "    println!(\"{}\", build(9).unwrap());",
        );
        assert_eq!(out.trim(), "9");
    }
}
