use compylr_core::{Frontend, Source};
use compylr_frontend_typescript::TypeScriptFrontend;
use compylr_ir::{Behavior, BinOp, Expr, Stmt, Ty};

#[test]
fn lowers_simple_addition() {
    let frontend = TypeScriptFrontend;
    let code = r#"
function add(a: number, b: number): number {
    return a + b;
}
"#;
    let source = Source::new(code, Behavior::of(frontend.behavior()));
    let unit = frontend.lower(&[source]).expect("lowering failed");

    let func = unit
        .functions()
        .find(|f| f.name == "add")
        .expect("function add not found");
    assert_eq!(func.name, "add");
    assert_eq!(func.params.len(), 2);
    assert_eq!(func.params[0].name, "a");
    assert_eq!(func.params[0].ty, Ty::Int);
    assert_eq!(func.params[1].name, "b");
    assert_eq!(func.params[1].ty, Ty::Int);
    assert_eq!(func.ret, Ty::Int);

    match &func.body[0] {
        Stmt::Return(Expr::Binary { op, left, right }) => {
            assert!(matches!(op, BinOp::Add { .. }));
            assert_eq!(&**left, &Expr::Name("a".to_string()));
            assert_eq!(&**right, &Expr::Name("b".to_string()));
        }
        other => panic!("unexpected body stmt: {other:?}"),
    }
}

#[test]
fn lowers_if_while_and_for_loops() {
    let frontend = TypeScriptFrontend;
    let code = r#"
function loopTest(n: number): number {
    let count: number = 0;
    if (n > 0) {
        let i: number = 0;
        while (i < n) {
            count = count + i;
            i = i + 1;
        }
    }
    const items: Array<number> = [1, 2, 3];
    for (const x of items) {
        count = count + x;
    }
    return count;
}
"#;
    let source = Source::new(code, Behavior::of(frontend.behavior()));
    let unit = frontend.lower(&[source]).expect("lowering failed");

    let func = unit
        .functions()
        .find(|f| f.name == "loopTest")
        .expect("function not found");
    assert_eq!(func.ret, Ty::Int);
    assert!(!func.body.is_empty());
}

#[test]
fn lowers_classes_and_methods() {
    let frontend = TypeScriptFrontend;
    let code = r#"
class Counter {
    value: number;

    constructor(initial: number) {
        this.value = initial;
    }

    bump(): void {
        this.value = this.value + 1;
    }

    get(): number {
        return this.value;
    }
}
"#;
    let source = Source::new(code, Behavior::of(frontend.behavior()));
    let unit = frontend.lower(&[source]).expect("lowering failed");

    let cls = unit
        .classes()
        .find(|c| c.name == "Counter")
        .expect("class not found");
    assert_eq!(cls.name, "Counter");
    assert_eq!(cls.attributes.len(), 1);
    assert_eq!(cls.attributes[0].name, "value");
    assert_eq!(cls.attributes[0].ty, Ty::Int);

    assert_eq!(cls.methods.len(), 2);
    assert!(cls.methods.contains_key("bump"));
    assert!(cls.methods.contains_key("get"));
}

#[test]
fn rejects_unannotated_parameter() {
    let frontend = TypeScriptFrontend;
    let code = r#"
function bad(x): number {
    return x;
}
"#;
    let source = Source::new(code, Behavior::of(frontend.behavior()));
    let err = frontend.lower(&[source]).unwrap_err();
    assert!(
        err.to_string()
            .contains("parameter 'x' must have an explicit type annotation")
    );
}

#[test]
fn rejects_missing_return_on_all_paths() {
    let frontend = TypeScriptFrontend;
    let code = r#"
function missingReturn(n: number): number {
    if (n > 0) {
        return n;
    }
}
"#;
    let source = Source::new(code, Behavior::of(frontend.behavior()));
    let err = frontend.lower(&[source]).unwrap_err();
    assert!(
        err.to_string()
            .contains("does not return a value on all paths")
    );
}

#[test]
fn rejects_mutating_parameter() {
    let frontend = TypeScriptFrontend;
    let code = r#"
function mutateParam(xs: Array<number>): void {
    xs.push(1);
}
"#;
    let source = Source::new(code, Behavior::of(frontend.behavior()));
    let err = frontend.lower(&[source]).unwrap_err();
    assert!(
        err.to_string()
            .contains("cannot mutate collection parameter")
    );
}
