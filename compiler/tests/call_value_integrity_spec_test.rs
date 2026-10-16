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
//! The fixed-array batch stores a row through a constant or runtime index,
//! passes a conditional array argument whose arms are calls, literals, or
//! places, projects an array literal in value and argument positions, and
//! initializes a struct-literal array field from a conditional. The conditional
//! store also reads the un-chosen arm afterwards, so its exit code proves the
//! un-chosen arm kept its own storage.
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

/// A runtime row index stores a fixed-array value in the selected row.
///
/// The rows start as `[[1, 2], [3, 4]]` and the stored row is `[5, 6]`. The
/// index comes from a call, so the store must land in row one; reading row one
/// back gives `5 * 10 + 6 = 56`, while a store that landed in row zero would
/// leave `34`.
#[test]
fn should_store_the_value_in_the_second_row_when_a_runtime_index_selects_it() {
    let source = r#"module codegen.cvi.st9;

func runtimeIndex(): i32 {
    return 1;
}

func main(): i32 {
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let value: i32[2] = [5, 6];
    let i: i32 = runtimeIndex();
    rows[i] = value;
    return rows[1][0] * 10 + rows[1][1];
}
"#;
    let output = run_host("cvi09", source);
    assert_exit("cvi09", &output, 56);
}

/// A constant row index stores a fixed-array value in the first row.
///
/// The rows start as `[[1, 2], [3, 4]]` and the stored row is `[5, 6]`.
/// Reading row zero back gives `5 * 10 + 6 = 56`, while a store that landed in
/// row one would leave `12`.
#[test]
fn should_store_the_value_in_the_first_row_when_a_constant_index_selects_it() {
    let source = r#"module codegen.cvi.st10;

func main(): i32 {
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let value: i32[2] = [5, 6];
    rows[0] = value;
    return rows[0][0] * 10 + rows[0][1];
}
"#;
    let output = run_host("cvi10", source);
    assert_exit("cvi10", &output, 56);
}

/// A runtime row index stores an array literal built from variables.
///
/// The rows start as `[[1, 2], [3, 4]]`; the literal `[x, y]` holds `7` and
/// `8`, so row one reads back as `7 * 10 + 8 = 78`. A store that landed in row
/// zero would leave `12`.
#[test]
fn should_store_the_row_when_a_runtime_index_takes_an_array_literal() {
    let source = r#"module codegen.cvi.st11;

func runtimeIndex(): i32 {
    return 1;
}

func main(): i32 {
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let x: i32 = 7;
    let y: i32 = 8;
    let i: i32 = runtimeIndex();
    rows[i] = [x, y];
    return rows[1][0] * 10 + rows[1][1];
}
"#;
    let output = run_host("cvi11", source);
    assert_exit("cvi11", &output, 78);
}

/// A conditional row store takes the `then` arm and leaves the un-chosen arm
/// intact.
///
/// The rows start as `[[1, 2], [3, 4]]`, `a` is `[5, 6]`, and `b` is `[7, 8]`.
/// With `ok` true row one becomes `[5, 6]`, so the store reads back as
/// `5 * 10 = 50`; the un-chosen `b` still reads `7`, giving `57`. A copy that
/// aliased the arms or emptied the un-chosen one would change the answer.
#[test]
fn should_take_the_then_row_when_a_conditional_selects_a_row_arm() {
    let source = r#"module codegen.cvi.st12;

func runtimeIndex(): i32 {
    return 1;
}

func main(): i32 {
    let ok: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let a: i32[2] = [5, 6];
    let b: i32[2] = [7, 8];
    let i: i32 = runtimeIndex();
    rows[i] = ok ? a : b;
    return rows[1][0] * 10 + b[0];
}
"#;
    let output = run_host("cvi12t", source);
    assert_exit("cvi12t", &output, 57);
}

/// A conditional row store takes the `else` arm and leaves the un-chosen arm
/// intact.
///
/// The rows start as `[[1, 2], [3, 4]]`, `a` is `[5, 6]`, and `b` is `[7, 8]`.
/// With `ok` false row one becomes `[7, 8]`, so the store reads back as
/// `7 * 10 = 70`; the un-chosen `a` still reads `5`, giving `75`. A copy that
/// aliased the arms or emptied the un-chosen one would change the answer.
#[test]
fn should_take_the_else_row_when_a_conditional_selects_a_row_arm() {
    let source = r#"module codegen.cvi.st12;

func runtimeIndex(): i32 {
    return 1;
}

func main(): i32 {
    let ok: bool = false;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let a: i32[2] = [5, 6];
    let b: i32[2] = [7, 8];
    let i: i32 = runtimeIndex();
    rows[i] = ok ? a : b;
    return rows[1][0] * 10 + a[0];
}
"#;
    let output = run_host("cvi12f", source);
    assert_exit("cvi12f", &output, 75);
}

/// A conditional array argument whose `then` arm is a call passes the chosen
/// call's values.
///
/// `mkA` returns `[1, 2]` and `mkB` returns `[3, 4]`; with `ok` true the callee
/// sums the `mkA` result, so the exit code is `3`.
#[test]
fn should_sum_the_then_call_when_a_conditional_is_an_array_argument() {
    let source = r#"module codegen.cvi.st13;

func mkA(): i32[2] {
    return [1, 2];
}

func mkB(): i32[2] {
    return [3, 4];
}

func eat(a: i32[2]): i32 {
    return a[0] + a[1];
}

func main(): i32 {
    let ok: bool = true;
    return eat(ok ? mkA() : mkB());
}
"#;
    let output = run_host("cvi13t", source);
    assert_exit("cvi13t", &output, 3);
}

/// A conditional array argument whose `else` arm is a call passes the chosen
/// call's values.
///
/// `mkA` returns `[1, 2]` and `mkB` returns `[3, 4]`; with `ok` false the
/// callee sums the `mkB` result, so the exit code is `7`.
#[test]
fn should_sum_the_else_call_when_a_conditional_is_an_array_argument() {
    let source = r#"module codegen.cvi.st13;

func mkA(): i32[2] {
    return [1, 2];
}

func mkB(): i32[2] {
    return [3, 4];
}

func eat(a: i32[2]): i32 {
    return a[0] + a[1];
}

func main(): i32 {
    let ok: bool = false;
    return eat(ok ? mkA() : mkB());
}
"#;
    let output = run_host("cvi13f", source);
    assert_exit("cvi13f", &output, 7);
}

/// A conditional array argument whose `then` arm is a literal passes the
/// chosen literal's values.
///
/// The arms are `[10, 20]` and `[30, 40]`; with `ok` true the callee sums the
/// first, so the exit code is `30`.
#[test]
fn should_sum_the_then_literal_when_a_conditional_is_an_array_argument() {
    let source = r#"module codegen.cvi.st14;

func eat(a: i32[2]): i32 {
    return a[0] + a[1];
}

func main(): i32 {
    let ok: bool = true;
    return eat(ok ? [10, 20] : [30, 40]);
}
"#;
    let output = run_host("cvi14t", source);
    assert_exit("cvi14t", &output, 30);
}

/// A conditional array argument whose `else` arm is a literal passes the
/// chosen literal's values.
///
/// The arms are `[10, 20]` and `[30, 40]`; with `ok` false the callee sums the
/// second, so the exit code is `70`.
#[test]
fn should_sum_the_else_literal_when_a_conditional_is_an_array_argument() {
    let source = r#"module codegen.cvi.st14;

func eat(a: i32[2]): i32 {
    return a[0] + a[1];
}

func main(): i32 {
    let ok: bool = false;
    return eat(ok ? [10, 20] : [30, 40]);
}
"#;
    let output = run_host("cvi14f", source);
    assert_exit("cvi14f", &output, 70);
}

/// An array call result used as an argument gives the callee its values.
///
/// `mkA` returns `[1, 2]`; the callee receives a place for the result and sums
/// it, so the exit code is `3`.
#[test]
fn should_sum_the_call_result_when_an_array_call_is_an_argument() {
    let source = r#"module codegen.cvi.st15;

func mkA(): i32[2] {
    return [1, 2];
}

func sink(a: i32[2]): i32 {
    return a[0] + a[1];
}

func main(): i32 {
    return sink(mkA());
}
"#;
    let output = run_host("cvi15", source);
    assert_exit("cvi15", &output, 3);
}

/// An array literal can be projected in value position and as a call argument.
///
/// `[a, b]` is `[2, 3]`; the value position reads the first element and the
/// call argument passes the second through `pick`. The answer combines both
/// reads as `2 * 10 + 3 = 23`, and the emitted C must build without a
/// diagnostic.
#[test]
fn should_read_the_element_when_an_array_literal_is_projected() {
    let source = r#"module codegen.cvi.st16;

func pick(x: i32): i32 {
    return x;
}

func main(): i32 {
    let a: i32 = 2;
    let b: i32 = 3;
    let first: i32 = [a, b][0];
    return first * 10 + pick([a, b][1]);
}
"#;
    let output = run_host("cvi16", source);
    assert_exit("cvi16", &output, 23);
}

/// A struct-literal array field takes the `then` arm of a conditional.
///
/// `arr: ok ? a : b` stores `[1, 2]` with `ok` true and `tag` is `1`; the field
/// reads back as `1 * 10 + 2 + 1 = 13`, so both the array field and the scalar
/// field are initialized.
#[test]
fn should_take_the_then_row_when_a_conditional_fills_a_struct_literal_array_field() {
    let source = r#"module codegen.cvi.st17;

struct Box {
    arr: i32[2];
    tag: i32;
}

func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let box: Box = Box { arr: ok ? a : b, tag: 1 };
    return box.arr[0] * 10 + box.arr[1] + box.tag;
}
"#;
    let output = run_host("cvi17t", source);
    assert_exit("cvi17t", &output, 13);
}

/// A struct-literal array field takes the `else` arm of a conditional.
///
/// `arr: ok ? a : b` stores `[3, 4]` with `ok` false and `tag` is `1`; the
/// field reads back as `3 * 10 + 4 + 1 = 35`, so both the array field and the
/// scalar field are initialized.
#[test]
fn should_take_the_else_row_when_a_conditional_fills_a_struct_literal_array_field() {
    let source = r#"module codegen.cvi.st17;

struct Box {
    arr: i32[2];
    tag: i32;
}

func main(): i32 {
    let ok: bool = false;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let box: Box = Box { arr: ok ? a : b, tag: 1 };
    return box.arr[0] * 10 + box.arr[1] + box.tag;
}
"#;
    let output = run_host("cvi17f", source);
    assert_exit("cvi17f", &output, 35);
}

/// A struct literal in argument position carries the `then` array into the
/// callee.
///
/// The literal's `arr` is `[1, 2]` with `ok` true and `tag` is `1`; the callee
/// reads `1 * 10 + 2 + 1 = 13`, and the emitted C must build and run.
#[test]
fn should_read_the_then_struct_when_a_struct_literal_is_an_argument() {
    let source = r#"module codegen.cvi.st41;

struct Box {
    arr: i32[2];
    tag: i32;
}

func eat(box: Box): i32 {
    return box.arr[0] * 10 + box.arr[1] + box.tag;
}

func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return eat(Box { arr: ok ? a : b, tag: 1 });
}
"#;
    let output = run_host("cvi41t", source);
    assert_exit("cvi41t", &output, 13);
}

/// A struct literal in argument position carries the `else` array into the
/// callee.
///
/// The literal's `arr` is `[3, 4]` with `ok` false and `tag` is `1`; the callee
/// reads `3 * 10 + 4 + 1 = 35`, and the emitted C must build and run.
#[test]
fn should_read_the_else_struct_when_a_struct_literal_is_an_argument() {
    let source = r#"module codegen.cvi.st41;

struct Box {
    arr: i32[2];
    tag: i32;
}

func eat(box: Box): i32 {
    return box.arr[0] * 10 + box.arr[1] + box.tag;
}

func main(): i32 {
    let ok: bool = false;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return eat(Box { arr: ok ? a : b, tag: 1 });
}
"#;
    let output = run_host("cvi41f", source);
    assert_exit("cvi41f", &output, 35);
}

/// A func-typed local holding an array-returning capture-less lambda reads the
/// returned element.
///
/// `g` is a plain function pointer whose signature returns `i32[2]`; with the
/// argument `5` the array is `[5, 6]`, so the exit code is `6`. The emitted C
/// must build, which pins the hidden destination on the lifted lambda, on the
/// func value's declarator, and on the call through the value.
#[test]
fn should_read_the_second_element_when_a_func_typed_local_returns_a_fixed_array() {
    let source = r#"module codegen.cvi.st18;

func main(): i32 {
    let g: func(i32): i32[2] = (x: i32) => { return [x, x + 1]; };
    return g(5)[1];
}
"#;
    let output = run_host("cvi18", source);
    assert_exit("cvi18", &output, 6);
}

/// An array-returning capturing closure fills a generic callable bound and its
/// result reaches the caller.
///
/// The closure captures `base = 10`, so it is a closure struct whose `fn` field
/// carries the signature. The bound `F: func(i32): i32[2]` requires a
/// fixed-array result, so the lifted function takes the environment parameter
/// first and the hidden destination last. With `5` the array is `[5, 15]`, so
/// the exit code is `15`.
#[test]
fn should_read_the_captured_element_when_an_array_lambda_fills_a_callable_bound() {
    let source = r#"module codegen.cvi.st19a;

func apply<F: func(i32): i32[2]>(x: i32, f: F): i32 {
    let r: i32[2] = f(x);
    return r[1];
}

func main(): i32 {
    let base: i32 = 10;
    return apply(5, [base] (x: i32) => { return [x, x + base]; });
}
"#;
    let output = run_host("cvi19a", source);
    assert_exit("cvi19a", &output, 15);
}

/// A capture-less array-returning lambda stored in a plain `func`-typed struct
/// field reads the returned element.
///
/// `Ops.run` has type `func(i32): i32[2]`; with `5` the array is `[5, 6]`, so
/// the exit code is `6`. The field declarator and the call through the field
/// must both carry the hidden destination.
#[test]
fn should_read_the_element_when_an_array_lambda_fills_a_func_typed_field() {
    let source = r#"module codegen.cvi.st19b;

struct Ops {
    run: func(i32): i32[2];
}

func main(): i32 {
    let ops: Ops = Ops { run: (x: i32) => { return [x, x + 1]; } };
    return ops.run(5)[1];
}
"#;
    let output = run_host("cvi19b", source);
    assert_exit("cvi19b", &output, 6);
}

/// A function that returns a function value can be called twice.
///
/// `make` returns the bare function `double`, so `make()(4)` is `8`. The
/// emitted declarator must nest the pointer for the func-typed return
/// (`i32 (*cl_make(void))(i32)`), or the C compiler rejects the translation
/// unit.
#[test]
fn should_call_the_returned_function_when_a_function_returns_a_func_value() {
    let source = r#"module codegen.cvi.st20;

func double(x: i32): i32 {
    return x * 2;
}

func make(): func(i32): i32 {
    return double;
}

func main(): i32 {
    return make()(4);
}
"#;
    let output = run_host("cvi20", source);
    assert_exit("cvi20", &output, 8);
}

/// A lambda that captures a func-typed local calls the capture, not the
/// same-named module function.
///
/// The local `step` adds one and the module function `step` adds a hundred. A
/// local may shadow a module name, so the call by name inside the closure must
/// reach the captured value; `4 + 1` exits `5`, while the module function would
/// exit `104`.
#[test]
fn should_call_the_captured_function_when_a_module_function_has_the_same_name() {
    let source = r#"module codegen.cvi.st21;

func step(x: i32): i32 {
    return x + 100;
}

func main(): i32 {
    let step: func(i32): i32 = (x: i32) => x + 1;
    return ([step] (y: i32) => step(y))(4);
}
"#;
    let output = run_host("cvi21", source);
    assert_exit("cvi21", &output, 5);
}

/// A module-level func-typed `let` called by name resolves to the global.
///
/// The global holds `double`; `global_fn(3)` must load the global's pointer and
/// call it, so the exit code is `6`.
#[test]
fn should_call_the_global_function_when_a_module_global_holds_a_func_value() {
    let source = r#"module codegen.cvi.st22;

func double(x: i32): i32 {
    return x * 2;
}

let global_fn: func(i32): i32 = double;

func main(): i32 {
    return global_fn(3);
}
"#;
    let output = run_host("cvi22", source);
    assert_exit("cvi22", &output, 6);
}

/// A function-pointer array element is called through the element value.
///
/// An indexed callee is the element the access yields, so `a[0](3)` calls the
/// selected entry and every read-back adds up; a call that dropped the callee
/// would compute the bare argument (red at the phase base: the emission
/// reduced both calls to their arguments and the run exited 248).
#[test]
fn should_call_an_element_when_a_function_pointer_array_element_is_the_callee() {
    let source = r#"module codegen.cvi.st42;

func twice(x: i32): i32 {
    return x * 2;
}

struct Holder {
    arr: (func(i32): i32)[2];
}

func main(): i32 {
    let a: (func(i32): i32)[2] = [twice, twice];
    let h: Holder = Holder { arr: [twice, twice] };
    return (a[0](3) - 6) + (h.arr[0](5) - 10);
}
"#;
    let output = run_host("cvi42", source);
    assert_exit("cvi42", &output, 0);
}

/// A `String` argument widens to `str` when the callee is a func value.
///
/// `show` has type `func(str): void`; passing `stringOf("hi")` must emit the
/// `String` to `str` view conversion before the indirect call. `print` writes
/// the view's bytes, so the emitted C must build and the process writes `hi`
/// and exits `0`.
#[test]
fn should_widen_the_string_argument_when_a_func_value_takes_a_str_parameter() {
    let source = r#"module codegen.cvi.st23;

import { stringOf } from std.text;
import { print } from std.io;

func main(): i32 {
    let show: func(str): void = print;
    show(stringOf("hi"));
    return 0;
}
"#;
    let output = run_host("cvi23", source);
    assert_exit("cvi23", &output, 0);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hi",
        "cvi23: the widened argument must reach the callee; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A `borrow` func value writes through the pointer and the caller sees the
/// update.
///
/// `Entity` owns a `String`, so a lend cannot bind a copy. `bump` has type
/// `func(borrow Entity): void` and calls `nudge`, which sets `value` to `42`;
/// the caller reads `e.value` after the indirect call, so the exit code is
/// `42`. A func-type parameter that dropped the mode would pass the value
/// instead of its address and leave `value` at `1`.
#[test]
fn should_see_the_write_when_a_borrow_func_value_updates_the_caller_value() {
    let source = r#"module codegen.cvi.st24;

import { String, stringOf } from std.text;

struct Entity {
    name: String;
    value: i32;
}

func nudge(borrow e: Entity): void {
    e.value = 42;
}

func main(): i32 {
    let e: Entity = Entity { name: stringOf("e"), value: 1 };
    let bump: func(borrow Entity): void = nudge;
    bump(e);
    return e.value;
}
"#;
    let output = run_host("cvi24", source);
    assert_exit("cvi24", &output, 42);
}

/// The same `borrow` func type as a struct field writes through the pointer and
/// the caller sees the update.
///
/// `Ops.run` has type `func(borrow Entity): void` and holds `nudge`; invoking
/// the field with `e` sets `e.value` to `42`, so the exit code is `42`.
#[test]
fn should_see_the_write_when_a_borrow_func_field_updates_the_caller_value() {
    let source = r#"module codegen.cvi.st25;

import { String, stringOf } from std.text;

struct Entity {
    name: String;
    value: i32;
}

struct Ops {
    run: func(borrow Entity): void;
}

func nudge(borrow e: Entity): void {
    e.value = 42;
}

func main(): i32 {
    let e: Entity = Entity { name: stringOf("e"), value: 1 };
    let ops: Ops = Ops { run: nudge };
    ops.run(e);
    return e.value;
}
"#;
    let output = run_host("cvi25", source);
    assert_exit("cvi25", &output, 42);
}

/// A directly invoked array-returning lambda reads the returned element.
///
/// The parenthesized lambda is called in place; with `5` the array is `[5, 6]`,
/// so the exit code is `6`. The direct call must supply the lifted function's
/// hidden destination.
#[test]
fn should_read_the_second_element_when_an_array_lambda_is_invoked_directly() {
    let source = r#"module codegen.cvi.st39;

func main(): i32 {
    return ((x: i32) => { return [x, x + 1]; })(5)[1];
}
"#;
    let output = run_host("cvi39", source);
    assert_exit("cvi39", &output, 6);
}

/// Assigning a `borrow` function to a by-value func type is rejected.
///
/// The declared `nudge` takes `borrow Entity`, while the binding's type says
/// `func(Entity): void`. Parameter modes are part of the exact signature, so the
/// mismatch reports E10210 and the compiler writes no C.
#[test]
fn should_reject_a_by_value_func_value_when_the_function_takes_a_borrow_parameter() {
    let source = r#"module codegen.cvi.st40;

struct Entity {
    value: i32;
}

func nudge(borrow e: Entity): void {
    e.value = 42;
}

func main(): i32 {
    let g: func(Entity): void = nudge;
    let e: Entity = Entity { value: 1 };
    g(e);
    return 0;
}
"#;
    assert_rejected_with("cvi40", source, "E10210", &[]);
}

/// Moving a `take` array parameter into a local copies its elements.
///
/// `f` receives a `String[2]`, moves it into `b`, and reads the first element,
/// which is `"ab"`. The parameter is passed as a pointer, so the move must copy
/// the elements the pointer addresses; a copy that read the parameter slot
/// itself would over-read, and zeroing the slot before the copy would leave
/// `b[0]` dangling. The exit code scales the length by ten and adds one, so a
/// zero or garbage length cannot masquerade as the right answer, and a clean
/// exit proves neither the copy nor the parameter cleanup touched freed memory.
#[test]
fn should_copy_the_elements_when_a_take_array_parameter_moves_into_a_local() {
    let source = r#"module codegen.cvi.st26;

import { String, stringOf } from std.text;

func f(take a: String[2]): i32 {
    let b: String[2] = a;
    return i32(len(b[0]));
}

func main(): i32 {
    let pair: String[2] = [stringOf("ab"), stringOf("cde")];
    return f(pair) * 10 + 1;
}
"#;
    let output = run_host("cvi26", source);
    assert_exit("cvi26", &output, 21);
}

/// A guarded `do`/`while` re-evaluates its condition after every body pass.
///
/// The condition materializes `mk()` and compares its first element's length
/// with two. Each body pass increments `i`, so the test must run after every
/// pass and the loop ends once `i` reaches three. A condition that reads a
/// temporary that was never materialized sees an empty element, so the exit
/// code proves the condition ran after each pass.
#[test]
fn should_re_evaluate_the_condition_when_a_do_while_body_falls_through() {
    let source = r#"module codegen.cvi.st27;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cd")];
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
    } while (vlen(mk()[0]) == 2 && i < 3);
    return i;
}
"#;
    let output = run_host("cvi27", source);
    assert_exit("cvi27", &output, 3);
}

/// A `continue` in a guarded `do`/`while` body reaches the condition too.
///
/// The first pass falls through and sets `i` to one; the second pass takes the
/// `continue` edge, which must still evaluate the guarded condition and its
/// test before the next pass; the third pass falls through and ends the loop at
/// three. A condition block that a `continue` bypasses leaves the loop at a
/// wrong value or reads an empty temporary, so the exit code proves the
/// `continue` edge re-evaluated the condition.
#[test]
fn should_re_evaluate_the_condition_when_a_do_while_body_continues() {
    let source = r#"module codegen.cvi.st38;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cd")];
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let i: i32 = 0;
    do {
        if (i == 1) {
            i = i + 1;
            continue;
        }
        i = i + 1;
    } while (vlen(mk()[0]) == 2 && i < 3);
    return i;
}
"#;
    let output = run_host("cvi38", source);
    assert_exit("cvi38", &output, 3);
}

/// A guarded `do`/`while` condition carrying a fixed-array call re-evaluates
/// the call after every body pass.
///
/// `mk2` always returns `[5, 7]`, so the first element is five on every test.
/// The body increments `i`, so the test must run after each pass and the loop
/// ends at three. A stale or missing temporary would compare a zero element and
/// stop the loop after the first pass.
#[test]
fn should_re_evaluate_the_guarded_operand_when_a_do_while_condition_holds_a_fixed_array_call() {
    let source = r#"module codegen.cvi.st28;

func mk2(): i32[2] {
    return [5, 7];
}

func main(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
    } while (mk2()[0] == 5 && i < 3);
    return i;
}
"#;
    let output = run_host("cvi28", source);
    assert_exit("cvi28", &output, 3);
}

/// A `while` guard with an embedded growable call runs the body three times.
///
/// Each test re-materializes `growable()` and reads the first growable array's
/// count, which is one, so the body runs three times and the loop ends at
/// three. A re-materialization that does not release the previous growable
/// value would leak its buffer; that leak is only visible to AddressSanitizer,
/// so this case pins the loop count and the clean exit and leaves the leak
/// check to the sanitizer run.
#[test]
fn should_run_the_body_three_times_when_a_while_guard_holds_a_growable_call() {
    let source = r#"module codegen.cvi.st29;

import { Array, arrayNew } from std.mem;

func growable(): Array<i32>[2] {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    let b: Array<i32> = arrayNew<i32>();
    b.push(2);
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 3 && growable()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("cvi29", source);
    assert_exit("cvi29", &output, 3);
}

/// A `while` guard with an embedded fixed-array call keeps its behavior.
///
/// Each test materializes `mk2()` and compares its first element with five,
/// which always holds, so the body runs until the `i` counter reaches two.
/// This pins the fixed-array embedded guard while the growable form changes.
#[test]
fn should_keep_the_fixed_array_guard_when_a_while_condition_holds_a_call() {
    let source = r#"module codegen.cvi.st30;

func mk2(): i32[2] {
    return [5, 7];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && mk2()[0] == 5) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("cvi30", source);
    assert_exit("cvi30", &output, 2);
}

/// A growable array bound in a statement keeps its count and element reads.
///
/// `growableOne` returns a growable array holding seven and nine, and the loop
/// walks it by its `count`, so the sum is sixteen. The clean exit pins the
/// statement binding's cleanup path while the embedded guard spelling changes.
#[test]
fn should_keep_the_growable_statement_reads_when_a_local_holds_an_array() {
    let source = r#"module codegen.cvi.st31;

import { Array, arrayNew } from std.mem;

func growableOne(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(7);
    a.push(9);
    return a;
}

func main(): i32 {
    let arr: Array<i32> = growableOne();
    let total: i32 = 0;
    let i: usize = 0;
    while (i < arr.count) {
        total = total + arr.get(i)[0];
        i = i + 1;
    }
    return total;
}
"#;
    let output = run_host("cvi31", source);
    assert_exit("cvi31", &output, 16);
}
