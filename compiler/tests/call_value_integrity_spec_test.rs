//! Runnable specification tests for owned conditional stores and deferred
//! take-call results.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The chosen value travels in
//! `main`'s return code as a length or a sum, so a case that prints nothing
//! states its answer as the exit code.
//!
//! The behavior under test is that a move-only value selected by `?:` moves the
//! chosen arm into its destination — a new binding, an existing owned binding,
//! a `take` argument, a return, or a struct-literal field — and that a deferred
//! `take` call result is released exactly once when discarded and transfers
//! ownership when adopted. Each decision runs with both `ok` values as separate
//! cases. A move-only element arm stays the `E10026` limit, while a duplicable
//! element arm copies.
//!
//! Every run case asserts an exit code that proves the chosen value was read
//! correctly, and a double release aborts in the C library, so a clean exit
//! also pins release-once. The rejection case asserts the diagnostic code and
//! that no translation unit is written.

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
            "clc-cvispec-{}-{tag}-{sequence}",
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

/// Asserts the front end rejects the program with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
    assert_rejected_with(tag, source, code, &[]);
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

/// An owned conditional initializing a local takes the `then` arm.
///
/// `ok` is true, so `c` holds `a`'s bytes and the length is two. The un-chosen
/// arm and the destination are each released once; a second release of either
/// aborts in the C library before the exit code is reached.
#[test]
fn should_take_the_then_handle_when_an_owned_conditional_initializes_a_local() {
    let source = r#"module codegen.cvi.st1;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: String = ok ? a : b;
    return i32(len(c));
}
"#;
    let output = run_host("cvi01t", source);
    assert_exit("cvi01t", &output, 2);
}

/// An owned conditional initializing a local takes the `else` arm.
///
/// `ok` is false, so `c` holds `b`'s bytes and the length is three. The
/// un-chosen arm and the destination are each released once; a second release
/// of either aborts in the C library before the exit code is reached.
#[test]
fn should_take_the_else_handle_when_an_owned_conditional_initializes_a_local() {
    let source = r#"module codegen.cvi.st1;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = false;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: String = ok ? a : b;
    return i32(len(c));
}
"#;
    let output = run_host("cvi01f", source);
    assert_exit("cvi01f", &output, 3);
}

/// A mixed conditional with a `take` call in the `then` arm adopts it.
///
/// `ok` is true, so `c` holds `mk()`'s bytes and the length is four. The place
/// arm is not evaluated and stays owned; the transfer temporary releases
/// nothing after the destination adopted it.
#[test]
fn should_adopt_the_call_arm_when_the_conditional_then_is_a_call() {
    let source = r#"module codegen.cvi.st2;

import { String, stringOf } from std.text;

func mk(): String {
    return stringOf("wxyz");
}

func main(): i32 {
    let ok: bool = true;
    let b: String = stringOf("cde");
    let c: String = ok ? mk() : b;
    return i32(len(c));
}
"#;
    let output = run_host("cvi02t", source);
    assert_exit("cvi02t", &output, 4);
}

/// A mixed conditional with a `take` call in the `then` arm takes the
/// place arm when the condition is false.
///
/// `ok` is false, so `c` holds `b`'s bytes and the length is three. The call
/// arm is not evaluated, and the chosen place arm is zeroed so its cleanup
/// releases nothing.
#[test]
fn should_take_the_place_arm_when_the_conditional_then_is_a_call() {
    let source = r#"module codegen.cvi.st2;

import { String, stringOf } from std.text;

func mk(): String {
    return stringOf("wxyz");
}

func main(): i32 {
    let ok: bool = false;
    let b: String = stringOf("cde");
    let c: String = ok ? mk() : b;
    return i32(len(c));
}
"#;
    let output = run_host("cvi02f", source);
    assert_exit("cvi02f", &output, 3);
}

/// A mixed conditional with a `take` call in the `else` arm takes the
/// place arm when the condition is true.
///
/// `ok` is true, so `c` holds `a`'s bytes and the length is two. The call arm
/// is not evaluated, and the chosen place arm is zeroed so its cleanup releases
/// nothing.
#[test]
fn should_take_the_place_arm_when_the_conditional_else_is_a_call() {
    let source = r#"module codegen.cvi.st3;

import { String, stringOf } from std.text;

func mk(): String {
    return stringOf("wxyz");
}

func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let c: String = ok ? a : mk();
    return i32(len(c));
}
"#;
    let output = run_host("cvi03t", source);
    assert_exit("cvi03t", &output, 2);
}

/// A mixed conditional with a `take` call in the `else` arm adopts it.
///
/// `ok` is false, so `c` holds `mk()`'s bytes and the length is four. The place
/// arm is not evaluated and stays owned; the transfer temporary releases
/// nothing after the destination adopted it.
#[test]
fn should_adopt_the_call_arm_when_the_conditional_else_is_a_call() {
    let source = r#"module codegen.cvi.st3;

import { String, stringOf } from std.text;

func mk(): String {
    return stringOf("wxyz");
}

func main(): i32 {
    let ok: bool = false;
    let a: String = stringOf("ab");
    let c: String = ok ? a : mk();
    return i32(len(c));
}
"#;
    let output = run_host("cvi03f", source);
    assert_exit("cvi03f", &output, 4);
}

/// A conditional in a `take` array argument passes the `then` elements.
///
/// The callee sums the element lengths of the chosen `String[2]`, which is
/// three for the first array. Both arms count as maybe-moved after the call and
/// are not read again.
#[test]
fn should_sum_the_then_elements_when_a_conditional_is_a_take_array_argument() {
    let source = r#"module codegen.cvi.st4;

import { String, stringOf } from std.text;

func eat(take arr: String[2]): i32 {
    return i32(len(arr[0])) + i32(len(arr[1]));
}

func main(): i32 {
    let ok: bool = true;
    let a: String[2] = [stringOf("a"), stringOf("bb")];
    let b: String[2] = [stringOf("ccc"), stringOf("dddd")];
    return eat(ok ? a : b);
}
"#;
    let output = run_host("cvi04t", source);
    assert_exit("cvi04t", &output, 3);
}

/// A conditional in a `take` array argument passes the `else` elements.
///
/// The callee sums the element lengths of the chosen `String[2]`, which is
/// seven for the second array. Both arms count as maybe-moved after the call
/// and are not read again.
#[test]
fn should_sum_the_else_elements_when_a_conditional_is_a_take_array_argument() {
    let source = r#"module codegen.cvi.st4;

import { String, stringOf } from std.text;

func eat(take arr: String[2]): i32 {
    return i32(len(arr[0])) + i32(len(arr[1]));
}

func main(): i32 {
    let ok: bool = false;
    let a: String[2] = [stringOf("a"), stringOf("bb")];
    let b: String[2] = [stringOf("ccc"), stringOf("dddd")];
    return eat(ok ? a : b);
}
"#;
    let output = run_host("cvi04f", source);
    assert_exit("cvi04f", &output, 7);
}

/// A conditional whose move-only arms are array elements is rejected.
///
/// An element cannot move out of an array in v1, so binding the chosen
/// `String` element reports the element-move limit E10026 and writes no C.
#[test]
fn should_reject_a_conditional_handle_when_its_arms_are_move_only_elements() {
    let source = r#"module codegen.cvi.st5;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = false;
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let c: String = ok ? a[0] : a[1];
    return i32(len(c));
}
"#;
    assert_rejected("cvi05", source, "E10026");
}

/// A conditional over duplicable elements copies the `then` element.
///
/// `i32` elements copy, so `x` is the first element and the exit code is one.
/// Nothing moves out of the array.
#[test]
fn should_copy_the_then_element_when_a_conditional_selects_a_duplicable_element() {
    let source = r#"module codegen.cvi.st6;

func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let x: i32 = ok ? a[0] : a[1];
    return x;
}
"#;
    let output = run_host("cvi06t", source);
    assert_exit("cvi06t", &output, 1);
}

/// A conditional over duplicable elements copies the `else` element.
///
/// `i32` elements copy, so `x` is the second element and the exit code is two.
/// Nothing moves out of the array.
#[test]
fn should_copy_the_else_element_when_a_conditional_selects_a_duplicable_element() {
    let source = r#"module codegen.cvi.st6;

func main(): i32 {
    let ok: bool = false;
    let a: i32[2] = [1, 2];
    let x: i32 = ok ? a[0] : a[1];
    return x;
}
"#;
    let output = run_host("cvi06f", source);
    assert_exit("cvi06f", &output, 2);
}

/// A discarded take-call conditional leaves the place arm owned.
///
/// `ok` is false, so the discarded value is the place arm `t`; the statement
/// only reads it, so `t` stays owned and its length is three afterwards.
#[test]
fn should_keep_the_place_arm_readable_when_a_take_call_conditional_is_discarded() {
    let source = r#"module codegen.cvi.st7;

import { String, stringOf } from std.text;

func sink(take s: String): String {
    return s;
}

func main(): i32 {
    let ok: bool = false;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    ok ? sink(s) : t;
    return i32(len(t));
}
"#;
    let output = run_host("cvi07f", source);
    assert_exit("cvi07f", &output, 3);
}

/// A discarded take-call conditional runs the call arm once.
///
/// `ok` is true, so `s` moves into `sink` and the discarded result is released
/// exactly once at statement end. `t` is never evaluated, stays owned, and its
/// length is three.
#[test]
fn should_keep_the_unevaluated_arm_readable_when_a_take_call_conditional_is_discarded() {
    let source = r#"module codegen.cvi.st7;

import { String, stringOf } from std.text;

func sink(take s: String): String {
    return s;
}

func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    ok ? sink(s) : t;
    return i32(len(t));
}
"#;
    let output = run_host("cvi07t", source);
    assert_exit("cvi07t", &output, 3);
}

/// A `let` adopts the take-call result when the condition is true.
///
/// `sink` returns its argument, so `r` holds `s`'s bytes and the length is two.
/// The transfer temporary is zeroed by the adoption, so the statement-end
/// release is a no-op and `s` is never released twice.
#[test]
fn should_adopt_the_call_result_when_a_take_call_conditional_initializes_a_local() {
    let source = r#"module codegen.cvi.st8;

import { String, stringOf } from std.text;

func sink(take s: String): String {
    return s;
}

func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    let r: String = ok ? sink(s) : t;
    return i32(len(r));
}
"#;
    let output = run_host("cvi08t", source);
    assert_exit("cvi08t", &output, 2);
}

/// A `let` takes the place arm when the condition is false.
///
/// `ok` is false, so `r` holds `t`'s bytes and the length is three. The call
/// arm is not evaluated, `s` stays owned, and no value is released twice.
#[test]
fn should_take_the_place_arm_when_a_take_call_conditional_initializes_a_local() {
    let source = r#"module codegen.cvi.st8;

import { String, stringOf } from std.text;

func sink(take s: String): String {
    return s;
}

func main(): i32 {
    let ok: bool = false;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    let r: String = ok ? sink(s) : t;
    return i32(len(r));
}
"#;
    let output = run_host("cvi08f", source);
    assert_exit("cvi08f", &output, 3);
}

/// A conditional assignment takes the `then` arm and releases the old
/// value once.
///
/// `s` first owns a four-byte string; with `ok` true it takes `a`'s bytes, so
/// the length is two. The old value is released exactly once, and the un-chosen
/// arm releases nothing.
#[test]
fn should_release_the_old_handle_once_when_a_conditional_assignment_takes_the_then_arm() {
    let source = r#"module codegen.cvi.st32;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("zzzz");
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    s = ok ? a : b;
    return i32(len(s));
}
"#;
    let output = run_host("cvi32t", source);
    assert_exit("cvi32t", &output, 2);
}

/// A conditional assignment takes the `else` arm and releases the old
/// value once.
///
/// `s` first owns a four-byte string; with `ok` false it takes `b`'s bytes, so
/// the length is three. The old value is released exactly once, and the
/// un-chosen arm releases nothing.
#[test]
fn should_release_the_old_handle_once_when_a_conditional_assignment_takes_the_else_arm() {
    let source = r#"module codegen.cvi.st32;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = false;
    let s: String = stringOf("zzzz");
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    s = ok ? a : b;
    return i32(len(s));
}
"#;
    let output = run_host("cvi32f", source);
    assert_exit("cvi32f", &output, 3);
}

/// A conditional assignment whose `then` arm names the target keeps the
/// value.
///
/// With `ok` true, `s = ok ? s : b;` restores the saved old value, so `s` is
/// unchanged and its length is four. The saved value is released as a no-op,
/// so nothing is released twice.
#[test]
fn should_keep_the_saved_value_when_a_conditional_assignment_names_the_target() {
    let source = r#"module codegen.cvi.st33;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("keep");
    let b: String = stringOf("cde");
    s = ok ? s : b;
    return i32(len(s));
}
"#;
    let output = run_host("cvi33t", source);
    assert_exit("cvi33t", &output, 4);
}

/// A conditional assignment whose `then` arm names the target takes the
/// other arm when the condition is false.
///
/// With `ok` false, `s` takes `b`'s bytes and the length is three. The old
/// value is released exactly once through the save, so nothing is released
/// twice.
#[test]
fn should_replace_the_value_when_a_conditional_assignment_names_the_target() {
    let source = r#"module codegen.cvi.st33;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = false;
    let s: String = stringOf("keep");
    let b: String = stringOf("cde");
    s = ok ? s : b;
    return i32(len(s));
}
"#;
    let output = run_host("cvi33f", source);
    assert_exit("cvi33f", &output, 3);
}

/// A conditional return selects the take-call value when true.
///
/// `pick` returns `sink(s)` under `ok` true, so the caller's result holds
/// `s`'s bytes and the length is two. The returned storage is zeroed after the
/// store, so the cleanup ladder releases nothing and the caller's read is not a
/// use-after-free.
#[test]
fn should_return_the_call_value_when_a_conditional_return_selects_it() {
    let source = r#"module codegen.cvi.st34;

import { String, stringOf } from std.text;

func sink(take s: String): String {
    return s;
}

func pick(ok: bool, take s: String, take t: String): String {
    return ok ? sink(s) : t;
}

func main(): i32 {
    let r: String = pick(true, stringOf("ab"), stringOf("cde"));
    return i32(len(r));
}
"#;
    let output = run_host("cvi34t", source);
    assert_exit("cvi34t", &output, 2);
}

/// A conditional return selects the place value when false.
///
/// `pick` returns `t` under `ok` false, so the caller's result holds `t`'s
/// bytes and the length is three. The returned storage is zeroed after the
/// store, so the cleanup ladder releases nothing and the caller's read is not a
/// use-after-free.
#[test]
fn should_return_the_place_value_when_a_conditional_return_selects_it() {
    let source = r#"module codegen.cvi.st34;

import { String, stringOf } from std.text;

func sink(take s: String): String {
    return s;
}

func pick(ok: bool, take s: String, take t: String): String {
    return ok ? sink(s) : t;
}

func main(): i32 {
    let r: String = pick(false, stringOf("ab"), stringOf("cde"));
    return i32(len(r));
}
"#;
    let output = run_host("cvi34f", source);
    assert_exit("cvi34f", &output, 3);
}

/// A `take` argument that is a conditional gives the callee the `then`
/// handle.
///
/// `eat` returns the chosen handle's length; `ok` true selects `a`, so the exit
/// code is two. The caller releases nothing it transferred, and the callee's
/// parameter cleanup releases the value once.
#[test]
fn should_read_the_then_handle_when_a_conditional_is_a_take_argument() {
    let source = r#"module codegen.cvi.st35;

import { String, stringOf } from std.text;

func eat(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    return eat(ok ? a : b);
}
"#;
    let output = run_host("cvi35t", source);
    assert_exit("cvi35t", &output, 2);
}

/// A `take` argument that is a conditional gives the callee the `else`
/// handle.
///
/// `eat` returns the chosen handle's length; `ok` false selects `b`, so the
/// exit code is three. The caller releases nothing it transferred, and the
/// callee's parameter cleanup releases the value once.
#[test]
fn should_read_the_else_handle_when_a_conditional_is_a_take_argument() {
    let source = r#"module codegen.cvi.st35;

import { String, stringOf } from std.text;

func eat(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let ok: bool = false;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    return eat(ok ? a : b);
}
"#;
    let output = run_host("cvi35f", source);
    assert_exit("cvi35f", &output, 3);
}

/// An owned conditional initializing an aggregate local takes the `then`
/// value.
///
/// `Box` owns a `String`; with `ok` true, `c` holds `a`'s name and tag, so the
/// exit code is two plus one. No field is released twice.
#[test]
fn should_take_the_then_aggregate_when_an_owned_conditional_initializes_a_struct_local() {
    let source = r#"module codegen.cvi.st36;

import { String, stringOf } from std.text;

struct Box {
    name: String;
    tag: i32;
}

func main(): i32 {
    let ok: bool = true;
    let a: Box = Box { name: stringOf("ab"), tag: 1 };
    let b: Box = Box { name: stringOf("cde"), tag: 1 };
    let c: Box = ok ? a : b;
    return i32(len(c.name)) + c.tag;
}
"#;
    let output = run_host("cvi36t", source);
    assert_exit("cvi36t", &output, 3);
}

/// An owned conditional initializing an aggregate local takes the `else`
/// value.
///
/// `Box` owns a `String`; with `ok` false, `c` holds `b`'s name and tag, so the
/// exit code is three plus one. No field is released twice.
#[test]
fn should_take_the_else_aggregate_when_an_owned_conditional_initializes_a_struct_local() {
    let source = r#"module codegen.cvi.st36;

import { String, stringOf } from std.text;

struct Box {
    name: String;
    tag: i32;
}

func main(): i32 {
    let ok: bool = false;
    let a: Box = Box { name: stringOf("ab"), tag: 1 };
    let b: Box = Box { name: stringOf("cde"), tag: 1 };
    let c: Box = ok ? a : b;
    return i32(len(c.name)) + c.tag;
}
"#;
    let output = run_host("cvi36f", source);
    assert_exit("cvi36f", &output, 4);
}

/// A struct-literal field takes the `then` arm of an owned conditional.
///
/// `name: ok ? a : b` stores `a`'s bytes with `ok` true; the field is read back
/// through `c`, so the exit code is two plus one and no `String` is released
/// twice.
#[test]
fn should_take_the_then_handle_when_an_owned_conditional_fills_a_struct_literal_field() {
    let source = r#"module codegen.cvi.st37;

import { String, stringOf } from std.text;

struct Box {
    name: String;
    tag: i32;
}

func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: Box = Box { name: ok ? a : b, tag: 1 };
    return i32(len(c.name)) + c.tag;
}
"#;
    let output = run_host("cvi37t", source);
    assert_exit("cvi37t", &output, 3);
}

/// A struct-literal field takes the `else` arm of an owned conditional.
///
/// `name: ok ? a : b` stores `b`'s bytes with `ok` false; the field is read back
/// through `c`, so the exit code is three plus one and no `String` is released
/// twice.
#[test]
fn should_take_the_else_handle_when_an_owned_conditional_fills_a_struct_literal_field() {
    let source = r#"module codegen.cvi.st37;

import { String, stringOf } from std.text;

struct Box {
    name: String;
    tag: i32;
}

func main(): i32 {
    let ok: bool = false;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: Box = Box { name: ok ? a : b, tag: 1 };
    return i32(len(c.name)) + c.tag;
}
"#;
    let output = run_host("cvi37f", source);
    assert_exit("cvi37f", &output, 4);
}

/// A struct-literal field adopts the take-call result when true.
///
/// `name: ok ? sink(s) : t` stores `sink`'s result with `ok` true, which is
/// `s`'s bytes; the field reads back as length two. The field store zeroes the
/// temporary, so nothing is released twice or read after release.
#[test]
fn should_adopt_the_call_value_when_an_owned_conditional_fills_a_struct_literal_field() {
    let source = r#"module codegen.cvi.st37;

import { String, stringOf } from std.text;

struct Box {
    name: String;
    tag: i32;
}

func sink(take s: String): String {
    return s;
}

func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    let c: Box = Box { name: ok ? sink(s) : t, tag: 1 };
    return i32(len(c.name)) + c.tag;
}
"#;
    let output = run_host("cvi37ct", source);
    assert_exit("cvi37ct", &output, 3);
}

/// A struct-literal field takes the place arm of a take-call conditional
/// when false.
///
/// `name: ok ? sink(s) : t` stores `t`'s bytes with `ok` false; the field reads
/// back as length three. The call arm is not evaluated, and nothing is released
/// twice or read after release.
#[test]
fn should_take_the_place_value_when_an_owned_conditional_fills_a_struct_literal_field() {
    let source = r#"module codegen.cvi.st37;

import { String, stringOf } from std.text;

struct Box {
    name: String;
    tag: i32;
}

func sink(take s: String): String {
    return s;
}

func main(): i32 {
    let ok: bool = false;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    let c: Box = Box { name: ok ? sink(s) : t, tag: 1 };
    return i32(len(c.name)) + c.tag;
}
"#;
    let output = run_host("cvi37cf", source);
    assert_exit("cvi37cf", &output, 4);
}
