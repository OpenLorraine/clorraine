//! Runnable specification tests for conditional projection integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. The memory helper
//! runs a case as an ordinary strict host build and adds leak and
//! use-after-free detection when `RDC_ASAN` is set.
//!
//! The cases pin the ownership of an owned member projected out of a
//! conditional. When such a projection is consumed — by a store, a return, a
//! `take` argument, or a deferred body — the chosen arm's member must move
//! into the consuming destination while the chosen root keeps its un-moved
//! fields, so every created owner releases exactly once and the place arm
//! stays readable. A guarded intrinsic argument and a nested conditional arm
//! follow the same rule. The final group holds accepted controls whose
//! behavior must not change.

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
            "clc-cpi-{}-{tag}-{sequence}",
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

/// Compiles `source` for the host profile and returns the scratch directory,
/// the compiler output, and the emitted-C path.
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

/// Emits one host-profile program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
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

/// Runs a built program with a time limit and the given environment values.
fn run_with_timeout(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
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
    let emitted = emit_host(tag, source);
    let program = build_host_c(tag, &scratch, &emitted);
    run_with_timeout(tag, &program, &[])
}

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
fn run_asan(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let emitted = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emitted.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&emitted.stderr)
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
        "{tag}: the sanitizer build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(
        tag,
        &program,
        &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")],
    )
}

/// Runs a memory-safety case: under AddressSanitizer when `RDC_ASAN` is set,
/// otherwise as an ordinary strict host build.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
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

/// Asserts the run produced no AddressSanitizer report.
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// armstore: the projection of a mixed conditional consumed by a store.
//
// The conditional mixes a named place arm with a fresh call arm, and the
// chosen member is stored into an owned local. The chosen member moves into
// the local; the chosen root (the named place or the fresh call result) keeps
// its un-moved sibling field and releases it once. Reading the place arm's
// sibling after the store proves the root was not emptied.
// ---------------------------------------------------------------------------

/// A store consumes the projection taken from the place arm.
///
/// The place arm's `v` transfers into `s`, and the same root keeps its `w`
/// field, which the later read still sees. Exactly one release runs for each
/// owner, so the run is clean and exits 0.
#[test]
fn should_store_a_conditional_projections_place_arm() {
    let source = r#"module spec.cpi.armstoreplace;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = true;
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    let s: String = (c ? a : mkBox()).v;
    return i32(len(s)) + i32(len(a.w)) - 6;
}
"#;
    let output = run_memory("armstoreplace", source);
    assert_no_asan("armstoreplace", &output);
    assert_exit("armstoreplace", &output, 0);
}

/// A store consumes the projection taken from the fresh call arm.
///
/// The fresh call's `v` transfers into `s`, and the same call result keeps its
/// `w` field, which is released once by that root. The named place arm stays
/// fully owned and readable. The run is clean and exits 0.
#[test]
fn should_store_a_conditional_projections_fresh_arm() {
    let source = r#"module spec.cpi.armstorefresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = false;
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    let s: String = (c ? a : mkBox()).v;
    return i32(len(s)) + i32(len(a.w)) - 6;
}
"#;
    let output = run_memory("armstorefresh", source);
    assert_no_asan("armstorefresh", &output);
    assert_exit("armstorefresh", &output, 0);
}

// ---------------------------------------------------------------------------
// armreturn: the projection of a mixed conditional consumed by a return.
//
// The returned member must move out of the chosen arm's root so that the
// function's exit cleanup releases only the un-moved fields; the caller then
// owns the returned value exactly once.
// ---------------------------------------------------------------------------

/// A return consumes the projection taken from the place arm.
///
/// The local `a`'s `v` transfers into the returned `String`, and `a`'s `w`
/// field is released once at the function exit. The run is clean and the
/// returned length is correct.
#[test]
fn should_return_a_conditional_projections_place_arm() {
    let source = r#"module spec.cpi.armreturnplace;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func project(c: bool): String {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    return (c ? a : mkBox()).v;
}

func main(): i32 {
    let s: String = project(true);
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("armreturnplace", source);
    assert_no_asan("armreturnplace", &output);
    assert_exit("armreturnplace", &output, 0);
}

/// A return consumes the projection taken from the fresh call arm.
///
/// The fresh call's `v` transfers into the returned `String`, and that call
/// result's `w` field is released once at the function exit. The run is clean
/// and the returned length is correct.
#[test]
fn should_return_a_conditional_projections_fresh_arm() {
    let source = r#"module spec.cpi.armreturnfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func project(c: bool): String {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    return (c ? a : mkBox()).v;
}

func main(): i32 {
    let s: String = project(false);
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("armreturnfresh", source);
    assert_no_asan("armreturnfresh", &output);
    assert_exit("armreturnfresh", &output, 0);
}

// ---------------------------------------------------------------------------
// armtake: the projection of a mixed conditional consumed by a `take`
// argument.
//
// Ownership transfers into the callee, so the chosen arm's member must move
// out and the caller's chosen root must keep releasing its un-moved fields.
// ---------------------------------------------------------------------------

/// A `take` argument consumes the projection taken from the place arm.
///
/// The place arm's `v` transfers into the callee, and the caller's `a` keeps
/// its `w` field readable. The run is clean and the callee's length is
/// correct.
#[test]
fn should_take_a_conditional_projections_place_arm() {
    let source = r#"module spec.cpi.armtakeplace;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func consume(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let c: bool = true;
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    let n: i32 = consume((c ? a : mkBox()).v);
    return n + i32(len(a.w)) - 6;
}
"#;
    let output = run_memory("armtakeplace", source);
    assert_no_asan("armtakeplace", &output);
    assert_exit("armtakeplace", &output, 0);
}

/// A `take` argument consumes the projection taken from the fresh call arm.
///
/// The fresh call's `v` transfers into the callee, and that call result's `w`
/// field is released once. The named place arm stays fully owned and readable.
/// The run is clean and the callee's length is correct.
#[test]
fn should_take_a_conditional_projections_fresh_arm() {
    let source = r#"module spec.cpi.armtakefresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func consume(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let c: bool = false;
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    let n: i32 = consume((c ? a : mkBox()).v);
    return n + i32(len(a.w)) - 6;
}
"#;
    let output = run_memory("armtakefresh", source);
    assert_no_asan("armtakefresh", &output);
    assert_exit("armtakefresh", &output, 0);
}

// ---------------------------------------------------------------------------
// armdefer: the projection of a mixed conditional consumed inside a `defer`
// body.
//
// The deferred body's owned `take` argument follows the same move-out rule,
// and the deferred block must still run on exit. A module-level counter
// records each execution so the run result proves the defer ran for both
// taken arms.
// ---------------------------------------------------------------------------

/// A deferred `take` consumes the projection taken from the place arm.
///
/// The deferred block runs on exit, consuming the place arm's `v` and adding
/// its length to the counter. The run is clean and the counter shows the
/// block ran once.
#[test]
fn should_consume_a_conditional_projections_place_arm_in_a_defer() {
    let source = r#"module spec.cpi.armdeferplace;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func consume(take s: String): i32 {
    return i32(len(s));
}

let total: i32 = 0;

func body(c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    defer {
        total = total + consume((c ? a : mkBox()).v);
    }
}

func main(): i32 {
    body(true);
    return total - 2;
}
"#;
    let output = run_memory("armdeferplace", source);
    assert_no_asan("armdeferplace", &output);
    assert_exit("armdeferplace", &output, 0);
}

/// A deferred `take` consumes the projection taken from the fresh call arm.
///
/// The deferred block runs on exit, consuming the fresh call's `v` and adding
/// its length to the counter; the call result's `w` field is released once.
/// The run is clean and the counter shows the block ran once.
#[test]
fn should_consume_a_conditional_projections_fresh_arm_in_a_defer() {
    let source = r#"module spec.cpi.armdeferfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func consume(take s: String): i32 {
    return i32(len(s));
}

let total: i32 = 0;

func body(c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    defer {
        total = total + consume((c ? a : mkBox()).v);
    }
}

func main(): i32 {
    body(false);
    return total - 2;
}
"#;
    let output = run_memory("armdeferfresh", source);
    assert_no_asan("armdeferfresh", &output);
    assert_exit("armdeferfresh", &output, 0);
}

// ---------------------------------------------------------------------------
// armguard: an owned projection of a mixed conditional passed to a guarded
// intrinsic argument.
//
// The `len` intrinsic reads the projection, so no ownership is consumed; the
// place arm must stay live and the fresh arm's root must be released once.
// The guard's right operand is evaluated only when the left operand does not
// already fix the result, so each guard spelling is paired with a guard
// variable that forces the projection to be reached.
// ---------------------------------------------------------------------------

/// A guarded intrinsic argument reads the place arm under `&&`.
///
/// The left operand is true, so the projection is evaluated and the place arm
/// stays live; the run is clean and the place arm is readable afterward.
#[test]
fn should_guard_a_conditional_projections_place_arm_under_and() {
    let source = r#"module spec.cpi.armguardandplace;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("mk") };
}

func main(): i32 {
    let c: bool = true;
    let b: Box = Box { v: stringOf("ab") };
    if (c && i32(len((c ? b : mk()).v)) > 0) {
        return i32(len(b.v)) - 2;
    }
    return 99;
}
"#;
    let output = run_memory("armguardandplace", source);
    assert_no_asan("armguardandplace", &output);
    assert_exit("armguardandplace", &output, 0);
}

/// A guarded intrinsic argument reads the fresh arm under `&&`.
///
/// The guard variable is true, so the projection is reached and the fresh
/// call's root is released once. The named place arm stays live and readable;
/// the run is clean and exits 0.
#[test]
fn should_guard_a_conditional_projections_fresh_arm_under_and() {
    let source = r#"module spec.cpi.armguardandfresh;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("mk") };
}

func main(): i32 {
    let g: bool = true;
    let c: bool = false;
    let b: Box = Box { v: stringOf("ab") };
    if (g && i32(len((c ? b : mk()).v)) > 0) {
        return i32(len(b.v)) - 2;
    }
    return 99;
}
"#;
    let output = run_memory("armguardandfresh", source);
    assert_no_asan("armguardandfresh", &output);
    assert_exit("armguardandfresh", &output, 0);
}

/// A guarded intrinsic argument reads the place arm under `||`.
///
/// The guard variable is false, so the right operand is reached and the
/// projection reads the place arm, which stays live. The run is clean and
/// exits 0.
#[test]
fn should_guard_a_conditional_projections_place_arm_under_or() {
    let source = r#"module spec.cpi.armguardorplace;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("mk") };
}

func main(): i32 {
    let g: bool = false;
    let c: bool = true;
    let b: Box = Box { v: stringOf("ab") };
    if (g || i32(len((c ? b : mk()).v)) > 0) {
        return i32(len(b.v)) - 2;
    }
    return 99;
}
"#;
    let output = run_memory("armguardorplace", source);
    assert_no_asan("armguardorplace", &output);
    assert_exit("armguardorplace", &output, 0);
}

/// A guarded intrinsic argument reads the fresh arm under `||`.
///
/// The guard variable is false, so the right operand is reached and the
/// projection reads the fresh call, whose root is released once. The named
/// place arm stays live and readable; the run is clean and exits 0.
#[test]
fn should_guard_a_conditional_projections_fresh_arm_under_or() {
    let source = r#"module spec.cpi.armguardorfresh;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("mk") };
}

func main(): i32 {
    let c: bool = false;
    let b: Box = Box { v: stringOf("ab") };
    if (c || i32(len((c ? b : mk()).v)) > 0) {
        return i32(len(b.v)) - 2;
    }
    return 99;
}
"#;
    let output = run_memory("armguardorfresh", source);
    assert_no_asan("armguardorfresh", &output);
    assert_exit("armguardorfresh", &output, 0);
}

// ---------------------------------------------------------------------------
// armnested: the projection of a nested conditional consumed by a store.
//
// The inner conditional selects the member's root; the store must move out of
// the innermost chosen root and each level must release only its un-moved
// fields. All four combinations of the inner and outer conditions are pinned.
// ---------------------------------------------------------------------------

/// A nested store takes both the inner and the outer place arms.
///
/// The innermost root is the named place, whose `v` moves into `s` and whose
/// `w` field is released once. The run is clean and exits 0.
#[test]
fn should_store_a_nested_conditional_projection_with_both_place_arms() {
    let source = r#"module spec.cpi.armnestedpp;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = true;
    let d: bool = true;
    let b: Box = Box { v: stringOf("ab"), w: stringOf("wwww") };
    let s: String = d ? (c ? b : mk()).v : stringOf("zz");
    return i32(len(s)) + i32(len(b.w)) - 6;
}
"#;
    let output = run_memory("armnestedpp", source);
    assert_no_asan("armnestedpp", &output);
    assert_exit("armnestedpp", &output, 0);
}

/// A nested store takes the inner fresh arm and the outer place arm.
///
/// The innermost root is the fresh call, whose `v` moves into `s` and whose
/// `w` field is released once. The named place arm stays fully owned and
/// readable. The run is clean and exits 0.
#[test]
fn should_store_a_nested_conditional_projection_with_an_inner_fresh_arm() {
    let source = r#"module spec.cpi.armnestedpf;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = false;
    let d: bool = true;
    let b: Box = Box { v: stringOf("ab"), w: stringOf("wwww") };
    let s: String = d ? (c ? b : mk()).v : stringOf("zz");
    return i32(len(s)) + i32(len(b.w)) - 6;
}
"#;
    let output = run_memory("armnestedpf", source);
    assert_no_asan("armnestedpf", &output);
    assert_exit("armnestedpf", &output, 0);
}

/// A nested store takes the inner place arm and the outer fresh arm.
///
/// The outer condition selects the else arm, so the inner conditional is not
/// evaluated and the named local stays fully owned and readable. The run is
/// clean and exits 0.
#[test]
fn should_store_a_nested_conditional_projection_with_an_outer_fresh_arm() {
    let source = r#"module spec.cpi.armnestedfp;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = true;
    let d: bool = false;
    let b: Box = Box { v: stringOf("ab"), w: stringOf("wwww") };
    let s: String = d ? (c ? b : mk()).v : stringOf("zz");
    return i32(len(s)) + i32(len(b.w)) - 6;
}
"#;
    let output = run_memory("armnestedfp", source);
    assert_no_asan("armnestedfp", &output);
    assert_exit("armnestedfp", &output, 0);
}

/// A nested store takes both the inner and the outer fresh arms.
///
/// The outer condition selects the else arm, so the inner conditional is not
/// evaluated and the named local stays fully owned and readable. The run is
/// clean and exits 0.
#[test]
fn should_store_a_nested_conditional_projection_with_both_fresh_arms() {
    let source = r#"module spec.cpi.armnestedff;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = false;
    let d: bool = false;
    let b: Box = Box { v: stringOf("ab"), w: stringOf("wwww") };
    let s: String = d ? (c ? b : mk()).v : stringOf("zz");
    return i32(len(s)) + i32(len(b.w)) - 6;
}
"#;
    let output = run_memory("armnestedff", source);
    assert_no_asan("armnestedff", &output);
    assert_exit("armnestedff", &output, 0);
}

// ---------------------------------------------------------------------------
// armplace: accepted controls whose behavior must not change.
//
// These guard against binding a whole conditional as one owner, which would
// empty or release the place arm. The place arm must stay readable around an
// operation that uses the other arm, and the two accepted lend/view controls
// below pin the same behavior.
// ---------------------------------------------------------------------------

/// The place arm stays readable around a fresh-arm view binding.
///
/// The view selects the fresh call arm, so the named place is never emptied;
/// its sibling field can be read before the binding and mutated and read
/// after it. The run is clean and exits 0.
#[test]
fn should_keep_a_place_arm_readable_around_a_fresh_arm_view() {
    let source = r#"module spec.cpi.armplaceview;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = false;
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    let before: i32 = i32(len(a.w));
    let s: str = (c ? a : mkBox()).v;
    a.w = stringOf("ww");
    let after: i32 = i32(len(a.w));
    return before + after + i32(len(s)) - 8;
}
"#;
    let output = run_memory("armplaceview", source);
    assert_no_asan("armplaceview", &output);
    assert_exit("armplaceview", &output, 0);
}

/// A mixed conditional lend argument keeps its place arm readable.
///
/// The conditional mixes a fresh call result with a named place, and the
/// argument is a read-only lend, so the place stays an inline read and the
/// later use sees the live value. The run is clean and exits 0.
#[test]
fn should_keep_a_mixed_conditional_lend_arguments_place_arm_readable() {
    let source = r#"module spec.cpi.armplacelend;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let b: String = stringOf("bb");
    let pick: bool = false;
    let n: i32 = lenOf(pick ? stringOf("aa") : b);
    let m: i32 = i32(len(b));
    return n + m - 4;
}
"#;
    let output = run_memory("armplacelend", source);
    assert_no_asan("armplacelend", &output);
    assert_exit("armplacelend", &output, 0);
}

/// Both owning conditional view arms release their siblings.
///
/// Both arms project a fresh struct literal, so the taken arm's projected
/// member and every sibling field must be released on that path. The run is
/// clean and exits 0.
#[test]
fn should_release_both_owning_conditional_view_arms_siblings() {
    let source = r#"module spec.cpi.armplacesiblings;

import { String, stringOf } from std.text;

struct P { a: String; b: String; }
struct Q { a: String; b: String; }

func main(): i32 {
    let c: bool = true;
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let v: str = c ? (P { a: stringOf("ab"), b: stringOf("cd") }).a : (Q { a: stringOf("ef"), b: stringOf("gh") }).a;
        n = n + i32(len(v));
        i = i + 1;
    }
    return n - 6;
}
"#;
    let output = run_memory("armplacesiblings", source);
    assert_no_asan("armplacesiblings", &output);
    assert_exit("armplacesiblings", &output, 0);
}

/// A re-executed `take` of a conditional projection transfers the value once
/// per iteration.
///
/// The place arm is taken twice in one loop and the fresh arm twice in a
/// second loop; the caller's copy of the transferred value must be emptied
/// after each call, so re-entering the same arm never releases what the
/// callee already owns. The run is clean and every length is correct.
#[test]
fn should_take_a_conditional_projection_again_in_a_loop() {
    let source = r#"module spec.cpi.armtakeloop;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func consume(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
        n = n + consume((true ? a : mkBox()).v);
        i = i + 1;
    }
    let j: i32 = 0;
    while (j < 2) {
        let b: Box = Box { v: stringOf("bb"), w: stringOf("wwww") };
        n = n + consume((false ? b : mkBox()).v);
        j = j + 1;
    }
    return n - 8;
}
"#;
    let output = run_memory("armtakeloop", source);
    assert_no_asan("armtakeloop", &output);
    assert_exit("armtakeloop", &output, 0);
}

/// A read of the moved member after a nested conditional store reports the
/// moved-value error.
///
/// Consuming `d ? (c ? b : mk()).v : z` moves `b.v` out on the path that
/// selects it, so a later read of that member is the ordinary moved-value
/// error rather than a read of an emptied handle.
#[test]
fn should_report_the_moved_member_after_a_nested_conditional_store() {
    let source = r#"module spec.cpi.armnestedread;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = true;
    let d: bool = true;
    let b: Box = Box { v: stringOf("ab"), w: stringOf("wwww") };
    let s: String = d ? (c ? b : mk()).v : stringOf("zz");
    let t: i32 = i32(len(b.v));
    return i32(len(s)) + t - 4;
}
"#;
    let (_scratch, output, c_path) = compile_only("armnestedread", source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "armnestedread: expected a rejection; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("E10020"),
        "armnestedread: expected the moved-value report; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "armnestedread: a rejected program must not produce a translation unit"
    );
}

/// A read of the moved member reports after a nested projection whose sibling
/// arm is a named place.
///
/// The sibling arm contributes its own move source, so the walk must still
/// collect the projection arm's member: consuming the outer conditional moves
/// `b.v`, and a later read of it is the ordinary moved-value error.
#[test]
fn should_report_the_moved_member_after_a_nested_projection_with_a_place_sibling() {
    let source = r#"module spec.cpi.armnestedplace;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = true;
    let d: bool = true;
    let b: Box = Box { v: stringOf("ab"), w: stringOf("wwww") };
    let z: String = stringOf("zz");
    let s: String = d ? (c ? b : mk()).v : z;
    let t: i32 = i32(len(b.v));
    return i32(len(s)) + t - 4;
}
"#;
    let (_scratch, output, c_path) = compile_only("armnestedplace", source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "armnestedplace: expected a rejection; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("E10020"),
        "armnestedplace: expected the moved-value report; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "armnestedplace: a rejected program must not produce a translation unit"
    );
}
