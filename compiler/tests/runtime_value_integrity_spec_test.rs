//! Runnable specification tests for compiler-made temporaries that own values.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The value under test
//! travels in `main`'s return code, so a case that prints nothing states its
//! answer as the exit code.
//!
//! The behavior under test is that a materialized value is a real owner: a
//! `for` loop over a call result walks a compiler-made copy, a `borrow` element
//! loop writes into that copy, and a temporary that holds owning elements is
//! released at the end of its statement. A fixed-array conditional value is
//! stored through a branch that evaluates only its chosen arm, whether it
//! initializes a local, replaces an owned local, returns from a function, or
//! fills an argument slot. A view derived from a statement temporary carries a
//! loan of that temporary, so a later use is rejected as a loan that outlives
//! its owner.
//!
//! Cases whose answer is a compile-time rejection assert the diagnostic code
//! and, where the message names a binding, the message text too. Some cases
//! pass before the implementation exists; they are the regression guards for
//! behavior that must not change.

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
            "clc-rvispec-{}-{tag}-{sequence}",
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

/// Asserts the run aborted at the runtime guard for `code`.
///
/// A trapped run exits nonzero and prints `error CODE:` on stderr.
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
///
/// A rejection that also names a binding asserts the message through
/// `assert_rejected_with`; this shorter form stays for the cases that do not.
#[allow(dead_code)]
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

/// A `for` loop over a call result walks a compiler-made copy of the array.
///
/// The call returns `[1, 2]`; summing the visited elements gives `3`.
#[test]
fn should_sum_a_call_result_when_a_for_loop_iterates_it() {
    let source = r#"module codegen.rvi.st1;

func mk(): i32[2] {
    return [1, 2];
}

func main(): i32 {
    let sum: i32 = 0;
    for (x in mk()) {
        sum = sum + x;
    }
    return sum;
}
"#;
    let output = run_host("rvi01", source);
    assert_exit("rvi01", &output, 3);
}

/// A `borrow` element loop over a call result writes into the compiler-made
/// copy of the rows.
///
/// Each visited row grows its first element by ten, so the two first-column
/// values are `11` and `14` and their sum is `25`. A write that landed in a
/// discarded copy would leave the loop reading the old values.
#[test]
fn should_write_materialized_rows_when_a_borrow_element_loop_iterates_a_call_result() {
    let source = r#"module codegen.rvi.st2;

func mkRows(): i32[2][3] {
    return [[1, 2, 3], [4, 5, 6]];
}

func main(): i32 {
    let total: i32 = 0;
    for (borrow r in mkRows()) {
        r[0] = r[0] + 10;
        total = total + r[0];
    }
    return total;
}
"#;
    let output = run_host("rvi02", source);
    assert_exit("rvi02", &output, 25);
}

/// An element of a call result is lent to a view parameter in the same
/// statement.
///
/// The call returns `"a"` and `"bb"`, and the borrowed second element has two
/// bytes. The temporary's owning elements are released after the statement;
/// the release itself is checked later by a sanitizer run.
#[test]
fn should_lend_a_call_result_element_when_it_is_passed_to_a_view() {
    let source = r#"module codegen.rvi.st3;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("a"), stringOf("bb")];
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    return vlen(mk()[1]);
}
"#;
    let output = run_host("rvi03", source);
    assert_exit("rvi03", &output, 2);
}

/// A `borrow` element loop over a call result reads every element, and the
/// materialized iterable is released after the loop.
///
/// The two strings hold one byte and two bytes, so the total is `3`.
#[test]
fn should_sum_element_lengths_when_a_borrow_element_loop_iterates_a_call_result() {
    let source = r#"module codegen.rvi.st4;

import { String, stringOf } from std.text;

func mkStrings(): String[2] {
    return [stringOf("a"), stringOf("bb")];
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let total: i32 = 0;
    for (borrow s in mkStrings()) {
        total = total + vlen(s);
    }
    return total;
}
"#;
    let output = run_host("rvi04", source);
    assert_exit("rvi04", &output, 3);
}

/// A fixed-array conditional with `true` stores the then arm.
///
/// The rows are `[1, 2]` and `[3, 4]`; the chosen row sums to `3`.
#[test]
fn should_choose_the_then_arm_when_a_conditional_initializes_a_scalar_array() {
    let source = r#"module codegen.rvi.st5;

func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let c: i32[2] = ok ? a : b;
    return c[0] + c[1];
}
"#;
    let output = run_host("rvi05", source);
    assert_exit("rvi05", &output, 3);
}

/// A conditional that moves an owned array keeps the chosen arm's elements in
/// the destination.
///
/// With `true`, `c` takes `a`'s elements, so the second length is `2`. Reading
/// either arm afterwards is the moved-value error pinned by the separate
/// rejection case.
#[test]
fn should_move_the_chosen_array_when_a_conditional_selects_one() {
    let source = r#"module codegen.rvi.st6;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let ok: bool = true;
    let a: String[2] = [stringOf("x"), stringOf("yy")];
    let b: String[2] = [stringOf("z"), stringOf("www")];
    let c: String[2] = ok ? a : b;
    return vlen(c[1]);
}
"#;
    let output = run_host("rvi06", source);
    assert_exit("rvi06", &output, 2);
}

/// A conditional that reassigns an owned array takes the else arm and releases
/// the old value once.
///
/// With `false`, `s` takes `t`'s elements, so the second length is `3`. A
/// double release of the old `s` would abort in the C library, so a clean exit
/// proves the release ran once.
#[test]
fn should_take_the_else_arm_when_a_conditional_reassigns_an_owned_array() {
    let source = r#"module codegen.rvi.st7;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let ok: bool = false;
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    let t: String[2] = [stringOf("fg"), stringOf("hij")];
    s = ok ? s : t;
    return vlen(s[1]);
}
"#;
    let output = run_host("rvi07", source);
    assert_exit("rvi07", &output, 3);
}

/// A conditional returned from a function produces the chosen arm.
///
/// With `false`, the returned row is `[3, 4]`, so the sum is `7`.
#[test]
fn should_return_the_chosen_arm_when_a_conditional_returns_an_array() {
    let source = r#"module codegen.rvi.st8;

func pick(ok: bool): i32[2] {
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return ok ? a : b;
}

func main(): i32 {
    let r: i32[2] = pick(false);
    return r[0] + r[1];
}
"#;
    let output = run_host("rvi08", source);
    assert_exit("rvi08", &output, 7);
}

/// A `take` argument that is a conditional passes the chosen arm.
///
/// With `false`, the callee receives `[3, 4]`, so the sum is `7`.
#[test]
fn should_pass_the_chosen_arm_when_a_conditional_is_a_take_argument() {
    let source = r#"module codegen.rvi.st9;

func eat(take arr: i32[2]): i32 {
    return arr[0] + arr[1];
}

func main(): i32 {
    let ok: bool = false;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return eat(ok ? a : b);
}
"#;
    let output = run_host("rvi09", source);
    assert_exit("rvi09", &output, 7);
}

/// The un-chosen arm of a conditional is never evaluated.
///
/// The else arm is a call that counts itself in a global and returns a
/// different pair. The chosen `[1, 2]` sums to `3` and the counter stays `0`,
/// so an arm that ran would change the answer.
#[test]
fn should_skip_the_unchosen_call_when_a_conditional_selects_an_array() {
    let source = r#"module codegen.rvi.st10;

let calls: i32 = 0;

func unwantedPair(): i32[2] {
    calls = calls + 1;
    return [7, 8];
}

func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let c: i32[2] = ok ? a : unwantedPair();
    return c[0] + c[1] + calls;
}
"#;
    let output = run_host("rvi10", source);
    assert_exit("rvi10", &output, 3);
}

/// A guarded statement whose right operand never runs releases nothing.
///
/// The left operand is `false`, so the array-returning call in the right
/// operand never runs; the unevaluated temporary must stay empty and its
/// release must be a no-op. The program exits `0`.
#[test]
fn should_skip_the_temporary_when_a_guard_makes_its_statement_dead() {
    let source = r#"module codegen.rvi.st11;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    if (false && vlen(mk()[0]) > 1) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("rvi11", source);
    assert_exit("rvi11", &output, 0);
}

/// The length of a call-result element initializes a local.
///
/// The call returns `"ab"` and `"cde"`, so the first element has two bytes. The
/// temporary's elements are released after the statement; the release itself is
/// checked later by a sanitizer run.
#[test]
fn should_read_a_call_result_element_when_its_length_initializes_a_local() {
    let source = r#"module codegen.rvi.st24;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let n: i32 = i32(len(mk()[0]));
    return n;
}
"#;
    let output = run_host("rvi24", source);
    assert_exit("rvi24", &output, 2);
}

/// Binding a view to a call-result element and using it later is rejected.
///
/// The element belongs to a temporary that is released at the end of its
/// statement, so the view's later use has no release-free path and the loan
/// outlives its owner.
#[test]
fn should_reject_a_view_use_when_the_view_derives_from_a_statement_temporary() {
    let source = r#"module codegen.rvi.st25;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let v: str = mk()[0];
    return i32(len(v));
}
"#;
    assert_rejected_with("rvi25", source, "E10030", &["does not live long enough"]);
}

/// A `take` argument that is a conditional consumes and releases the chosen
/// arm's elements.
///
/// With `false`, the callee receives the second array; the first byte of each
/// element there is `'f'` and `'h'`, so the callee sums `206` and the first
/// array would give `196`. Reading either arm afterwards is the moved-value
/// error pinned by the separate rejection case.
#[test]
fn should_consume_the_chosen_elements_when_a_conditional_is_a_take_argument() {
    let source = r#"module codegen.rvi.st26;

import { String, stringOf, bytes } from std.text;

func eat(take arr: String[2]): i32 {
    return i32(bytes(arr[0])[0]) + i32(bytes(arr[1])[0]);
}

func main(): i32 {
    let ok: bool = false;
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = [stringOf("fg"), stringOf("hij")];
    if (eat(ok ? a : b) != 206) {
        return 100;
    }
    return 0;
}
"#;
    let output = run_host("rvi26", source);
    assert_exit("rvi26", &output, 0);
}

/// Indexing a conditional array result reads the chosen row's element.
///
/// With `false`, the row is `[3, 4]` and the second element is `4`.
#[test]
fn should_index_the_chosen_row_when_a_conditional_result_is_indexed() {
    let source = r#"module codegen.rvi.st27;

func main(): i32 {
    let ok: bool = false;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let x: i32 = (ok ? a : b)[1];
    return x;
}
"#;
    let output = run_host("rvi27", source);
    assert_exit("rvi27", &output, 4);
}

/// A `for` loop over a conditional array result walks the chosen row.
///
/// With `false`, the row is `[3, 4]`, so the sum is `7`.
#[test]
fn should_iterate_the_chosen_row_when_a_conditional_is_a_loop_iterable() {
    let source = r#"module codegen.rvi.st28;

func main(): i32 {
    let ok: bool = false;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let sum: i32 = 0;
    for (x in ok ? a : b) {
        sum = sum + x;
    }
    return sum;
}
"#;
    let output = run_host("rvi28", source);
    assert_exit("rvi28", &output, 7);
}

/// A `borrow` argument that is a conditional reads the chosen arm's elements
/// while both arms keep owning their own.
///
/// With `false`, the callee reads the second array; the first byte of each
/// element there is `'f'` and `'h'`, so the callee sums `206` and the first
/// array would give `196`. The first array still owns `"ab"`, whose length is
/// `2`, and a clean exit proves neither arm is released twice.
#[test]
fn should_lend_the_chosen_elements_when_a_conditional_is_a_view_argument() {
    let source = r#"module codegen.rvi.st29;

import { String, stringOf, bytes } from std.text;

func eat(borrow arr: String[2]): i32 {
    return i32(bytes(arr[0])[0]) + i32(bytes(arr[1])[0]);
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let ok: bool = false;
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = [stringOf("fg"), stringOf("hij")];
    if (eat(ok ? a : b) != 206) {
        return 100;
    }
    return vlen(a[0]);
}
"#;
    let output = run_host("rvi29", source);
    assert_exit("rvi29", &output, 2);
}

/// A conditional that consumes an owned arm leaves either arm maybe-moved.
///
/// The condition selects one arm at runtime, so the checker cannot know which
/// arm the conditional emptied; reading either arm afterwards is the
/// moved-value error.
#[test]
fn should_reject_a_later_arm_use_when_a_conditional_moves_an_owned_arm() {
    let source = r#"module codegen.rvi.st30;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = false;
    let a: String[2] = [stringOf("x"), stringOf("yy")];
    let b: String[2] = [stringOf("z"), stringOf("www")];
    let c: String[2] = ok ? a : b;
    return i32(len(b[0]));
}
"#;
    assert_rejected_with("rvi30", source, "E10020", &["was moved"]);
}

/// A statement call through a field with an in-range receiver index runs.
///
/// The index selects the first entry of a two-entry array, so the call is
/// reachable and the program finishes at its final `0` without a trap.
#[test]
fn should_run_the_statement_call_when_a_field_receiver_index_is_in_range() {
    let source = r#"module codegen.rvi.st12;

struct Ops {
    run: func(i32): i32;
}

func bump(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let ops: Ops[2] = [Ops { run: bump }, Ops { run: bump }];
    let i: i32 = 0;
    ops[i].run(1);
    return 0;
}
"#;
    let output = run_host("rvi12", source);
    assert_exit("rvi12", &output, 0);
}

/// A statement call through a field traps before the call when the receiver
/// index is out of range.
///
/// The array holds two entries and the index is `5`, so the receiver's bounds
/// check must abort with the runtime fault before the field's function runs.
#[test]
fn should_trap_when_a_statement_call_field_receiver_index_is_out_of_range() {
    let source = r#"module codegen.rvi.st13;

struct Ops {
    run: func(i32): i32;
}

func bump(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let ops: Ops[2] = [Ops { run: bump }, Ops { run: bump }];
    let i: i32 = 5;
    ops[i].run(1);
    return 0;
}
"#;
    let output = run_host("rvi13", source);
    assert_trap("rvi13", &output, "E11211");
}

/// A field call whose fixed-array result initializes a local traps on an
/// out-of-range receiver index before the call.
///
/// The receiver index is `5` over a two-entry array, so the bounds check must
/// abort before the field's function runs to fill the destination array.
#[test]
fn should_trap_when_an_array_assign_field_call_receiver_index_is_out_of_range() {
    let source = r#"module codegen.rvi.st14;

struct Ops {
    run: func(i32): i32[2];
}

func pair(x: i32): i32[2] {
    return [x, x + 1];
}

func main(): i32 {
    let ops: Ops[2] = [Ops { run: pair }, Ops { run: pair }];
    let i: i32 = 5;
    let a: i32[2] = ops[i].run(1);
    return a[0];
}
"#;
    let output = run_host("rvi14", source);
    assert_trap("rvi14", &output, "E11211");
}

/// A field call returned as a fixed array traps on an out-of-range receiver
/// index before the call.
///
/// The receiver index is `5` over a two-entry array, so the bounds check must
/// abort before the field's function runs and writes the returned array.
#[test]
fn should_trap_when_a_returned_field_call_receiver_index_is_out_of_range() {
    let source = r#"module codegen.rvi.st15;

struct Ops {
    run: func(i32): i32[2];
}

func pair(x: i32): i32[2] {
    return [x, x + 1];
}

func dispatch(i: i32): i32[2] {
    let ops: Ops[2] = [Ops { run: pair }, Ops { run: pair }];
    return ops[i].run(1);
}

func main(): i32 {
    let a: i32[2] = dispatch(5);
    return a[0];
}
"#;
    let output = run_host("rvi15", source);
    assert_trap("rvi15", &output, "E11211");
}

/// A struct field that returns a fixed array produces the function's result.
///
/// The field holds `pair`, the call passes `3`, and `pair` returns `[3, 4]`,
/// so the second element is `4`. The emitted C must build under the strict
/// warning flags with the destination parameter carried by the field's type.
#[test]
fn should_return_the_second_element_when_a_struct_field_returns_a_fixed_array() {
    let source = r#"module codegen.rvi.st16;

struct Maker {
    make: func(i32): i32[2];
}

func pair(x: i32): i32[2] {
    return [x, x + 1];
}

func main(): i32 {
    let m: Maker = Maker { make: pair };
    let a: i32[2] = m.make(3);
    return a[1];
}
"#;
    let output = run_host("rvi16", source);
    assert_exit("rvi16", &output, 4);
}

/// A function stored in a local returns a fixed array, and a discarded call
/// through the same value leaves the stored result intact.
///
/// The stored call fills `a` with `[3, 4]`; the discarded call runs afterwards
/// and its result is dropped. The program returns `3 + 4`, so the discard must
/// not disturb the earlier array and the emitted C must build under the strict
/// warning flags.
#[test]
fn should_sum_the_stored_array_when_a_function_local_call_is_discarded() {
    let source = r#"module codegen.rvi.st17;

func pair(x: i32): i32[2] {
    return [x, x + 1];
}

func main(): i32 {
    let g: func(i32): i32[2] = pair;
    let a: i32[2] = g(3);
    g(3);
    return a[0] + a[1];
}
"#;
    let output = run_host("rvi17", source);
    assert_exit("rvi17", &output, 7);
}
