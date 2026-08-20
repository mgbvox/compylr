//! A corpus of IR units every implemented backend must render.
//!
//! Authored as IR, not as Python. That is the whole point: a backend's job is to render the IR,
//! and a corpus written in one source language can only cover what that language happens to
//! produce. `python/fixtures/accepted/` is a good test of the Python *frontend* and a poor test
//! of a backend, because a tree Python cannot express is a tree the corpus would never contain —
//! and the modes Python cannot write were exactly where a backend could have been silently wrong.
//!
//! Backends are enumerated from the registry rather than listed here. A hand-maintained list is
//! how the fixture lists in this repo once drifted and hid a real defect; deriving it means a
//! backend added tomorrow is covered today.

use std::collections::BTreeMap;

use compylr::ir::{
    Attribute, BinOp, Class, DivMode, Expr, Function, Literal, Param, RemSign, Rounding, Stmt, Ty,
    Unit,
};
use compylr::span::Span;

fn param(name: &str, ty: Ty) -> Param {
    Param {
        name: name.to_string(),
        ty,
    }
}

fn function(name: &str, params: Vec<Param>, ret: Ty, body: Vec<Stmt>) -> Function {
    Function {
        name: name.to_string(),
        params,
        ret,
        body,
        doc: None,
        span: Span::default(),
    }
}

fn int(value: i64) -> Expr {
    Expr::Literal(Literal::Int(value))
}

fn binary(op: BinOp, left: Expr, right: Expr) -> Expr {
    Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn unit_of(functions: Vec<Function>, classes: Vec<Class>) -> Unit {
    let mut unit = Unit::new();
    for class in classes {
        unit.add_class(class).expect("corpus names must be unique");
    }
    for function in functions {
        unit.add_function(function)
            .expect("corpus names must be unique");
    }
    unit
}

/// Every scalar and collection type, in a signature and in a binding.
fn types() -> Unit {
    unit_of(
        vec![
            function(
                "scalars",
                vec![
                    param("i", Ty::Int),
                    param("f", Ty::Float),
                    param("b", Ty::Bool),
                    param("s", Ty::Str),
                ],
                Ty::Str,
                vec![Stmt::Return(Expr::name("s"))],
            ),
            function("nothing", vec![], Ty::Unit, vec![Stmt::ReturnUnit]),
            function(
                "collections",
                vec![
                    param("xs", Ty::List(Box::new(Ty::Int))),
                    param(
                        "d",
                        Ty::Dict(Box::new(Ty::Str), Box::new(Ty::List(Box::new(Ty::Int)))),
                    ),
                    param("st", Ty::Set(Box::new(Ty::Str))),
                    param("t", Ty::Tuple(vec![Ty::Int, Ty::Str])),
                ],
                Ty::Int,
                vec![Stmt::Return(Expr::Len(Box::new(Expr::name("xs"))))],
            ),
        ],
        vec![],
    )
}

/// Every arithmetic and comparison form, including the modes no Python program can produce.
fn operators() -> Unit {
    let mut body = vec![Stmt::Bind {
        name: "acc".to_string(),
        ty: Ty::Int,
        value: binary(BinOp::Add, Expr::name("a"), Expr::name("b")),
    }];
    for op in [
        BinOp::Sub,
        BinOp::Mul,
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardNegInf),
        },
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
        },
        BinOp::Rem {
            sign: RemSign::Divisor,
        },
        BinOp::Rem {
            sign: RemSign::Dividend,
        },
    ] {
        body.push(Stmt::Assign {
            name: "acc".to_string(),
            ty: Ty::Int,
            value: binary(op, Expr::name("acc"), Expr::name("b")),
        });
    }
    body.push(Stmt::Return(Expr::name("acc")));

    let mut comparisons = Vec::new();
    for (index, op) in [
        BinOp::Eq,
        BinOp::NotEq,
        BinOp::Lt,
        BinOp::LtE,
        BinOp::Gt,
        BinOp::GtE,
    ]
    .into_iter()
    .enumerate()
    {
        comparisons.push(Stmt::Bind {
            name: format!("c{index}"),
            ty: Ty::Bool,
            value: binary(op, Expr::name("a"), Expr::name("b")),
        });
    }
    comparisons.push(Stmt::Return(Expr::Not(Box::new(Expr::name("c0")))));

    unit_of(
        vec![
            function(
                "arithmetic",
                vec![param("a", Ty::Int), param("b", Ty::Int)],
                Ty::Int,
                body,
            ),
            function(
                "comparisons",
                vec![param("a", Ty::Int), param("b", Ty::Int)],
                Ty::Bool,
                comparisons,
            ),
            function(
                "promotion",
                vec![param("a", Ty::Int), param("b", Ty::Int)],
                Ty::Float,
                vec![Stmt::Return(binary(
                    BinOp::Div {
                        mode: DivMode::Exact,
                    },
                    Expr::name("a").to_float(),
                    Expr::name("b").to_float(),
                ))],
            ),
            function(
                "negation",
                vec![param("a", Ty::Int)],
                Ty::Int,
                vec![Stmt::Return(Expr::Neg(Box::new(Expr::name("a"))))],
            ),
            function(
                "literals",
                vec![],
                Ty::Float,
                vec![
                    Stmt::Bind {
                        name: "flag".to_string(),
                        ty: Ty::Bool,
                        value: Expr::Literal(Literal::Bool(true)),
                    },
                    Stmt::Bind {
                        name: "text".to_string(),
                        ty: Ty::Str,
                        value: Expr::Literal(Literal::Str("corpus".to_string())),
                    },
                    Stmt::Return(Expr::Literal(Literal::float(1.5))),
                ],
            ),
        ],
        vec![],
    )
}

/// Every collection literal, read, membership test, and mutation.
fn collections() -> Unit {
    unit_of(
        vec![
            function(
                "literals",
                vec![],
                Ty::Int,
                vec![
                    Stmt::Bind {
                        name: "xs".to_string(),
                        ty: Ty::List(Box::new(Ty::Int)),
                        value: Expr::ListLit(vec![int(1), int(2)]),
                    },
                    Stmt::Bind {
                        name: "d".to_string(),
                        ty: Ty::Dict(Box::new(Ty::Str), Box::new(Ty::Int)),
                        value: Expr::DictLit(vec![(
                            Expr::Literal(Literal::Str("k".to_string())),
                            int(1),
                        )]),
                    },
                    Stmt::Bind {
                        name: "st".to_string(),
                        ty: Ty::Set(Box::new(Ty::Int)),
                        value: Expr::SetLit(vec![int(3)]),
                    },
                    Stmt::Bind {
                        name: "t".to_string(),
                        ty: Ty::Tuple(vec![Ty::Int, Ty::Str]),
                        value: Expr::TupleLit(vec![
                            int(4),
                            Expr::Literal(Literal::Str("s".to_string())),
                        ]),
                    },
                    Stmt::Append {
                        sequence: Expr::name("xs"),
                        value: int(5),
                    },
                    Stmt::SetItem {
                        collection: Expr::name("d"),
                        index: Expr::Literal(Literal::Str("k".to_string())),
                        value: int(6),
                    },
                    Stmt::Bind {
                        name: "present".to_string(),
                        ty: Ty::Bool,
                        value: Expr::Contains {
                            value: Box::new(int(1)),
                            container: Box::new(Expr::name("xs")),
                        },
                    },
                    Stmt::Bind {
                        name: "first".to_string(),
                        ty: Ty::Int,
                        value: Expr::Subscript {
                            base: Box::new(Expr::name("xs")),
                            index: Box::new(int(0)),
                        },
                    },
                    Stmt::Bind {
                        name: "second".to_string(),
                        ty: Ty::Int,
                        value: Expr::TupleIndex {
                            base: Box::new(Expr::name("t")),
                            position: 0,
                        },
                    },
                    Stmt::Return(binary(
                        BinOp::Add,
                        Expr::name("first"),
                        Expr::name("second"),
                    )),
                ],
            ),
            function(
                "iterate",
                vec![param("xs", Ty::List(Box::new(Ty::Int)))],
                Ty::Int,
                vec![
                    Stmt::Bind {
                        name: "total".to_string(),
                        ty: Ty::Int,
                        value: int(0),
                    },
                    Stmt::For {
                        name: "x".to_string(),
                        ty: Ty::Int,
                        iter: Expr::name("xs"),
                        body: vec![Stmt::Assign {
                            name: "total".to_string(),
                            ty: Ty::Int,
                            value: binary(BinOp::Add, Expr::name("total"), Expr::name("x")),
                        }],
                    },
                    Stmt::Return(Expr::name("total")),
                ],
            ),
        ],
        vec![],
    )
}

/// Every control-flow form, and the counted-iteration expression.
fn control_flow() -> Unit {
    unit_of(
        vec![function(
            "shapes",
            vec![param("n", Ty::Int)],
            Ty::Int,
            vec![
                Stmt::Bind {
                    name: "total".to_string(),
                    ty: Ty::Int,
                    value: int(0),
                },
                Stmt::For {
                    name: "i".to_string(),
                    ty: Ty::Int,
                    iter: Expr::Range {
                        start: Box::new(int(0)),
                        stop: Box::new(Expr::name("n")),
                        step: Box::new(int(2)),
                    },
                    body: vec![
                        Stmt::If {
                            test: binary(BinOp::Lt, Expr::name("i"), int(0)),
                            then: vec![Stmt::Continue],
                            otherwise: vec![Stmt::Assign {
                                name: "total".to_string(),
                                ty: Ty::Int,
                                value: binary(BinOp::Add, Expr::name("total"), Expr::name("i")),
                            }],
                        },
                        Stmt::If {
                            test: binary(BinOp::Gt, Expr::name("total"), int(100)),
                            then: vec![Stmt::Break],
                            otherwise: vec![],
                        },
                    ],
                },
                Stmt::While {
                    test: binary(BinOp::Lt, Expr::name("total"), int(0)),
                    body: vec![Stmt::Assign {
                        name: "total".to_string(),
                        ty: Ty::Int,
                        value: binary(BinOp::Add, Expr::name("total"), int(1)),
                    }],
                },
                Stmt::Return(Expr::name("total")),
            ],
        )],
        vec![],
    )
}

/// A class: attributes, a constructor, a mutating method, and every form that reaches one.
fn classes() -> Unit {
    let init = function(
        "__init__",
        vec![param("start", Ty::Int)],
        Ty::Unit,
        vec![
            Stmt::SetAttr {
                object: Expr::name("self"),
                name: "count".to_string(),
                ty: Ty::Int,
                value: Expr::name("start"),
            },
            Stmt::SetAttr {
                object: Expr::name("self"),
                name: "seen".to_string(),
                ty: Ty::List(Box::new(Ty::Int)),
                value: Expr::ListLit(vec![]),
            },
            Stmt::ReturnUnit,
        ],
    );

    let bump = function(
        "bump",
        vec![param("by", Ty::Int)],
        Ty::Unit,
        vec![
            Stmt::SetAttr {
                object: Expr::name("self"),
                name: "count".to_string(),
                ty: Ty::Int,
                value: binary(
                    BinOp::Add,
                    Expr::Attribute {
                        object: Box::new(Expr::name("self")),
                        name: "count".to_string(),
                    },
                    Expr::name("by"),
                ),
            },
            Stmt::Append {
                sequence: Expr::Attribute {
                    object: Box::new(Expr::name("self")),
                    name: "seen".to_string(),
                },
                value: Expr::name("by"),
            },
            Stmt::ReturnUnit,
        ],
    );

    let total = function(
        "total",
        vec![],
        Ty::Int,
        vec![Stmt::Return(Expr::Attribute {
            object: Box::new(Expr::name("self")),
            name: "count".to_string(),
        })],
    );

    let counter = Class {
        name: "Counter".to_string(),
        attributes: vec![
            Attribute {
                name: "count".to_string(),
                ty: Ty::Int,
            },
            Attribute {
                name: "seen".to_string(),
                ty: Ty::List(Box::new(Ty::Int)),
            },
        ],
        init,
        methods: BTreeMap::from([("bump".to_string(), bump), ("total".to_string(), total)]),
        doc: None,
        span: Span::default(),
    };

    unit_of(
        vec![function(
            "use_counter",
            vec![param("n", Ty::Int)],
            Ty::Int,
            vec![
                Stmt::Bind {
                    name: "c".to_string(),
                    ty: Ty::Instance("Counter".to_string()),
                    value: Expr::Construct {
                        class: "Counter".to_string(),
                        args: vec![int(0)],
                    },
                },
                // A method call whose value is discarded: the form a backend must emit as a
                // statement rather than as an expression.
                Stmt::Effect(Expr::MethodCall {
                    receiver: Box::new(Expr::name("c")),
                    class: Some("Counter".to_string()),
                    method: "bump".to_string(),
                    args: vec![Expr::name("n")],
                }),
                Stmt::Return(Expr::MethodCall {
                    receiver: Box::new(Expr::name("c")),
                    class: Some("Counter".to_string()),
                    method: "total".to_string(),
                    args: vec![],
                }),
            ],
        )],
        vec![counter],
    )
}

/// A call between two functions in one unit, and a documented function.
fn calls() -> Unit {
    unit_of(
        vec![
            function(
                "helper",
                vec![param("n", Ty::Int)],
                Ty::Int,
                vec![Stmt::Return(binary(BinOp::Mul, Expr::name("n"), int(2)))],
            ),
            Function {
                doc: Some("Documented, so a backend that carries docs is exercised.".to_string()),
                ..function(
                    "caller",
                    vec![param("n", Ty::Int)],
                    Ty::Int,
                    vec![Stmt::Return(Expr::Call {
                        callee: "helper".to_string(),
                        args: vec![Expr::name("n")],
                    })],
                )
            },
        ],
        vec![],
    )
}

/// The corpus, by name.
fn corpus() -> Vec<(&'static str, Unit)> {
    vec![
        ("types", types()),
        ("operators", operators()),
        ("collections", collections()),
        ("control-flow", control_flow()),
        ("classes", classes()),
        ("calls", calls()),
    ]
}

#[test]
fn every_corpus_entry_is_well_formed() {
    for (name, unit) in corpus() {
        compylr_core::verify::verify(&unit)
            .unwrap_or_else(|error| panic!("corpus entry '{name}' is malformed: {error}"));
    }
}

/// Every implemented backend must render every entry.
///
/// The backend list comes from the registry, never from a list here. Adding a target tomorrow
/// puts it under this test today, which is the only version of "the corpus covers every backend"
/// that stays true without somebody remembering.
#[test]
fn every_implemented_backend_renders_the_whole_corpus() {
    let backends = compylr::backend::implemented_names();
    assert!(!backends.is_empty(), "at least one backend must compile");

    for backend_name in &backends {
        let backend = compylr::backend::lookup(backend_name).expect("the registry listed it");
        for (name, unit) in corpus() {
            let files = backend.emit(&unit).unwrap_or_else(|error| {
                panic!("the '{backend_name}' backend cannot render corpus entry '{name}': {error}")
            });
            assert!(
                !files.is_empty(),
                "'{backend_name}' rendered '{name}' as no files at all"
            );
        }
    }
}

/// Rendering is not enough: the result has to build.
///
/// A backend can emit plausible text for a form it does not really support — that is exactly how
/// tuple indexing once shipped emitting a call to a helper that did not exist. Only handing the
/// output to the target's own compiler settles it.
#[test]
fn every_corpus_entry_compiles_for_the_rust_backend() {
    use std::process::Command;

    let backend = compylr::backend::lookup("rust").expect("the shipped backend");
    for (name, unit) in corpus() {
        let files = backend.post_process(backend.emit(&unit).expect("must emit"));
        let dir =
            std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("conf_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        for (relative, contents) in &files {
            let path = dir.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).expect("scratch directory");
            std::fs::write(&path, contents).expect("write generated source");
        }

        let output = Command::new("rustc")
            .args(["--edition", "2024", "--crate-type", "lib", "-o"])
            .arg(dir.join("libcorpus.rlib"))
            .arg(dir.join("src/lib.rs"))
            .output()
            .expect("rustc ships with the toolchain running these tests");
        assert!(
            output.status.success(),
            "corpus entry '{name}' emitted Rust that does not compile:\n{}\n--- generated ---\n{}",
            String::from_utf8_lossy(&output.stderr),
            files["src/generated.rs"]
        );
    }
}

/// A hand-built unit has no source language, so it must not be blocked by a guarantee check.
#[test]
fn the_corpus_needs_no_source_language() {
    for (name, unit) in corpus() {
        assert!(
            unit.origin().is_none(),
            "corpus entry '{name}' must not claim a frontend"
        );
        for backend_name in compylr::backend::implemented_names() {
            let backend = compylr::backend::lookup(&backend_name).unwrap();
            assert!(compylr_core::negotiation::negotiate(&unit, backend).is_ok());
        }
    }
}

/// Every IR node form must appear somewhere in the corpus.
///
/// Read off the IR's own source rather than from a list, so a variant added tomorrow fails this
/// test until it is covered. The alternative is a corpus that quietly stops keeping pace with the
/// model it exists to exercise — which is how the fixture lists in this repo drifted before.
#[test]
fn the_corpus_covers_every_ir_node_form() {
    let serialized: String = corpus()
        .iter()
        .map(|(_, unit)| unit.to_json().expect("corpus entries serialize"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut missing = Vec::new();
    for variant in ir_variants() {
        // Serde tags externally, so a variant carrying data is a JSON *key* — `"Int":` — and one
        // carrying none is a bare string — `"Int"`. Distinguishing the two matters: `Ty::Int` and
        // `Literal::Int` share a name, and matching on the name alone would let either stand in
        // for the other. The quoting also keeps `Return` from matching `ReturnUnit`.
        let token = if variant.carries_data {
            format!("\"{}\":", variant.name)
        } else {
            format!("\"{}\"", variant.name)
        };
        if !serialized.contains(&token) {
            missing.push(format!("{}::{}", variant.enum_name, variant.name));
        }
    }
    assert!(
        missing.is_empty(),
        "these IR forms are not exercised by any corpus entry: {}",
        missing.join(", ")
    );
}

/// One variant of an IR enum, as declared in the source.
struct IrVariant {
    enum_name: String,
    name: String,
    /// Whether the variant carries a payload, which decides how serde writes it.
    carries_data: bool,
}

/// The variants of the IR's four central enums, read from its source.
fn ir_variants() -> Vec<IrVariant> {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/crates/compylr-ir/src/ir.rs"
    ))
    .expect("the IR source must be readable");

    let mut found = Vec::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("pub enum ") {
            let name = rest.trim_end_matches(" {").trim();
            current = ["Ty", "Expr", "Stmt", "Literal"]
                .contains(&name)
                .then(|| name.to_string());
            continue;
        }
        if line == "}" {
            current = None;
            continue;
        }
        let Some(enum_name) = &current else { continue };
        // Variants are indented exactly four spaces and start with a capital.
        let Some(rest) = line.strip_prefix("    ") else {
            continue;
        };
        if rest.starts_with(' ') || !rest.starts_with(|c: char| c.is_ascii_uppercase()) {
            continue;
        }
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        let after = rest[name.len()..].trim_start();
        found.push(IrVariant {
            enum_name: enum_name.clone(),
            name,
            carries_data: after.starts_with('(') || after.starts_with('{'),
        });
    }

    assert!(
        found.len() > 30,
        "the variant scan found only {} forms, so it has probably broken",
        found.len()
    );
    found
}

/// The pieces a caller reaches that nothing else exercises.
///
/// `pairs()` on each registry answers "what can compylr do?", which is a question a CLI or a
/// diagnostic asks and no other test does.
#[test]
fn every_registry_can_enumerate_itself() {
    let backends = compylr::backend::names();
    let frontends = compylr::frontend::names();
    let bridges = compylr::bridge_registry::pairs();
    let directed = compylr_registry::passes::pairs();

    assert!(backends.contains(&"rust"));
    assert!(frontends.contains(&"python"));
    assert!(bridges.contains(&("python".to_string(), "rust".to_string())));
    // Empty, and asserted as such: a directed pass registered without anyone noticing would
    // change what runs for a pair, and this is the only place that would say so.
    assert!(
        directed.is_empty(),
        "a pair-directed pass appeared without a test: {directed:?}"
    );

    // Every bridged pair must name languages both registries know, or the bridge is unreachable.
    for (source, target) in &bridges {
        assert!(frontends.contains(&source.as_str()), "{source}");
        assert!(backends.contains(&target.as_str()), "{target}");
    }
}
