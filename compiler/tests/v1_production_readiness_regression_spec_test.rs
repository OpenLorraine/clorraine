//! Regression specification tests for the Phase 1 review findings.
//!
//! The original ST-1 – ST-6 fixtures are in
//! `v1_production_readiness_spec_test.rs`. These cases cover the wider shapes
//! that the independent review used: a global fill with a side-effecting
//! run-time index, an untaken conditional move-out, guarded nested literal
//! roots, source-order fill evaluation, a loop-local lend root, and byte reads
//! that prove the retained backing is still valid, not merely its cached length.
//!
//! The ASan helper links children with `-no-pie` and ignores stack/register
//! roots. The latter keeps an intentionally live global root visible while a
//! leaked temporary held only in a stack slot is still reported.

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
            "clc-v1pr-reg-{}-{tag}-{sequence}",
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

/// Emits `source` into `scratch` and returns the translation unit.
fn emit_into(tag: &str, scratch: &ScratchDir, source: &str) -> String {
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
    let program = if std::env::var_os("RDC_ASAN").is_some() {
        build_asan(tag, &scratch)
    } else {
        build_host(tag, &scratch)
    };
    let env: &[(&str, &str)] = if std::env::var_os("RDC_ASAN").is_some() {
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

/// F-1: a global fill's run-time index is evaluated once and shared.
#[test]
fn should_evaluate_a_global_fill_index_once() {
    assert_accepts(
        "f1-fill-index",
        r#"module spec.v1pr.reg.fillindex;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func mkArr(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
func pick(): usize { calls = calls + 1; if (calls == 1) { return 0; } return 1; }

let g: str[2] = [; mkArr()[pick()]];

func main(): i32 {
    return i32(g[0][0]) - 97;
}
"#,
    );
}

/// F-1: a second out-of-range index must not write past the root array.
#[test]
fn should_not_write_out_of_bounds_for_a_second_fill_index() {
    assert_accepts(
        "f1-fill-oob",
        r#"module spec.v1pr.reg.filloob;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func mkArr(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
func pick(): usize { calls = calls + 1; if (calls == 1) { return 0; } return 5; }

let g: str[2] = [; mkArr()[pick()]];

func main(): i32 {
    return i32(g[0][0]) - 97;
}
"#,
    );
}

/// F-2: an untaken conditional arm must not run its move-out.
#[test]
fn should_guard_an_untaken_conditional_move_out() {
    assert_accepts(
        "f2-conditional",
        r#"module spec.v1pr.reg.conditional;

import { String, stringOf } from std.text;

func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }

let flag: bool = false;
let idx: usize = 5;
let g: str = flag ? mk()[idx] : stringOf("zz");

func main(): i32 { return i32(g[0]) - 122; }
"#,
    );
}

/// F-2: a taken conditional arm still reads its selected backing.
#[test]
fn should_guard_a_taken_conditional_move_out() {
    assert_accepts(
        "f2-taken-conditional",
        r#"module spec.v1pr.reg.conditionaltaken;

import { String, stringOf } from std.text;

func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }

let flag: bool = true;
let idx: usize = 0;
let g: str = flag ? mk()[idx] : stringOf("zz");

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// F-3: a guarded nested literal declares its roots outside the arm block.
#[test]
fn should_build_a_guarded_nested_view_initializer() {
    assert_accepts(
        "f3-guarded-nested",
        r#"module spec.v1pr.reg.guardednested;

import { String, stringOf } from std.text;

struct Grid { rows: str[2][2]; }
struct H { v: String; w: String; }

let c: bool = true;

func mkh(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let g: str = c ? Grid { rows: [[mkh().v, mkh().w], [; stringOf("cc")]] }.rows[0][0]
              : Grid { rows: [[; stringOf("dd")], [; stringOf("ee")]] }.rows[1][0];

func main(): i32 {
    return i32(g[0]) - 97;
}
"#,
    );
}

/// F-3: the same guarded literal as a call argument stays buildable.
#[test]
fn should_build_a_guarded_literal_as_a_call_argument() {
    assert_accepts(
        "f3-guarded-callarg",
        r#"module spec.v1pr.reg.guardedcallarg;

import { String, stringOf } from std.text;

struct Grid { rows: str[2][2]; }
struct H { v: String; w: String; }

let c: bool = true;
func mkh(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }
func choose(s: str): str { return s; }

let g: str = choose(c ? Grid { rows: [[mkh().v, mkh().w], [; stringOf("cc")]] }.rows[0][0]
                     : Grid { rows: [[; stringOf("dd")], [; stringOf("ee")]] }.rows[1][0]);

func main(): i32 {
    return i32(g[0]) - 97;
}
"#,
    );
}

/// F-4: a flat fill after an explicit element keeps source order.
#[test]
fn should_preserve_source_order_for_a_flat_fill() {
    assert_accepts(
        "f4-flat-order",
        r#"module spec.v1pr.reg.flatorder;

import { String, stringOf } from std.text;

struct Box { rows: str[2]; }

let order: i32 = 0;

func a(): String { order = order * 10 + 1; return stringOf("aa"); }
func f(): String { order = order * 10 + 2; return stringOf("bb"); }

let g: str = Box { rows: [a(); f()] }.rows[1];

func main(): i32 { return order - 12; }
"#,
    );
}

/// F-4: nested rows keep the explicit elements before the fill.
#[test]
fn should_preserve_source_order_for_nested_fill_rows() {
    assert_accepts(
        "f4-nested-order",
        r#"module spec.v1pr.reg.nestedorder;

import { String, stringOf } from std.text;

struct Grid { rows: str[2][2]; }

let order: i32 = 0;

func a(): String { order = order * 10 + 1; return stringOf("aa"); }
func b(): String { order = order * 10 + 2; return stringOf("bb"); }
func f(): String { order = order * 10 + 3; return stringOf("cc"); }

let g: str = Grid { rows: [[a(), b()], [; f()]] }.rows[0][0];

func main(): i32 { return order - 123; }
"#,
    );
}

/// F-4: a preceding struct field still runs before the array's fill.
#[test]
fn should_preserve_source_order_for_a_field_before_a_fill() {
    assert_accepts(
        "f4-field-order",
        r#"module spec.v1pr.reg.fieldorder;

import { String, stringOf } from std.text;

struct Box { tag: String; rows: str[2]; }

let order: i32 = 0;

func tagf(): String { order = order * 10 + 1; return stringOf("t"); }
func a(): String { order = order * 10 + 2; return stringOf("aa"); }
func f(): String { order = order * 10 + 3; return stringOf("bb"); }

let g: str = Box { tag: tagf(), rows: [a(); f()] }.rows[0];

func main(): i32 { return order - 123; }
"#,
    );
}

/// F-5: a loop releases each iteration's lend root.
#[test]
fn should_release_each_loop_iteration_lend_root() {
    assert_accepts(
        "f5-loop-lend",
        r#"module spec.v1pr.reg.looplend;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let b: Box = Box { n: sink(mk().xs[0]) };
        total = total + b.n;
        i = i + 1;
    }
    return total - 6;
}
"#,
    );
}

/// F-6: read the selected nested-fill backing, not only its cached length.
#[test]
fn should_read_the_selected_nested_fill_backing() {
    assert_accepts(
        "f6-nested-byte",
        r#"module spec.v1pr.reg.nestedbyte;

import { stringOf } from std.text;

struct Grid { rows: str[2][2]; }

let first: str = Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0];

func main(): i32 {
    return i32(first[0]) - 97;
}
"#,
    );
}

/// F-6: read the selected multi-element backing after the sibling is released.
#[test]
fn should_read_the_selected_multi_element_backing() {
    assert_accepts(
        "f6-multi-byte",
        r#"module spec.v1pr.reg.multibyte;

import { String, stringOf } from std.text;

struct H { v: String; w: String; }
struct Box { names: str[2]; tag: String; }

func mk(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let picked: str = Box { names: [mk().v, mk().w], tag: stringOf("t") }.names[0];

func main(): i32 {
    return i32(picked[0]) - 97;
}
"#,
    );
}

/// F-7: a run-time indexed element in a view-array field is bound and shared.
#[test]
fn should_release_a_runtime_indexed_element_in_a_view_array_field() {
    assert_accepts(
        "f7-runtime-field",
        r#"module spec.v1pr.reg.runtimefield;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { rows: str[2]; }

let calls: i32 = 0;

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func pick(): usize { calls = calls + 1; return 0; }

let g: str = Box { rows: [mk().xs[pick()], "zz"] }.rows[0];

func main(): i32 { return i32(calls) - 1 + i32(g[0]) - 97; }
"#,
    );
}

/// F-7: an unselected conditional element releases its whole chosen root.
#[test]
fn should_release_an_unselected_conditional_element() {
    assert_accepts(
        "f7-conditional-element",
        r#"module spec.v1pr.reg.conditionalelement;

import { String, stringOf } from std.text;

struct H { v: String; w: String; }
struct Box { rows: str[2]; }

let flag: bool = true;

func mk(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let g: str = Box { rows: [mk().v, flag ? mk().v : mk().w] }.rows[0];

func main(): i32 { return i32(g[0]) - 97; }
"#,
    );
}

/// F-6: read the selected indexed view after the unselected slot is released,
/// and prove the runtime `pick()` ran exactly once.
#[test]
fn should_read_the_selected_indexed_view_and_evaluate_pick_once() {
    assert_accepts(
        "f6-index-byte",
        r#"module spec.v1pr.reg.indexbyte;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
func pick(): usize { calls = calls + 1; return 1; }

let selected: str = mk()[pick()];

func main(): i32 {
    return i32(calls) - 1 + i32(selected[0]) - 98;
}
"#,
    );
}
