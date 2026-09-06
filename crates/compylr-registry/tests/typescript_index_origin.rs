//! Compile TypeScript indexing through Rust and check bounds and explicit overrides.

use std::path::PathBuf;
use std::process::Command;

use compylr_core::{Behavior, Source};
use compylr_ir::IndexOrigin;
use compylr_registry::{backends, frontends};

#[test]
fn negative_indices_do_not_select_the_last_element_by_default() {
    let frontend = frontends::lookup("typescript").unwrap();
    let source = include_str!("../../../frontends/typescript/fixtures/accepted/index_origin.ts");
    for from_end in [false, true] {
        let mut behavior = *frontend.behavior();
        if from_end {
            behavior.sequence_index.origin = IndexOrigin::FromEitherEnd;
        }
        let unit = frontend
            .lower(&[Source::new(source, Behavior::of(&behavior))])
            .unwrap();
        unit.validate().unwrap();
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("typescript-index-origin-{from_end}"));
        let files = backends::lookup("rust").unwrap().emit(&unit).unwrap();
        for (path, contents) in &files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let main = format!(
            "{}\nfn main() {{
                use generated::{{typescript_at, typescript_last}};
                let xs = vec![10, 20, 30];
                assert_eq!(typescript_at(xs.clone(), 0).unwrap(), 10);
                assert_eq!(typescript_at(xs.clone(), 2).unwrap(), 30);
                assert!(typescript_at(xs.clone(), 3).is_err());
                assert!(typescript_at(xs.clone(), -4).is_err());
                assert!(typescript_at(vec![], 0).is_err());
                assert!(typescript_last(vec![]).is_err());
                if {from_end} {{
                    assert_eq!(typescript_last(xs.clone()).unwrap(), 30);
                    assert_eq!(typescript_at(xs.clone(), -1).unwrap(), 30);
                    assert_eq!(typescript_at(xs, -3).unwrap(), 10);
                }} else {{
                    assert!(typescript_last(xs.clone()).is_err());
                    assert!(typescript_at(xs.clone(), -1).is_err());
                    assert!(typescript_at(xs, -3).is_err());
                }}
            }}\n",
            files["src/lib.rs"]
        );
        std::fs::write(root.join("src/main.rs"), main).unwrap();
        let binary = root.join("index-origin");
        let build = Command::new("rustc")
            .args(["--edition=2024", "src/main.rs", "-o"])
            .arg(&binary)
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "{}",
            String::from_utf8_lossy(&build.stderr)
        );
        let output = Command::new(binary).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
