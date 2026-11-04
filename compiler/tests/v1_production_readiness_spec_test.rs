//! Runnable specification tests for the v1.0 production-readiness residual fixes.
//!
//! The fixtures come from the programme's ST-1 – ST-6 cases. ST-1 compiles three
//! independent null programs: the annotated `bool` control already reports the
//! frozen null rule, while the explicit cast and the negated condition must
//! switch from their old diagnostics to the same rule. ST-2 – ST-5 build and run
//! accepted release-root shapes; under `RDC_ASAN` a missing release is a leak, so
//! the red phase fails before the fix and the green phase fails on any regression.
//! ST-6 pins the accepted controls and proves the fixes did not broaden
//! acceptance.
//!
//! The ASan helper links its children with `-no-pie`. The SDK image's GCC 12
//! ASan runtime can otherwise enter a `DEADLYSIGNAL` loop at startup for PIE
//! executables under the container's ASLR; the non-PIE link is stable.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one emitted program may run before the case fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(120);

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
            "clc-v1pr-{}-{tag}-{sequence}",
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

/// Compiles `source` into `scratch` and returns the result and C path.
fn compile_into(tag: &str, scratch: &ScratchDir, source: &str) -> (Output, PathBuf) {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            c_path.to_str().expect("C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    (output, c_path)
}

/// Compiles `source` and returns the scratch directory, result, and C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let (output, c_path) = compile_into(tag, &scratch, source);
    (scratch, output, c_path)
}

/// Emits an accepted host program and returns the C path inside `scratch`.
fn emit_host(tag: &str, scratch: &ScratchDir, source: &str) -> PathBuf {
    let (output, c_path) = compile_into(tag, scratch, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    c_path
}

/// Builds one emitted host translation unit under the strict warning set.
fn build_host_c(tag: &str, scratch: &ScratchDir, c_path: &Path) -> PathBuf {
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
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

/// Builds one emitted host translation unit with AddressSanitizer.
///
/// The `-no-pie` link avoids the SDK image's PIE/ASan startup failure; see the
/// module documentation for the measured reason.
fn build_asan_c(tag: &str, scratch: &ScratchDir, c_path: &Path) -> PathBuf {
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
    program
}

/// Runs a built program with a time limit and the given environment values.
///
/// The child writes to regular files rather than pipes: an ASan child can
/// inherit a pipe's write end and keep the reader from seeing EOF. A program
/// that is still running when the limit passes is killed and the case fails.
fn run_with_timeout(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let capture = ScratchDir::new(&format!("run-{tag}"));
    let stdout_path = capture.path().join("stdout");
    let stderr_path = capture.path().join("stderr");
    let stdout_file = File::create(&stdout_path)
        .unwrap_or_else(|err| panic!("{tag}: cannot create stdout capture: {err}"));
    let stderr_file = File::create(&stderr_path)
        .unwrap_or_else(|err| panic!("{tag}: cannot create stderr capture: {err}"));
    let mut command = Command::new(program);
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
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
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{tag}: cannot wait for the program: {err}");
            }
        }
    }
    let status = child
        .wait()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program status: {err}"));
    let stdout = std::fs::read(&stdout_path).unwrap_or_default();
    let stderr = std::fs::read(&stderr_path).unwrap_or_default();
    Output {
        status,
        stdout,
        stderr,
    }
}

/// Emits, strictly builds, and runs one accepted host program.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let c_path = emit_host(tag, &scratch, source);
    let program = build_host_c(tag, &scratch, &c_path);
    run_with_timeout(tag, &program, &[])
}

/// Emits, builds with AddressSanitizer, and runs one accepted program.
fn run_asan(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let c_path = emit_host(tag, &scratch, source);
    let program = build_asan_c(tag, &scratch, &c_path);
    // The module-level fixtures keep the selected root reachable from a global
    // while the unselected root may remain in a stack or register temporary.
    // LSan's default stack/register scans would treat that leaked temporary as
    // live, so ignore those roots; globals stay visible, keeping the
    // intentional selected root clean.
    run_with_timeout(
        tag,
        &program,
        &[
            ("ASAN_OPTIONS", "detect_leaks=1:symbolize=0"),
            ("LSAN_OPTIONS", "use_stacks=0:use_registers=0"),
        ],
    )
}

/// Runs a memory case under AddressSanitizer when `RDC_ASAN` is set, otherwise
/// as an ordinary strict host build.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
}

/// Asserts an accepted program's exit code.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit code {expected}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts the run produced no AddressSanitizer or LeakSanitizer report.
fn assert_no_sanitizer(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer") && !stderr.contains("LeakSanitizer"),
        "{tag}: the run reported a sanitizer error; stderr:\n{stderr}"
    );
}

/// Compiles one null program and asserts that it reports exactly the null rule.
fn assert_null_rule(tag: &str, source: &str) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected E10012, but the front end accepted the program"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "{tag}: expected exactly one E10012 report; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("error E10201:"),
        "{tag}: the old cast diagnostic must not survive; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("error E10218:"),
        "{tag}: the old condition diagnostic must not survive; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not write a translation unit"
    );
}

/// ST-1 control: an annotated `bool` already reports the null rule.
#[test]
fn should_report_e10012_for_an_annotated_null_bool() {
    let source = r#"module spec.v1pr.nullcontrol;

func main(): i32 {
    let b: bool = null;
    return 0;
}
"#;
    assert_null_rule("st1-control", source);
}

/// ST-1: the explicit cast must report the null rule instead of E10201.
#[test]
fn should_report_e10012_for_a_bool_null_cast() {
    let source = r#"module spec.v1pr.nullcast;

func main(): i32 {
    let b: bool = bool(null);
    return 0;
}
"#;
    assert_null_rule("st1-cast", source);
}

/// ST-1: the negated null condition must report the null rule instead of E10218.
#[test]
fn should_report_e10012_for_a_negated_null_condition() {
    let source = r#"module spec.v1pr.nullnot;

func main(): i32 {
    let b: bool = !null;
    return 0;
}
"#;
    assert_null_rule("st1-not", source);
}

/// ST-2: a module-level nested fill must release the unselected row.
#[test]
fn should_release_a_nested_fill_root_at_module_scope() {
    let source = r#"module spec.v1pr.nestedfill;

import { stringOf } from std.text;

struct Grid { rows: str[2][2]; }

let first: str = Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0];

func main(): i32 {
    return i32(len(first)) - 2;
}
"#;
    let output = run_memory("st2-nested-fill", source);
    assert_exit("st2-nested-fill", &output, 0);
    assert_no_sanitizer("st2-nested-fill", &output);
}

/// ST-3: a multi-element view array must retain and release both element roots.
#[test]
fn should_release_the_unselected_root_of_a_multi_element_view_array() {
    let source = r#"module spec.v1pr.multielement;

import { String, stringOf } from std.text;

struct H { v: String; w: String; }
struct Box { names: str[2]; tag: String; }

func mk(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let picked: str = Box { names: [mk().v, mk().w], tag: stringOf("t") }.names[0];

func main(): i32 {
    return i32(len(picked)) - 2;
}
"#;
    let output = run_memory("st3-multielement", source);
    assert_exit("st3-multielement", &output, 0);
    assert_no_sanitizer("st3-multielement", &output);
}

/// ST-4: a local initializer field call must release the sibling element.
#[test]
fn should_release_the_sibling_of_a_field_call_root() {
    let source = r#"module spec.v1pr.fieldcall;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let b: Box = Box { n: sink(mk().xs[0]) };
    return b.n - 2;
}
"#;
    let output = run_memory("st4-field-call", source);
    assert_exit("st4-field-call", &output, 0);
    assert_no_sanitizer("st4-field-call", &output);
}

/// ST-4b: the func-typed callee twin must release the sibling element.
#[test]
fn should_release_the_sibling_of_a_func_typed_field_call_root() {
    let source = r#"module spec.v1pr.fieldcallfunc;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let f: func(str): i32 = sink;
    let b: Box = Box { n: f(mk().xs[0]) };
    return b.n - 2;
}
"#;
    let output = run_memory("st4b-field-call-func", source);
    assert_exit("st4b-field-call-func", &output, 0);
    assert_no_sanitizer("st4b-field-call-func", &output);
}

/// ST-5: an indexed module view must release the unselected slot.
#[test]
fn should_release_the_unselected_slot_of_an_indexed_module_view() {
    let source = r#"module spec.v1pr.indexedview;

import { String, stringOf } from std.text;

func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
func pick(): usize { return 1; }

let selected: str = mk()[pick()];

func main(): i32 {
    return i32(len(selected)) - 2;
}
"#;
    let output = run_memory("st5-indexed-view", source);
    assert_exit("st5-indexed-view", &output, 0);
    assert_no_sanitizer("st5-indexed-view", &output);
}

/// ST-6: the accepted controls stay accepted and sanitizer clean.
#[test]
fn should_accept_the_control_programs() {
    let source = r#"module spec.v1pr.controls;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let s: String? = null;
    if (s == null) {
        let xs: u8[2] = [1, 2];
        let ys: u8[] = slice(xs, 0, 2);
        let names: str[2] = [; stringOf("x")];
        let rows: str[2][2] = [["aa", "bb"], ["cc", "dd"]];
        return i32(xs[0]) + i32(ys[1]) + i32(len(names[0])) + i32(len(rows[0][0])) - 6;
    }
    return 1;
}
"#;
    let output = run_memory("st6-controls", source);
    assert_exit("st6-controls", &output, 0);
    assert_no_sanitizer("st6-controls", &output);
}
