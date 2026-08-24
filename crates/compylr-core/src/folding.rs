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
//!
//! The first rule is where the checking mode comes in, and it is worth being precise about what
//! reading it does and does not change. A failing operation is left unfolded under **both** modes,
//! but for two different reasons — see `left_unfolded`, which is private and so is named here
//! as code rather than linked. The answer being the same today is a
//! fact about these two modes rather than a property of folding, which is why the decision is
//! made by matching on the mode rather than by ignoring it: a mode added later has to answer the
//! question here instead of inheriting an answer nobody wrote down.

use compylr_ir::{BinOp, Checked, DivMode, Expr, Literal, RemSign, Rounding, Stmt, Unit};

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
        Expr::Neg { value: inner, .. } | Expr::ToFloat(inner) | Expr::Not(inner) => {
            fold_expr(inner)
        }
        // Neither container node folds — a length or a subscript needs the value, not just its
        // type — but both have to be descended through, or a constant inside one survives.
        Expr::Len { value, .. } => fold_expr(value),
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
        Expr::Subscript { base, index, .. } => {
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
        Expr::Neg { value, checked } => match value.as_ref() {
            // `-i64::MIN` is the one negation that overflows, and it is left in place under
            // either mode — see `left_unfolded` for the two reasons.
            Expr::Literal(Literal::Int(value)) => match value.checked_neg() {
                Some(negated) => Some(Literal::Int(negated)),
                None => left_unfolded(*checked),
            },
            // Float negation cannot overflow, so the mode has nothing to govern here.
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

/// A fold that would fail is left alone, whichever checking mode the node declares.
///
/// The answer is the same under both and the reasons are not, which is why this matches rather
/// than returning a constant — and why a mode added later fails to compile here instead of
/// quietly inheriting one of these two arguments.
fn left_unfolded<T>(checked: Checked) -> Option<T> {
    match checked {
        // The failure is a value the program asked to observe. Folding it away would delete a
        // reported error, which is the module's first rule.
        Checked::Reported => None,
        // The program declined to define a result, so there is nothing to fold *to*. Any value
        // chosen here would be one particular target's answer written into a tree that every
        // backend reads — the one thing a mode may not do, since `Unchecked` is a statement about
        // the program and not about who compiles it.
        Checked::Unchecked => None,
    }
}

fn integer(op: BinOp, a: i64, b: i64) -> Option<Literal> {
    let value = match op {
        BinOp::Add { checked } => match a.checked_add(b) {
            Some(value) => value,
            None => return left_unfolded(checked),
        },
        BinOp::Sub { checked } => match a.checked_sub(b) {
            Some(value) => value,
            None => return left_unfolded(checked),
        },
        BinOp::Mul { checked } => match a.checked_mul(b) {
            Some(value) => value,
            None => return left_unfolded(checked),
        },
        BinOp::Div {
            mode: DivMode::Integer(rounding),
            checked,
        } => {
            if b == 0 {
                return left_unfolded(checked);
            }
            // `i64::MIN / -1` is the one quotient that does not fit, and it is a failure like any
            // other rather than a case to wrap.
            let Some(quotient) = a.checked_div(b) else {
                return left_unfolded(checked);
            };
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
        BinOp::Rem { sign, checked } => {
            if b == 0 {
                return left_unfolded(checked);
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
            ..
        } => return None,
        _ => return comparison(op, &a, &b),
    };
    Some(Literal::Int(value))
}

fn float(op: BinOp, a: f64, b: f64) -> Option<Literal> {
    let value = match op {
        // The overflow axis governs *integer* arithmetic; a float result that leaves the range
        // becomes an infinity, which `finite_float` already refuses to fold. So the mode has
        // nothing to govern on these three and is wildcarded deliberately rather than by
        // oversight.
        BinOp::Add { .. } => a + b,
        BinOp::Sub { .. } => a - b,
        BinOp::Mul { .. } => a * b,
        BinOp::Div {
            mode: DivMode::Exact,
            checked,
        } => {
            // A zero divisor *is* governed, on the exact-division axis. Under a stance that
            // leaves it undefined the IEEE result is an infinity, which does not fold either —
            // so the expression survives to the backend, which emits the target's own division.
            if b == 0.0 {
                return left_unfolded(checked);
            }
            a / b
        }
        BinOp::Div {
            mode: DivMode::Integer(rounding),
            checked,
        } => {
            if b == 0.0 {
                return left_unfolded(checked);
            }
            match rounding {
                Rounding::TowardNegInf => (a / b).floor(),
                Rounding::TowardZero => (a / b).trunc(),
            }
        }
        BinOp::Rem { sign, checked } => {
            if b == 0.0 {
                return left_unfolded(checked);
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
        // Concatenation has no integer range to leave, so the overflow axis says nothing about
        // it. Wildcarded on purpose: binding a mode here and pretending to consult it would be
        // the more misleading of the two.
        BinOp::Add { .. } => Some(Literal::Str(format!("{a}{b}"))),
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
    use compylr_ir::{Function, IndexOrigin, Param, TextUnits, Ty};

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
        let body = folded(vec![Stmt::Return(binop(
            BinOp::Add {
                checked: Checked::Reported,
            },
            int(2),
            int(3),
        ))]);
        assert_eq!(*returned(&body), int(5));
    }

    #[test]
    fn folding_is_bottom_up() {
        // `(1 + 2) * 3` must reach the multiplication with a literal on its left.
        let inner = binop(
            BinOp::Add {
                checked: Checked::Reported,
            },
            int(1),
            int(2),
        );
        let body = folded(vec![Stmt::Return(binop(
            BinOp::Mul {
                checked: Checked::Reported,
            },
            inner,
            int(3),
        ))]);
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
                checked: Checked::Reported,
            },
            int(7),
            int(-2),
        ))]);
        let truncating = folded(vec![Stmt::Return(binop(
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardZero),
                checked: Checked::Reported,
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
                checked: Checked::Reported,
            },
            int(-7),
            int(2),
        ))]);
        let dividend = folded(vec![Stmt::Return(binop(
            BinOp::Rem {
                sign: RemSign::Dividend,
                checked: Checked::Reported,
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
                checked: Checked::Reported,
            },
            Expr::ToFloat(Box::new(int(7))),
            Expr::ToFloat(Box::new(int(2))),
        ))]);
        assert_eq!(*returned(&body), Expr::Literal(Literal::float(3.5)));
    }

    #[test]
    fn a_non_literal_operand_is_left_alone() {
        let original = binop(
            BinOp::Add {
                checked: Checked::Reported,
            },
            Expr::name("a"),
            int(1),
        );
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    /// An error the program would have reported must still be reported.
    #[test]
    fn division_by_zero_is_not_folded_away() {
        for op in [
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
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
        let original = binop(
            BinOp::Add {
                checked: Checked::Reported,
            },
            int(i64::MAX),
            int(1),
        );
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    #[test]
    fn a_negation_that_overflows_is_left_in_place() {
        let original = Expr::Neg {
            value: Box::new(int(i64::MIN)),
            checked: Checked::Reported,
        };
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    /// Reassociation is not folding, and doing it would break a declared guarantee.
    #[test]
    fn a_partially_constant_float_expression_is_not_reassociated() {
        let inner = binop(
            BinOp::Add {
                checked: Checked::Reported,
            },
            Expr::name("a"),
            Expr::Literal(Literal::float(1.0)),
        );
        let original = binop(
            BinOp::Add {
                checked: Checked::Reported,
            },
            inner,
            Expr::Literal(Literal::float(2.0)),
        );
        let body = folded(vec![Stmt::Return(original.clone())]);
        assert_eq!(*returned(&body), original);
    }

    /// A value with no literal syntax must stay an operation.
    #[test]
    fn a_result_that_is_not_finite_is_left_in_place() {
        let original = binop(
            BinOp::Mul {
                checked: Checked::Reported,
            },
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
            (
                BinOp::Add {
                    checked: Checked::Reported,
                },
                1.5,
                2.25,
                3.75,
            ),
            (
                BinOp::Sub {
                    checked: Checked::Reported,
                },
                1.5,
                2.25,
                -0.75,
            ),
            (
                BinOp::Mul {
                    checked: Checked::Reported,
                },
                1.5,
                2.0,
                3.0,
            ),
            (
                BinOp::Div {
                    mode: DivMode::Exact,
                    checked: Checked::Reported,
                },
                7.0,
                2.0,
                3.5,
            ),
            (
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardNegInf),
                    checked: Checked::Reported,
                },
                -7.0,
                2.0,
                -4.0,
            ),
            (
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardZero),
                    checked: Checked::Reported,
                },
                -7.0,
                2.0,
                -3.0,
            ),
            (
                BinOp::Rem {
                    sign: RemSign::Divisor,
                    checked: Checked::Reported,
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
                checked: Checked::Reported,
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
                checked: Checked::Reported,
            },
            BinOp::Div {
                mode: DivMode::Integer(Rounding::TowardNegInf),
                checked: Checked::Reported,
            },
            BinOp::Rem {
                sign: RemSign::Divisor,
                checked: Checked::Reported,
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
        let negated = folded(vec![Stmt::Return(Expr::Neg {
            value: Box::new(int(3)),
            checked: Checked::Reported,
        })]);
        assert_eq!(*returned(&negated), int(-3));

        let float_negated = folded(vec![Stmt::Return(Expr::Neg {
            value: Box::new(float_lit(1.5)),
            checked: Checked::Reported,
        })]);
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
            let original = binop(
                BinOp::Sub {
                    checked: Checked::Reported,
                },
                left,
                right,
            );
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
        let sum = || {
            binop(
                BinOp::Add {
                    checked: Checked::Reported,
                },
                int(1),
                int(2),
            )
        };
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
                    origin: IndexOrigin::FromEitherEnd,
                    checked: Checked::Reported,
                },
            },
            Stmt::Bind {
                name: "len".to_string(),
                ty: Ty::Int,
                value: Expr::Len {
                    value: Box::new(Expr::name("xs")),
                    units: TextUnits::CodePoints,
                },
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
            BinOp::Add {
                checked: Checked::Reported,
            },
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
                then: vec![Stmt::Return(binop(
                    BinOp::Add {
                        checked: Checked::Reported,
                    },
                    int(1),
                    int(1),
                ))],
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

    /// Folding reads the checking mode, and the reasons it does differ per mode.
    ///
    /// A fold that would fail is left alone under both, which makes the *answer* the same and the
    /// reasoning different — see `left_unfolded`. What these pin is that the answer really is the
    /// same, because the tempting mistake in each direction is a wrong constant in otherwise
    /// correct output: no crash, no diagnostic, just one number that disagrees with the source.
    mod checking_mode {
        use super::*;

        fn add(checked: Checked, left: i64, right: i64) -> Vec<Stmt> {
            folded(vec![Stmt::Return(binop(
                BinOp::Add { checked },
                int(left),
                int(right),
            ))])
        }

        fn divide(checked: Checked, left: i64, right: i64) -> Vec<Stmt> {
            folded(vec![Stmt::Return(binop(
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardNegInf),
                    checked,
                },
                int(left),
                int(right),
            ))])
        }

        /// An overflowing constant expression survives to the backend under either mode.
        ///
        /// Under `Reported` because folding it away would delete a failure the program asked to
        /// observe. Under `Unchecked` because the program declined to define a result, so there
        /// is no value to fold *to* — any choice would be one target's answer written into a tree
        /// that every backend reads.
        #[test]
        fn an_overflowing_expression_is_left_unfolded_under_either_mode() {
            for checked in [Checked::Reported, Checked::Unchecked] {
                let body = add(checked, i64::MAX, 1);
                assert!(
                    matches!(returned(&body), Expr::Binary { .. }),
                    "{checked:?}: an overflowing fold must leave the operation in place, got {:?}",
                    returned(&body)
                );
            }
        }

        #[test]
        fn a_zero_divisor_is_left_unfolded_under_either_mode() {
            for checked in [Checked::Reported, Checked::Unchecked] {
                let body = divide(checked, 1, 0);
                assert!(
                    matches!(returned(&body), Expr::Binary { .. }),
                    "{checked:?}: a zero divisor must leave the operation in place"
                );
            }
        }

        /// Negating the least representable integer is the one negation that overflows.
        #[test]
        fn an_overflowing_negation_is_left_unfolded_under_either_mode() {
            for checked in [Checked::Reported, Checked::Unchecked] {
                let body = folded(vec![Stmt::Return(Expr::Neg {
                    value: Box::new(int(i64::MIN)),
                    checked,
                })]);
                assert!(matches!(returned(&body), Expr::Neg { .. }), "{checked:?}");
            }
        }

        /// An unchecked operation that *cannot* fail still folds.
        ///
        /// The other half, and the one a nervous implementation gets wrong: refusing to fold
        /// anything unchecked would be safe and would quietly stop the pass doing its job for
        /// every program that asked for the target's arithmetic.
        #[test]
        fn an_unchecked_fold_that_cannot_fail_still_folds() {
            assert_eq!(*returned(&add(Checked::Unchecked, 2, 3)), int(5));
            assert_eq!(*returned(&divide(Checked::Unchecked, -7, 2)), int(-4));
            assert_eq!(
                *returned(&folded(vec![Stmt::Return(Expr::Neg {
                    value: Box::new(int(7)),
                    checked: Checked::Unchecked,
                })])),
                int(-7)
            );
        }

        /// The rounding mode is still honoured under an unchecked division.
        ///
        /// `Div { mode: Integer(TowardNegInf), checked: Unchecked }` is a real combination — a
        /// flooring division whose zero divisor is undefined — and folding it as truncation would
        /// produce `-3` where the program says `-4`.
        #[test]
        fn an_unchecked_division_still_folds_by_its_declared_rounding() {
            assert_eq!(*returned(&divide(Checked::Unchecked, -7, 2)), int(-4));

            let truncating = folded(vec![Stmt::Return(binop(
                BinOp::Div {
                    mode: DivMode::Integer(Rounding::TowardZero),
                    checked: Checked::Unchecked,
                },
                int(-7),
                int(2),
            ))]);
            assert_eq!(*returned(&truncating), int(-3));
        }
    }
}
