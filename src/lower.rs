//! Lowering a parsed Python module into compylr IR.
//!
//! Lowering is where the "strict annotated subset" is enforced. Anything outside it is rejected
//! with a located diagnostic rather than guessed at, because a transpiler that quietly picks a
//! meaning produces code that compiles and does the wrong thing.
//!
//! Two decisions shape the structure:
//!
//! * **Call targets are not resolved here.** A function may legitimately call one that has not
//!   been lowered yet, since in the target design each function is submitted independently.
//!   Resolving during lowering would make success depend on arrival order, so calls are
//!   recorded by name and checked later by [`crate::ir::Unit::validate`].
//! * **Inference covers whatever is determined.** A binding may omit its annotation when its
//!   initializer's type follows from literals, already-typed names, negation, arithmetic, and
//!   comparisons. Each of those has exactly one possible result given its operands, so this
//!   computes an answer that was already fixed rather than choosing among candidates. An
//!   expression containing a call is *undetermined* — not an error — and such a binding still
//!   needs an annotation.
//!
//! Lowering is therefore also a small type checker: [`lower_expr`] returns an expression and
//! its type together, so shape and type can never be derived from separate traversals and
//! disagree.

use std::collections::HashMap;

use ruff_python_ast::{
    CmpOp, Expr as PyExpr, ModModule, Number, Operator, Parameters, Stmt as PyStmt,
    StmtFunctionDef, UnaryOp,
};
use ruff_python_parser::Parsed;
use ruff_text_size::Ranged;

use crate::error::{LowerError, LowerErrorKind};
use crate::ir::{BinOp, Expr, Function, Param, Stmt, Ty};
use crate::span::Span;

/// Names visible inside a function body, with the type each was bound at.
type Scope = HashMap<String, Ty>;

fn err(kind: LowerErrorKind, message: impl Into<String>, node: &impl Ranged) -> LowerError {
    LowerError::new(kind, message, Span::from(node.range()))
}

/// Lower every top-level function definition in a parsed source.
///
/// Only function definitions are permitted at top level; a module-level statement such as an
/// `if __name__ == '__main__':` guard has no meaning once the function is compiled into a
/// shared artifact, so it is rejected rather than silently dropped.
pub fn lower_source(parsed: &Parsed<ModModule>) -> Result<Vec<Function>, LowerError> {
    let mut functions = Vec::new();
    for stmt in &parsed.syntax().body {
        match stmt {
            PyStmt::FunctionDef(def) => functions.push(lower_function(def)?),
            PyStmt::Import(_) | PyStmt::ImportFrom(_) => {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "imports are not supported; only function definitions may appear at top level",
                    stmt,
                ));
            }
            PyStmt::ClassDef(_) => {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "class definitions are not supported; only function definitions may appear at top level",
                    stmt,
                ));
            }
            other => {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "only function definitions are permitted at top level",
                    other,
                ));
            }
        }
    }
    Ok(functions)
}

/// Lower a single function definition.
pub fn lower_function(def: &StmtFunctionDef) -> Result<Function, LowerError> {
    if def.is_async {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!(
                "'{}' is an async function, which is not supported",
                def.name
            ),
            def,
        ));
    }
    if !def.decorator_list.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!("'{}' carries a decorator, which is not supported", def.name),
            def,
        ));
    }
    if def.type_params.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedType,
            format!(
                "'{}' declares type parameters, which are not yet supported",
                def.name
            ),
            def,
        ));
    }

    let params = lower_parameters(&def.parameters, def.name.as_str())?;

    let ret = match def.returns.as_deref() {
        Some(annotation) => lower_annotation(annotation, true)?,
        None => {
            return Err(err(
                LowerErrorKind::MissingAnnotation,
                format!("function '{}' needs a return type annotation", def.name),
                def,
            ));
        }
    };

    let mut scope: Scope = params
        .iter()
        .map(|param| (param.name.clone(), param.ty))
        .collect();
    let (doc, rest) = split_docstring(&def.body);
    let body = lower_body(rest, &mut scope, ret)?;

    Ok(Function {
        name: def.name.to_string(),
        params,
        ret,
        body,
        doc,
        span: Span::from(def.range()),
    })
}

/// Split a leading docstring off a function body.
///
/// Python treats a bare string literal in first position as documentation: the interpreter records
/// it from the code object rather than by executing the statement, so it contributes nothing to
/// what the function does. Removing it here means the rest of lowering never sees it, and the
/// catch-all that rejects discarded expression statements keeps working everywhere else — a string
/// in *second* position is still an error, because there it really is a value thrown away.
///
/// Adjacent literals (`"a" "b"`) are concatenated by the parser into one node, so they are covered
/// without special handling. An f-string is a different node and is not matched, which is correct:
/// Python does not treat an f-string as a docstring either.
fn split_docstring(body: &[PyStmt]) -> (Option<String>, &[PyStmt]) {
    let Some(PyStmt::Expr(statement)) = body.first() else {
        return (None, body);
    };
    let PyExpr::StringLiteral(literal) = statement.value.as_ref() else {
        return (None, body);
    };
    (Some(literal.value.to_str().to_string()), &body[1..])
}

fn lower_parameters(parameters: &Parameters, owner: &str) -> Result<Vec<Param>, LowerError> {
    // Only plain positional parameters are in the subset. Each of the other forms would need a
    // calling-convention decision on the target side that nothing depends on yet.
    if !parameters.posonlyargs.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "positional-only parameters are not supported",
            parameters,
        ));
    }
    if !parameters.kwonlyargs.is_empty() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "keyword-only parameters are not supported",
            parameters,
        ));
    }
    if parameters.vararg.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "variadic '*args' parameters are not supported",
            parameters,
        ));
    }
    if parameters.kwarg.is_some() {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "variadic '**kwargs' parameters are not supported",
            parameters,
        ));
    }

    let mut params = Vec::with_capacity(parameters.args.len());
    for arg in &parameters.args {
        if arg.default.is_some() {
            return Err(err(
                LowerErrorKind::UnsupportedConstruct,
                format!(
                    "parameter '{}' has a default value, which is not supported",
                    arg.parameter.name
                ),
                arg,
            ));
        }
        let Some(annotation) = arg.parameter.annotation.as_deref() else {
            return Err(err(
                LowerErrorKind::MissingAnnotation,
                format!(
                    "parameter '{}' of '{owner}' needs a type annotation",
                    arg.parameter.name
                ),
                &arg.parameter,
            ));
        };
        params.push(Param {
            name: arg.parameter.name.to_string(),
            ty: lower_annotation(annotation, false)?,
        });
    }
    Ok(params)
}

/// Convert a Python annotation expression into an IR type.
///
/// `allow_unit` is true only for return annotations: `None` describes "returns nothing", which
/// is meaningless for a parameter.
fn lower_annotation(annotation: &PyExpr, allow_unit: bool) -> Result<Ty, LowerError> {
    match annotation {
        PyExpr::Name(name) => match name.id.as_str() {
            "int" => Ok(Ty::Int),
            "float" => Ok(Ty::Float),
            "bool" => Ok(Ty::Bool),
            "str" => Ok(Ty::Str),
            other => Err(err(
                LowerErrorKind::UnsupportedType,
                format!("'{other}' is not a supported type annotation"),
                annotation,
            )),
        },
        PyExpr::NoneLiteral(_) => {
            if allow_unit {
                Ok(Ty::Unit)
            } else {
                Err(err(
                    LowerErrorKind::UnsupportedType,
                    "'None' is only supported as a return annotation",
                    annotation,
                ))
            }
        }
        PyExpr::Subscript(_) => Err(err(
            LowerErrorKind::UnsupportedType,
            "generic type annotations are not supported",
            annotation,
        )),
        other => Err(err(
            LowerErrorKind::UnsupportedType,
            "unsupported type annotation",
            other,
        )),
    }
}

/// The type of an expression, or `None` when it is not determined during lowering.
///
/// `None` is not an error. A call's type comes from the callee's signature, and lowering
/// deliberately does not resolve callees — doing so would make results depend on which
/// function was submitted first. So an expression containing a call anywhere inside it is
/// simply undetermined, and the *binding* decides what that means: infer when `Some`, demand
/// an annotation when `None`.
///
/// Keeping the uncertainty in an `Option` rather than a `Ty::Unknown` variant confines it to
/// lowering; no backend ever has to match on a state that must not reach codegen.
type TyResult = Option<Ty>;

fn lower_body(body: &[PyStmt], scope: &mut Scope, ret: Ty) -> Result<Vec<Stmt>, LowerError> {
    let mut lowered = Vec::with_capacity(body.len());
    for stmt in body {
        lowered.push(lower_stmt(stmt, scope, ret)?);
    }
    Ok(lowered)
}

fn lower_stmt(stmt: &PyStmt, scope: &mut Scope, ret: Ty) -> Result<Stmt, LowerError> {
    match stmt {
        PyStmt::Return(node) => match node.value.as_deref() {
            Some(value) => {
                let (lowered, ty) = lower_expr(value, scope)?;
                if ret == Ty::Unit {
                    return Err(err(
                        LowerErrorKind::TypeMismatch,
                        "function is declared to return no value, but a value is returned here",
                        stmt,
                    ));
                }
                // Only check when the type is determined; a returned call cannot be checked.
                match ty {
                    Some(actual) => Ok(Stmt::Return(coerce(lowered, actual, ret).ok_or_else(
                        || {
                            err(
                                LowerErrorKind::TypeMismatch,
                                format!(
                                    "function returns '{}' but this expression is '{}'",
                                    ret.python_name(),
                                    actual.python_name()
                                ),
                                stmt,
                            )
                        },
                    )?)),
                    None => Ok(Stmt::Return(lowered)),
                }
            }
            None => Ok(Stmt::ReturnUnit),
        },
        PyStmt::Pass(_) => Ok(Stmt::ReturnUnit),
        PyStmt::AnnAssign(assign) => lower_annotated_binding(stmt, assign, scope),
        PyStmt::Assign(assign) => lower_bare_binding(stmt, assign, scope),
        PyStmt::If(_) => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "conditional statements are not supported",
            stmt,
        )),
        PyStmt::While(_) | PyStmt::For(_) => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "loops are not supported",
            stmt,
        )),
        PyStmt::AugAssign(_) => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "augmented assignment is not supported",
            stmt,
        )),
        other => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "unsupported statement",
            other,
        )),
    }
}

/// Adapt `expr` of type `actual` to the expected type, or `None` if it cannot be adapted.
///
/// The only adaptation is widening an integer to a float, which is Python's own promotion.
/// Narrowing a float to an integer is deliberately not offered: it would lose information
/// silently, which is exactly the kind of quiet wrongness this compiler is meant to avoid.
fn coerce(expr: Expr, actual: Ty, expected: Ty) -> Option<Expr> {
    if actual == expected {
        return Some(expr);
    }
    if actual == Ty::Int && expected == Ty::Float {
        return Some(expr.to_float());
    }
    None
}

/// Reject binding a name that already exists, keeping every binding a fresh introduction.
fn ensure_unbound(name: &str, scope: &Scope, node: &impl Ranged) -> Result<(), LowerError> {
    if scope.contains_key(name) {
        return Err(err(
            LowerErrorKind::Reassignment,
            format!("'{name}' is already bound; reassignment is not yet supported"),
            node,
        ));
    }
    Ok(())
}

fn binding_target<'a>(target: &'a PyExpr, node: &impl Ranged) -> Result<&'a str, LowerError> {
    match target {
        PyExpr::Name(name) => Ok(name.id.as_str()),
        _ => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "only simple name targets are supported in assignments",
            node,
        )),
    }
}

fn lower_annotated_binding(
    stmt: &PyStmt,
    assign: &ruff_python_ast::StmtAnnAssign,
    scope: &mut Scope,
) -> Result<Stmt, LowerError> {
    let name = binding_target(&assign.target, stmt)?.to_string();
    ensure_unbound(&name, scope, stmt)?;

    let declared = lower_annotation(&assign.annotation, false)?;
    let Some(value) = assign.value.as_deref() else {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            format!("'{name}' is declared without a value, which is not supported"),
            stmt,
        ));
    };
    let (lowered, actual) = lower_expr(value, scope)?;

    // An undetermined initializer cannot be checked; the declared type is taken on trust.
    let value = match actual {
        Some(actual) => coerce(lowered, actual, declared).ok_or_else(|| {
            err(
                LowerErrorKind::TypeMismatch,
                format!(
                    "'{name}' is declared as '{}' but the value is '{}'",
                    declared.python_name(),
                    actual.python_name()
                ),
                stmt,
            )
        })?,
        None => lowered,
    };

    scope.insert(name.clone(), declared);
    Ok(Stmt::Bind {
        name,
        ty: declared,
        value,
    })
}

fn lower_bare_binding(
    stmt: &PyStmt,
    assign: &ruff_python_ast::StmtAssign,
    scope: &mut Scope,
) -> Result<Stmt, LowerError> {
    if assign.targets.len() != 1 {
        return Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "chained assignment is not supported",
            stmt,
        ));
    }
    let name = binding_target(&assign.targets[0], stmt)?.to_string();
    ensure_unbound(&name, scope, stmt)?;

    let (value, inferred) = lower_expr(&assign.value, scope)?;

    // Infer when the initializer's type is determined; otherwise the answer is genuinely
    // unknown here and an annotation is the only way to supply it.
    let Some(ty) = inferred else {
        return Err(err(
            LowerErrorKind::MissingAnnotation,
            format!(
                "'{name}' needs an explicit type annotation: its value contains a call, whose \
                 type is not known while lowering"
            ),
            stmt,
        ));
    };

    scope.insert(name.clone(), ty);
    Ok(Stmt::Bind { name, ty, value })
}

/// Result type of an arithmetic operator applied to two determined operand types.
fn arithmetic_result(op: BinOp, left: Ty, right: Ty) -> Option<Ty> {
    // Python's `+` is overloaded on strings; every other arithmetic operator is numeric only.
    if op == BinOp::Add && left == Ty::Str && right == Ty::Str {
        return Some(Ty::Str);
    }
    if !left.is_numeric() || !right.is_numeric() {
        return None;
    }
    // True division always yields a float, even for two integers. This is the single most
    // likely place for a backend to be accidentally wrong, which is why it is explicit here.
    if op == BinOp::TrueDiv {
        return Some(Ty::Float);
    }
    if left == Ty::Float || right == Ty::Float {
        Some(Ty::Float)
    } else {
        Some(Ty::Int)
    }
}

/// Build a typed binary expression, applying promotion and rejecting invalid operand types.
fn build_binary(
    op: BinOp,
    left: Expr,
    left_ty: Ty,
    right: Expr,
    right_ty: Ty,
    node: &impl Ranged,
) -> Result<(Expr, Ty), LowerError> {
    let mismatch = |extra: &str| {
        err(
            LowerErrorKind::TypeMismatch,
            format!(
                "operator '{}' is not defined for '{}' and '{}'{extra}",
                op.python_symbol(),
                left_ty.python_name(),
                right_ty.python_name()
            ),
            node,
        )
    };

    if op.is_comparison() {
        // Comparison operands must agree, except that numbers compare across int and float.
        let operand = if left_ty == right_ty {
            left_ty
        } else if left_ty.is_numeric() && right_ty.is_numeric() {
            Ty::Float
        } else {
            return Err(mismatch(""));
        };
        let left = coerce(left, left_ty, operand).ok_or_else(|| mismatch(""))?;
        let right = coerce(right, right_ty, operand).ok_or_else(|| mismatch(""))?;
        return Ok((Expr::binary(op, left, right), Ty::Bool));
    }

    let result = arithmetic_result(op, left_ty, right_ty).ok_or_else(|| {
        if left_ty == Ty::Bool || right_ty == Ty::Bool {
            mismatch("; booleans are not numbers in compylr")
        } else {
            mismatch("")
        }
    })?;

    // Operands are widened to the result type so a backend can emit them positionally.
    let operand = if result == Ty::Str { Ty::Str } else { result };
    let left = coerce(left, left_ty, operand).ok_or_else(|| mismatch(""))?;
    let right = coerce(right, right_ty, operand).ok_or_else(|| mismatch(""))?;
    Ok((Expr::binary(op, left, right), result))
}

/// Lower an expression and determine its type in one traversal.
///
/// Shape and type are produced together so they cannot be computed from different traversals
/// and disagree about what an expression means.
fn lower_expr(expr: &PyExpr, scope: &Scope) -> Result<(Expr, TyResult), LowerError> {
    match expr {
        PyExpr::NumberLiteral(literal) => match &literal.value {
            Number::Int(value) => match value.as_i64() {
                Some(int) => Ok((Expr::int(int), Some(Ty::Int))),
                // `Int::as_i64` returns None beyond the 64-bit range. Truncating would
                // silently change the program's meaning, so this is an error.
                None => Err(err(
                    LowerErrorKind::LiteralOutOfRange,
                    "integer literal is too large for a 64-bit signed integer",
                    expr,
                )),
            },
            Number::Float(value) => Ok((Expr::float(*value), Some(Ty::Float))),
            Number::Complex { .. } => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "complex literals are not supported",
                expr,
            )),
        },
        PyExpr::BooleanLiteral(literal) => Ok((Expr::bool(literal.value), Some(Ty::Bool))),
        PyExpr::StringLiteral(literal) => Ok((Expr::string(literal.value.to_str()), Some(Ty::Str))),
        PyExpr::FString(_) => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "f-strings are not supported",
            expr,
        )),
        PyExpr::Name(name) => {
            let id = name.id.as_str();
            match scope.get(id) {
                Some(ty) => Ok((Expr::name(id), Some(*ty))),
                None => Err(err(
                    LowerErrorKind::Unresolved,
                    format!("'{id}' is not defined"),
                    expr,
                )),
            }
        }
        PyExpr::UnaryOp(unary) => match unary.op {
            UnaryOp::USub => {
                let (operand, ty) = lower_expr(&unary.operand, scope)?;
                match ty {
                    Some(ty) if ty.is_numeric() => Ok((Expr::Neg(Box::new(operand)), Some(ty))),
                    Some(ty) => Err(err(
                        LowerErrorKind::TypeMismatch,
                        format!("cannot negate a value of type '{}'", ty.python_name()),
                        expr,
                    )),
                    None => Ok((Expr::Neg(Box::new(operand)), None)),
                }
            }
            UnaryOp::UAdd => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "unary '+' is not supported",
                expr,
            )),
            UnaryOp::Not => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "'not' is not supported",
                expr,
            )),
            UnaryOp::Invert => Err(err(
                LowerErrorKind::UnsupportedConstruct,
                "bitwise inversion is not supported",
                expr,
            )),
        },
        PyExpr::BinOp(binary) => {
            let op = match binary.op {
                Operator::Add => BinOp::Add,
                Operator::Sub => BinOp::Sub,
                Operator::Mult => BinOp::Mul,
                Operator::Div => BinOp::TrueDiv,
                Operator::FloorDiv => BinOp::FloorDiv,
                Operator::Mod => BinOp::Mod,
                other => {
                    return Err(err(
                        LowerErrorKind::UnsupportedConstruct,
                        format!("operator '{}' is not supported", other.as_str()),
                        expr,
                    ));
                }
            };
            let (left, left_ty) = lower_expr(&binary.left, scope)?;
            let (right, right_ty) = lower_expr(&binary.right, scope)?;
            match (left_ty, right_ty) {
                (Some(l), Some(r)) => {
                    let (node, ty) = build_binary(op, left, l, right, r, expr)?;
                    Ok((node, Some(ty)))
                }
                // Undetermined propagates outward rather than becoming a type error.
                _ => Ok((Expr::binary(op, left, right), None)),
            }
        }
        PyExpr::Compare(compare) => {
            if compare.ops.len() != 1 || compare.comparators.len() != 1 {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "chained comparisons are not supported",
                    expr,
                ));
            }
            let op = match compare.ops[0] {
                CmpOp::Eq => BinOp::Eq,
                CmpOp::NotEq => BinOp::NotEq,
                CmpOp::Lt => BinOp::Lt,
                CmpOp::LtE => BinOp::LtE,
                CmpOp::Gt => BinOp::Gt,
                CmpOp::GtE => BinOp::GtE,
                other => {
                    return Err(err(
                        LowerErrorKind::UnsupportedConstruct,
                        format!("comparison '{}' is not supported", other.as_str()),
                        expr,
                    ));
                }
            };
            let (left, left_ty) = lower_expr(&compare.left, scope)?;
            let (right, right_ty) = lower_expr(&compare.comparators[0], scope)?;
            match (left_ty, right_ty) {
                (Some(l), Some(r)) => {
                    let (node, ty) = build_binary(op, left, l, right, r, expr)?;
                    Ok((node, Some(ty)))
                }
                _ => Ok((Expr::binary(op, left, right), None)),
            }
        }
        PyExpr::Call(call) => {
            if !call.arguments.keywords.is_empty() {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "keyword arguments are not supported",
                    expr,
                ));
            }
            let PyExpr::Name(callee) = call.func.as_ref() else {
                return Err(err(
                    LowerErrorKind::UnsupportedConstruct,
                    "only calls to plain function names are supported",
                    expr,
                ));
            };
            let mut args = Vec::with_capacity(call.arguments.args.len());
            for arg in &call.arguments.args {
                args.push(lower_expr(arg, scope)?.0);
            }
            // The callee is not resolved here, so the call's type is undetermined. Unit
            // validation checks the target once the whole unit is assembled.
            Ok((
                Expr::Call {
                    callee: callee.id.to_string(),
                    args,
                },
                None,
            ))
        }
        other => Err(err(
            LowerErrorKind::UnsupportedConstruct,
            "unsupported expression",
            other,
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parse_source;

    fn lower(source: &str) -> Result<Vec<Function>, LowerError> {
        let parsed = parse_source(source).expect("fixture must parse");
        lower_source(&parsed)
    }

    fn lower_one(source: &str) -> Function {
        let mut functions = lower(source).expect("expected lowering to succeed");
        assert_eq!(functions.len(), 1);
        functions.remove(0)
    }

    fn error_for(source: &str) -> LowerError {
        lower(source).expect_err("expected lowering to fail")
    }

    // ---- happy path -------------------------------------------------------

    #[test]
    fn lowers_a_simple_annotated_function() {
        let f = lower_one("def add(a: int, b: int) -> int:\n    return a + b\n");
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].ty, Ty::Int);
        assert_eq!(f.ret, Ty::Int);
        assert_eq!(
            f.body,
            vec![Stmt::Return(Expr::binary(
                BinOp::Add,
                Expr::name("a"),
                Expr::name("b")
            ))]
        );
    }

    #[test]
    fn preserves_function_order_within_a_source() {
        let functions = lower(
            "def a() -> None:\n    pass\ndef b() -> None:\n    pass\ndef c() -> None:\n    pass\n",
        )
        .unwrap();
        let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn empty_source_lowers_to_no_functions() {
        assert!(lower("").unwrap().is_empty());
    }

    #[test]
    fn covers_bindings_arithmetic_comparison_strings_and_calls() {
        let f = lower_one(
            "def f(a: int) -> bool:\n\
             \x20   x: int = a * 2\n\
             \x20   y: str = \"hi\"\n\
             \x20   z: int = helper(x)\n\
             \x20   return z >= 0\n",
        );
        assert_eq!(f.body.len(), 4);
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
        assert!(matches!(f.body[1], Stmt::Bind { ty: Ty::Str, .. }));
        assert!(matches!(f.body[2], Stmt::Bind { ty: Ty::Int, .. }));
        assert!(matches!(
            f.body[3],
            Stmt::Return(Expr::Binary { op: BinOp::GtE, .. })
        ));
    }

    #[test]
    fn bare_return_and_pass_lower_to_unit_statements() {
        let f = lower_one("def f() -> None:\n    pass\n");
        assert_eq!(f.body, vec![Stmt::ReturnUnit]);
        let g = lower_one("def g() -> None:\n    return\n");
        assert_eq!(g.body, vec![Stmt::ReturnUnit]);
    }

    #[test]
    fn all_supported_operators_lower() {
        for (symbol, expected) in [
            ("+", BinOp::Add),
            ("-", BinOp::Sub),
            ("*", BinOp::Mul),
            ("//", BinOp::FloorDiv),
            ("%", BinOp::Mod),
        ] {
            let f = lower_one(&format!(
                "def f(a: int, b: int) -> int:\n    return a {symbol} b\n"
            ));
            match &f.body[0] {
                Stmt::Return(Expr::Binary { op, .. }) => assert_eq!(*op, expected),
                other => panic!("expected binary for {symbol}, got {other:?}"),
            }
        }
        for (symbol, expected) in [
            ("==", BinOp::Eq),
            ("!=", BinOp::NotEq),
            ("<", BinOp::Lt),
            ("<=", BinOp::LtE),
            (">", BinOp::Gt),
            (">=", BinOp::GtE),
        ] {
            let f = lower_one(&format!(
                "def f(a: int, b: int) -> bool:\n    return a {symbol} b\n"
            ));
            match &f.body[0] {
                Stmt::Return(Expr::Binary { op, .. }) => assert_eq!(*op, expected),
                other => panic!("expected comparison for {symbol}, got {other:?}"),
            }
        }
    }

    #[test]
    fn literals_and_negation_lower() {
        let f = lower_one("def f() -> int:\n    return -7\n");
        assert_eq!(f.body[0], Stmt::Return(Expr::Neg(Box::new(Expr::int(7)))));

        let g = lower_one("def g() -> bool:\n    return True\n");
        assert_eq!(g.body[0], Stmt::Return(Expr::bool(true)));

        let h = lower_one("def h() -> str:\n    return \"hello\"\n");
        assert_eq!(h.body[0], Stmt::Return(Expr::string("hello")));
    }

    #[test]
    fn none_is_accepted_as_a_return_annotation() {
        assert_eq!(lower_one("def f() -> None:\n    pass\n").ret, Ty::Unit);
    }

    // ---- annotations ------------------------------------------------------

    #[test]
    fn unannotated_parameter_is_rejected() {
        let error = error_for("def add(a, b: int) -> int:\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
        assert!(error.message().contains('a'));
    }

    #[test]
    fn missing_return_annotation_is_rejected() {
        let error = error_for("def add(a: int, b: int):\n    return a\n");
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
        assert!(error.message().contains("add"));
    }

    #[test]
    fn unsupported_annotations_are_rejected() {
        let complex = error_for("def f(a: complex) -> int:\n    return 1\n");
        assert_eq!(complex.kind(), LowerErrorKind::UnsupportedType);
        assert!(complex.message().contains("complex"));

        let generic = error_for("def f(a: list[int]) -> int:\n    return 1\n");
        assert_eq!(generic.kind(), LowerErrorKind::UnsupportedType);

        let none_param = error_for("def f(a: None) -> int:\n    return 1\n");
        assert_eq!(none_param.kind(), LowerErrorKind::UnsupportedType);
        assert!(none_param.message().contains("return"));
    }

    #[test]
    fn type_parameters_are_rejected() {
        let error = error_for("def f[T](a: T) -> T:\n    return a\n");
        assert_eq!(error.kind(), LowerErrorKind::UnsupportedType);
        assert!(error.message().contains("type parameters"));
    }

    #[test]
    fn non_simple_parameter_forms_are_rejected() {
        for source in [
            "def f(*args: int) -> int:\n    return 1\n",
            "def f(**kwargs: int) -> int:\n    return 1\n",
            "def f(*, a: int) -> int:\n    return a\n",
            "def f(a: int, /) -> int:\n    return a\n",
            "def f(a: int = 1) -> int:\n    return a\n",
        ] {
            let error = error_for(source);
            assert_eq!(
                error.kind(),
                LowerErrorKind::UnsupportedConstruct,
                "source should be rejected: {source}"
            );
        }
    }

    #[test]
    fn decorated_and_async_functions_are_rejected() {
        let decorated = error_for("@cache\ndef f() -> None:\n    pass\n");
        assert_eq!(decorated.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(decorated.message().contains("decorator"));

        let asynchronous = error_for("async def f() -> None:\n    pass\n");
        assert_eq!(asynchronous.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(asynchronous.message().contains("async"));
    }

    // ---- constructs outside the subset ------------------------------------

    #[test]
    fn control_flow_is_rejected() {
        let conditional =
            error_for("def f(a: int) -> int:\n    if a:\n        return 1\n    return 0\n");
        assert_eq!(conditional.kind(), LowerErrorKind::UnsupportedConstruct);

        let loop_stmt =
            error_for("def f(a: int) -> int:\n    while a:\n        pass\n    return 0\n");
        assert_eq!(loop_stmt.kind(), LowerErrorKind::UnsupportedConstruct);
    }

    #[test]
    fn top_level_statements_are_rejected() {
        let guard =
            error_for("def main() -> None:\n    pass\nif __name__ == '__main__':\n    main()\n");
        assert_eq!(guard.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(guard.message().contains("top level"));

        let import = error_for("import os\n");
        assert_eq!(import.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(import.message().contains("import"));

        let class = error_for("class C:\n    pass\n");
        assert_eq!(class.kind(), LowerErrorKind::UnsupportedConstruct);
        assert!(class.message().contains("class"));
    }

    #[test]
    fn unsupported_operators_are_rejected() {
        let power = error_for("def f(a: int, b: int) -> int:\n    return a ** b\n");
        assert_eq!(power.kind(), LowerErrorKind::UnsupportedConstruct);
    }

    #[test]
    fn out_of_range_integer_literal_is_rejected() {
        let error = error_for(&format!("def f() -> int:\n    return {}\n", "9".repeat(40)));
        assert_eq!(error.kind(), LowerErrorKind::LiteralOutOfRange);
    }

    // ---- name resolution --------------------------------------------------

    #[test]
    fn parameters_and_prior_locals_resolve() {
        let f = lower_one("def f(a: int) -> int:\n    x: int = a + 1\n    return x\n");
        assert_eq!(f.body.len(), 2);
    }

    #[test]
    fn unbound_name_is_rejected() {
        let error = error_for("def f() -> int:\n    return q\n");
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
        assert!(error.message().contains('q'));
    }

    #[test]
    fn reference_before_binding_is_rejected() {
        let error = error_for("def f() -> int:\n    y: int = x\n    x: int = 1\n    return y\n");
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
    }

    #[test]
    fn rebinding_a_local_or_parameter_is_rejected() {
        let local = error_for("def f() -> int:\n    x: int = 1\n    x: int = 2\n    return x\n");
        assert_eq!(local.kind(), LowerErrorKind::Reassignment);

        let parameter = error_for("def f(a: int) -> int:\n    a: int = 2\n    return a\n");
        assert_eq!(parameter.kind(), LowerErrorKind::Reassignment);
    }

    // ---- alias inference --------------------------------------------------

    #[test]
    fn alias_of_a_parameter_is_inferred() {
        let f = lower_one("def foo(a: int) -> int:\n    b = a\n    return b\n");
        assert_eq!(
            f.body[0],
            Stmt::Bind {
                name: "b".into(),
                ty: Ty::Int,
                value: Expr::name("a"),
            }
        );
    }

    #[test]
    fn alias_of_a_prior_local_is_inferred() {
        let f = lower_one("def f() -> str:\n    x: str = \"hi\"\n    y = x\n    return y\n");
        assert!(matches!(f.body[1], Stmt::Bind { ty: Ty::Str, .. }));
    }

    #[test]
    fn chained_aliases_are_inferred() {
        let f = lower_one("def f(a: bool) -> bool:\n    b = a\n    c = b\n    return c\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Bool, .. }));
        assert!(matches!(f.body[1], Stmt::Bind { ty: Ty::Bool, .. }));
    }

    #[test]
    fn unannotated_binding_from_a_literal_is_inferred() {
        let f = lower_one("def f() -> int:\n    x = 1\n    return x\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    #[test]
    fn unannotated_binding_from_an_expression_is_inferred() {
        let f = lower_one("def f(a: int) -> int:\n    b = a + 1\n    return b\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    #[test]
    fn unannotated_binding_from_a_call_is_rejected() {
        let error = error_for("def f(a: int) -> int:\n    b = helper(a)\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
    }

    #[test]
    fn alias_of_an_unbound_name_reports_unresolved_not_missing_annotation() {
        let error = error_for("def f() -> int:\n    b = q\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::Unresolved);
        assert!(error.message().contains('q'));
    }

    #[test]
    fn explicit_annotation_still_wins_over_inference() {
        let f = lower_one("def f(a: int) -> int:\n    b: int = a\n    return b\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    #[test]
    fn annotation_conflicting_with_the_aliased_type_is_rejected() {
        let error = error_for("def f(a: int) -> str:\n    b: str = a\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::TypeMismatch);
        assert!(error.message().contains("str") && error.message().contains("int"));
    }

    // ---- literal and expression inference ---------------------------------

    /// Type a binding by name in the first function of a source.
    fn bound_ty(source: &str, want: &str) -> Ty {
        let f = lower_one(source);
        for stmt in &f.body {
            if let Stmt::Bind { name, ty, .. } = stmt
                && name == want
            {
                return *ty;
            }
        }
        panic!("no binding named {want}");
    }

    #[test]
    fn literal_initializers_are_inferred() {
        // The motivating cases from the proposal.
        assert_eq!(
            bound_ty("def f() -> str:\n    a = \"x\"\n    return a\n", "a"),
            Ty::Str
        );
        assert_eq!(
            bound_ty("def f() -> int:\n    b = 3\n    return b\n", "b"),
            Ty::Int
        );
        assert_eq!(
            bound_ty("def f() -> float:\n    c = 1.3\n    return c\n", "c"),
            Ty::Float
        );
        assert_eq!(
            bound_ty("def f() -> bool:\n    d = True\n    return d\n", "d"),
            Ty::Bool
        );
    }

    #[test]
    fn expression_initializers_are_inferred() {
        assert_eq!(
            bound_ty("def f(a: int) -> int:\n    b = a + 1\n    return b\n", "b"),
            Ty::Int
        );
        assert_eq!(
            bound_ty(
                "def f(a: int) -> bool:\n    b = a < 10\n    return b\n",
                "b"
            ),
            Ty::Bool
        );
        assert_eq!(
            bound_ty("def f(c: float) -> float:\n    b = -c\n    return b\n", "b"),
            Ty::Float
        );
        assert_eq!(
            bound_ty(
                "def f(a: int) -> int:\n    b = (a + 1) * 2 - 3\n    return b\n",
                "b"
            ),
            Ty::Int
        );
    }

    #[test]
    fn true_division_yields_float_while_floor_division_stays_int() {
        assert_eq!(
            bound_ty(
                "def f(a: int, b: int) -> float:\n    q = a / b\n    return q\n",
                "q"
            ),
            Ty::Float
        );
        assert_eq!(
            bound_ty(
                "def f(a: int, b: int) -> int:\n    q = a // b\n    return q\n",
                "q"
            ),
            Ty::Int
        );
    }

    #[test]
    fn string_concatenation_is_inferred() {
        assert_eq!(
            bound_ty(
                "def f(a: str, b: str) -> str:\n    c = a + b\n    return c\n",
                "c"
            ),
            Ty::Str
        );
    }

    #[test]
    fn mixed_arithmetic_promotes_and_records_the_conversion() {
        let f = lower_one("def f(a: int, b: float) -> float:\n    c = a + b\n    return c\n");
        match &f.body[0] {
            Stmt::Bind { ty, value, .. } => {
                assert_eq!(*ty, Ty::Float);
                // The integer operand must be wrapped, or a backend emitting operands
                // positionally would produce integer arithmetic.
                match value {
                    Expr::Binary { left, right, .. } => {
                        assert!(
                            matches!(**left, Expr::ToFloat(_)),
                            "int operand should be promoted, got {left:?}"
                        );
                        assert!(matches!(**right, Expr::Name(_)));
                    }
                    other => panic!("expected binary, got {other:?}"),
                }
            }
            other => panic!("expected bind, got {other:?}"),
        }
    }

    #[test]
    fn true_division_of_two_ints_promotes_both_operands() {
        let f = lower_one("def f(a: int, b: int) -> float:\n    q = a / b\n    return q\n");
        match &f.body[0] {
            Stmt::Bind {
                value: Expr::Binary { op, left, right },
                ..
            } => {
                assert_eq!(*op, BinOp::TrueDiv);
                assert!(matches!(**left, Expr::ToFloat(_)));
                assert!(matches!(**right, Expr::ToFloat(_)));
            }
            other => panic!("expected binary bind, got {other:?}"),
        }
    }

    #[test]
    fn mixed_comparison_is_permitted_and_yields_bool() {
        assert_eq!(
            bound_ty(
                "def f(a: int, b: float) -> bool:\n    c = a < b\n    return c\n",
                "c"
            ),
            Ty::Bool
        );
    }

    #[test]
    fn ill_typed_operands_are_rejected() {
        for (source, note) in [
            (
                "def f(a: str, b: int) -> str:\n    c = a + b\n    return c\n",
                "str + int",
            ),
            (
                "def f(a: bool, b: bool) -> int:\n    c = a + b\n    return c\n",
                "bool arithmetic",
            ),
            (
                "def f(a: str) -> str:\n    c = -a\n    return c\n",
                "negate str",
            ),
            (
                "def f(a: str, b: int) -> bool:\n    c = a < b\n    return c\n",
                "str < int",
            ),
        ] {
            let error = error_for(source);
            assert_eq!(
                error.kind(),
                LowerErrorKind::TypeMismatch,
                "{note} should be a type mismatch"
            );
        }
    }

    #[test]
    fn boolean_arithmetic_explains_itself() {
        let error = error_for("def f(a: bool, b: bool) -> int:\n    c = a + b\n    return c\n");
        assert!(
            error.message().contains("booleans are not numbers"),
            "message should explain the deliberate divergence, got: {}",
            error.message()
        );
    }

    #[test]
    fn call_makes_an_expression_undetermined_rather_than_ill_typed() {
        // The case a naive implementation gets wrong: it must demand an annotation, not
        // report a type error, when a call is buried inside arithmetic.
        let error = error_for("def f(a: int) -> int:\n    b = helper(a) + 1\n    return b\n");
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
        assert!(error.message().contains("call"));

        // ...and with an annotation it lowers fine, unchecked.
        let f = lower_one("def f(a: int) -> int:\n    b: int = helper(a) + 1\n    return b\n");
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Int, .. }));
    }

    // ---- declared versus inferred -----------------------------------------

    #[test]
    fn annotation_conflicting_with_the_initializer_is_rejected() {
        for source in [
            "def f() -> str:\n    b: str = 1\n    return b\n",
            "def f(a: int) -> str:\n    b: str = a\n    return b\n",
        ] {
            let error = error_for(source);
            assert_eq!(error.kind(), LowerErrorKind::TypeMismatch);
        }
    }

    #[test]
    fn widening_is_accepted_but_narrowing_is_not() {
        let f = lower_one("def f() -> float:\n    c: float = 1\n    return c\n");
        match &f.body[0] {
            Stmt::Bind { ty, value, .. } => {
                assert_eq!(*ty, Ty::Float);
                assert!(
                    matches!(value, Expr::ToFloat(_)),
                    "int should widen to float"
                );
            }
            other => panic!("expected bind, got {other:?}"),
        }

        let error = error_for("def f() -> int:\n    n: int = 1.5\n    return n\n");
        assert_eq!(error.kind(), LowerErrorKind::TypeMismatch);
    }

    #[test]
    fn float_annotations_are_accepted_everywhere() {
        let f = lower_one("def f(a: float) -> float:\n    b: float = a\n    return b\n");
        assert_eq!(f.params[0].ty, Ty::Float);
        assert_eq!(f.ret, Ty::Float);
        assert!(matches!(f.body[0], Stmt::Bind { ty: Ty::Float, .. }));
    }

    #[test]
    fn returned_value_is_checked_against_the_declared_type() {
        let wrong = error_for("def f() -> int:\n    return \"x\"\n");
        assert_eq!(wrong.kind(), LowerErrorKind::TypeMismatch);

        let from_unit = error_for("def f() -> None:\n    return 1\n");
        assert_eq!(from_unit.kind(), LowerErrorKind::TypeMismatch);

        // Widening applies to returns too.
        let widened = lower_one("def f() -> float:\n    return 1\n");
        assert_eq!(widened.body[0], Stmt::Return(Expr::int(1).to_float()));

        // A returned call is undetermined, so it is not checked.
        let unchecked = lower_one("def f(a: int) -> int:\n    return helper(a)\n");
        assert!(matches!(unchecked.body[0], Stmt::Return(Expr::Call { .. })));
    }

    // ---- diagnostics ------------------------------------------------------

    #[test]
    fn diagnostics_carry_a_useful_span() {
        let source = "def f(a: int) -> int:\n    x = a ** 2\n    return x\n";
        let error = lower(source).unwrap_err();
        let rendered = error.render(source);
        assert!(
            rendered.starts_with("2:"),
            "expected line 2, got {rendered}"
        );
    }

    #[test]
    fn first_violation_in_source_order_is_reported() {
        // Two violations: the unannotated call binding on line 2, the `if` on line 3.
        let source =
            "def f(a: int) -> int:\n    x = helper(a)\n    if a:\n        pass\n    return a\n";
        let error = lower(source).unwrap_err();
        assert_eq!(error.kind(), LowerErrorKind::MissingAnnotation);
    }

    #[test]
    fn lowering_never_panics_on_parsed_input() {
        for source in [
            "def f(a) -> int:\n    return a\n",
            "class C:\n    pass\n",
            "def f() -> int:\n    return undefined_name\n",
            "x = 1\n",
            "def f() -> float:\n    return 1.5\n",
        ] {
            let _ = lower(source);
        }
    }
}
