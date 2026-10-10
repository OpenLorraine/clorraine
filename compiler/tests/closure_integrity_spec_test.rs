//! Runnable specification tests for closure and callable integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. Programs that must
//! be rejected assert the diagnostics instead, and no translation unit may be
//! written for them.
//!
//! The capture-listing group pins the validity of capture lists: a nested
//! lambda's entry is judged against the enclosing lambda's list (the fix is
//! to add the name there), a module-level name is rejected because it needs
//! no capture, and a dotted capture write reports the capture-write rule
//! rather than the read-only-parameter rule, while the borrow-capture write
//! and the copy-capture read stay accepted.
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
            "clc-closureintegrity-{}-{tag}-{sequence}",
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
///
/// Used by the golden-program cases: the file under `spec/` is compiled where
/// it lives, so the case asserts the frozen fixture itself.
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

/// A nested lambda's entry for a name the enclosing lambda does not list is
/// rejected once.
///
/// The inner lambda captures `m`, but `m` is a function local the enclosing
/// lambda does not carry, so the relay has nothing to travel through. The
/// listing rule reports the entry and the fix is to add `m` to the outer
/// list; exactly one `E10405` names `m` and no translation unit is written.
#[test]
fn should_reject_a_nested_relay_without_the_enclosing_capture() {
    let source = r#"module spec.ci.relaymissing;

func main(): i32 {
    let n: i32 = 2;
    let m: i32 = 3;
    return ([n] () => ([m] () => m)())();
}
"#;
    assert_rejected_once("relay-missing", source, "E10405", &["'m'"], &[]);
}

/// The same relay with the enclosing list carrying the name builds and runs.
#[test]
fn should_build_and_run_a_nested_relay_with_the_enclosing_capture() {
    let source = r#"module spec.ci.relaylisted;

func main(): i32 {
    let n: i32 = 2;
    let m: i32 = 3;
    return ([n, m] () => ([m] () => m)())();
}
"#;
    let output = run_host("relay-listed", source);
    assert_exit("relay-listed", &output, 3);
}

/// A capture entry naming a module-level `let` is rejected once.
///
/// The binding is visible inside the lambda without capture, so the entry is
/// meaningless; the new rule reports it and asks for the direct use.
#[test]
fn should_reject_a_module_let_capture_entry() {
    let source = r#"module spec.ci.modulelet;

let g: i32 = 5;

func main(): i32 {
    return ([g] () => g)();
}
"#;
    assert_rejected_once("module-let-entry", source, "E10406", &["'g'"], &[]);
}

/// A capture entry naming a module-level `const` is rejected once.
#[test]
fn should_reject_a_module_const_capture_entry() {
    let source = r#"module spec.ci.moduleconst;

const base: i32 = 7;

func main(): i32 {
    return ([base] () => base)();
}
"#;
    assert_rejected_once("module-const-entry", source, "E10406", &["'base'"], &[]);

    let imported = r#"module spec.ci.importentry;

import { stringOf } from std.text;

func main(): i32 {
    return ([stringOf] () => 0)();
}
"#;
    assert_rejected_once("import-entry", imported, "E10406", &["'stringOf'"], &[]);
}

/// Module-level function and type names are rejected as capture entries too.
///
/// The rule covers every module-level name: a function or type name is
/// visible without capture, so listing it is meaningless. The documented fix
/// — drop the list and use the name directly — keeps the program working.
#[test]
fn should_reject_module_function_and_type_capture_entries() {
    let function_entry = r#"module spec.ci.modulefunc;

func helper(): i32 {
    return 4;
}

func main(): i32 {
    return ([helper] () => helper())();
}
"#;
    assert_rejected_once("module-func-entry", function_entry, "E10406", &["'helper'"], &[]);

    let type_entry = r#"module spec.ci.moduletype;

struct Point {
    x: i32;
}

func main(): i32 {
    return ([Point] () => 1)();
}
"#;
    assert_rejected_once("module-type-entry", type_entry, "E10406", &["'Point'"], &[]);

    let fixed = r#"module spec.ci.moduleread;

func helper(): i32 {
    return 4;
}

func main(): i32 {
    return (() => helper())();
}
"#;
    let output = run_host("module-func-read", fixed);
    assert_exit("module-func-read", &output, 4);
}

/// Reading a module-level name without a capture list stays accepted.
#[test]
fn should_accept_a_captureless_module_name_read() {
    let source = r#"module spec.ci.moduleplain;

const base: i32 = 7;

func main(): i32 {
    return (() => base)();
}
"#;
    let output = run_host("module-plain-read", source);
    assert_exit("module-plain-read", &output, 7);
}

/// A dotted write through a copy capture reports the write rule once.
///
/// The capture is a copy, so the write would reach only the copy; the
/// capture-write rule reports `E10402` and the read-only-parameter rule must
/// not fire for the same site.
#[test]
fn should_reject_a_dotted_copy_capture_write() {
    let source = r#"module spec.ci.dottedwrite;

struct P {
    x: i32;
}

func main(): i32 {
    let p: P = P { x: 1 };
    ([p] () => { p.x = 2; })();
    return p.x;
}
"#;
    assert_rejected_once(
        "dotted-copy-write",
        source,
        "E10402",
        &["'p'"],
        &["error E10024:"],
    );
}

/// A dotted write through a borrow capture reaches the owner and runs.
#[test]
fn should_accept_a_dotted_borrow_capture_write() {
    let source = r#"module spec.ci.dottedborrow;

struct P {
    x: i32;
}

func main(): i32 {
    let p: P = P { x: 1 };
    ([borrow p] () => { p.x = 2; })();
    return p.x;
}
"#;
    let output = run_host("dotted-borrow-write", source);
    assert_exit("dotted-borrow-write", &output, 2);
}

/// A dotted read through a copy capture stays accepted.
#[test]
fn should_accept_a_copy_capture_read() {
    let source = r#"module spec.ci.dottedread;

struct P {
    x: i32;
}

func main(): i32 {
    let p: P = P { x: 1 };
    return ([p] () => p.x)();
}
"#;
    let output = run_host("copy-capture-read", source);
    assert_exit("copy-capture-read", &output, 1);
}

/// The module-capture golden is rejected exactly as the frozen suite expects.
#[test]
fn should_reject_the_module_capture_golden_directly() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../spec/tests/compile-fail/E10406_module_capture.clo");
    assert_rejected_once_at("module-capture-golden", &golden, "E10406", &["'base'"], &[]);
}

/// The generic captured-owner return is rejected once when the instance owns.
///
/// A bare `T` capture copies at the declaration, but an owning substitution
/// makes the copy alias the enclosing binding, so the return would release
/// the same storage twice. The per-instance judgment reports the same
/// diagnostic the concrete spelling does: exactly one `E10401` naming `x`.
#[test]
fn should_reject_the_generic_captured_owner_return_when_the_instance_owns() {
    let source = r#"module spec.ci.genericowner;

import { String, stringOf } from std.text;

struct Wrap {
    v: String;
}

func outer<T>(x: T): T {
    return ([x] () => x)();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("abcd") };
    let r: Wrap = outer<Wrap>(w);
    return i32(len(r.v));
}
"#;
    assert_rejected_once("generic-owner-return", source, "E10401", &["'x'"], &[]);
}

/// The concrete captured-owner return keeps its single report.
#[test]
fn should_reject_the_concrete_captured_owner_return() {
    let source = r#"module spec.ci.concreteowner;

import { String, stringOf } from std.text;

struct Wrap {
    v: String;
}

func outer(x: Wrap): Wrap {
    return ([x] () => x)();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("abcd") };
    let r: Wrap = outer(w);
    return i32(len(r.v));
}
"#;
    assert_rejected_once("concrete-owner-return", source, "E10401", &["'x'"], &[]);
}

/// The same generic return with a duplicable instance is accepted and runs.
#[test]
fn should_accept_and_run_the_captured_owner_return_when_the_instance_copies() {
    let source = r#"module spec.ci.genericcopy;

func outer<T>(x: T): T {
    return ([x] () => x)();
}

func main(): i32 {
    return outer<i32>(42);
}
"#;
    let output = run_host("generic-copy-return", source);
    assert_exit("generic-copy-return", &output, 42);
}

/// A called func-typed capture named like a module lender releases once.
///
/// The local `run` shares its name with the module function, but the call
/// goes through the captured value; the owned result must be bound and
/// released like any other temporary, so the run reads the result's length
/// and the sanitizer stays quiet.
#[test]
fn should_accept_and_run_a_called_func_typed_capture_named_like_a_lender() {
    let source = r#"module spec.ci.lendercapture;

import { String, stringOf } from std.text;

func run(borrow s: String): String {
    return stringOf("mod");
}

func main(): i32 {
    let run: func(borrow String): String = run;
    let r: String = ([run] () => run(stringOf("ab")))();
    return i32(len(r));
}
"#;
    let output = run_memory("lender-capture", source);
    assert_exit("lender-capture", &output, 3);
    assert_no_asan("lender-capture", &output);
}
