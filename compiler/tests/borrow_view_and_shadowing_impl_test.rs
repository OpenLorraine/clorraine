//! Implementation tests for the reporting shape of the borrow-view move rule
//! and the shadowed-narrowing resolution order.
//!
//! The specification suite proves that a move through a `for (borrow e in xs)`
//! view is refused with the element-move diagnostic, that a move out of a
//! `borrow` parameter keeps its own diagnostic, and that a shadowing
//! initializer under a null narrowing compiles. These cases pin the reporting
//! and resolution details: one transfer produces one report, the parameter rule
//! reports its own code rather than the element-move code, the null diagnostic
//! still reports once per due use, the shadowing binding is usable after its
//! declaration, and the `null`-against-non-nullable rule is unchanged.
//!
//! The move cases are compile-time rejections; the narrowing batch adds a
//! runtime case that reads the shadowing binding, so the helper set includes a
//! build-and-run path.

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
            "clc-bvsimp-{}-{tag}-{sequence}",
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

/// Compiles `source` in the host profile and returns the compiler output.
///
/// The scratch directory travels with the output so the caller keeps the
/// program file alive while it reads the diagnostics.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output) {
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
    (scratch, output)
}

/// Counts the reports of one error code in the compiler's stderr.
fn count_reports(stderr: &str, code: &str) -> usize {
    stderr.matches(&format!("error {code}:")).count()
}

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit. A rejection is a failure of the case,
/// not a result to assert, because the runtime cases are about behavior.
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

/// One transfer through a view reports the element-move limit exactly once.
///
/// The `take` call is the only move in the program, so a second report would
/// mean the same transfer is diagnosed twice.
#[test]
fn should_report_a_single_element_move_when_a_take_argument_reads_a_borrow_view() {
    let source = r#"module codegen.bvs.imp1;

import { String, stringOf } from std.text;

func eat(take s: String): void { }

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    for (borrow e in xs) {
        eat(e);
    }
    return 0;
}
"#;
    let (_scratch, output) = compile_only("bvsimp1", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp1: the front end accepted a move through a borrow view"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10026"),
        1,
        "bvsimp1: expected exactly one E10026 report; stderr:\n{stderr}"
    );
}

/// A field path out of a view also reports the element-move limit once.
///
/// The projected path is a different route into the move rule than a bare
/// name, and it must not add a report on top of the element-move one.
#[test]
fn should_report_a_single_element_move_when_a_field_path_leaves_a_borrow_view() {
    let source = r#"module codegen.bvs.imp2;

import { String, stringOf } from std.text;

struct P { name: String; n: i32; }

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    for (borrow p in ps) {
        let s: String = p.name;
        if (len(s) == 0) { return 1; }
    }
    return 0;
}
"#;
    let (_scratch, output) = compile_only("bvsimp2", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp2: the front end accepted a field move through a borrow view"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10026"),
        1,
        "bvsimp2: expected exactly one E10026 report; stderr:\n{stderr}"
    );
}

/// A move out of a `borrow` parameter keeps the parameter-specific report.
///
/// The view guard must not swallow the parameter rule: this program still
/// reports the borrow-move diagnostic and never the element-move one.
#[test]
fn should_keep_the_borrow_parameter_move_when_a_take_argument_reads_a_borrow_parameter() {
    let source = r#"module codegen.bvs.imp3;

import { String, stringOf } from std.text;

func eat(take s: String): void { }

func demo(borrow s: String): void {
    eat(s);
}

func main(): i32 { return 0; }
"#;
    let (_scratch, output) = compile_only("bvsimp3", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp3: the front end accepted a move out of a borrow parameter"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10034"),
        1,
        "bvsimp3: expected exactly one E10034 report; stderr:\n{stderr}"
    );
    assert_eq!(
        count_reports(&stderr, "E10026"),
        0,
        "bvsimp3: the parameter rule must not report the element-move code; stderr:\n{stderr}"
    );
}

/// One un-narrowed use reports the null error once.
///
/// The single call is the only nullable use in the program, so a second report
/// would mean the initializer reorder double-reports.
#[test]
fn should_report_a_single_null_use_when_one_un_narrowed_use_is_due() {
    let source = r#"module codegen.bvs.imp4;

import { Buffer, bufferNew } from std.mem;

func use(b: Buffer): i32 { return 0; }

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    return use(b);
}
"#;
    let (_scratch, output) = compile_only("bvsimp4", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp4: the front end accepted an un-narrowed nullable use"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10010"),
        1,
        "bvsimp4: expected exactly one E10010 report; stderr:\n{stderr}"
    );
}

/// The shadowing binding stays usable after its declaration.
///
/// The inner `b` owns the buffer the outer binding held; reading its length
/// through the standard-library call returns the eight bytes the buffer was
/// created with.
#[test]
fn should_keep_the_shadowing_binding_usable_when_it_is_read_after_declaration() {
    let source = r#"module codegen.bvs.imp5;

import { Buffer, bufferNew, bufferLen } from std.mem;

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    if (b != null) {
        let b: Buffer = b;
        return i32(bufferLen(b));
    }
    return 1;
}
"#;
    let output = run_host("bvsimp5", source);
    assert_exit("bvsimp5", &output, 8);
}

/// A null initializer for a non-nullable binding still reports the null rule.
///
/// Moving the check earlier must not lose the `null` rejection: `null` against
/// a declared non-nullable type reports the null-initializer error once.
#[test]
fn should_report_the_null_initializer_when_a_non_nullable_binding_is_declared() {
    let source = r#"module codegen.bvs.imp6;

import { Buffer } from std.mem;

func main(): i32 {
    let b: Buffer = null;
    return 0;
}
"#;
    let (_scratch, output) = compile_only("bvsimp6", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp6: the front end accepted null for a non-nullable binding"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10012"),
        1,
        "bvsimp6: expected exactly one E10012 report; stderr:\n{stderr}"
    );
}
