//! Runnable specification tests for the zero-defect null and type-rule batch.
//!
//! Each case compiles a small Clorraine program and either checks the frozen
//! diagnostic at the offending site or builds and runs the accepted spelling.
//! The batch pins the call-result null report, the index/qualified-read rules,
//! short-circuit and loop-head narrowing, the `len` surface, the aggregate and
//! operator null positions, the inferred generic null argument, TY-4 coverage,
//! the negated float literal, the raw out-slot self-copy, and the generic
//! non-nullable view out-slot store.

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
            "clc-zeronull-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the scratch directory, output, and C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
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
    (scratch, output, c_path)
}

/// Emits one host-profile program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
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

/// Builds one emitted host translation unit under the strict warning set.
///
/// The scratch directory is returned with the program path, so the built
/// executable stays on disk until the caller has run it.
fn build_strict_host(tag: &str, emitted: &str) -> (ScratchDir, PathBuf) {
    let scratch = ScratchDir::new(&format!("{tag}-build"));
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
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
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    (scratch, program)
}

/// Emits, strictly builds, and runs one accepted host program.
fn run_strict_host(tag: &str, source: &str) -> Output {
    let emitted = emit_host(tag, source);
    let (_scratch, program) = build_strict_host(tag, &emitted);
    Command::new(&program)
        .output()
        .expect("failed to spawn the built program")
}

/// Asserts the front end rejects `source` with `expected` occurrences of `code`.
fn assert_rejected(
    tag: &str,
    source: &str,
    code: &str,
    expected: usize,
    fragments: &[&str],
) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected {code}, but the front end accepted the program"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let found = stderr.matches(&format!("error {code}:")).count();
    assert_eq!(
        found, expected,
        "{tag}: expected {expected} {code} report(s), found {found}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the report to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not write a translation unit"
    );
}

/// Asserts an accepted program's exit code.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: unexpected exit code; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A call result widened into a non-nullable view names the call in E10010.
#[test]
fn should_name_a_call_result_in_the_possibly_null_report() {
    let source = r#"module spec.zn.d190;

import { String, stringOf } from std.text;

func get(): String? {
    return stringOf("ab");
}

func main(): i32 {
    let g: str = get();
    return i32(len(g)) - 2;
}
"#;
    assert_rejected(
        "d190",
        source,
        "E10010",
        1,
        &["value 'get()' may be null here"],
    );
}

/// The nullable index and store-index bases report inside `unsafe` too.
#[test]
fn should_report_a_nullable_index_base_inside_unsafe() {
    let read = r#"module spec.zn.d191read;

func f(xs: i32[]?): i32 {
    unsafe {
        return xs[0];
    }
}

func main(): i32 { return 0; }
"#;
    assert_rejected("d191read", read, "E10010", 1, &["value 'xs'"]);

    let store = r#"module spec.zn.d191store;

func f(borrow xs: i32[]?): void {
    unsafe {
        xs[0] = 1;
    }
}

func main(): i32 { return 0; }
"#;
    assert_rejected("d191store", store, "E10010", 1, &["value 'xs'"]);
}

/// A qualified read of a nullable slice checks the head's null state.
#[test]
fn should_report_a_qualified_read_of_a_nullable_view() {
    let source = r#"module spec.zn.d192;

func length(v: u8[]?): usize {
    return v.len;
}

func pointer(v: u8[]?): raw u8 {
    return v.ptr;
}

func main(): i32 { return 0; }
"#;
    assert_rejected(
        "d192",
        source,
        "E10010",
        2,
        &["value 'v' may be null here"],
    );
}

/// A nullable comparison narrows its short-circuit right operand and a loop
/// head narrows the loop body.
#[test]
fn should_narrow_a_short_circuit_right_and_a_loop_head() {
    let source = r#"module spec.zn.d193;

func count(s: str?): i32 {
    let n: i32 = 0;
    if (s != null && len(s) > 0) {
        n = n + 1;
    }
    if (s == null || len(s) > 0) {
        n = n + 1;
    }
    while (s != null && n < 2) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let t: str = "ab";
    return count(t) - 2;
}
"#;
    let output = run_strict_host("d193", source);
    assert_exit("d193", &output, 0);
}

/// `len` reads a cstring's runtime bytes and rejects a scalar operand.
#[test]
fn should_support_len_of_a_cstring_and_reject_scalars() {
    let accepted = r#"module spec.zn.d195;

import { toCString } from std.text;

func main(): i32 {
    let c: cstring = toCString("abcd");
    if (c == null) {
        return 1;
    }
    return i32(len(c)) - 4;
}
"#;
    let output = run_strict_host("d195", accepted);
    assert_exit("d195", &output, 0);

    let unchecked = r#"module spec.zn.d195unchecked;

import { toCString } from std.text;

func main(): i32 {
    let c: cstring = toCString("abcd");
    return i32(len(c)) - 4;
}
"#;
    assert_rejected("d195unchecked", unchecked, "E10010", 1, &["value 'c'"]);

    let usize_operand = r#"module spec.zn.d195usize;

func main(): i32 {
    let u: usize = 3;
    return i32(len(u));
}
"#;
    assert_rejected("d195usize", usize_operand, "E10200", 1, &["len"]);

    let u8_operand = r#"module spec.zn.d195u8;

func main(): i32 {
    let n: u8 = 3;
    return i32(len(n));
}
"#;
    assert_rejected("d195u8", u8_operand, "E10200", 1, &["len"]);

    let generic_scalar = r#"module spec.zn.d195generic;

func f<T>(x: T): i32 {
    return i32(len(x));
}

func main(): i32 {
    let n: u8 = 3;
    return f<u8>(n);
}
"#;
    assert_rejected("d195generic", generic_scalar, "E10200", 1, &["len"]);

    let generic_cstring = r#"module spec.zn.d195genericcstring;

func f<T>(x: T): i32 {
    return i32(len(x));
}

func main(): i32 {
    let c: cstring = null;
    return f<cstring>(c);
}
"#;
    assert_rejected(
        "d195genericcstring",
        generic_cstring,
        "E10010",
        1,
        &["value 'x'"],
    );

    let generic_cstring_narrowed = r#"module spec.zn.d195genericcstringok;

func f<T>(x: T): i32 {
    if (x == null) {
        return 0;
    }
    return i32(len(x));
}

func main(): i32 {
    let c: cstring = null;
    return f<cstring>(c);
}
"#;
    let output = run_strict_host("d195genericcstringok", generic_cstring_narrowed);
    assert_exit("d195genericcstringok", &output, 0);
}

/// The literal null in an aggregate field or array element reports the frozen
/// null rule.
#[test]
fn should_report_a_literal_null_in_an_aggregate_position() {
    let field = r#"module spec.zn.d246field;

struct P { n: i32; x: i32; }

func main(): i32 {
    let p: P = P { n: null, x: 1 };
    return p.x;
}
"#;
    assert_rejected("d246field", field, "E10012", 1, &["'null' is not allowed here"]);

    let element = r#"module spec.zn.d246element;

func main(): i32 {
    let a: i32[2] = [1, null];
    return a[0];
}
"#;
    assert_rejected(
        "d246element",
        element,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );
}

/// An unresolved null-shaped head has no member surface for a read or a store.
#[test]
fn should_report_a_member_read_or_store_through_a_null_shaped_head() {
    let length = r#"module spec.zn.d247len;

func main(): i32 {
    let x = null;
    let n: i32 = x.len;
    return n;
}
"#;
    assert_rejected("d247len", length, "E10212", 1, &["'len'"]);

    let field = r#"module spec.zn.d247field;

func main(): i32 {
    let x = null;
    let n: i32 = x.foo;
    return n;
}
"#;
    assert_rejected("d247field", field, "E10212", 1, &["'foo'"]);

    let store = r#"module spec.zn.d247store;

func main(): i32 {
    let x = null;
    x.foo = 1;
    return 0;
}
"#;
    assert_rejected("d247store", store, "E10212", 1, &["'foo'"]);

    let length_store = r#"module spec.zn.d247lenstore;

func main(): i32 {
    let x = null;
    x.len = 1;
    return 0;
}
"#;
    assert_rejected("d247lenstore", length_store, "E10212", 1, &["'len'"]);
}

/// A null-shaped value cannot stand in an arithmetic or comparison operand,
/// and an uncontracted ffi parameter rejects it through the same null rule.
#[test]
fn should_reject_a_null_shaped_operator_operand() {
    let arithmetic = r#"module spec.zn.d248arith;

func main(): i32 {
    let x = null;
    let a: i32 = x + 1;
    return a;
}
"#;
    assert_rejected(
        "d248arith",
        arithmetic,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );

    let comparison = r#"module spec.zn.d248cmp;

func main(): i32 {
    let x = null;
    let b: bool = x == 5;
    if (b) { return 1; }
    return 0;
}
"#;
    assert_rejected(
        "d248cmp",
        comparison,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );

    let ffi = r#"module spec.zn.d248ffi;

ffi probe {
    func F(v: i32): i32;
}

func main(): i32 {
    let x = null;
    unsafe {
        return F(x);
    }
}
"#;
    assert_rejected(
        "d248ffi",
        ffi,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );
}

/// A null-shaped condition is not a bool.
#[test]
fn should_reject_a_null_shaped_condition() {
    let source = r#"module spec.zn.d249;

func main(): i32 {
    let x = null;
    if (x) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected("d249", source, "E10218", 1, &["condition must be 'bool'"]);
}

/// An inferred generic call uses its resolved nullable parameter for a null
/// argument; the explicit nullable instance keeps the same acceptance.
#[test]
fn should_accept_a_null_argument_at_an_inferred_nullable_instance() {
    let source = r#"module spec.zn.d383;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let s: str? = "ab";
    let a: i32 = pair(s, null);
    let b: i32 = pair<str?>(s, null);
    return a + b;
}
"#;
    let output = run_strict_host("d383", source);
    assert_exit("d383", &output, 0);
}

/// TY-4 reaches the enum backing, lambda parameter, function type, ffi
/// parameter, and `sizeof` type positions.
#[test]
fn should_report_nullable_types_at_every_declaration_position() {
    let enum_backing = r#"module spec.zn.d417enum;

enum E: i32? { A, B }

func main(): i32 { return 0; }
"#;
    assert_rejected("d417enum", enum_backing, "E10011", 1, &["nullable type"]);

    let lambda_param = r#"module spec.zn.d417lambda;

func main(): i32 {
    let f = (x: i32?) => 0;
    return f(1);
}
"#;
    assert_rejected(
        "d417lambda",
        lambda_param,
        "E10011",
        1,
        &["nullable type"],
    );

    let func_type = r#"module spec.zn.d417func;

func apply(g: func(i32?): i32): i32 {
    return 0;
}

func main(): i32 {
    return apply(null);
}
"#;
    assert_rejected("d417func", func_type, "E10011", 1, &["nullable type"]);

    let ffi_param = r#"module spec.zn.d417ffi;

ffi probe {
    func F(x: i32?): i32;
}

func main(): i32 {
    unsafe {
        return F(1);
    }
}
"#;
    assert_rejected("d417ffi", ffi_param, "E10011", 1, &["nullable type"]);

    let sizeof_type = r#"module spec.zn.d417sizeof;

func main(): i32 {
    let n: usize = sizeof(i32?);
    return i32(n);
}
"#;
    assert_rejected(
        "d417sizeof",
        sizeof_type,
        "E10011",
        1,
        &["nullable type"],
    );
}

/// A negated float literal takes its f32 destination context.
#[test]
fn should_accept_a_negated_float_literal_at_f32() {
    let source = r#"module spec.zn.d420;

func main(): i32 {
    let x: f32 = -5.0;
    if (x < 0.0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_strict_host("d420", source);
    assert_exit("d420", &output, 1);
}

/// A raw self-copy of an out-slot borrow follows the direct self-assign and
/// reports the moved-value rule.
#[test]
fn should_reject_a_raw_out_slot_self_copy() {
    let raw = r#"module spec.zn.d467;

import { String } from std.text;

func wipe(borrow p: String?): void {
    unsafe {
        p = *(&p);
    }
}

func main(): i32 {
    let p: String? = null;
    wipe(borrow p);
    return 0;
}
"#;
    assert_rejected("d467", raw, "E10020", 1, &["'p' was moved"]);

    let direct = r#"module spec.zn.d467direct;

import { String } from std.text;

func wipe(borrow p: String?): void {
    p = p;
}

func main(): i32 {
    let p: String? = null;
    wipe(borrow p);
    return 0;
}
"#;
    assert_rejected("d467direct", direct, "E10020", 1, &["'p' was moved"]);
}

/// A generic non-nullable view out-slot store reports the concrete twin's
/// escape code; the concrete spelling keeps its own report.
#[test]
fn should_reject_a_generic_non_nullable_view_out_slot_store() {
    let generic = r#"module spec.zn.d542;

func copy<T>(borrow v: T, borrow slot: T): void {
    slot = v;
}

func main(): i32 {
    let v: str = "aa";
    let slot: str = "zz";
    copy<str>(borrow v, borrow slot);
    return i32(slot[0]) - 97;
}
"#;
    assert_rejected("d542", generic, "E10033", 1, &["'slot'"]);

    let concrete = r#"module spec.zn.d542c;

func copy(v: str, borrow slot: str): void {
    slot = v;
}

func main(): i32 {
    let v: str = "aa";
    let slot: str = "zz";
    copy(v, borrow slot);
    return i32(slot[0]) - 97;
}
"#;
    assert_rejected("d542c", concrete, "E10033", 1, &["'slot'"]);
}
