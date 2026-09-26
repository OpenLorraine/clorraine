//! Specification tests for array ownership, classification, and drop.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary and
//! asserts the behavior the frozen specification requires for arrays: an element
//! moved into an array literal cannot be used again, a fixed array stays a lend
//! candidate for return elision, and a fixed array is not nullable. The cases
//! drive the published command-line behavior, so a regression in the front end
//! is visible as a wrong diagnostic or an unexpected acceptance.
//!
//! Cases whose expectation is only observable at run time (an element released
//! exactly once) live in `ownership_drop_impl_test.rs`, which builds and runs the
//! emitted C against the host C library.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

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
            "clc-ownership-drop-{}-{tag}-{sequence}",
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

/// Compiles one source snippet for the host profile and returns the output.
fn run_source(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            source_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Asserts that compiling one snippet reports every expected diagnostic line.
fn assert_reports(tag: &str, source: &str, expected_lines: &[&str]) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    for expected in expected_lines {
        assert!(
            stderr.lines().any(|line| line == *expected),
            "{tag}: missing diagnostic `{expected}`; stderr:\n{stderr}"
        );
    }
    assert_eq!(run.status.code(), Some(1), "{tag}: failing compiles exit 1");
}

/// Asserts that the compiler accepts a snippet and emits C.
fn assert_accepts(tag: &str, source: &str) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{stderr}"
    );
    assert!(
        !stderr.lines().any(|line| line.starts_with("error ")),
        "{tag}: unexpected error diagnostic; stderr:\n{stderr}"
    );
}

/// Compiles one snippet and returns the scratch directory and emitted C path.
///
/// The front end must accept the program; the caller inspects the emitted C.
fn emit_c(tag: &str, source: &str) -> (ScratchDir, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let c_path = scratch.path().join("out.c");
    let run = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            source_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    (scratch, c_path)
}

/// Asserts that the emitted translation unit is valid C.
///
/// The front end can accept a program whose emitted C does not compile, so the
/// host C compiler is the oracle for the emitter half of the case. This is the
/// checker-plus-emitter contract; run-time behavior is covered separately.
fn assert_emits_valid_c(tag: &str, source: &str) {
    let (_scratch, c_path) = emit_c(tag, source);
    let built = Command::new("cc")
        .args(["-std=c99", "-fsyntax-only"])
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// `spec/08-arrays-strings.md` §6: a binding moved into an array literal is
/// invalid afterward, so reading it again is `E10020`.
///
/// The expectation is the moved-source rule: the array owns its elements the
/// same way a struct literal owns its fields.
#[test]
fn should_report_e10020_when_a_moved_element_source_is_used() {
    let source = r#"module tests.od.classification.moved;

import { String, stringOf } from std.text;

func observe(s: String): void {}

func demo(): void {
    let a: String = stringOf("a");
    let arr: String[1] = [a];
    observe(a);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "st2",
        source,
        &["error E10020: 'a' was moved and cannot be used"],
    );
}

/// `spec/16-borrow-rules.md`: a fixed-array parameter is borrowed storage, so it
/// stays a lifetime-elision candidate and a view may be returned from it.
///
/// This case passed before the change: it is the regression guard for routing
/// the shape consumers, and it must keep passing.
#[test]
fn should_accept_a_view_return_derived_from_a_fixed_array_parameter() {
    let source = r#"module tests.od.classification.lend;

import { slice } from std.mem;

func head(arr: u8[2]): u8[] {
    return slice(arr, 0, 1);
}

func main(): i32 {
    let data: u8[2] = [1, 2];
    let h: u8[] = head(data);
    return i32(h[0]);
}
"#;
    assert_accepts("st15", source);
}

/// `spec/08-arrays-strings.md` §3: a fixed array is inline owned storage, so the
/// `?` empty state is meaningless on it and the suffix is rejected.
#[test]
fn should_reject_a_nullable_fixed_array() {
    let source = r#"module tests.od.classification.nullable;

import { String } from std.text;

func demo(): void {
    let a: String[2]? = null;
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "st16",
        source,
        &["error E10011: nullable type is not allowed on 'String[2]'"],
    );
}

/// `spec/08-arrays-strings.md` §6: a function may return a fixed array by value.
///
/// C cannot return an array, so the emitter must lower the by-value return into
/// valid C (a hidden out-parameter) rather than `return <array>`. The emitted
/// translation unit is the oracle here; the run-time release behavior is the
/// companion implementation test. A slice return is unchanged and is covered by
/// the lend-candidate case above.
#[test]
fn should_emit_valid_c_when_a_function_returns_a_fixed_array_by_value() {
    let source = r#"module tests.od.freereturn;

import { String, stringOf } from std.text;

func pair(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func demo(): void {
    let p: String[2] = pair();
}

func main(): i32 { demo(); return 0; }
"#;
    assert_emits_valid_c("st1", source);
}

/// A function may return the result of another fixed-array-returning call.
///
/// The hidden destination is threaded through both calls, so the emitted C
/// stays valid for the forwarded form as well as the literal form.
#[test]
fn should_emit_valid_c_when_a_fixed_array_return_forwards_another_call() {
    let source = r#"module tests.od.forward_return;

import { String, stringOf } from std.text;

func inner(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func outer(): String[2] {
    return inner();
}

func main(): i32 { let p: String[2] = outer(); return 0; }
"#;
    assert_emits_valid_c("rv002", source);
}

/// A discarded fixed-array-returning call still provides a destination.
///
/// The result is not used, but the callee still writes through a hidden
/// out-parameter, so the emitter must pass one even when the value is dropped.
#[test]
fn should_emit_valid_c_when_a_fixed_array_result_is_discarded() {
    let source = r#"module tests.od.discard_return;

import { String, stringOf } from std.text;

func inner(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func demo(): void {
    inner();
}

func main(): i32 { demo(); return 0; }
"#;
    assert_emits_valid_c("rv003", source);
}

/// `spec/08-arrays-strings.md`: a nested array literal initializes a
/// multi-dimensional fixed array, one dimension per brace level.
///
/// The front end already accepts the shape, but C cannot assign a brace list to
/// an array element, so the emitter must recurse one dimension per level. The
/// host C compiler is the oracle: at the defect the element assignment is
/// `d[0] = { 1, 2 };`, which C rejects.
#[test]
fn should_emit_valid_c_when_a_nested_array_literal_initializes_a_fixed_array() {
    let source = r#"module tests.od.nested_literal;

func demo(): void {
    let d: u8[3][2] = [[1, 2], [3, 4], [5, 6]];
}

func main(): i32 { demo(); return 0; }
"#;
    assert_emits_valid_c("pf003", source);
}

/// A fixed-array `Array` element is rejected even when the `Array` type is built
/// by a generic call's type argument, not a written annotation.
#[test]
fn should_reject_a_fixed_array_element_built_by_a_generic_call() {
    let source = r#"module tests.od.array_fixed_call;

import { String } from std.text;
import { Array, arrayNew } from std.mem;

func main(): i32 {
    let xs = arrayNew<String[2]>();
    return 0;
}
"#;
    assert_reports(
        "e90000-call",
        source,
        &["error E90000: 'String[2]' is not yet supported"],
    );
}

/// `spec/appendix-stdlib.md`: a growable `Array<T>` accepts a move-only element
/// now that element drop glue exists, so the staged `E90000` no longer fires.
#[test]
fn should_accept_a_growable_array_of_a_move_only_element() {
    let source = r#"module tests.od.growable_move_only;

import { Buffer, Array, arrayNew } from std.mem;

func main(): i32 {
    let xs: Array<Buffer> = arrayNew<Buffer>();
    return 0;
}
"#;
    assert_accepts("st8", source);
}

/// `spec/appendix-stdlib.md:62`: replacing a growable element is accepted for a
/// move-only element, which the former staged `E90000` rejected.
#[test]
fn should_accept_replacing_a_growable_move_only_element() {
    let source = r#"module tests.od.set_move_only;

import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;

func demo(): void {
    let xs: Array<String> = arrayNew<String>();
    xs.push(stringOf("a"));
    xs.set(0, stringOf("b"));
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st17", source);
}

/// The fixed-array drop glue releases each element in place, one dimension per
/// level, so a nested declaration indexes a row and then an element.
///
/// The per-dimension shape is the point: a flattened index would name a row
/// instead of an element and drop the wrong storage.
#[test]
fn should_emit_fixed_array_drop_glue_one_dimension_per_level() {
    let source = r#"module tests.od.fixed_drop_shape;

import { String, stringOf } from std.text;

func demo(): void {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    let c: String = stringOf("c");
    let d: String = stringOf("d");
    let grid: String[2][2] = [[a, b], [c, d]];
}

func main(): i32 { demo(); return 0; }
"#;
    let (_scratch, c_path) = emit_c("drop-shape-fixed", source);
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    for name in ["grid[0][0]", "grid[0][1]", "grid[1][0]", "grid[1][1]"] {
        let expected = format!("FreeString({name})");
        assert!(
            emitted.contains(&expected),
            "the fixed-array drop must release `{name}` in place; emitted C:\n{emitted}"
        );
    }
}

/// The growable drop glue walks the elements through the buffer, then frees the
/// buffer and empties the array.
#[test]
fn should_emit_growable_array_drop_glue_through_the_buffer() {
    let source = r#"module tests.od.grow_drop_shape;

import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;

func demo(): void {
    let xs: Array<String> = arrayNew<String>();
    xs.push(stringOf("x"));
}

func main(): i32 { demo(); return 0; }
"#;
    let (_scratch, c_path) = emit_c("drop-shape-grow", source);
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        emitted.contains("cl_buffer_ptr(xs.data)"),
        "the growable drop must walk the buffer; emitted C:\n{emitted}"
    );
    assert!(
        emitted.contains("FreeBuffer(xs.data)"),
        "the growable drop must free the buffer; emitted C:\n{emitted}"
    );
}

/// `spec/08-arrays-strings.md`: a struct field may not be a view. A slice
/// `u8[]` is a pointer and a length over storage owned elsewhere, so a struct
/// that stored one could outlive the storage it points at; the field is
/// `E10205`.
#[test]
fn should_reject_a_slice_view_field() {
    assert_reports(
        "e10205-slice-field",
        r#"module tests.od.view_field_slice;

struct Bad {
    part: u8[];
}

func main(): i32 {
    return 0;
}
"#,
        &["error E10205: view-typed field 'part' is not allowed"],
    );
}

/// A plain `str` field is a view as well — the original shape of the rule —
/// and stays `E10205`.
#[test]
fn should_reject_a_str_view_field() {
    assert_reports(
        "e10205-str-field",
        r#"module tests.od.view_field_str;

struct Bad {
    part: str;
}

func main(): i32 {
    return 0;
}
"#,
        &["error E10205: view-typed field 'part' is not allowed"],
    );
}

/// A struct may hold owned storage, a literal (`static str`), an FFI pointer
/// (`cstring`), and an owned fixed array; none of those is a view.
#[test]
fn should_accept_a_struct_with_owned_literal_pointer_and_fixed_array_fields() {
    assert_accepts(
        "view-field-allowed",
        r#"module tests.od.view_field_allowed;

import { String } from std.text;

struct Ok {
    name: String;
    tag: static str;
    path: cstring;
    row: u8[8];
}

func main(): i32 {
    return 0;
}
"#,
    );
}

/// A fill repeats one value into every element, so the element must be
/// duplicable. A move-only element would be copied into each slot and then
/// released once per slot, freeing the same storage twice; the literal is
/// `E10215`.
#[test]
fn should_reject_a_fill_of_a_move_only_element() {
    assert_reports(
        "e10215-fill-move-only",
        r#"module tests.od.fill_move_only;

import { String } from std.text;

func demo(a: String): void {
    let xs: String[2] = [; a];
}

func main(): i32 {
    return 0;
}
"#,
        &["error E10215: array fill requires a fixed extent"],
    );
}

/// A fill of a duplicable element is the ordinary zero-initializer form.
#[test]
fn should_accept_a_fill_of_a_duplicable_element() {
    assert_accepts(
        "fill-duplicable",
        r#"module tests.od.fill_duplicable;

func demo(): void {
    let xs: u8[4] = [; 0];
}

func main(): i32 {
    demo();
    return 0;
}
"#,
    );
}

/// `spec/12-ffi-contracts.md`: an uncontracted parameter accepts only scalars,
/// `raw`, `cstring`, views, and `null`. A fixed array is owned inline storage,
/// so passing one is `E10101` even inside `unsafe`.
#[test]
fn should_report_e10101_when_a_fixed_array_reaches_an_uncontracted_extern() {
    assert_reports(
        "e10101-fixed-array",
        r#"module tests.od.fixed_array_extern;

ffi lib {
    func RawConsume(data: u8[4]): i32;
}

func demo(xs: u8[4]): i32 {
    let n: i32 = 0;
    unsafe {
        n = RawConsume(xs);
    }
    return n;
}

func main(): i32 {
    return 0;
}
"#,
        &["error E10101: 'RawConsume' has no contract for parameter 'data'"],
    );
}

/// The same uncontracted signature accepts a slice: it is a view over storage
/// owned elsewhere, so the call carries no ownership the callee could keep.
#[test]
fn should_accept_a_slice_passed_to_an_uncontracted_extern() {
    assert_accepts(
        "slice-extern",
        r#"module tests.od.slice_extern;

ffi lib {
    func RawConsume(data: u8[]): i32;
}

func demo(s: u8[]): i32 {
    let n: i32 = 0;
    unsafe {
        n = RawConsume(s);
    }
    return n;
}

func main(): i32 {
    return 0;
}
"#,
    );
}

/// The nullable suffix does not change what a field is: `str?` is still a
/// `str` view and `u8[]?` is still a slice view, so both are `E10205`.
#[test]
fn should_reject_a_nullable_view_field() {
    assert_reports(
        "e10205-nullable-str",
        r#"module tests.od.nullable_str_field;

struct Bad {
    part: str?;
}

func main(): i32 {
    return 0;
}
"#,
        &["error E10205: view-typed field 'part' is not allowed"],
    );
    assert_reports(
        "e10205-nullable-slice",
        r#"module tests.od.nullable_slice_field;

struct Bad {
    part: u8[]?;
}

func main(): i32 {
    return 0;
}
"#,
        &["error E10205: view-typed field 'part' is not allowed"],
    );
}

/// A one-dimensional prefix-plus-fill literal repeats the fill value, so a
/// move-only fill is `E10215` even when a prefix element is present.
#[test]
fn should_reject_a_prefix_and_fill_literal_of_a_move_only_element() {
    assert_reports(
        "e10215-prefix-fill",
        r#"module tests.od.fill_prefix_move;

import { String, stringOf } from std.text;

func demo(): void {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    let xs: String[3] = [a; b];
}

func main(): i32 {
    return 0;
}
"#,
        &["error E10215: array fill requires a fixed extent"],
    );
}

/// A multi-dimensional fixed array is owned inline storage, so a struct field
/// may hold one.
#[test]
fn should_accept_a_nested_fixed_array_field() {
    assert_accepts(
        "nested-fixed-field",
        r#"module tests.od.nested_fixed_field;

struct Ok {
    grid: u8[2][2];
}

func main(): i32 {
    return 0;
}
"#,
    );
}

/// Asserts that compiling one snippet reports each expected line exactly the
/// given number of times, and fails with exit code 1.
///
/// The plain `assert_reports` checks only presence, so it cannot pin a count.
/// A case that must report one error per field, or must not report an error at
/// all for a field, needs the exact count; the line text carries the field name
/// so an over-report is visible.
fn assert_reports_counted(tag: &str, source: &str, expected: &[(&str, usize)]) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    for (line, count) in expected {
        let seen = stderr.lines().filter(|candidate| candidate == line).count();
        assert_eq!(
            seen, *count,
            "{tag}: expected {count} occurrence(s) of `{line}`, saw {seen}; stderr:\n{stderr}"
        );
    }
    assert_eq!(run.status.code(), Some(1), "{tag}: failing compiles exit 1");
}

// -- Instantiation-time view-field guards (E10205) -------------------------
//
// A field typed `T` is not a view as written, so the declaration-site guard
// cannot judge it until a type argument supplies the concrete kind. These cases
// pin the instantiation-time audit: a substituted `str` or slice is rejected,
// while an owned substitution, a literal, a pointer, and a fixed array are not.

/// A `str` substituted into a field typed `T` is a view, so the instantiation
/// is rejected the same way a written `str` field is (`spec/02-types.md` TY-10).
#[test]
fn should_reject_a_str_substituted_into_a_generic_view_field() {
    let source = r#"module tests.gs.view_str;

struct Wrap<T> {
    v: T;
}

func demo(): void {
    let w: Wrap<str> = Wrap<str> { v: "hi" };
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-view-str",
        source,
        &["error E10205: view-typed field 'v' is not allowed"],
    );
}

/// A slice substituted into a field typed `T` is a view over storage owned
/// elsewhere, so the instantiation is rejected.
#[test]
fn should_reject_a_slice_substituted_into_a_generic_view_field() {
    let source = r#"module tests.gs.view_slice;

import { String, stringOf, bytes } from std.text;

struct Wrap<T> {
    v: T;
}

func demo(): void {
    let s: String = stringOf("hi");
    let b: Wrap<u8[]> = Wrap<u8[]> { v: bytes(s) };
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-view-slice",
        source,
        &["error E10205: view-typed field 'v' is not allowed"],
    );
}

/// The nullable suffix does not change what a field is: a substituted `str?` is
/// still a `str` view, so it is rejected.
#[test]
fn should_reject_a_nullable_str_substituted_into_a_generic_view_field() {
    let source = r#"module tests.gs.view_nullable;

struct Wrap<T> {
    v: T;
}

func demo(): void {
    let w: Wrap<str?> = Wrap<str?> { v: null };
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-view-nullable",
        source,
        &["error E10205: view-typed field 'v' is not allowed"],
    );
}

/// A generic struct may hold another generic struct; the inner field is judged
/// once the outer instance fixes the parameter, so `Outer<str>` is rejected at
/// the inner `v` field.
#[test]
fn should_reject_a_nested_str_substituted_through_a_generic_field() {
    let source = r#"module tests.gs.view_nested;

struct Wrap<T> {
    v: T;
}

struct Outer<T> {
    w: Wrap<T>;
}

func demo(): void {
    let o: Outer<str> = Outer<str> { w: Wrap<str> { v: "hi" } };
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-view-nested",
        source,
        &["error E10205: view-typed field 'v' is not allowed"],
    );
}

/// A generic function's return type fixes the parameter at the call: the
/// concrete return type records `Wrap<str>`, so the field is judged with `str`
/// substituted and rejected.
#[test]
fn should_reject_a_str_returned_by_a_generic_call_into_a_view_field() {
    let source = r#"module tests.gs.view_call;

struct Wrap<T> {
    v: T;
}

func make<T>(v: T): Wrap<T> {
    return Wrap<T> { v: v };
}

func demo(): void {
    let w: Wrap<str> = make<str>("hi");
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-view-call",
        source,
        &["error E10205: view-typed field 'v' is not allowed"],
    );
}

/// A control set for the accepted substitutions: an owned handle, a literal, a
/// fixed array, and an FFI pointer are not views, so each instantiation compiles
/// and the emitted C stays valid.
#[test]
fn should_accept_owning_substitutions_into_a_generic_field() {
    let source = r#"module tests.gs.view_controls;

import { String, stringOf } from std.text;
import { Buffer } from std.mem;

struct Wrap<T> {
    v: T;
}

func demo(take own: Buffer): void {
    let a: Wrap<static str> = Wrap<static str> { v: "hi" };
    let b: Wrap<u8[2]> = Wrap<u8[2]> { v: [1, 2] };
    let c: Wrap<String> = Wrap<String> { v: stringOf("x") };
    let d: Wrap<Buffer> = Wrap<Buffer> { v: own };
}

func main(): i32 { return 0; }
"#;
    assert_emits_valid_c("gs-view-controls", source);
}

/// A two-parameter instantiation with two view arguments reports one error per
/// field and no error for anything else.
#[test]
fn should_report_two_view_field_errors_for_a_two_parameter_instantiation() {
    let source = r#"module tests.gs.view_pair;

import { stringOf, bytes } from std.text;

struct Pair<T, U> {
    a: T;
    b: U;
}

func demo(): void {
    let p: Pair<str, u8[]> =
        Pair<str, u8[]> { a: "hi", b: bytes(stringOf("hi")) };
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "gs-view-pair",
        source,
        &[
            ("error E10205: view-typed field 'a' is not allowed", 1),
            ("error E10205: view-typed field 'b' is not allowed", 1),
        ],
    );
}

/// A field already concrete at the declaration is the declaration-site guard's
/// case; the type checker stops the program before the audit runs, so the field
/// is reported once and the parameterized sibling is not reported at all.
#[test]
fn should_report_the_declaration_site_view_field_once_for_a_concrete_field() {
    let source = r#"module tests.gs.view_concrete;

import { String, stringOf, bytes } from std.text;

struct Mixed<T> {
    part: u8[];
    v: T;
}

func demo(): void {
    let m: Mixed<String> =
        Mixed<String> { part: bytes(stringOf("x")), v: stringOf("x") };
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "gs-view-concrete",
        source,
        &[
            ("error E10205: view-typed field 'part' is not allowed", 1),
            ("error E10205: view-typed field 'v' is not allowed", 0),
        ],
    );
}

// -- Instantiation-time fill guards (E10215) -------------------------------
//
// A fill typed by an array whose element is a parameter cannot be judged at the
// declaration, because duplicability is unknown until the argument is supplied.
// These cases pin the audit of the substituted element (`spec/08-arrays-strings.md`
// AL-1): a move-only substitution is rejected and a duplicable one is not.

/// The explicit string argument substitutes a move-only element into the fill,
/// so repeating it would alias one owner into both slots and is rejected.
#[test]
fn should_reject_a_move_only_fill_substituted_by_an_explicit_argument() {
    let source = r#"module tests.gs.fill_explicit;

import { String, stringOf } from std.text;

func fillIt<T>(a: T): T[2] {
    return [; a];
}

func demo(): void {
    let s: String = stringOf("x");
    let cs: String[2] = fillIt<String>(s);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-fill-explicit",
        source,
        &["error E10215: array fill requires a fixed extent"],
    );
}

/// An inferred type argument is recorded from the lowered call, so the move-only
/// substitution is rejected without a written argument list.
#[test]
fn should_reject_a_move_only_fill_substituted_by_an_inferred_argument() {
    let source = r#"module tests.gs.fill_inferred;

import { String, stringOf } from std.text;

func fillIt<T>(a: T): T[2] {
    return [; a];
}

func demo(): void {
    let s: String = stringOf("x");
    let cs: String[2] = fillIt(s);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-fill-inferred",
        source,
        &["error E10215: array fill requires a fixed extent"],
    );
}

/// A prefix plus a fill repeats the fill value into the remaining slots, so a
/// move-only substitution is rejected through the shared path.
#[test]
fn should_reject_a_prefix_and_fill_instantiated_with_a_move_only_element() {
    let source = r#"module tests.gs.fill_prefix;

import { String, stringOf } from std.text;

func pair<T>(a: T, b: T): T[2] {
    return [a; b];
}

func demo(): void {
    let x: String = stringOf("x");
    let y: String = stringOf("y");
    let ps: String[2] = pair<String>(x, y);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-fill-prefix",
        source,
        &["error E10215: array fill requires a fixed extent"],
    );
}

/// The accepted fill controls: an explicit scalar argument, an inferred scalar
/// argument, and an ordinary scalar fill each compile and emit valid C.
#[test]
fn should_accept_duplicable_fill_instantiations() {
    let source = r#"module tests.gs.fill_controls;

func fillIt<T>(a: T): T[2] {
    return [; a];
}

func demo(): void {
    let ns: i32[2] = fillIt<i32>(7);
    let a: u8 = 3;
    let us: u8[2] = fillIt(a);
    let xs: u8[4] = [; 0];
}

func main(): i32 { demo(); return 0; }
"#;
    assert_emits_valid_c("gs-fill-controls", source);
}

/// A substituted struct that looked duplicable at the declaration is move-only
/// once its argument is supplied, so the fill is rejected: the declaration-site
/// guard saw `Wrap<T>` and could not tell.
#[test]
fn should_reject_a_move_only_fill_substituted_from_a_generic_struct() {
    let source = r#"module tests.gs.fill_struct;

import { String, stringOf } from std.text;

struct Wrap<T> {
    v: T;
}

func rep<T>(w: Wrap<T>): Wrap<T>[2] {
    return [; w];
}

func demo(): void {
    let w: Wrap<String> = Wrap<String> { v: stringOf("x") };
    let r: Wrap<String>[2] = rep<String>(w);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports(
        "gs-fill-struct",
        source,
        &["error E10215: array fill requires a fixed extent"],
    );
}

/// A fill already concrete at the declaration is the declaration-site guard's
/// case; the type checker stops the program before the audit runs, so the fill
/// is reported once even though the enclosing function is generic.
#[test]
fn should_report_the_declaration_site_fill_once_for_a_generic_body() {
    let source = r#"module tests.gs.fill_concrete;

import { String, stringOf } from std.text;

func f<T>(x: T): i32 {
    let xs: String[2] = [; stringOf("x")];
    return 0;
}

func demo(): void {
    let n: i32 = f<i32>(1);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "gs-fill-concrete",
        source,
        &[("error E10215: array fill requires a fixed extent", 1)],
    );
}
