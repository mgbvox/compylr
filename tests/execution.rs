//! Python semantics, verified by running the emitted code.
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
