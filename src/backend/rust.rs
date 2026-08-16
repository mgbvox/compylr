//! The Rust backend: IR in, Rust source out.
//!
//! Three decisions shape everything here, and each exists to remove a way of being subtly wrong.
//!
//! **Operands arrive pre-widened.** Lowering wraps integer operands in [`Expr::ToFloat`] wherever
//! Python promotes, so `a / b` on two integers reaches the backend as `TrueDiv(ToFloat(a),
//! ToFloat(b))`. The backend never re-derives promotion; it emits operands positionally.
//!
//! **Operator emission is type-directed by Rust, not by the backend.** The IR does not annotate
//! expressions with their types, so the backend cannot tell whether `+` means integer addition or
//! string concatenation by inspection. Rather than reimplement the type checker, arithmetic is
//! emitted as trait calls (`PyAdd::py_add(&(a), &(b))?`) and Rust selects the implementation.
//! An operand combination lowering would have rejected simply fails to compile.
//!
//! **Every emitted function is fallible.** Division by zero and overflow must be reportable, and
//! any body can contain either, so signatures are uniformly `Result<T, RuntimeError>` rather than
//! becoming fallible only when the backend judges that they might fail — a judgement that would
//! change a function's signature on an unrelated edit.
//!
//! Expressions are emitted fully parenthesized. Preserving the IR's grouping through a precedence
//! table would mean keeping that table correct forever; parentheses make it true by construction.

use std::fmt::Write as _;

use super::{Backend, BackendError};
use crate::ir::{BinOp, Expr, Function, Literal, Stmt, Ty, Unit};

/// The runtime helpers, embedded verbatim into generated crates.
///
/// The same text is compiled as `super::runtime` inside compylr, so the helpers are unit-tested
/// natively rather than only through generated code. A generated crate cannot depend on compylr
/// by path (this repo will not exist on a user's machine) and publishing a runtime crate to
/// depend on would mean release-managing a second crate for a few dozen lines.
pub const RUNTIME_SOURCE: &str = include_str!("runtime.rs");

/// Rust keywords, which Python allows as identifiers.
///
/// Most can be escaped as raw identifiers. `crate`, `self`, `Self`, and `super` cannot, so they
/// are suffixed instead; the rename is invisible because nothing outside the generated module
/// refers to these names by their Rust spelling.
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "dyn", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async",
    "await", "gen", "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try",
    "typeof", "unsized", "virtual", "yield",
];

/// Keywords that cannot be written as raw identifiers.
const UNRAWABLE_KEYWORDS: &[&str] = &["crate", "self", "Self", "super"];

/// The Rust spelling of a Python name.
pub fn rust_ident(name: &str) -> String {
    if UNRAWABLE_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

/// The Rust type a semantic IR type maps onto.
///
/// This mapping is the backend's alone. Nothing in `src/ir.rs` names a Rust type, which is what
/// lets a Go or TypeScript backend consume the same tree and choose differently.
pub fn rust_ty(ty: Ty) -> &'static str {
    match ty {
        Ty::Int => "i64",
        Ty::Float => "f64",
        Ty::Bool => "bool",
        Ty::Str => "String",
        Ty::Unit => "()",
    }
}

/// Render a string as a Rust string literal denoting exactly the same characters.
fn string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            // Other control characters have no readable escape; `\u{..}` is exact.
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render a float so that reading it back yields the identical bit pattern.
fn float_literal(value: f64) -> String {
    if value.is_nan() {
        // Not producible from Python source, but an IR built by hand can hold one.
        return "f64::NAN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "f64::INFINITY".to_string()
        } else {
            "f64::NEG_INFINITY".to_string()
        };
    }
    // `{:?}` on f64 produces the shortest representation that round-trips exactly, and always
    // includes a decimal point, so the literal is unambiguously floating point.
    format!("{value:?}f64")
}

/// Render an integer literal.
fn int_literal(value: i64) -> String {
    if value == i64::MIN {
        // `-9223372036854775808i64` does not parse: Rust reads it as negation of a literal that
        // is itself out of range.
        return "i64::MIN".to_string();
    }
    format!("{value}i64")
}

/// The Rust backend.
#[derive(Debug)]
pub struct RustBackend;

/// PyO3 version generated crates depend on.
///
/// Pinned to match this crate's own dependency: the bindings emitted here are written against
/// that API, so letting a generated crate float to a different major version would produce code
/// that does not compile.
pub const PYO3_VERSION: &str = "0.29.2";

impl Backend for RustBackend {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn emit_python_extension(&self, unit: &Unit) -> Result<String, BackendError> {
        super::bindings::emit_extension(unit)
    }

    fn build_manifest(&self, unit: &Unit) -> Result<String, BackendError> {
        Ok(super::bindings::cargo_manifest(unit, PYO3_VERSION))
    }

    fn emit(&self, unit: &Unit) -> Result<String, BackendError> {
        let mut out = String::new();
        out.push_str(&header());
        out.push_str("pub mod runtime {\n");
        for line in RUNTIME_SOURCE.lines() {
            if line.is_empty() {
                out.push('\n');
            } else {
                let _ = writeln!(out, "    {line}");
            }
        }
        out.push_str("}\n\n");

        // The translated functions live in their own module so nothing a user named can collide
        // with the binding layer that wraps them. A Python function called `to_py_err` is
        // perfectly legal, and without this it would clash with the helper of that name.
        out.push_str(GENERATED_MODULE_HEADER);
        for function in unit.functions() {
            for line in emit_function(function, unit)?.lines() {
                if line.is_empty() {
                    out.push('\n');
                } else {
                    let _ = writeln!(out, "    {line}");
                }
            }
            out.push('\n');
        }
        out.push_str("}\n");
        Ok(out)
    }
}

/// Opening of the module holding the translated functions.
pub const GENERATED_MODULE_HEADER: &str =
    "pub mod generated {\n    use super::runtime::{PyAdd, PyNum, RuntimeError, py_truediv};\n\n";

/// The preamble every generated file carries.
///
/// The `allow`s are deliberate rather than lazy. Full parenthesization guarantees
/// `unused_parens`; preserving the user's Python names guarantees `non_snake_case`; a function
/// that ignores a parameter guarantees `unused_variables`; and a unit whose functions are not all
/// reachable from one another guarantees `dead_code`. None of these indicate a defect in
/// generated code, and silencing them keeps a real warning visible.
fn header() -> String {
    "// Generated by compylr. Do not edit: regenerated from the IR on every rebuild.\n\
     #![allow(unused_parens, non_snake_case, unused_variables, dead_code, unused_imports)]\n\n"
        .to_string()
}

/// Emit one function, including its fallible signature.
fn emit_function(function: &Function, unit: &Unit) -> Result<String, BackendError> {
    let mut out = String::new();
    let params = function
        .params
        .iter()
        .map(|p| format!("{}: {}", rust_ident(&p.name), rust_ty(p.ty)))
        .collect::<Vec<_>>()
        .join(", ");

    let _ = writeln!(
        out,
        "pub fn {}({}) -> Result<{}, RuntimeError> {{",
        rust_ident(&function.name),
        params,
        rust_ty(function.ret)
    );

    let body = emit_body(function, unit)?;
    out.push_str(&body);
    out.push_str("}\n");
    Ok(out)
}

/// Emit a function body, ending in a tail expression rather than a `return`.
///
/// The final statement becomes the tail so the generated function has no unreachable trailing
/// expression, which would be a warning in code that must compile clean.
fn emit_body(function: &Function, unit: &Unit) -> Result<String, BackendError> {
    let mut out = String::new();
    let last = function.body.len().saturating_sub(1);

    for (index, stmt) in function.body.iter().enumerate() {
        let is_last = index == last;
        match stmt {
            Stmt::Bind { name, ty, value } => {
                let value = emit_expr(value, unit, *ty)?;
                let _ = writeln!(
                    out,
                    "    let {}: {} = {};",
                    rust_ident(name),
                    rust_ty(*ty),
                    value
                );
            }
            Stmt::Return(expr) => {
                let value = emit_expr(expr, unit, function.ret)?;
                if is_last {
                    let _ = writeln!(out, "    Ok({value})");
                } else {
                    let _ = writeln!(out, "    return Ok({value});");
                }
            }
            Stmt::ReturnUnit => {
                if is_last {
                    out.push_str("    Ok(())\n");
                } else {
                    out.push_str("    return Ok(());\n");
                }
            }
        }
    }

    // A body that falls off the end without returning is only well-formed for a unit function.
    // Lowering should not produce anything else; if it does, that is a compylr defect and saying
    // so beats emitting Rust that fails to compile with an unrelated message.
    let falls_through = match function.body.last() {
        None => true,
        Some(Stmt::Bind { .. }) => true,
        Some(Stmt::Return(_) | Stmt::ReturnUnit) => false,
    };
    if falls_through {
        if function.ret != Ty::Unit {
            return Err(BackendError::Unsupported {
                detail: format!(
                    "function '{}' declares a return type of '{}' but its body does not return",
                    function.name,
                    function.ret.python_name()
                ),
            });
        }
        out.push_str("    Ok(())\n");
    }
    Ok(out)
}

/// Emit an expression, fully parenthesized.
///
/// `expected` is the type the surrounding context wants. It is used for exactly one thing:
/// deciding whether an owned `String` has to be produced where a value is consumed, so that a
/// string parameter used twice is not moved on first use.
fn emit_expr(expr: &Expr, unit: &Unit, expected: Ty) -> Result<String, BackendError> {
    Ok(match expr {
        Expr::Literal(literal) => match literal {
            Literal::Int(value) => int_literal(*value),
            Literal::Bool(value) => value.to_string(),
            Literal::Str(value) => format!("String::from({})", string_literal(value)),
            Literal::Float(_) => float_literal(
                literal
                    .as_f64()
                    .expect("a float literal always converts back to f64"),
            ),
        },
        Expr::Name(name) => {
            let name = rust_ident(name);
            if expected == Ty::Str {
                // Cloning rather than moving: the same name may be read again later, and Python
                // has no notion of a value being consumed by being used.
                format!("{name}.clone()")
            } else {
                name
            }
        }
        Expr::Neg(inner) => {
            let inner = emit_expr(inner, unit, expected)?;
            format!("PyNum::py_neg(&({inner}))?")
        }
        Expr::ToFloat(inner) => {
            // The operand is an integer expression; `expected` describes the float context it is
            // being widened into, so it must not be propagated inward.
            let inner = emit_expr(inner, unit, Ty::Int)?;
            format!("(({inner}) as f64)")
        }
        Expr::Binary { op, left, right } => emit_binary(*op, left, right, unit, expected)?,
        Expr::Call { callee, args } => {
            let signature = unit.get(callee).ok_or_else(|| BackendError::Unsupported {
                detail: format!("call to '{callee}', which is not in the unit"),
            })?;
            if signature.params.len() != args.len() {
                return Err(BackendError::Unsupported {
                    detail: format!(
                        "call to '{}' passes {} arguments but it takes {}",
                        callee,
                        args.len(),
                        signature.params.len()
                    ),
                });
            }
            // Argument types come from the callee's signature, which the unit already holds —
            // the one place the backend can learn an expression's type without inferring it.
            let rendered = args
                .iter()
                .zip(&signature.params)
                .map(|(arg, param)| emit_expr(arg, unit, param.ty))
                .collect::<Result<Vec<_>, _>>()?;
            format!("{}({})?", rust_ident(callee), rendered.join(", "))
        }
    })
}

/// Emit a binary operation as a trait call, letting Rust choose the implementation by type.
fn emit_binary(
    op: BinOp,
    left: &Expr,
    right: &Expr,
    unit: &Unit,
    expected: Ty,
) -> Result<String, BackendError> {
    // Comparisons yield a bool regardless of operand type, so the expected type says nothing
    // about the operands. They are emitted by reference, which both avoids moving a string and
    // works uniformly for every comparable type.
    if op.is_comparison() {
        let left = emit_expr(left, unit, Ty::Unit)?;
        let right = emit_expr(right, unit, Ty::Unit)?;
        let symbol = match op {
            BinOp::Eq => "==",
            BinOp::NotEq => "!=",
            BinOp::Lt => "<",
            BinOp::LtE => "<=",
            BinOp::Gt => ">",
            BinOp::GtE => ">=",
            _ => unreachable!("is_comparison covers exactly these"),
        };
        return Ok(format!("((&({left})) {symbol} (&({right})))"));
    }

    // True division's operands are always floats: lowering inserted the promotion nodes.
    if op == BinOp::TrueDiv {
        let left = emit_expr(left, unit, Ty::Float)?;
        let right = emit_expr(right, unit, Ty::Float)?;
        return Ok(format!("py_truediv(&({left}), &({right}))?"));
    }

    // Arithmetic operands share the expression's own type, except that a string operand must not
    // be cloned here: the trait takes a reference.
    let operand = if expected == Ty::Str {
        Ty::Unit
    } else {
        expected
    };
    let left = emit_expr(left, unit, operand)?;
    let right = emit_expr(right, unit, operand)?;
    let call = match op {
        BinOp::Add => "PyAdd::py_add",
        BinOp::Sub => "PyNum::py_sub",
        BinOp::Mul => "PyNum::py_mul",
        BinOp::FloorDiv => "PyNum::py_floordiv",
        BinOp::Mod => "PyNum::py_mod",
        _ => unreachable!("comparisons and true division are handled above"),
    };
    Ok(format!("{call}(&({left}), &({right}))?"))
}
