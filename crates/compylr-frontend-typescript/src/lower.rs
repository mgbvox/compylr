//! Lowering a parsed TypeScript module into compylr IR.

use std::collections::{BTreeMap, HashMap, HashSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, BindingPattern, Class as OxcClass, ClassElement, Declaration,
    Expression as OxcExpr, ForStatementLeft, FormalParameters, Function as OxcFunction,
    FunctionBody, Program, Statement as OxcStmt, TSType,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span as OxcSpan};

use compylr_diagnostics::span::Span;
use compylr_ir::{
    Attribute, Behavior, BinOp, Class, Expr, Function, Literal, Param, Stmt, Ty, Unit,
    returns_on_all_paths,
};

use crate::error::{Category, unsupported};
use crate::spelling::TypeScriptSpelling;

/// Convert oxc_span::Span to compylr_diagnostics::span::Span.
pub fn to_span(span: OxcSpan) -> Span {
    Span::new(span.start, span.end)
}

/// Signatures of top-level functions in a unit.
pub type Signatures = HashMap<String, (Vec<Param>, Ty)>;

/// Signatures of classes in a unit: class name -> (attributes, constructor params, methods).
pub type ClassSignatures = HashMap<
    String,
    (
        HashMap<String, Ty>,
        Vec<Param>,
        HashMap<String, (Vec<Param>, Ty)>,
    ),
>;

/// Parse TypeScript source into an AST.
pub fn parse_ts_source<'a>(
    allocator: &'a Allocator,
    source: &'a str,
) -> Result<Program<'a>, compylr_core::frontend::LoweringError> {
    let source_type = SourceType::ts();
    let ret = Parser::new(allocator, source, source_type).parse();
    if let Some(first_err) = ret.diagnostics.first() {
        let span = to_span(
            first_err
                .labels
                .as_ref()
                .first()
                .map_or(OxcSpan::default(), |l| l.span()),
        );
        let lc = span.line_column(source);
        return Err(compylr_core::frontend::LoweringError::Syntax {
            message: first_err.to_string(),
            line: lc.line,
            column: lc.column,
        });
    }
    Ok(ret.program)
}

/// Collect class names declared across a TypeScript AST.
pub fn collect_class_names<'a>(program: &Program<'a>) -> HashSet<String> {
    let mut names = HashSet::new();
    for stmt in &program.body {
        match stmt {
            OxcStmt::ClassDeclaration(decl) => {
                if let Some(ident) = &decl.id {
                    names.insert(ident.name.to_string());
                }
            }
            OxcStmt::ExportDeclaration(decl) => {
                if let Declaration::ClassDeclaration(c) = &decl.declaration {
                    if let Some(ident) = &c.id {
                        names.insert(ident.name.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    names
}

/// Collect function signatures across a TypeScript AST.
pub fn collect_signatures<'a>(
    program: &Program<'a>,
    class_names: &HashSet<String>,
    source: &str,
) -> Result<Signatures, compylr_core::frontend::LoweringError> {
    let mut signatures = HashMap::new();
    for stmt in &program.body {
        let fn_decl = match stmt {
            OxcStmt::FunctionDeclaration(f) => Some(f.as_ref()),
            OxcStmt::ExportDeclaration(decl) => match &decl.declaration {
                Declaration::FunctionDeclaration(f) => Some(f.as_ref()),
                _ => None,
            },
            _ => None,
        };
        if let Some(f) = fn_decl {
            if let Some(ident) = &f.id {
                let name = ident.name.to_string();
                let params = lower_params(&f.params, class_names, source)?;
                let ret_ty = if let Some(type_annot) = &f.return_type {
                    lower_type(&type_annot.type_annotation, class_names, source)?
                } else {
                    Ty::Unit
                };
                signatures.insert(name, (params, ret_ty));
            }
        }
    }
    Ok(signatures)
}

/// Collect class signatures across a TypeScript AST.
pub fn collect_class_signatures<'a>(
    program: &Program<'a>,
    class_names: &HashSet<String>,
    source: &str,
) -> Result<ClassSignatures, compylr_core::frontend::LoweringError> {
    let mut class_sigs = HashMap::new();
    for stmt in &program.body {
        let class_decl = match stmt {
            OxcStmt::ClassDeclaration(c) => Some(c.as_ref()),
            OxcStmt::ExportDeclaration(decl) => match &decl.declaration {
                Declaration::ClassDeclaration(c) => Some(c.as_ref()),
                _ => None,
            },
            _ => None,
        };
        if let Some(c) = class_decl {
            if let Some(ident) = &c.id {
                let class_name = ident.name.to_string();
                let mut attributes = HashMap::new();
                let mut ctor_params = Vec::new();
                let mut methods = HashMap::new();
                for element in &c.body.body {
                    match element {
                        ClassElement::PropertyDefinition(prop) => {
                            if let Some(name) = prop.key.name() {
                                if let Some(annot) = &prop.type_annotation {
                                    let ty =
                                        lower_type(&annot.type_annotation, class_names, source)?;
                                    attributes.insert(name.to_string(), ty);
                                }
                            }
                        }
                        ClassElement::MethodDefinition(m) => {
                            if m.kind.is_constructor() {
                                ctor_params = lower_params(&m.value.params, class_names, source)?;
                            } else if let Some(name) = m.key.name() {
                                let m_params = lower_params(&m.value.params, class_names, source)?;
                                let m_ret = if let Some(type_annot) = &m.value.return_type {
                                    lower_type(&type_annot.type_annotation, class_names, source)?
                                } else {
                                    Ty::Unit
                                };
                                methods.insert(name.to_string(), (m_params, m_ret));
                            }
                        }
                        _ => {}
                    }
                }
                class_sigs.insert(class_name, (attributes, ctor_params, methods));
            }
        }
    }
    Ok(class_sigs)
}

/// Parse and lower a TypeScript type annotation.
pub fn lower_type(
    ts_type: &TSType,
    class_names: &HashSet<String>,
    source: &str,
) -> Result<Ty, compylr_core::frontend::LoweringError> {
    match ts_type {
        TSType::TSNumberKeyword(_) => Ok(Ty::Int),
        TSType::TSStringKeyword(_) => Ok(Ty::Str),
        TSType::TSBooleanKeyword(_) => Ok(Ty::Bool),
        TSType::TSVoidKeyword(_) | TSType::TSNullKeyword(_) | TSType::TSUndefinedKeyword(_) => {
            Ok(Ty::Unit)
        }
        TSType::TSTupleType(tup) => {
            let mut elems = Vec::new();
            for el in &tup.element_types {
                elems.push(lower_type(el.to_ts_type(), class_names, source)?);
            }
            Ok(Ty::Tuple(elems))
        }
        TSType::TSArrayType(arr) => {
            let elem_ty = lower_type(&arr.element_type, class_names, source)?;
            Ok(Ty::List(Box::new(elem_ty)))
        }
        TSType::TSTypeReference(t_ref) => {
            let type_name = match &t_ref.type_name {
                oxc_ast::ast::TSTypeName::IdentifierReference(id) => id.name.as_str(),
                _ => {
                    let lc = to_span(t_ref.span).line_column(source);
                    return Err(unsupported(
                        Category::UnsupportedType,
                        "qualified type names are outside the supported subset",
                        lc.line,
                        lc.column,
                    ));
                }
            };
            match type_name {
                "int" | "number" => Ok(Ty::Int),
                "float" => Ok(Ty::Float),
                "string" => Ok(Ty::Str),
                "boolean" => Ok(Ty::Bool),
                "void" => Ok(Ty::Unit),
                "Array" => {
                    if let Some(type_params) = &t_ref.type_arguments {
                        if let Some(first) = type_params.params.first() {
                            let inner = lower_type(first, class_names, source)?;
                            return Ok(Ty::List(Box::new(inner)));
                        }
                    }
                    let lc = to_span(t_ref.span).line_column(source);
                    Err(unsupported(
                        Category::MissingAnnotation,
                        "Array must specify an element type parameter e.g. Array<number>",
                        lc.line,
                        lc.column,
                    ))
                }
                "Map" => {
                    if let Some(type_params) = &t_ref.type_arguments {
                        if type_params.params.len() == 2 {
                            let k = lower_type(&type_params.params[0], class_names, source)?;
                            let v = lower_type(&type_params.params[1], class_names, source)?;
                            return Ok(Ty::Dict(Box::new(k), Box::new(v)));
                        }
                    }
                    let lc = to_span(t_ref.span).line_column(source);
                    Err(unsupported(
                        Category::MissingAnnotation,
                        "Map must specify key and value type parameters e.g. Map<string, number>",
                        lc.line,
                        lc.column,
                    ))
                }
                "Set" => {
                    if let Some(type_params) = &t_ref.type_arguments {
                        if let Some(first) = type_params.params.first() {
                            let inner = lower_type(first, class_names, source)?;
                            return Ok(Ty::Set(Box::new(inner)));
                        }
                    }
                    let lc = to_span(t_ref.span).line_column(source);
                    Err(unsupported(
                        Category::MissingAnnotation,
                        "Set must specify an element type parameter e.g. Set<number>",
                        lc.line,
                        lc.column,
                    ))
                }
                custom if class_names.contains(custom) => Ok(Ty::Instance(custom.to_string())),
                unknown => {
                    let lc = to_span(t_ref.span).line_column(source);
                    Err(unsupported(
                        Category::UnsupportedType,
                        format!("unrecognized type '{unknown}'"),
                        lc.line,
                        lc.column,
                    ))
                }
            }
        }
        other => {
            let lc = to_span(other.span()).line_column(source);
            Err(unsupported(
                Category::UnsupportedType,
                "unsupported type annotation in TypeScript subset",
                lc.line,
                lc.column,
            ))
        }
    }
}

/// Lower formal parameter list to IR Param list.
fn lower_params<'a>(
    formal_params: &FormalParameters<'a>,
    class_names: &HashSet<String>,
    source: &str,
) -> Result<Vec<Param>, compylr_core::frontend::LoweringError> {
    let mut params = Vec::new();
    for p in &formal_params.items {
        let (name, span) = match &p.pattern {
            BindingPattern::BindingIdentifier(id) => (id.name.to_string(), id.span),
            _ => {
                let lc = to_span(p.span).line_column(source);
                return Err(unsupported(
                    Category::UnsupportedStatement,
                    "destructuring in parameters is not supported",
                    lc.line,
                    lc.column,
                ));
            }
        };
        let ty = if let Some(type_annot) = &p.type_annotation {
            lower_type(&type_annot.type_annotation, class_names, source)?
        } else {
            let lc = to_span(span).line_column(source);
            return Err(unsupported(
                Category::MissingAnnotation,
                format!("parameter '{name}' must have an explicit type annotation"),
                lc.line,
                lc.column,
            ));
        };
        params.push(Param { name, ty });
    }
    Ok(params)
}

/// Context for lowering statements and expressions within a function/method.
#[allow(dead_code)]
struct LoweringContext<'a> {
    signatures: &'a Signatures,
    class_signatures: &'a ClassSignatures,
    class_names: &'a HashSet<String>,
    source: &'a str,
    behavior: Behavior,
    /// Stack of lexical variable scopes: name -> (Ty, is_param)
    scopes: Vec<HashMap<String, (Ty, bool)>>,
    /// Names of parameters for checking parameter mutation restrictions
    params: HashSet<String>,
    /// Inside constructor?
    in_constructor: bool,
    /// Inside class method? (class name if so)
    in_class: Option<String>,
    /// Expected return type of enclosing function/method
    expected_ret: Option<Ty>,
}

impl<'a> LoweringContext<'a> {
    fn new(
        signatures: &'a Signatures,
        class_signatures: &'a ClassSignatures,
        class_names: &'a HashSet<String>,
        source: &'a str,
        behavior: Behavior,
        params: &[Param],
        in_constructor: bool,
        in_class: Option<String>,
    ) -> Self {
        let mut initial_scope = HashMap::new();
        let mut param_names = HashSet::new();
        for p in params {
            initial_scope.insert(p.name.clone(), (p.ty.clone(), true));
            param_names.insert(p.name.clone());
        }
        Self {
            signatures,
            class_signatures,
            class_names,
            source,
            behavior,
            scopes: vec![initial_scope],
            params: param_names,
            in_constructor,
            in_class,
            expected_ret: None,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert_binding(&mut self, name: String, ty: Ty) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, (ty, false));
        }
    }

    fn lookup_var(&self, name: &str) -> Option<&(Ty, bool)> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(binding);
            }
        }
        None
    }

    fn err(
        &self,
        category: Category,
        msg: impl Into<String>,
        span: OxcSpan,
    ) -> compylr_core::frontend::LoweringError {
        let lc = to_span(span).line_column(self.source);
        unsupported(category, msg, lc.line, lc.column)
    }
}

/// Lower a top-level function into an IR Function.
pub fn lower_function<'a>(
    f: &OxcFunction<'a>,
    signatures: &Signatures,
    class_signatures: &ClassSignatures,
    class_names: &HashSet<String>,
    behavior: Behavior,
    source: &str,
) -> Result<Function, compylr_core::frontend::LoweringError> {
    let name = f.id.as_ref().map_or("anonymous", |id| id.name.as_str());
    let params = lower_params(&f.params, class_names, source)?;
    let ret_ty = if let Some(type_annot) = &f.return_type {
        lower_type(&type_annot.type_annotation, class_names, source)?
    } else {
        Ty::Unit
    };

    let mut ctx = LoweringContext::new(
        signatures,
        class_signatures,
        class_names,
        source,
        behavior,
        &params,
        false,
        None,
    );
    ctx.expected_ret = Some(ret_ty.clone());

    let body_stmts = if let Some(body) = &f.body {
        lower_block(body, &mut ctx)?
    } else {
        Vec::new()
    };

    if ret_ty != Ty::Unit && !returns_on_all_paths(&body_stmts) {
        let lc = to_span(f.span).line_column(source);
        return Err(unsupported(
            Category::MissingReturn,
            format!(
                "function '{name}' with return type '{}' does not return a value on all paths",
                ret_ty.typescript_name()
            ),
            lc.line,
            lc.column,
        ));
    }

    let func = Function {
        name: name.to_string(),
        params,
        ret: ret_ty,
        body: body_stmts,
        doc: None,
        span: to_span(f.span),
    };
    Ok(func)
}

/// Lower a class into an IR Class.
pub fn lower_class<'a>(
    c: &OxcClass<'a>,
    signatures: &Signatures,
    class_signatures: &ClassSignatures,
    class_names: &HashSet<String>,
    behavior: Behavior,
    source: &str,
) -> Result<Class, compylr_core::frontend::LoweringError> {
    let name = c.id.as_ref().map_or("Anonymous", |id| id.name.as_str());

    let mut constructor_body = Vec::new();
    let mut ctor_params = Vec::new();
    let mut attributes = BTreeMap::new();
    let mut methods = BTreeMap::new();

    // First pass: extract constructor and attributes
    for el in &c.body.body {
        match el {
            ClassElement::MethodDefinition(m) => {
                if m.kind.is_constructor() {
                    ctor_params = lower_params(&m.value.params, class_names, source)?;
                    let mut ctx = LoweringContext::new(
                        signatures,
                        class_signatures,
                        class_names,
                        source,
                        behavior,
                        &ctor_params,
                        true,
                        Some(name.to_string()),
                    );
                    if let Some(body) = &m.value.body {
                        constructor_body = lower_block(body, &mut ctx)?;
                    }
                }
            }
            ClassElement::PropertyDefinition(p) => {
                if let Some(prop_name) = p.key.name() {
                    if let Some(annot) = &p.type_annotation {
                        let ty = lower_type(&annot.type_annotation, class_names, source)?;
                        attributes.insert(
                            prop_name.to_string(),
                            Attribute {
                                name: prop_name.to_string(),
                                ty,
                            },
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Second pass: lower methods
    for el in &c.body.body {
        if let ClassElement::MethodDefinition(m) = el {
            if !m.kind.is_constructor() {
                if let Some(m_name) = m.key.name() {
                    let m_params = lower_params(&m.value.params, class_names, source)?;
                    let m_ret = if let Some(annot) = &m.value.return_type {
                        lower_type(&annot.type_annotation, class_names, source)?
                    } else {
                        Ty::Unit
                    };
                    let mut ctx = LoweringContext::new(
                        signatures,
                        class_signatures,
                        class_names,
                        source,
                        behavior,
                        &m_params,
                        false,
                        Some(name.to_string()),
                    );
                    ctx.expected_ret = Some(m_ret.clone());
                    let m_body = if let Some(body) = &m.value.body {
                        lower_block(body, &mut ctx)?
                    } else {
                        Vec::new()
                    };

                    if m_ret != Ty::Unit && !returns_on_all_paths(&m_body) {
                        let lc = to_span(m.span).line_column(source);
                        return Err(unsupported(
                            Category::MissingReturn,
                            format!(
                                "method '{m_name}' with return type '{}' does not return a value on all paths",
                                m_ret.typescript_name()
                            ),
                            lc.line,
                            lc.column,
                        ));
                    }

                    methods.insert(
                        m_name.to_string(),
                        Function {
                            name: m_name.to_string(),
                            params: m_params,
                            ret: m_ret,
                            body: m_body,
                            doc: None,
                            span: to_span(m.span),
                        },
                    );
                }
            }
        }
    }

    // Infer attributes from constructor assignments if not explicitly annotated on class
    for stmt in &constructor_body {
        if let Stmt::SetAttr {
            name: attr_name,
            ty,
            ..
        } = stmt
        {
            if !attributes.contains_key(attr_name) && *ty != Ty::Unit {
                attributes.insert(
                    attr_name.clone(),
                    Attribute {
                        name: attr_name.clone(),
                        ty: ty.clone(),
                    },
                );
            }
        }
    }

    let init = Function {
        name: "__init__".to_string(),
        params: ctor_params,
        ret: Ty::Unit,
        body: constructor_body,
        doc: None,
        span: to_span(c.span),
    };

    let cls = Class {
        name: name.to_string(),
        attributes: attributes.into_values().collect(),
        init,
        methods,
        doc: None,
        span: to_span(c.span),
    };
    Ok(cls)
}

fn lower_block<'a>(
    block: &FunctionBody<'a>,
    ctx: &mut LoweringContext<'a>,
) -> Result<Vec<Stmt>, compylr_core::frontend::LoweringError> {
    let mut stmts = Vec::new();
    ctx.push_scope();
    for stmt in &block.statements {
        lower_statement(stmt, &mut stmts, ctx)?;
    }
    ctx.pop_scope();
    Ok(stmts)
}

fn lower_statement<'a>(
    stmt: &OxcStmt<'a>,
    out: &mut Vec<Stmt>,
    ctx: &mut LoweringContext<'a>,
) -> Result<(), compylr_core::frontend::LoweringError> {
    match stmt {
        OxcStmt::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument {
                if let OxcExpr::ArrayExpression(arr) = arg {
                    if let Some(expected) = &ctx.expected_ret {
                        if matches!(expected, Ty::Tuple(_)) {
                            let mut elements = Vec::new();
                            for el in &arr.elements {
                                if let Some(e) = el.as_expression() {
                                    let (lowered, _) = lower_expr(e, ctx)?;
                                    elements.push(lowered);
                                }
                            }
                            out.push(Stmt::Return(Expr::TupleLit(elements)));
                            return Ok(());
                        }
                    }
                }
                let (expr, _) = lower_expr(arg, ctx)?;
                out.push(Stmt::Return(expr));
            } else {
                out.push(Stmt::ReturnUnit);
            }
        }
        OxcStmt::VariableDeclaration(var_decl) => {
            for decl in &var_decl.declarations {
                let (name, span) = match &decl.id {
                    BindingPattern::BindingIdentifier(id) => (id.name.to_string(), id.span),
                    _ => {
                        return Err(ctx.err(
                            Category::UnsupportedStatement,
                            "destructuring variable declarations are not supported",
                            decl.span,
                        ));
                    }
                };

                let init = if let Some(init_expr) = &decl.init {
                    init_expr
                } else {
                    return Err(ctx.err(
                        Category::UnsupportedStatement,
                        format!("variable '{name}' must have an initializer"),
                        span,
                    ));
                };

                let (lowered_init, inferred_ty) =
                    if let (Some(type_annot), OxcExpr::ArrayExpression(arr)) =
                        (&decl.type_annotation, init)
                    {
                        let annot_ty =
                            lower_type(&type_annot.type_annotation, ctx.class_names, ctx.source)?;
                        if matches!(annot_ty, Ty::Tuple(_)) {
                            let mut elements = Vec::new();
                            for el in &arr.elements {
                                if let Some(e) = el.as_expression() {
                                    let (lowered, _) = lower_expr(e, ctx)?;
                                    elements.push(lowered);
                                }
                            }
                            (Expr::TupleLit(elements), annot_ty)
                        } else {
                            lower_expr(init, ctx)?
                        }
                    } else {
                        lower_expr(init, ctx)?
                    };

                let ty = if let Some(type_annot) = &decl.type_annotation {
                    lower_type(&type_annot.type_annotation, ctx.class_names, ctx.source)?
                } else if inferred_ty != Ty::Unit {
                    inferred_ty
                } else {
                    return Err(ctx.err(
                        Category::MissingAnnotation,
                        format!(
                            "variable '{name}' must have an explicit type annotation or typed initializer"
                        ),
                        span,
                    ));
                };

                ctx.insert_binding(name.clone(), ty.clone());
                out.push(Stmt::Bind {
                    name,
                    ty,
                    value: lowered_init,
                });
            }
        }
        OxcStmt::ExpressionStatement(expr_stmt) => {
            lower_expr_statement(&expr_stmt.expression, out, ctx)?;
        }
        OxcStmt::IfStatement(if_stmt) => {
            let (test_expr, test_ty) = lower_expr(&if_stmt.test, ctx)?;
            if test_ty != Ty::Bool {
                return Err(ctx.err(
                    Category::TypeMismatch,
                    "condition in 'if' statement must evaluate to a boolean",
                    if_stmt.test.span(),
                ));
            }
            let mut then_stmts = Vec::new();
            ctx.push_scope();
            lower_nested_statement(&if_stmt.consequent, &mut then_stmts, ctx)?;
            ctx.pop_scope();

            let mut else_stmts = Vec::new();
            if let Some(alt) = &if_stmt.alternate {
                ctx.push_scope();
                lower_nested_statement(alt, &mut else_stmts, ctx)?;
                ctx.pop_scope();
            }

            out.push(Stmt::If {
                test: test_expr,
                then: then_stmts,
                otherwise: else_stmts,
            });
        }
        OxcStmt::WhileStatement(while_stmt) => {
            let (test_expr, test_ty) = lower_expr(&while_stmt.test, ctx)?;
            if test_ty != Ty::Bool {
                return Err(ctx.err(
                    Category::TypeMismatch,
                    "condition in 'while' statement must evaluate to a boolean",
                    while_stmt.test.span(),
                ));
            }
            let mut body_stmts = Vec::new();
            ctx.push_scope();
            lower_nested_statement(&while_stmt.body, &mut body_stmts, ctx)?;
            ctx.pop_scope();

            out.push(Stmt::While {
                test: test_expr,
                body: body_stmts,
            });
        }
        OxcStmt::ForOfStatement(for_of) => {
            let target_name = match &for_of.left {
                ForStatementLeft::VariableDeclaration(v) => {
                    if let Some(first) = v.declarations.first() {
                        if let BindingPattern::BindingIdentifier(id) = &first.id {
                            id.name.to_string()
                        } else {
                            return Err(ctx.err(
                                Category::UnsupportedStatement,
                                "unsupported binding in for..of loop",
                                for_of.span,
                            ));
                        }
                    } else {
                        return Err(ctx.err(
                            Category::UnsupportedStatement,
                            "missing variable in for..of loop",
                            for_of.span,
                        ));
                    }
                }
                _ => {
                    return Err(ctx.err(
                        Category::UnsupportedStatement,
                        "for..of loop must declare variable with 'const' or 'let'",
                        for_of.span,
                    ));
                }
            };

            let (iter_expr, iter_ty) = lower_expr(&for_of.right, ctx)?;
            let elem_ty = match iter_ty {
                Ty::List(el) => *el,
                Ty::Set(el) => *el,
                Ty::Dict(k, _) => *k,
                Ty::Str => Ty::Str,
                _ => Ty::Int,
            };

            let mut body_stmts = Vec::new();
            ctx.push_scope();
            ctx.insert_binding(target_name.clone(), elem_ty.clone());
            lower_nested_statement(&for_of.body, &mut body_stmts, ctx)?;
            ctx.pop_scope();

            out.push(Stmt::For {
                name: target_name,
                ty: elem_ty,
                iter: iter_expr,
                body: body_stmts,
            });
        }
        OxcStmt::BreakStatement(_) => {
            out.push(Stmt::Break);
        }
        OxcStmt::ContinueStatement(_) => {
            out.push(Stmt::Continue);
        }
        OxcStmt::BlockStatement(b) => {
            ctx.push_scope();
            for s in &b.body {
                lower_statement(s, out, ctx)?;
            }
            ctx.pop_scope();
        }
        OxcStmt::EmptyStatement(_) => {}
        other => {
            return Err(ctx.err(
                Category::UnsupportedStatement,
                "unsupported statement type in TypeScript subset",
                other.span(),
            ));
        }
    }
    Ok(())
}

fn lower_nested_statement<'a>(
    stmt: &OxcStmt<'a>,
    out: &mut Vec<Stmt>,
    ctx: &mut LoweringContext<'a>,
) -> Result<(), compylr_core::frontend::LoweringError> {
    match stmt {
        OxcStmt::BlockStatement(b) => {
            for s in &b.body {
                lower_statement(s, out, ctx)?;
            }
        }
        single => {
            lower_statement(single, out, ctx)?;
        }
    }
    Ok(())
}

fn lower_expr_statement<'a>(
    expr: &OxcExpr<'a>,
    out: &mut Vec<Stmt>,
    ctx: &mut LoweringContext<'a>,
) -> Result<(), compylr_core::frontend::LoweringError> {
    match expr {
        OxcExpr::AssignmentExpression(assign) => {
            let (right_expr, right_ty) = lower_expr(&assign.right, ctx)?;
            match &assign.left {
                oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(id) => {
                    let name = id.name.as_str();
                    if ctx.params.contains(name) {
                        return Err(ctx.err(
                            Category::ParameterMutation,
                            format!(
                                "parameter '{name}' cannot be reassigned; parameters cross boundary by value"
                            ),
                            assign.span,
                        ));
                    }
                    let ty = if let Some((var_ty, _)) = ctx.lookup_var(name) {
                        var_ty.clone()
                    } else {
                        return Err(ctx.err(
                            Category::TypeMismatch,
                            format!("cannot assign to undefined variable '{name}'"),
                            assign.span,
                        ));
                    };
                    out.push(Stmt::Assign {
                        name: name.to_string(),
                        ty,
                        value: right_expr,
                    });
                }
                oxc_ast::ast::AssignmentTarget::ComputedMemberExpression(comp) => {
                    let (obj_expr, _) = lower_expr(&comp.object, ctx)?;
                    let (idx_expr, _) = lower_expr(&comp.expression, ctx)?;
                    if let OxcExpr::Identifier(id) = &comp.object {
                        if ctx.params.contains(id.name.as_str()) {
                            return Err(ctx.err(
                                Category::ParameterMutation,
                                format!("cannot mutate collection parameter '{}'", id.name),
                                assign.span,
                            ));
                        }
                    }
                    out.push(Stmt::SetItem {
                        collection: obj_expr,
                        index: idx_expr,
                        value: right_expr,
                    });
                }
                oxc_ast::ast::AssignmentTarget::StaticMemberExpression(mem) => {
                    let (obj_expr, _) = lower_expr(&mem.object, ctx)?;
                    let prop = mem.property.name.to_string();
                    out.push(Stmt::SetAttr {
                        object: obj_expr,
                        name: prop,
                        ty: right_ty,
                        value: right_expr,
                    });
                }
                _ => {
                    return Err(ctx.err(
                        Category::UnsupportedExpression,
                        "unsupported assignment target",
                        assign.span,
                    ));
                }
            }
        }
        OxcExpr::CallExpression(call) => {
            // Method calls like `xs.push(v)`, `set.add(v)`, or `map.set(k, v)`
            if let OxcExpr::StaticMemberExpression(stat) = &call.callee {
                let method_name = stat.property.name.as_str();
                let (target_expr, obj_ty) = lower_expr(&stat.object, ctx)?;
                if (method_name == "push" || method_name == "add")
                    && matches!(obj_ty, Ty::List(_) | Ty::Set(_))
                {
                    if let OxcExpr::Identifier(id) = &stat.object {
                        if ctx.params.contains(id.name.as_str()) {
                            return Err(ctx.err(
                                Category::ParameterMutation,
                                format!("cannot mutate collection parameter '{}'", id.name),
                                call.span,
                            ));
                        }
                    }
                    if let Some(arg) = call.arguments.first() {
                        if let Argument::NumericLiteral(n) = arg {
                            out.push(Stmt::Append {
                                sequence: target_expr,
                                value: Expr::Literal(Literal::Int(n.value as i64)),
                            });
                            return Ok(());
                        } else if let Argument::StringLiteral(s) = arg {
                            out.push(Stmt::Append {
                                sequence: target_expr,
                                value: Expr::Literal(Literal::Str(s.value.to_string())),
                            });
                            return Ok(());
                        } else if let Some(arg_expr) = arg.as_expression() {
                            let (val_expr, _) = lower_expr(arg_expr, ctx)?;
                            out.push(Stmt::Append {
                                sequence: target_expr,
                                value: val_expr,
                            });
                            return Ok(());
                        }
                    }
                } else if method_name == "set"
                    && call.arguments.len() == 2
                    && matches!(obj_ty, Ty::Dict(_, _))
                {
                    if let (Some(k_arg), Some(v_arg)) = (
                        call.arguments[0].as_expression(),
                        call.arguments[1].as_expression(),
                    ) {
                        let (k_expr, _) = lower_expr(k_arg, ctx)?;
                        let (v_expr, _) = lower_expr(v_arg, ctx)?;
                        out.push(Stmt::SetItem {
                            collection: target_expr,
                            index: k_expr,
                            value: v_expr,
                        });
                        return Ok(());
                    }
                }
            }
            let (expr, _) = lower_expr(expr, ctx)?;
            out.push(Stmt::Effect(expr));
        }
        _ => {
            let (expr, _) = lower_expr(expr, ctx)?;
            out.push(Stmt::Effect(expr));
        }
    }
    Ok(())
}

fn lower_expr<'a>(
    expr: &OxcExpr<'a>,
    ctx: &mut LoweringContext<'a>,
) -> Result<(Expr, Ty), compylr_core::frontend::LoweringError> {
    match expr {
        OxcExpr::NumericLiteral(num) => {
            if num.value.fract() == 0.0 {
                Ok((Expr::Literal(Literal::Int(num.value as i64)), Ty::Int))
            } else {
                Ok((Expr::Literal(Literal::float(num.value)), Ty::Float))
            }
        }
        OxcExpr::StringLiteral(s) => {
            Ok((Expr::Literal(Literal::Str(s.value.to_string())), Ty::Str))
        }
        OxcExpr::BooleanLiteral(b) => Ok((Expr::Literal(Literal::Bool(b.value)), Ty::Bool)),
        OxcExpr::Identifier(id) => {
            let name = id.name.as_str();
            if let Some((ty, _)) = ctx.lookup_var(name) {
                Ok((Expr::Name(name.to_string()), ty.clone()))
            } else {
                Err(ctx.err(
                    Category::TypeMismatch,
                    format!("unbound identifier '{name}'"),
                    id.span,
                ))
            }
        }
        OxcExpr::ThisExpression(this_expr) => {
            if let Some(class_name) = &ctx.in_class {
                Ok((
                    Expr::Name("self".to_string()),
                    Ty::Instance(class_name.clone()),
                ))
            } else {
                Err(ctx.err(
                    Category::UnsupportedExpression,
                    "'this' expression outside class method",
                    this_expr.span,
                ))
            }
        }
        OxcExpr::UnaryExpression(unary) => {
            let (arg_expr, arg_ty) = lower_expr(&unary.argument, ctx)?;
            match unary.operator {
                oxc_ast::ast::UnaryOperator::UnaryNegation => Ok((
                    Expr::Neg {
                        value: Box::new(arg_expr),
                        checked: ctx.behavior.arithmetic(),
                    },
                    arg_ty,
                )),
                oxc_ast::ast::UnaryOperator::LogicalNot => {
                    Ok((Expr::Not(Box::new(arg_expr)), Ty::Bool))
                }
                _ => Err(ctx.err(
                    Category::UnsupportedExpression,
                    "unsupported unary operator",
                    unary.span,
                )),
            }
        }
        OxcExpr::BinaryExpression(bin) => {
            let (left_expr, left_ty) = lower_expr(&bin.left, ctx)?;
            let (right_expr, right_ty) = lower_expr(&bin.right, ctx)?;
            match bin.operator {
                oxc_ast::ast::BinaryOperator::Addition => {
                    let ty = if left_ty == Ty::Str || right_ty == Ty::Str {
                        Ty::Str
                    } else if left_ty == Ty::Float || right_ty == Ty::Float {
                        Ty::Float
                    } else {
                        Ty::Int
                    };
                    Ok((
                        Expr::Binary {
                            op: BinOp::Add {
                                checked: ctx.behavior.arithmetic(),
                            },
                            left: Box::new(left_expr),
                            right: Box::new(right_expr),
                        },
                        ty,
                    ))
                }
                oxc_ast::ast::BinaryOperator::Subtraction => Ok((
                    Expr::Binary {
                        op: BinOp::Sub {
                            checked: ctx.behavior.arithmetic(),
                        },
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    if left_ty == Ty::Float || right_ty == Ty::Float {
                        Ty::Float
                    } else {
                        Ty::Int
                    },
                )),
                oxc_ast::ast::BinaryOperator::Multiplication => Ok((
                    Expr::Binary {
                        op: BinOp::Mul {
                            checked: ctx.behavior.arithmetic(),
                        },
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    if left_ty == Ty::Float || right_ty == Ty::Float {
                        Ty::Float
                    } else {
                        Ty::Int
                    },
                )),
                oxc_ast::ast::BinaryOperator::Division => Ok((
                    Expr::Binary {
                        op: ctx.behavior.integer_division(),
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Int,
                )),
                oxc_ast::ast::BinaryOperator::Remainder => Ok((
                    Expr::Binary {
                        op: ctx.behavior.remainder(),
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Int,
                )),
                oxc_ast::ast::BinaryOperator::StrictEquality
                | oxc_ast::ast::BinaryOperator::Equality => Ok((
                    Expr::Binary {
                        op: BinOp::Eq,
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Bool,
                )),
                oxc_ast::ast::BinaryOperator::StrictInequality
                | oxc_ast::ast::BinaryOperator::Inequality => Ok((
                    Expr::Binary {
                        op: BinOp::NotEq,
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Bool,
                )),
                oxc_ast::ast::BinaryOperator::LessThan => Ok((
                    Expr::Binary {
                        op: BinOp::Lt,
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Bool,
                )),
                oxc_ast::ast::BinaryOperator::LessEqualThan => Ok((
                    Expr::Binary {
                        op: BinOp::LtE,
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Bool,
                )),
                oxc_ast::ast::BinaryOperator::GreaterThan => Ok((
                    Expr::Binary {
                        op: BinOp::Gt,
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Bool,
                )),
                oxc_ast::ast::BinaryOperator::GreaterEqualThan => Ok((
                    Expr::Binary {
                        op: BinOp::GtE,
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    Ty::Bool,
                )),
                _ => Err(ctx.err(
                    Category::UnsupportedExpression,
                    "unsupported binary operator in TypeScript subset",
                    bin.span,
                )),
            }
        }
        OxcExpr::ParenthesizedExpression(p) => lower_expr(&p.expression, ctx),
        OxcExpr::TSNonNullExpression(non_null) => lower_expr(&non_null.expression, ctx),
        OxcExpr::TSAsExpression(as_expr) => lower_expr(&as_expr.expression, ctx),
        OxcExpr::TSSatisfiesExpression(sat) => lower_expr(&sat.expression, ctx),
        OxcExpr::TSTypeAssertion(type_assert) => lower_expr(&type_assert.expression, ctx),
        OxcExpr::LogicalExpression(log) => {
            let (left_expr, left_ty) = lower_expr(&log.left, ctx)?;
            let (_, right_ty) = lower_expr(&log.right, ctx)?;
            let ty = if left_ty == Ty::Bool && right_ty == Ty::Bool {
                Ty::Bool
            } else if left_ty != Ty::Unit {
                left_ty
            } else {
                right_ty
            };
            Ok((left_expr, ty))
        }
        OxcExpr::ArrayExpression(arr) => {
            let mut elements = Vec::new();
            let mut elem_ty = Ty::Int;
            for el in &arr.elements {
                if let ArrayExpressionElement::NumericLiteral(n) = el {
                    elements.push(Expr::Literal(Literal::Int(n.value as i64)));
                    elem_ty = Ty::Int;
                } else if let ArrayExpressionElement::StringLiteral(s) = el {
                    elements.push(Expr::Literal(Literal::Str(s.value.to_string())));
                    elem_ty = Ty::Str;
                } else if let Some(e) = el.as_expression() {
                    let (lowered, ty) = lower_expr(e, ctx)?;
                    elem_ty = ty;
                    elements.push(lowered);
                }
            }
            Ok((Expr::ListLit(elements), Ty::List(Box::new(elem_ty))))
        }
        OxcExpr::ComputedMemberExpression(comp) => {
            let (obj_expr, obj_ty) = lower_expr(&comp.object, ctx)?;
            if let Ty::Tuple(ref elems) = obj_ty {
                if let OxcExpr::NumericLiteral(num) = &comp.expression {
                    let pos = num.value as usize;
                    let elem_ty = elems.get(pos).cloned().unwrap_or(Ty::Int);
                    return Ok((
                        Expr::TupleIndex {
                            base: Box::new(obj_expr),
                            position: pos,
                        },
                        elem_ty,
                    ));
                }
            }
            let (idx_expr, _) = lower_expr(&comp.expression, ctx)?;
            let elem_ty = match obj_ty {
                Ty::List(inner) => *inner,
                Ty::Dict(_, v) => *v,
                Ty::Str => Ty::Str,
                _ => Ty::Int,
            };
            let origin = ctx.behavior.index_origin();
            let checked = ctx.behavior.index_checked();
            Ok((
                Expr::Subscript {
                    base: Box::new(obj_expr),
                    index: Box::new(idx_expr),
                    origin,
                    checked,
                },
                elem_ty,
            ))
        }
        OxcExpr::StaticMemberExpression(stat) => {
            let (obj_expr, obj_ty) = lower_expr(&stat.object, ctx)?;
            let prop = stat.property.name.as_str();
            if prop == "length" || prop == "size" {
                Ok((
                    Expr::Len {
                        value: Box::new(obj_expr),
                        units: ctx.behavior.text_units(),
                    },
                    Ty::Int,
                ))
            } else {
                let attr_ty = if let Some(class_name) = &ctx.in_class {
                    ctx.class_signatures
                        .get(class_name)
                        .and_then(|(attrs, _, _)| attrs.get(prop))
                        .cloned()
                        .unwrap_or(Ty::Int)
                } else if let Ty::Instance(class_name) = &obj_ty {
                    ctx.class_signatures
                        .get(class_name)
                        .and_then(|(attrs, _, _)| attrs.get(prop))
                        .cloned()
                        .unwrap_or(Ty::Int)
                } else {
                    ctx.class_signatures
                        .values()
                        .find_map(|(attrs, _, _)| attrs.get(prop))
                        .cloned()
                        .unwrap_or(Ty::Int)
                };
                Ok((
                    Expr::Attribute {
                        object: Box::new(obj_expr),
                        name: prop.to_string(),
                    },
                    attr_ty,
                ))
            }
        }
        OxcExpr::PrivateFieldExpression(priv_field) => Err(ctx.err(
            Category::UnsupportedExpression,
            "private fields are not supported in compylr subset",
            priv_field.span,
        )),
        OxcExpr::CallExpression(call) => {
            let mut args = Vec::new();
            for arg in &call.arguments {
                if let Argument::NumericLiteral(n) = arg {
                    args.push(Expr::Literal(Literal::Int(n.value as i64)));
                } else if let Argument::StringLiteral(s) = arg {
                    args.push(Expr::Literal(Literal::Str(s.value.to_string())));
                } else if let Some(e) = arg.as_expression() {
                    let (lowered, _) = lower_expr(e, ctx)?;
                    args.push(lowered);
                }
            }

            match &call.callee {
                OxcExpr::Identifier(id) => {
                    let callee_name = id.name.as_str();
                    if callee_name == "String" && !args.is_empty() {
                        return Ok((args.remove(0), Ty::Str));
                    }
                    let ret_ty = ctx
                        .signatures
                        .get(callee_name)
                        .map(|(_, ret)| ret.clone())
                        .unwrap_or(Ty::Int);
                    Ok((
                        Expr::Call {
                            callee: callee_name.to_string(),
                            args,
                        },
                        ret_ty,
                    ))
                }
                OxcExpr::StaticMemberExpression(stat) => {
                    let method_name = stat.property.name.as_str();
                    if let OxcExpr::Identifier(obj_id) = &stat.object {
                        if obj_id.name.as_str() == "Math"
                            && method_name == "floor"
                            && !args.is_empty()
                        {
                            return Ok((args.remove(0), Ty::Int));
                        }
                    }
                    let (obj_expr, obj_ty) = lower_expr(&stat.object, ctx)?;
                    if method_name == "get" && !args.is_empty() {
                        let elem_ty = match &obj_ty {
                            Ty::Dict(_, v) => (**v).clone(),
                            _ => Ty::Int,
                        };
                        return Ok((
                            Expr::Subscript {
                                base: Box::new(obj_expr),
                                index: Box::new(args.remove(0)),
                                origin: ctx.behavior.index_origin(),
                                checked: ctx.behavior.index_checked(),
                            },
                            elem_ty,
                        ));
                    } else if method_name == "has" && !args.is_empty() {
                        return Ok((
                            Expr::Contains {
                                container: Box::new(obj_expr),
                                value: Box::new(args.remove(0)),
                            },
                            Ty::Bool,
                        ));
                    } else if method_name == "keys" {
                        return Ok((obj_expr, obj_ty));
                    }
                    let ret_ty = if let Some(class_name) = &ctx.in_class {
                        ctx.class_signatures
                            .get(class_name)
                            .and_then(|(_, _, methods)| methods.get(method_name))
                            .map(|(_, ret)| ret.clone())
                            .unwrap_or(Ty::Int)
                    } else if let Ty::Instance(class_name) = &obj_ty {
                        ctx.class_signatures
                            .get(class_name)
                            .and_then(|(_, _, methods)| methods.get(method_name))
                            .map(|(_, ret)| ret.clone())
                            .unwrap_or(Ty::Int)
                    } else {
                        ctx.class_signatures
                            .values()
                            .find_map(|(_, _, methods)| methods.get(method_name))
                            .map(|(_, ret)| ret.clone())
                            .unwrap_or(Ty::Int)
                    };
                    Ok((
                        Expr::MethodCall {
                            receiver: Box::new(obj_expr),
                            class: None,
                            method: method_name.to_string(),
                            args,
                        },
                        ret_ty,
                    ))
                }
                _ => Err(ctx.err(
                    Category::UnsupportedExpression,
                    "unsupported callee expression",
                    call.span,
                )),
            }
        }
        OxcExpr::NewExpression(new_expr) => {
            if let OxcExpr::Identifier(id) = &new_expr.callee {
                let class_name = id.name.as_str();
                if class_name == "Map" {
                    return Ok((
                        Expr::DictLit(Vec::new()),
                        Ty::Dict(Box::new(Ty::Int), Box::new(Ty::Int)),
                    ));
                }
                if class_name == "Set" {
                    return Ok((Expr::SetLit(Vec::new()), Ty::Set(Box::new(Ty::Int))));
                }
                if class_name == "Array" {
                    return Ok((Expr::ListLit(Vec::new()), Ty::List(Box::new(Ty::Int))));
                }
                let mut args = Vec::new();
                for arg in &new_expr.arguments {
                    if let Some(e) = arg.as_expression() {
                        let (lowered, _) = lower_expr(e, ctx)?;
                        args.push(lowered);
                    }
                }
                Ok((
                    Expr::Construct {
                        class: class_name.to_string(),
                        args,
                    },
                    Ty::Instance(class_name.to_string()),
                ))
            } else {
                Err(ctx.err(
                    Category::UnsupportedExpression,
                    "unsupported new expression",
                    new_expr.span,
                ))
            }
        }
        other => Err(ctx.err(
            Category::UnsupportedExpression,
            "unsupported expression in TypeScript subset",
            other.span(),
        )),
    }
}

/// Lower multiple TypeScript sources into a single compylr Unit.
pub fn lower_typescript_sources(
    sources: &[compylr_core::frontend::Source],
) -> Result<Unit, compylr_core::frontend::LoweringError> {
    let allocator = Allocator::default();
    let mut parsed_programs = Vec::with_capacity(sources.len());
    for source in sources {
        let prog = parse_ts_source(&allocator, &source.text)?;
        parsed_programs.push((source, prog));
    }

    let mut class_names = HashSet::new();
    for (_, prog) in &parsed_programs {
        class_names.extend(collect_class_names(prog));
    }

    let mut signatures = HashMap::new();
    let mut class_signatures = HashMap::new();
    for (source, prog) in &parsed_programs {
        signatures.extend(collect_signatures(prog, &class_names, &source.text)?);
        class_signatures.extend(collect_class_signatures(prog, &class_names, &source.text)?);
    }

    let mut unit = Unit::new();
    for (source, prog) in &parsed_programs {
        for stmt in &prog.body {
            match stmt {
                OxcStmt::FunctionDeclaration(f) => {
                    let func = lower_function(
                        f,
                        &signatures,
                        &class_signatures,
                        &class_names,
                        source.behavior,
                        &source.text,
                    )?;
                    unit.add_function(func).map_err(|e| {
                        unsupported(Category::UnsupportedStatement, e.to_string(), 1, 1)
                    })?;
                }
                OxcStmt::ExportDeclaration(decl) => {
                    if let Declaration::FunctionDeclaration(f) = &decl.declaration {
                        let func = lower_function(
                            f.as_ref(),
                            &signatures,
                            &class_signatures,
                            &class_names,
                            source.behavior,
                            &source.text,
                        )?;
                        unit.add_function(func).map_err(|e| {
                            unsupported(Category::UnsupportedStatement, e.to_string(), 1, 1)
                        })?;
                    } else if let Declaration::ClassDeclaration(c) = &decl.declaration {
                        let cls = lower_class(
                            c.as_ref(),
                            &signatures,
                            &class_signatures,
                            &class_names,
                            source.behavior,
                            &source.text,
                        )?;
                        unit.add_class(cls).map_err(|e| {
                            unsupported(Category::UnsupportedStatement, e.to_string(), 1, 1)
                        })?;
                    }
                }
                OxcStmt::ClassDeclaration(c) => {
                    let cls = lower_class(
                        c,
                        &signatures,
                        &class_signatures,
                        &class_names,
                        source.behavior,
                        &source.text,
                    )?;
                    unit.add_class(cls).map_err(|e| {
                        unsupported(Category::UnsupportedStatement, e.to_string(), 1, 1)
                    })?;
                }
                _ => {}
            }
        }
    }

    unit.set_origin("typescript");
    Ok(unit)
}
