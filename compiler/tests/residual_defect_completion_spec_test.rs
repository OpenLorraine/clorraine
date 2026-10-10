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

/// Compiles `source` and returns the scratch directory, output, and C path.
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

/// A generic call inside a lifted lambda must be instantiated.
///
/// The lambda in `outer<i32>` calls `apply<i32>`; the lifted function's body
/// must carry the enclosing instance's substitution, so the emitted C names
/// `cl_apply__i32` rather than the unresolved parameter. The exit code `3`
/// proves the value travelled through both calls.
#[test]
fn should_instantiate_when_a_captureless_lambda_calls_a_generic() {
    let source = r#"module spec.rdc.st7;

func apply<T>(x: T): T {
    return x;
}

func outer<T>(x: T): T {
    let f: func(T): T = (v: T) => apply<T>(v);
    return f(x);
}

func main(): i32 {
    return outer<i32>(3);
}
"#;
    let output = run_host("rdc07", source);
    assert_exit("rdc07", &output, 3);
}

/// Two enclosing instances must each specialize the same lifted lambda.
///
/// One program instantiates `outer<i32>` and `outer<String>`. Both lifted
/// copies must build with their own substitution; the exit code `3` is the
/// `String` length plus the `i32` result minus three, so both calls ran. The
/// String result is measured in place, because binding it to an owning local
/// is a separate, pre-existing defect outside this behavior.
#[test]
fn should_build_when_two_enclosing_instances_specialize_the_same_lambda() {
    let source = r#"module spec.rdc.st8;

import { String, stringOf } from std.text;

func apply<T>(take x: T): T {
    return x;
}

func outer<T>(x: T): T {
    let f: func(T): T = (v: T) => apply<T>(v);
    return f(x);
}

func main(): i32 {
    let n: i32 = outer<i32>(3);
    return i32(len(outer<String>(stringOf("abc")))) + (n - 3);
}
"#;
    let output = run_host("rdc08", source);
    assert_exit("rdc08", &output, 3);
}

/// An owning instance of a parameter-typed capture return is rejected.
///
/// `outer<String>` captures its read-only parameter and returns it through an
/// immediate closure call; moving a borrowed capture out of the lambda is
/// rejected exactly as the concrete `String` spelling is, so the alias the
/// copy would create cannot be reached. One `E10401` names the capture and no
/// translation unit is written.
#[test]
fn should_reject_the_capture_return_under_an_owning_instance() {
    let source = r#"module spec.rdc.st9;

import { String, stringOf } from std.text;

func outer<T>(x: T): T {
    return ([x] () => x)();
}

func main(): i32 {
    return i32(len(outer<String>(stringOf("abc"))));
}
"#;
    assert_rejected("rdc09", source, "E10401");
}

/// A returned projection of a fresh call result must release the sibling.
///
/// `takea` returns `mkv().a`; the other field `b` must be released once, the
/// returned `"x"` kept, and a double release would abort before the exit.
#[test]
fn should_release_the_sibling_when_a_returned_projection_is_taken() {
    let source = r#"module spec.rdc.st10;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func takea(): String {
    return mkv().a;
}

func main(): i32 {
    let s: String = takea();
    return i32(len(s));
}
"#;
    let output = run_host("rdc10", source);
    assert_exit("rdc10", &output, 1);
}

/// A projection bound to a local must release the call result's siblings.
///
/// `let s: String = mkv().a` moves `a` into `s` and releases `b`; the exit code
/// is `"x"`'s length.
#[test]
fn should_release_the_sibling_when_a_projection_is_bound() {
    let source = r#"module spec.rdc.st11;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func main(): i32 {
    let s: String = mkv().a;
    return i32(len(s));
}
"#;
    let output = run_host("rdc11", source);
    assert_exit("rdc11", &output, 1);
}

/// A projection passed to a `take` parameter must release the sibling.
///
/// `sink(mkv().a)` hands `a` to the callee and releases `b` with the
/// temporary; the callee returns the received length, so the exit code is `1`.
#[test]
fn should_release_the_sibling_when_a_projection_is_passed_to_take() {
    let source = r#"module spec.rdc.st12;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    return sink(mkv().a);
}
"#;
    let output = run_host("rdc12", source);
    assert_exit("rdc12", &output, 1);
}

/// An ownerless element take under an owning substitution must free once.
///
/// `at<String>` takes element zero of `mk()`'s fresh array; the callee owns the
/// element and the temporary releases the remaining element, so the exit code
/// is the callee's constant and a double free would abort.
#[test]
fn should_release_the_element_once_when_an_ownerless_take_is_substituted() {
    let source = r#"module spec.rdc.st13;

import { String, stringOf } from std.text;

func sink<T>(take s: T): i32 {
    return 0;
}

func at<T>(mk: func(): T[2], i: i32): i32 {
    return sink<T>(mk()[i]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return at<String>(mkString, 0);
}
"#;
    let output = run_host("rdc13", source);
    assert_exit("rdc13", &output, 0);
}

/// A concrete ownerless element take keeps its rejection.
///
/// Without a generic substitution the element take has no place to mark, so the
/// front end still reports `E10026` and writes no translation unit.
#[test]
fn should_reject_when_a_concrete_ownerless_element_is_taken() {
    let source = r#"module spec.rdc.st14;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return 0;
}

func mkv(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return sink(mkv()[0]);
}
"#;
    assert_rejected("rdc14", source, "E10026");
}

/// A guarded re-store of a growable element must release the previous value.
///
/// The `while` test stores `grow()[0]` in a temporary it re-evaluates every
/// iteration. The program exits `2`; the previous element loop must be released
/// on each re-store, which the host close-out probe and the implementation
/// tests pin (the plain harness cannot see a leak).
#[test]
fn should_release_the_guarded_value_when_a_while_condition_stores_a_growable_element() {
    let source = r#"module spec.rdc.st15;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("rdc15", source);
    assert_exit("rdc15", &output, 2);
}

/// The guarded re-store also releases in the `do` spelling.
///
/// The condition re-runs after the body, so the same temporary is re-stored
/// each test; the body runs twice and the exit code is `2`.
#[test]
fn should_release_the_guarded_value_when_a_do_condition_stores_a_growable_element() {
    let source = r#"module spec.rdc.st16;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
    } while (i < 2 && grow()[0].count > 0);
    return i;
}
"#;
    let output = run_host("rdc16d", source);
    assert_exit("rdc16d", &output, 2);
}

/// The guarded re-store also releases in the `for` spelling.
///
/// The condition runs once per test with the step between, so the body runs
/// twice and the exit code is `2`.
#[test]
fn should_release_the_guarded_value_when_a_for_condition_stores_a_growable_element() {
    let source = r#"module spec.rdc.st16;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    for (i = 0; i < 2 && grow()[0].count > 0; i = i + 1) {
    }
    return i;
}
"#;
    let output = run_host("rdc16f", source);
    assert_exit("rdc16f", &output, 2);
}

/// A struct containing a growable element drops through the same hoist.
///
/// The condition stores a `Bag` whose `Array<String>` field needs an element
/// loop to drop; the body runs twice and the exit code is `2`.
#[test]
fn should_release_the_guarded_value_when_a_struct_contains_a_growable_element() {
    let source = r#"module spec.rdc.st17;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

struct Bag {
    items: Array<String>;
}

func grow(): Bag {
    let b: Bag = Bag { items: arrayNew<String>() };
    b.items.push(stringOf("ab"));
    return b;
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow().items.count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("rdc17", source);
    assert_exit("rdc17", &output, 2);
}

/// A guarded element that needs no drop keeps its count and shape.
///
/// `Array<i32>` drops without an element loop, so its expression-form drop is
/// already available and the emitted shape must not change; the exit code is
/// still `2`.
#[test]
fn should_keep_the_count_when_a_guarded_element_needs_no_drop() {
    let source = r#"module spec.rdc.st18;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32>[2] {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    let b: Array<i32> = arrayNew<i32>();
    b.push(2);
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let output = run_host("rdc18", source);
    assert_exit("rdc18", &output, 2);
}
