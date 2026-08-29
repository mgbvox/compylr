//! Measuring how far two units are from being the same program.
//!
//! Two halves, and the separation between them is the point:
//!
//! * [`normalize`] produces the form units are *compared* in. It standardizes orderings that carry
//!   no meaning, and it is never compiled — a program must not change because the project measures
//!   it. Nothing here is registered as a [`Pass`](crate::pass::Pass), deliberately: a pass runs on
//!   the way to a backend, and `ir-optimization` already requires that a pass preserve observable
//!   behavior, which reordering the operands of `f() + g()` does not.
//! * [`divergence`] scores the remaining difference. It disregards everything the IR carries on
//!   purpose — the resolved semantic modes, source spans, and documentation — so that what is left
//!   is disagreement between *frontends* rather than a restatement of the fact that the two sources
//!   were written in different languages.
//!
//! The score is a structural edit distance, not a boolean. Two frontends that disagree about one
//! node should not look the same as two that disagree about a whole function body, because the
//! measurement exists to be driven down and a boolean gives nothing to drive.

use std::collections::BTreeSet;

use compylr_ir::{BinOp, Class, DivMode, Expr, Function, Literal, Stmt, Ty, Unit};

/// A node in the comparison shape of a member.
///
/// Deliberately untyped: the shape exists to be compared to another shape, and giving it the IR's
/// own structure would mean a differ that has to be extended in lockstep with every IR form. A
/// label and children is the whole contract, and building one is the only place that knows what an
/// IR node contributes to a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// What this node is, already stripped of everything the comparison disregards.
    label: String,
    /// Children, in an order that is part of the shape.
    children: Vec<Shape>,
}

impl Shape {
    /// Build a node.
    fn node(label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            label: label.into(),
            children,
        }
    }

    /// Build a childless node.
    fn leaf(label: impl Into<String>) -> Self {
        Self::node(label, Vec::new())
    }

    /// What this node is.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// This node's children.
    pub fn children(&self) -> &[Shape] {
        &self.children
    }

    /// The number of nodes in this subtree, which is what deleting it costs.
    fn size(&self) -> u32 {
        1 + self.children.iter().map(Shape::size).sum::<u32>()
    }

    /// A deterministic rendering, used to order operands that may be reordered.
    ///
    /// Any total order would do; this one is stable across runs and readable in a failure.
    fn render(&self) -> String {
        if self.children.is_empty() {
            return self.label.clone();
        }
        let inner: Vec<String> = self.children.iter().map(Shape::render).collect();
        format!("{}({})", self.label, inner.join(","))
    }
}

/// How far two units are from being the same program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    score: u32,
    members: Vec<MemberDivergence>,
}

impl Divergence {
    /// The total score. Zero means the two units are the same program under this comparison.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Whether the two units agree.
    pub fn is_zero(&self) -> bool {
        self.score == 0
    }

    /// Per-member scores, in name order, including members only one side defines.
    pub fn members(&self) -> &[MemberDivergence] {
        &self.members
    }

    /// The members that account for the score, in name order.
    pub fn divergent(&self) -> impl Iterator<Item = &MemberDivergence> {
        self.members.iter().filter(|member| member.score > 0)
    }
}

/// What one member contributed to a divergence, and why.
///
/// The notes are not decoration. A bare number says the frontends disagree without saying where,
/// and a measurement nobody can act on will not be driven down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberDivergence {
    name: String,
    score: u32,
    notes: Vec<String>,
}

impl MemberDivergence {
    /// The member's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// What this member contributed to the total.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// What differed within it, deepest-first along the path the comparison took.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
}

/// Produce the normalized form of `unit`, for comparison only.
///
/// The argument is borrowed and the result is a new unit: normalization must not reach a backend,
/// and returning a copy is what makes that structural rather than a rule someone has to remember.
/// The original's fingerprint is unaffected, because the original is unaffected.
pub fn normalize(unit: &Unit) -> Unit {
    let mut normalized = unit.clone();
    // `map_functions` rather than a walk over `functions()`, so constructors and methods are
    // normalized too. A normalizer that silently skipped method bodies would report agreement
    // between two classes on the strength of never having looked inside them.
    normalized.map_functions(|function| {
        function.body = normalize_block(&function.body);
    });
    normalized
}

/// Score the structural divergence between two units.
///
/// Members are matched by name. A member only one side defines contributes its whole size: the
/// other frontend does not express that program at all, and calling that zero would let a corpus
/// score well by staying small.
pub fn divergence(left: &Unit, right: &Unit) -> Divergence {
    let mut names: BTreeSet<&str> = BTreeSet::new();
    names.extend(left.functions().map(|function| function.name.as_str()));
    names.extend(right.functions().map(|function| function.name.as_str()));
    names.extend(left.classes().map(|class| class.name.as_str()));
    names.extend(right.classes().map(|class| class.name.as_str()));

    let mut members = Vec::new();
    let mut total = 0;
    for name in names {
        let member = match (shape_member(left, name), shape_member(right, name)) {
            (Some(a), Some(b)) => {
                let mut notes = Vec::new();
                let score = distance(&a, &b, name, &mut notes);
                MemberDivergence {
                    name: name.to_string(),
                    score,
                    notes,
                }
            }
            (Some(a), None) => only_one_side(name, &a, "left"),
            (None, Some(b)) => only_one_side(name, &b, "right"),
            // Unreachable: the name came from one of the two units.
            (None, None) => continue,
        };
        total += member.score;
        members.push(member);
    }

    Divergence {
        score: total,
        members,
    }
}

/// Score one member against another, when both sides define it.
///
/// Exposed because pairing members across two units is the caller's business — a cross-language
/// corpus pairs by name, and a test of one function pairs by having built both by hand.
pub fn member_divergence(left: &Function, right: &Function) -> MemberDivergence {
    let mut notes = Vec::new();
    let score = distance(
        &shape_function(left),
        &shape_function(right),
        &left.name,
        &mut notes,
    );
    MemberDivergence {
        name: left.name.clone(),
        score,
        notes,
    }
}

/// The record for a member only one unit defines.
fn only_one_side(name: &str, shape: &Shape, side: &str) -> MemberDivergence {
    MemberDivergence {
        name: name.to_string(),
        score: shape.size(),
        notes: vec![format!("{name}: defined only on the {side}")],
    }
}

/// The shape of whichever member `name` refers to, if either kind defines it.
fn shape_member(unit: &Unit, name: &str) -> Option<Shape> {
    if let Some(function) = unit.get(name) {
        return Some(shape_function(function));
    }
    unit.class(name).map(shape_class)
}

// ---------------------------------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------------------------------

/// Normalize a block: its statements first, then the order of the statements themselves.
fn normalize_block(stmts: &[Stmt]) -> Vec<Stmt> {
    let mut normalized: Vec<Stmt> = stmts.iter().map(normalize_stmt).collect();
    sort_independent_binds(&mut normalized);
    normalized
}

/// Normalize one statement, recursing into its expressions and nested blocks.
fn normalize_stmt(stmt: &Stmt) -> Stmt {
    match stmt {
        Stmt::Return(value) => Stmt::Return(normalize_expr(value)),
        Stmt::ReturnUnit => Stmt::ReturnUnit,
        Stmt::Bind { name, ty, value } => Stmt::Bind {
            name: name.clone(),
            ty: ty.clone(),
            value: normalize_expr(value),
        },
        Stmt::Assign { name, ty, value } => Stmt::Assign {
            name: name.clone(),
            ty: ty.clone(),
            value: normalize_expr(value),
        },
        Stmt::Effect(value) => Stmt::Effect(normalize_expr(value)),
        Stmt::SetAttr {
            object,
            name,
            ty,
            value,
        } => Stmt::SetAttr {
            object: normalize_expr(object),
            name: name.clone(),
            ty: ty.clone(),
            value: normalize_expr(value),
        },
        Stmt::SetItem {
            collection,
            index,
            value,
        } => Stmt::SetItem {
            collection: normalize_expr(collection),
            index: normalize_expr(index),
            value: normalize_expr(value),
        },
        Stmt::Append { sequence, value } => Stmt::Append {
            sequence: normalize_expr(sequence),
            value: normalize_expr(value),
        },
        Stmt::If {
            test,
            then,
            otherwise,
        } => Stmt::If {
            test: normalize_expr(test),
            then: normalize_block(then),
            otherwise: normalize_block(otherwise),
        },
        Stmt::While { test, body } => Stmt::While {
            test: normalize_expr(test),
            body: normalize_block(body),
        },
        Stmt::For {
            name,
            ty,
            iter,
            body,
        } => Stmt::For {
            name: name.clone(),
            ty: ty.clone(),
            iter: normalize_expr(iter),
            body: normalize_block(body),
        },
        Stmt::Break => Stmt::Break,
        Stmt::Continue => Stmt::Continue,
    }
}

/// Normalize one expression, ordering commutative operands where that is honest.
fn normalize_expr(expr: &Expr) -> Expr {
    match expr {
        Expr::Literal(value) => Expr::Literal(value.clone()),
        Expr::Name(name) => Expr::Name(name.clone()),
        Expr::Neg { value, checked } => Expr::Neg {
            value: Box::new(normalize_expr(value)),
            checked: *checked,
        },
        Expr::ToFloat(value) => Expr::ToFloat(Box::new(normalize_expr(value))),
        Expr::Binary { op, left, right } => {
            let left = normalize_expr(left);
            let right = normalize_expr(right);
            // Reordering is refused wherever an operand can call, and the reason is honesty rather
            // than correctness: this form is never compiled, but `f() + g()` and `g() + f()` are
            // different programs, and normalizing them together would report an agreement that is
            // not there. The whole value of a zero is that it means something.
            let reorderable = is_commutative(op) && is_effect_free(&left) && is_effect_free(&right);
            let swap = reorderable && shape_expr(&right).render() < shape_expr(&left).render();
            let (left, right) = if swap { (right, left) } else { (left, right) };
            Expr::Binary {
                op: *op,
                left: Box::new(left),
                right: Box::new(right),
            }
        }
        Expr::ListLit(elements) => Expr::ListLit(normalize_all(elements)),
        Expr::SetLit(elements) => Expr::SetLit(normalize_all(elements)),
        Expr::TupleLit(elements) => Expr::TupleLit(normalize_all(elements)),
        Expr::DictLit(pairs) => Expr::DictLit(
            pairs
                .iter()
                .map(|(key, value)| (normalize_expr(key), normalize_expr(value)))
                .collect(),
        ),
        Expr::TupleIndex { base, position } => Expr::TupleIndex {
            base: Box::new(normalize_expr(base)),
            position: *position,
        },
        Expr::Attribute { object, name } => Expr::Attribute {
            object: Box::new(normalize_expr(object)),
            name: name.clone(),
        },
        Expr::Construct { class, args } => Expr::Construct {
            class: class.clone(),
            args: normalize_all(args),
        },
        Expr::MethodCall {
            receiver,
            class,
            method,
            args,
        } => Expr::MethodCall {
            receiver: Box::new(normalize_expr(receiver)),
            class: class.clone(),
            method: method.clone(),
            args: normalize_all(args),
        },
        Expr::Contains { value, container } => Expr::Contains {
            value: Box::new(normalize_expr(value)),
            container: Box::new(normalize_expr(container)),
        },
        Expr::Not(value) => Expr::Not(Box::new(normalize_expr(value))),
        Expr::Subscript {
            base,
            index,
            origin,
            checked,
        } => Expr::Subscript {
            base: Box::new(normalize_expr(base)),
            index: Box::new(normalize_expr(index)),
            origin: *origin,
            checked: *checked,
        },
        Expr::Len { value, units } => Expr::Len {
            value: Box::new(normalize_expr(value)),
            units: *units,
        },
        Expr::Range { start, stop, step } => Expr::Range {
            start: Box::new(normalize_expr(start)),
            stop: Box::new(normalize_expr(stop)),
            step: Box::new(normalize_expr(step)),
        },
        Expr::Call { callee, args } => Expr::Call {
            callee: callee.clone(),
            args: normalize_all(args),
        },
    }
}

/// Normalize a list of expressions, whose order is always part of the program.
fn normalize_all(exprs: &[Expr]) -> Vec<Expr> {
    exprs.iter().map(normalize_expr).collect()
}

/// Whether an operator's operands may be exchanged without changing the result.
///
/// The comparisons that merely *reverse* under exchange — `<` becoming `>` — are excluded: turning
/// one into the other is rewriting the operator, not reordering its operands, and a normalizer that
/// did it would have to be trusted about a second thing.
fn is_commutative(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Add { .. } | BinOp::Mul { .. } | BinOp::Eq | BinOp::NotEq
    )
}

/// Whether evaluating this expression can do anything but produce a value.
fn is_effect_free(expr: &Expr) -> bool {
    let mut clean = true;
    expr.walk(&mut |node| {
        if matches!(
            node,
            Expr::Call { .. } | Expr::MethodCall { .. } | Expr::Construct { .. }
        ) {
            clean = false;
        }
    });
    clean
}

/// Sort each run of consecutive bindings whose order carries no meaning.
///
/// A run qualifies when every binding in it is effect-free and none of them reads a name another
/// one binds. Those are exactly the conditions under which the run's order is a fact about how
/// someone typed it rather than about what the program does.
fn sort_independent_binds(stmts: &mut [Stmt]) {
    let mut start = 0;
    while start < stmts.len() {
        let Some(end) = run_end(stmts, start) else {
            start += 1;
            continue;
        };
        if end - start > 1 && is_independent_run(&stmts[start..end]) {
            stmts[start..end].sort_by(|a, b| bind_name(a).cmp(bind_name(b)));
        }
        start = end.max(start + 1);
    }
}

/// The end of the run of effect-free bindings starting at `start`, if one starts there.
fn run_end(stmts: &[Stmt], start: usize) -> Option<usize> {
    if !is_sortable_bind(&stmts[start]) {
        return None;
    }
    let mut end = start;
    while end < stmts.len() && is_sortable_bind(&stmts[end]) {
        end += 1;
    }
    Some(end)
}

/// Whether a statement is a binding that could take part in a reordering.
fn is_sortable_bind(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Bind { value, .. } if is_effect_free(value))
}

/// The name a binding introduces. Only ever called on a statement already known to be one.
fn bind_name(stmt: &Stmt) -> &str {
    match stmt {
        Stmt::Bind { name, .. } => name,
        _ => "",
    }
}

/// Whether no binding in the run reads a name another one binds.
fn is_independent_run(run: &[Stmt]) -> bool {
    let bound: BTreeSet<String> = run.iter().map(|stmt| bind_name(stmt).to_string()).collect();
    run.iter().all(|stmt| match stmt {
        Stmt::Bind { value, .. } => names_read(value).is_disjoint(&bound),
        _ => true,
    })
}

/// Every name this expression reads.
fn names_read(expr: &Expr) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    expr.walk(&mut |node| {
        if let Expr::Name(name) = node {
            names.insert(name.clone());
        }
    });
    names
}

// ---------------------------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------------------------

/// The comparison shape of a function.
///
/// The function's own name is absent: it is how two members were paired in the first place, so
/// including it would compare a value already known to be equal.
fn shape_function(function: &Function) -> Shape {
    // The docstring is absent, and so is the span. Both for the reason `Function::fingerprint`
    // excludes them: prose is not part of what a function computes, and a span is an offset into a
    // source text the other side does not share. Counting spans would make every cross-language
    // pair diverge in every node and the score a constant that never moves.
    Shape::node(
        "function",
        vec![
            Shape::node(
                "parameters",
                function
                    .params
                    .iter()
                    .map(|param| {
                        Shape::node(
                            format!("parameter {}", param.name),
                            vec![shape_ty(&param.ty)],
                        )
                    })
                    .collect(),
            ),
            Shape::node("returns", vec![shape_ty(&function.ret)]),
            shape_block("body", &function.body),
        ],
    )
}

/// The comparison shape of a class.
fn shape_class(class: &Class) -> Shape {
    Shape::node(
        "class",
        vec![
            Shape::node(
                "attributes",
                class
                    .attributes
                    .iter()
                    .map(|attribute| {
                        Shape::node(
                            format!("attribute {}", attribute.name),
                            vec![shape_ty(&attribute.ty)],
                        )
                    })
                    .collect(),
            ),
            Shape::node("constructor", vec![shape_function(&class.init)]),
            Shape::node(
                "methods",
                class
                    .methods
                    .iter()
                    .map(|(name, method)| {
                        Shape::node(format!("method {name}"), vec![shape_function(method)])
                    })
                    .collect(),
            ),
        ],
    )
}

/// A type, rendered in the IR's own neutral spelling rather than any language's.
fn shape_ty(ty: &Ty) -> Shape {
    Shape::leaf(ty.to_string())
}

/// A block of statements under a named node.
fn shape_block(label: &str, stmts: &[Stmt]) -> Shape {
    Shape::node(label, stmts.iter().map(shape_stmt).collect())
}

/// The comparison shape of a statement.
fn shape_stmt(stmt: &Stmt) -> Shape {
    match stmt {
        Stmt::Return(value) => Shape::node("return", vec![shape_expr(value)]),
        Stmt::ReturnUnit => Shape::leaf("return-unit"),
        Stmt::Bind { name, ty, value } => Shape::node(
            format!("bind {name}"),
            vec![shape_ty(ty), shape_expr(value)],
        ),
        Stmt::Assign { name, ty, value } => Shape::node(
            format!("assign {name}"),
            vec![shape_ty(ty), shape_expr(value)],
        ),
        Stmt::Effect(value) => Shape::node("effect", vec![shape_expr(value)]),
        Stmt::SetAttr {
            object,
            name,
            ty,
            value,
        } => Shape::node(
            format!("set-attribute {name}"),
            vec![shape_expr(object), shape_ty(ty), shape_expr(value)],
        ),
        Stmt::SetItem {
            collection,
            index,
            value,
        } => Shape::node(
            "set-item",
            vec![shape_expr(collection), shape_expr(index), shape_expr(value)],
        ),
        Stmt::Append { sequence, value } => {
            Shape::node("append", vec![shape_expr(sequence), shape_expr(value)])
        }
        Stmt::If {
            test,
            then,
            otherwise,
        } => Shape::node(
            "if",
            vec![
                shape_expr(test),
                shape_block("then", then),
                shape_block("otherwise", otherwise),
            ],
        ),
        Stmt::While { test, body } => {
            Shape::node("while", vec![shape_expr(test), shape_block("body", body)])
        }
        Stmt::For {
            name,
            ty,
            iter,
            body,
        } => Shape::node(
            format!("for {name}"),
            vec![shape_ty(ty), shape_expr(iter), shape_block("body", body)],
        ),
        Stmt::Break => Shape::leaf("break"),
        Stmt::Continue => Shape::leaf("continue"),
    }
}

/// The comparison shape of an expression.
///
/// Every semantic mode is dropped here, and that is the single most important thing this function
/// does. Python and TypeScript *must* disagree about overflow, division rounding, remainder sign,
/// index origin, and how text is counted, because each preserves what its own source meant. A
/// differ that counted those would be measuring the languages rather than the frontends.
fn shape_expr(expr: &Expr) -> Shape {
    match expr {
        Expr::Literal(value) => Shape::leaf(format!("literal {}", render_literal(value))),
        Expr::Name(name) => Shape::leaf(format!("name {name}")),
        // `checked` dropped.
        Expr::Neg { value, .. } => Shape::node("negate", vec![shape_expr(value)]),
        Expr::ToFloat(value) => Shape::node("to-float", vec![shape_expr(value)]),
        Expr::Binary { op, left, right } => Shape::node(
            format!("binary {}", operator_label(op)),
            vec![shape_expr(left), shape_expr(right)],
        ),
        Expr::ListLit(elements) => Shape::node("list-literal", shape_all(elements)),
        Expr::SetLit(elements) => Shape::node("set-literal", shape_all(elements)),
        Expr::TupleLit(elements) => Shape::node("tuple-literal", shape_all(elements)),
        Expr::DictLit(pairs) => Shape::node(
            "mapping-literal",
            pairs
                .iter()
                .map(|(key, value)| Shape::node("entry", vec![shape_expr(key), shape_expr(value)]))
                .collect(),
        ),
        Expr::TupleIndex { base, position } => {
            Shape::node(format!("tuple-index {position}"), vec![shape_expr(base)])
        }
        Expr::Attribute { object, name } => {
            Shape::node(format!("attribute {name}"), vec![shape_expr(object)])
        }
        Expr::Construct { class, args } => {
            Shape::node(format!("construct {class}"), shape_all(args))
        }
        // The receiver's resolved class is dropped: it is `None` exactly when the receiver came
        // from another source, so counting it would score a difference in what lowering happened
        // to know as a difference in the program.
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            let mut children = vec![shape_expr(receiver)];
            children.extend(shape_all(args));
            Shape::node(format!("method {method}"), children)
        }
        Expr::Contains { value, container } => {
            Shape::node("contains", vec![shape_expr(value), shape_expr(container)])
        }
        Expr::Not(value) => Shape::node("not", vec![shape_expr(value)]),
        // `origin` and `checked` dropped.
        Expr::Subscript { base, index, .. } => {
            Shape::node("subscript", vec![shape_expr(base), shape_expr(index)])
        }
        // `units` dropped.
        Expr::Len { value, .. } => Shape::node("length", vec![shape_expr(value)]),
        Expr::Range { start, stop, step } => Shape::node(
            "range",
            vec![shape_expr(start), shape_expr(stop), shape_expr(step)],
        ),
        Expr::Call { callee, args } => Shape::node(format!("call {callee}"), shape_all(args)),
    }
}

/// Shapes for a list of expressions.
fn shape_all(exprs: &[Expr]) -> Vec<Shape> {
    exprs.iter().map(shape_expr).collect()
}

/// A literal's contribution, by value.
fn render_literal(value: &Literal) -> String {
    match value {
        Literal::Int(number) => format!("{number}"),
        Literal::Bool(flag) => format!("{flag}"),
        Literal::Str(text) => format!("{text:?}"),
        // Compared by the bits, which is how the IR stores it and how its own equality works.
        other => other
            .as_f64()
            .map_or_else(|| "?".to_string(), |number| format!("{number:?}")),
    }
}

/// An operator's contribution.
///
/// `Exact` and `Integer` divisions stay distinct while the rounding within `Integer` does not:
/// `a / b` and `a // b` are different operations, whereas which way an integer division rounds is
/// the source language's stance on one operation. Collapsing the first pair would say those two
/// programs are the same; keeping the second would count Python being Python.
fn operator_label(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add { .. } => "add",
        BinOp::Sub { .. } => "subtract",
        BinOp::Mul { .. } => "multiply",
        BinOp::Div {
            mode: DivMode::Exact,
            ..
        } => "divide-exact",
        BinOp::Div {
            mode: DivMode::Integer(_),
            ..
        } => "divide-integer",
        BinOp::Rem { .. } => "remainder",
        BinOp::Eq => "equal",
        BinOp::NotEq => "not-equal",
        BinOp::Lt => "less",
        BinOp::LtE => "less-or-equal",
        BinOp::Gt => "greater",
        BinOp::GtE => "greater-or-equal",
    }
}

// ---------------------------------------------------------------------------------------------
// Distance
// ---------------------------------------------------------------------------------------------

/// The edit distance between two shapes, appending a note for each difference found.
///
/// Order-preserving: children are aligned as sequences, so a statement inserted into a body costs
/// that statement rather than shifting everything below it out of alignment.
fn distance(left: &Shape, right: &Shape, path: &str, notes: &mut Vec<String>) -> u32 {
    let relabel = if left.label == right.label {
        0
    } else {
        notes.push(format!(
            "{path}: '{}' where the other has '{}'",
            left.label, right.label
        ));
        1
    };
    let here = if left.label == right.label {
        format!("{path}/{}", left.label)
    } else {
        path.to_string()
    };
    relabel + align(&left.children, &right.children, &here, notes)
}

/// Align two child sequences, charging a whole subtree for one that has no counterpart.
fn align(left: &[Shape], right: &[Shape], path: &str, notes: &mut Vec<String>) -> u32 {
    // Levenshtein over subtrees: substitution costs the recursive distance, and an insertion or
    // deletion costs the size of what is missing. Computed without notes first, so that the notes
    // describe only the alignment actually chosen rather than every one considered.
    let rows = left.len() + 1;
    let columns = right.len() + 1;
    let mut cost = vec![vec![0u32; columns]; rows];
    for (row, entry) in cost.iter_mut().enumerate().skip(1) {
        entry[0] = cost_of(&left[..row]);
    }
    for column in 1..columns {
        cost[0][column] = cost_of(&right[..column]);
    }
    for row in 1..rows {
        for column in 1..columns {
            let substitute = cost[row - 1][column - 1]
                + distance(&left[row - 1], &right[column - 1], path, &mut Vec::new());
            let delete = cost[row - 1][column] + left[row - 1].size();
            let insert = cost[row][column - 1] + right[column - 1].size();
            cost[row][column] = substitute.min(delete).min(insert);
        }
    }

    describe(left, right, &cost, path, notes);
    cost[left.len()][right.len()]
}

/// The total size of a run of subtrees.
fn cost_of(shapes: &[Shape]) -> u32 {
    shapes.iter().map(Shape::size).sum()
}

/// Walk the chosen alignment backwards, recording what it did.
fn describe(
    left: &[Shape],
    right: &[Shape],
    cost: &[Vec<u32>],
    path: &str,
    notes: &mut Vec<String>,
) {
    let mut row = left.len();
    let mut column = right.len();
    let mut found = Vec::new();
    while row > 0 || column > 0 {
        if row > 0 && column > 0 {
            let substitute = cost[row - 1][column - 1]
                + distance(&left[row - 1], &right[column - 1], path, &mut Vec::new());
            if cost[row][column] == substitute {
                let mut inner = Vec::new();
                distance(&left[row - 1], &right[column - 1], path, &mut inner);
                found.extend(inner);
                row -= 1;
                column -= 1;
                continue;
            }
        }
        if row > 0 && cost[row][column] == cost[row - 1][column] + left[row - 1].size() {
            found.push(format!(
                "{path}: '{}' has no counterpart",
                left[row - 1].label
            ));
            row -= 1;
            continue;
        }
        if column > 0 {
            found.push(format!(
                "{path}: '{}' appears only on the other side",
                right[column - 1].label
            ));
            column -= 1;
            continue;
        }
        break;
    }
    found.reverse();
    notes.extend(found);
}

#[cfg(test)]
mod tests {
    use super::*;
    use compylr_diagnostics::span::Span;
    use compylr_ir::{Checked, Param, Rounding};

    /// `a + b` with the given checking mode.
    fn add(checked: Checked) -> Expr {
        Expr::binary(BinOp::Add { checked }, Expr::name("a"), Expr::name("b"))
    }

    /// A function of two integers returning `body`.
    fn function_of(name: &str, body: Vec<Stmt>) -> Function {
        Function {
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
            body,
            doc: None,
            span: Span::default(),
        }
    }

    /// A unit holding one function.
    fn unit_of(function: Function) -> Unit {
        let mut unit = Unit::new();
        unit.add_function(function).expect("name is unique");
        unit
    }

    /// Normalization runs on a copy. If it could reach the unit a backend emits from, the project
    /// would be changing programs in order to measure them.
    #[test]
    fn normalizing_leaves_the_original_alone() {
        let unit = unit_of(function_of(
            "f",
            vec![
                Stmt::Bind {
                    name: "z".to_string(),
                    ty: Ty::Int,
                    value: Expr::int(1),
                },
                Stmt::Bind {
                    name: "y".to_string(),
                    ty: Ty::Int,
                    value: Expr::int(2),
                },
                Stmt::Return(Expr::name("z")),
            ],
        ));
        let before = unit.fingerprint();
        let normalized = normalize(&unit);

        assert_eq!(unit.fingerprint(), before, "the original was rewritten");
        assert_ne!(
            normalized.fingerprint(),
            before,
            "this fixture is supposed to be one normalization changes"
        );
    }

    /// Two orders of the same independent bindings are one program.
    #[test]
    fn independent_bindings_normalize_together() {
        let one = unit_of(function_of(
            "f",
            vec![
                Stmt::Bind {
                    name: "x".to_string(),
                    ty: Ty::Int,
                    value: Expr::int(1),
                },
                Stmt::Bind {
                    name: "y".to_string(),
                    ty: Ty::Int,
                    value: Expr::int(2),
                },
                Stmt::Return(Expr::name("x")),
            ],
        ));
        let other = unit_of(function_of(
            "f",
            vec![
                Stmt::Bind {
                    name: "y".to_string(),
                    ty: Ty::Int,
                    value: Expr::int(2),
                },
                Stmt::Bind {
                    name: "x".to_string(),
                    ty: Ty::Int,
                    value: Expr::int(1),
                },
                Stmt::Return(Expr::name("x")),
            ],
        ));

        assert!(
            divergence(&one, &other).score() > 0,
            "unnormalized they differ"
        );
        assert!(
            divergence(&normalize(&one), &normalize(&other)).is_zero(),
            "normalized they are one program"
        );
    }

    /// A binding that reads what the one before it bound is not independent of it.
    #[test]
    fn dependent_bindings_keep_their_order() {
        let dependent = vec![
            Stmt::Bind {
                name: "z".to_string(),
                ty: Ty::Int,
                value: Expr::int(1),
            },
            Stmt::Bind {
                name: "y".to_string(),
                ty: Ty::Int,
                value: Expr::name("z"),
            },
        ];
        let normalized = normalize_block(&dependent);

        assert_eq!(bind_name(&normalized[0]), "z", "the run was reordered");
    }

    /// Commutative operands sort; the operator itself is untouched.
    #[test]
    fn commutative_operands_normalize_together() {
        let one = unit_of(function_of(
            "f",
            vec![Stmt::Return(Expr::binary(
                BinOp::Add {
                    checked: Checked::Reported,
                },
                Expr::name("b"),
                Expr::name("a"),
            ))],
        ));
        let other = unit_of(function_of("f", vec![Stmt::Return(add(Checked::Reported))]));

        assert!(
            divergence(&normalize(&one), &normalize(&other)).is_zero(),
            "b + a and a + b are one program"
        );
    }

    /// Subtraction is not commutative, so its operands stay where they are.
    #[test]
    fn non_commutative_operands_keep_their_order() {
        let one = unit_of(function_of(
            "f",
            vec![Stmt::Return(Expr::binary(
                BinOp::Sub {
                    checked: Checked::Reported,
                },
                Expr::name("b"),
                Expr::name("a"),
            ))],
        ));
        let other = unit_of(function_of(
            "f",
            vec![Stmt::Return(Expr::binary(
                BinOp::Sub {
                    checked: Checked::Reported,
                },
                Expr::name("a"),
                Expr::name("b"),
            ))],
        ));

        assert!(
            !divergence(&normalize(&one), &normalize(&other)).is_zero(),
            "b - a and a - b are different programs"
        );
    }

    /// `f() + g()` and `g() + f()` call in different orders, so they are different programs and
    /// normalization must leave them that way.
    #[test]
    fn calling_operands_are_not_reordered() {
        let call = |name: &str| Expr::Call {
            callee: name.to_string(),
            args: Vec::new(),
        };
        let one = unit_of(function_of(
            "f",
            vec![Stmt::Return(Expr::binary(
                BinOp::Add {
                    checked: Checked::Reported,
                },
                call("g"),
                call("h"),
            ))],
        ));
        let other = unit_of(function_of(
            "f",
            vec![Stmt::Return(Expr::binary(
                BinOp::Add {
                    checked: Checked::Reported,
                },
                call("h"),
                call("g"),
            ))],
        ));

        assert!(
            !divergence(&normalize(&one), &normalize(&other)).is_zero(),
            "reordering calls would report an agreement that is not there"
        );
    }

    /// A binding whose value calls cannot move either, for the same reason.
    #[test]
    fn calling_bindings_are_not_reordered() {
        let calling = vec![
            Stmt::Bind {
                name: "z".to_string(),
                ty: Ty::Int,
                value: Expr::Call {
                    callee: "g".to_string(),
                    args: Vec::new(),
                },
            },
            Stmt::Bind {
                name: "y".to_string(),
                ty: Ty::Int,
                value: Expr::Call {
                    callee: "h".to_string(),
                    args: Vec::new(),
                },
            },
        ];
        let normalized = normalize_block(&calling);

        assert_eq!(bind_name(&normalized[0]), "z", "the calls were reordered");
    }

    /// The modes are the point of the IR. Two frontends preserving their own languages' stances
    /// are agreeing about the program, not disagreeing.
    #[test]
    fn checking_modes_do_not_diverge() {
        let reported = unit_of(function_of("f", vec![Stmt::Return(add(Checked::Reported))]));
        let unchecked = unit_of(function_of(
            "f",
            vec![Stmt::Return(add(Checked::Unchecked))],
        ));

        assert!(divergence(&reported, &unchecked).is_zero());
    }

    /// Likewise for how an integer division rounds.
    #[test]
    fn rounding_modes_do_not_diverge() {
        let divide = |rounding| {
            unit_of(function_of(
                "f",
                vec![Stmt::Return(Expr::binary(
                    BinOp::Div {
                        mode: DivMode::Integer(rounding),
                        checked: Checked::Reported,
                    },
                    Expr::name("a"),
                    Expr::name("b"),
                ))],
            ))
        };

        assert!(
            divergence(
                &divide(Rounding::TowardNegInf),
                &divide(Rounding::TowardZero)
            )
            .is_zero()
        );
    }

    /// But which division it is remains a difference: `a / b` and `a // b` compute different
    /// things, and a differ that called them equal would be measuring nothing.
    #[test]
    fn exact_and_integer_division_diverge() {
        let divide = |mode| {
            unit_of(function_of(
                "f",
                vec![Stmt::Return(Expr::binary(
                    BinOp::Div {
                        mode,
                        checked: Checked::Reported,
                    },
                    Expr::name("a"),
                    Expr::name("b"),
                ))],
            ))
        };

        assert!(
            !divergence(
                &divide(DivMode::Exact),
                &divide(DivMode::Integer(Rounding::TowardNegInf))
            )
            .is_zero()
        );
    }

    /// A span is an offset into a source text the other side does not share. Counting spans would
    /// make every cross-language pair diverge in every node.
    #[test]
    fn spans_do_not_diverge() {
        let mut here = function_of("f", vec![Stmt::Return(add(Checked::Reported))]);
        here.span = Span::new(10, 20);
        let mut there = function_of("f", vec![Stmt::Return(add(Checked::Reported))]);
        there.span = Span::new(400, 410);

        assert!(divergence(&unit_of(here), &unit_of(there)).is_zero());
    }

    /// Documentation is prose about a function, not part of what it computes.
    #[test]
    fn documentation_does_not_diverge() {
        let mut documented = function_of("f", vec![Stmt::Return(add(Checked::Reported))]);
        documented.doc = Some("Adds.".to_string());
        let bare = function_of("f", vec![Stmt::Return(add(Checked::Reported))]);

        assert!(divergence(&unit_of(documented), &unit_of(bare)).is_zero());
    }

    /// Structure is what is left, and it does diverge.
    #[test]
    fn structure_diverges() {
        let adding = unit_of(function_of("f", vec![Stmt::Return(add(Checked::Reported))]));
        let looping = unit_of(function_of(
            "f",
            vec![
                Stmt::While {
                    test: Expr::bool(false),
                    body: vec![Stmt::Break],
                },
                Stmt::Return(Expr::int(0)),
            ],
        ));

        assert!(divergence(&adding, &looping).score() > 0);
    }

    /// A score nobody can act on will not be driven down, so it comes with its location.
    #[test]
    fn a_score_names_what_differs() {
        let adding = unit_of(function_of("f", vec![Stmt::Return(add(Checked::Reported))]));
        let subtracting = unit_of(function_of(
            "f",
            vec![Stmt::Return(Expr::binary(
                BinOp::Sub {
                    checked: Checked::Reported,
                },
                Expr::name("a"),
                Expr::name("b"),
            ))],
        ));

        let found = divergence(&adding, &subtracting);
        let member = found.divergent().next().expect("f diverges");

        assert_eq!(member.name(), "f");
        assert!(
            member
                .notes()
                .iter()
                .any(|note| note.contains("binary add") && note.contains("binary subtract")),
            "notes did not name the operators: {:?}",
            member.notes()
        );
    }

    /// A member only one side defines is not free. A corpus that scored well by staying small
    /// would be reporting on its own size.
    #[test]
    fn a_member_only_one_side_defines_counts() {
        let both = unit_of(function_of("f", vec![Stmt::Return(add(Checked::Reported))]));
        let mut one_more = both.clone();
        one_more
            .add_function(function_of("g", vec![Stmt::Return(add(Checked::Reported))]))
            .expect("name is unique");

        let found = divergence(&both, &one_more);
        assert!(found.score() > 0);
        assert_eq!(found.divergent().next().expect("g diverges").name(), "g");
    }

    /// The same unit against itself is the identity the whole measurement rests on.
    #[test]
    fn a_unit_does_not_diverge_from_itself() {
        let unit = unit_of(function_of(
            "f",
            vec![
                Stmt::Bind {
                    name: "x".to_string(),
                    ty: Ty::Int,
                    value: add(Checked::Reported),
                },
                Stmt::Return(Expr::name("x")),
            ],
        ));

        assert!(divergence(&unit, &unit).is_zero());
    }
}
