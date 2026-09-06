use compylr_core::{Frontend, Source};
use compylr_frontend_typescript::TypeScriptFrontend;
use compylr_ir::{Behavior, Checked, Expr, IndexOrigin, Stmt};

const SOURCE: &str =
    include_str!("../../../frontends/typescript/fixtures/accepted/index_origin.ts");

#[test]
fn default_indices_count_only_from_the_start() {
    let frontend = TypeScriptFrontend;
    let unit = frontend
        .lower(&[Source::new(SOURCE, Behavior::of(frontend.behavior()))])
        .unwrap();
    unit.validate().unwrap();
    for function in unit.functions() {
        assert!(matches!(
            function.body[0],
            Stmt::Return(Expr::Subscript {
                origin: IndexOrigin::FromStart,
                checked: Checked::Reported,
                ..
            })
        ));
    }
}

#[test]
fn an_explicit_index_origin_override_is_preserved() {
    let frontend = TypeScriptFrontend;
    let mut behavior = *frontend.behavior();
    behavior.sequence_index.origin = IndexOrigin::FromEitherEnd;
    let unit = frontend
        .lower(&[Source::new(SOURCE, Behavior::of(&behavior))])
        .unwrap();
    for function in unit.functions() {
        assert!(matches!(
            function.body[0],
            Stmt::Return(Expr::Subscript {
                origin: IndexOrigin::FromEitherEnd,
                checked: Checked::Reported,
                ..
            })
        ));
    }
}
