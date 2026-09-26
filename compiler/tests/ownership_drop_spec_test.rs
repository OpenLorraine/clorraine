//! Specification tests for array ownership, classification, and drop.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary and
//! asserts the behavior the frozen specification requires for arrays: an element
//! moved into an array literal cannot be used again, a fixed array stays a lend
//! candidate for return elision, and a fixed array is not nullable. The cases
//! drive the published command-line behavior, so a regression in the front end
//! is visible as a wrong diagnostic or an unexpected acceptance.
//!
//! Cases whose expectation is only observable at run time (an element released
//! exactly once) live in `ownership_drop_impl_test.rs`, which builds and runs the
//! emitted C against the host C library.

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

/// Compiles one source snippet for the host profile and returns the output.
fn run_source(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            source_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Asserts that compiling one snippet reports every expected diagnostic line.
fn assert_reports(tag: &str, source: &str, expected_lines: &[&str]) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    for expected in expected_lines {
        assert!(
            stderr.lines().any(|line| line == *expected),
            "{tag}: missing diagnostic `{expected}`; stderr:\n{stderr}"
        );
    }
    assert_eq!(run.status.code(), Some(1), "{tag}: failing compiles exit 1");
}

/// Asserts that the compiler accepts a snippet and emits C.
fn assert_accepts(tag: &str, source: &str) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{stderr}"
    );
    assert!(
        !stderr.lines().any(|line| line.starts_with("error ")),
        "{tag}: unexpected error diagnostic; stderr:\n{stderr}"
    );
}

/// Compiles one snippet and returns the scratch directory and emitted C path.
///
/// The front end must accept the program; the caller inspects the emitted C.
fn emit_c(tag: &str, source: &str) -> (ScratchDir, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let c_path = scratch.path().join("out.c");
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
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    (scratch, c_path)
}

/// Asserts that the emitted translation unit is valid C.
///
/// The front end can accept a program whose emitted C does not compile, so the
/// host C compiler is the oracle for the emitter half of the case. This is the
/// checker-plus-emitter contract; run-time behavior is covered separately.
fn assert_emits_valid_c(tag: &str, source: &str) {
    let (_scratch, c_path) = emit_c(tag, source);
    let built = Command::new("cc")
        .args(["-std=c99", "-fsyntax-only"])
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// `spec/08-arrays-strings.md` §6: a binding moved into an array literal is
/// invalid afterward, so reading it again is `E10020`.
///
/// The expectation is the moved-source rule: the array owns its elements the
/// same way a struct literal owns its fields.
#[test]
fn should_report_e10020_when_a_moved_element_source_is_used() {
    let source = r#"module tests.od.classification.moved;

import { String, stringOf } from std.text;

func observe(s: String): void {}

func demo(): void {
    let a: String = stringOf("a");
    let arr: String[1] = [a];
    observe(a);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "st2",
        source,
        &["error E10020: 'a' was moved and cannot be used"],
    );
}

/// `spec/16-borrow-rules.md`: a fixed-array parameter is borrowed storage, so it
/// stays a lifetime-elision candidate and a view may be returned from it.
///
/// This case passed before the change: it is the regression guard for routing
/// the shape consumers, and it must keep passing.
#[test]
fn should_accept_a_view_return_derived_from_a_fixed_array_parameter() {
    let source = r#"module tests.od.classification.lend;

import { slice } from std.mem;

func head(arr: u8[2]): u8[] {
    return slice(arr, 0, 1);
}

func main(): i32 {
    let data: u8[2] = [1, 2];
    let h: u8[] = head(data);
    return i32(h[0]);
}
"#;
    assert_accepts("st15", source);
}

/// `spec/08-arrays-strings.md` §3: a fixed array is inline owned storage, so the
/// `?` empty state is meaningless on it and the suffix is rejected.
#[test]
fn should_reject_a_nullable_fixed_array() {
    let source = r#"module tests.od.classification.nullable;

import { String } from std.text;

func demo(): void {
    let a: String[2]? = null;
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "st16",
        source,
        &["error E10011: nullable type is not allowed on 'String[2]'"],
    );
}

/// `spec/08-arrays-strings.md` §6: a function may return a fixed array by value.
///
/// C cannot return an array, so the emitter must lower the by-value return into
/// valid C (a hidden out-parameter) rather than `return <array>`. The emitted
/// translation unit is the oracle here; the run-time release behavior is the
/// companion implementation test. A slice return is unchanged and is covered by
/// the lend-candidate case above.
#[test]
fn should_emit_valid_c_when_a_function_returns_a_fixed_array_by_value() {
    let source = r#"module tests.od.freereturn;

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
"#;
    assert_emits_valid_c("st1", source);
}

/// A function may return the result of another fixed-array-returning call.
///
/// The hidden destination is threaded through both calls, so the emitted C
/// stays valid for the forwarded form as well as the literal form.
#[test]
fn should_emit_valid_c_when_a_fixed_array_return_forwards_another_call() {
    let source = r#"module tests.od.forward_return;

import { String, stringOf } from std.text;

func inner(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func outer(): String[2] {
    return inner();
}

func main(): i32 { let p: String[2] = outer(); return 0; }
"#;
    assert_emits_valid_c("rv002", source);
}

/// A discarded fixed-array-returning call still provides a destination.
///
/// The result is not used, but the callee still writes through a hidden
/// out-parameter, so the emitter must pass one even when the value is dropped.
#[test]
fn should_emit_valid_c_when_a_fixed_array_result_is_discarded() {
    let source = r#"module tests.od.discard_return;

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
"#;
    assert_emits_valid_c("rv003", source);
}

/// `spec/08-arrays-strings.md`: a nested array literal initializes a
/// multi-dimensional fixed array, one dimension per brace level.
///
/// The front end already accepts the shape, but C cannot assign a brace list to
/// an array element, so the emitter must recurse one dimension per level. The
/// host C compiler is the oracle: at the defect the element assignment is
/// `d[0] = { 1, 2 };`, which C rejects.
#[test]
fn should_emit_valid_c_when_a_nested_array_literal_initializes_a_fixed_array() {
    let source = r#"module tests.od.nested_literal;

func demo(): void {
    let d: u8[3][2] = [[1, 2], [3, 4], [5, 6]];
}

func main(): i32 { demo(); return 0; }
"#;
    assert_emits_valid_c("pf003", source);
}

/// `spec/appendix-stdlib.md`: a growable `Array<T>` accepts a move-only element
/// now that element drop glue exists, so the staged `E90000` no longer fires.
#[test]
fn should_accept_a_growable_array_of_a_move_only_element() {
    let source = r#"module tests.od.growable_move_only;

import { Buffer, Array, arrayNew } from std.mem;

func main(): i32 {
    let xs: Array<Buffer> = arrayNew<Buffer>();
    return 0;
}
"#;
    assert_accepts("st8", source);
}

/// `spec/appendix-stdlib.md:62`: replacing a growable element is accepted for a
/// move-only element, which the former staged `E90000` rejected.
#[test]
fn should_accept_replacing_a_growable_move_only_element() {
    let source = r#"module tests.od.set_move_only;

import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;

func demo(): void {
    let xs: Array<String> = arrayNew<String>();
    xs.push(stringOf("a"));
    xs.set(0, stringOf("b"));
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st17", source);
}

/// The fixed-array drop glue releases each element in place, one dimension per
/// level, so a nested declaration indexes a row and then an element.
///
/// The per-dimension shape is the point: a flattened index would name a row
/// instead of an element and drop the wrong storage.
#[test]
fn should_emit_fixed_array_drop_glue_one_dimension_per_level() {
    let source = r#"module tests.od.fixed_drop_shape;

import { String, stringOf } from std.text;

func demo(): void {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    let c: String = stringOf("c");
    let d: String = stringOf("d");
    let grid: String[2][2] = [[a, b], [c, d]];
}

func main(): i32 { demo(); return 0; }
"#;
    let (_scratch, c_path) = emit_c("drop-shape-fixed", source);
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    for name in ["grid[0][0]", "grid[0][1]", "grid[1][0]", "grid[1][1]"] {
        let expected = format!("FreeString({name})");
        assert!(
            emitted.contains(&expected),
            "the fixed-array drop must release `{name}` in place; emitted C:\n{emitted}"
        );
    }
}

/// The growable drop glue walks the elements through the buffer, then frees the
/// buffer and empties the array.
#[test]
fn should_emit_growable_array_drop_glue_through_the_buffer() {
    let source = r#"module tests.od.grow_drop_shape;

import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;

func demo(): void {
    let xs: Array<String> = arrayNew<String>();
    xs.push(stringOf("x"));
}

func main(): i32 { demo(); return 0; }
"#;
    let (_scratch, c_path) = emit_c("drop-shape-grow", source);
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        emitted.contains("cl_buffer_ptr(xs.data)"),
        "the growable drop must walk the buffer; emitted C:\n{emitted}"
    );
    assert!(
        emitted.contains("FreeBuffer(xs.data)"),
        "the growable drop must free the buffer; emitted C:\n{emitted}"
    );
}
