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

use compylr_core::backend::{Backend, BackendError, GeneratedFiles};
use std::collections::BTreeSet;

use compylr_ir::{
    BinOp, Class, DivMode, Expr, Function, Literal, RemSign, Rounding, Stmt, Ty, Unit,
    returns_on_all_paths,
};

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
pub fn rust_ty(ty: &Ty) -> String {
    match ty {
        Ty::Int => "i64".to_string(),
        Ty::Float => "f64".to_string(),
        Ty::Bool => "bool".to_string(),
        Ty::Str => "String".to_string(),
        Ty::Unit => "()".to_string(),
        Ty::List(element) => format!("Vec<{}>", rust_ty(element)),
        Ty::Dict(key, value) => format!("HashMap<{}, {}>", rust_ty(key), rust_ty(value)),
        Ty::Set(element) => format!("HashSet<{}>", rust_ty(element)),
        // A class emits a struct of the same name, so the instance type is spelled the same way.
        Ty::Instance(class) => rust_ident(class),
        Ty::Tuple(elements) => {
            let inner: Vec<String> = elements.iter().map(rust_ty).collect();
            // A one-element tuple needs the trailing comma, or it is just a parenthesised type.
            if inner.len() == 1 {
                format!("({},)", inner[0])
            } else {
                format!("({})", inner.join(", "))
            }
        }
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

impl Backend for RustBackend {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn emit(&self, unit: &Unit) -> Result<GeneratedFiles, BackendError> {
        let mut functions = String::new();
        // Classes first: a reader opening this file wants the shapes before the operations, and a
        // free function may well take one as a parameter.
        for class in unit.classes() {
            functions.push_str(&emit_class(class, unit)?);
            functions.push('\n');
        }
        for function in unit.functions() {
            functions.push_str(&emit_function(function, unit)?);
            functions.push('\n');
        }

        Ok(GeneratedFiles::from([
            (LIB_PATH.to_string(), emit_crate_root(unit)),
            (GENERATED_PATH.to_string(), emit_generated(&functions)),
            (COMPAT_PATH.to_string(), RUNTIME_SOURCE.to_string()),
        ]))
    }
}

/// Path of the crate root.
pub const LIB_PATH: &str = "src/lib.rs";
/// Path of the file holding the translated functions, and nothing else.
pub const GENERATED_PATH: &str = "src/generated.rs";
/// Path of the file holding the Python-semantics helpers.
pub const COMPAT_PATH: &str = "src/compat.rs";

/// The crate root: lint allowances and module declarations, and nothing that grows.
///
/// Deliberately constant-size. The wrappers grow by two items per compiled function, so keeping
/// them here would make the file described as "lean" the one that grows fastest.
///
/// The lint allowances are inner attributes at the crate root, and lint attributes are inherited
/// by items in nested modules — which is what lets `generated.rs` hold the translated functions
/// and nothing else, not even an `allow`.
fn emit_crate_root(_unit: &Unit) -> String {
    "// Generated by compylr. Do not edit: regenerated from the IR on every rebuild.\n\
     #![allow(unused_parens, non_snake_case, unused_variables, dead_code, unused_imports, unused_assignments)]\n\
     \n\
     pub mod compat;\n\
     pub mod generated;\n"
        .to_string()
}

/// The translated functions, with the imports they need and nothing else.
fn emit_generated(functions: &str) -> String {
    format!(
        "//! Translated by compylr.\n\
         \n\
         use std::collections::{{HashMap, HashSet}};\n\
         \n\
         use crate::compat::{{\n\
         {}PyAdd, PyContains, PyIterate, PyLen, PyNum, PySetItem, RuntimeError, div_exact,\n\
         {}py_subscript,\n\
         }};\n\
         \n\
         {functions}",
        "    ", "    "
    )
}

/// Render a docstring as Rust doc-comment lines.
///
/// The generated source is written to disk for people to read, and a translated function stripped
/// of the explanation its author wrote is harder to check against the original than it needs to
/// be. A host bridge that lifts doc comments onto the exposed function gets the documentation
/// across for free as a side benefit, but that is the bridge's business, not this crate's.
///
/// The text is arbitrary user input, so it is made comment-safe rather than trusted: every line is
/// prefixed individually, and carriage returns are dropped so nothing can escape the comment and
/// be read as code.
fn doc_comment(doc: &str) -> String {
    let mut out = String::new();
    for line in doc.replace('\r', "").lines() {
        if line.is_empty() {
            out.push_str("///\n");
        } else {
            let _ = writeln!(out, "/// {line}");
        }
    }
    // A docstring that is entirely blank would otherwise emit nothing, leaving a stray attribute
    // position; emitting one empty line keeps the output well-formed.
    if out.is_empty() {
        out.push_str("///\n");
    }
    out
}

/// Emit one function, including its fallible signature.
fn emit_function(function: &Function, unit: &Unit) -> Result<String, BackendError> {
    let mut out = String::new();
    if let Some(doc) = &function.doc {
        out.push_str(&doc_comment(doc));
    }
    // A parameter the body assigns to is declared `mut`. This changes nothing a caller sees —
    // `mut` on a parameter is a property of the binding, not of the signature — so whatever
    // wraps this function for a host language is unaffected.
    let params = function
        .params
        .iter()
        .map(|p| {
            let mutable = if is_assigned(&function.body, &p.name, unit) {
                "mut "
            } else {
                ""
            };
            format!("{mutable}{}: {}", rust_ident(&p.name), rust_ty(&p.ty))
        })
        .collect::<Vec<_>>()
        .join(", ");

    let _ = writeln!(
        out,
        "pub fn {}({}) -> Result<{}, RuntimeError> {{",
        rust_ident(&function.name),
        params,
        rust_ty(&function.ret)
    );

    let body = emit_body(function, unit)?;
    out.push_str(&body);
    out.push_str("}\n");
    Ok(out)
}

/// Emit a class: a struct of its attributes and one implementation block of its methods.
fn emit_class(class: &Class, unit: &Unit) -> Result<String, BackendError> {
    let mut out = String::new();
    if let Some(doc) = &class.doc {
        out.push_str(&doc_comment(doc));
    }
    let name = rust_ident(&class.name);
    // `Clone` so an instance can be passed to a free function without being consumed, on the same
    // terms as every other non-copyable value here.
    out.push_str("#[derive(Clone)]\n");
    let _ = writeln!(out, "pub struct {name} {{");
    for attribute in &class.attributes {
        let _ = writeln!(
            out,
            "    pub {}: {},",
            rust_ident(&attribute.name),
            rust_ty(&attribute.ty)
        );
    }
    out.push_str("}\n\n");

    let mutating = mutating_methods(class);

    let _ = writeln!(out, "impl {name} {{");
    // The constructor is named rather than spelled `new`, so a class with a method called `new`
    // cannot collide with it.
    out.push_str(&emit_constructor(class, unit)?);
    for method in class.methods.values() {
        out.push('\n');
        out.push_str(&emit_method(
            method,
            class,
            unit,
            mutating.contains(&method.name),
        )?);
    }
    out.push_str("}\n");
    Ok(out)
}

/// Emit `__init__` as an associated constructor.
fn emit_constructor(class: &Class, unit: &Unit) -> Result<String, BackendError> {
    let mut out = String::new();
    if let Some(doc) = &class.init.doc {
        out.push_str(&indent(&doc_comment(doc)));
    }
    let params = class
        .init
        .params
        .iter()
        .map(|p| format!("{}: {}", rust_ident(&p.name), rust_ty(&p.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(
        out,
        "    pub fn __compylr_new({params}) -> Result<Self, RuntimeError> {{"
    );

    // The constructor body is a sequence of attribute assignments against a `self` that does not
    // exist yet, so each is evaluated into a local of the attribute's name and the struct is built
    // from them at the end. That also means an attribute's initialiser may read one declared
    // before it, which is what a reader of the Python would expect.
    let mut emitter = Emitter {
        function: &class.init,
        unit,
        out: String::new(),
    };
    for stmt in &class.init.body {
        match stmt {
            Stmt::SetAttr {
                name, ty, value, ..
            } => {
                let value = emit_expr(value, unit, ty)?;
                let _ = writeln!(
                    emitter.out,
                    "        let {}: {} = {value};",
                    rust_ident(name),
                    rust_ty(ty)
                );
            }
            other => emitter.stmts(std::slice::from_ref(other), 2)?,
        }
    }
    out.push_str(&emitter.out);

    let fields = class
        .attributes
        .iter()
        .map(|a| rust_ident(&a.name))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "        Ok(Self {{ {fields} }})");
    out.push_str("    }\n");
    Ok(out)
}

/// Emit one method, with the receiver its body requires.
fn emit_method(
    method: &Function,
    class: &Class,
    unit: &Unit,
    mutates: bool,
) -> Result<String, BackendError> {
    let mut out = String::new();
    if let Some(doc) = &method.doc {
        out.push_str(&indent(&doc_comment(doc)));
    }
    let receiver = if mutates { "&mut self" } else { "&self" };
    let params = method
        .params
        .iter()
        .map(|p| {
            let mutable = if is_assigned(&method.body, &p.name, unit) {
                "mut "
            } else {
                ""
            };
            format!("{mutable}{}: {}", rust_ident(&p.name), rust_ty(&p.ty))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let separator = if params.is_empty() { "" } else { ", " };
    let _ = writeln!(
        out,
        "    pub fn {}({receiver}{separator}{params}) -> Result<{}, RuntimeError> {{",
        rust_ident(&method.name),
        rust_ty(&method.ret)
    );
    let _ = class;
    out.push_str(&indent(&emit_body(method, unit)?));
    out.push_str("    }\n");
    Ok(out)
}

/// Indent a block by one level, leaving blank lines alone.
fn indent(block: &str) -> String {
    block
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("    {line}")
            }
        })
        .map(|line| line + "\n")
        .collect()
}

/// Which methods need a mutable receiver.
///
/// A method mutates when it assigns an attribute, mutates a collection attribute, **or calls a
/// method that does**. The transitive case is the one that will be got wrong: a method whose body
/// is only `self.record(x)` mutates through the call, and a shared receiver there produces a
/// borrow-checker error about generated code rather than a diagnostic about the user's program.
///
/// So this is a fixpoint: mark the directly-mutating methods, then repeatedly mark any method
/// calling a marked one until nothing changes. A class has few methods, so it converges in a
/// handful of passes over a small set.
pub fn mutating_methods(class: &Class) -> BTreeSet<String> {
    let mut mutating: BTreeSet<String> = class
        .methods
        .values()
        .filter(|method| mutates_self_directly(&method.body))
        .map(|method| method.name.clone())
        .collect();

    loop {
        let mut added = false;
        for method in class.methods.values() {
            if mutating.contains(&method.name) {
                continue;
            }
            if calls_any_of(&method.body, &mutating) {
                mutating.insert(method.name.clone());
                added = true;
            }
        }
        if !added {
            return mutating;
        }
    }
}

/// Whether a body assigns an attribute of `self`, or mutates a collection held in one.
fn mutates_self_directly(stmts: &[Stmt]) -> bool {
    fn targets_self(expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => name == "self",
            // `self.entries[k] = v` mutates through an attribute of self.
            Expr::Attribute { object, .. } => targets_self(object),
            _ => false,
        }
    }
    stmts.iter().any(|stmt| match stmt {
        Stmt::SetAttr { object, .. } => targets_self(object),
        Stmt::SetItem { collection, .. } => targets_self(collection),
        Stmt::Append { sequence, .. } => targets_self(sequence),
        Stmt::If {
            then, otherwise, ..
        } => mutates_self_directly(then) || mutates_self_directly(otherwise),
        Stmt::While { body, .. } | Stmt::For { body, .. } => mutates_self_directly(body),
        _ => false,
    })
}

/// Whether a body calls any of the named methods on `self`.
///
/// Only calls on `self` count. A call on some *other* instance mutates that object, which the
/// receiver of the enclosing method has nothing to do with.
fn calls_any_of(stmts: &[Stmt], named: &BTreeSet<String>) -> bool {
    stmts.iter().any(|stmt| match stmt {
        Stmt::Return(expr) | Stmt::Effect(expr) => expr_calls_any_of(expr, named),
        Stmt::Bind { value, .. } | Stmt::Assign { value, .. } => expr_calls_any_of(value, named),
        Stmt::SetAttr { object, value, .. } => {
            expr_calls_any_of(object, named) || expr_calls_any_of(value, named)
        }
        Stmt::SetItem {
            collection,
            index,
            value,
        } => {
            expr_calls_any_of(collection, named)
                || expr_calls_any_of(index, named)
                || expr_calls_any_of(value, named)
        }
        Stmt::Append { sequence, value } => {
            expr_calls_any_of(sequence, named) || expr_calls_any_of(value, named)
        }
        Stmt::If {
            test,
            then,
            otherwise,
        } => {
            expr_calls_any_of(test, named)
                || calls_any_of(then, named)
                || calls_any_of(otherwise, named)
        }
        Stmt::While { test, body } => expr_calls_any_of(test, named) || calls_any_of(body, named),
        Stmt::For { iter, body, .. } => expr_calls_any_of(iter, named) || calls_any_of(body, named),
        Stmt::ReturnUnit | Stmt::Break | Stmt::Continue => false,
    })
}

/// Whether an expression calls any of the named methods on `self`.
fn expr_calls_any_of(expr: &Expr, named: &BTreeSet<String>) -> bool {
    let mut found = false;
    let recurse = |children: &[&Expr]| children.iter().any(|c| expr_calls_any_of(c, named));
    match expr {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            if matches!(receiver.as_ref(), Expr::Name(name) if name == "self")
                && named.contains(method.as_str())
            {
                found = true;
            }
            found
                || expr_calls_any_of(receiver, named)
                || args.iter().any(|a| expr_calls_any_of(a, named))
        }
        Expr::Literal(_) | Expr::Name(_) => false,
        Expr::Neg(inner) | Expr::ToFloat(inner) | Expr::Not(inner) | Expr::Len(inner) => {
            expr_calls_any_of(inner, named)
        }
        Expr::Attribute { object, .. } => expr_calls_any_of(object, named),
        Expr::TupleIndex { base, .. } => expr_calls_any_of(base, named),
        Expr::Binary { left, right, .. } => recurse(&[left, right]),
        Expr::Subscript { base, index } => recurse(&[base, index]),
        Expr::Contains { value, container } => recurse(&[value, container]),
        Expr::Range { start, stop, step } => recurse(&[start, stop, step]),
        Expr::ListLit(items) | Expr::SetLit(items) | Expr::TupleLit(items) => {
            items.iter().any(|i| expr_calls_any_of(i, named))
        }
        Expr::DictLit(pairs) => pairs
            .iter()
            .any(|(k, v)| expr_calls_any_of(k, named) || expr_calls_any_of(v, named)),
        Expr::Call { args, .. } | Expr::Construct { args, .. } => {
            args.iter().any(|a| expr_calls_any_of(a, named))
        }
    }
}

/// Emit a function body, ending in a tail expression rather than a `return`.
///
/// The final statement becomes the tail so the generated function has no unreachable trailing
/// expression, which would be a warning in code that must compile clean.
fn emit_body(function: &Function, unit: &Unit) -> Result<String, BackendError> {
    // A `return` in final position is emitted as a tail expression instead. Rust reads it as the
    // same thing, and a straight-line function — still the common case — is left looking like
    // idiomatic Rust rather than like Python transliterated into it.
    let (leading, tail) = match function.body.split_last() {
        Some((last @ (Stmt::Return(_) | Stmt::ReturnUnit), leading)) => (leading, Some(last)),
        _ => (&function.body[..], None),
    };
    let mut emitter = Emitter {
        function,
        unit,
        out: String::new(),
    };
    emitter.stmts(leading, 1)?;
    let mut out = emitter.out;

    match tail {
        Some(Stmt::Return(expr)) => {
            let value = emit_expr(expr, unit, &function.ret)?;
            let _ = writeln!(out, "    Ok({value})");
        }
        Some(Stmt::ReturnUnit) => out.push_str("    Ok(())\n"),
        // No tail return. Lowering guarantees a non-unit function returns on every path, so the
        // end of its body is genuinely unreachable and Rust agrees; a unit function needs the
        // value supplied, unless it already returned — appending regardless would emit
        // unreachable code.
        _ if returns_on_all_paths(&function.body) => {}
        _ if function.ret == Ty::Unit => out.push_str("    Ok(())\n"),
        _ => {
            return Err(BackendError::Unsupported {
                detail: format!(
                    "function '{}' declares a return type of '{}' but does not return on every path",
                    function.name, function.ret
                ),
            });
        }
    }
    Ok(out)
}

/// The context a function body is emitted against.
///
/// Bundled rather than threaded through every call: emission recurses into nested blocks, and a
/// signature long enough to carry each piece separately obscures the two things that actually vary
/// between levels — the statements and the indentation.
struct Emitter<'a> {
    /// The function being emitted, for its return type.
    function: &'a Function,
    /// The unit, for resolving callees.
    unit: &'a Unit,
    /// The source being built up.
    out: String,
}

impl Emitter<'_> {
    /// Emit a sequence of statements at a given indentation depth.
    fn stmts(&mut self, stmts: &[Stmt], depth: usize) -> Result<(), BackendError> {
        let pad = "    ".repeat(depth);
        for stmt in stmts {
            match stmt {
                Stmt::Bind { name, ty, value } => {
                    let value = emit_expr(value, self.unit, ty)?;
                    // `mut` only when something assigns to it later, so generated code carries no
                    // avoidable warning. Scanning the enclosing block rather than the whole
                    // function: lowering scopes a binding to the block that introduced it, so
                    // nothing outside it can assign to the name.
                    let mutable = if is_assigned(stmts, name, self.unit) {
                        "mut "
                    } else {
                        ""
                    };
                    let _ = writeln!(
                        self.out,
                        "{pad}let {mutable}{}: {} = {value};",
                        rust_ident(name),
                        rust_ty(ty)
                    );
                }
                Stmt::Assign { name, ty, value } => {
                    let value = emit_expr(value, self.unit, ty)?;
                    let _ = writeln!(self.out, "{pad}{} = {value};", rust_ident(name));
                }
                Stmt::Return(expr) => {
                    let value = emit_expr(expr, self.unit, &self.function.ret)?;
                    let _ = writeln!(self.out, "{pad}return Ok({value});");
                }
                Stmt::ReturnUnit => {
                    let _ = writeln!(self.out, "{pad}return Ok(());");
                }
                Stmt::Effect(expr) => {
                    let value = emit_expr(expr, self.unit, &Ty::Unit)?;
                    let _ = writeln!(self.out, "{pad}{value};");
                }
                Stmt::SetAttr {
                    object,
                    name,
                    ty,
                    value,
                } => {
                    let object = emit_place(object, self.unit)?;
                    let value = emit_expr(value, self.unit, ty)?;
                    let _ = writeln!(self.out, "{pad}({object}).{} = {value};", rust_ident(name));
                }
                Stmt::SetItem {
                    collection,
                    index,
                    value,
                } => {
                    // The collection is emitted as a place rather than a value: this is the one
                    // context where the usual clone would be actively wrong, since assigning into
                    // a copy compiles and does nothing.
                    //
                    // The operands are bound first, in Python's own order — the value, then the
                    // target's index. That is not cosmetic: `d[k] = d[k] + 1` reads the same
                    // collection it writes, and evaluating inline would ask for a shared borrow
                    // inside a mutable one.
                    let collection = emit_place(collection, self.unit)?;
                    let index = emit_owned_operand(index, self.unit)?;
                    let value = emit_owned_operand(value, self.unit)?;
                    let _ = writeln!(self.out, "{pad}{{");
                    let _ = writeln!(self.out, "{pad}    let __compylr_value = {value};");
                    let _ = writeln!(self.out, "{pad}    let __compylr_index = {index};");
                    let _ = writeln!(
                        self.out,
                        "{pad}    PySetItem::py_set(&mut ({collection}), &__compylr_index, __compylr_value)?;"
                    );
                    let _ = writeln!(self.out, "{pad}}}");
                }
                Stmt::Append { sequence, value } => {
                    // Bound first for the same reason: `xs.append(xs[0])` reads what it extends.
                    let sequence = emit_place(sequence, self.unit)?;
                    let value = emit_owned_operand(value, self.unit)?;
                    let _ = writeln!(self.out, "{pad}{{");
                    let _ = writeln!(self.out, "{pad}    let __compylr_value = {value};");
                    let _ = writeln!(self.out, "{pad}    ({sequence}).push(__compylr_value);");
                    let _ = writeln!(self.out, "{pad}}}");
                }
                Stmt::Break => {
                    let _ = writeln!(self.out, "{pad}break;");
                }
                Stmt::Continue => {
                    let _ = writeln!(self.out, "{pad}continue;");
                }
                Stmt::If {
                    test,
                    then,
                    otherwise,
                } => {
                    let test = emit_expr(test, self.unit, &Ty::Bool)?;
                    let _ = writeln!(self.out, "{pad}if {test} {{");
                    self.stmts(then, depth + 1)?;
                    if !otherwise.is_empty() {
                        let _ = writeln!(self.out, "{pad}}} else {{");
                        self.stmts(otherwise, depth + 1)?;
                    }
                    let _ = writeln!(self.out, "{pad}}}");
                }
                Stmt::While { test, body } => {
                    let test = emit_expr(test, self.unit, &Ty::Bool)?;
                    let _ = writeln!(self.out, "{pad}while {test} {{");
                    self.stmts(body, depth + 1)?;
                    let _ = writeln!(self.out, "{pad}}}");
                }
                Stmt::For {
                    name,
                    ty,
                    iter,
                    body,
                } => match iter {
                    Expr::Range { start, stop, step } => {
                        self.range_loop(name, start, stop, step, body, depth)?
                    }
                    iterable => self.collection_loop(name, ty, iterable, body, depth)?,
                },
            }
        }
        Ok(())
    }

    /// Emit `for <name> in range(...)`.
    ///
    /// Python's `range` has no Rust equivalent: `..` counts up by one, `step_by` takes an unsigned
    /// step, and neither composes with a step that is computed or negative. So the loop is written
    /// out, driven by a cursor the body cannot disturb — assigning to the loop variable does not
    /// affect iteration in Python either.
    fn range_loop(
        &mut self,
        name: &str,
        start: &Expr,
        stop: &Expr,
        step: &Expr,
        body: &[Stmt],
        depth: usize,
    ) -> Result<(), BackendError> {
        let pad = "    ".repeat(depth);
        let bound = rust_ident(name);
        let mutable = if is_assigned(body, name, self.unit) {
            "mut "
        } else {
            ""
        };
        let start = emit_expr(start, self.unit, &Ty::Int)?;
        let stop = emit_expr(stop, self.unit, &Ty::Int)?;
        let step = emit_expr(step, self.unit, &Ty::Int)?;

        let _ = writeln!(self.out, "{pad}{{");
        let _ = writeln!(self.out, "{pad}    let __compylr_stop: i64 = {stop};");
        let _ = writeln!(self.out, "{pad}    let __compylr_step: i64 = {step};");
        // Checked before the loop rather than inside it: with a zero step the condition never
        // changes, and a program that hangs gives nothing at all to diagnose from.
        let _ = writeln!(self.out, "{pad}    if __compylr_step == 0 {{");
        let _ = writeln!(self.out, "{pad}        return Err(RuntimeError::ZeroStep);");
        let _ = writeln!(self.out, "{pad}    }}");
        let _ = writeln!(
            self.out,
            "{pad}    let mut __compylr_cursor: i64 = {start};"
        );
        let _ = writeln!(
            self.out,
            "{pad}    while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)\n\
             {pad}        || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)\n\
             {pad}    {{"
        );
        let _ = writeln!(
            self.out,
            "{pad}        let {mutable}{bound}: i64 = __compylr_cursor;"
        );
        self.stmts(body, depth + 2)?;
        // Checked, because a range whose stop is near i64::MAX would otherwise wrap and run again.
        let _ = writeln!(
            self.out,
            "{pad}        __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;"
        );
        let _ = writeln!(self.out, "{pad}    }}");
        let _ = writeln!(self.out, "{pad}}}");
        Ok(())
    }

    /// Emit `for <name> in <collection>`.
    fn collection_loop(
        &mut self,
        name: &str,
        ty: &Ty,
        iterable: &Expr,
        body: &[Stmt],
        depth: usize,
    ) -> Result<(), BackendError> {
        let pad = "    ".repeat(depth);
        let bound = rust_ident(name);
        let mutable = if is_assigned(body, name, self.unit) {
            "mut "
        } else {
            ""
        };
        // A snapshot only when the body could disturb what is being iterated. Python's `for` holds
        // the object, so rebinding or mutating the name inside the body must not change what the
        // loop walks — an owned copy says that directly, and keeps a loop-long borrow from
        // colliding with the write.
        //
        // Copying unconditionally is what the first version did, and it is quadratic: a `for` over
        // a collection that grows in an enclosing loop copies the whole thing every pass. That is
        // invisible in a correctness test and showed up immediately in the demo's benchmark, which
        // is exactly what a benchmark is for.
        let disturbed = match iterable {
            Expr::Name(target) => is_assigned(body, target, self.unit),
            // Anything else is already a temporary, so there is nothing to alias.
            _ => false,
        };

        let _ = writeln!(self.out, "{pad}{{");
        if disturbed {
            let owned = emit_owned_operand(iterable, self.unit)?;
            let _ = writeln!(self.out, "{pad}    let __compylr_iter = {owned};");
        } else {
            let place = emit_place(iterable, self.unit)?;
            let _ = writeln!(self.out, "{pad}    let __compylr_iter = &{place};");
        }
        let _ = writeln!(
            self.out,
            "{pad}    for __compylr_item in PyIterate::py_iter(__compylr_iter) {{"
        );
        // The loop variable is bound inside rather than in the pattern, so it carries the element
        // type lowering derived — a disagreement between the two then fails to compile here,
        // rather than somewhere in the body.
        let _ = writeln!(
            self.out,
            "{pad}        let {mutable}{bound}: {} = __compylr_item;",
            rust_ty(ty)
        );
        self.stmts(body, depth + 2)?;
        let _ = writeln!(self.out, "{pad}    }}");
        let _ = writeln!(self.out, "{pad}}}");
        Ok(())
    }
}

/// Emit an expression as a **place**: something that can be written through.
///
/// The ordinary path clones a collection wherever it is consumed, so a name read twice is not
/// moved. That rule is exactly wrong for a mutation target — `xs.clone().push(v)` compiles, runs,
/// and does nothing — and it reaches through attributes too, where `self.entries[k] = v` would
/// otherwise mutate a copy of the field and leave the object untouched.
///
/// Only the two shapes a mutation target can take are places. Anything else is a value, and
/// lowering has already refused to mutate one.
fn emit_place(expr: &Expr, unit: &Unit) -> Result<String, BackendError> {
    match expr {
        Expr::Name(name) if name == "self" => Ok("self".to_string()),
        Expr::Name(name) => Ok(rust_ident(name)),
        Expr::Attribute { object, name } => Ok(format!(
            "({}).{}",
            emit_place(object, unit)?,
            rust_ident(name)
        )),
        other => emit_expr(other, unit, &Ty::Unit),
    }
}

/// Emit an expression that is about to be bound to a temporary or moved into a container.
///
/// A bare name is cloned. Python has no notion of a value being consumed by being used, so a name
/// that is passed somewhere must still be readable afterwards — `d[k] = 1` followed by `d[k]` reads
/// `k` twice. Anything else already produces an owned value.
///
/// `expected` is unavailable at these sites (the IR does not annotate expressions with their
/// types), so the name check stands in for it. Cloning a `Copy` type is free enough that being
/// wrong in that direction costs nothing.
fn emit_owned_operand(expr: &Expr, unit: &Unit) -> Result<String, BackendError> {
    match expr {
        Expr::Name(name) => Ok(format!("{}.clone()", rust_ident(name))),
        other => emit_expr(other, unit, &Ty::Unit),
    }
}

/// Whether any statement writes to `name`, including inside nested bodies.
///
/// Emission needs this before the binding is written, because the `let` comes before the statement
/// that makes it mutable. Mutating a collection counts as writing: `xs.push(v)` needs `xs` to be
/// `mut` just as `x = 1` does, and a scan that missed one would produce code that fails to compile.
///
/// Only a bare name is a target. `f(xs)[0] = v` is not something the subset can express, so there
/// is no case where the collection being written is anything but a name.
fn is_assigned(stmts: &[Stmt], name: &str, unit: &Unit) -> bool {
    let names_it = |expr: &Expr| matches!(expr, Expr::Name(target) if target == name);
    stmts.iter().any(|stmt| match stmt {
        Stmt::Assign { name: target, .. } => target == name,
        Stmt::SetItem { collection, .. } => names_it(collection),
        Stmt::Append { sequence, .. } => names_it(sequence),
        Stmt::SetAttr { object, .. } => names_it(object),
        Stmt::Effect(expr) => mutating_call_on(expr, name, unit),
        Stmt::If {
            test,
            then,
            otherwise,
        } => {
            mutating_call_on(test, name, unit)
                || is_assigned(then, name, unit)
                || is_assigned(otherwise, name, unit)
        }
        Stmt::While { test, body } => {
            mutating_call_on(test, name, unit) || is_assigned(body, name, unit)
        }
        Stmt::For { iter, body, .. } => {
            mutating_call_on(iter, name, unit) || is_assigned(body, name, unit)
        }
        Stmt::Return(expr) => mutating_call_on(expr, name, unit),
        Stmt::Bind { value, .. } => mutating_call_on(value, name, unit),
        _ => false,
    })
}

/// Whether an expression calls a mutating method on the local named `name`.
///
/// A local instance needs `mut` for the same reason a reassigned one does — the `let` comes before
/// the call that requires it. The method name alone does not settle whether it mutates, since two
/// classes may both define `get`, so the receiver's class is carried on the node. When lowering
/// could not determine it the call is assumed to mutate: a spurious `mut` is a warning, and a
/// missing one is code that does not compile.
fn mutating_call_on(expr: &Expr, name: &str, unit: &Unit) -> bool {
    let mut found = false;
    visit_exprs(expr, &mut |node| {
        if let Expr::MethodCall {
            receiver,
            class,
            method,
            ..
        } = node
            && matches!(receiver.as_ref(), Expr::Name(target) if target == name)
        {
            found |= match class.as_deref().and_then(|c| unit.class(c)) {
                Some(class) => mutating_methods(class).contains(method),
                None => true,
            };
        }
    });
    found
}

/// Visit an expression and every expression nested in it.
fn visit_exprs(expr: &Expr, visit: &mut impl FnMut(&Expr)) {
    visit(expr);
    match expr {
        Expr::Literal(_) | Expr::Name(_) => {}
        Expr::Neg(inner) | Expr::ToFloat(inner) | Expr::Not(inner) | Expr::Len(inner) => {
            visit_exprs(inner, visit)
        }
        Expr::Attribute { object, .. } => visit_exprs(object, visit),
        Expr::TupleIndex { base, .. } => visit_exprs(base, visit),
        Expr::Binary { left, right, .. } => {
            visit_exprs(left, visit);
            visit_exprs(right, visit);
        }
        Expr::Subscript { base, index } => {
            visit_exprs(base, visit);
            visit_exprs(index, visit);
        }
        Expr::Contains { value, container } => {
            visit_exprs(value, visit);
            visit_exprs(container, visit);
        }
        Expr::Range { start, stop, step } => {
            visit_exprs(start, visit);
            visit_exprs(stop, visit);
            visit_exprs(step, visit);
        }
        Expr::ListLit(items) | Expr::SetLit(items) | Expr::TupleLit(items) => {
            for item in items {
                visit_exprs(item, visit);
            }
        }
        Expr::DictLit(pairs) => {
            for (key, value) in pairs {
                visit_exprs(key, visit);
                visit_exprs(value, visit);
            }
        }
        Expr::Call { args, .. } | Expr::Construct { args, .. } => {
            for arg in args {
                visit_exprs(arg, visit);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            visit_exprs(receiver, visit);
            for arg in args {
                visit_exprs(arg, visit);
            }
        }
    }
}

/// Emit an expression, fully parenthesized.
///
/// `expected` is the type the surrounding context wants. It is used for exactly one thing:
/// deciding whether an owned `String` has to be produced where a value is consumed, so that a
/// string parameter used twice is not moved on first use.
fn emit_expr(expr: &Expr, unit: &Unit, expected: &Ty) -> Result<String, BackendError> {
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
        // `self` is the Rust receiver, so it is never escaped and never cloned: cloning it would
        // detach every mutation from the object the caller holds. Lowering reserves the name
        // outside a method, so nothing else can reach this branch.
        Expr::Name(name) if name == "self" => "self".to_string(),
        Expr::Name(name) => {
            let name = rust_ident(name);
            if !expected.is_trivially_copyable() {
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
            let inner = emit_expr(inner, unit, &Ty::Int)?;
            format!("(({inner}) as f64)")
        }
        Expr::ListLit(items) => {
            let element = element_ty(expected);
            let rendered = render_all(items, unit, &element)?;
            format!("vec![{}]", rendered.join(", "))
        }
        Expr::SetLit(items) => {
            let element = element_ty(expected);
            let rendered = render_all(items, unit, &element)?;
            format!("HashSet::from([{}])", rendered.join(", "))
        }
        Expr::TupleLit(items) => {
            // A type per position, so each element is rendered against its own.
            let types: Vec<Ty> = match expected {
                Ty::Tuple(types) if types.len() == items.len() => types.clone(),
                _ => vec![Ty::Unit; items.len()],
            };
            let mut rendered = Vec::with_capacity(items.len());
            for (item, ty) in items.iter().zip(&types) {
                rendered.push(emit_expr(item, unit, ty)?);
            }
            // A one-element tuple needs the trailing comma to be a tuple at all.
            if rendered.len() == 1 {
                format!("({},)", rendered[0])
            } else {
                format!("({})", rendered.join(", "))
            }
        }
        Expr::DictLit(pairs) => {
            let (key_ty, value_ty) = match expected {
                Ty::Dict(key, value) => ((**key).clone(), (**value).clone()),
                _ => (Ty::Unit, Ty::Unit),
            };
            let mut rendered = Vec::with_capacity(pairs.len());
            for (key, value) in pairs {
                rendered.push(format!(
                    "({}, {})",
                    emit_expr(key, unit, &key_ty)?,
                    emit_expr(value, unit, &value_ty)?
                ));
            }
            format!("HashMap::from([{}])", rendered.join(", "))
        }
        Expr::Attribute { object, name } => {
            // Cloned rather than moved: reading one field must not consume the object, and the
            // object may well be `self`.
            let object = emit_expr(object, unit, &Ty::Unit)?;
            format!("({object}).{}.clone()", rust_ident(name))
        }
        Expr::Construct { class, args } => {
            let class_def = unit.class(class).ok_or_else(|| BackendError::Unsupported {
                detail: format!("class '{class}' is not in this unit"),
            })?;
            let rendered = args
                .iter()
                .zip(class_def.init.params.iter())
                .map(|(arg, param)| emit_expr(arg, unit, &param.ty))
                .collect::<Result<Vec<_>, _>>()?;
            format!(
                "{}::__compylr_new({})?",
                rust_ident(class),
                rendered.join(", ")
            )
        }
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            // A place, not a value: a mutating method needs the receiver itself, and calling one
            // on a clone would compile and lose the mutation.
            let receiver = emit_place(receiver, unit)?;
            let rendered = render_all(args, unit, &Ty::Unit)?;
            format!(
                "({receiver}).{}({})?",
                rust_ident(method),
                rendered.join(", ")
            )
        }
        Expr::Not(inner) => {
            let inner = emit_expr(inner, unit, &Ty::Bool)?;
            format!("!({inner})")
        }
        Expr::Contains { value, container } => {
            // Borrowed, so a container can be tested and then still read or iterated.
            let value = emit_expr(value, unit, &Ty::Unit)?;
            let container = emit_expr(container, unit, &Ty::Unit)?;
            format!("PyContains::py_contains(&({container}), &({value}))")
        }
        Expr::TupleIndex { base, position } => {
            // A field access rather than a call: the result type differs per position, so no
            // single lookup operation could have the right signature. Cloned rather than moved,
            // since the tuple may be read again.
            let base = emit_expr(base, unit, &Ty::Unit)?;
            format!("({base}).{position}.clone()")
        }
        Expr::Subscript { base, index } => {
            // The base is borrowed rather than consumed, so a collection read twice is not moved.
            let base = emit_expr(base, unit, &Ty::Unit)?;
            let index = emit_expr(index, unit, &Ty::Unit)?;
            format!("py_subscript(&({base}), &({index}))?")
        }
        Expr::Len(inner) => {
            let inner = emit_expr(inner, unit, &Ty::Unit)?;
            format!("PyLen::py_len(&({inner}))")
        }
        Expr::Range { .. } => {
            // A range is only meaningful as something to iterate, and lowering rejects it
            // anywhere else — so reaching here is a compylr defect rather than a user error.
            return Err(BackendError::Unsupported {
                detail: "a range cannot be evaluated outside a loop".to_string(),
            });
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
                .map(|(arg, param)| emit_expr(arg, unit, &param.ty))
                .collect::<Result<Vec<_>, _>>()?;
            format!("{}({})?", rust_ident(callee), rendered.join(", "))
        }
    })
}

/// The element type of an expected collection type, or unit when the context says nothing.
fn element_ty(expected: &Ty) -> Ty {
    match expected {
        Ty::List(element) | Ty::Set(element) => (**element).clone(),
        _ => Ty::Unit,
    }
}

/// Render every expression against one expected type.
fn render_all(items: &[Expr], unit: &Unit, expected: &Ty) -> Result<Vec<String>, BackendError> {
    items
        .iter()
        .map(|item| emit_expr(item, unit, expected))
        .collect()
}

/// Emit a binary operation as a trait call, letting Rust choose the implementation by type.
fn emit_binary(
    op: BinOp,
    left: &Expr,
    right: &Expr,
    unit: &Unit,
    expected: &Ty,
) -> Result<String, BackendError> {
    // Comparisons yield a bool regardless of operand type, so the expected type says nothing
    // about the operands. They are emitted by reference, which both avoids moving a string and
    // works uniformly for every comparable type.
    if op.is_comparison() {
        let left = emit_expr(left, unit, &Ty::Unit)?;
        let right = emit_expr(right, unit, &Ty::Unit)?;
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

    // Exact division's operands are always floats: lowering inserted the promotion nodes.
    if op
        == (BinOp::Div {
            mode: DivMode::Exact,
        })
    {
        let left = emit_expr(left, unit, &Ty::Float)?;
        let right = emit_expr(right, unit, &Ty::Float)?;
        return Ok(format!("div_exact(&({left}), &({right}))?"));
    }

    // Arithmetic operands share the expression's own type, except that a string operand must not
    // be cloned here: the trait takes a reference.
    let operand = if *expected == Ty::Str {
        Ty::Unit
    } else {
        expected.clone()
    };
    let left = emit_expr(left, unit, &operand)?;
    let right = emit_expr(right, unit, &operand)?;
    // Read off the node, not off the operator's name. The same `BinOp::Div` reaches here meaning
    // either rounding, and a backend that assumed one of them would be silently wrong for any
    // frontend that meant the other.
    let call = match op {
        BinOp::Add => "PyAdd::py_add",
        BinOp::Sub => "PyNum::py_sub",
        BinOp::Mul => "PyNum::py_mul",
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardNegInf),
        } => "PyNum::div_floor",
        BinOp::Div {
            mode: DivMode::Integer(Rounding::TowardZero),
        } => "PyNum::div_trunc",
        BinOp::Rem {
            sign: RemSign::Divisor,
        } => "PyNum::rem_floor",
        BinOp::Rem {
            sign: RemSign::Dividend,
        } => "PyNum::rem_trunc",
        _ => unreachable!("comparisons and exact division are handled above"),
    };
    Ok(format!("{call}(&({left}), &({right}))?"))
}
