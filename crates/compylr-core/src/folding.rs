//! Constant folding: evaluate what is already known.
//!
//! The one optimization compylr ships, and it is here as much for what it proves as for what it
//! saves. Folding `7 // -2` correctly is impossible without reading the rounding mode off the
//! node — get it from the operator's *name* and you produce `-3` where the source said `-4`. A
//! pass that gets this right is a working demonstration that the semantics really do travel with
//! the tree.
//!
//! Two rules constrain it, and both are about not being clever:
//!
//! * **An error is not optimized away.** `1 // 0` and `i64::MAX + 1` are left exactly as written,
//!   so the failure still reaches the caller. A compiler that folded them into nothing would have
//!   turned a reported error into a missing one.
//! * **Nothing is reassociated.** Only an operation whose operands are *both* literals folds.
//!   `(a + 1.0) + 2.0` stays as it is, because rewriting it to `a + 3.0` changes the result under
//!   floating-point arithmetic — and preserving that is a guarantee frontends declare.

use compylr_ir::{BinOp, DivMode, Expr, Literal, RemSign, Rounding, Stmt, Unit};

use crate::pass::{Pass, PassError};

/// The constant-folding pass.
#[derive(Debug)]
pub struct ConstantFolding;

/// The name this pass is selected and reported by.
pub const NAME: &str = "constant-folding";

impl Pass for ConstantFolding {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(&self, unit: &mut Unit) -> Result<(), PassError> {
        unit.map_functions(|function| fold_stmts(&mut function.body));
        Ok(())
    }
}

fn fold_stmts(stmts: &mut [Stmt]) {
    for stmt in stmts {
        match stmt {
            Stmt::Return(value) | Stmt::Effect(value) => fold_expr(value),
            Stmt::Bind { value, .. } | Stmt::Assign { value, .. } => fold_expr(value),
            Stmt::SetAttr { object, value, .. } => {
                fold_expr(object);
                fold_expr(value);
            }
            Stmt::SetItem {
                collection,
                index,
                value,
            } => {
                fold_expr(collection);
                fold_expr(index);
                fold_expr(value);
            }
            Stmt::Append { sequence, value } => {
                fold_expr(sequence);
                fold_expr(value);
            }
            Stmt::If {
                test,
                then,
                otherwise,
            } => {
                fold_expr(test);
                fold_stmts(then);
                fold_stmts(otherwise);
            }
            Stmt::While { test, body } => {
                fold_expr(test);
                fold_stmts(body);
            }
            Stmt::For { iter, body, .. } => {
                fold_expr(iter);
                fold_stmts(body);
            }
            Stmt::ReturnUnit | Stmt::Break | Stmt::Continue => {}
        }
    }
}

/// Fold `expr` in place, bottom up.
///
/// Children first, so `(1 + 2) * 3` reaches the multiplication with a literal on its left.
fn fold_expr(expr: &mut Expr) {
    match expr {
        Expr::Literal(_) | Expr::Name(_) => return,
        Expr::Neg(inner) | Expr::ToFloat(inner) | Expr::Not(inner) | Expr::Len(inner) => {
            fold_expr(inner)
        }
        Expr::Binary { left, right, .. } => {
            fold_expr(left);
            fold_expr(right);
        }
        Expr::ListLit(items) | Expr::SetLit(items) | Expr::TupleLit(items) => {
            items.iter_mut().for_each(fold_expr)
        }
        Expr::DictLit(entries) => {
            for (key, value) in entries {
                fold_expr(key);
                fold_expr(value);
            }
        }
        Expr::TupleIndex { base, .. } | Expr::Attribute { object: base, .. } => fold_expr(base),
        Expr::Subscript { base, index } => {
            fold_expr(base);
            fold_expr(index);
        }
        Expr::Contains { value, container } => {
            fold_expr(value);
            fold_expr(container);
        }
        Expr::Range { start, stop, step } => {
            fold_expr(start);
            fold_expr(stop);
            fold_expr(step);
        }
        Expr::Call { args, .. } | Expr::Construct { args, .. } => {
            args.iter_mut().for_each(fold_expr)
        }
        Expr::MethodCall { receiver, args, .. } => {
            fold_expr(receiver);
            args.iter_mut().for_each(fold_expr);
        }
    }

    if let Some(folded) = evaluate(expr) {
        *expr = Expr::Literal(folded);
    }
}

/// The value of `expr`, when every operand is already known and the result is representable.
///
/// `None` means "leave it alone", which is the answer for anything uncertain: an operand that is
/// not a literal, an operation that would fail at runtime, or a result outside what the target
/// type can hold. Uncertainty must not become a rewrite.
fn evaluate(expr: &Expr) -> Option<Literal> {
    match expr {
        Expr::ToFloat(inner) => match inner.as_ref() {
            Expr::Literal(Literal::Int(value)) => finite_float(*value as f64),
            Expr::Literal(Literal::Float(bits)) => Some(Literal::Float(*bits)),
            _ => None,
        },
        Expr::Neg(inner) => match inner.as_ref() {
            // `-i64::MIN` is the one negation that overflows. Left in place, so the failure the
            // program would have reported still happens.
            Expr::Literal(Literal::Int(value)) => value.checked_neg().map(Literal::Int),
            Expr::Literal(Literal::Float(bits)) => finite_float(-f64::from_bits(*bits)),
            _ => None,
        },
        Expr::Not(inner) => match inner.as_ref() {
            Expr::Literal(Literal::Bool(value)) => Some(Literal::Bool(!value)),
            _ => None,
        },
        Expr::Binary { op, left, right } => {
            let (Expr::Literal(left), Expr::Literal(right)) = (left.as_ref(), right.as_ref())
            else {
                return None;
            };
            binary(*op, left, right)
        }
        _ => None,
    }
}

fn binary(op: BinOp, left: &Literal, right: &Literal) -> Option<Literal> {
    match (left, right) {
        (Literal::Int(a), Literal::Int(b)) => integer(op, *a, *b),
        (Literal::Float(a), Literal::Float(b)) => float(op, f64::from_bits(*a), f64::from_bits(*b)),
        (Literal::Str(a), Literal::Str(b)) => string(op, a, b),
        (Literal::Bool(a), Literal::Bool(b)) => match op {
            BinOp::Eq => Some(Literal::Bool(a == b)),
            BinOp::NotEq => Some(Literal::Bool(a != b)),
            _ => None,
        },
        // Mixed kinds cannot arise: lowering inserts an explicit promotion, and the promotion
        // itself folds first. Refusing rather than coercing keeps that an invariant instead of a
        // second place types are decided.
        _ => None,
    }
}

fn integer(op: BinOp, a: i64, b: i64) -> Option<Literal> {
    let value = match op {
        BinOp::Add => a.checked_add(b)?,
        BinOp::Sub => a.checked_sub(b)?,
        BinOp::Mul => a.checked_mul(b)?,
        BinOp::Div {
            mode: DivMode::Integer(rounding),
        } => {
            if b == 0 {
                return None;
            }
            let quotient = a.checked_div(b)?;
            match rounding {
                Rounding::TowardZero => quotient,
                Rounding::TowardNegInf => {
                    let remainder = a % b;
                    if remainder != 0 && ((remainder < 0) != (b < 0)) {
                        quotient - 1
                    } else {
                        quotient
                    }
                }
            }
        }
        BinOp::Rem { sign } => {
            if b == 0 {
                return None;
            }
            // `i64::MIN % -1` overflows in Rust though the answer, 0, is representable.
            if b == -1 {
                0
            } else {
                let remainder = a % b;
                match sign {
                    RemSign::Dividend => remainder,
                    RemSign::Divisor => {
                        if remainder != 0 && ((remainder < 0) != (b < 0)) {
                            remainder + b
                        } else {
                            remainder
                        }
                    }
                }
            }
        }
        // Exact division on two integers cannot occur: lowering promotes both operands first.
        BinOp::Div {
            mode: DivMode::Exact,
        } => return None,
        _ => return comparison(op, &a, &b),
    };
    Some(Literal::Int(value))
}

fn float(op: BinOp, a: f64, b: f64) -> Option<Literal> {
    let value = match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div {
            mode: DivMode::Exact,
        } => {
            if b == 0.0 {
                return None;
            }
            a / b
        }
        BinOp::Div {
            mode: DivMode::Integer(rounding),
        } => {
            if b == 0.0 {
                return None;
            }
            match rounding {
                Rounding::TowardNegInf => (a / b).floor(),
                Rounding::TowardZero => (a / b).trunc(),
            }
        }
        BinOp::Rem { sign } => {
            if b == 0.0 {
                return None;
            }
            let remainder = a % b;
            match sign {
                RemSign::Dividend => remainder,
                RemSign::Divisor => {
                    if remainder != 0.0 && ((remainder < 0.0) != (b < 0.0)) {
                        remainder + b
                    } else {
                        remainder
                    }
                }
            }
        }
        _ => return comparison(op, &a, &b),
    };
    finite_float(value)
}

fn string(op: BinOp, a: &str, b: &str) -> Option<Literal> {
    match op {
        BinOp::Add => Some(Literal::Str(format!("{a}{b}"))),
        _ => comparison(op, &a, &b),
    }
}

fn comparison<T: PartialOrd>(op: BinOp, a: &T, b: &T) -> Option<Literal> {
    let value = match op {
        BinOp::Eq => a == b,
        BinOp::NotEq => a != b,
        BinOp::Lt => a < b,
        BinOp::LtE => a <= b,
        BinOp::Gt => a > b,
        BinOp::GtE => a >= b,
        _ => return None,
    };
    Some(Literal::Bool(value))
}

/// A float literal, but only if the value is one a target can spell.
///
/// Infinity and NaN have no literal syntax in most targets, so folding into one would produce
/// source that does not compile. Leaving the operation in place keeps the arithmetic at runtime,
/// where the target's own rules apply.
fn finite_float(value: f64) -> Option<Literal> {
    value.is_finite().then(|| Literal::float(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use compylr_diagnostics::span::Span;
    use compylr_ir::{Function, Param, Ty};

    fn folded(body: Vec<Stmt>) -> Vec<Stmt> {
        let mut unit = Unit::new();
        unit.add_function(Function {
            name: "f".to_string(),
            params: vec![Param {
                name: "a".to_string(),
                ty: Ty::Int,
            }],
            ret: Ty::Int,
            body,
            doc: None,
            span: Span::default(),
        })
        .unwrap();
        ConstantFolding.run(&mut unit).unwrap();
        unit.get("f").unwrap().body.clone()
    }

    fn int(value: i64) -> Expr {
        Expr::Literal(Literal::Int(value))
    }

    fn binop(op: BinOp, left: Expr, right: Expr) -> Expr {
        Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn returned(body: &[Stmt]) -> &Expr {
        match &body[0] {
            Stmt::Return(value) => value,
            other => panic!("expected a return, got {other:?}"),
        }
    }

    #[test]
    fn arithmetic_on_literals_folds() {
        let body = folded(vec![Stmt::Return(binop(BinOp::Add, int(2), int(3)))]);
        assert_eq!(*returned(&body), int(5));
    }

    #[test]
    fn folding_is_bottom_up() {
        // `(1 + 2) * 3` must reach the multiplication with a literal on its left.
        let inner = binop(BinOp::Add, int(1), int(2));
        let body = folded(vec![Stmt::Return(binop(BinOp::Mul, inner, int(3)))]);
        assert_eq!(*returned(&body), int(9));
    }

    /// The assertion the whole change exists for.
    ///
    /// `7 // -2` is `-4` under Python's rounding and `-3` under everyone else's. A folder that
    /// read the operator's name rather than its mode would produce one of them for both.
    #[test]
    fn the_same_operands_fold_differently_under_each_rounding_mode() {
        let flooring = folded(vec![Stmt::Return(binop(
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
            },
            int(7),
            int(-2),
        ))]);
        let truncating = folded(vec![Stmt::Return(binop(
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardZero),
            },
            int(7),
            int(-2),
        ))]);

        assert_eq!(*returned(&flooring), int(-4));
        assert_eq!(*returned(&truncating), int(-3));
    }

    #[test]
    fn the_same_operands_fold_differently_under_each_remainder_convention() {
        let divisor = folded(vec![Stmt::Return(binop(
            BinOp::Rem {
                sign: RemSign::Divisor,
            },
            int(-7),
            int(2),
        ))]);
        let dividend = folded(vec![Stmt::Return(binop(
            BinOp::Rem {
                sign: RemSign::Dividend,
            },
            int(-7),
            int(2),
        ))]);

        assert_eq!(*returned(&divisor), int(1));
        assert_eq!(*returned(&dividend), int(-1));
    }

    #[test]
    fn exact_division_of_two_integers_folds_to_a_float() {
        // As lowering hands it over: both operands promoted, so the promotions fold first.
        let body = folded(vec![Stmt::Return(binop(
            BinOp::Div {
                mode: DivMode::Exact,
            },
            Expr::ToFloat(Box::new(int(7))),
            Expr::ToFloat(Box::new(int(2))),
        ))]);
        assert_eq!(*returned(&body), Expr::Literal(Literal::float(3.5)));
    }

    #[test]
    fn a_non_literal_operand_is_left_alone() {
        let original = binop(BinOp::Add, Expr::name("a"), int(1));
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    /// An error the program would have reported must still be reported.
    #[test]
    fn division_by_zero_is_not_folded_away() {
        for op in [
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
            },
        ] {
            let original = binop(op, int(1), int(0));
            let body = folded(vec![Stmt::Return(original.clone())]);
            assert_eq!(
                *returned(&body),
                original,
                "folding {op} by zero would replace a reported failure with a missing one"
            );
        }
    }

    #[test]
    fn an_overflowing_constant_is_not_folded_away() {
        let original = binop(BinOp::Add, int(i64::MAX), int(1));
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    #[test]
    fn a_negation_that_overflows_is_left_in_place() {
        let original = Expr::Neg(Box::new(int(i64::MIN)));
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    /// Reassociation is not folding, and doing it would break a declared guarantee.
    #[test]
    fn a_partially_constant_float_expression_is_not_reassociated() {
        let inner = binop(
            BinOp::Add,
            Expr::name("a"),
            Expr::Literal(Literal::float(1.0)),
        );
        let original = binop(BinOp::Add, inner, Expr::Literal(Literal::float(2.0)));
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    /// A value with no literal syntax must stay an operation.
    #[test]
    fn a_result_that_is_not_finite_is_left_in_place() {
        let original = binop(
            BinOp::Mul,
            Expr::Literal(Literal::float(f64::MAX)),
            Expr::Literal(Literal::float(2.0)),
        );
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    fn float_lit(value: f64) -> Expr {
        Expr::Literal(Literal::float(value))
    }

    /// Floating-point arithmetic folds under every operator, including the modes with two
    /// readings.
    ///
    /// Worth covering separately from the integer paths: the corrections differ — flooring a
    /// float quotient is `floor()`, not a quotient adjustment — so a shared test would prove
    /// neither.
    #[test]
    fn float_arithmetic_folds_under_every_mode() {
        let cases: [(BinOp, f64, f64, f64); 7] = [
            (BinOp::Add, 1.5, 2.25, 3.75),
            (BinOp::Sub, 1.5, 2.25, -0.75),
            (BinOp::Mul, 1.5, 2.0, 3.0),
            (
                BinOp::Div {
                    mode: DivMode::Exact,
                },
                7.0,
                2.0,
                3.5,
            ),
            (
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardNegInf),
                },
                -7.0,
                2.0,
                -4.0,
            ),
            (
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardZero),
                },
                -7.0,
                2.0,
                -3.0,
            ),
            (
                BinOp::Rem {
                    sign: RemSign::Divisor,
                },
                -7.0,
                2.0,
                1.0,
            ),
        ];

        for (op, a, b, expected) in cases {
            let body = folded(vec![Stmt::Return(binop(op, float_lit(a), float_lit(b)))]);
            assert_eq!(*returned(&body), float_lit(expected), "{op} on {a} and {b}");
        }

        let dividend = folded(vec![Stmt::Return(binop(
            BinOp::Rem {
                sign: RemSign::Dividend,
            },
            float_lit(-7.0),
            float_lit(2.0),
        ))]);
        assert_eq!(*returned(&dividend), float_lit(-1.0));
    }

    #[test]
    fn dividing_a_float_by_zero_is_not_folded_away() {
        for op in [
            BinOp::Div {
                mode: DivMode::Exact,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
            },
        ] {
            let original = binop(op, float_lit(1.0), float_lit(0.0));
            let body = folded(vec![Stmt::Return(original.clone())]);
            assert_eq!(*returned(&body), original, "{op} by zero");
        }
    }

    /// A promotion of a value that is already floating point is still a promotion.
    #[test]
    fn promoting_a_float_folds_to_the_same_value() {
        let body = folded(vec![Stmt::Return(Expr::ToFloat(Box::new(float_lit(2.5))))]);
        assert_eq!(*returned(&body), float_lit(2.5));
    }

    #[test]
    fn negation_and_logical_not_fold() {
        let negated = folded(vec![Stmt::Return(Expr::Neg(Box::new(int(3))))]);
        assert_eq!(*returned(&negated), int(-3));

        let float_negated = folded(vec![Stmt::Return(Expr::Neg(Box::new(float_lit(1.5))))]);
        assert_eq!(*returned(&float_negated), float_lit(-1.5));

        let flipped = folded(vec![Stmt::Return(Expr::Not(Box::new(Expr::Literal(
            Literal::Bool(true),
        ))))]);
        assert_eq!(*returned(&flipped), Expr::Literal(Literal::Bool(false)));
    }

    #[test]
    fn booleans_and_strings_compare() {
        let equal = folded(vec![Stmt::Return(binop(
            BinOp::Eq,
            Expr::Literal(Literal::Bool(true)),
            Expr::Literal(Literal::Bool(true)),
        ))]);
        assert_eq!(*returned(&equal), Expr::Literal(Literal::Bool(true)));

        let ordered = folded(vec![Stmt::Return(binop(
            BinOp::Lt,
            Expr::Literal(Literal::Str("a".into())),
            Expr::Literal(Literal::Str("b".into())),
        ))]);
        assert_eq!(*returned(&ordered), Expr::Literal(Literal::Bool(true)));
    }

    /// An operator with no meaning for a pair of operands must be left alone, not guessed at.
    #[test]
    fn an_operation_with_no_defined_result_is_left_in_place() {
        // Subtraction of two booleans, and arithmetic across kinds. Neither can arise from an
        // accepted program; leaving them alone keeps that an invariant rather than a coercion.
        for (left, right) in [
            (
                Expr::Literal(Literal::Bool(true)),
                Expr::Literal(Literal::Bool(false)),
            ),
            (int(1), float_lit(1.0)),
        ] {
            let original = binop(BinOp::Sub, left, right);
            let body = folded(vec![Stmt::Return(original.clone())]);
            assert_eq!(*returned(&body), original);
        }
    }

    /// Folding must descend through every expression form, not only the ones it can fold.
    ///
    /// A form missed in the traversal is a constant that silently survives inside it — invisible,
    /// because the surrounding expression still emits correctly.
    #[test]
    fn folding_descends_through_every_container_form() {
        let sum = || binop(BinOp::Add, int(1), int(2));
        let body = folded(vec![
            Stmt::Bind {
                name: "xs".to_string(),
                ty: Ty::List(Box::new(Ty::Int)),
                value: Expr::ListLit(vec![sum()]),
            },
            Stmt::Bind {
                name: "d".to_string(),
                ty: Ty::Dict(Box::new(Ty::Int), Box::new(Ty::Int)),
                value: Expr::DictLit(vec![(sum(), sum())]),
            },
            Stmt::Bind {
                name: "st".to_string(),
                ty: Ty::Set(Box::new(Ty::Int)),
                value: Expr::SetLit(vec![sum()]),
            },
            Stmt::Bind {
                name: "t".to_string(),
                ty: Ty::Tuple(vec![Ty::Int]),
                value: Expr::TupleLit(vec![sum()]),
            },
            Stmt::Bind {
                name: "n".to_string(),
                ty: Ty::Int,
                value: Expr::Subscript {
                    base: Box::new(Expr::name("xs")),
                    index: Box::new(sum()),
                },
            },
            Stmt::Bind {
                name: "len".to_string(),
                ty: Ty::Int,
                value: Expr::Len(Box::new(Expr::name("xs"))),
            },
            Stmt::Bind {
                name: "has".to_string(),
                ty: Ty::Bool,
                value: Expr::Contains {
                    value: Box::new(sum()),
                    container: Box::new(Expr::name("xs")),
                },
            },
            Stmt::For {
                name: "i".to_string(),
                ty: Ty::Int,
                iter: Expr::Range {
                    start: Box::new(sum()),
                    stop: Box::new(sum()),
                    step: Box::new(sum()),
                },
                body: vec![Stmt::Effect(Expr::Call {
                    callee: "f".to_string(),
                    args: vec![sum()],
                })],
            },
            Stmt::While {
                test: binop(BinOp::Lt, int(0), int(1)),
                body: vec![Stmt::Append {
                    sequence: Expr::name("xs"),
                    value: sum(),
                }],
            },
            Stmt::SetItem {
                collection: Expr::name("d"),
                index: sum(),
                value: sum(),
            },
            Stmt::Return(Expr::TupleIndex {
                base: Box::new(Expr::name("t")),
                position: 0,
            }),
        ]);

        let rendered = format!("{body:?}");
        assert!(
            !rendered.contains("Add"),
            "a foldable constant survived the traversal:\n{rendered}"
        );
    }

    #[test]
    fn comparisons_and_concatenation_fold() {
        let less = folded(vec![Stmt::Return(binop(BinOp::Lt, int(1), int(2)))]);
        assert_eq!(*returned(&less), Expr::Literal(Literal::Bool(true)));

        let joined = folded(vec![Stmt::Return(binop(
            BinOp::Add,
            Expr::Literal(Literal::Str("a".into())),
            Expr::Literal(Literal::Str("b".into())),
        ))]);
        assert_eq!(*returned(&joined), Expr::Literal(Literal::Str("ab".into())));
    }

    /// Folding must reach everywhere an expression can appear, not just returns.
    #[test]
    fn folding_reaches_into_nested_statements() {
        let body = folded(vec![
            Stmt::If {
                test: binop(BinOp::Lt, int(1), int(2)),
                then: vec![Stmt::Return(binop(BinOp::Add, int(1), int(1)))],
                otherwise: vec![],
            },
            Stmt::Return(int(0)),
        ]);
        match &body[0] {
            Stmt::If { test, then, .. } => {
                assert_eq!(*test, Expr::Literal(Literal::Bool(true)));
                assert_eq!(*returned(then), int(2));
            }
            other => panic!("expected an if, got {other:?}"),
        }
    }
}
