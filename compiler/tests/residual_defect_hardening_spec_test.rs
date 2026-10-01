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
    assert_rejected_with(tag, source, code, &[]);
}

/// Asserts the front end rejects the program with `code`, that the message
/// contains every fragment, and that no C is written.
fn assert_rejected_with(tag: &str, source: &str, code: &str, fragments: &[&str]) {
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
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
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

/// A guarded deferred literal selects a call row.
///
/// The first row is `make()`, which returns `[7, 8]`; with `ok` true the read
/// element is `7`, so the exit code is `7`. The row is written through the
/// call's hidden destination, and the emitted C must build under the strict
/// warning flags.
#[test]
fn should_take_a_call_row_when_a_guarded_literal_holds_one() {
    let source = r#"module spec.rdh.st1;

func make(): i32[2] {
    return [7, 8];
}

func main(): i32 {
    let ok: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [make(), [5, 6]][0][0] == 7;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh01c", source);
    assert_exit("rdh01c", &output, 7);
}

/// A guarded deferred literal selects a call arm inside a conditional row.
///
/// With `c` true the first row is `make()`, whose first element is `7`, so the
/// comparison holds and the exit code is `7`. The call arm is wrapped with the
/// place arm's common value, and the emitted C must build under the strict
/// warning flags.
#[test]
fn should_take_the_call_arm_when_a_conditional_row_selects_it() {
    let source = r#"module spec.rdh.st1;

func make(): i32[2] {
    return [7, 8];
}

func main(): i32 {
    let ok: bool = true;
    let c: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [c ? make() : rows[0], [5, 6]][0][0] == 7;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh01cc", source);
    assert_exit("rdh01cc", &output, 7);
}

/// A guarded deferred literal runs a take-call row exactly once.
///
/// `consume` takes its argument and returns a row whose second element is the
/// argument's length, so a second call would read the emptied source and
/// crash. The program must exit with the row's first element.
#[test]
fn should_run_a_take_call_row_once_when_a_guarded_literal_holds_one() {
    let source = r#"module spec.rdh.st1;

import { String, stringOf } from std.text;

func consume(take src: String): i32[2] {
    return [1, i32(len(src))];
}

func main(): i32 {
    let ok: bool = true;
    let src: String = stringOf("ab");
    let hit: bool = ok && [consume(src), [5, 6]][0][0] == 1;
    return hit ? 7 : 9;
}
"#;
    let output = run_host("rdh01m", source);
    assert_exit("rdh01m", &output, 7);
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

/// A returned multi-extent owned parameter copies every row and empties it.
///
/// `copy` takes `src` and returns it by value; the caller reads row one's
/// first string (length two) and row zero's second string (length one), so
/// the exit code is `221`. A copy that landed in one extent, or a double
/// release of a moved string, would fail the build, abort, or change the
/// answer.
#[test]
fn should_copy_every_row_when_a_multi_extent_owned_parameter_returns() {
    let source = r#"module spec.rdh.st6;

import { String, stringOf } from std.text;

func copy(take a: String[2][2]): String[2][2] {
    return a;
}

func main(): i32 {
    let src: String[2][2] = [[stringOf("ab"), stringOf("c")], [stringOf("de"), stringOf("f")]];
    let out: String[2][2] = copy(src);
    return i32(len(out[0][0])) * 100 + i32(len(out[1][0])) * 10 + i32(len(out[0][1]));
}
"#;
    let output = run_host("rdh06", source);
    assert_exit("rdh06", &output, 221);
}

/// A returned multi-extent duplicable parameter copies every row.
///
/// `copy` takes `src` and returns it by value; with no release needed the
/// operation is a plain byte copy. The exit code `142` proves both rows
/// travelled.
#[test]
fn should_copy_every_row_when_a_multi_extent_duplicable_parameter_returns() {
    let source = r#"module spec.rdh.st7;

func copy(take a: i32[2][2]): i32[2][2] {
    return a;
}

func main(): i32 {
    let src: i32[2][2] = [[1, 2], [3, 4]];
    let out: i32[2][2] = copy(src);
    return out[0][0] * 100 + out[1][1] * 10 + out[0][1];
}
"#;
    let output = run_host("rdh07", source);
    assert_exit("rdh07", &output, 142);
}

/// An unguarded `while` condition holding an owned call runs and releases it.
///
/// Every test calls `grow`, which builds a fresh growable array, and reads its
/// count. The body runs exactly three times, so the exit code is `3`. The
/// per-iteration buffer leak is invisible to the exit code; it is pinned by
/// the recorded AddressSanitizer probe.
#[test]
fn should_run_three_times_when_an_unguarded_while_condition_holds_an_owned_call() {
    let source = r#"module spec.rdh.st8;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    while (grow().count > 0 && i < 3) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("rdh08", source);
    assert_exit("rdh08", &output, 3);
}

/// A condition call trapped in a materialized read still releases.
///
/// The condition indexes a literal holding `grow().count`; the literal is
/// filled in the loop head, so the owned call result must travel through its
/// own temporary and be released after the loop. The exit code is `3`; the
/// per-iteration leak is pinned by the recorded AddressSanitizer probe.
#[test]
fn should_release_when_a_condition_call_is_trapped_in_a_materialized_read() {
    let source = r#"module spec.rdh.st8b;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    while ([grow().count][0] > 0 && i < 3) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("rdh08b", source);
    assert_exit("rdh08b", &output, 3);
}

/// An owned conditional array element moves its then-call into the element.
///
/// The literal is read through `[0]`, so it is filled through the element
/// store; the chosen call's string is moved and only the un-chosen arm is
/// released, so the exit code is the then-string's length. A plain copy would
/// leave the arm and the element owning the same storage and double-free.
#[test]
fn should_move_the_then_call_when_an_owned_conditional_is_an_array_element() {
    let source = r#"module spec.rdh.st8c;

import { stringOf } from std.text;

func main(): i32 {
    let c: bool = true;
    let x: i32 = i32(len([c ? stringOf("ab") : stringOf("c")][0]));
    return x;
}
"#;
    let output = run_host("rdh08c", source);
    assert_exit("rdh08c", &output, 2);
}

/// An owned conditional array element moves its else-call into the element.
///
/// With `c` false the chosen arm is the else string, whose length one is the
/// exit code; the then arm stays owned and is released once.
#[test]
fn should_move_the_else_call_when_an_owned_conditional_is_an_array_element() {
    let source = r#"module spec.rdh.st8c;

import { stringOf } from std.text;

func main(): i32 {
    let c: bool = false;
    let x: i32 = i32(len([c ? stringOf("ab") : stringOf("c")][0]));
    return x;
}
"#;
    let output = run_host("rdh08cf", source);
    assert_exit("rdh08cf", &output, 1);
}

/// An owned conditional element moves per test in a loop condition.
///
/// Every test fills a one-element literal from the chosen call and reads its
/// length; the body runs three times, so the exit code is `3`. A double free
/// would abort before the exit.
#[test]
fn should_move_the_chosen_call_when_an_owned_conditional_is_a_condition_element() {
    let source = r#"module spec.rdh.st8c;

import { stringOf } from std.text;

func main(): i32 {
    let c: bool = true;
    let i: i32 = 0;
    while (len([c ? stringOf("ab") : stringOf("c")][0]) == 2 && i < 3) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("rdh08cl", source);
    assert_exit("rdh08cl", &output, 3);
}

/// A `take` parameter instantiated with an owning type releases its elements.
///
/// `keep` never touches `a`, so the caller hands the only reference to the
/// callee; the parameter's element cleanup must release both strings. The exit
/// code is `7`; the leak is pinned by the recorded AddressSanitizer probe.
#[test]
fn should_release_the_parameter_when_a_take_parameter_is_owning() {
    let source = r#"module spec.rdh.st10;

import { String, stringOf } from std.text;

func keep<T>(take a: T[2]): i32 {
    return 0;
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("c")];
    keep<String>(xs);
    return 7;
}
"#;
    let output = run_host("rdh10", source);
    assert_exit("rdh10", &output, 7);
}

/// A generic local move moves under an owning instance.
///
/// `dup` initializes `b` from `a` and returns `b`; under `String` the handle
/// travels to the caller and the callee keeps nothing, so the caller reads the
/// length four and the storage is released exactly once.
#[test]
fn should_move_through_the_local_when_a_generic_move_is_owning() {
    let source = r#"module spec.rdh.st11;

import { String, stringOf } from std.text;

func dup<T>(take a: T): T {
    let b: T = a;
    return b;
}

func main(): i32 {
    let s: String = stringOf("abcd");
    let t: String = dup<String>(s);
    return i32(len(t));
}
"#;
    let output = run_host("rdh11", source);
    assert_exit("rdh11", &output, 4);
}

/// A later use after a generic move is rejected for an owning instance.
///
/// `reuse` moves `a` into `b` and then lends `a` again; under `String` the
/// compiler reports `E10020` once and writes no translation unit.
#[test]
fn should_reject_a_later_use_when_a_generic_move_needs_duplicability() {
    let source = r#"module spec.rdh.st12;

import { String, stringOf } from std.text;

func sink(borrow v: String): i32 {
    return i32(len(v));
}

func reuse<T>(take a: T): i32 {
    let b: T = a;
    sink(borrow a);
    return 0;
}

func main(): i32 {
    let s: String = stringOf("abcd");
    return reuse<String>(s);
}
"#;
    assert_rejected_with(
        "rdh12",
        source,
        "E10020",
        &["'a' was moved and cannot be used"],
    );
}

/// A later use after a generic move is accepted for a duplicable instance.
///
/// Under `i32` both bindings hold five, so the function compiles and returns
/// ten.
#[test]
fn should_accept_a_later_use_when_a_generic_move_is_duplicable() {
    let source = r#"module spec.rdh.st12;

func sinkInt(borrow v: i32): i32 {
    return v;
}

func reuse<T>(take a: T): i32 {
    let b: T = a;
    return sinkInt(borrow a) + sinkInt(borrow b);
}

func main(): i32 {
    return reuse<i32>(5);
}
"#;
    let output = run_host("rdh12i", source);
    assert_exit("rdh12i", &output, 10);
}

/// Duplicable instances and a borrowed parameter keep their behavior.
///
/// The `i32` instantiations release nothing extra and the `String` borrow
/// parameter frees nothing in the callee, so the caller's string stays valid
/// and the exit code is three plus one.
#[test]
fn should_keep_duplicable_instances_and_borrows_unchanged() {
    let source = r#"module spec.rdh.st13;

import { String, stringOf } from std.text;

func keep<T>(take a: T[2]): i32 {
    return 0;
}

func dup<T>(take a: T): T {
    let b: T = a;
    return b;
}

func peek<T>(borrow a: T): i32 {
    return 1;
}

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let n: i32 = dup<i32>(3);
    keep<i32>(xs);
    let s: String = stringOf("ab");
    let p: i32 = peek<String>(s);
    return n + p;
}
"#;
    let output = run_host("rdh13", source);
    assert_exit("rdh13", &output, 4);
}

/// A guarded param-typed call is materialized once and its spare elements freed.
///
/// `at` lends element one of `mk()` to `sink`; the guarded operand call runs
/// once, and the unread element of its fixed array is released with the
/// temporary, so the exit code is the sink's constant and the recorded
/// AddressSanitizer probe is clean.
#[test]
fn should_materialize_a_guarded_param_call_once() {
    let source = r#"module spec.rdh.st14;

import { String, stringOf } from std.text;

func sink<T>(borrow v: T): i32 {
    return 1;
}

func at<T>(mk: func(): T[2], i: i32): i32 {
    return sink<T>(borrow mk()[i]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return at<String>(mkString, 1);
}
"#;
    let output = run_host("rdh14", source);
    assert_exit("rdh14", &output, 1);
}

/// A param-typed iterable is evaluated once and does not leak.
///
/// The element loop borrows the two elements of `mk()`'s array; the iterable
/// travels through one temporary whose release frees the elements, and the
/// exit code is two.
#[test]
fn should_materialize_a_param_iterable_once() {
    let source = r#"module spec.rdh.st15;

import { String, stringOf } from std.text;

func all<T>(mk: func(): T[2]): i32 {
    let n: i32 = 0;
    for (borrow _e in mk()) {
        n = n + 1;
    }
    return n;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return all<String>(mkString);
}
"#;
    let output = run_host("rdh15", source);
    assert_exit("rdh15", &output, 2);
}

/// A param-typed call passed to a `take` parameter is released by the callee.
///
/// `pass` hands `mk()`'s fresh string to `sink`, whose maybe-owning parameter
/// cleanup releases it; the caller releases nothing and the exit code is zero.
#[test]
fn should_release_a_param_call_passed_to_a_take_parameter() {
    let source = r#"module spec.rdh.st16;

import { String, stringOf } from std.text;

func sink<T>(take v: T): i32 {
    return 0;
}

func pass<T>(mk: func(): T): i32 {
    return sink<T>(mk());
}

func mkString(): String {
    return stringOf("abcd");
}

func main(): i32 {
    return pass<String>(mkString);
}
"#;
    let output = run_host("rdh16", source);
    assert_exit("rdh16", &output, 0);
}

/// A param-typed loop-condition call is released per test.
///
/// Every test calls `mk`, which returns a fresh string, and reads its length;
/// the body runs three times and the exit code is three. The per-test leak is
/// pinned by the recorded AddressSanitizer probe.
#[test]
fn should_release_a_param_call_in_a_loop_condition() {
    let source = r#"module spec.rdh.st18;

import { String, stringOf } from std.text;

func mkString(): String {
    return stringOf("ab");
}

func loop<T>(mk: func(): T): i32 {
    let i: i32 = 0;
    while (i < 3 && len(mk()) > 0) {
        i = i + 1;
    }
    return i;
}

func main(): i32 {
    return loop<String>(mkString);
}
"#;
    let output = run_host("rdh18", source);
    assert_exit("rdh18", &output, 3);
}

/// A param-typed `take` argument place is emptied and released once.
///
/// `caller` hands its own parameter to `sink`; the caller's place is emptied
/// and the callee releases the value exactly once. The exit code is zero and a
/// double free would abort.
#[test]
fn should_empty_the_take_argument_when_it_is_a_param_typed_place() {
    let source = r#"module spec.rdh.st19;

import { String, stringOf } from std.text;

func sink<T>(take v: T): i32 {
    return 0;
}

func caller<T>(take a: T): i32 {
    return sink<T>(a);
}

func main(): i32 {
    let s: String = stringOf("abcd");
    return caller<String>(s);
}
"#;
    let output = run_host("rdh19", source);
    assert_exit("rdh19", &output, 0);
}

/// A param-typed operand used twice is materialized once.
///
/// `bump` increments the element at `at(i)`; the index call must run once and
/// the store must see that one value, so the array becomes one and three and
/// the sum is four.
#[test]
fn should_materialize_a_param_operand_once_when_it_is_used_twice() {
    let source = r#"module spec.rdh.st20;

func at<T>(v: T): T {
    return v;
}

func bump<T>(borrow xs: i32[2], i: T): i32 {
    xs[at<T>(i)] += 1;
    return xs[0] + xs[1];
}

func main(): i32 {
    let xs: i32[2] = [1, 2];
    return bump<i32>(xs, 1);
}
"#;
    let output = run_host("rdh20", source);
    assert_exit("rdh20", &output, 4);
}

/// A param-typed conditional read place is materialized and moved.
///
/// `pick` takes the two rows and lends element one of `c ? a : b` to `sink`;
/// with `c` true the chosen row moves into the materialized temporary, the
/// un-chosen parameter still releases once, and the exit code is the sink's
/// constant.
#[test]
fn should_materialize_a_param_conditional_read_place() {
    let source = r#"module spec.rdh.st21;

import { String, stringOf } from std.text;

func sink<T>(borrow v: T): i32 {
    return 1;
}

func pick<T>(take a: T[2], take b: T[2], c: bool, i: i32): i32 {
    return sink<T>(borrow (c ? a : b)[i]);
}

func main(): i32 {
    let x: String[2] = [stringOf("ab"), stringOf("cde")];
    let y: String[2] = [stringOf("f"), stringOf("gh")];
    return pick<String>(x, y, true, 1);
}
"#;
    let output = run_host("rdh21", source);
    assert_exit("rdh21", &output, 1);
}

/// A later use after a projected field move is rejected when owning.
#[test]
fn should_reject_a_later_use_after_a_projected_field_move() {
    let source = r#"module spec.rdh.st23;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func sink(borrow s: String): i32 {
    return 0;
}

func f<T>(take p: Box<T>): i32 {
    let b: T = p.v;
    sink(borrow p.v);
    return 0;
}

func main(): i32 {
    let b: Box<String> = Box<String> { v: stringOf("ab") };
    return f<String>(b);
}
"#;
    assert_rejected_with("rdh23", source, "E10020", &["was moved and cannot be used"]);
}

/// A later use after a projected element move is rejected when owning.
#[test]
fn should_reject_a_later_use_after_a_projected_element_move() {
    let source = r#"module spec.rdh.st23;

import { String, stringOf } from std.text;

func sink(borrow s: String): i32 {
    return 0;
}

func f<T>(take xs: T[2]): i32 {
    let b: T = xs[0];
    sink(borrow xs[0]);
    return 0;
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("c")];
    return f<String>(xs);
}
"#;
    assert_rejected_with("rdh23b", source, "E10020", &["was moved and cannot be used"]);
}

/// A second take call after an element was consumed is rejected.
#[test]
fn should_reject_a_second_take_call_when_the_element_was_consumed() {
    let source = r#"module spec.rdh.st24;

import { String, stringOf } from std.text;

func sink<T>(take v: T): i32 {
    return 0;
}

func f<T>(take xs: T[2]): i32 {
    let a: i32 = sink<T>(xs[0]);
    let b: i32 = sink<T>(xs[1]);
    return a + b;
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("c")];
    return f<String>(xs);
}
"#;
    assert_rejected_with("rdh24", source, "E10020", &["was moved and cannot be used"]);
}

/// A returned projected element moves out of the take row once.
#[test]
fn should_move_the_returned_element_when_a_take_row_returns_it() {
    let source = r#"module spec.rdh.st25;

import { String, stringOf } from std.text;

func first<T>(take xs: T[2]): T {
    return xs[0];
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("cde")];
    let s: String = first<String>(xs);
    return i32(len(s));
}
"#;
    let output = run_host("rdh25", source);
    assert_exit("rdh25", &output, 2);
}

/// A later use after a conditional aggregate move is rejected.
#[test]
fn should_reject_a_later_use_after_a_conditional_aggregate_move() {
    let source = r#"module spec.rdh.st26;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func sink(borrow s: String): i32 {
    return 0;
}

func f<T>(take a: T, take b: T, c: bool): i32 {
    let box: Box<T> = Box<T> { v: c ? a : b };
    sink(borrow a);
    return 0;
}

func main(): i32 {
    let s1: String = stringOf("ab");
    let s2: String = stringOf("c");
    return f<String>(s1, s2, true);
}
"#;
    assert_rejected_with("rdh26", source, "E10020", &["was moved and cannot be used"]);
}

/// A later use after a struct-literal place move is rejected.
#[test]
fn should_reject_a_later_use_after_a_struct_literal_place_move() {
    let source = r#"module spec.rdh.st27;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func sink(borrow s: String): i32 {
    return 0;
}

func f<T>(take a: T): i32 {
    let box: Box<T> = Box<T> { v: a };
    sink(borrow a);
    return 0;
}

func main(): i32 {
    let s: String = stringOf("ab");
    return f<String>(s);
}
"#;
    assert_rejected_with("rdh27", source, "E10020", &["was moved and cannot be used"]);
}

/// A conditional take argument moves the chosen arm into the callee.
#[test]
fn should_take_the_chosen_arm_when_a_conditional_is_a_take_argument() {
    let source = r#"module spec.rdh.st28;

import { String, stringOf } from std.text;

func sink<T>(take v: T): i32 {
    return 0;
}

func f<T>(take a: T, take b: T, c: bool): i32 {
    return sink<T>(c ? a : b);
}

func main(): i32 {
    let s1: String = stringOf("ab");
    let s2: String = stringOf("c");
    return f<String>(s1, s2, true);
}
"#;
    let output = run_host("rdh28", source);
    assert_exit("rdh28", &output, 0);
}

/// Pushing owned elements grows the container and keeps every element.
#[test]
fn should_keep_owned_elements_when_a_container_is_pushed() {
    let source = r#"module spec.rdh.st29;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func main(): i32 {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    a.push(stringOf("c"));
    return i32(a.count);
}
"#;
    let output = run_host("rdh29", source);
    assert_exit("rdh29", &output, 2);
}

/// A later use after a borrow-element move is rejected.
#[test]
fn should_reject_a_later_use_after_a_borrow_element_move() {
    let source = r#"module spec.rdh.st30;

import { String, stringOf } from std.text;

func sink(borrow s: String): i32 {
    return 0;
}

func f<T>(take xs: T[2]): i32 {
    for (borrow e in xs) {
        let b: T = e;
        sink(borrow e);
    }
    return 0;
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("c")];
    return f<String>(xs);
}
"#;
    assert_rejected_with("rdh30", source, "E10020", &["was moved and cannot be used"]);
}

/// An ownerless borrowed call result is bound and released.
#[test]
fn should_release_an_ownerless_borrowed_call_result() {
    let source = r#"module spec.rdh.st31;

import { String, stringOf } from std.text;

func sink(borrow s: String): i32 {
    return i32(len(s));
}

func mk(): String {
    return stringOf("abcd");
}

func f<T>(mkv: func(): T): i32 {
    return sink(borrow mkv());
}

func main(): i32 {
    return f<String>(mk);
}
"#;
    let output = run_host("rdh31", source);
    assert_exit("rdh31", &output, 4);
}

/// An element write keeps an unrelated consumed path for the duty.
#[test]
fn should_reject_a_later_whole_use_after_an_element_write_kept_a_consumed_path() {
    let source = r#"module spec.rdh.st32;

import { String, stringOf } from std.text;

func sink<T>(borrow v: T[2]): i32 {
    return 0;
}

func f<T>(take xs: T[2], take y: T): i32 {
    let b: T[2] = xs;
    xs[0] = y;
    sink<T>(borrow xs);
    return 0;
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("c")];
    let y: String = stringOf("z");
    return f<String>(xs, y);
}
"#;
    assert_rejected_with("rdh32", source, "E10020", &["was moved and cannot be used"]);
}

/// An ancestor use of a consumed sub-path is rejected.
#[test]
fn should_reject_an_ancestor_use_after_a_projected_move() {
    let source = r#"module spec.rdh.st33;

import { String, stringOf } from std.text;

struct Inner<T> {
    w: T;
}

struct Outer<T> {
    v: Inner<T>;
}

func sink<T>(borrow v: Inner<T>): i32 {
    return 0;
}

func f<T>(take p: Outer<T>): i32 {
    let b: T = p.v.w;
    sink<T>(borrow p.v);
    return 0;
}

func main(): i32 {
    let p: Outer<String> = Outer<String> { v: Inner<String> { w: stringOf("ab") } };
    return f<String>(p);
}
"#;
    assert_rejected_with("rdh33", source, "E10020", &["was moved and cannot be used"]);
}

/// A later use after a concrete conditional array move is rejected.
#[test]
fn should_reject_a_later_use_after_a_concrete_conditional_array_move() {
    let source = r#"module spec.rdh.st34;

import { String, stringOf } from std.text;

func sink(borrow s: String): i32 {
    return 0;
}

func main(): i32 {
    let c: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("c");
    let s: String = stringOf("d");
    let xs: String[2] = [c ? a : b, s];
    sink(borrow a);
    return 0;
}
"#;
    assert_rejected_with("rdh34", source, "E10020", &["was moved and cannot be used"]);
}

/// A later use after a conditional generic move is rejected when owning.
///
/// `pick` moves `a` into `b` through `c ? a : a` and then lends `a`; under
/// `String` the compiler reports `E10020` once and writes no translation unit.
#[test]
fn should_reject_a_later_use_after_a_conditional_move_when_owning() {
    let source = r#"module spec.rdh.st22;

import { String, stringOf } from std.text;

func sink(borrow v: String): i32 {
    return i32(len(v));
}

func pick<T>(take a: T, c: bool): i32 {
    let b: T = c ? a : a;
    sink(borrow a);
    return 0;
}

func main(): i32 {
    let s: String = stringOf("abcd");
    return pick<String>(s, true);
}
"#;
    assert_rejected_with(
        "rdh22",
        source,
        "E10020",
        &["'a' was moved and cannot be used"],
    );
}

/// A conditional generic move is accepted for a duplicable instance.
///
/// Under `i32` the conditional copies five into `b` and `a` stays readable, so
/// the function compiles and returns ten.
#[test]
fn should_accept_a_conditional_move_when_the_instance_is_duplicable() {
    let source = r#"module spec.rdh.st22;

func sinkInt(borrow v: i32): i32 {
    return v;
}

func pick<T>(take a: T, c: bool): i32 {
    let b: T = c ? a : a;
    return sinkInt(borrow a) + sinkInt(borrow b);
}

func main(): i32 {
    return pick<i32>(5, true);
}
"#;
    let output = run_host("rdh22i", source);
    assert_exit("rdh22i", &output, 10);
}

/// An unguarded `do while` condition re-runs after `continue`.
///
/// The body increments `i` and a `continue` skips the rest, so the condition
/// is evaluated again each time; the body runs three times and the exit code
/// is `3`. The leak is pinned by the recorded AddressSanitizer probe.
#[test]
fn should_re_evaluate_when_an_unguarded_do_while_condition_holds_an_owned_call() {
    let source = r#"module spec.rdh.st9;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
        if (i == 2) {
            continue;
        }
    } while (grow().count > 0 && i < 3);
    return i;
}
"#;
    let output = run_host("rdh09d", source);
    assert_exit("rdh09d", &output, 3);
}

/// An unguarded `for` condition holding an owned call runs once per test.
///
/// The step increments `i`, the condition calls `grow` every test, and the
/// loop ends when `i` reaches three.
#[test]
fn should_run_three_times_when_an_unguarded_for_condition_holds_an_owned_call() {
    let source = r#"module spec.rdh.st9;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    for (i = 0; grow().count > 0 && i < 3; i = i + 1) {
    }
    return i;
}
"#;
    let output = run_host("rdh09f", source);
    assert_exit("rdh09f", &output, 3);
}
