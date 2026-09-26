//! Implementation tests for partial-move / drop coherence.
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
            "clc-partial-move-{}-{tag}-{sequence}",
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

/// Builds and runs one source, asserting a clean exit.
///
/// Each owned value is freed exactly once, so the run must exit zero and print
/// no C-library double-free message. The pre-fix programs abort on the double
/// free, which makes this oracle red before the fix and green after.
fn run_clean(tag: &str, source: &str) {
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
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected a clean exit; stderr:\n{stderr}"
    );
}

/// A binding moved into a struct literal keeps a single owner.
#[test]
fn a_temporary_binding_moved_into_a_literal_frees_once() {
    run_clean(
        "tmp",
        r#"module tests.pm.tmp;

import { String, stringOf } from std.text;

struct Box<T> {
    value: T;
}

func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    let t = b.value;
    b = Box<String> { value: t };
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// A literal that reads its own assignment target keeps a single owner.
#[test]
fn a_literal_reading_its_target_frees_once() {
    run_clean(
        "self",
        r#"module tests.pm.self;

import { String, stringOf } from std.text;

struct Box<T> {
    value: T;
}

func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    b = Box<String> { value: b.value };
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// The non-generic spelling of the reassignment keeps a single owner.
#[test]
fn a_non_generic_owning_literal_frees_once() {
    run_clean(
        "plain",
        r#"module tests.pm.plain;

import { String, stringOf } from std.text;

struct Label {
    name: String;
}

func demo(): void {
    let l = Label { name: stringOf("x") };
    l = Label { name: l.name };
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// A literal that moves two owned fields keeps a single owner for each.
#[test]
fn a_multi_field_move_frees_once() {
    run_clean(
        "multi",
        r#"module tests.pm.multi;

import { String, stringOf } from std.text;

struct Pair<T> {
    first: T;
    second: T;
}

func demo(): void {
    let p = Pair<String> { first: stringOf("a"), second: stringOf("b") };
    p = Pair<String> { first: p.first, second: p.second };
}

func main(): i32 { demo(); return 0; }
"#,
    );
}

/// A parenthesized (postfix) field path is consumed like a dotted one.
#[test]
fn a_postfix_field_move_frees_once() {
    run_clean(
        "postfix",
        r#"module tests.pm.postfix;

import { String, stringOf } from std.text;

struct Box<T> {
    value: T;
}

func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    b = Box<String> { value: (b).value };
}

func main(): i32 { demo(); return 0; }
"#,
    );
}
