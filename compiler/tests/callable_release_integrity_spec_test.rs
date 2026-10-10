//! Runnable specification tests for callable release integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. The memory helper
//! runs a case as an ordinary strict host build and adds leak and
//! use-after-free detection when `RDC_ASAN` is set.
//!
//! The identity group pins the three generated-identity cases: a source local
//! spelled like a generated owner stays an ordinary local, two `func`
//! signatures of one generic struct are two instances, and an inline
//! capturing lambda builds with one environment definition.

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
            "clc-cri-{}-{tag}-{sequence}",
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

/// A source local spelled like a generated owner stays an ordinary local.
///
/// The ownership layer binds the borrowed argument's owned temporary in a
/// generated slot; the user's local of the same spelling must keep its own
/// identity, so the program runs one release per value (red at the pre-fix
/// head: the collision double-frees under the sanitizer).
#[test]
fn should_keep_a_generated_owner_distinct_from_a_source_local() {
    let source = r#"module spec.cri.identity;

import { String, stringOf } from std.text;

func use(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let __cl_owned0: String = stringOf("q");
    let n: i32 = use(stringOf("w"));
    return n + i32(len(__cl_owned0)) - 2;
}
"#;
    let output = run_memory("identity", source);
    assert_no_asan("identity", &output);
    assert_exit("identity", &output, 0);
}

/// Two `func` signatures of one generic struct are two instances.
///
/// `Holder<func(i32): i32>` and `Holder<func(u32): u32>` have distinct C
/// representations, so the translation unit defines two structs and builds
/// (red at the pre-fix head: both instances share the `fn` key and the strict
/// build fails on the redefinition).
#[test]
fn should_build_two_func_signature_instances_of_one_generic_struct() {
    let source = r#"module spec.cri.funckey;

struct Holder<T> {
    call: T;
}

func addOne(v: i32): i32 {
    return v + 1;
}

func addTwo(v: u32): u32 {
    return v + 2;
}

func main(): i32 {
    let a: Holder<func(i32): i32> = Holder<func(i32): i32> { call: addOne };
    let b: Holder<func(u32): u32> = Holder<func(u32): u32> { call: addTwo };
    return a.call(2) + i32(b.call(2)) - 7;
}
"#;
    let output = run_host("funckey", source);
    assert_exit("funckey", &output, 0);
}

/// Two mode-differing func signatures of one generic struct are two instances.
///
/// A `borrow` parameter reaches the callee as a pointer, so `func(String):
/// i32` and `func(borrow String): i32` are different types with different C
/// representations: each instance builds and resolves its own callable.
#[test]
fn should_build_two_mode_differing_func_instances_of_one_generic_struct() {
    let source = r#"module spec.cri.modekey;

import { String, stringOf } from std.text;

struct Holder<T> {
    call: T;
}

func lenOf(s: String): i32 {
    return i32(len(s));
}

func lendOf(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let a: Holder<func(String): i32> = Holder<func(String): i32> { call: lenOf };
    let b: Holder<func(borrow String): i32> = Holder<func(borrow String): i32> { call: lendOf };
    return a.call(stringOf("ab")) + b.call(stringOf("cde")) - 5;
}
"#;
    let output = run_host("modekey", source);
    assert_exit("modekey", &output, 0);
}

/// A capturing lambda in a loop releases a temporary its body borrows.
///
/// The speculative loop pass suppresses owned-temporary materialization, so
/// its lifted record must never be the emitted one: the reporting pass's
/// record binds the argument's owner and releases it once per call (red at
/// the pre-fix head: the emitted speculative body leaks under the sanitizer).
#[test]
fn should_release_a_temporary_inside_a_loop_lambda() {
    let source = r#"module spec.cri.loopowned;

import { String, stringOf } from std.text;

func use(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: String = stringOf("ab");
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        total = total + ([s] () => i32(use(stringOf("w"))))();
        i = i + 1;
    }
    return total - 2;
}
"#;
    let output = run_memory("loopowned", source);
    assert_no_asan("loopowned", &output);
    assert_exit("loopowned", &output, 0);
}

/// A capturing lambda in a loop body builds with one definition set.
///
/// The ownership layer's speculative loop pass must not emit a second lifted
/// function for the same site: the translation unit defines the environment
/// struct, the closure typedef, and the lifted function once (red at the
/// pre-fix head: the strict build fails on the redefinitions).
#[test]
fn should_build_a_capturing_lambda_in_a_loop_once() {
    let source = r#"module spec.cri.envdedupe;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("ab");
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        total = total + ([s] () => i32(len(s)))();
        i = i + 1;
    }
    return total - 4;
}
"#;
    let output = run_host("envdedupe", source);
    assert_exit("envdedupe", &output, 0);
}
