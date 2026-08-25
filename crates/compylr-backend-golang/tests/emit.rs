use compylr_backend_golang::GoBackend;
use compylr_core::Backend;
use compylr_ir::{BinOp, Checked, Expr, Function, Param, Stmt, Ty, Unit};

#[test]
fn emits_valid_go_package_and_function() {
    let backend = GoBackend;
    let mut unit = Unit::new();
    let func = Function {
        name: "Add".to_string(),
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
        body: vec![Stmt::Return(Expr::Binary {
            op: BinOp::Add {
                checked: Checked::Unchecked,
            },
            left: Box::new(Expr::Name("a".to_string())),
            right: Box::new(Expr::Name("b".to_string())),
        })],
        doc: None,
        span: Default::default(),
    };
    unit.add_function(func).unwrap();

    let files = backend.emit(&unit).expect("emission failed");
    assert_eq!(files.len(), 3);
    assert!(files.contains_key("go.mod"));
    assert!(files.contains_key("compat.go"));

    let text = files.get("generated.go").expect("generated.go missing");
    assert!(text.contains("package main"));
    assert!(text.contains("func Add(a int64, b int64) int64 {"));
    assert!(text.contains("return (a + b)"));
}
