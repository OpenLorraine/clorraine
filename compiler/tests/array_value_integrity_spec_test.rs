//! Runnable specification tests for fixed-array values and length.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The value under test
//! travels in `main`'s return code, so a case that prints nothing states its
//! answer as the exit code.
//!
//! The behavior under test is that a fixed array is a first-class value: a
//! duplicable array copies, a move-only array moves so the source empties, a
//! whole-array reassignment releases the old elements exactly once, and a
//! self-referencing call keeps the old value valid until the call returns.
//! `len` reports the outermost element count for a multi-extent array and the
//! byte count for an owned `String`.
//!
//! Cases whose answer is a compile-time rejection assert the diagnostic code
//! and, where the message names a type or binding, the message text too. Some
//! of those rejections pass before the implementation exists; they are the
//! regression guards for behavior that must not change.
//!
//! Later phases extend this suite with the element-loop and projection cases;
//! each phase's batch is fully green before its full workspace check runs.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one program may run before the case fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

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
            "clc-avspec-{}-{tag}-{sequence}",
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

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit. A rejection is a failure of the case,
/// not a result to assert, because the run cases are about runtime behavior.
fn emit_host(tag: &str, scratch: &ScratchDir, source: &str) -> String {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// Compiles the emitted C with the host compiler under the strict flag set.
///
/// Panics with the compiler's stderr when the build fails. Returns the built
/// program path.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Runs a built program with a time limit and returns its output.
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_with_timeout(tag: &str, program: &Path) -> Output {
    let mut child = Command::new(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{tag}: the program did not finish within {RUN_TIMEOUT:?}");
            }
            Err(err) => panic!("{tag}: cannot wait for the program: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program output: {err}"))
}

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, &scratch, source);
    let program = build_host_c(tag, &scratch, &emitted);
    run_with_timeout(tag, &program)
}

/// Asserts the program exited with exactly `expected`, reporting its stderr.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit code {expected}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Compiles `source` and returns the compiler output plus the C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// Asserts the front end rejects the program with `code`, that the message
/// contains every fragment, and that no C is written.
fn assert_rejected_with(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// A multi-extent array reports its outermost element count to `len`.
///
/// The array holds two rows of three, so the length is two, not six. The
/// element read keeps the array a used value in the emitted C; the exit code
/// is the length alone.
#[test]
fn should_report_the_row_count_when_len_reads_a_multi_extent_array() {
    let source = r#"module codegen.av.st1;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    if (m[0][0] != 1) {
        return 100;
    }
    return i32(len(m));
}
"#;
    let output = run_host("av1", source);
    assert_exit("av1", &output, 2);
}

/// Binding a duplicable array to a second local copies it.
///
/// The copy sums its two elements; the source is read afterwards, so a copy
/// that consumed the source fails the guard instead of reporting three.
#[test]
fn should_copy_a_duplicable_array_when_bound_to_a_second_local() {
    let source = r#"module codegen.av.st2;

func main(): i32 {
    let a: i32[2] = [1, 2];
    let b: i32[2] = a;
    if (a[0] + a[1] != 3) {
        return 100;
    }
    return b[0] + b[1];
}
"#;
    let output = run_host("av2", source);
    assert_exit("av2", &output, 3);
}

/// A fixed-array field of a struct copies when bound to a local.
///
/// The struct owns the array; the local receives a byte copy, and the sum of
/// its elements is three.
#[test]
fn should_copy_a_struct_array_field_when_bound_to_a_local() {
    let source = r#"module codegen.av.st3;

struct S {
    v: i32[2];
}

func main(): i32 {
    let s: S = S { v: [1, 2] };
    let c: i32[2] = s.v;
    return c[0] + c[1];
}
"#;
    let output = run_host("av3", source);
    assert_exit("av3", &output, 3);
}

/// A selected row copies when bound to a row-typed local.
///
/// The first row of the two-by-three array is `[1, 2, 3]`; its sum is six.
#[test]
fn should_copy_a_row_when_bound_to_a_row_local() {
    let source = r#"module codegen.av.st4;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let r: i32[3] = m[0];
    return r[0] + r[1] + r[2];
}
"#;
    let output = run_host("av4", source);
    assert_exit("av4", &output, 6);
}

/// Moving a `String` array into a second local empties the source without
/// double-freeing its elements.
///
/// The second element of the destination is "cde", so the length is three and
/// a clean exit proves the two handles were released once each.
#[test]
fn should_move_a_string_array_when_bound_to_a_second_local() {
    let source = r#"module codegen.av.st5;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = a;
    return i32(len(b[1]));
}
"#;
    let output = run_host("av5", source);
    assert_exit("av5", &output, 3);
}

/// A second use of a moved array reports the moved-binding error and names it.
#[test]
fn should_reject_a_second_use_of_a_moved_array() {
    let source = r#"module codegen.av.st6;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = a;
    let c: String[2] = a;
    return 0;
}
"#;
    assert_rejected_with("av6", source, "E10020", &["'a' was moved and cannot be used"]);
}

/// Reassigning a `String` array releases the old elements before the new value
/// arrives.
///
/// The destination ends up holding the source array, so the length of its
/// second element is three; a clean exit proves the old elements were released
/// exactly once.
#[test]
fn should_release_the_old_elements_when_an_array_is_reassigned() {
    let source = r#"module codegen.av.st7;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = [stringOf("pq"), stringOf("rs")];
    b = a;
    return i32(len(b[1]));
}
"#;
    let output = run_host("av7", source);
    assert_exit("av7", &output, 3);
}

/// Assigning an array to itself keeps its elements intact.
///
/// The assignment saves the old value first, stores the new one, and releases
/// the saved value; the array still sums to three.
#[test]
fn should_keep_an_array_intact_when_it_is_assigned_to_itself() {
    let source = r#"module codegen.av.st8;

func main(): i32 {
    let a: i32[2] = [1, 2];
    a = a;
    return a[0] + a[1];
}
"#;
    let output = run_host("av8", source);
    assert_exit("av8", &output, 3);
}

/// An array returned by a call initializes a local by move.
///
/// The callee fills the hidden out-parameter and the caller binds it; the sum
/// of the two elements is three. This case passes before the copy path exists,
/// because the call shape already routes through the out-parameter.
#[test]
fn should_bind_a_call_result_when_it_initializes_an_array_local() {
    let source = r#"module codegen.av.st9;

func mk(): i32[2] {
    return [1, 2];
}

func main(): i32 {
    let b: i32[2] = mk();
    return b[0] + b[1];
}
"#;
    let output = run_host("av9", source);
    assert_exit("av9", &output, 3);
}

/// An owned `String` reports its byte count to `len`, in both spellings.
///
/// The literal and the binding hold the same five bytes, so the two lengths
/// sum to ten.
#[test]
fn should_report_the_byte_count_when_len_reads_a_string() {
    let source = r#"module codegen.av.st22;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("hello");
    return i32(len(stringOf("hello"))) + i32(s.len());
}
"#;
    let output = run_host("av22", source);
    assert_exit("av22", &output, 10);
}

/// A call on the right of an array reassignment replaces the old elements and
/// drops them once.
///
/// The destination ends up holding the call's result, so the length of its
/// second element is two; a clean exit proves the old elements were released
/// exactly once.
#[test]
fn should_read_the_new_elements_when_a_call_reassigns_an_array() {
    let source = r#"module codegen.av.st25;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("pq"), stringOf("rs")];
}

func main(): i32 {
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    s = mk();
    return i32(len(s[1]));
}
"#;
    let output = run_host("av25", source);
    assert_exit("av25", &output, 2);
}

/// A call that reads the target keeps the old value valid during the call.
///
/// The callee builds its result from the old elements, swapping them; the
/// destination then holds "cde" first, so its first length is three and a
/// clean exit proves the old value was released after the call.
#[test]
fn should_read_the_copied_elements_when_a_call_reassigns_an_array_from_itself() {
    let source = r#"module codegen.av.st26;

import { String, stringOf } from std.text;

func dup(borrow source: String[2]): String[2] {
    return [stringOf(source[1]), stringOf(source[0])];
}

func main(): i32 {
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    s = dup(borrow s);
    return i32(len(s[0]));
}
"#;
    let output = run_host("av26", source);
    assert_exit("av26", &output, 3);
}
