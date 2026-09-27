//! Implementation tests for non-lexical lifetimes.
//!
//! The specification suite stops at the front end, so it cannot observe whether
//! an accepted program actually builds and runs. These tests compile the
//! accepted shapes to C, build that C with the host C compiler, and run it, and
//! they pin the reporting behavior: a conflict is reported once and two runs
//! produce identical diagnostics. A defect here is an implementation bug, not a
//! specification change.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-nll-impl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Compiles one source under the host profile and returns the process output
/// together with the path of the emitted C, when it was written.
fn compile(_tag: &str, scratch: &ScratchDir, source: &str) -> (Output, PathBuf) {
    let source_path = scratch.path().join("main.clo");
    let c_path = scratch.path().join("out.c");
    std::fs::write(&source_path, source).expect("cannot write the snippet");
    let run = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            source_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    (run, c_path)
}

/// Builds and runs one accepted program, asserting a clean exit.
///
/// The front end must accept the program, the emitted C must compile with
/// warnings as errors, and the program must exit zero. This is the runtime
/// oracle the compile-only specification suite cannot provide.
fn run_clean(tag: &str, source: &str) {
    let scratch = ScratchDir::new(tag);
    let (compile, c_path) = compile(tag, &scratch, source);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99".to_owned(),
            "-O0".to_owned(),
            "-g".to_owned(),
            "-Wall".to_owned(),
            "-Wextra".to_owned(),
            "-Werror".to_owned(),
            "-o".to_owned(),
            program.display().to_string(),
            c_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let output = Command::new(&program)
        .output()
        .expect("failed to run the program");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected a clean exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `spec/16-borrow-rules.md` section 7: a move after the view's last use is
/// accepted and the program builds and runs.
#[test]
fn a_move_after_the_views_last_use_builds_and_runs() {
    run_clean(
        "move-after-last-use",
        r#"module tests.nll_impl.move_after_last_use;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    observe(v);
    sink(s);
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// `spec/16-borrow-rules.md` section 7: a widened view used inside a shorter
/// scope is accepted and the program builds and runs.
#[test]
fn a_widened_view_inside_a_shorter_scope_builds_and_runs() {
    run_clean(
        "widen-inside-scope",
        r#"module tests.nll_impl.view_inside_shorter_scope;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let held: str;
    {
        let s: String = stringOf("x");
        held = s;
        observeStr(held);
    }
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// `spec/03-ownership.md`: a defer that owns a view and uses it before its own
/// release is accepted and the program builds and runs.
#[test]
fn a_defer_that_owns_and_uses_its_view_builds_and_runs() {
    run_clean(
        "defer-local-owner",
        r#"module tests.nll_impl.defer_local_owner;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    defer {
        let s: String = stringOf("x");
        let v: str = s;
        observeStr(v);
    }
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// `spec/14-diagnostics.md` E10022: a move before the view's last use is
/// reported exactly once.
#[test]
fn a_conflict_is_reported_exactly_once() {
    let scratch = ScratchDir::new("conflict-once");
    let source = r#"module tests.nll_impl.conflict_once;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    sink(s);
    observe(v);
}

func main(): i32 { demo(); return 0; }
"#;
    let (run, _) = compile("conflict-once", &scratch, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    let seen = stderr
        .lines()
        .filter(|line| *line == "error E10022: cannot move 's' while it is borrowed")
        .count();
    assert_eq!(
        seen, 1,
        "expected exactly one E10022; stderr:\n{stderr}"
    );
    assert_eq!(run.status.code(), Some(1));
}

/// The diagnostics of one compilation are deterministic: compiling the same
/// program twice produces byte-identical stderr.
#[test]
fn two_runs_produce_identical_diagnostics() {
    let scratch = ScratchDir::new("determinism");
    let source = r#"module tests.nll_impl.determinism;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    sink(s);
    observe(v);
}

func main(): i32 { demo(); return 0; }
"#;
    let (first, _) = compile("determinism-a", &scratch, source);
    let (second, _) = compile("determinism-b", &scratch, source);
    assert_eq!(
        first.stderr, second.stderr,
        "the diagnostics changed between two identical compilations"
    );
    assert_eq!(first.status.code(), second.status.code());
}
