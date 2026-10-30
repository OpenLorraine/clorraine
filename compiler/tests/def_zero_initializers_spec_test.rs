//! Specification tests for the zero-defect initializer and carrier batch.
//!
//! These cases are memory-safety judgments: an accepted program emits C,
//! builds under AddressSanitizer with leak detection, and runs clean; a
//! rejected program reports its frozen diagnostic and writes no C.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-zeroinitializers-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Compiles one host program and returns the compiler output and C path.
fn compile(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            c_path.to_str().expect("C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn clc");
    (scratch, output, c_path)
}

/// Runs one accepted program under AddressSanitizer with leak detection.
fn run_asan(tag: &str, source: &str) -> Output {
    let (scratch, output, c_path) = compile(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-fsanitize=address",
            "-no-pie",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the ASan build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new(&program)
        .env("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")
        .output()
        .expect("failed to spawn the built program")
}

/// Asserts one exit code and a clean sanitizer report.
fn assert_clean(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: unexpected exit code; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("ERROR: AddressSanitizer") && !stderr.contains("LeakSanitizer"),
        "{tag}: sanitizer report:\n{stderr}"
    );
}

/// Asserts one diagnostic count, optional fragments, and no emitted C.
fn assert_rejected(tag: &str, source: &str, code: &str, expected: usize, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected {code}, but the front end accepted the program"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let found = stderr.matches(&format!("error {code}:")).count();
    assert_eq!(
        found, expected,
        "{tag}: expected {expected} {code} report(s), found {found}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the report to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(!c_path.exists(), "{tag}: a rejected program must not write C");
}

// ---------------------------------------------------------------------------
// an initializer `take` argument projecting a call-result element.
// ---------------------------------------------------------------------------

/// A module-initializer `take` of one element from a call result reports the
/// same partial-move limit as the function-body twin.
#[test]
fn should_reject_an_initializer_take_of_a_call_result_element() {
    let source = r#"module spec.zi.init_take_element;

import { String, stringOf } from std.text;

func two(): String[2] {
    return [stringOf("a"), stringOf("bb")];
}

func consume(take xs: String): i32 {
    return i32(len(xs));
}

let g: i32 = consume(two()[0]);

func main(): i32 {
    return g - 1;
}
"#;
    assert_rejected(
        "init_take_element",
        source,
        "E10026",
        1,
        &["partial moves are limited to top-level fields in v1"],
    );
}

// ---------------------------------------------------------------------------
// a duplicable member argument under a guard.
// ---------------------------------------------------------------------------

/// A guarded scalar member argument releases its fresh owning root.
///
/// `use(mk().n)` reads a duplicable scalar, but `mk()` owns a `String` field;
/// the deferred root bind must release that residual field on the evaluated
/// path, and the skipped path must not run the call.
#[test]
fn should_release_a_guarded_duplicable_member_argument_root() {
    let source = r#"module spec.zi.guarded_member_arg;

import { String, stringOf } from std.text;

struct Box { n: i32; v: String; }

func mk(): Box {
    return Box { n: 1, v: stringOf("0123456789") };
}

func use(x: i32): i32 {
    return x;
}

func main(): i32 {
    let g: bool = true;
    if (g && use(mk().n) == 1) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_asan("guarded_member_arg", source);
    assert_clean("guarded_member_arg", &output, 0);
}

// ---------------------------------------------------------------------------
// a module-initializer carrier's residual sibling.
// ---------------------------------------------------------------------------

/// The view element read from an initializer literal keeps its backing while
/// the literal's residual sibling fields release.
#[test]
fn should_release_a_module_initializer_carrier_residual() {
    let source = r#"module spec.zi.initializer_carrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct H { names: str[1]; tag: String; }

func mk(): Box {
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

let g: str = H { names: [mk().v], tag: stringOf("t") }.names[0];

func main(): i32 {
    return i32(len(g)) - 2;
}
"#;
    let output = run_asan("initializer_carrier", source);
    assert_clean("initializer_carrier", &output, 0);
}

/// A conditional arm that takes an ownerless element reports the body's
/// partial-move limit.
#[test]
fn should_reject_a_conditional_initializer_element_take() {
    let source = r#"module spec.zi.conditional_take_element;

import { String, stringOf } from std.text;

func two(): String[2] {
    return [stringOf("a"), stringOf("bb")];
}

func consume(take xs: String): i32 {
    return i32(len(xs));
}

let g: i32 = consume(true ? two()[0] : two()[1]);

func main(): i32 {
    return g - 1;
}
"#;
    assert_rejected(
        "conditional_take_element",
        source,
        "E10026",
        1,
        &["partial moves are limited to top-level fields in v1"],
    );
}
