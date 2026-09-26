//! Implementation tests for array ownership and drop.
//!
//! These tests compile an owning program, build the emitted C with the host C
//! compiler, and run it. A double free aborts inside the C library
//! (`free(): double free detected`), so a non-zero exit catches the defect that
//! the compile-only specification suite cannot observe. They are not the
//! specification oracle; a defect here is an implementation bug.

use std::path::{Path, PathBuf};
use std::process::Command;
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
            "clc-ownership-drop-{}-{tag}-{sequence}",
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

/// Compiles one source to C under the host profile and returns the C path.
///
/// The front end must accept the program; a rejection is a test failure, not a
/// result to assert.
fn emit_c(tag: &str, scratch: &ScratchDir, source: &str) -> PathBuf {
    let c_path = scratch.path().join("out.c");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("cannot write the snippet");
    let compile = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            source_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    c_path
}

/// Builds and runs one source, asserting no double free.
///
/// A double free aborts inside the C library, so the run's exit code and stderr
/// are the oracle. The caller decides which exit code is expected.
fn build_and_run(tag: &str, source: &str) -> std::process::Output {
    let scratch = ScratchDir::new(tag);
    let c_path = emit_c(tag, &scratch, source);
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("double free"),
        "{tag}: the C library reported a double free:\n{stderr}"
    );
    output
}

/// Builds and runs one source, asserting a clean exit (no double free, exit 0).
fn run_clean(tag: &str, source: &str) {
    let output = build_and_run(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected a clean exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A by-value fixed-array return transfers each element to the caller exactly
/// once: the returned `String` elements are released by the caller, not by the
/// callee's cleanup.
#[test]
fn a_returned_owning_fixed_array_frees_each_element_once() {
    run_clean(
        "ret-own",
        r#"module tests.od.return_owning;

import { String, stringOf } from std.text;

func pair(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func demo(): void {
    let p: String[2] = pair();
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// A by-value fixed-array return copies the element values, so the caller can
/// read them back after the callee has returned.
#[test]
fn a_returned_scalar_fixed_array_keeps_its_values() {
    let output = build_and_run(
        "ret-scalar",
        r#"module tests.od.return_scalar;

func pair(): u8[2] {
    return [7, 9];
}

func main(): i32 {
    let p: u8[2] = pair();
    return i32(p[0]) + i32(p[1]);
}
"#,
    );
    assert_eq!(
        output.status.code(),
        Some(16),
        "the returned scalar elements must be 7 and 9; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A forwarded fixed-array return threads the destination through both calls,
/// so the owning elements transfer once and nothing is freed twice.
#[test]
fn a_forwarded_owning_fixed_array_return_frees_each_element_once() {
    run_clean(
        "ret-forward",
        r#"module tests.od.forward_owning;

import { String, stringOf } from std.text;

func inner(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func outer(): String[2] {
    return inner();
}

func demo(): void {
    let p: String[2] = outer();
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// A discarded fixed-array-returning call passes a destination, so the emitted
/// C compiles and the run does not free anything twice.
#[test]
fn a_discarded_owning_fixed_array_return_runs_clean() {
    run_clean(
        "ret-discard",
        r#"module tests.od.discard_owning;

import { String, stringOf } from std.text;

func inner(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func demo(): void {
    inner();
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// A nested array literal fills every dimension, so the emitted C compiles and
/// a deep element reads back its initializer value.
#[test]
fn a_nested_array_literal_keeps_its_element_values() {
    let output = build_and_run(
        "nested-literal",
        r#"module tests.od.nested_literal_run;

func main(): i32 {
    let d: u8[3][2] = [[1, 2], [3, 4], [5, 6]];
    return i32(d[2][1]);
}
"#,
    );
    assert_eq!(
        output.status.code(),
        Some(6),
        "the deep element must be 6; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
