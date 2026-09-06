//! Fractional literals must survive lowering, compile, and execute without truncation.

use std::path::PathBuf;
use std::process::Command;

use compylr_core::{Behavior, Source};
use compylr_registry::{backends, frontends};

#[test]
fn fractional_literals_compile_and_run() {
    let frontend = frontends::lookup("typescript").unwrap();
    let source = Source::new(
        include_str!("../../../frontends/typescript/fixtures/accepted/float_literals.ts"),
        Behavior::of(frontend.behavior()),
    );
    let unit = frontend.lower(&[source]).unwrap();
    unit.validate().unwrap();

    // Go hardcodes nonempty list literals as []int64, independently of lowering.
    // Exercise the complete fixture through Rust until that backend defect is fixed.
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("typescript-float-literals-rust");
    let files = backends::lookup("rust").unwrap().emit(&unit).unwrap();
    for (path, contents) in &files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
    let main = format!(
        "{}\nfn main() {{
            assert_eq!(generated::float_call().unwrap(), 3.7);
            assert_eq!(generated::float_array().unwrap(), vec![1.5, 2.5]);
            assert_eq!(generated::float_push().unwrap(), vec![1.5]);
        }}\n",
        files["src/lib.rs"]
    );
    std::fs::write(root.join("src/main.rs"), main).unwrap();
    let binary = root.join("float-literals");
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
