//! Runnable specification tests for the residual value-crossing defect fixes.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The chosen value travels in
//! `main`'s return code, so a case that prints nothing states its answer as the
//! exit code.
//!
//! A case whose program is accepted but whose emitted C the host compiler
//! rejects states the defect directly: the emitted translation must build. A
//! case that runs also pins release-once behavior, because a double release
//! aborts in the C library before the exit code is seen.
//!
//! The literal-type and projection cases pin program-visible behavior only;
//! their memory-cleanliness claims are pinned by the implementation tests and
//! the close-out sanitizer probes, which this suite does not run.

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
            "clc-rdcspec-{}-{tag}-{sequence}",
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

/// Two distinct fixed-array type arguments must get distinct C structs.
///
/// `Grid<i32[2]>` and `Grid<i32[3]>` are different types; the emitted C must
/// define one struct per instantiation and build. The exit code `2` proves both
/// first elements were read from the right rows.
#[test]
fn should_build_when_two_fixed_array_instances_coexist() {
    let source = r#"module spec.rdc.st1;

struct Grid<T> {
    value: T;
}

func main(): i32 {
    let a: Grid<i32[2]> = Grid<i32[2]> { value: [1, 2] };
    let b: Grid<i32[3]> = Grid<i32[3]> { value: [1, 2, 3] };
    return a.value[0] + b.value[0];
}
"#;
    let output = run_host("rdc01", source);
    assert_exit("rdc01", &output, 2);
}

/// Two fixed-array arguments with different extent lists must not share a name.
///
/// `Grid<i32[2][2]>` and `Grid<i32[2]>` differ in extents alone; the emitted C
/// must define both structs and build. The exit code `10` is
/// `g.value[1][1] + h.value[1]`, so both layouts were read correctly.
#[test]
fn should_build_when_nested_and_single_extent_instances_coexist() {
    let source = r#"module spec.rdc.st2;

struct Grid<T> {
    value: T;
}

func main(): i32 {
    let g: Grid<i32[2][2]> = Grid<i32[2][2]> { value: [[1, 2], [3, 4]] };
    let h: Grid<i32[2]> = Grid<i32[2]> { value: [5, 6] };
    return g.value[1][1] + h.value[1];
}
"#;
    let output = run_host("rdc02", source);
    assert_exit("rdc02", &output, 10);
}

/// An owned fixed-array element type must build and release its elements.
///
/// `Grid<String[2]>` holds two owned strings; the exit code `3` is the summed
/// lengths (`"ab"` plus `"c"`). A double release would abort before the exit.
#[test]
fn should_run_when_an_owned_fixed_array_instance_is_used() {
    let source = r#"module spec.rdc.st3;

import { String, stringOf } from std.text;

struct Grid<T> {
    value: T;
}

func main(): i32 {
    let g: Grid<String[2]> = Grid<String[2]> { value: [stringOf("ab"), stringOf("c")] };
    return i32(len(g.value[0])) + i32(len(g.value[1]));
}
"#;
    let output = run_host("rdc03", source);
    assert_exit("rdc03", &output, 3);
}

/// A generic-struct borrow argument must render with its substituted type.
///
/// `probe<String>` lends its parameter `p: Outer<String>` and passes `p.v`, of
/// type `Inner<String>`, to `sink<String>`. The substituted parameter lowers to
/// a pointer, so the emitted C must pass the projection's address and build.
/// The exit code `1` is the callee's constant.
#[test]
fn should_build_when_a_substituted_borrow_argument_passes() {
    let source = r#"module spec.rdc.st4;

import { String, stringOf } from std.text;

struct Inner<T> {
    s: T;
}

struct Outer<T> {
    v: Inner<T>;
}

func sink<T>(borrow i: Inner<T>): i32 {
    return 1;
}

func probe<T>(borrow p: Outer<T>): i32 {
    return sink<T>(borrow p.v);
}

func main(): i32 {
    let p: Outer<String> = Outer<String> { v: Inner<String> { s: stringOf("x") } };
    return probe<String>(borrow p);
}
"#;
    let output = run_host("rdc04", source);
    assert_exit("rdc04", &output, 1);
}

/// A substituted borrow argument must stay a view of the caller's place.
///
/// The same shape runs with `T = i32`; after the call the caller reads `p.v.s`
/// again. The exit code `78` is `before * 10 + result + after`: the stored
/// value seven survives the call unchanged on both sides, and the callee
/// returned one. A move or release through the argument would change or abort
/// the run.
#[test]
fn should_keep_the_argument_readable_when_a_substituted_borrow_argument_passes() {
    let source = r#"module spec.rdc.st5;

struct Inner<T> {
    s: T;
}

struct Outer<T> {
    v: Inner<T>;
}

func sink<T>(borrow i: Inner<T>): i32 {
    return 1;
}

func probe<T>(borrow p: Outer<T>): i32 {
    return sink<T>(borrow p.v);
}

func main(): i32 {
    let p: Outer<i32> = Outer<i32> { v: Inner<i32> { s: 7 } };
    let before: i32 = p.v.s;
    let result: i32 = probe<i32>(borrow p);
    let after: i32 = p.v.s;
    return before * 10 + result + after;
}
"#;
    let output = run_host("rdc05", source);
    assert_exit("rdc05", &output, 78);
}

/// Emitting the same program twice must produce identical C.
///
/// The symbol keys that name fixed-array instances must be deterministic; the
/// two translations are compared byte for byte. Both runs name the source with
/// the same relative path, because the emitted `#line` directives carry the
/// command-line path.
#[test]
fn should_emit_identical_translations_when_the_same_program_is_emitted_twice() {
    let source = r#"module spec.rdc.st6;

struct Grid<T> {
    value: T;
}

func main(): i32 {
    let a: Grid<i32[2]> = Grid<i32[2]> { value: [1, 2] };
    let b: Grid<i32[3]> = Grid<i32[3]> { value: [1, 2, 3] };
    return a.value[0] + b.value[0];
}
"#;
    let scratch = ScratchDir::new("rdc06");
    std::fs::write(scratch.path().join("main.clo"), source).expect("write the program");
    let emit = |tag: &str, output_name: &str| -> String {
        let output_path = scratch.path().join(output_name);
        let output = Command::new(env!("CARGO_BIN_EXE_clc"))
            .current_dir(scratch.path())
            .args([
                "--profile".to_owned(),
                "host".to_owned(),
                "-o".to_owned(),
                output_path.display().to_string(),
                "main.clo".to_owned(),
            ])
            .output()
            .expect("failed to spawn the compiler binary");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{tag}: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(&output_path).unwrap_or_default()
    };
    let first = emit("rdc06a", "first.c");
    let second = emit("rdc06b", "second.c");
    assert!(
        !first.trim().is_empty(),
        "rdc06: the emitted C is empty, so the emitter is not implemented yet"
    );
    assert_eq!(
        first, second,
        "rdc06: the same program emitted two different translations"
    );
}
