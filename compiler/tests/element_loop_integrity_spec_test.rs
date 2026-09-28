//! Runnable specification tests for owned-element reads passed to parameters.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The value under test travels
//! in `main`'s return code, except for the case that prints.
//!
//! The behavior under test is that reading an owned container's element (or a
//! field of one) and passing it to a parameter must be a lend or a view: the
//! container stays the only owner, so the program returns instead of aborting
//! with a double free. Two cases are compile-time rejections: moving an element
//! out of its container stays limited to top-level fields. One case asserts the
//! existing bounds trap for a runtime index.
//!
//! The suite also covers nested loops: a `for` loop inside any other loop must
//! run its own step. The nested cases sum concrete values, exercise `continue`
//! and `break`, and include non-nested regression loops. A loop whose step is
//! mis-selected never terminates, so the time limit turns it into one failing
//! case.
//!
//! Every run goes through a helper with a time limit, so a loop regression
//! fails the case instead of freezing the suite.

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
            "clc-elemspec-{}-{tag}-{sequence}",
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

/// Asserts the run wrote exactly `expected` to stdout and exited zero.
fn assert_stdout(tag: &str, output: &Output, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected a clean exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "{tag}: unexpected stdout"
    );
}

/// Asserts the front end rejects the program with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
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

/// Asserts the run aborted at the runtime guard for `code`.
fn assert_trap(tag: &str, output: &Output, code: &str) {
    assert!(
        !output.status.success(),
        "{tag}: a trapped run must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected the {code} trap; stderr:\n{stderr}"
    );
}

/// A constant-index element read passed to a view parameter is a lend.
///
/// The array owns both strings; the callee reads only the first one, so the
/// program returns the first length instead of aborting when both owners drop.
#[test]
fn should_read_the_first_element_when_the_owner_passes_a_constant_index() {
    let source = r#"module codegen.elem.st1;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    return vlen(arr[0]);
}
"#;
    let output = run_host("es1", source);
    assert_exit("es1", &output, 1);
}

/// A runtime-index element read passed to a view parameter is a lend.
///
/// The index is a variable, so the emitted bounds guard runs before the read;
/// the selected string has length two, and the program returns instead of
/// aborting when the scope cleanup runs.
#[test]
fn should_read_the_selected_element_when_the_index_is_a_runtime_value() {
    let source = r#"module codegen.elem.st2;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    let i: i32 = 1;
    return vlen(arr[i]);
}
"#;
    let output = run_host("es2", source);
    assert_exit("es2", &output, 2);
}

/// Element reads inside a callee that borrows the whole array stay lends.
///
/// The callee owns nothing; the caller's array owns both strings and releases
/// them once, so the sum of the two lengths is returned.
#[test]
fn should_lend_elements_when_the_callee_borrows_the_array() {
    let source = r#"module codegen.elem.st3;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func total(borrow xs: String[2]): i32 {
    return vlen(xs[0]) + vlen(xs[1]);
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    return total(borrow arr);
}
"#;
    let output = run_host("es3", source);
    assert_exit("es3", &output, 3);
}

/// An owned field of an element is lent, not copied into a second owner.
///
/// The first element's `name` has length one and the second element's `n` is
/// four, so the sum is five and both the element field and the temporary view
/// coexist without a double free.
#[test]
fn should_lend_a_struct_field_when_the_argument_is_an_element_field() {
    let source = r#"module codegen.elem.st4;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 3 }, P { name: stringOf("bb"), n: 4 }];
    return vlen(ps[0].name) + ps[1].n;
}
"#;
    let output = run_host("es4", source);
    assert_exit("es4", &output, 5);
}

/// An element reached through a borrow parameter is lent to the callee.
///
/// The callee sees the caller's bag through a borrow, reads both element
/// lengths (three and two), and the caller's struct keeps ownership; the sum is
/// five.
#[test]
fn should_lend_a_nested_element_when_the_callee_borrows_a_struct_field() {
    let source = r#"module codegen.elem.st5;

import { String, stringOf } from std.text;

struct Bag {
    xs: String[2];
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func pick(borrow b: Bag): i32 {
    return vlen(b.xs[0]) + vlen(b.xs[1]);
}

func main(): i32 {
    let bag: Bag = Bag { xs: [stringOf("abc"), stringOf("de")] };
    return pick(borrow bag);
}
"#;
    let output = run_host("es5", source);
    assert_exit("es5", &output, 5);
}

/// Unmarked and `const` owned parameters take read-only lends.
///
/// The caller keeps the array; the callee's parameters never release the
/// borrowed strings. The two answers (35 and 7) sum to 42, and the scope
/// cleanup frees each element exactly once.
#[test]
fn should_lend_to_unmarked_and_const_parameters_without_releasing() {
    let source = r#"module codegen.elem.st6;

import { String, stringOf } from std.text;

func byval(s: String): i32 {
    return 35;
}

func byconst(const s: String): i32 {
    return 7;
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    return byval(arr[0]) + byconst(arr[1]);
}
"#;
    let output = run_host("es6", source);
    assert_exit("es6", &output, 42);
}

/// A `borrow` owned parameter takes a writable lend of the element.
///
/// The call names the element with an explicit `borrow` marker; the callee does
/// not release it, and the caller's array frees it once, so the answer 42 is
/// returned.
#[test]
fn should_lend_to_a_borrow_parameter_without_releasing() {
    let source = r#"module codegen.elem.st7;

import { String, stringOf } from std.text;

func sink(borrow s: String): i32 {
    return 42;
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    return sink(borrow arr[0]);
}
"#;
    let output = run_host("es7", source);
    assert_exit("es7", &output, 42);
}

/// An element read inside a template placeholder is printed and lent.
///
/// The placeholder lowers through the same materialization guard as a call
/// argument; with the element lent, printing writes `ab` and the scope cleanup
/// releases the array exactly once.
#[test]
fn should_print_an_element_when_it_is_a_template_placeholder() {
    let source = r#"module codegen.elem.st8;

import { String, stringOf } from std.text;
import { println } from std.io;

func main(): i32 {
    let arr: String[1] = [stringOf("ab")];
    println($"{arr[0]}");
    return 0;
}
"#;
    let output = run_host("es8", source);
    assert_stdout("es8", &output, "ab\n");
}

/// Moving an element into a `take` parameter stays a compile-time error.
///
/// Partial moves are limited to top-level fields, so consuming `arr[0]` with
/// `take` reports the partial-move diagnostic and no translation unit is
/// written.
#[test]
fn should_reject_moving_an_element_into_a_take_parameter() {
    let source = r#"module codegen.elem.st9;

import { String, stringOf } from std.text;

func eat(take s: String): void {
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    eat(arr[0]);
    return 0;
}
"#;
    assert_rejected("es9", source, "E10026");
}

/// Binding an element to an owned local stays a compile-time error.
///
/// The same partial-move rule rejects `let a: String = arr[0];`, so the program
/// is refused before any C is emitted.
#[test]
fn should_reject_binding_an_element_to_an_owned_local() {
    let source = r#"module codegen.elem.st10;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    let a: String = arr[0];
    return vlen(a);
}
"#;
    assert_rejected("es10", source, "E10026");
}

/// An element loop lends each element to the read site.
///
/// The loop binding is a view of the current element; both lengths (one and
/// two) are added and the array is released once, so the total is three.
#[test]
fn should_lend_each_element_when_an_element_loop_reads_it() {
    let source = r#"module codegen.elem.st11;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    let total: i32 = 0;
    for (borrow e in xs) {
        total = total + vlen(e);
    }
    return total;
}
"#;
    let output = run_host("es11", source);
    assert_exit("es11", &output, 3);
}

/// An ownerless temporary passed to a view parameter is still released once.
///
/// A call result has no container behind it, so it keeps its scope binding and
/// the cleanup releases it exactly once; its length is two.
#[test]
fn should_release_an_ownerless_temporary_once_when_it_is_lent() {
    let source = r#"module codegen.elem.st12;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    return vlen(stringOf("hi"));
}
"#;
    let output = run_host("es12", source);
    assert_exit("es12", &output, 2);
}

/// A runtime index past the extent still traps with the bounds guard.
///
/// The index is two for a two-element array; the emitted guard aborts before
/// the element is read, so the process never returns a value.
#[test]
fn should_trap_when_the_runtime_index_is_past_the_extent() {
    let source = r#"module codegen.elem.st13;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    let i: i32 = 2;
    return vlen(arr[i]);
}
"#;
    let output = run_host("es13", source);
    assert_trap("es13", &output, "E11211");
}

/// Nested element loops run both their own heads and their own steps.
///
/// The outer loop walks `[1, 2]`, the inner loop walks `[1, 2, 3]`, and the sum
/// of every `x + y` pair is `21`. A nested loop that re-enters its own
/// pre-header as the C step never advances and hits the bounded run instead of
/// returning.
#[test]
fn should_sum_both_loop_variables_when_element_loops_are_nested() {
    let source = r#"module codegen.elem.st14;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let ys: i32[3] = [1, 2, 3];
    let s: i32 = 0;
    for (borrow x in xs) {
        for (borrow y in ys) {
            s = s + x + y;
        }
    }
    return s;
}
"#;
    let output = run_host("es14", source);
    assert_exit("es14", &output, 21);
}

/// An element loop inside a `while` body advances once per outer iteration.
///
/// The outer `while` runs twice and the inner loop adds `1 + 2` each time, so
/// the sum is `6`; the inner loop must terminate to reach the second iteration.
#[test]
fn should_advance_the_element_loop_when_it_is_inside_a_while() {
    let source = r#"module codegen.elem.st15;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let s: i32 = 0;
    let k: i32 = 0;
    while (k < 2) {
        for (borrow x in xs) {
            s = s + x;
        }
        k = k + 1;
    }
    return s;
}
"#;
    let output = run_host("es15", source);
    assert_exit("es15", &output, 6);
}

/// A three-clause `for` inside a `while` body advances its own index.
///
/// The outer `while` runs twice and the inner `for` adds `0 + 1 + 2` each time,
/// so the sum is `6`; the inner loop must run its increment, not its initializer.
#[test]
fn should_advance_the_three_clause_loop_when_it_is_inside_a_while() {
    let source = r#"module codegen.elem.st16;

func main(): i32 {
    let s: i32 = 0;
    let k: i32 = 0;
    while (k < 2) {
        for (let j: i32 = 0; j < 3; j = j + 1) {
            s = s + j;
        }
        k = k + 1;
    }
    return s;
}
"#;
    let output = run_host("es16", source);
    assert_exit("es16", &output, 6);
}

/// A `continue` in a nested element loop resumes at the inner step.
///
/// The inner loop skips `y == 2` for both outer elements; the observed sum is
/// `1+1 + 1+3 + 2+1 + 2+3 = 14`. A `continue` that resumes at the wrong block
/// either hangs or counts the wrong pairs.
#[test]
fn should_resume_at_the_inner_step_when_a_nested_element_loop_continues() {
    let source = r#"module codegen.elem.st17;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let ys: i32[3] = [1, 2, 3];
    let s: i32 = 0;
    for (borrow x in xs) {
        for (borrow y in ys) {
            if (y == 2) {
                continue;
            }
            s = s + x + y;
        }
    }
    return s;
}
"#;
    let output = run_host("es17", source);
    assert_exit("es17", &output, 14);
}

/// A `break` in a nested element loop leaves only the inner loop.
///
/// The inner loop stops at `y == 2` for both outer elements, so the sum is
/// `1+1 + 2+1 = 5`; the outer loop must continue after the inner break.
#[test]
fn should_leave_the_inner_loop_when_a_nested_element_loop_breaks() {
    let source = r#"module codegen.elem.st18;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let ys: i32[3] = [1, 2, 3];
    let s: i32 = 0;
    for (borrow x in xs) {
        for (borrow y in ys) {
            if (y == 2) {
                break;
            }
            s = s + x + y;
        }
    }
    return s;
}
"#;
    let output = run_host("es18", source);
    assert_exit("es18", &output, 5);
}

/// Two sequential element loops both complete and both advance.
///
/// The first loop adds `1 + 2` and the second adds `10 + 20 + 30`, so the sum
/// is `63`; the first loop's exit must not corrupt the second loop's step.
#[test]
fn should_run_both_when_two_element_loops_run_in_sequence() {
    let source = r#"module codegen.elem.st19;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let ys: i32[3] = [10, 20, 30];
    let s: i32 = 0;
    for (borrow x in xs) {
        s = s + x;
    }
    for (borrow y in ys) {
        s = s + y;
    }
    return s;
}
"#;
    let output = run_host("es19", source);
    assert_exit("es19", &output, 63);
}

/// A single top-level element loop keeps its current behavior.
///
/// The loop walks `[1, 2]`, adds both values, and returns `3`, so the step
/// selection change does not disturb the non-nested shape.
#[test]
fn should_run_a_top_level_element_loop() {
    let source = r#"module codegen.elem.st20;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let s: i32 = 0;
    for (borrow x in xs) {
        s = s + x;
    }
    return s;
}
"#;
    let output = run_host("es20", source);
    assert_exit("es20", &output, 3);
}

/// A three-clause `for` inside an element loop advances its own index.
///
/// The outer loop walks `[1, 2]` and the inner loop adds `x + 0`, `x + 1`, and
/// `x + 2` for each element, so the sum is `6 + 9 = 15`; the inner loop must
/// run its increment to terminate.
#[test]
fn should_advance_the_three_clause_loop_when_it_is_inside_an_element_loop() {
    let source = r#"module codegen.elem.st21;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let s: i32 = 0;
    for (borrow x in xs) {
        for (let j: i32 = 0; j < 3; j = j + 1) {
            s = s + x + j;
        }
    }
    return s;
}
"#;
    let output = run_host("es21", source);
    assert_exit("es21", &output, 15);
}
