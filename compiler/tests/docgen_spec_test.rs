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

/// Copies the two source fixtures into a scratch directory.
fn copied_fixtures(scratch: &Scratch) -> Vec<PathBuf> {
    let directory = scratch.path.join("fixture");
    std::fs::create_dir_all(&directory).expect("fixture copy");
    let mut paths = Vec::new();
    for name in ["app.core.clo", "app.io.clo"] {
        let target = directory.join(name);
        std::fs::copy(fixture_root().join(name), &target).expect("copy fixture");
        paths.push(target);
    }
    paths
}

/// Runs the driver's documentation mode.
fn run_doc(output: &Path, files: &[PathBuf]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clorraine"))
        .arg("--doc")
        .arg("-o")
        .arg(output)
        .args(files)
        .env_clear()
        .output()
        .expect("run clorraine --doc")
}

#[test]
fn should_write_the_golden_markdown_tree() {
    let scratch = Scratch::new("golden");
    let output = scratch.path.join("out");
    let files = copied_fixtures(&scratch);
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
    let files = copied_fixtures(&scratch);
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


#[test]
fn should_refuse_to_overwrite_a_symlink() {
    #[cfg(unix)]
    {
        let scratch = Scratch::new("symlink");
        let output = scratch.path.join("out");
        std::fs::create_dir_all(&output).expect("out");
        let victim = scratch.path.join("victim.txt");
        std::fs::write(&victim, "keep me").expect("victim");
        std::os::unix::fs::symlink(&victim, output.join("app.core.md")).expect("symlink");
        let result = run_doc(&output, &copied_fixtures(&scratch));
        assert_eq!(result.status.code(), Some(2));
        assert_eq!(std::fs::read_to_string(&victim).expect("victim"), "keep me");
    }
}

#[test]
fn should_reject_a_mixed_input_with_a_missing_module() {
    let scratch = Scratch::new("mixed");
    let source = scratch.path.join("plain.clo");
    std::fs::write(&source, "func helper(): i32 { return 1; }\n").expect("write");
    let mut files = copied_fixtures(&scratch);
    files.push(source);
    let output = scratch.path.join("out");
    let result = run_doc(&output, &files);
    assert_eq!(result.status.code(), Some(1));
    assert!(!output.exists(), "a missing module must write nothing");
}

#[test]
fn should_reject_a_module_named_index() {
    let scratch = Scratch::new("index-name");
    let source = scratch.path.join("index.clo");
    std::fs::write(&source, "module index;\nfunc helper(): i32 { return 1; }\n").expect("write");
    let output = scratch.path.join("out");
    let result = run_doc(&output, &[source]);
    assert_eq!(result.status.code(), Some(2));
    assert!(!output.exists());
}

#[test]
fn should_write_lf_only_from_a_crlf_source() {
    let scratch = Scratch::new("crlf");
    let source = scratch.path.join("crlf.clo");
    std::fs::write(
        &source,
        "module app.crlf;\r\n\r\n/// Describes.\r\nexport func describe(\r\n    value: i32\r\n): void {\r\n}\r\n",
    )
    .expect("write");
    let output = scratch.path.join("out");
    let result = run_doc(&output, &[source]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let page = std::fs::read(output.join("app.crlf.md")).expect("page");
    assert!(!page.contains(&b'\r'), "generated Markdown must use LF only");
}
