//! Second-pass regression specification tests for the Phase 1 re-review.
//!
//! These cases cover the findings the independent re-review raised after the
//! first remediation: loop-escape safety, a mutable conditional condition,
//! guarded field order, loop-condition ownership, a whole-array projection,
//! non-call element order, and conditional-field selection. The original and
//! first-pass suites remain in `v1_production_readiness_spec_test.rs` and
//! `v1_production_readiness_regression_spec_test.rs`.

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
            "clc-v1pr-review2-{}-{tag}-{sequence}",
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

/// Emits one accepted translation unit and returns its C text.
fn emit_into(tag: &str, scratch: &ScratchDir, source: &str) -> String {
    let (output, c_path) = compile_into(tag, scratch, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    emitted
}

/// Builds one emitted translation unit under the strict host warnings.
fn build_host(tag: &str, scratch: &ScratchDir) -> PathBuf {
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
            scratch.path().join("out.c").to_str().expect("C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the strict host build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Builds one emitted translation unit with AddressSanitizer.
fn build_asan(tag: &str, scratch: &ScratchDir) -> PathBuf {
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
            scratch.path().join("out.c").to_str().expect("C path"),
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

/// Runs a built program with a time limit and the given environment.
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
    Output {
        status,
        stdout: std::fs::read(&stdout_path).unwrap_or_default(),
        stderr: std::fs::read(&stderr_path).unwrap_or_default(),
    }
}

/// Runs an accepted program under ASan when requested, otherwise as strict host.
fn run_memory(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    emit_into(tag, &scratch, source);
    let asan = std::env::var_os("RDC_ASAN").is_some();
    let program = if asan {
        build_asan(tag, &scratch)
    } else {
        build_host(tag, &scratch)
    };
    let env: &[(&str, &str)] = if asan {
        &[
            ("ASAN_OPTIONS", "detect_leaks=1:symbolize=0"),
            ("LSAN_OPTIONS", "use_stacks=0:use_registers=0"),
        ]
    } else {
        &[]
    };
    run_with_timeout(tag, &program, env)
}

/// Asserts that an accepted program exits zero without a sanitizer report.
fn assert_accepts(tag: &str, source: &str) {
    let output = run_memory(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected exit code 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer") && !stderr.contains("LeakSanitizer"),
        "{tag}: the run reported a sanitizer error; stderr:\n{stderr}"
    );
}

/// Asserts that a source is rejected with `code` and writes no translation unit.
fn assert_rejected(tag: &str, source: &str, code: &str) {
    let scratch = ScratchDir::new(tag);
    let (output, c_path) = compile_into(tag, &scratch, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected {code}, but the front end accepted the program"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches(&format!("error {code}:")).count(),
        1,
        "{tag}: expected exactly one {code}; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not write a translation unit"
    );
}

/// RR-2: a mutable conditional condition is stored once before arm binding.
#[test]
fn should_evaluate_a_mutable_conditional_condition_once() {
    assert_accepts(
        "rr2-mutable-condition",
        r#"module spec.v1pr.review2.mutablecondition;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }

let flag: bool = false;
let idx: usize = 0;

func mkThen(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func mkElse(): H { flag = true; return H { xs: [stringOf("cc"), stringOf("dd")] }; }

let g: str = flag ? mkThen().xs[idx] : mkElse().xs[idx];

func main(): i32 { return i32(g[0]) - 99; }
"#,
    );
}

/// RR-3: a guarded array field does not hoist above an earlier field call.
#[test]
fn should_preserve_order_before_a_guarded_array_field() {
    assert_accepts(
        "rr3-guarded-order",
        r#"module spec.v1pr.review2.guardedorder;

import { String, stringOf } from std.text;

struct Box { tag: String; rows: str[2]; }

let c: bool = true;
let order: i32 = 0;

func tagf(): String { order = order * 10 + 1; return stringOf("t"); }
func a(): String { order = order * 10 + 2; return stringOf("aa"); }
func f(): String { order = order * 10 + 3; return stringOf("bb"); }

let g: str = c ? Box { tag: tagf(), rows: [a(); f()] }.rows[0] : stringOf("zz");

func main(): i32 { return order - 123; }
"#,
    );
}

/// RR-4: a lend root in a loop condition releases on every evaluation.
#[test]
fn should_release_a_loop_condition_lend_root() {
    assert_accepts(
        "rr4-loop-condition",
        r#"module spec.v1pr.review2.loopcondition;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let i: i32 = 0;
    while (sink(mk().xs[0]) > 0 && i < 1000) {
        i = i + 1;
    }
    return i - 1000;
}
"#,
    );
}

/// RR-5: a whole-array projection releases the call root's residual fields.
#[test]
fn should_release_a_whole_array_projection_root() {
    assert_accepts(
        "rr5-whole-array",
        r#"module spec.v1pr.review2.wholearray;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; k: String; }

func mkH(): Holder { return Holder { names: ["xy", "zz"], k: stringOf("kk") }; }

let g: str[2] = mkH().names;

func main(): i32 { return i32(g[0][0]) - 120; }
"#,
    );
}

/// RR-6: a place element is read before a later fill mutates it.
#[test]
fn should_read_a_place_element_before_a_following_fill() {
    assert_accepts(
        "rr6-place-element",
        r#"module spec.v1pr.review2.placeelement;

import { String, stringOf } from std.text;

struct Box { rows: str[2]; }

let s: str = "aa";

func f(): String { s = "bb"; return stringOf("cc"); }

let g: str = Box { rows: [s; f()] }.rows[0];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// RR-6: a conditional element is read before a later fill mutates its source.
#[test]
fn should_read_a_conditional_element_before_a_following_fill() {
    assert_accepts(
        "rr6-conditional-element",
        r#"module spec.v1pr.review2.conditionalelement;

import { String, stringOf } from std.text;

struct Box { rows: str[2]; }

let flag: bool = true;
let s: str = "aa";

func f(): String { s = "bb"; return stringOf("cc"); }

let g: str = Box { rows: [flag ? s : "zz"; f()] }.rows[0];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// RR-7: an unselected conditional struct field releases its whole arm root.
#[test]
fn should_release_an_unselected_conditional_struct_field() {
    assert_accepts(
        "rr7-conditional-field",
        r#"module spec.v1pr.review2.conditionalfield;

import { String, stringOf } from std.text;

struct H { v: String; w: String; }
struct Inner { rows: str[2]; }
struct Box { inner: Inner; tag: String; }

let flag: bool = true;

func mk(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let g: str = Box { inner: flag ? Inner { rows: [mk().v, mk().w] } : Inner { rows: ["zz"; mk().v] }, tag: stringOf("t") }.inner.rows[0];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// RR-8: nested conditional arms retain under the composed guard.
#[test]
fn should_release_a_nested_conditional_arm() {
    assert_accepts(
        "rr8-nested-conditional",
        r#"module spec.v1pr.review2.nestedconditional;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }

let flag1: bool = true;
let flag2: bool = false;
let idx: usize = 0;

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }

let g: str = flag1 ? (flag2 ? mk().xs[idx] : mk().xs[idx]) : mk().xs[idx];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// RR-8: a filled member projection releases its fresh root's sibling.
#[test]
fn should_release_a_fill_member_projection() {
    assert_accepts(
        "rr8-fill-member",
        r#"module spec.v1pr.review2.fillmember;

import { String, stringOf } from std.text;

struct H { v: String; w: String; }
struct Box { rows: str[2]; }

func a(): String { return stringOf("aa"); }
func mk(): H { return H { v: stringOf("bb"), w: stringOf("cc") }; }

let g: str = Box { rows: [a(); mk().v] }.rows[1];

func main(): i32 { return i32(g[0]) - 98; }
"#,
    );
}

/// RR-1: a view of a loop-body owner must not escape the loop.
#[test]
fn should_reject_a_view_escaping_a_loop_body() {
    assert_rejected(
        "rr1-loop-escape",
        r#"module spec.v1pr.review2.loopescape;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func head(s: str): str { return s; }

func main(): i32 {
    let v: str = "";
    let i: i32 = 0;
    while (i < 1) {
        v = head(mk().xs[0]);
        i = 1;
    }
    return i32(v[0]) - 97;
}
"#,
        "E10030",
    );
}

/// RR-1: a loop-local view copied to an outer binding is rejected too.
#[test]
fn should_reject_a_loop_local_view_escaping_through_a_binding() {
    assert_rejected(
        "rr1-loop-escape-local",
        r#"module spec.v1pr.review2.loopescapelocal;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func head(s: str): str { return s; }

func main(): i32 {
    let v: str = "";
    let i: i32 = 0;
    while (i < 3) {
        let inner: str = head(mk().xs[0]);
        if (i == 2) { v = inner; }
        i = i + 1;
    }
    return i32(v[0]) - 97;
}
"#,
        "E10030",
    );
}

/// RR-9: a fresh projection element keeps its root instead of taking a copy.
#[test]
fn should_release_a_fresh_projection_element() {
    assert_accepts(
        "rr9-projection-element",
        r#"module spec.v1pr.review2.projectionelement;

import { String, stringOf } from std.text;

struct H { names: str[2]; k: String; }
struct Box { rows: str[2]; }

func mk(): H { return H { names: ["aa", "bb"], k: stringOf("kk") }; }

let g: str = Box { rows: [mk().names[0]; "zz"] }.rows[0];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// RR-1 capture: a write-capture lambda storing a fresh argument is rejected.
#[test]
fn should_reject_a_view_stored_through_a_capturing_lambda() {
    assert_rejected(
        "rr1-capture-escape",
        r#"module spec.v1pr.review2.captureescape;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }

func main(): i32 {
    let v: str = "";
    if (true) {
        ([borrow v] (s: str) => { v = s; })(mk().xs[0]);
    }
    return i32(v[0]) - 97;
}
"#,
        "E10030",
    );
}

/// RR-1 literal: a view nested in an array literal cannot escape a loop.
#[test]
fn should_reject_a_view_in_an_array_literal_escaping_a_loop() {
    assert_rejected(
        "rr1-array-escape",
        r#"module spec.v1pr.review2.arrayescape;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func head(s: str): str { return s; }

func main(): i32 {
    let arr: str[2] = ["x", "y"];
    let i: i32 = 0;
    while (i < 1) { arr = [head(mk().xs[0]), "z"]; i = 1; }
    return i32(arr[0][0]) - 97;
}
"#,
        "E10030",
    );
}

/// RR-1 negative control: a fresh argument that is not the returned source is
/// accepted.
#[test]
fn should_accept_a_static_view_with_a_fresh_unrelated_argument() {
    assert_accepts(
        "rr1-static-view",
        r#"module spec.v1pr.review2.staticview;

func staticView(n: i32, v: str): str { return v; }
func next(): i32 { return 1; }

func main(): i32 {
    let v: str = "";
    let i: i32 = 0;
    while (i < 1) {
        v = staticView(next(), "zz");
        i = 1;
    }
    return i32(v[0]) - 122;
}
"#,
    );
}

/// RR-3b: a conditional field before a guarded array keeps source order.
#[test]
fn should_preserve_order_for_a_conditional_field_before_a_guarded_array() {
    assert_accepts(
        "rr3b-guarded-conditional-field",
        r#"module spec.v1pr.review2.guardedcondfield;

import { String, stringOf } from std.text;

struct S { tag: String; rows: str[2]; }

let c: bool = true;
let flag: bool = true;
let order: i32 = 0;

func tagf(): String { order = order * 10 + 1; return stringOf("t"); }
func tagg(): String { order = order * 10 + 9; return stringOf("u"); }
func a(): String { order = order * 10 + 2; return stringOf("aa"); }
func f(): String { order = order * 10 + 3; return stringOf("bb"); }

let g: str = c ? S { tag: flag ? tagf() : tagg(), rows: [a(); f()] }.rows[0] : stringOf("zz");

func main(): i32 { return order - 123; }
"#,
    );
}

/// RR-9: a conditional fresh projection element retains the chosen arm root.
#[test]
fn should_release_a_conditional_fresh_projection_element() {
    assert_accepts(
        "rr9-conditional-projection",
        r#"module spec.v1pr.review2.conditionalprojection;

import { String, stringOf } from std.text;

struct H { names: str[2]; k: String; }
struct Box { rows: str[2]; }

let flag: bool = true;

func mk(): H { return H { names: ["aa", "bb"], k: stringOf("kk") }; }

let g: str = Box { rows: [flag ? mk().names[0] : mk().names[1]; "zz"] }.rows[0];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// RR-9: a struct-literal projection element retains its fresh root.
#[test]
fn should_release_a_struct_literal_projection_element() {
    assert_accepts(
        "rr9-struct-projection",
        r#"module spec.v1pr.review2.structprojection;

import { String, stringOf } from std.text;

struct H { names: str[2]; k: String; }
struct Box { rows: str[2]; }

let g: str = Box { rows: [H { names: ["aa", "bb"], k: stringOf("kk") }.names[0]; "zz"] }.rows[0];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// RR-1 negative control: a capture store of a static view is accepted.
#[test]
fn should_accept_a_capture_store_of_a_static_view() {
    assert_accepts(
        "rr1-static-capture",
        r#"module spec.v1pr.review2.staticcapture;

func staticView(n: i32, v: str): str { return v; }
func next(): i32 { return 1; }

func main(): i32 {
    let v: str = "";
    let i: i32 = 0;
    while (i < 2) {
        ([borrow v] (s: str) => { v = s; })(staticView(next(), "zz"));
        i = i + 1;
    }
    return i32(v[0]) - 122;
}
"#,
    );
}
