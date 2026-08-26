//! The translation tier: generated Rust must answer what CPython answers.
//!
//! Every other execution test in this repository asserts a value a person typed while reading the
//! rule they were implementing. That catches a backend disagreeing with its author's belief; it
//! cannot catch one disagreeing with Python, because Python was never consulted. Here the expected
//! answer is produced by CPython running the fixture's own source, so there is nothing for anyone
//! to type incorrectly.
//!
//! This tier exercises the generated target source directly, without the host language's calling
//! convention: it emits the crate, writes a `main` around it, compiles, runs, and compares
//! transcripts as text. The boundary tier does the same corpus through PyO3 and compares values.
//! Neither stands in for the other -- a program can be translated correctly and converted wrongly
//! at the boundary, and only the second sees that.
//!
//! Text, here, and values there, for a reason: the Rust side has no Python object to compare, so a
//! canonical transcript is the only shared form. It is defined in `_runner.py` and mirrored below,
//! and `the_two_renderings_of_every_type_agree` is what keeps the mirror honest.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use compylr_backend_rust::{InstanceAccess, instance_parameter_accesses};
use compylr_frontend_python::frontend::parse_file;
use compylr_frontend_python::lower::lower_source_members;
use compylr_ir::{Class, Function, Ty, Unit};
use compylr_registry::backends::lookup;
use serde_json::Value;

mod support;
use support::drivers;

/// Python's own stance, which is what an unconfigured compilation resolves to.
fn python_stance() -> compylr_ir::Behavior {
    compylr_ir::Behavior::of(&compylr_frontend_python::component::PYTHON_BEHAVIOR)
}

/// Whether `rustc` is installed. Its absence is a fact about the machine, not about the program,
/// and is the only thing this tier is allowed to skip for.
fn rustc_available() -> bool {
    Command::new("rustc").arg("--version").output().is_ok()
}

// ---------------------------------------------------------------------------------------------
// The corpus, grouped
// ---------------------------------------------------------------------------------------------

/// Every accepted fixture, grouped so that cross-source calls resolve.
///
/// The same grouping `emit_quality.rs` uses, and the same one `_runner.group_for` states for the
/// Python side: a call across two sources resolves only when both are in one unit.
fn fixture_groups() -> Vec<(String, Vec<String>)> {
    let mut singles = Vec::new();
    let mut cross_source = Vec::new();
    for stem in drivers::accepted_stems() {
        if stem.starts_with("cross_source_") {
            cross_source.push(stem);
        } else {
            singles.push((stem.clone(), vec![stem]));
        }
    }
    if !cross_source.is_empty() {
        singles.push(("cross_source".to_string(), cross_source));
    }
    singles
}

fn unit_from(stems: &[String]) -> Unit {
    let mut unit = Unit::new();
    for stem in stems {
        let path = drivers::accepted_dir().join(format!("{stem}.py"));
        let parsed = parse_file(&path).expect("fixture must parse");
        let (functions, classes) = lower_source_members(&parsed, python_stance())
            .unwrap_or_else(|e| panic!("{stem} should lower: {e}"));
        for class in classes {
            unit.add_class(class).expect("names are unique corpus-wide");
        }
        for function in functions {
            unit.add_function(function)
                .expect("names are unique corpus-wide");
        }
    }
    unit.validate().expect("calls must resolve");
    unit
}

fn function_named<'a>(unit: &'a Unit, name: &str) -> Option<&'a Function> {
    unit.functions().find(|f| f.name == name)
}

fn class_named<'a>(unit: &'a Unit, name: &str) -> Option<&'a Class> {
    unit.class(name)
}

// ---------------------------------------------------------------------------------------------
// Ty-directed literals and rendering
// ---------------------------------------------------------------------------------------------

/// A Rust expression building the argument a driver declared, at the type the signature declares.
///
/// Directed by the parameter's `Ty` rather than by the JSON value's shape, because the two differ
/// exactly where it matters: an integer written for a float parameter must become an `f64`, and a
/// tagged set must become a `FastSet` rather than a `Vec`.
fn literal(ty: &Ty, value: &Value, unit: &Unit) -> String {
    match ty {
        Ty::Int => format!("{}i64", value.as_i64().expect("an integer argument")),
        Ty::Float => {
            let number = value.as_f64().expect("a numeric argument");
            format!("{number:?}f64")
        }
        Ty::Bool => format!("{}", value.as_bool().expect("a boolean argument")),
        Ty::Str => format!(
            "String::from({:?})",
            value.as_str().expect("a text argument")
        ),
        Ty::Unit => "()".to_string(),
        Ty::List(inner) => {
            let items = value.as_array().expect("a sequence argument");
            let rendered: Vec<String> = items.iter().map(|v| literal(inner, v, unit)).collect();
            format!("vec![{}]", rendered.join(", "))
        }
        Ty::Set(inner) => {
            let items = tagged(value, "$set").expect("a set argument must be tagged");
            let rendered: Vec<String> = items.iter().map(|v| literal(inner, v, unit)).collect();
            format!("FastSet::from_iter([{}])", rendered.join(", "))
        }
        Ty::Dict(key, val) => {
            let pairs = tagged(value, "$dict").expect("a mapping argument must be tagged");
            let rendered: Vec<String> = pairs
                .iter()
                .map(|pair| {
                    let pair = pair.as_array().expect("a mapping entry is a pair");
                    format!(
                        "({}, {})",
                        literal(key, &pair[0], unit),
                        literal(val, &pair[1], unit)
                    )
                })
                .collect();
            format!("FastMap::from_iter([{}])", rendered.join(", "))
        }
        Ty::Tuple(items) => {
            let values = tagged(value, "$tuple").expect("a tuple argument must be tagged");
            let rendered: Vec<String> = items
                .iter()
                .zip(values)
                .map(|(t, v)| literal(t, v, unit))
                .collect();
            format!("({})", rendered.join(", "))
        }
        Ty::Instance(name) => {
            let class = class_named(unit, name)
                .unwrap_or_else(|| panic!("{name} is not a class in this unit"));
            let args = value["args"].as_array().cloned().unwrap_or_default();
            let rendered: Vec<String> = class
                .init
                .params
                .iter()
                .zip(&args)
                .map(|(p, v)| literal(&p.ty, v, unit))
                .collect();
            format!("{name}::__compylr_new({})?", rendered.join(", "))
        }
    }
}

fn tagged<'a>(value: &'a Value, tag: &str) -> Option<&'a Vec<Value>> {
    value.get(tag).and_then(Value::as_array)
}

/// The same value, expressed at the type its signature declares.
///
/// Only the declared type decides how a value renders: `def widen(n: int) -> float: return n`
/// answers the integer 3 in Python and 3.0 translated, and those are the same answer. Rust builds
/// its literal from the declared `Ty` already, so this is what lets the Python side of the mirror
/// be handed the same value rather than the spelling the table happened to use.
fn declared_value(ty: &Ty, value: &Value) -> Value {
    match ty {
        Ty::Float => match value.as_f64() {
            Some(number) => serde_json::json!(number),
            None => value.clone(),
        },
        Ty::List(inner) => match value.as_array() {
            Some(items) => Value::Array(items.iter().map(|v| declared_value(inner, v)).collect()),
            None => value.clone(),
        },
        Ty::Set(inner) => match tagged(value, "$set") {
            Some(items) => serde_json::json!({
                "$set": items.iter().map(|v| declared_value(inner, v)).collect::<Vec<_>>()
            }),
            None => value.clone(),
        },
        Ty::Tuple(items) => match tagged(value, "$tuple") {
            Some(values) => serde_json::json!({
                "$tuple": items
                    .iter()
                    .zip(values)
                    .map(|(t, v)| declared_value(t, v))
                    .collect::<Vec<_>>()
            }),
            None => value.clone(),
        },
        Ty::Dict(key, val) => match tagged(value, "$dict") {
            Some(pairs) => serde_json::json!({
                "$dict": pairs
                    .iter()
                    .filter_map(Value::as_array)
                    .map(|pair| vec![declared_value(key, &pair[0]), declared_value(val, &pair[1])])
                    .collect::<Vec<_>>()
            }),
            None => value.clone(),
        },
        _ => value.clone(),
    }
}

/// How a `Ty` is spelled in generated Rust.
///
/// Only the mirror test needs this: a value bound on its own has no call site to infer from, so
/// `vec![]` for an empty sequence would be ambiguous. Everywhere else the signature supplies it.
fn rust_type(ty: &Ty) -> String {
    match ty {
        Ty::Int => "i64".to_string(),
        Ty::Float => "f64".to_string(),
        Ty::Bool => "bool".to_string(),
        Ty::Str => "String".to_string(),
        Ty::Unit => "()".to_string(),
        Ty::List(inner) => format!("Vec<{}>", rust_type(inner)),
        Ty::Set(inner) => format!("FastSet<{}>", rust_type(inner)),
        Ty::Dict(key, val) => format!("FastMap<{}, {}>", rust_type(key), rust_type(val)),
        Ty::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(rust_type).collect();
            format!("({},)", parts.join(", "))
        }
        Ty::Instance(name) => name.clone(),
    }
}

/// A Rust expression rendering `expr` as one canonical JSON value.
///
/// Mapping keys are sorted and sets become sorted arrays, because the subset promises neither
/// order. Asserting on the order a hash map happens to yield would make this suite flaky rather
/// than make the compiler right.
fn render_expr(ty: &Ty, expr: &str, depth: usize) -> String {
    match ty {
        Ty::Int => format!("format!(\"{{}}\", {expr})"),
        Ty::Bool => format!("(if {expr} {{ \"true\" }} else {{ \"false\" }}).to_string()"),
        Ty::Float => format!("__render_float({expr})"),
        Ty::Str => format!("__render_str(&{expr})"),
        Ty::Unit => format!("{{ let _unit = {expr}; \"null\".to_string() }}"),
        Ty::List(inner) => {
            let (acc, item, index) = (
                format!("__acc{depth}"),
                format!("__item{depth}"),
                format!("__i{depth}"),
            );
            let rendered = render_expr(inner, &format!("({item}).clone()"), depth + 1);
            format!(
                "{{ let mut {acc} = String::from(\"[\"); \
                 for ({index}, {item}) in ({expr}).iter().enumerate() {{ \
                 if {index} > 0 {{ {acc}.push(','); }} {acc}.push_str(&{rendered}); }} \
                 {acc}.push(']'); {acc} }}"
            )
        }
        Ty::Set(inner) => {
            let sorted = format!("__sorted{depth}");
            let rendered = render_expr(&Ty::List(inner.clone()), &sorted, depth + 1);
            format!(
                "{{ let mut {sorted}: Vec<_> = ({expr}).iter().cloned().collect(); \
                 {sorted}.sort(); {rendered} }}"
            )
        }
        Ty::Dict(key, val) => {
            let (pairs, acc, index, entry) = (
                format!("__pairs{depth}"),
                format!("__acc{depth}"),
                format!("__i{depth}"),
                format!("__kv{depth}"),
            );
            let key_text = render_key(key, &format!("({entry}).0"));
            let value_text = render_expr(val, &format!("({entry}).1.clone()"), depth + 1);
            format!(
                "{{ let mut {pairs}: Vec<_> = ({expr}).iter().map(|(__k, __v)| \
                 (__k.clone(), __v.clone())).collect(); \
                 {pairs}.sort_by(|__a, __b| __a.0.cmp(&__b.0)); \
                 let mut {acc} = String::from(\"{{\"); \
                 for ({index}, {entry}) in {pairs}.iter().enumerate() {{ \
                 if {index} > 0 {{ {acc}.push(','); }} \
                 {acc}.push_str(&{key_text}); {acc}.push(':'); \
                 {acc}.push_str(&{value_text}); }} \
                 {acc}.push('}}'); {acc} }}"
            )
        }
        Ty::Tuple(items) => {
            let parts: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(i, t)| render_expr(t, &format!("({expr}).{i}.clone()"), depth + 1))
                .collect();
            let holes = vec!["{}"; parts.len()].join(",");
            format!("format!(\"[{holes}]\", {})", parts.join(", "))
        }
        // Unreachable by construction: a driver entry whose result is an instance must declare
        // methods, and those methods' results are what the transcript renders.
        Ty::Instance(name) => panic!(
            "an instance of {name} has no transcript of its own; the driver must call methods on it"
        ),
    }
}

/// A JSON object key is always a string, whatever the mapping's key type is.
fn render_key(ty: &Ty, expr: &str) -> String {
    match ty {
        Ty::Str => format!("__render_str(&{expr})"),
        Ty::Bool => format!("__render_str(if {expr} {{ \"true\" }} else {{ \"false\" }})"),
        _ => format!("__render_str(&format!(\"{{}}\", {expr}))"),
    }
}

/// The two helpers the rendered expressions call, mirroring `_runner.py`.
const RENDER_HELPERS: &str = r##"
fn __render_float(value: f64) -> String {
    if value.is_nan() { return "\"NaN\"".to_string(); }
    if value.is_infinite() {
        return if value > 0.0 { "\"Infinity\"".to_string() } else { "\"-Infinity\"".to_string() };
    }
    // Python pads an exponent to two digits and Rust does not, so the exponent is re-rendered
    // from its integer value on both sides rather than taken from either one's default.
    let text = format!("{:.8e}", value);
    let (mantissa, exponent) = text.split_once('e').expect("scientific notation has an exponent");
    format!("{}e{:+}", mantissa, exponent.parse::<i32>().expect("an integer exponent"))
}

fn __render_str(value: &str) -> String {
    // Matches `json.dumps(ensure_ascii=True)`: everything outside printable ASCII is escaped, and
    // anything above the basic plane becomes a surrogate pair.
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if ('\u{20}'..='\u{7e}').contains(&c) => out.push(c),
            c => {
                let point = c as u32;
                if point > 0xFFFF {
                    let adjusted = point - 0x10000;
                    out.push_str(&format!("\\u{:04x}", 0xD800 + (adjusted >> 10)));
                    out.push_str(&format!("\\u{:04x}", 0xDC00 + (adjusted & 0x3FF)));
                } else {
                    out.push_str(&format!("\\u{point:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}
"##;

// ---------------------------------------------------------------------------------------------
// Building the harness
// ---------------------------------------------------------------------------------------------

/// The statements that run one driver's calls and print one line per call.
fn calls_body(unit: &Unit, calls: &[Value], stem: &str) -> String {
    let mut body = String::new();
    let instance_accesses = instance_parameter_accesses(unit);
    for (index, entry) in calls.iter().enumerate() {
        let methods = entry["methods"].as_array().cloned().unwrap_or_default();
        let args = entry["args"].as_array().cloned().unwrap_or_default();

        if let Some(class_name) = entry.get("new").and_then(Value::as_str) {
            let class = class_named(unit, class_name)
                .unwrap_or_else(|| panic!("{stem}: {class_name} is not a class in this unit"));
            let rendered: Vec<String> = class
                .init
                .params
                .iter()
                .zip(&args)
                .map(|(p, v)| literal(&p.ty, v, unit))
                .collect();
            body.push_str(&format!(
                "    let mut __obj{index} = {class_name}::__compylr_new({})?;\n",
                rendered.join(", ")
            ));
            body.push_str(&method_lines(unit, class, &methods, index, stem));
            continue;
        }

        let name = entry["call"].as_str().expect("a call names its member");
        let function = function_named(unit, name)
            .unwrap_or_else(|| panic!("{stem}: {name} is not a function in this unit"));
        let mut rendered = Vec::with_capacity(function.params.len());
        for (position, (param, value)) in function.params.iter().zip(&args).enumerate() {
            let argument = literal(&param.ty, value, unit);
            if matches!(param.ty, Ty::Instance(_)) {
                let access = instance_accesses
                    .get(&(function.name.clone(), param.name.clone()))
                    .copied()
                    .unwrap_or(InstanceAccess::Shared);
                let mutable = matches!(access, InstanceAccess::Mutable);
                let binding = format!("__arg{index}_{position}");
                body.push_str(&format!(
                    "    let {}{binding} = {argument};\n",
                    if mutable { "mut " } else { "" }
                ));
                rendered.push(if mutable {
                    format!("&mut {binding}")
                } else {
                    format!("&{binding}")
                });
            } else {
                rendered.push(argument);
            }
        }
        let call = format!("{name}({})?", rendered.join(", "));

        if methods.is_empty() {
            body.push_str(&format!("    let __r{index} = {call};\n"));
            body.push_str(&format!(
                "    println!(\"{{}}\", {});\n",
                render_expr(&function.ret, &format!("__r{index}"), 0)
            ));
        } else {
            let Ty::Instance(class_name) = &function.ret else {
                panic!("{stem}: {name} declares methods but does not return an instance");
            };
            let class = class_named(unit, class_name)
                .unwrap_or_else(|| panic!("{stem}: {class_name} is not a class in this unit"));
            body.push_str(&format!("    let mut __obj{index} = {call};\n"));
            body.push_str(&method_lines(unit, class, &methods, index, stem));
        }
    }
    body
}

/// A `new` entry, or a call that returns an instance, renders as the list of its method results.
fn method_lines(unit: &Unit, class: &Class, methods: &[Value], index: usize, stem: &str) -> String {
    let mut body = format!("    let mut __parts{index}: Vec<String> = Vec::new();\n");
    for (step, method) in methods.iter().enumerate() {
        let pair = method.as_array().expect("a method is a name and arguments");
        let name = pair[0].as_str().expect("a method name");
        let args = pair[1].as_array().cloned().unwrap_or_default();
        let signature = class
            .methods
            .get(name)
            .unwrap_or_else(|| panic!("{stem}: {} has no method {name}", class.name));
        let rendered: Vec<String> = signature
            .params
            .iter()
            .zip(&args)
            .map(|(p, v)| literal(&p.ty, v, unit))
            .collect();
        body.push_str(&format!(
            "    let __m{index}_{step} = __obj{index}.{name}({})?;\n",
            rendered.join(", ")
        ));
        body.push_str(&format!(
            "    __parts{index}.push({});\n",
            render_expr(&signature.ret, &format!("__m{index}_{step}"), 0)
        ));
    }
    body.push_str(&format!(
        "    println!(\"[{{}}]\", __parts{index}.join(\",\"));\n"
    ));
    body
}

/// The inner attributes the generated crate root declares.
///
/// The harness replaces that root, so without these it would compile the same code under
/// different lint settings than the pipeline uses.
fn crate_attributes(lib: &str) -> String {
    lib.lines()
        .filter(|line| line.trim_start().starts_with("#!["))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Emit the group's crate, write a `main` around it, compile, run, and return what it printed.
///
/// Built on `execution.rs`'s pattern: the crate is written out and a `main.rs` added beside it, so
/// the code under test is compiled exactly as it ships rather than concatenated into a shape it
/// never takes.
fn run_group(label: &str, unit: &Unit, body: &str) -> String {
    let emitted = lookup("rust").unwrap().emit(unit).expect("must emit");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("differential_{label}"));
    let _ = std::fs::remove_dir_all(&dir);
    for (relative, contents) in &emitted {
        let path = dir.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).expect("scratch directory");
        std::fs::write(&path, contents).expect("write generated source");
    }

    let source_path = dir.join("src/main.rs");
    let binary_path = dir.join("program");
    // Warnings are denied, and the harness inherits the crate root's own allowances rather than
    // choosing its own. `main.rs` becomes the crate root here, so the generated `lib.rs`'s inner
    // attributes would otherwise not apply and this tier would be denying warnings the shipped
    // crate allows -- reporting the backend for output that builds cleanly as it is actually
    // built. Taking them from the emitted file rather than copying them also means that if the
    // backend ever narrows its allowances, this tier narrows with it.
    let attributes = crate_attributes(&emitted["src/lib.rs"]);
    let program = format!(
        "{attributes}\n\
         mod compat;\n\
         mod generated;\n\
         use compat::{{FastMap, FastSet, RuntimeError}};\n\
         use generated::*;\n\
         {RENDER_HELPERS}\n\
         fn main() -> Result<(), RuntimeError> {{\n{body}    Ok(())\n}}\n"
    );
    std::fs::write(&source_path, &program).expect("write the harness");

    let compile = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-D")
        .arg("warnings")
        .arg("-o")
        .arg(&binary_path)
        .arg(&source_path)
        .output()
        .expect("rustc must be available; it ships with the toolchain that runs these tests");
    assert!(
        compile.status.success(),
        "the harness for `{label}` did not compile:\n{}\n--- harness ---\n{}\n--- translated ---\n{}",
        String::from_utf8_lossy(&compile.stderr),
        program,
        emitted["src/generated.rs"]
    );

    let output = Command::new(&binary_path)
        .output()
        .expect("run the program");
    assert!(
        output.status.success(),
        "the program for `{label}` aborted, which means a failure escaped as a panic:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("output must be UTF-8")
}

/// What generated Rust prints for one fixture's driver.
///
/// `label` scopes the scratch directory. Cargo runs these tests in parallel and more than one
/// covers the same fixture, so a label taken from the fixture alone has two tests writing to and
/// deleting the same path.
fn translated_transcript(
    label: &str,
    stem: &str,
    loaded: &BTreeMap<String, drivers::Driver>,
) -> String {
    let group = fixture_groups()
        .into_iter()
        .find(|(_, stems)| stems.iter().any(|s| s == stem))
        .expect("every fixture belongs to a group");
    let unit = unit_from(&group.1);
    let calls = &loaded[stem].calls;
    let body = calls_body(&unit, calls, stem);
    run_group(&format!("{label}_{stem}"), &unit, &body)
        .trim_end()
        .to_string()
}

// ---------------------------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------------------------

#[test]
fn one_fixture_agrees_with_cpython() {
    if !rustc_available() {
        eprintln!("skipping: rustc is not installed");
        return;
    }
    let Some(loaded) = drivers::load_all() else {
        eprintln!("skipping: python3 is not installed");
        return;
    };
    let Some(expected) = drivers::interpreted_transcripts() else {
        eprintln!("skipping: python3 is not installed");
        return;
    };

    let actual = translated_transcript("one", "arithmetic", &loaded);
    assert_eq!(
        actual, expected["arithmetic"],
        "translated Rust disagreed with CPython for `arithmetic`"
    );
}

#[test]
fn a_disagreement_names_the_fixture_and_shows_both() {
    // The tier's own failure mode, checked rather than assumed: a corrupted expectation must
    // fail, and the message must carry enough to act on. A differential test that reported only
    // "not equal" would send you to run both halves by hand.
    let report = std::panic::catch_unwind(|| {
        let actual = "1\n2\n3";
        let expected = "1\n9\n3";
        compare("arithmetic", actual, expected);
    });
    let message = *report
        .expect_err("a disagreement must fail")
        .downcast::<String>()
        .expect("the failure carries a message");

    assert!(message.contains("arithmetic"), "{message}");
    assert!(message.contains("line 2"), "{message}");
    assert!(message.contains("translated: 2"), "{message}");
    assert!(message.contains("interpreted: 9"), "{message}");
}

/// Report a disagreement with everything needed to act on it.
fn compare(stem: &str, translated: &str, interpreted: &str) {
    if translated == interpreted {
        return;
    }
    let mut difference = String::new();
    let mut left = translated.lines();
    let mut right = interpreted.lines();
    let mut line = 0;
    loop {
        line += 1;
        match (left.next(), right.next()) {
            (None, None) => break,
            (a, b) if a == b => continue,
            (a, b) => {
                difference = format!(
                    "first difference at line {line}\n  translated: {}\n  interpreted: {}",
                    a.unwrap_or("<end of output>"),
                    b.unwrap_or("<end of output>")
                );
                break;
            }
        }
    }
    panic!(
        "`{stem}` translated and interpreted differently\n{difference}\n\
         --- translated ---\n{translated}\n--- interpreted ---\n{interpreted}"
    );
}

#[test]
fn the_whole_accepted_corpus_agrees_with_cpython() {
    if !rustc_available() {
        eprintln!("skipping: rustc is not installed");
        return;
    }
    let Some(loaded) = drivers::load_all() else {
        eprintln!("skipping: python3 is not installed");
        return;
    };
    let Some(expected) = drivers::interpreted_transcripts() else {
        eprintln!("skipping: python3 is not installed");
        return;
    };

    let stems = drivers::accepted_stems();
    assert!(!stems.is_empty(), "there must be accepted fixtures");
    for stem in &stems {
        let actual = translated_transcript("corpus", stem, &loaded);
        compare(stem, &actual, &expected[stem]);
    }
}

/// The Rust rendering of a value and the Python rendering of the same value agree, for every
/// shape a `Ty` can take.
///
/// A renderer written twice is a renderer written wrong. This is the same shape as the test that
/// keeps `runtime.rs`'s mirrored `IndexOrigin` in step with the IR's: the two definitions exist
/// for good reasons, and something has to hold them together.
#[test]
fn the_two_renderings_of_every_type_agree() {
    if !rustc_available() {
        eprintln!("skipping: rustc is not installed");
        return;
    }

    let table: Vec<(Ty, Value)> = vec![
        (Ty::Int, serde_json::json!(3)),
        (Ty::Int, serde_json::json!(-7)),
        (Ty::Int, serde_json::json!(0)),
        (Ty::Bool, serde_json::json!(true)),
        (Ty::Bool, serde_json::json!(false)),
        (Ty::Float, serde_json::json!(0.5)),
        (Ty::Float, serde_json::json!(-2.0)),
        // An integer written where a float is declared: the declared type is what renders.
        (Ty::Float, serde_json::json!(3)),
        (Ty::Float, serde_json::json!(0.020000000000000004)),
        (Ty::Str, serde_json::json!("hi")),
        (Ty::Str, serde_json::json!("a\"b\\c")),
        (Ty::Str, serde_json::json!("tab\there\nand newline")),
        // Escaped as `\uXXXX` on both sides, which is what settles `'a'` against `"a"`.
        (Ty::Str, serde_json::json!("héllo")),
        (Ty::Str, serde_json::json!("日本語")),
        (Ty::Str, serde_json::json!("")),
        (Ty::Unit, Value::Null),
        (Ty::List(Box::new(Ty::Int)), serde_json::json!([1, 2, 3])),
        (Ty::List(Box::new(Ty::Int)), serde_json::json!([])),
        (Ty::List(Box::new(Ty::Str)), serde_json::json!(["a", "é"])),
        (Ty::List(Box::new(Ty::Float)), serde_json::json!([1.5, 2])),
        (
            Ty::List(Box::new(Ty::List(Box::new(Ty::Int)))),
            serde_json::json!([[1, 2], []]),
        ),
        // Given out of order deliberately: a set has no order to preserve, so both sides sort.
        (
            Ty::Set(Box::new(Ty::Int)),
            serde_json::json!({"$set": [3, 1, 2]}),
        ),
        (
            Ty::Set(Box::new(Ty::Str)),
            serde_json::json!({"$set": ["b", "a"]}),
        ),
        // Insertion order differs from sorted order, which is the case that would be flaky if
        // either side rendered a mapping in the order it happened to hold it.
        (
            Ty::Dict(Box::new(Ty::Str), Box::new(Ty::Int)),
            serde_json::json!({"$dict": [["b", 2], ["a", 1]]}),
        ),
        (
            Ty::Dict(Box::new(Ty::Int), Box::new(Ty::Str)),
            serde_json::json!({"$dict": [[10, "x"], [9, "y"]]}),
        ),
        (
            Ty::Dict(Box::new(Ty::Str), Box::new(Ty::List(Box::new(Ty::Int)))),
            serde_json::json!({"$dict": [["k", [1, 2]]]}),
        ),
        (
            Ty::Tuple(vec![Ty::Int, Ty::Str]),
            serde_json::json!({"$tuple": [1, "a"]}),
        ),
        (
            Ty::Tuple(vec![Ty::Bool, Ty::Float, Ty::Int]),
            serde_json::json!({"$tuple": [true, 1.25, -1]}),
        ),
    ];

    let values: Vec<Value> = table.iter().map(|(t, v)| declared_value(t, v)).collect();
    let Some(expected) = drivers::python_renderings(&values) else {
        eprintln!("skipping: python3 is not installed");
        return;
    };

    // One crate, one program: the renderings need `compat`'s collection types, and the fixture
    // the unit is built from is irrelevant to what is being compared.
    let unit = unit_from(&["arithmetic".to_string()]);
    let mut body = String::new();
    for (index, (ty, value)) in table.iter().enumerate() {
        body.push_str(&format!(
            "    let __v{index}: {} = {};\n",
            rust_type(ty),
            literal(ty, value, &unit)
        ));
        body.push_str(&format!(
            "    println!(\"{{}}\", {});\n",
            render_expr(ty, &format!("__v{index}"), 0)
        ));
    }
    let printed = run_group("renderings", &unit, &body);
    let actual: Vec<&str> = printed.trim_end().lines().collect();

    assert_eq!(
        actual.len(),
        table.len(),
        "every row must render exactly one line"
    );
    for (index, (ty, value)) in table.iter().enumerate() {
        assert_eq!(
            actual[index], expected[index],
            "the two renderings of {ty:?} disagree for {value}"
        );
    }
}

#[test]
fn an_instance_has_no_transcript_of_its_own() {
    // Rendering an instance would mean reading fields a generated `#[pyclass]` does not expose,
    // so the driver format requires methods on any entry whose result is one. This is the
    // message someone gets if that rule is ever bypassed.
    let failure = std::panic::catch_unwind(|| {
        render_expr(&Ty::Instance("Counter".to_string()), "__x", 0);
    })
    .expect_err("rendering an instance must fail");
    let message = *failure
        .downcast::<String>()
        .expect("the failure carries a message");
    assert!(message.contains("Counter"), "{message}");
    assert!(message.contains("methods"), "{message}");
}
