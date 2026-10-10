//! Runnable specification tests for evaluation and emission integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. Programs that must
//! be rejected assert the diagnostics instead, and no translation unit may be
//! written for them.
//!
//! The module-initializer group pins the six accepted positions for a
//! fixed-array-returning call (whole value, element and member projection,
//! user-function and intrinsic arguments, conditional arm), the
//! single-evaluation rule for an impure guard operand, and the rejection of a
//! slice-typed binding initialized from an array literal.
//!
//! The memory helper runs a case as an ordinary strict host build and adds
//! leak and use-after-free detection when `RDC_ASAN` is set.

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
            "clc-eei-{}-{tag}-{sequence}",
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

/// Compiles an existing source file for the host profile.
fn compile_path(tag: &str, source_path: &Path) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
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

/// Asserts one compiler run rejected the program with exactly one `code`
/// diagnostic, that the message contains every fragment and none of the
/// absent markers, and that no translation unit is written.
fn assert_rejected_output(
    tag: &str,
    output: &Output,
    c_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = format!("error {code}:");
    assert_eq!(
        stderr.matches(&marker).count(),
        1,
        "{tag}: expected exactly one {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    for absent_marker in absent {
        assert!(
            !stderr.contains(absent_marker),
            "{tag}: the report {absent_marker:?} must be absent; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the rejection contract on the output.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str], absent: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
}

/// Compiles an existing file and asserts the rejection contract on the
/// output.
fn assert_rejected_once_at(
    tag: &str,
    source_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
) {
    let (_scratch, output, c_path) = compile_path(tag, source_path);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
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

/// A fixed-array-returning call as a whole module-binding value builds and
/// runs.
///
/// The initializer stores through the callee's hidden destination, so the
/// emitted C passes the array's destination instead of assigning a call
/// result.
#[test]
fn should_build_and_run_the_module_init_whole_value() {
    let source = r#"module spec.eei.initwhole;

func mk(): i32[2] {
    return [4, 2];
}

let a: i32[2] = mk();

func main(): i32 {
    return a[1];
}
"#;
    let output = run_host("init-whole", source);
    assert_exit("init-whole", &output, 2);
}

/// The element-projection init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_element_projection() {
    let source = r#"module spec.eei.initelement;

func mk(): i32[2] {
    return [7, 2];
}

let b: i32 = mk()[1];

func main(): i32 {
    return b;
}
"#;
    let output = run_host("init-element", source);
    assert_exit("init-element", &output, 2);
}

/// The member-projection init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_member_projection() {
    let source = r#"module spec.eei.initmember;

struct Box {
    arr: i32[2];
}

func mkBox(): Box {
    return Box { arr: [5, 2] };
}

let c: i32[2] = mkBox().arr;

func main(): i32 {
    return c[1];
}
"#;
    let output = run_host("init-member", source);
    assert_exit("init-member", &output, 2);
}

/// The user-function and intrinsic argument positions build and run.
#[test]
fn should_build_and_run_the_module_init_argument_positions() {
    let source = r#"module spec.eei.initargs;

import { slice } from std.mem;

func mk(): i32[2] {
    return [3, 9];
}

func eat(arr: i32[2]): i32 {
    return arr[0] + arr[1];
}

let d: i32 = eat(mk());
let e: i32 = i32(len(slice(mk(), 0, 2)));

func main(): i32 {
    return d + e;
}
"#;
    let output = run_host("init-args", source);
    assert_exit("init-args", &output, 14);
}

/// The conditional-arm init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_conditional_arm() {
    let source = r#"module spec.eei.initcond;

func mk(): i32[2] {
    return [6, 1];
}

func pick(): bool {
    return true;
}

let y: i32[2] = [8, 8];
let f: i32[2] = mk();
let g: i32[2] = pick() ? f : y;

func main(): i32 {
    return g[0];
}
"#;
    let output = run_host("init-cond", source);
    assert_exit("init-cond", &output, 6);
}

/// An impure module-initializer guard operand is evaluated once.
///
/// The index function counts its calls and goes out of range on a second
/// evaluation; the bounds guard and the read must share one evaluation, so
/// the count is one and the read is in bounds (a second evaluation reads out
/// of bounds, which the sanitizer probe observes at the baseline).
#[test]
fn should_evaluate_the_module_init_guard_operand_once() {
    let source = r#"module spec.eei.guardonce;

let calls: i32 = 0;

func idx(): i32 {
    calls = calls + 1;
    if (calls == 1) {
        return 0;
    }
    return 5;
}

let s: u8[2] = [1, 2];
let v: u8 = s[idx()];

func main(): i32 {
    return calls;
}
"#;
    let output = run_host("guard-once", source);
    assert_exit("guard-once", &output, 1);
}

/// A slice-typed binding from an array literal is rejected; the slice form
/// stays accepted.
#[test]
fn should_reject_the_literal_slice_binding_and_accept_the_slice_form() {
    let rejected = r#"module spec.eei.literalslice;

let x: i32[] = [1, 2];

func main(): i32 {
    return i32(len(x));
}
"#;
    assert_rejected_once("literal-slice", rejected, "E10200", &[], &[]);

    let accepted = r#"module spec.eei.slicedform;

import { slice } from std.mem;

let arr: i32[2] = [1, 2];
let y: i32[] = slice(arr, 0, 2);

func main(): i32 {
    return i32(len(y));
}
"#;
    let output = run_host("slice-form", accepted);
    assert_exit("slice-form", &output, 2);
}
