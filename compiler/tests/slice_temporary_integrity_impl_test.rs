//! Implementation tests for row-typed slice view emission.
//!
//! The specification suite proves *what* a row slice program computes; these
//! cases prove *how* the emitter spells it. A multi-extent view must key,
//! declare, build, and index by its whole row (`i32[][3]` elements are
//! `i32[3]`), the null and closing returns of a view-returning function must
//! use the row-keyed zero helper, scalar views must keep their pointer
//! spelling, and two emissions of one program must be byte-identical.

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
            "clc-stiimpl-{}-{tag}-{sequence}",
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
/// Returns the emitted translation unit. These cases are about what the
/// emitter produces for accepted programs, so a rejection is a failure.
fn emit_host(tag: &str, source: &str) -> String {
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

/// Emits the scratch directory's `main.clo` with a relative source path.
///
/// Both runs of a determinism case then name the same logical source, so the
/// emitted `#line` directives stay comparable.
fn emit_relative(scratch: &ScratchDir, tag: &str, output_name: &str) -> String {
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
}

/// Runs a built program with a time limit and the given environment values.
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_with_timeout(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
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
            Err(err) => panic!("{tag}: cannot wait for the program: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program output: {err}"))
}

/// Emits, builds, and runs one host program, returning its output.
///
/// The emitted C is built with the same strict flag set the specification
/// suite uses, so a spelling that only builds without `-Werror` still fails
/// here.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let emitted = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emitted.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
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
    run_with_timeout(tag, &program, &[])
}

/// Asserts the text contains a fragment.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: the emitted C does not contain {needle:?}"
    );
}

/// Asserts the text does not contain a superseded fragment.
fn assert_absent(tag: &str, text: &str, needle: &str) {
    assert!(
        !text.contains(needle),
        "{tag}: the emitted C still contains {needle:?}"
    );
}

/// Asserts the first fragment appears before the second one in the text.
fn assert_before(tag: &str, text: &str, first: &str, second: &str) {
    let first_at = text
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {first:?}"));
    let second_at = text
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {second:?}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}"
    );
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

/// A row view declares its pointer with the row extent and keeps the scalar
/// view spelling.
///
/// The `i32[][3]` parameter and the `slice<i32[3]>` result share one typedef
/// whose `ptr` field is `i32 (*ptr)[3]`; the flat `ClSlice_i32` name must not
/// appear, while the imported standard library's `u8[]` views keep their
/// ordinary `u8 *` spelling. `sink` reads the second cell of the first row
/// (`2`), so `main` answers two.
#[test]
fn should_declare_the_row_pointer_and_keep_scalar_views_flat() {
    let source = r#"module spec.sti.impl1;

import { slice } from std.mem;

func sink(xs: i32[][3]): i32 {
    return xs[0][1];
}

func main(): i32 {
    let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return sink(slice<i32[3]>(grid, 0, 2));
}
"#;
    let emitted = emit_host("sti-impl1", source);
    assert_contains(
        "sti-impl1",
        &emitted,
        "typedef struct { i32 (*ptr)[3]; usize len; } ClSlice_arr3_i32;",
    );
    assert_contains(
        "sti-impl1",
        &emitted,
        "typedef struct { u8 *ptr; usize len; } ClSlice_u8;",
    );
    assert_absent("sti-impl1", &emitted, "ClSlice_i32");
    let output = run_host("sti-impl1-run", source);
    assert_exit("sti-impl1-run", &output, 2);
}

/// A null return and the closing return of a row-view function use the
/// row-keyed zero helper.
///
/// The nullable return emits `cl_zero_ClSlice_arr3_i32()` on the null path,
/// and the function's closing return emits the same helper, so the helper must
/// exist for the row key. Passing the nullable view to `use` answers three.
#[test]
fn should_zero_a_row_view_through_the_row_keyed_helper() {
    let source = r#"module spec.sti.impl2;

import { slice } from std.mem;

func maybe(xs: i32[2][3], flag: bool): i32[][3]? {
    if (flag) {
        return null;
    }
    return slice<i32[3]>(xs, 0, 2);
}

func use(view: i32[][3]?): i32 {
    return 3;
}

func main(): i32 {
    let xs: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let a: i32[][3]? = maybe(xs, true);
    return use(a);
}
"#;
    let emitted = emit_host("sti-impl2", source);
    assert_contains(
        "sti-impl2",
        &emitted,
        "static inline ClSlice_arr3_i32 cl_zero_ClSlice_arr3_i32(void)",
    );
    assert_eq!(
        emitted.matches("cl_zero_ClSlice_arr3_i32()").count(),
        2,
        "sti-impl2: the null return and the closing return must both use the helper"
    );
    let output = run_host("sti-impl2-run", source);
    assert_exit("sti-impl2-run", &output, 3);
}

/// A row slice builder offsets in rows and the access yields the whole row.
///
/// The builder stores the view's element pointer directly and the length
/// counts rows; the access spells the view's `ptr` field, so `xs[0][2]` reads
/// the third cell of the selected row (`6`).
#[test]
fn should_build_and_index_a_row_view_in_rows() {
    let source = r#"module spec.sti.impl3;

import { slice } from std.mem;

func sliceRows(xs: i32[2][3], lo: usize, hi: usize): i32[][3] {
    return slice<i32[3]>(xs, lo, hi);
}

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let xs: i32[][3] = sliceRows(m, 1, 2);
    return xs[0][2];
}
"#;
    let emitted = emit_host("sti-impl3", source);
    assert_contains(
        "sti-impl3",
        &emitted,
        "((ClSlice_arr3_i32){ (xs) + (lo), (usize)((hi) - (lo)) })",
    );
    assert_contains("sti-impl3", &emitted, "(xs).ptr[0][2]");
    assert_absent("sti-impl3", &emitted, "(i32 *)((");
    let output = run_host("sti-impl3-run", source);
    assert_exit("sti-impl3-run", &output, 6);
}

/// Two emissions of one row-view program must be byte-identical.
///
/// Both runs name the source with the same relative path so the emitted
/// `#line` directives stay comparable.
#[test]
fn should_emit_identical_translations_when_the_same_row_view_program_is_emitted_twice() {
    let source = r#"module spec.sti.impl4;

import { slice } from std.mem;

func pick(xs: i32[2][3], i: usize): i32 {
    let view: i32[][3] = slice<i32[3]>(xs, 0, 2);
    return view[i][0];
}

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return pick(m, 1);
}
"#;
    let scratch = ScratchDir::new("sti-impl4");
    std::fs::write(scratch.path().join("main.clo"), source).expect("write the program");
    let first = emit_relative(&scratch, "sti-impl4", "first.c");
    let second = emit_relative(&scratch, "sti-impl4", "second.c");
    assert_eq!(
        first, second,
        "sti-impl4: two emissions of one program must be byte-identical"
    );
}

/// A generic aggregate field reads a call result at a run-time index in the
/// consuming statement's order.
///
/// The field's guards fill the root array temporary, run the pinned index
/// once, copy the selected slot into the deferred consume temporary, and empty
/// the slot; the destination adopts the value and empties the temporary. The
/// selected element measures three bytes, so `main` answers three.
#[test]
fn should_defer_the_literal_fill_and_empty_the_slot_when_a_generic_field_reads_a_runtime_index() {
    let source = r#"module spec.sti.impl5;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func pack<T>(make: func(): T[2], i: i32): Box<T> {
    return Box<T> { v: make()[i] };
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let i: i32 = 1;
    let b: Box<String> = pack<String>(mkString, i);
    return i32(len(b.v));
}
"#;
    let emitted = emit_host("sti-impl5", source);
    assert_contains("sti-impl5", &emitted, "__cl_tmp1 = i;");
    assert_before(
        "sti-impl5",
        &emitted,
        "make(",
        "__cl_tmp2 = __cl_tmp0[__cl_tmp1];",
    );
    assert_contains(
        "sti-impl5",
        &emitted,
        "__cl_tmp2 = __cl_tmp0[__cl_tmp1];\n    __cl_tmp0[__cl_tmp1] = NULL;",
    );
    assert_contains(
        "sti-impl5",
        &emitted,
        "__cl_return_1.v = __cl_tmp2;\n    __cl_tmp2 = NULL;",
    );
    let output = run_host("sti-impl5-run", source);
    assert_exit("sti-impl5-run", &output, 3);
}

/// A member-rooted consume stores its literal per field.
///
/// `Box { v: mk().a }` binds the root call through the deferred
/// materialization, moves the selected field out of the bound temporary, and
/// leaves the sibling to the statement cleanup. The per-field store form is
/// selected because the field carries the deferred consume; `main` answers the
/// two-byte bound field.
#[test]
fn should_store_per_field_when_a_member_rooted_consume_feeds_a_literal() {
    let source = r#"module spec.sti.impl6;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

struct Box {
    v: String;
}

func mk(): Pair {
    return Pair { a: stringOf("ab"), b: stringOf("cde") };
}

func main(): i32 {
    let box: Box = Box { v: mk().a };
    return i32(len(box.v));
}
"#;
    let emitted = emit_host("sti-impl6", source);
    assert_before("sti-impl6", &emitted, "__cl_tmp0 = cl_", "__cl_tmp1 = __cl_tmp0.a;");
    assert_contains(
        "sti-impl6",
        &emitted,
        "__cl_tmp1 = __cl_tmp0.a;\n    __cl_tmp0.a = NULL;",
    );
    assert_contains(
        "sti-impl6",
        &emitted,
        "box.v = __cl_tmp1;\n    __cl_tmp1 = NULL;",
    );
    assert_absent("sti-impl6", &emitted, "(ClBox){");
    let output = run_host("sti-impl6-run", source);
    assert_exit("sti-impl6-run", &output, 2);
}

/// An indexed store empties the deferred consume carrier it adopts.
///
/// `out[target()] = mkArray()[rhsIndex()]` pins both run-time indices, copies
/// the selected element into the carrier, empties the source slot, then moves
/// the value into the element and empties both the store temporary and the
/// carrier, so a loop's reassigned-temp drop cannot free what the element
/// owns. The stored element measures three bytes.
#[test]
fn should_empty_the_deferred_carrier_when_an_indexed_store_adopts_it() {
    let source = r#"module spec.sti.impl7;

import { String, stringOf } from std.text;

let cursor: i32 = 0;

func target(): i32 {
    cursor = cursor + 1;
    return cursor - 1;
}

func rhsIndex(): i32 {
    return 1;
}

func mkArray(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let out: String[4] = [stringOf("1"), stringOf("2"), stringOf("3"), stringOf("4")];
    out[target()] = mkArray()[rhsIndex()];
    return i32(len(out[0]));
}
"#;
    let emitted = emit_host("sti-impl7", source);
    assert_before(
        "sti-impl7",
        &emitted,
        "_mkArray(__cl_tmp1);",
        "__cl_tmp3 = __cl_tmp1[__cl_tmp2];",
    );
    assert_contains(
        "sti-impl7",
        &emitted,
        "__cl_tmp3 = __cl_tmp1[__cl_tmp2];\n    __cl_tmp1[__cl_tmp2] = NULL;",
    );
    assert_contains(
        "sti-impl7",
        &emitted,
        "__cl_arm_1 = NULL;\n    __cl_tmp3 = NULL;",
    );
    let output = run_host("sti-impl7-run", source);
    assert_exit("sti-impl7-run", &output, 3);
}

/// A duplicable element read uses the store form and keeps source order.
///
/// `Box<i32> { w: f(), v: make()[i] }` has no consume carrier for an `i32`
/// element, but the deferred base fill still forces the per-field store form
/// so `f()` runs before `make()`. `main` answers the selected element (`42`).
#[test]
fn should_keep_source_order_when_a_duplicable_element_read_uses_the_store_form() {
    let source = r#"module spec.sti.impl8;

struct Box<T> {
    w: i32;
    v: T;
}

let trace: i32 = 0;

func f(): i32 {
    trace = trace * 10 + 1;
    return 1;
}

func makeI32(): i32[2] {
    trace = trace * 10 + 2;
    return [7, 42];
}

func pack<T>(first: func(): i32, make: func(): T[2], i: i32): Box<T> {
    return Box<T> { w: first(), v: make()[i] };
}

func main(): i32 {
    let b: Box<i32> = pack<i32>(f, makeI32, 1);
    return b.v;
}
"#;
    let emitted = emit_host("sti-impl8", source);
    assert_contains("sti-impl8", &emitted, "__cl_return_1.w = first();");
    assert_before("sti-impl8", &emitted, "__cl_return_1.w = first();", "make(__cl_tmp0);");
    assert_contains(
        "sti-impl8",
        &emitted,
        "__cl_tmp2 = __cl_tmp0[__cl_tmp1];\n    cl_zero(&__cl_tmp0[__cl_tmp1]);",
    );
    let output = run_host("sti-impl8-run", source);
    assert_exit("sti-impl8-run", &output, 42);
}

/// Two emissions of one consume-preparation program must be byte-identical.
///
/// Both runs name the source with the same relative path so the emitted
/// `#line` directives stay comparable.
#[test]
fn should_emit_identical_translations_when_a_consume_program_is_emitted_twice() {
    let source = r#"module spec.sti.impl9;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func pack<T>(make: func(): T[2], i: i32): Box<T> {
    return Box<T> { v: make()[i] };
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let b: Box<String> = pack<String>(mkString, 1);
    return i32(len(b.v));
}
"#;
    let scratch = ScratchDir::new("sti-impl9");
    std::fs::write(scratch.path().join("main.clo"), source).expect("write the program");
    let first = emit_relative(&scratch, "sti-impl9", "first.c");
    let second = emit_relative(&scratch, "sti-impl9", "second.c");
    assert_eq!(
        first, second,
        "sti-impl9: two emissions of one program must be byte-identical"
    );
}
