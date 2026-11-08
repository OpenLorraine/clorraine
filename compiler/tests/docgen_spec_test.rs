//! Specification tests for the `--doc` Markdown generator.
//!
//! The golden tree is authored from the documented output contract: sorted
//! modules, source-order declarations, exact signature slices, and linked
//! types. The test runs the built driver in a private directory so no run can
//! touch the committed fixture.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A temporary output directory removed when the test finishes.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "clorraine-docgen-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("scratch");
        Self { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The fixture and golden roots.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/doc")
}

/// Runs the driver's documentation mode.
fn run_doc(output: &Path, files: &[PathBuf]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clorraine"))
        .arg("--doc")
        .arg("-o")
        .arg(output)
        .args(files)
        .output()
        .expect("run clorraine --doc")
}

#[test]
fn should_write_the_golden_markdown_tree() {
    let scratch = Scratch::new("golden");
    let output = scratch.path.join("out");
    let files = vec![
        fixture_root().join("app.core.clo"),
        fixture_root().join("app.io.clo"),
    ];
    let result = run_doc(&output, &files);
    assert!(
        result.status.success(),
        "docgen failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for name in ["index.md", "app.core.md", "app.io.md"] {
        let expected = std::fs::read(fixture_root().join("golden").join(name)).expect("golden");
        let actual = std::fs::read(output.join(name)).expect("generated");
        assert_eq!(
            actual, expected,
            "{name} differs from the committed golden"
        );
    }
}

#[test]
fn should_be_deterministic_across_runs() {
    let scratch = Scratch::new("determinism");
    let first = scratch.path.join("first");
    let second = scratch.path.join("second");
    let files = vec![
        fixture_root().join("app.core.clo"),
        fixture_root().join("app.io.clo"),
    ];
    assert!(run_doc(&first, &files).status.success());
    assert!(run_doc(&second, &files).status.success());
    for name in ["index.md", "app.core.md", "app.io.md"] {
        assert_eq!(
            std::fs::read(first.join(name)).expect("first"),
            std::fs::read(second.join(name)).expect("second"),
            "{name} is not deterministic"
        );
    }
}

#[test]
fn should_write_nothing_when_a_source_has_a_syntax_error() {
    let scratch = Scratch::new("syntax");
    let source = scratch.path.join("bad.clo");
    std::fs::write(&source, "module app.bad;\nfunc broken( {\n").expect("write");
    let output = scratch.path.join("out");
    let result = run_doc(&output, &[source]);
    assert_eq!(result.status.code(), Some(1));
    assert!(!output.exists(), "a failed parse must write no output");
}

#[test]
fn should_reject_a_document_without_a_module() {
    let scratch = Scratch::new("nomodule");
    let source = scratch.path.join("plain.clo");
    std::fs::write(&source, "func main(): i32 { return 0; }\n").expect("write");
    let output = scratch.path.join("out");
    let result = run_doc(&output, &[source]);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("module"));
}
