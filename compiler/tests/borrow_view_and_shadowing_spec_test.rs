//! Runnable specification tests for moves through a borrow element view and
//! for a shadowing initializer under a null narrowing.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The value under test travels
//! in `main`'s return code.
//!
//! The behavior under test is that a `for (borrow e in xs)` element view binds
//! as an alias, never as an owner: every ownership transfer through the view —
//! a `take` argument, an owned-local binding, a field path, a struct-literal
//! field, and a return — is a compile-time rejection with the element-move
//! diagnostic, while reads, lends, writes, and the copy-form loop binding stay
//! accepted. Moving out of a `borrow` parameter keeps its own diagnostic.
//!
//! The narrowing batch covers a declaration that shadows a narrowed nullable
//! binding: its initializer must resolve to the outer binding before the new
//! name exists, so the `let` and `const` shadowing forms compile under a
//! `!= null` check (and under the `else` of an `== null` check), while a use
//! outside any narrowing keeps its null rejection.
//!
//! Cases whose answer is a compile-time rejection assert the diagnostic code
//! and the message text. The rejection cases fail before the implementation
//! exists; the accepted cases are the regression guards for behavior that must
//! not change.

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
            "clc-bvsspec-{}-{tag}-{sequence}",
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

/// A `take` argument reading a borrow view is the v1 element-move limit.
///
/// The view never owns its element, so passing it to a `take` parameter would
/// transfer an element out of the array; the compiler must refuse it.
#[test]
fn should_report_the_element_move_limit_when_a_take_argument_reads_a_borrow_view() {
    let source = r#"module codegen.bvs.st1;

import { String, stringOf } from std.text;

func eat(take s: String): void { }

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    for (borrow e in xs) {
        eat(e);
    }
    return 0;
}
"#;
    assert_rejected_with(
        "bvs1",
        source,
        "E10026",
        &["partial moves are limited to top-level fields in v1"],
    );
}

/// Binding a borrow view to an owned local would move the element out.
///
/// The loop variable names the array's element, not a copy, so the initializer
/// is the v1 element-move limit and the compiler must refuse it.
#[test]
fn should_report_the_element_move_limit_when_a_borrow_view_initializes_an_owned_local() {
    let source = r#"module codegen.bvs.st2;

import { String, stringOf } from std.text;

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    for (borrow e in xs) {
        let s: String = e;
        if (len(s) == 0) { return 1; }
    }
    return 0;
}
"#;
    assert_rejected("bvs2", source, "E10026");
}

/// A field of a borrow-view element is still part of the borrowed value.
///
/// Moving `p.name` out of the view would steal a field of the array's element,
/// so the compiler must refuse it even though the path is a top-level field.
#[test]
fn should_report_the_element_move_limit_when_a_borrow_view_field_initializes_an_owned_local() {
    let source = r#"module codegen.bvs.st3;

import { String, stringOf } from std.text;

struct P { name: String; n: i32; }

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    for (borrow p in ps) {
        let s: String = p.name;
        if (len(s) == 0) { return 1; }
    }
    return 0;
}
"#;
    assert_rejected("bvs3", source, "E10026");
}

/// Returning a borrow view would move the element out of the array.
///
/// The view cannot outlive the loop iteration, so returning it must report the
/// v1 element-move limit.
#[test]
fn should_report_the_element_move_limit_when_a_borrow_view_is_returned() {
    let source = r#"module codegen.bvs.st4;

import { String, stringOf } from std.text;

func first(borrow xs: String[2]): String {
    for (borrow e in xs) {
        return e;
    }
    return stringOf("");
}

func main(): i32 { return i32(len(first(borrow [stringOf("a"), stringOf("bb")]))); }
"#;
    assert_rejected("bvs4", source, "E10026");
}

/// A struct-literal field reading a borrow view is an element move.
///
/// The literal would take ownership of the element through the view, so the
/// compiler must refuse it.
#[test]
fn should_report_the_element_move_limit_when_a_borrow_view_fills_an_aggregate_field() {
    let source = r#"module codegen.bvs.st5;

import { String, stringOf } from std.text;

struct Holder { s: String; }

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    for (borrow e in xs) {
        let h: Holder = Holder { s: e };
        if (len(h.s) == 0) { return 1; }
    }
    return 0;
}
"#;
    assert_rejected("bvs5", source, "E10026");
}

/// Lending a borrow view to a view parameter stays accepted.
///
/// `vlen` reads the string through a `str` view, which is a lend and not a
/// move, so the loop sums the lengths of every element.
#[test]
fn should_sum_element_lengths_when_a_borrow_view_is_lent_to_a_view_parameter() {
    let source = r#"module codegen.bvs.st6;

import { String, stringOf } from std.text;

func vlen(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    let total: i32 = 0;
    for (borrow e in xs) {
        total = total + vlen(e);
    }
    return total;
}
"#;
    let output = run_host("bvs6", source);
    assert_exit("bvs6", &output, 3);
}

/// Writing through a borrow row view reaches the array.
///
/// The element binding aliases the row, so the write is visible after the
/// loop; the two first-column values each gain ten.
#[test]
fn should_write_through_when_a_borrow_row_view_is_indexed() {
    let source = r#"module codegen.bvs.st7;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    for (borrow row in m) {
        row[0] = row[0] + 10;
    }
    return m[0][0] + m[1][0];
}
"#;
    let output = run_host("bvs7", source);
    assert_exit("bvs7", &output, 25);
}

/// A copy-form loop binding owns its copy.
///
/// Without `borrow`, every iteration copies the row into the binding, so
/// reading the copy is an owned read and the loop sums the first column.
#[test]
fn should_copy_each_row_when_the_element_binding_is_not_borrowed() {
    let source = r#"module codegen.bvs.st8;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let s: i32 = 0;
    for (row in m) {
        let copy: i32[3] = row;
        s = s + copy[0];
    }
    return s;
}
"#;
    let output = run_host("bvs8", source);
    assert_exit("bvs8", &output, 5);
}

/// A `take` argument reading a borrow parameter is still the borrow-move rule.
///
/// A parameter declared `borrow` never owns its value, so passing it on as a
/// `take` argument reports the parameter-specific diagnostic.
#[test]
fn should_report_the_borrow_move_when_a_take_argument_reads_a_borrow_parameter() {
    let source = r#"module codegen.bvs.st13a;

import { String, stringOf } from std.text;

func eat(take s: String): void { }

func demo(borrow s: String): void {
    eat(s);
}

func main(): i32 { return 0; }
"#;
    assert_rejected_with(
        "bvs13a",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
    );
}

/// An owned-local binding reading a borrow parameter is still rejected.
///
/// The initializer would move the parameter's value into the local, which the
/// borrow mode forbids; the parameter-specific diagnostic reports it.
#[test]
fn should_report_the_borrow_move_when_a_borrow_parameter_initializes_an_owned_local() {
    let source = r#"module codegen.bvs.st13b;

import { String, stringOf } from std.text;

func demo(borrow s: String): void {
    let t: String = s;
    if (len(t) == 0) { return; }
}

func main(): i32 { return 0; }
"#;
    assert_rejected("bvs13b", source, "E10034");
}

/// Returning a borrow parameter is still rejected as a borrow move.
///
/// The function does not own the parameter's value, so returning it reports the
/// parameter-specific diagnostic.
#[test]
fn should_report_the_borrow_move_when_a_borrow_parameter_is_returned() {
    let source = r#"module codegen.bvs.st13c;

import { String, stringOf } from std.text;

func leave(borrow s: String): String {
    return s;
}

func main(): i32 { return 0; }
"#;
    assert_rejected("bvs13c", source, "E10034");
}

/// A shadowing `let` initializer reads the narrowed outer binding.
///
/// The `!= null` check narrows the outer `b`, and the declaration's own
/// initializer must see that narrowing: its name resolves before the new
/// binding exists, so the program compiles and returns zero.
#[test]
fn should_accept_when_a_shadowing_let_initializer_reads_the_narrowed_outer_binding() {
    let source = r#"module codegen.bvs.st9;

import { Buffer, bufferNew } from std.mem;

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    if (b != null) {
        let b: Buffer = b;
        return 0;
    }
    return 1;
}
"#;
    let output = run_host("bvs9", source);
    assert_exit("bvs9", &output, 0);
}

/// A shadowing `const` initializer reads the narrowed outer binding too.
///
/// The declaration order is the same for `const`, so the narrowed outer value
/// flows into the new immutable binding and the program returns zero.
#[test]
fn should_accept_when_a_shadowing_const_initializer_reads_the_narrowed_outer_binding() {
    let source = r#"module codegen.bvs.st10;

import { Buffer, bufferNew } from std.mem;

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    if (b != null) {
        const b: Buffer = b;
        return 0;
    }
    return 1;
}
"#;
    let output = run_host("bvs10", source);
    assert_exit("bvs10", &output, 0);
}

/// A nullable binding used without a narrowing check keeps its rejection.
///
/// The call reaches a parameter that needs a non-null `Buffer`, and no
/// `!= null` check narrows `b`, so the compiler reports the null-use error at
/// the argument.
#[test]
fn should_report_the_null_use_when_a_nullable_binding_is_used_without_a_narrowing_check() {
    let source = r#"module codegen.bvs.st11;

import { Buffer, bufferNew } from std.mem;

func use(b: Buffer): i32 { return 0; }

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    return use(b);
}
"#;
    assert_rejected_with(
        "bvs11",
        source,
        "E10010",
        &["value 'b' may be null here"],
    );
}

/// A narrowing check alone makes the use valid.
///
/// The `!= null` check narrows the outer binding for the body, and the use
/// inside it needs no shadowing declaration; the program returns zero.
#[test]
fn should_accept_when_a_narrowing_check_guards_the_use_without_shadowing() {
    let source = r#"module codegen.bvs.st12;

import { Buffer, bufferNew } from std.mem;

func use(b: Buffer): i32 { return 0; }

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    if (b != null) {
        return use(b);
    }
    return 1;
}
"#;
    let output = run_host("bvs12", source);
    assert_exit("bvs12", &output, 0);
}

/// The `else` of an `== null` check narrows the shadowing initializer too.
///
/// Inside the `else` branch the outer `b` is known non-null, so the shadowing
/// declaration's initializer reads the narrowed outer binding and the program
/// compiles.
#[test]
fn should_accept_when_a_shadowing_initializer_reads_the_else_branch_narrowing() {
    let source = r#"module codegen.bvs.st14;

import { Buffer, bufferNew } from std.mem;

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    if (b == null) {
        return 1;
    } else {
        let b: Buffer = b;
        return 0;
    }
}
"#;
    let output = run_host("bvs14", source);
    assert_exit("bvs14", &output, 0);
}
