//! Runnable specification tests for fixed-array values and length.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The value under test
//! travels in `main`'s return code, so a case that prints nothing states its
//! answer as the exit code.
//!
//! The behavior under test is that a fixed array is a first-class value: a
//! duplicable array copies, a move-only array moves so the source empties, a
//! whole-array reassignment releases the old elements exactly once, and a
//! self-referencing call keeps the old value valid until the call returns.
//! `len` reports the outermost element count for a multi-extent array and the
//! byte count for an owned `String`.
//!
//! The element-loop batch makes the element form of `for` walk rows: a copy
//! loop binds each row by value, a `borrow` loop writes through the row view,
//! a slice of rows iterates its rows, nested loops visit every scalar once, and
//! a copy loop over non-duplicable rows reports the row type by name.
//!
//! The projection batch makes a projection over an array-returning call run
//! the call once into a compiler temporary and then read or lend the element;
//! a guarded operand keeps the call lazy. Moving a non-duplicable element out
//! of an array stays a compile-time limit in every move form — initializer,
//! reassignment, field-target assignment, struct-literal field, `take`
//! argument, and return — and the unchanged lengths of a string literal and a
//! growable sequence are pinned as guards.
//!
//! Cases whose answer is a compile-time rejection assert the diagnostic code
//! and, where the message names a type or binding, the message text too. Some
//! of those rejections pass before the implementation exists; they are the
//! regression guards for behavior that must not change.

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
            "clc-avspec-{}-{tag}-{sequence}",
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
    run_with_timeout_env(tag, program, &[])
}

/// Runs a built program with a time limit and the given environment.
fn run_with_timeout_env(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut child = Command::new(program)
        .envs(env.iter().copied())
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

/// Builds the emitted C with AddressSanitizer and the strict flag set.
fn build_host_c_asan(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build-asan.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog-asan");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
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
        "{tag}: the host C compiler rejected the sanitizer build; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
fn run_asan(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, &scratch, source);
    let program = build_host_c_asan(tag, &scratch, &emitted);
    run_with_timeout_env(tag, &program, &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")])
}

/// Runs a memory-safety case under AddressSanitizer when `RDC_ASAN` is set,
/// otherwise as an ordinary strict host build.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
}

/// Asserts the run produced no AddressSanitizer report.
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
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

/// A multi-extent array reports its outermost element count to `len`.
///
/// The array holds two rows of three, so the length is two, not six. The
/// element read keeps the array a used value in the emitted C; the exit code
/// is the length alone.
#[test]
fn should_report_the_row_count_when_len_reads_a_multi_extent_array() {
    let source = r#"module codegen.av.st1;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    if (m[0][0] != 1) {
        return 100;
    }
    return i32(len(m));
}
"#;
    let output = run_host("av1", source);
    assert_exit("av1", &output, 2);
}

/// Binding a duplicable array to a second local copies it.
///
/// The copy sums its two elements; the source is read afterwards, so a copy
/// that consumed the source fails the guard instead of reporting three.
#[test]
fn should_copy_a_duplicable_array_when_bound_to_a_second_local() {
    let source = r#"module codegen.av.st2;

func main(): i32 {
    let a: i32[2] = [1, 2];
    let b: i32[2] = a;
    if (a[0] + a[1] != 3) {
        return 100;
    }
    return b[0] + b[1];
}
"#;
    let output = run_host("av2", source);
    assert_exit("av2", &output, 3);
}

/// A fixed-array field of a struct copies when bound to a local.
///
/// The struct owns the array; the local receives a byte copy, and the sum of
/// its elements is three.
#[test]
fn should_copy_a_struct_array_field_when_bound_to_a_local() {
    let source = r#"module codegen.av.st3;

struct S {
    v: i32[2];
}

func main(): i32 {
    let s: S = S { v: [1, 2] };
    let c: i32[2] = s.v;
    return c[0] + c[1];
}
"#;
    let output = run_host("av3", source);
    assert_exit("av3", &output, 3);
}

/// A selected row copies when bound to a row-typed local.
///
/// The first row of the two-by-three array is `[1, 2, 3]`; its sum is six.
#[test]
fn should_copy_a_row_when_bound_to_a_row_local() {
    let source = r#"module codegen.av.st4;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let r: i32[3] = m[0];
    return r[0] + r[1] + r[2];
}
"#;
    let output = run_host("av4", source);
    assert_exit("av4", &output, 6);
}

/// Moving a `String` array into a second local empties the source without
/// double-freeing its elements.
///
/// The second element of the destination is "cde", so the length is three and
/// a clean exit proves the two handles were released once each.
#[test]
fn should_move_a_string_array_when_bound_to_a_second_local() {
    let source = r#"module codegen.av.st5;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = a;
    return i32(len(b[1]));
}
"#;
    let output = run_host("av5", source);
    assert_exit("av5", &output, 3);
}

/// A second use of a moved array reports the moved-binding error and names it.
#[test]
fn should_reject_a_second_use_of_a_moved_array() {
    let source = r#"module codegen.av.st6;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = a;
    let c: String[2] = a;
    return 0;
}
"#;
    assert_rejected_with("av6", source, "E10020", &["'a' was moved and cannot be used"]);
}

/// Reassigning a `String` array releases the old elements before the new value
/// arrives.
///
/// The destination ends up holding the source array, so the length of its
/// second element is three; a clean exit proves the old elements were released
/// exactly once.
#[test]
fn should_release_the_old_elements_when_an_array_is_reassigned() {
    let source = r#"module codegen.av.st7;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = [stringOf("pq"), stringOf("rs")];
    b = a;
    return i32(len(b[1]));
}
"#;
    let output = run_host("av7", source);
    assert_exit("av7", &output, 3);
}

/// Assigning an array to itself keeps its elements intact.
///
/// The assignment saves the old value first, stores the new one, and releases
/// the saved value; the array still sums to three.
#[test]
fn should_keep_an_array_intact_when_it_is_assigned_to_itself() {
    let source = r#"module codegen.av.st8;

func main(): i32 {
    let a: i32[2] = [1, 2];
    a = a;
    return a[0] + a[1];
}
"#;
    let output = run_host("av8", source);
    assert_exit("av8", &output, 3);
}

/// An array returned by a call initializes a local by move.
///
/// The callee fills the hidden out-parameter and the caller binds it; the sum
/// of the two elements is three. This case passes before the copy path exists,
/// because the call shape already routes through the out-parameter.
#[test]
fn should_bind_a_call_result_when_it_initializes_an_array_local() {
    let source = r#"module codegen.av.st9;

func mk(): i32[2] {
    return [1, 2];
}

func main(): i32 {
    let b: i32[2] = mk();
    return b[0] + b[1];
}
"#;
    let output = run_host("av9", source);
    assert_exit("av9", &output, 3);
}

/// An owned `String` reports its byte count to `len`, in both spellings.
///
/// The literal and the binding hold the same five bytes, so the two lengths
/// sum to ten.
#[test]
fn should_report_the_byte_count_when_len_reads_a_string() {
    let source = r#"module codegen.av.st22;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("hello");
    return i32(len(stringOf("hello"))) + i32(s.len());
}
"#;
    let output = run_host("av22", source);
    assert_exit("av22", &output, 10);
}

/// A call on the right of an array reassignment replaces the old elements and
/// drops them once.
///
/// The destination ends up holding the call's result, so the length of its
/// second element is two; a clean exit proves the old elements were released
/// exactly once.
#[test]
fn should_read_the_new_elements_when_a_call_reassigns_an_array() {
    let source = r#"module codegen.av.st25;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("pq"), stringOf("rs")];
}

func main(): i32 {
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    s = mk();
    return i32(len(s[1]));
}
"#;
    let output = run_host("av25", source);
    assert_exit("av25", &output, 2);
}

/// A call that reads the target keeps the old value valid during the call.
///
/// The callee builds its result from the old elements, swapping them; the
/// destination then holds "cde" first, so its first length is three and a
/// clean exit proves the old value was released after the call.
#[test]
fn should_read_the_copied_elements_when_a_call_reassigns_an_array_from_itself() {
    let source = r#"module codegen.av.st26;

import { String, stringOf } from std.text;

func dup(borrow source: String[2]): String[2] {
    return [stringOf(source[1]), stringOf(source[0])];
}

func main(): i32 {
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    s = dup(borrow s);
    return i32(len(s[0]));
}
"#;
    let output = run_host("av26", source);
    assert_exit("av26", &output, 3);
}

/// A copy element loop walks rows and sums all three columns of each.
///
/// The two rows sum to `1+2+3 + 4+5+6 = 21`.
#[test]
fn should_sum_each_row_when_the_element_loop_copies_rows() {
    let source = r#"module codegen.av.st10;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let s: i32 = 0;
    for (row in m) {
        s = s + row[0] + row[1] + row[2];
    }
    return s;
}
"#;
    let output = run_host("av10", source);
    assert_exit("av10", &output, 21);
}

/// A borrow element loop writes through the row view into the array.
///
/// Each row's first element grows by ten, so the two first-column values are
/// `11` and `14` and the sum is `25`.
#[test]
fn should_write_through_the_view_when_a_borrow_element_loop_updates_a_row() {
    let source = r#"module codegen.av.st11;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    for (borrow row in m) {
        row[0] = row[0] + 10;
    }
    return m[0][0] + m[1][0];
}
"#;
    let output = run_host("av11", source);
    assert_exit("av11", &output, 25);
}

/// An element loop over three extents runs once per outermost row.
///
/// Each of the two rows contributes its first corner plus its far corner:
/// `1+4` and `5+8`, so the sum is `18` and a six-iteration walk is wrong.
#[test]
fn should_iterate_once_per_row_when_rows_have_inner_extents() {
    let source = r#"module codegen.av.st12;

func main(): i32 {
    let m3: i32[2][2][2] = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]];
    let s: i32 = 0;
    for (row in m3) {
        s = s + row[0][0] + row[1][1];
    }
    return s;
}
"#;
    let output = run_host("av12", source);
    assert_exit("av12", &output, 18);
}

/// An element loop over a slice of rows walks the rows it views.
///
/// The slice contains both rows of the source array, and each contributes its
/// second column: `2 + 5 = 7`.
#[test]
fn should_sum_the_second_column_when_the_iterable_is_a_slice_of_rows() {
    let source = r#"module codegen.av.st13;

import { slice } from std.mem;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let xs: i32[][3] = slice(m, 0, len(m));
    let s: i32 = 0;
    for (row in xs) {
        s = s + row[1];
    }
    return s;
}
"#;
    let output = run_host("av13", source);
    assert_exit("av13", &output, 7);
}

/// A copy element loop over non-duplicable rows reports E10040 and names the
/// row type.
#[test]
fn should_reject_a_copy_element_loop_when_the_row_is_not_duplicable() {
    let source = r#"module codegen.av.st14;

import { String, stringOf } from std.text;

func main(): i32 {
    let fs: String[2][2] = [[stringOf("a"), stringOf("b")], [stringOf("c"), stringOf("d")]];
    for (row in fs) {
    }
    return 0;
}
"#;
    assert_rejected_with(
        "av14",
        source,
        "E10040",
        &["non-duplicable 'String[2]'"],
    );
}

/// Nested element loops visit every scalar of a two-by-three array once.
///
/// The outer loop walks rows and the inner loop walks each row's elements, so
/// the sum is `21`; a mis-selected inner form never advances or visits the
/// scalars as rows.
#[test]
fn should_sum_every_scalar_when_element_loops_are_nested_over_rows() {
    let source = r#"module codegen.av.st15;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let s: i32 = 0;
    for (row in m) {
        for (x in row) {
            s = s + x;
        }
    }
    return s;
}
"#;
    let output = run_host("av15", source);
    assert_exit("av15", &output, 21);
}

/// A row slice that starts at one selects row one.
///
/// The slice covers the second row only, so its first element is four; an
/// offset counted in scalars would land inside row zero instead.
#[test]
fn should_select_row_one_when_a_row_slice_starts_at_one() {
    let source = r#"module codegen.av.st31;

import { slice } from std.mem;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let xs: i32[][3] = slice(m, 1, 2);
    return xs[0][0];
}
"#;
    let output = run_host("av31", source);
    assert_exit("av31", &output, 4);
}

/// A sub-slice of a row slice keeps counting rows.
///
/// The first slice covers both rows and the second selects the last row, so
/// the third element of its first row is six; an offset counted in scalars
/// would read across a row boundary.
#[test]
fn should_select_row_one_when_a_row_slice_is_sub_sliced() {
    let source = r#"module codegen.av.st32;

import { slice } from std.mem;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let a: i32[][3] = slice(m, 0, 2);
    let b: i32[][3] = slice(a, 1, 2);
    return b[0][2];
}
"#;
    let output = run_host("av32", source);
    assert_exit("av32", &output, 6);
}

/// A row slice wider than its storage traps at the slice guard.
///
/// The array holds two rows and the request asks for three, so building the
/// slice must trap before any row is read or written.
#[test]
fn should_trap_when_a_row_slice_asks_for_more_rows_than_exist() {
    let source = r#"module codegen.av.st33;

import { slice } from std.mem;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let xs: i32[][3] = slice(m, 0, 3);
    return 0;
}
"#;
    let output = run_host("av33", source);
    assert_trap("av33", &output, "E11211");
}

/// Indexing a call result reads the element without rerunning the call.
///
/// The call returns `[7, 9]` and the projection selects the second element, so
/// the answer is nine and the call must be written into a temporary first.
#[test]
fn should_read_the_second_element_when_a_call_result_is_indexed() {
    let source = r#"module codegen.av.st16;

func mk(): i32[2] {
    return [7, 9];
}

func main(): i32 {
    let x: i32 = mk()[1];
    return x;
}
"#;
    let output = run_host("av16", source);
    assert_exit("av16", &output, 9);
}

/// Chained indexing of a call result selects the inner element.
///
/// The call returns a two-by-three array and the projection selects row one,
/// column two, which is six.
#[test]
fn should_read_the_inner_element_when_a_call_result_is_indexed_twice() {
    let source = r#"module codegen.av.st17;

func mk2(): i32[2][3] {
    return [[1, 2, 3], [4, 5, 6]];
}

func main(): i32 {
    return mk2()[1][2];
}
"#;
    let output = run_host("av17", source);
    assert_exit("av17", &output, 6);
}

/// A string element of a call result is lent to a view parameter.
///
/// The call returns `["ab", "cde"]`, the second element is lent, and its
/// length is three; the call must run once behind the lend.
#[test]
fn should_read_the_string_element_when_a_call_result_is_passed_to_a_view() {
    let source = r#"module codegen.av.st18;

import { String, stringOf } from std.text;

func vlen(borrow s: String): i32 {
    return i32(len(s));
}

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return vlen(mk()[1]);
}
"#;
    let output = run_host("av18", source);
    assert_exit("av18", &output, 3);
}

/// Binding an element of a call result to an owned local is rejected.
///
/// The element has no owner that outlives the statement, so moving it out is
/// the element-move limit and reports E10026.
#[test]
fn should_reject_binding_an_element_when_the_array_comes_from_a_call() {
    let source = r#"module codegen.av.st19;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let s: String = mk()[0];
    return i32(len(s));
}
"#;
    assert_rejected("av19", source, "E10026");
}

/// Moving an element of a call result into a `take` argument is rejected.
#[test]
fn should_reject_a_take_argument_when_the_element_comes_from_a_call() {
    let source = r#"module codegen.av.st20;

import { String, stringOf } from std.text;

func consume(take s: String): void {
}

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    consume(take mk()[0]);
    return 0;
}
"#;
    assert_rejected("av20", source, "E10026");
}

/// Returning an element of a call result from an owning return type is
/// rejected.
#[test]
fn should_reject_a_return_when_the_element_comes_from_a_call() {
    let source = r#"module codegen.av.st21;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func first(): String {
    return mk()[0];
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected("av21", source, "E10026");
}

/// A guarded projection skips the call when the guard is false.
///
/// The left operand is false, so the array-returning call in the right operand
/// never runs and the program exits cleanly.
#[test]
fn should_skip_the_call_when_the_guarded_operand_is_not_evaluated() {
    let source = r#"module codegen.av.st23;

func boom(): i32[2] {
    panic("boom");
}

func main(): i32 {
    let ok: bool = false;
    if (ok && boom()[0] == 1) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("av23", source);
    assert_exit("av23", &output, 0);
}

/// `len` keeps its existing answers for a literal, an inferred array, and a
/// growable sequence.
///
/// The literal has five bytes, the inferred array has two elements, and the
/// growable sequence holds three, so the sum is ten. This case passes before
/// the fix and guards the unchanged paths.
#[test]
fn should_report_string_literal_and_growable_lengths() {
    let source = r#"module codegen.av.st24;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let g: Array<i32> = arrayNew<i32>();
    g.push(1);
    g.push(2);
    g.push(3);
    return i32(len("hello")) + i32(len([1, 2])) + i32(len(g));
}
"#;
    let output = run_host("av24", source);
    assert_exit("av24", &output, 10);
}

/// Reassigning an owning local from a call-result element is rejected.
#[test]
fn should_reject_a_reassignment_when_the_element_comes_from_a_call() {
    let source = r#"module codegen.av.st27;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let s: String = stringOf("x");
    s = mk()[0];
    return i32(len(s));
}
"#;
    assert_rejected("av27", source, "E10026");
}

/// A struct-literal field initialized from a call-result element is rejected.
#[test]
fn should_reject_a_struct_literal_field_when_the_element_comes_from_a_call() {
    let source = r#"module codegen.av.st28;

import { String, stringOf } from std.text;

struct Holder {
    s: String;
}

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let h: Holder = Holder { s: mk()[0] };
    return 0;
}
"#;
    assert_rejected("av28", source, "E10026");
}

/// A field-target assignment from a call-result element is rejected.
#[test]
fn should_reject_a_field_target_assignment_when_the_element_comes_from_a_call() {
    let source = r#"module codegen.av.st29;

import { String, stringOf } from std.text;

struct Holder {
    s: String;
}

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let h: Holder = Holder { s: stringOf("x") };
    h.s = mk()[0];
    return 0;
}
"#;
    assert_rejected("av29", source, "E10026");
}

/// A field-target assignment from a place element is rejected.
///
/// The element belongs to a local array, but the target is a projected field,
/// so the assignment is a partial move of a non-top-level place and reports
/// E10026. The program is accepted before the fix, which is the defect.
#[test]
fn should_reject_a_field_target_assignment_when_the_element_reads_a_place() {
    let source = r#"module codegen.av.st30;

import { String, stringOf } from std.text;

struct Holder {
    s: String;
}

func main(): i32 {
    let a: String[2] = [stringOf("a"), stringOf("bb")];
    let h: Holder = Holder { s: stringOf("x") };
    h.s = a[0];
    return 0;
}
"#;
    assert_rejected("av30", source, "E10026");
}

// ---------------------------------------------------------------------------
// viewlengthcontrol: the frozen view-length matrix and the filed scalar-`len`
// probe.
// ---------------------------------------------------------------------------

/// The view-length matrix is green and the filed double-index probe is clean.
///
/// `viewlengthmatrix` checks the single-index element length, the concrete and
/// runtime index, the owned `String` length, and a loop-alias element read.
/// The filed `viewlengthfiled` reads `len(w.items[i][0])`, a `u8`; the run is
/// accepted and memory-clean, while its scalar fallback value is recorded in
/// the implementation pin rather than asserted here.
#[test]
fn should_report_the_view_length_matrix_and_accept_the_filed_scalar_len_probe() {
    let matrix = r#"module spec.avi.viewlengthcontrol;

import { String, stringOf } from std.text;

struct W { items: str[2]; name: String; }

func main(): i32 {
    let w: W = W { items: ["AA", "CC"], name: stringOf("dd") };
    let i: i32 = 1;
    if (i32(len(w.items[i])) != 2) { return 1; }
    if (i32(len(w.items[0])) != 2) { return 2; }
    if (i32(len(w.name)) != 2) { return 3; }
    for (borrow e in w.items) {
        if (i32(len(e)) != 2) { return 4; }
    }
    return 0;
}
"#;
    let output = run_memory("viewlengthcontrol", matrix);
    assert_no_asan("viewlengthcontrol", &output);
    assert_exit("viewlengthcontrol", &output, 0);

    let filed = r#"module spec.avi.viewlengthfiled;

struct W { items: str[2]; }

func get(w: W, i: i32): i32 {
    return i32(len(w.items[i][0])) - 2;
}

func main(): i32 {
    let w: W = W { items: ["AA", "CC"] };
    return get(w, 1);
}
"#;
    let output = run_memory("viewlengthfiled", filed);
    assert_no_asan("viewlengthfiled", &output);
}

/// A folded `len` operand still evaluates a non-call effectful shape.
///
/// The extent is structural, but a conditional runs only the selected arm, an
/// array literal runs each element expression once, and a run-time index runs
/// its index expression once. At the phase base each shape folded to the
/// constant and dropped its effects.
#[test]
fn should_evaluate_a_non_call_folded_len_operand() {
    let conditional_false = r#"module spec.avi.lencondfalse;

let hits: i32 = 0;
let which: i32 = 0;

func mkA(): i32[2] {
    hits = hits + 1;
    which = 1;
    return [20, 21];
}

func mkB(): i32[2] {
    hits = hits + 1;
    which = 2;
    return [20, 21];
}

func main(): i32 {
    let run: bool = false;
    let n: usize = std.mem.len(run ? mkA() : mkB());
    return i32(n) - 2 + hits - 1 + which - 2;
}
"#;
    let output = run_memory("lencondfalse", conditional_false);
    assert_no_asan("lencondfalse", &output);
    assert_exit("lencondfalse", &output, 0);

    let conditional_true = r#"module spec.avi.lencondtrue;

let hits: i32 = 0;
let which: i32 = 0;

func mkA(): i32[2] {
    hits = hits + 1;
    which = 1;
    return [20, 21];
}

func mkB(): i32[2] {
    hits = hits + 1;
    which = 2;
    return [20, 21];
}

func main(): i32 {
    let run: bool = true;
    let n: usize = std.mem.len(run ? mkA() : mkB());
    return i32(n) - 2 + hits - 1 + which - 1;
}
"#;
    let output = run_memory("lencondtrue", conditional_true);
    assert_no_asan("lencondtrue", &output);
    assert_exit("lencondtrue", &output, 0);

    let literal = r#"module spec.avi.lenliteral;

let hits: i32 = 0;
let which: i32 = 0;

func mkA(): i32[2] {
    hits = hits + 1;
    which = 1;
    return [20, 21];
}

func mkB(): i32[2] {
    hits = hits + 1;
    which = 2;
    return [20, 21];
}

func main(): i32 {
    let n: usize = std.mem.len([mkA(), mkB()]);
    return i32(n) - 2 + hits - 2 + which - 2;
}
"#;
    let output = run_memory("lenliteral", literal);
    assert_no_asan("lenliteral", &output);
    assert_exit("lenliteral", &output, 0);

    let index = r#"module spec.avi.lenindex;

let hits: i32 = 0;

func bump(): i32 {
    hits = hits + 1;
    return 0;
}

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [20, 21]];
    let n: usize = std.mem.len(a[bump()]);
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenindex", index);
    assert_no_asan("lenindex", &output);
    assert_exit("lenindex", &output, 0);
}

/// Every `len` spelling evaluates a non-call operand's effects.
///
/// The bare `len(...)`, the dot `xs.len()`, and the qualified `std.mem.len`
/// agree: a conditional runs only the selected arm, a literal runs each
/// element once, and a run-time index runs its index expression and bounds
/// guard once. A call receiver evaluates its call once, and an owned call
/// result is released. A conditional with a caller-backed arm never moves that
/// arm: its live view stays valid.
#[test]
fn should_evaluate_a_non_call_folded_len_operand_in_every_spelling() {
    let bare_conditional = r#"module spec.avi.lenbarecond;

let hits: i32 = 0;
let which: i32 = 0;

func mkA(): i32[2] { hits = hits + 1; which = 1; return [20, 21]; }
func mkB(): i32[2] { hits = hits + 1; which = 2; return [20, 21]; }

func main(): i32 {
    let run: bool = true;
    let n: usize = len(run ? mkA() : mkB());
    return i32(n) - 2 + hits - 1 + which - 1;
}
"#;
    let output = run_memory("lenbarecond", bare_conditional);
    assert_no_asan("lenbarecond", &output);
    assert_exit("lenbarecond", &output, 0);

    let bare_literal = r#"module spec.avi.lenbarelit;

let hits: i32 = 0;

func mkA(): i32[2] { hits = hits + 1; return [20, 21]; }
func mkB(): i32[2] { hits = hits + 1; return [20, 21]; }

func main(): i32 {
    let n: usize = len([mkA(), mkB()]);
    return i32(n) - 2 + hits - 2;
}
"#;
    let output = run_memory("lenbarelit", bare_literal);
    assert_no_asan("lenbarelit", &output);
    assert_exit("lenbarelit", &output, 0);

    let bare_index = r#"module spec.avi.lenbareidx;

let hits: i32 = 0;

func bump(): i32 { hits = hits + 1; return 0; }

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [20, 21]];
    let n: usize = len(a[bump()]);
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenbareidx", bare_index);
    assert_no_asan("lenbareidx", &output);
    assert_exit("lenbareidx", &output, 0);

    let dot_conditional = r#"module spec.avi.lendotcond;

let hits: i32 = 0;
let which: i32 = 0;

func mkA(): i32[2] { hits = hits + 1; which = 1; return [20, 21]; }
func mkB(): i32[2] { hits = hits + 1; which = 2; return [20, 21]; }

func main(): i32 {
    let run: bool = true;
    let n: usize = (run ? mkA() : mkB()).len();
    return i32(n) - 2 + hits - 1 + which - 1;
}
"#;
    let output = run_memory("lendotcond", dot_conditional);
    assert_no_asan("lendotcond", &output);
    assert_exit("lendotcond", &output, 0);

    let dot_call = r#"module spec.avi.lendotcall;

let hits: i32 = 0;

func mk(): i32[2] { hits = hits + 1; return [20, 21]; }

func main(): i32 {
    let n: usize = mk().len();
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lendotcall", dot_call);
    assert_no_asan("lendotcall", &output);
    assert_exit("lendotcall", &output, 0);

    let dot_index = r#"module spec.avi.lendotidx;

let hits: i32 = 0;

func bump(): i32 { hits = hits + 1; return 0; }

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [20, 21]];
    let n: usize = a[bump()].len();
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lendotidx", dot_index);
    assert_no_asan("lendotidx", &output);
    assert_exit("lendotidx", &output, 0);

    let owned_bare = r#"module spec.avi.lenownedbare;

import { String, stringOf } from std.text;

let hits: i32 = 0;

func mk(): String[2] {
    hits = hits + 1;
    return [stringOf("aa"), stringOf("bb")];
}

func main(): i32 {
    let n: usize = len(mk());
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenownedbare", owned_bare);
    assert_no_asan("lenownedbare", &output);
    assert_exit("lenownedbare", &output, 0);

    let owned_dot = r#"module spec.avi.lenowneddot;

import { String, stringOf } from std.text;

let hits: i32 = 0;

func mk(): String[2] {
    hits = hits + 1;
    return [stringOf("aa"), stringOf("bb")];
}

func main(): i32 {
    let n: usize = mk().len();
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenowneddot", owned_dot);
    assert_no_asan("lenowneddot", &output);
    assert_exit("lenowneddot", &output, 0);

    let mixed_bare = r#"module spec.avi.lenmixedbare;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("cc"), stringOf("dd")];
}

func main(): i32 {
    let a: String[2] = [stringOf("aa"), stringOf("bb")];
    let v: str = a[0];
    let flag: bool = true;
    let n: usize = len(flag ? a : mk());
    return i32(n) - 2 + i32(v[0]) - 97;
}
"#;
    let output = run_memory("lenmixedbare", mixed_bare);
    assert_no_asan("lenmixedbare", &output);
    assert_exit("lenmixedbare", &output, 0);

    let mixed_dot = r#"module spec.avi.lenmixeddot;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("cc"), stringOf("dd")];
}

func main(): i32 {
    let a: String[2] = [stringOf("aa"), stringOf("bb")];
    let v: str = a[0];
    let flag: bool = true;
    let n: usize = (flag ? a : mk()).len();
    return i32(n) - 2 + i32(v[0]) - 97;
}
"#;
    let output = run_memory("lenmixeddot", mixed_dot);
    assert_no_asan("lenmixeddot", &output);
    assert_exit("lenmixeddot", &output, 0);

    let owned_literal = r#"module spec.avi.lenlitowned;

import { stringOf } from std.text;

func main(): i32 {
    let n: usize = len([stringOf("aa"), stringOf("bb")]);
    return i32(n) - 2;
}
"#;
    let output = run_memory("lenlitowned", owned_literal);
    assert_no_asan("lenlitowned", &output);
    assert_exit("lenlitowned", &output, 0);

    let member_root = r#"module spec.avi.lenmemberroot;

import { String, stringOf } from std.text;

let hits: i32 = 0;

struct Box { name: String; rows: String[2][2]; }

func mk(): Box {
    hits = hits + 1;
    return Box {
        name: stringOf("n"),
        rows: [[stringOf("aa"), stringOf("ab")], [stringOf("ba"), stringOf("bb")]],
    };
}

func main(): i32 {
    let n: usize = len(mk().rows[0]);
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenmemberroot", member_root);
    assert_no_asan("lenmemberroot", &output);
    assert_exit("lenmemberroot", &output, 0);

    let member_dot = r#"module spec.avi.lenmemberdot;

import { String, stringOf } from std.text;

let hits: i32 = 0;

struct Box { name: String; rows: String[2][2]; }

func mk(): Box {
    hits = hits + 1;
    return Box {
        name: stringOf("n"),
        rows: [[stringOf("aa"), stringOf("ab")], [stringOf("ba"), stringOf("bb")]],
    };
}

func main(): i32 {
    let n: usize = mk().rows.len();
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenmemberdot", member_dot);
    assert_no_asan("lenmemberdot", &output);
    assert_exit("lenmemberdot", &output, 0);

    let index_conditional = r#"module spec.avi.lenindexcond;

import { String, stringOf } from std.text;

let hits: i32 = 0;

struct Box { rows: String[2][2]; }

func mkA(): Box {
    hits = hits + 1;
    return Box { rows: [[stringOf("aa"), stringOf("ab")], [stringOf("ba"), stringOf("bb")]] };
}

func mkB(): Box {
    hits = hits + 1;
    return Box { rows: [[stringOf("ca"), stringOf("cb")], [stringOf("da"), stringOf("db")]] };
}

func main(): i32 {
    let flag: bool = true;
    let n: usize = len(flag ? mkA().rows[0] : mkB().rows[0]);
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenindexcond", index_conditional);
    assert_no_asan("lenindexcond", &output);
    assert_exit("lenindexcond", &output, 0);

    let fresh_member_conditional = r#"module spec.avi.lenmembercond;

import { String, stringOf } from std.text;

struct Box { name: String; rows: String[2]; }

func mkA(): Box {
    return Box { name: stringOf("a"), rows: [stringOf("aa"), stringOf("ab")] };
}

func mkB(): Box {
    return Box { name: stringOf("b"), rows: [stringOf("ba"), stringOf("bb")] };
}

func main(): i32 {
    let flag: bool = true;
    let n: usize = len(flag ? mkA().rows : mkB().rows);
    return i32(n) - 2;
}
"#;
    let output = run_memory("lenmembercond", fresh_member_conditional);
    assert_no_asan("lenmembercond", &output);
    assert_exit("lenmembercond", &output, 0);

    let nested_member_conditional = r#"module spec.avi.lennestedmember;

let hits: i32 = 0;

struct Box { rows: i32[2]; }

func mkA(): Box { hits = hits + 1; return Box { rows: [1, 2] }; }
func mkB(): Box { hits = hits + 10; return Box { rows: [3, 4] }; }
func mkC(): Box { hits = hits + 100; return Box { rows: [5, 6] }; }

func main(): i32 {
    let flag: bool = true;
    let flag2: bool = false;
    let n: usize = len(flag ? (flag2 ? mkA() : mkB()).rows : mkC().rows);
    return i32(n) - 2 + hits - 10;
}
"#;
    let output = run_memory("lennestedmember", nested_member_conditional);
    assert_no_asan("lennestedmember", &output);
    assert_exit("lennestedmember", &output, 0);

    let mixed_fresh = r#"module spec.avi.lenmixedfresh;

let hits: i32 = 0;

func mk(): i32[2] { hits = hits + 1; return [20, 21]; }

func main(): i32 {
    let a: i32[2] = [1, 2];
    let flag: bool = false;
    let n: usize = len(flag ? a : mk());
    return i32(n) - 2 + hits - 1 + a[0] - 1;
}
"#;
    let output = run_memory("lenmixedfresh", mixed_fresh);
    assert_no_asan("lenmixedfresh", &output);
    assert_exit("lenmixedfresh", &output, 0);

    let struct_root = r#"module spec.avi.lenstructroot;

let hits: i32 = 0;

struct Box { rows: i32[2]; }

func mkRows(): i32[2] { hits = hits + 1; return [1, 2]; }

func main(): i32 {
    let n: usize = len(Box { rows: mkRows() }.rows);
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("lenstructroot", struct_root);
    assert_no_asan("lenstructroot", &output);
    assert_exit("lenstructroot", &output, 0);

    let struct_conditional = r#"module spec.avi.lenstructcond;

let hits: i32 = 0;

struct Box { rows: i32[2]; }

func mkA(): i32[2] { hits = hits + 1; return [1, 2]; }
func mkB(): i32[2] { hits = hits + 10; return [3, 4]; }

func main(): i32 {
    let flag: bool = false;
    let n: usize = len(flag ? Box { rows: mkA() }.rows : Box { rows: mkB() }.rows);
    return i32(n) - 2 + hits - 10;
}
"#;
    let output = run_memory("lenstructcond", struct_conditional);
    assert_no_asan("lenstructcond", &output);
    assert_exit("lenstructcond", &output, 0);
}

/// A discarded fixed-array conditional leaves a place arm in place.
///
/// The statement's value is unused, so a place-rooted arm is a read: the
/// caller's row stays owned and readable after the statement. A fresh arm still
/// evaluates on its path and releases. Red at the pre-fix head: the place arm
/// was copied into the discard temporary, zeroed, and freed, so the later view
/// read freed storage (ASan use-after-free).
#[test]
fn should_keep_a_place_arm_in_a_discarded_fixed_array_conditional() {
    let place = r#"module spec.avi.discardplace;

import { String, stringOf } from std.text;

struct Box { rows: String[2]; }

func mk(): Box {
    return Box { rows: [stringOf("ba"), stringOf("bb")] };
}

func main(): i32 {
    let a: Box = Box { rows: [stringOf("aa"), stringOf("ab")] };
    let flag: bool = true;
    (flag ? a.rows : mk().rows);
    let v: str = a.rows[0];
    return i32(v[0]) - 97;
}
"#;
    let output = run_memory("discardplace", place);
    assert_no_asan("discardplace", &output);
    assert_exit("discardplace", &output, 0);

    let fresh = r#"module spec.avi.discardfresh;

import { String, stringOf } from std.text;

let hits: i32 = 0;

struct Box { rows: String[2]; }

func mk(): Box {
    hits = hits + 1;
    return Box { rows: [stringOf("ba"), stringOf("bb")] };
}

func main(): i32 {
    let a: Box = Box { rows: [stringOf("aa"), stringOf("ab")] };
    let flag: bool = false;
    (flag ? a.rows : mk().rows);
    let v: str = a.rows[0];
    return i32(v[0]) - 97 + hits - 1;
}
"#;
    let output = run_memory("discardfresh", fresh);
    assert_no_asan("discardfresh", &output);
    assert_exit("discardfresh", &output, 0);
}
