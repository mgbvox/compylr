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
    let mut d = compat::FastMap::default();
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
    let mut d = compat::FastMap::default();
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
    let mut d = compat::FastMap::default();
    d.insert(String::from("ab"), 1i64);
    d.insert(String::from("cde"), 2i64);
    println!("{}", key_chars(d).unwrap());
    println!("{}", set_total(compat::FastSet::from_iter([1i64, 2, 3])).unwrap());
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
fn a_nested_read_borrows_the_intermediate_rather_than_cloning_it() {
    // `m[i][j]` read `m[i]` with `py_subscript`, which hands back a **clone** of the row. In the
    // inner loop of a matrix multiply that is an allocation and an O(n) copy per element access,
    // turning an O(n^3) algorithm into O(n^4) -- and it is invisible, because every answer is
    // correct. The demo's benchmark is what found it: matrix multiply was no faster compiled.
    //
    // The emitted form is asserted as well as the values, because the defect *is* the emitted
    // form. Both versions produce the same numbers.
    let source = "def cell(m: list[list[int]], i: int, j: int) -> int:\n    return m[i][j]\n";
    let emitted = lookup("rust")
        .unwrap()
        .emit(&unit_from(source))
        .expect("must emit");
    let generated = &emitted["src/generated.rs"];
    assert!(
        generated.contains("py_borrow"),
        "the intermediate collection must be borrowed, not cloned:\n{generated}"
    );

    let out = run(
        "read_nested",
        concat!(
            "def cell(m: list[list[int]], i: int, j: int) -> int:\n",
            "    return m[i][j]\n\n",
            "def from_the_end(m: list[list[int]]) -> int:\n",
            "    return m[-1][-1]\n\n",
            "def through_a_mapping(d: dict[str, list[int]], k: str, i: int) -> int:\n",
            "    return d[k][i]\n\n",
            "def deep(m: list[list[list[int]]]) -> int:\n",
            "    return m[0][1][0]\n\n",
            "def measured(m: list[list[int]], i: int) -> int:\n",
            "    return len(m[i])\n\n",
            "def held(m: list[list[int]], i: int, x: int) -> bool:\n",
            "    return x in m[i]\n\n",
            "def summed(m: list[list[int]], i: int) -> int:\n",
            "    total = 0\n",
            "    for value in m[i]:\n",
            "        total = total + value\n",
            "    return total\n\n",
            "def missing_row(d: dict[str, list[int]], k: str) -> int:\n",
            "    return d[k][0]\n",
        ),
        r#"
    let m = vec![vec![1i64, 2], vec![3, 4]];
    println!("{}", cell(m.clone(), 1, 0).unwrap());
    println!("{}", from_the_end(m.clone()).unwrap());
    println!("{}", deep(vec![vec![vec![1i64], vec![9]]]).unwrap());
    println!("{}", measured(m.clone(), 0).unwrap());
    println!("{}", held(m.clone(), 1, 4).unwrap());
    println!("{}", summed(m.clone(), 1).unwrap());
    let mut d = compat::FastMap::default();
    d.insert(String::from("k"), vec![7i64, 8]);
    println!("{}", through_a_mapping(d.clone(), String::from("k"), 1).unwrap());
    println!("{}", missing_row(d, String::from("absent")).is_err());
    println!("{}", cell(m, 9, 0).is_err());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "3", "a nested read still reads the right element");
    assert_eq!(
        lines[1], "4",
        "a negative index still counts from the end at both levels"
    );
    assert_eq!(lines[2], "9", "the chain is followed to any depth");
    assert_eq!(lines[3], "2", "`len` measures the row, not a copy of it");
    assert_eq!(lines[4], "true", "membership tests the row");
    assert_eq!(lines[5], "7", "iterating a row still sees every element");
    assert_eq!(
        lines[6], "8",
        "and a mapping's value is reached the same way"
    );
    assert_eq!(
        lines[7], "true",
        "a missing key still reports rather than yielding an empty row"
    );
    assert_eq!(lines[8], "true", "an out-of-range row still reports");
}

#[test]
fn a_read_through_a_collection_the_loop_mutates_still_copies_it() {
    // The borrow above is only safe while nothing disturbs what it borrows from. A loop that
    // writes to the collection it is walking has to keep iterating the snapshot -- which is what
    // Python's `for` does -- and holding a borrow across the body would instead be a borrow
    // checker error about generated code.
    let out = run(
        "read_nested_disturbed",
        concat!(
            "def doubled(m: list[list[int]], i: int) -> list[list[int]]:\n",
            "    out: list[list[int]] = []\n",
            "    for value in m[i]:\n",
            "        row: list[int] = []\n",
            "        row.append(value * 2)\n",
            "        out.append(row)\n",
            "    return out\n\n",
            "def grown(n: int) -> int:\n",
            "    m: list[list[int]] = [[1, 2]]\n",
            "    seen = 0\n",
            "    for value in m[0]:\n",
            "        seen = seen + value\n",
            "        m[0] = [n]\n",
            "    return seen\n",
        ),
        r#"
    println!("{:?}", doubled(vec![vec![1i64, 2]], 0).unwrap());
    println!("{}", grown(99).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "[[2], [4]]");
    assert_eq!(
        lines[1], "3",
        "the loop walks what it started with, as Python's `for` does"
    );
}

#[test]
fn a_write_through_a_nested_collection_reaches_the_original() {
    // The ordinary rule clones a collection wherever it is consumed, and the base of a mutation
    // is the one place that is actively wrong. It was already handled for an attribute --
    // `self.entries[k] = v` -- and not for a subscript, so `table[i][j] = v` wrote into a clone of
    // the row and every write was lost.
    //
    // The failure mode is why this is executed rather than read: nothing about the emitted text
    // looks wrong, no error is raised, and a dynamic-programming table simply comes back full of
    // the value it was initialised with. That is a plausible answer, which is what makes it the
    // worst kind of defect.
    let out = run(
        "mut_nested_setitem",
        concat!(
            "def zeros(rows: int, columns: int) -> list[list[int]]:\n",
            "    out: list[list[int]] = []\n",
            "    for _r in range(rows):\n",
            "        line: list[int] = []\n",
            "        for _c in range(columns):\n",
            "            line.append(0)\n",
            "        out.append(line)\n",
            "    return out\n\n",
            "def diagonal(size: int) -> list[list[int]]:\n",
            "    out = zeros(size, size)\n",
            "    for i in range(size):\n",
            "        out[i][i] = 1\n",
            "    return out\n\n",
            "def through_a_mapping(k: str, v: int) -> dict[str, list[int]]:\n",
            "    d: dict[str, list[int]] = {}\n",
            "    row: list[int] = [0, 0]\n",
            "    d[k] = row\n",
            "    d[k][1] = v\n",
            "    return d\n\n",
            "def appended_through(k: str, v: int) -> dict[str, list[int]]:\n",
            "    d: dict[str, list[int]] = {}\n",
            "    row: list[int] = []\n",
            "    d[k] = row\n",
            "    d[k].append(v)\n",
            "    return d\n\n",
            "def deeper(v: int) -> list[list[list[int]]]:\n",
            "    out: list[list[list[int]]] = [[[0, 0]]]\n",
            "    out[0][0][1] = v\n",
            "    return out\n\n",
            "def missing_row(k: str) -> dict[str, list[int]]:\n",
            "    d: dict[str, list[int]] = {}\n",
            "    d[k][0] = 1\n",
            "    return d\n",
        ),
        r#"
    println!("{:?}", diagonal(3).unwrap());
    println!("{:?}", through_a_mapping(String::from("k"), 7).unwrap());
    println!("{:?}", appended_through(String::from("k"), 7).unwrap());
    println!("{:?}", deeper(7).unwrap());
    println!("{}", missing_row(String::from("k")).is_err());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0], "[[1, 0, 0], [0, 1, 0], [0, 0, 1]]",
        "a write through a row must reach the table, not a copy of the row"
    );
    assert_eq!(
        lines[1], "{\"k\": [0, 7]}",
        "a write through a mapping's value must reach the value in the map"
    );
    assert_eq!(
        lines[2], "{\"k\": [7]}",
        "appending through a mapping's value must reach the value in the map"
    );
    assert_eq!(
        lines[3], "[[[0, 7]]]",
        "the chain is followed to any depth, not only one level"
    );
    assert_eq!(
        lines[4], "true",
        "writing through a key that is absent reports, exactly as reading it does -- \
         creating an empty row would invent a value the program never wrote"
    );
}

#[test]
fn a_write_through_a_nested_attribute_reaches_the_instance() {
    // The same defect, reached through `self`. An attribute base was already a place; a subscript
    // of one was not, so a grid held in an instance had the same silent failure -- and this is the
    // shape where it matters most, because the whole reason to hold state in an instance is that
    // the next call sees it.
    let out = run(
        "mut_nested_attr",
        concat!(
            "class Grid:\n",
            "    def __init__(self, size: int) -> None:\n",
            "        self.rows: list[list[int]] = []\n",
            "        for _r in range(size):\n",
            "            line: list[int] = []\n",
            "            for _c in range(size):\n",
            "                line.append(0)\n",
            "            self.rows.append(line)\n\n",
            "    def set(self, row: int, column: int, value: int) -> None:\n",
            "        self.rows[row][column] = value\n\n",
            "    def get(self, row: int, column: int) -> int:\n",
            "        return self.rows[row][column]\n",
        ),
        r#"
    let mut grid = Grid::__compylr_new(2).unwrap();
    grid.set(1, 0, 5).unwrap();
    println!("{}", grid.get(1, 0).unwrap());
    println!("{}", grid.get(0, 0).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0], "5",
        "the write must survive the call that made it"
    );
    assert_eq!(lines[1], "0", "and must not have reached any other cell");
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
    println!("{}", in_set(compat::FastSet::from_iter([1i64, 2]), 2).unwrap());
    let mut d = compat::FastMap::default();
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
fn a_mutating_method_called_through_a_subscript_reaches_the_element() {
    // The same defect as the nested assignment, one step further: a receiver reached through a
    // subscript was emitted as a value, so `items[0].bump()` bumped a clone of the element and
    // the list was left exactly as it was.
    //
    // `total` is the other half. A method that does *not* mutate must still borrow the way a read
    // does, or a list of instances could never be read from without being bound `mut` — which
    // would be a borrow-checker error about generated code rather than anything a user could act
    // on.
    let out = run(
        "cls_subscript_receiver",
        concat!(
            "class Cell:\n",
            "    def __init__(self, value: int) -> None:\n",
            "        self.value: int = value\n",
            "\n",
            "    def bump(self, by: int) -> None:\n",
            "        self.value = self.value + by\n",
            "\n",
            "    def get(self) -> int:\n",
            "        return self.value\n",
            "\n",
            "def bumped(start: int, by: int) -> int:\n",
            "    cells: list[Cell] = []\n",
            "    cells.append(Cell(start))\n",
            "    cells[0].bump(by)\n",
            "    return cells[0].get()\n\n",
            "def total(a: int, b: int) -> int:\n",
            "    cells: list[Cell] = []\n",
            "    cells.append(Cell(a))\n",
            "    cells.append(Cell(b))\n",
            "    sum: int = 0\n",
            "    for cell in cells:\n",
            "        sum = sum + cell.get()\n",
            "    return sum\n",
        ),
        r#"
    println!("{}", bumped(10, 5).unwrap());
    println!("{}", total(3, 4).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0], "15",
        "the mutation must reach the element in the list, not a copy of it"
    );
    assert_eq!(
        lines[1], "7",
        "a method that does not mutate still reads through a shared borrow"
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

/// In-place accumulation, checked by running it.
///
/// The emission tests assert that `x = x + y` compiles to an in-place update. That is a claim
/// about text, and the thing that would actually hurt is an in-place update producing a different
/// *answer* — text assembled in the wrong order, or an overflow that stopped being reported
/// because the checked helper was traded for a raw `+=`. Only running it settles those.
#[test]
fn string_accumulation_builds_the_same_text_as_before() {
    let out = run(
        "accumulate_str",
        concat!(
            "def joined(words: list[str], sep: str) -> str:\n",
            "    out = \"\"\n",
            "    i = 0\n",
            "    while i < len(words):\n",
            "        out = out + words[i]\n",
            "        out = out + sep\n",
            "        i = i + 1\n",
            "    return out\n",
        ),
        r#"
    let words = vec![
        String::from("alpha"),
        String::from("beta"),
        String::from("gamma"),
    ];
    println!("{}", joined(words, String::from("-")).unwrap());
"#,
    );
    assert_eq!(out.trim(), "alpha-beta-gamma-");
}

#[test]
fn accumulation_preserves_order_for_non_ascii_text() {
    // Appending in place and rebuilding differ in where the bytes are copied, and a rule that
    // matched the mirrored form would reverse them. Non-ASCII makes a byte-level mistake visible
    // as mojibake rather than as a merely reordered word.
    let out = run(
        "accumulate_unicode",
        concat!(
            "def grow(head: str, tail: str) -> str:\n",
            "    out = head\n",
            "    out = out + tail\n",
            "    return out\n",
        ),
        r#"
    println!("{}", grow(String::from("héllo·"), String::from("wörld✓")).unwrap());
"#,
    );
    assert_eq!(out.trim(), "héllo·wörld✓");
}

#[test]
fn the_mirrored_form_still_prepends() {
    // `x = y + x` must keep meaning `y` followed by `x`. This is the test that would fail if the
    // rewrite over-fired onto the shape that looks like it should work.
    let out = run(
        "accumulate_mirrored",
        concat!(
            "def prefixed(head: str, tail: str) -> str:\n",
            "    out = tail\n",
            "    out = head + out\n",
            "    return out\n",
        ),
        r#"
    println!("{}", prefixed(String::from("<<"), String::from(">>")).unwrap());
"#,
    );
    assert_eq!(out.trim(), "<<>>");
}

#[test]
fn integer_accumulation_still_reports_overflow() {
    // The in-place numeric implementation keeps `checked_add`. An `+=` here would wrap silently
    // in release and panic in debug, and both are a change of meaning rather than a speedup.
    let out = run(
        "accumulate_overflow",
        concat!(
            "def climb(start: int, step: int) -> int:\n",
            "    total = start\n",
            "    total = total + step\n",
            "    return total\n",
        ),
        r#"
    println!("{}", climb(1, 2).unwrap());
    match climb(9223372036854775807, 1) {
        Ok(value) => println!("no overflow reported: {value}"),
        Err(error) => println!("reported: {error}"),
    }
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "3");
    assert_eq!(lines[1], "reported: integer overflow");
}

#[test]
fn float_accumulation_sums_as_before() {
    let out = run(
        "accumulate_float",
        concat!(
            "def total(values: list[float]) -> float:\n",
            "    sum = 0.0\n",
            "    i = 0\n",
            "    while i < len(values):\n",
            "        sum = sum + values[i]\n",
            "        i = i + 1\n",
            "    return sum\n",
        ),
        r#"
    println!("{}", total(vec![0.5, 1.25, 2.25]).unwrap());
"#,
    );
    assert_eq!(out.trim(), "4");
}

#[test]
fn a_collection_built_in_a_loop_still_accumulates_correctly() {
    // The loop counter is itself an accumulator (`i = i + 1`), so this exercises the rewrite in
    // the position where getting it wrong hangs rather than answers wrongly.
    let out = run(
        "accumulate_counter",
        concat!(
            "def squares(n: int) -> list[int]:\n",
            "    out: list[int] = []\n",
            "    i = 0\n",
            "    while i < n:\n",
            "        out.append(i * i)\n",
            "        i = i + 1\n",
            "    return out\n",
        ),
        r#"
    println!("{:?}", squares(5).unwrap());
"#,
    );
    assert_eq!(out.trim(), "[0, 1, 4, 9, 16]");
}

#[test]
fn a_chain_accumulation_keeps_its_order() {
    // This is the demo's `joined`, which is the function the in-place rule exists for. The risk
    // the emission test cannot see is ordering: two appends in the wrong order still compile and
    // still produce a string of the right length.
    let out = run(
        "accumulate_chain",
        concat!(
            "def joined(words: list[str], separator: str) -> str:\n",
            "    out = \"\"\n",
            "    first = True\n",
            "    for word in words:\n",
            "        if first:\n",
            "            out = out + word\n",
            "            first = False\n",
            "        else:\n",
            "            out = out + separator + word\n",
            "    return out\n",
        ),
        r#"
    let words = vec![
        String::from("alpha"),
        String::from("beta"),
        String::from("gamma"),
    ];
    println!("{}", joined(words, String::from("-")).unwrap());
    println!("{}", joined(vec![String::from("solo")], String::from("-")).unwrap());
    println!("[{}]", joined(vec![], String::from("-")).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "alpha-beta-gamma");
    assert_eq!(lines[1], "solo");
    assert_eq!(lines[2], "[]");
}

#[test]
fn a_chain_accumulation_of_floats_rounds_identically() {
    // Floating-point addition is not associative, so a rule that reassociated `((x + a) + b)`
    // into `x + (a + b)` would change the last bits. Walking the left spine performs the same two
    // additions in the same order; these values are chosen so the two groupings differ.
    let out = run(
        "accumulate_chain_float",
        concat!(
            "def drift(a: float, b: float) -> float:\n",
            "    total = 1.0\n",
            "    total = total + a + b\n",
            "    return total\n",
        ),
        r#"
    let a = 1e16_f64;
    let b = -1e16_f64;
    println!("{}", drift(a, b).unwrap());
    println!("{}", (1.0_f64 + a) + b);
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0], lines[1],
        "the compiled chain must round exactly as the left-associative source does"
    );
}

#[test]
fn a_chain_accumulation_of_integers_still_reports_overflow() {
    // Each step keeps its check, so the overflow is reported at the same operand it would have
    // been reported at before.
    let out = run(
        "accumulate_chain_overflow",
        concat!(
            "def climb(a: int, b: int) -> int:\n",
            "    total = 9223372036854775806\n",
            "    total = total + a + b\n",
            "    return total\n",
        ),
        r#"
    println!("{:?}", climb(1, 0).map_err(|e| e.to_string()));
    println!("{:?}", climb(1, 1).map_err(|e| e.to_string()));
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "Ok(9223372036854775807)");
    assert_eq!(lines[1], "Err(\"integer overflow\")");
}

#[test]
fn a_collection_built_in_a_loop_survives_being_moved_out() {
    let out = run(
        "moved_return",
        concat!(
            "def build(n: int) -> list[int]:\n",
            "    out: list[int] = []\n",
            "    i = 0\n",
            "    while i < n:\n",
            "        out.append(i * 2)\n",
            "        i = i + 1\n",
            "    return out\n",
        ),
        r#"
    println!("{:?}", build(5).unwrap());
    println!("{:?}", build(0).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "[0, 2, 4, 6, 8]");
    assert_eq!(lines[1], "[]");
}

#[test]
fn a_returned_attribute_leaves_the_instance_intact() {
    // The instance outlives the call, so a field must be copied rather than moved out of. If the
    // move rule ever reached an attribute, the *second* call would come back empty — which is
    // why this reads it twice.
    let out = run(
        "moved_return_attribute",
        concat!(
            "class Bag:\n",
            "    def __init__(self, items: list[int]) -> None:\n",
            "        self.items: list[int] = items\n",
            "\n",
            "    def contents(self) -> list[int]:\n",
            "        return self.items\n",
        ),
        r#"
    let bag = Bag::__compylr_new(vec![1i64, 2, 3]).unwrap();
    println!("{:?}", bag.contents().unwrap());
    println!("{:?}", bag.contents().unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "[1, 2, 3]");
    assert_eq!(
        lines[1], "[1, 2, 3]",
        "reading a field twice must give the same answer twice"
    );
}

#[test]
fn a_non_tail_return_still_answers_correctly() {
    let out = run(
        "moved_return_branch",
        concat!(
            "def pick(early: list[int], rest: list[int], flag: bool) -> list[int]:\n",
            "    if flag:\n",
            "        return early\n",
            "    return rest\n",
        ),
        r#"
    println!("{:?}", pick(vec![1i64], vec![2i64], true).unwrap());
    println!("{:?}", pick(vec![1i64], vec![2i64], false).unwrap());
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "[1]");
    assert_eq!(lines[1], "[2]");
}

#[test]
fn a_fused_mapping_update_counts_correctly() {
    let out = run(
        "fused_tally",
        concat!(
            "def tally(words: list[str]) -> dict[str, int]:\n",
            "    counts: dict[str, int] = {}\n",
            "    for word in words:\n",
            "        if word in counts:\n",
            "            counts[word] = counts[word] + 1\n",
            "        else:\n",
            "            counts[word] = 1\n",
            "    return counts\n",
        ),
        r#"
    let words = vec![
        String::from("a"),
        String::from("b"),
        String::from("a"),
        String::from("a"),
    ];
    let counts = tally(words).unwrap();
    // Sorted before printing: mapping iteration order is not guaranteed and varies between runs.
    let mut pairs: Vec<(String, i64)> = counts.into_iter().collect();
    pairs.sort();
    println!("{pairs:?}");
"#,
    );
    assert_eq!(out.trim(), r#"[("a", 3), ("b", 1)]"#);
}

#[test]
fn a_fused_update_of_a_missing_key_still_reports_and_does_not_create_it() {
    // The whole risk of fusing a read into a write is that the fused form quietly *inserts*.
    // Reading a key that is absent is an error in this subset; assignment is what creates one.
    let out = run(
        "fused_missing_key",
        concat!(
            "def bump(k: str) -> int:\n",
            "    counts: dict[str, int] = {}\n",
            "    counts[\"present\"] = 1\n",
            "    counts[k] = counts[k] + 1\n",
            "    return len(counts)\n",
        ),
        r#"
    match bump(String::from("present")) {
        Ok(size) => println!("ok {size}"),
        Err(error) => println!("reported: {error}"),
    }
    match bump(String::from("absent")) {
        Ok(size) => println!("no error, size {size}"),
        Err(error) => println!("reported: {error}"),
    }
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "ok 1");
    assert_eq!(
        lines[1], "reported: \"absent\"",
        "a fused update must not invent the key it was told to add to"
    );
}

#[test]
fn a_fused_sequence_update_reports_out_of_range() {
    let out = run(
        "fused_sequence",
        concat!(
            "def bump(n: int, at: int) -> list[int]:\n",
            "    xs: list[int] = []\n",
            "    i = 0\n",
            "    while i < n:\n",
            "        xs.append(i)\n",
            "        i = i + 1\n",
            "    xs[at] = xs[at] + 10\n",
            "    return xs\n",
        ),
        r#"
    println!("{:?}", bump(3, 0).unwrap());
    println!("{:?}", bump(3, -1).unwrap());
    println!("{:?}", bump(3, 5).map_err(|e| e.to_string()));
    println!("{:?}", bump(3, -9).map_err(|e| e.to_string()));
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "[10, 1, 2]");
    assert_eq!(
        lines[1], "[0, 1, 12]",
        "a negative index counts from the end"
    );
    assert_eq!(lines[2], r#"Err("index out of range")"#);
    assert_eq!(lines[3], r#"Err("index out of range")"#);
}

#[test]
fn a_fused_text_update_concatenates_in_the_right_order() {
    let out = run(
        "fused_text",
        concat!(
            "def build(parts: list[str]) -> dict[str, str]:\n",
            "    out: dict[str, str] = {}\n",
            "    out[\"k\"] = \"\"\n",
            "    for part in parts:\n",
            "        out[\"k\"] = out[\"k\"] + part\n",
            "    return out\n",
        ),
        r#"
    let parts = vec![String::from("a"), String::from("b"), String::from("c")];
    let built = build(parts).unwrap();
    println!("{}", built["k"]);
"#,
    );
    assert_eq!(out.trim(), "abc");
}

#[test]
fn a_fused_integer_update_still_reports_overflow() {
    let out = run(
        "fused_overflow",
        concat!(
            "def climb(step: int) -> list[int]:\n",
            "    xs: list[int] = []\n",
            "    xs.append(9223372036854775807)\n",
            "    xs[0] = xs[0] + step\n",
            "    return xs\n",
        ),
        r#"
    println!("{:?}", climb(0).unwrap());
    println!("{:?}", climb(1).map_err(|e| e.to_string()));
"#,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "[9223372036854775807]");
    assert_eq!(lines[1], r#"Err("integer overflow")"#);
}
