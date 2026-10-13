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

/// Asserts the front end rejected `source` with exactly `count` error reports,
/// each carrying `code`, and emitted no translation unit.
///
/// The exact count is the point: a shared-loan conflict is reported once at the
/// mutation site, matching the named spelling of the loan. Every error line
/// must carry the expected code, so a fix cannot pass by adding an unrelated
/// diagnostic, and a rejected program must leave no emitted `.c` behind.
fn compile_rejects(tag: &str, source: &str, code: &str, count: usize) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        count,
        "{tag}: expected {count} error(s), saw {}; stderr:\n{stderr}",
        errors.len()
    );
    for line in &errors {
        assert!(
            line.contains(code),
            "{tag}: expected the {code} report; saw `{line}`; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
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
/// its sibling field stays owned and readable before and after the binding —
/// an emptied root would read as the empty view. A write to the place while
/// the view is live is the shared-loan conflict its named spelling reports,
/// so the control only reads. The run is clean and exits 0.
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
    let after: i32 = i32(len(a.w));
    return before + after + i32(len(s)) - 10;
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

// ---------------------------------------------------------------------------
// guardliteral: a guarded callable-field call on a struct-literal receiver.
//
// The receiver is a struct literal that owns an owned field, so the call's
// taken path must release the literal's storage exactly once. When a guard
// short-circuits and never evaluates the call, the literal is never created,
// so nothing may be released on that path.
// ---------------------------------------------------------------------------

/// The `&&` right operand reaches the struct-literal receiver.
///
/// The left operand is `true`, so the guarded call runs: it returns 6 and the
/// taken path returns 1. The literal's owned field releases exactly once on
/// that path, so the run is clean.
#[test]
fn should_release_a_guarded_struct_literal_receiver_under_and() {
    let source = r#"module spec.cpi.guardliteral;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    if (true && H { call: addOne, s: stringOf("x") }.call(5) > 0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_memory("guardliteral", source);
    assert_no_asan("guardliteral", &output);
    assert_exit("guardliteral", &output, 1);
}

/// The `||` right operand reaches the struct-literal receiver.
///
/// The left operand is `false`, so the right operand runs the guarded call and
/// the taken path returns 1; the literal's owned field releases exactly once on
/// that path, so the run is clean.
#[test]
fn should_release_a_guarded_struct_literal_receiver_under_or() {
    let source = r#"module spec.cpi.guardliteralor;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    if (false || H { call: addOne, s: stringOf("x") }.call(5) > 0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_memory("guardliteralor", source);
    assert_no_asan("guardliteralor", &output);
    assert_exit("guardliteralor", &output, 1);
}

/// The chosen `?:` arm reaches the struct-literal receiver.
///
/// The condition is `true`, so the selected arm runs the guarded call and the
/// taken path returns 1; the literal's owned field releases exactly once on
/// that path, so the run is clean.
#[test]
fn should_release_a_guarded_struct_literal_receiver_in_a_chosen_arm() {
    let source = r#"module spec.cpi.guardliteralcond;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    if ((true ? H { call: addOne, s: stringOf("x") }.call(5) : 0) > 0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_memory("guardliteralcond", source);
    assert_no_asan("guardliteralcond", &output);
    assert_exit("guardliteralcond", &output, 1);
}

/// The `&&` right operand is skipped, so nothing is released.
///
/// The left operand is `false`, which fixes the guard without evaluating the
/// struct literal. The literal is never created, so its release must be a
/// no-op; the run exits 0 and is clean.
#[test]
fn should_not_release_a_skipped_struct_literal_receiver_under_and() {
    let source = r#"module spec.cpi.guardliteralskipand;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    if (false && H { call: addOne, s: stringOf("x") }.call(5) > 0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_memory("guardliteralskipand", source);
    assert_no_asan("guardliteralskipand", &output);
    assert_exit("guardliteralskipand", &output, 0);
}

/// The `||` right operand is skipped, so nothing is released.
///
/// The left operand is `true`, which fixes the guard without evaluating the
/// struct literal. The literal is never created, so its release must be a
/// no-op; the taken path returns 1 and the run is clean.
#[test]
fn should_not_release_a_skipped_struct_literal_receiver_under_or() {
    let source = r#"module spec.cpi.guardliteralskipor;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    if (true || H { call: addOne, s: stringOf("x") }.call(5) > 0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_memory("guardliteralskipor", source);
    assert_no_asan("guardliteralskipor", &output);
    assert_exit("guardliteralskipor", &output, 1);
}

/// The unselected `?:` arm is not evaluated, so nothing is released.
///
/// The condition is `false`, so the `0` arm is selected and the struct literal
/// is never created; the untaken release must be a no-op. The run exits 0 and
/// is clean.
#[test]
fn should_not_release_an_unselected_struct_literal_receiver_arm() {
    let source = r#"module spec.cpi.guardliteralskipcond;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    if ((false ? H { call: addOne, s: stringOf("x") }.call(5) : 0) > 0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_memory("guardliteralskipcond", source);
    assert_no_asan("guardliteralskipcond", &output);
    assert_exit("guardliteralskipcond", &output, 0);
}

// ---------------------------------------------------------------------------
// placearmloan: a shared loan recorded by a conditional view's place arm.
//
// Binding a view of a place records a shared loan of that place, and mutating
// the place while the loan is live is E10031. A conditional view whose arm
// roots in a place behaves exactly like the named spelling, so it records the
// same loan; the fresh-call arm adds no loan, and a read-only use of the view
// stays accepted.
// ---------------------------------------------------------------------------

/// A conditional view whose chosen arm is a place records that place's shared
/// loan, so the later mutation reports E10031 once.
///
/// The view `(c ? b : mk()).v` derives from the place `b`, exactly like the
/// named spelling `let s: str = b.v;`. Writing `b.v` while the view is live is
/// the shared-loan mutation error; it is reported once at the mutation site and
/// no translation unit is emitted.
#[test]
fn should_report_a_shared_loan_for_a_conditional_views_place_arm() {
    let source = r#"module spec.cpi.placearmloan;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("mm") }; }

func main(): i32 {
    let c: bool = true;
    let b: Box = Box { v: stringOf("bb") };
    let s: str = (c ? b : mk()).v;
    b.v = stringOf("zz");
    return i32(len(s));
}
"#;
    compile_rejects("placearmloan", source, "E10031", 1);
}

/// The named view spelling keeps reporting the shared-loan mutation error.
///
/// `let s: str = b.v;` records a shared loan of `b`, so the later `b.v` write
/// reports E10031 once. This control pins the behavior the conditional spelling
/// must match.
#[test]
fn should_report_a_shared_loan_for_a_named_views_place() {
    let source = r#"module spec.cpi.placearmnamed;

import { String, stringOf } from std.text;

struct Box { v: String; }

func main(): i32 {
    let b: Box = Box { v: stringOf("bb") };
    let s: str = b.v;
    b.v = stringOf("zz");
    return i32(len(s));
}
"#;
    compile_rejects("placearmnamed", source, "E10031", 1);
}

/// A conditional view whose chosen arm is a place stays accepted when the place
/// is not mutated.
///
/// The view derives from `b.v` and is read for its length; with no write while
/// the loan is live, the program is accepted and runs to exit 0.
#[test]
fn should_accept_a_conditional_views_place_arm_without_a_mutation() {
    let source = r#"module spec.cpi.placearmnomut;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("mm") }; }

func main(): i32 {
    let c: bool = true;
    let b: Box = Box { v: stringOf("bb") };
    let s: str = (c ? b : mk()).v;
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("placearmnomut", source);
    assert_no_asan("placearmnomut", &output);
    assert_exit("placearmnomut", &output, 0);
}

/// A mixed conditional view records the place arm's shared loan even when the
/// taken arm is the fresh call.
///
/// The conditional `(c ? b : mk())` has a place candidate on every path, so the
/// checker records the shared loan conservatively. Mutating `b` while the view
/// is live reports E10031 once, exactly like the named spelling.
#[test]
fn should_report_a_shared_loan_for_a_mixed_conditional_views_place_arm() {
    let source = r#"module spec.cpi.placearmfresh;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("mm") }; }

func main(): i32 {
    let c: bool = false;
    let b: Box = Box { v: stringOf("bb") };
    let s: str = (c ? b : mk()).v;
    b.v = stringOf("zz");
    return i32(len(s));
}
"#;
    compile_rejects("placearmfresh", source, "E10031", 1);
}

// ---------------------------------------------------------------------------
// dotlenreceiver: a dot-`len` receiver that is a conditional view projection.
//
// The receiver is a projection of a conditional, so the root that owns the
// projected member must release its un-moved fields exactly once on the taken
// path. The `len(...)` intrinsic argument spelling is a control and must stay
// unchanged.
// ---------------------------------------------------------------------------

/// A conditional view's dot-`len` receiver releases the fresh root's fields.
///
/// `(c ? mk(1) : mk(2)).v.len()` reads the projected member and releases the
/// chosen call result's un-moved sibling field exactly once. The length is 2,
/// so the run exits 0 and is clean.
#[test]
fn should_release_the_root_of_a_conditional_views_dot_len_receiver() {
    let source = r#"module spec.cpi.dotlenreceiver;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(n: i32): Box {
    return Box { v: stringOf("mm"), w: stringOf("ww") };
}

func main(): i32 {
    let c: bool = true;
    let n: usize = (c ? mk(1) : mk(2)).v.len();
    return i32(n) - 2;
}
"#;
    let output = run_memory("dotlenreceiver", source);
    assert_no_asan("dotlenreceiver", &output);
    assert_exit("dotlenreceiver", &output, 0);
}

/// A conditional view's dot-`len` receiver with a place arm keeps the place
/// live.
///
/// `(c ? b : mk()).v.len()` reads the named place's projected member; the place
/// `b` keeps its sibling field and is released once at scope end. The length is
/// 2, so the run exits 0 and is clean.
#[test]
fn should_release_the_place_arm_root_of_a_conditional_views_dot_len_receiver() {
    let source = r#"module spec.cpi.dotlenplace;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box {
    return Box { v: stringOf("mm"), w: stringOf("ww") };
}

func main(): i32 {
    let c: bool = true;
    let b: Box = Box { v: stringOf("bb"), w: stringOf("ww") };
    let n: usize = (c ? b : mk()).v.len();
    return i32(n) - 2;
}
"#;
    let output = run_memory("dotlenplace", source);
    assert_no_asan("dotlenplace", &output);
    assert_exit("dotlenplace", &output, 0);
}

/// A guarded conditional view's dot-`len` receiver releases the root once.
///
/// `g && (c ? mk(1) : mk(2)).v.len() > 0` reaches the projection, which
/// releases the chosen call result's un-moved fields exactly once. The run
/// exits 0 and is clean.
#[test]
fn should_release_a_guarded_conditional_views_dot_len_receiver() {
    let source = r#"module spec.cpi.dotlenguarded;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(n: i32): Box {
    return Box { v: stringOf("mm"), w: stringOf("ww") };
}

func main(): i32 {
    let g: bool = true;
    let c: bool = true;
    if (g && (c ? mk(1) : mk(2)).v.len() > 0) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("dotlenguarded", source);
    assert_no_asan("dotlenguarded", &output);
    assert_exit("dotlenguarded", &output, 0);
}

/// The `len(...)` intrinsic argument spelling stays accepted and clean.
///
/// `len((c ? mk(1) : mk(2)).v)` binds the conditional argument through the
/// intrinsic gate; the chosen root releases its un-moved fields once. The
/// length is 2, so the run exits 0 and is clean. This spelling is a control and
/// must not change.
#[test]
fn should_keep_the_len_intrinsic_argument_of_a_conditional_view_clean() {
    let source = r#"module spec.cpi.dotlencontrol;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(n: i32): Box {
    return Box { v: stringOf("mm"), w: stringOf("ww") };
}

func main(): i32 {
    let c: bool = true;
    let n: usize = len((c ? mk(1) : mk(2)).v);
    return i32(n) - 2;
}
"#;
    let output = run_memory("dotlencontrol", source);
    assert_no_asan("dotlencontrol", &output);
    assert_exit("dotlencontrol", &output, 0);
}

// ---------------------------------------------------------------------------
// guardplace: accepted controls whose behavior must not change.
//
// An unguarded struct-literal receiver, a conditional of two places, and a
// slice-view receiver in the dot-`len` position all stay accepted and release
// their storage exactly once.
// ---------------------------------------------------------------------------

/// An unguarded struct-literal receiver stays accepted.
///
/// `H { call: addOne, s: stringOf("x") }.call(5)` binds the literal as a
/// temporary, calls through the field, and releases the literal's owned field
/// once at statement end. The call returns 6, so the run exits 0 and is clean.
#[test]
fn should_keep_an_unguarded_struct_literal_receiver_clean() {
    let source = r#"module spec.cpi.guardplaceliteral;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    let n: i32 = H { call: addOne, s: stringOf("x") }.call(5);
    return n - 6;
}
"#;
    let output = run_memory("guardplaceliteral", source);
    assert_no_asan("guardplaceliteral", &output);
    assert_exit("guardplaceliteral", &output, 0);
}

/// A conditional of two places stays accepted.
///
/// `c ? a.v : b.v` selects one place's view and the chosen place keeps its
/// storage; both locals release once at scope end. The selected view reads
/// "aa", so the run exits 0 and is clean.
#[test]
fn should_keep_a_conditional_of_places_clean() {
    let source = r#"module spec.cpi.guardplacecond;

import { String, stringOf } from std.text;

struct Box { v: String; }

func main(): i32 {
    let c: bool = true;
    let a: Box = Box { v: stringOf("aa") };
    let b: Box = Box { v: stringOf("bb") };
    let s: str = c ? a.v : b.v;
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("guardplacecond", source);
    assert_no_asan("guardplacecond", &output);
    assert_exit("guardplacecond", &output, 0);
}

/// A slice-view receiver in the dot-`len` position stays accepted.
///
/// `slice(r.xs, 0, 2).len()` derives a `u8[]` view over a fixed-array field and
/// reads its length. The view borrows the local `r`, which releases once at
/// scope end. The length is 2, so the run exits 0 and is clean. This control
/// pins that an ordinary call-result slice receiver is not caught by the
/// conditional gate.
#[test]
fn should_keep_a_slice_view_receiver_in_the_dot_len_position_clean() {
    let source = r#"module spec.cpi.guardplaceslice;

import { slice } from std.mem;

struct Row { xs: u8[4]; }

func main(): i32 {
    let r: Row = Row { xs: [1, 2, 3, 4] };
    let n: usize = slice(r.xs, 0, 2).len();
    return i32(n) - 2;
}
"#;
    let output = run_memory("guardplaceslice", source);
    assert_no_asan("guardplaceslice", &output);
    assert_exit("guardplaceslice", &output, 0);
}

/// A guarded struct-literal receiver in a loop condition releases once per
/// iteration.
///
/// The condition re-evaluates the literal every iteration, so the literal's
/// owning field must be held by exactly one temporary: a second owner would
/// release the same storage twice when the loop exits. The run is clean and
/// exits 0.
#[test]
fn should_release_a_loop_condition_struct_literal_receiver_once_per_iteration() {
    let source = r#"module spec.cpi.guardliteralloop;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    let i: i32 = 0;
    while (i < 1 && H { call: addOne, s: stringOf("x") }.call(5) > 0) {
        i = i + 1;
    }
    return i - 1;
}
"#;
    let output = run_memory("guardliteralloop", source);
    assert_no_asan("guardliteralloop", &output);
    assert_exit("guardliteralloop", &output, 0);
}

// ---------------------------------------------------------------------------
// viewarrayloan: a view array bound over a place or a call temporary.
//
// Binding a `str[N]` array of view elements must follow the single-view rule.
// An element derived from a place keeps that place borrowed, so moving the
// owner while the view is live is the move-while-borrowed error; an element
// derived from a call temporary adopts that backing for the binding's scope,
// so the temporary is released exactly once and the view stays readable.
// ---------------------------------------------------------------------------

/// Moving a place while a view array over its member is live reports the
/// move-while-borrowed error.
///
/// The single-view twin `let v: str = a.v;` already reports one E10022 when the
/// borrowed owner is later moved, and the array spelling `let vs: str[1] =
/// [a.v];` must report the same code exactly once for the same move.
#[test]
fn should_report_a_move_of_a_borrowed_backing_behind_a_view_array() {
    let source_twin = r#"module spec.cpi.viewarrayloantwin;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func main(): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("ww") };
    let v: str = a.v;
    let b: Box = a;
    return i32(len(v)) + i32(len(b.v)) - 4;
}
"#;
    compile_rejects("viewarrayloantwin", source_twin, "E10022", 1);

    let source_array = r#"module spec.cpi.viewarrayloan;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func main(): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("ww") };
    let vs: str[1] = [a.v];
    let b: Box = a;
    return i32(len(vs[0])) + i32(len(b.v)) - 4;
}
"#;
    compile_rejects("viewarrayloan", source_array, "E10022", 1);
}

/// A view array bound over a call temporary adopts the backing and releases it
/// once.
///
/// Each element of `[mk().v]` derives from a temporary, so the binding owns the
/// backing for its own scope: the view stays readable after the statement, the
/// run exits 0, and neither a leak nor a use-after-free is reported.
///
/// Two spellings are pinned. The single-shot binding guards the adoption: it
/// reads the view one time and must be memory-clean on every path. The loop
/// spelling re-binds the array once per iteration, so the backing of each
/// iteration must be released when that iteration's binding ends; the earlier
/// iterations' storage is then genuinely unreachable, which makes a missing
/// release observable to the container's leak sanitizer even when a
/// single-shot binding would only leave the last, still-reachable block behind.
#[test]
fn should_adopt_a_temporary_backing_bound_into_a_view_array() {
    let source = r#"module spec.cpi.viewarraytemp;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("aa") };
}

func main(): i32 {
    let vs: str[1] = [mk().v];
    return i32(len(vs[0])) - 2;
}
"#;
    let output = run_memory("viewarraytemp", source);
    assert_no_asan("viewarraytemp", &output);
    assert_exit("viewarraytemp", &output, 0);

    let source_loop = r#"module spec.cpi.viewarraytemploop;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("aa") };
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let vs: str[1] = [mk().v];
        n = n + i32(len(vs[0]));
        i = i + 1;
    }
    return n - 6;
}
"#;
    let output = run_memory("viewarraytemploop", source_loop);
    assert_no_asan("viewarraytemploop", &output);
    assert_exit("viewarraytemploop", &output, 0);
}

/// Static-string and parameter-derived view arrays stay accepted and clean.
///
/// A `str[2]` binding of static strings and a `str[1]` binding of a view
/// parameter both read their elements without owning a temporary backing, so
/// both programs run to exit 0 with no memory report.
#[test]
fn should_keep_static_and_parameter_view_array_bindings_clean() {
    let source_static = r#"module spec.cpi.viewarraycontrolstatic;

func main(): i32 {
    let vs: str[2] = ["aa", "bb"];
    return i32(len(vs[0])) + i32(len(vs[1])) - 4;
}
"#;
    let output = run_memory("viewarraycontrolstatic", source_static);
    assert_no_asan("viewarraycontrolstatic", &output);
    assert_exit("viewarraycontrolstatic", &output, 0);

    let source_param = r#"module spec.cpi.viewarraycontrolparam;

func first(buf: str): i32 {
    let vs: str[1] = [buf];
    return i32(len(vs[0]));
}

func main(): i32 {
    return first("aa") - 2;
}
"#;
    let output = run_memory("viewarraycontrolparam", source_param);
    assert_no_asan("viewarraycontrolparam", &output);
    assert_exit("viewarraycontrolparam", &output, 0);
}

// ---------------------------------------------------------------------------
// elisionderive: a view return judged by its derivation, not by the count of
// view-like lend candidates.
//
// A view-returning function must derive its result from exactly one view-like
// lend parameter. A candidate that is not the derivation source is an error
// even when it is the only candidate, and zero candidates is an error too.
// The valid single-candidate spelling stays accepted.
// ---------------------------------------------------------------------------

/// A view return that derives from a non-candidate parameter reports the
/// derivation error, and the zero-candidate and valid spellings keep their
/// behavior.
///
/// The zero-candidate twin `f(h: Box): str { return h.v; }` already reports one
/// E10036. The valid control `pick(x: str): str { return x; }` derives from its
/// single candidate and stays accepted and clean. The candidate spelling
/// `pick(x: str, h: Box): str { return h.v; }` has exactly one view-like lend
/// candidate (`x`) but derives from `h`, so it must report one E10036 at its
/// declaration.
#[test]
fn should_report_a_view_return_that_derives_from_a_non_candidate() {
    let source_zero = r#"module spec.cpi.elisionderivezero;

import { String } from std.text;

struct Box { v: String; }

func f(h: Box): str {
    return h.v;
}

func main(): i32 {
    return 0;
}
"#;
    compile_rejects("elisionderivezero", source_zero, "E10036", 1);

    let source_ok = r#"module spec.cpi.elisionderiveok;

func pick(x: str): str {
    return x;
}

func main(): i32 {
    let r: str = pick("aa");
    return i32(len(r)) - 2;
}
"#;
    let output = run_memory("elisionderiveok", source_ok);
    assert_no_asan("elisionderiveok", &output);
    assert_exit("elisionderiveok", &output, 0);

    let source_candidate = r#"module spec.cpi.elisionderivecandidate;

import { String } from std.text;

struct Box { v: String; }

func pick(x: str, h: Box): str {
    return h.v;
}

func main(): i32 {
    return 0;
}
"#;
    compile_rejects("elisionderivecandidate", source_candidate, "E10036", 1);
}

/// A view return derived from a caller temporary inside a relay reports once at
/// the callee declaration.
///
/// `pick(x: str, h: Box)` returns a view derived from its `h` argument, so its
/// own declaration is E10036. The relay `mid(s: str)` returns `pick(s,
/// Box { ... })`, which derives from `mid`'s own view-like parameter `s`, so
/// `mid` itself is accepted. The single report names `pick`, and the rejected
/// program emits no translation unit, so nothing runs.
#[test]
fn should_report_a_caller_temporary_derivation_at_the_callee() {
    let source = r#"module spec.cpi.elisiontemp;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(x: str, h: Box): str {
    return h.v;
}

func mid(s: str): str {
    return pick(s, Box { v: stringOf("aa") });
}

func main(): i32 {
    let r: str = mid("qq");
    let c: String = stringOf(r);
    return i32(len(c)) - 2;
}
"#;
    compile_rejects("elisiontemp", source, "E10036", 1);
}

// ---------------------------------------------------------------------------
// viewarraymixed: a view array whose elements derive from different owners.
//
// Every element records its own loan, so an element list that mixes a view
// carrying an existing loan with a fresh place projection protects both
// owners: moving either backing while the array is live reports the
// move-while-borrowed error. A shared-loan shortcut that reads only one
// element's loan would leave the other owner movable and dangling.
// ---------------------------------------------------------------------------

/// Moving a view array's second element's backing reports the move error.
///
/// The first element is a view that already carries a loan of its own; the
/// second is a fresh projection of another place. Both owners are borrowed,
/// so moving the second owner reports one `E10022` at the move.
#[test]
fn should_report_the_mixed_view_arrays_place_backing_move() {
    let source = r#"module spec.cpi.viewarraymixed;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func main(): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("ww") };
    let b: Box = Box { v: stringOf("bb"), w: stringOf("xx") };
    let v: str = a.v;
    let vs: str[2] = [v, b.w];
    let c: Box = b;
    return i32(len(vs[1])) + i32(len(c.w)) - 4;
}
"#;
    compile_rejects("viewarraymixed", source, "E10022", 1);
}

// ---------------------------------------------------------------------------
// viewarrayfield: a view array stored in a struct field escapes.
//
// Storing a view in a field leaves a reference behind when the backing's
// scope ends, so a struct literal whose field value carries a loan reports
// the escape at that field instead of leaving the aggregate dangling.
// ---------------------------------------------------------------------------

/// A temporary-backed view array stored in a returned struct field reports.
#[test]
fn should_reject_a_view_arrays_escape_into_a_struct_field() {
    let source = r#"module spec.cpi.viewarrayfield;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct H { names: str[1]; }

func mk(): Box {
    return Box { v: stringOf("ab") };
}

func make(): H {
    let vs: str[1] = [mk().v];
    return H { names: vs };
}

func main(): i32 {
    return 0;
}
"#;
    compile_rejects("viewarrayfield", source, "E10033", 1);
}
