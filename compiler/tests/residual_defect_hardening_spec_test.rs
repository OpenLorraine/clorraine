//! Runnable specification tests for the residual fixed-array and ownership
//! defect fixes.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The chosen value travels in
//! `main`'s return code, so a case that prints nothing states its answer as the
//! exit code.
//!
//! The literal-type batch pins the canonical flat shape of a no-context array
//! literal, the expression-form store for a row that is not itself a literal
//! (conditional, place, call, or fill row), and the unchanged behavior of a
//! literal whose element is a slice or whose rows mismatch. A literal that
//! appears under a false guard is never evaluated, so its exit code also pins
//! the short-circuit.
//!
//! Every run case asserts an exit code that proves the chosen value was read
//! correctly, and a double release aborts in the C library, so a clean exit
//! also pins release-once. Rejection cases assert the diagnostic code and that
//! no translation unit is written.

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
            "clc-rdhspec-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the compiler output plus the C path.
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

/// Asserts the front end rejects the program with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// A guarded deferred literal selects its then-row when the guard holds.
///
/// The literal `[c ? rows[0] : rows[1], [5, 6]]` holds two rows; the first row
/// is a conditional and the second a literal. With `c` true the first row is
/// `rows[0]` and the read element is `1`, so the guard is true and the exit
/// code is `7`. A store that never reaches the literal's storage would leave
/// the comparison false and exit `9`.
#[test]
fn should_read_the_then_row_when_a_guarded_literal_holds_a_conditional() {
    let source = r#"module spec.rdh.st1;

func main(): i32 {
    let ok: bool = true;
    let c: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh01t", source);
    assert_exit("rdh01t", &output, 7);
}

/// A guarded deferred literal selects its else-row when the condition is false.
///
/// With `c` false the first row is `rows[1]`, whose read element is `3`, so the
/// comparison against `1` is false and the exit code is `9`. A store that
/// landed in the wrong row or dropped the literal's data would change this.
#[test]
fn should_read_the_else_row_when_a_guarded_literal_holds_a_conditional() {
    let source = r#"module spec.rdh.st1;

func main(): i32 {
    let ok: bool = true;
    let c: bool = false;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh01f", source);
    assert_exit("rdh01f", &output, 9);
}

/// A guarded deferred literal is not evaluated when the guard is false.
///
/// With `ok` false the `&&` short-circuits, so the literal is never stored and
/// the exit code is `9`. The case also pins that the materialized temporary's
/// cleanup runs without touching uninitialized rows: a double release or a
/// stray trap would change the exit.
#[test]
fn should_short_circuit_when_the_guard_is_false() {
    let source = r#"module spec.rdh.st1;

func main(): i32 {
    let ok: bool = false;
    let c: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh01s", source);
    assert_exit("rdh01s", &output, 9);
}

/// A no-context literal has the flat fixed-array type of its rows.
///
/// `lit` is inferred from `[[1, 2], [3, 4]]`, so reading `lit[1][0]` gives `3`.
/// A nested element type would emit a one-extent declarator and fail to build
/// or read the wrong element.
#[test]
fn should_infer_a_flat_type_when_a_literal_has_no_context() {
    let source = r#"module spec.rdh.st2;

func main(): i32 {
    let lit = [[1, 2], [3, 4]];
    return lit[1][0];
}
"#;
    let output = run_host("rdh02", source);
    assert_exit("rdh02", &output, 3);
}

/// A conditional between an untyped literal and a typed place takes the literal.
///
/// With `ok` true `pick` is `[a1, a2]`, so `pick[1][0]` is `3`. The literal has
/// no annotation of its own; the conditional's other arm supplies the type.
#[test]
fn should_take_the_literal_arm_when_a_conditional_is_untyped() {
    let source = r#"module spec.rdh.st3;

func main(): i32 {
    let ok: bool = true;
    let a1: i32[2] = [1, 2];
    let a2: i32[2] = [3, 4];
    let rows: i32[2][2] = [[5, 6], [7, 8]];
    let pick = ok ? [a1, a2] : rows;
    return pick[1][0];
}
"#;
    let output = run_host("rdh03t", source);
    assert_exit("rdh03t", &output, 3);
}

/// A conditional between an untyped literal and a typed place takes the place.
///
/// With `ok` false `pick` is `rows`, so `pick[1][0]` is `7`. The literal's
/// inferred type must match the place's type or the front end rejects a valid
/// program.
#[test]
fn should_take_the_place_arm_when_a_conditional_is_untyped() {
    let source = r#"module spec.rdh.st3;

func main(): i32 {
    let ok: bool = false;
    let a1: i32[2] = [1, 2];
    let a2: i32[2] = [3, 4];
    let rows: i32[2][2] = [[5, 6], [7, 8]];
    let pick = ok ? [a1, a2] : rows;
    return pick[1][0];
}
"#;
    let output = run_host("rdh03f", source);
    assert_exit("rdh03f", &output, 7);
}

/// A guarded deferred literal moves an owned conditional row exactly once.
///
/// `rows` holds owned strings; with `c` true the literal's first row is
/// `rows[0]`, whose first string is `"ab"` with length two, so the exit code is
/// `7`. A double release of any moved string aborts in the C library, so the
/// clean exit pins release-once.
#[test]
fn should_move_the_then_row_when_a_guarded_owned_literal_holds_a_conditional() {
    let source = r#"module spec.rdh.st4;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = true;
    let c: bool = true;
    let rows: String[2][2] = [[stringOf("ab"), stringOf("c")], [stringOf("d"), stringOf("ef")]];
    let hit: bool = ok && len([c ? rows[0] : rows[1], [stringOf("gh"), stringOf("i")]][0][0]) == 2;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh04t", source);
    assert_exit("rdh04t", &output, 7);
}

/// A guarded deferred owned literal moves its else-row when the condition is false.
///
/// With `c` false the first row is `rows[1]`, whose first string is `"d"` with
/// length one, so the comparison is false and the exit code is `9`. The case
/// pins that the un-chosen row is not released or emptied on the way.
#[test]
fn should_move_the_else_row_when_a_guarded_owned_literal_holds_a_conditional() {
    let source = r#"module spec.rdh.st4;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = true;
    let c: bool = false;
    let rows: String[2][2] = [[stringOf("ab"), stringOf("c")], [stringOf("d"), stringOf("ef")]];
    let hit: bool = ok && len([c ? rows[0] : rows[1], [stringOf("gh"), stringOf("i")]][0][0]) == 2;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh04f", source);
    assert_exit("rdh04f", &output, 9);
}

/// A literal whose element is a slice keeps its slice elements.
///
/// `pair` is inferred from two slices; the first has length two, so the exit
/// code `1 * 100 + 4 * 10 + 2 = 142` proves both slice headers were stored
/// unmangled. Folding a slice element into the extent list would produce a
/// different type and fail to build or read.
#[test]
fn should_keep_the_slice_element_when_a_literal_holds_slices() {
    let source = r#"module spec.rdh.st5;

import { slice } from std.mem;

func main(): i32 {
    let data: u8[4] = [1, 2, 3, 4];
    let a: u8[] = slice(data, 0, 2);
    let b: u8[] = slice(data, 2, 4);
    let pair = [a, b];
    return i32(pair[0][0]) * 100 + i32(pair[1][1]) * 10 + i32(len(pair[0]));
}
"#;
    let output = run_host("rdh05s", source);
    assert_exit("rdh05s", &output, 142);
}

/// A literal with a row longer than the context extent is rejected.
///
/// `[[1, 2], [3, 4, 5]]` does not fit `i32[2][2]`, so the front end reports
/// `E10215` and writes no translation unit.
#[test]
fn should_reject_a_literal_when_a_row_is_longer_than_the_context_extent() {
    let source = r#"module spec.rdh.st5;

func main(): i32 {
    let bad: i32[2][2] = [[1, 2], [3, 4, 5]];
    return bad[0][0];
}
"#;
    assert_rejected("rdh05r", source, "E10215");
}

/// A filled literal stores the chosen row when the fill value is conditional.
///
/// `g` is filled with `c ? a : b`; with `c` false every row is `b`, so
/// `g[1][0] * 10 + g[0][1]` is `34`. The fill's non-literal row must be stored
/// through the expression form for both rows.
#[test]
fn should_store_the_chosen_row_when_a_fill_value_is_conditional() {
    let source = r#"module spec.rdh.st17;

func main(): i32 {
    let c: bool = false;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let g: i32[2][2] = [; c ? a : b];
    return g[1][0] * 10 + g[0][1];
}
"#;
    let output = run_host("rdh17", source);
    assert_exit("rdh17", &output, 34);
}
