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

/// A fixed array has no empty state; every null destination reports E10012.
#[test]
fn should_reject_a_null_fixed_array_destination() {
    let local = r#"module spec.zn.n1local;

func main(): i32 {
    let a: i32[2] = null;
    return a[0];
}
"#;
    assert_rejected("n1local", local, "E10012", 1, &["'null' is not allowed here"]);

    let assignment = r#"module spec.zn.n1assign;

func main(): i32 {
    let a: i32[2] = [1, 2];
    a = null;
    return a[0];
}
"#;
    assert_rejected(
        "n1assign",
        assignment,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );

    let view_array = r#"module spec.zn.n1view;

func main(): i32 {
    let a: str[2] = null;
    return i32(len(a[0]));
}
"#;
    assert_rejected("n1view", view_array, "E10012", 1, &["'null' is not allowed here"]);

    let raw_array = r#"module spec.zn.n1raw;

func main(): i32 {
    let a: raw u8[2] = null;
    return 0;
}
"#;
    assert_rejected("n1raw", raw_array, "E10012", 1, &["'null' is not allowed here"]);

    let returned = r#"module spec.zn.n1return;

func f(): i32[2] {
    return null;
}

func main(): i32 {
    let a: i32[2] = f();
    return a[0];
}
"#;
    assert_rejected("n1return", returned, "E10012", 1, &["'null' is not allowed here"]);

    let field = r#"module spec.zn.n1field;

struct S { a: i32[2]; }

func main(): i32 {
    let s: S = S { a: null };
    return s.a[0];
}
"#;
    assert_rejected("n1field", field, "E10012", 1, &["'null' is not allowed here"]);

    let element = r#"module spec.zn.n1element;

func main(): i32 {
    let a: i32[2][2] = [null, [1, 2]];
    return a[1][0];
}
"#;
    assert_rejected(
        "n1element",
        element,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );

    let fill = r#"module spec.zn.n1fill;

func main(): i32 {
    let a: i32[2][2] = [; null];
    return a[1][0];
}
"#;
    assert_rejected("n1fill", fill, "E10012", 1, &["'null' is not allowed here"]);

    let global = r#"module spec.zn.n1global;

let g: i32[2] = null;

func main(): i32 {
    return g[0];
}
"#;
    assert_rejected("n1global", global, "E10012", 1, &["'null' is not allowed here"]);
}

/// A module initializer has no flow to narrow a cstring length; the operand
/// reports the possible-null rule.
#[test]
fn should_reject_a_module_initializer_cstring_length_without_a_check() {
    let global = r#"module spec.zn.n2global;

let c: cstring = null;
let n: usize = len(c);

func main(): i32 {
    return i32(n);
}
"#;
    assert_rejected("n2global", global, "E10010", 1, &["value 'c'"]);

    let field = r#"module spec.zn.n2field;

struct W { c: cstring; n: i32; }

let w: W = W { c: null, n: 0 };
let n: usize = len(w.c);

func main(): i32 {
    return i32(n);
}
"#;
    assert_rejected("n2field", field, "E10010", 1, &["value 'w'"]);
}

/// A write performed by a `&&` right operand invalidates the left narrowing
/// fact before the body.
#[test]
fn should_drop_a_narrowing_fact_written_by_a_short_circuit_right() {
    let string = r#"module spec.zn.n3string;

import { String, stringOf } from std.text;

func clear(borrow s: String?): bool {
    s = null;
    return true;
}

func main(): i32 {
    let s: String? = stringOf("abcd");
    if (s != null && clear(borrow s)) {
        return i32(len(s));
    }
    return 1;
}
"#;
    assert_rejected("n3string", string, "E10010", 1, &["value 's'"]);

    let cstring = r#"module spec.zn.n3cstring;

import { String, stringOf, toCString } from std.text;

struct W { name: String; c: cstring; }

func clear(borrow w: W): bool {
    w.c = null;
    return true;
}

func main(): i32 {
    let w: W = W { name: stringOf("ab"), c: toCString("abcd") };
    if (w.c != null && clear(borrow w)) {
        return i32(len(w.c)) - 4;
    }
    return 1;
}
"#;
    assert_rejected("n3cstring", cstring, "E10010", 1, &["value 'w'"]);
}

/// A short-circuit right operand keeps its moves and writes; the guarded body
/// cannot reuse a value it consumed or nulled.
#[test]
fn should_keep_a_short_circuit_right_operand_effects() {
    let take = r#"module spec.zn.memtake;

import { String, stringOf } from std.text;

func eat(take p: String): bool {
    return true;
}

func main(): i32 {
    let c: bool = true;
    let s: String = stringOf("ab");
    if (c && eat(take s)) { }
    return i32(len(s));
}
"#;
    assert_rejected("memtake", take, "E10020", 1, &["'s' was moved"]);

    let or_write = r#"module spec.zn.memorwrite;

import { String, stringOf } from std.text;

func reset(borrow p: String?): bool {
    p = null;
    return false;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    if (s == null || reset(borrow s)) {
        return 0;
    }
    return i32(len(s));
}
"#;
    assert_rejected("memorwrite", or_write, "E10010", 1, &["value 's'"]);

    let loop_write = r#"module spec.zn.memloopwrite;

import { String, stringOf } from std.text;

func reset(borrow p: String?): bool {
    p = null;
    return true;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    while (s != null && reset(borrow s)) {
        return i32(len(s));
    }
    return 0;
}
"#;
    assert_rejected("memloopwrite", loop_write, "E10010", 1, &["value 's'"]);
}

/// A null-shaped binding does not satisfy a non-nullable handle destination.
#[test]
fn should_reject_a_null_shaped_binding_at_a_non_nullable_handle() {
    let local = r#"module spec.zn.memhandle;

import { String } from std.text;

func main(): i32 {
    let x = null;
    let s: String = x;
    return i32(len(s));
}
"#;
    assert_rejected("memhandle", local, "E10012", 1, &["'null' is not allowed here"]);

    let assignment = r#"module spec.zn.memhandleassign;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("ab");
    let x = null;
    s = x;
    return i32(len(s));
}
"#;
    assert_rejected(
        "memhandleassign",
        assignment,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );

    let returned = r#"module spec.zn.memhandleret;

import { String } from std.text;

func get(): String {
    let x = null;
    return x;
}

func main(): i32 {
    let s: String = get();
    return i32(len(s));
}
"#;
    assert_rejected(
        "memhandleret",
        returned,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );
}

/// A module-level struct literal follows the same fixed-array null rule as a
/// function-local literal.
#[test]
fn should_reject_a_module_initializer_fixed_array_field_null() {
    let source = r#"module spec.zn.n1globalfield;

struct S { a: i32[2]; }

let s: S = S { a: null };

func main(): i32 {
    return s.a[0];
}
"#;
    assert_rejected(
        "n1globalfield",
        source,
        "E10012",
        1,
        &["'null' is not allowed here"],
    );
}

/// Module initializers and module bindings obey the same null rules as body
/// locals.
#[test]
fn should_reject_nullable_module_and_global_uses() {
    let module_call = r#"module spec.zn.p1call;

import { String, stringOf } from std.text;

func get(): String? {
    return stringOf("ab");
}

let g: str = get();

func main(): i32 {
    return i32(len(g)) - 2;
}
"#;
    assert_rejected("p1call", module_call, "E10010", 1, &["value 'get()'"]);

    let module_view = r#"module spec.zn.p1view;

let v: u8[]? = null;
let n: usize = v.len;

func main(): i32 {
    return i32(n);
}
"#;
    assert_rejected("p1view", module_view, "E10010", 1, &["value 'v'"]);

    let global_read = r#"module spec.zn.p2read;

let p: raw i32? = null;

func main(): i32 {
    unsafe {
        return p[0];
    }
}
"#;
    assert_rejected("p2read", global_read, "E10010", 1, &["value 'p'"]);

    let global_store = r#"module spec.zn.p2store;

let p: raw i32? = null;

func main(): i32 {
    unsafe {
        p[0] = 1;
    }
    return 0;
}
"#;
    assert_rejected("p2store", global_store, "E10010", 1, &["value 'p'"]);

    let global_member = r#"module spec.zn.p2member;

struct P { x: i32; }

let p: raw P? = null;

func main(): i32 {
    unsafe {
        p.x = 1;
    }
    return 0;
}
"#;
    assert_rejected("p2member", global_member, "E10010", 1, &["value 'p'"]);

    let global_cstring = r#"module spec.zn.p2cstring;

let c: cstring = null;

func main(): i32 {
    return i32(len(c));
}
"#;
    assert_rejected("p2cstring", global_cstring, "E10010", 1, &["value 'c'"]);

    let narrowed = r#"module spec.zn.p2narrow;

let p: raw i32? = null;

func main(): i32 {
    unsafe {
        if (p != null) {
            return p[0];
        }
    }
    return 0;
}
"#;
    let output = run_strict_host("p2narrow", narrowed);
    assert_exit("p2narrow", &output, 0);
}

/// The dot spelling of `len` and the generic dot spelling obey the cstring
/// null check and the instance length surface.
#[test]
fn should_judge_a_dot_len_receiver() {
    let cstring = r#"module spec.zn.p4dot;

func main(): i32 {
    let c: cstring = null;
    return i32(c.len());
}
"#;
    assert_rejected("p4dot", cstring, "E10010", 1, &["value 'c'"]);

    let narrowed = r#"module spec.zn.p4dotok;

import { toCString } from std.text;

func main(): i32 {
    let c: cstring = toCString("abcd");
    if (c == null) {
        return 1;
    }
    return i32(c.len()) - 4;
}
"#;
    let output = run_strict_host("p4dotok", narrowed);
    assert_exit("p4dotok", &output, 0);

    let generic = r#"module spec.zn.p5dot;

func f<T>(x: T): i32 {
    return i32(x.len());
}

func main(): i32 {
    let n: u8 = 3;
    return f<u8>(n);
}
"#;
    assert_rejected("p5dot", generic, "E10200", 1, &["len is not defined"]);
}

/// Nested short-circuit chains, `for`-C heads, and `?:` arms carry the
/// narrowing facts that their guarded position proves.
#[test]
fn should_narrow_nested_chains_for_c_and_conditional_arms() {
    let chain = r#"module spec.zn.p6chain;

func count(s: str?, t: str?): i32 {
    let n: i32 = 0;
    if (s != null && t != null && len(s) > 0 && len(t) > 0) {
        n = n + i32(len(s)) + i32(len(t));
    }
    return n;
}

func main(): i32 {
    let s: str = "ab";
    return count(s, s) - 4;
}
"#;
    let output = run_strict_host("p6chain", chain);
    assert_exit("p6chain", &output, 0);

    let for_c = r#"module spec.zn.p6for;

func count(s: str?): i32 {
    let n: i32 = 0;
    for (let i: usize = 0; s != null && i < 2; i = i + 1) {
        n = n + i32(len(s));
    }
    return n;
}

func main(): i32 {
    let s: str = "ab";
    return count(s) - 4;
}
"#;
    let output = run_strict_host("p6for", for_c);
    assert_exit("p6for", &output, 0);

    let while_body = r#"module spec.zn.p6while;

func count(s: str?): i32 {
    let n: i32 = 0;
    while (s != null && n < 4) {
        n = n + i32(len(s));
    }
    return n;
}

func main(): i32 {
    let s: str = "ab";
    return count(s) - 4;
}
"#;
    let output = run_strict_host("p6while", while_body);
    assert_exit("p6while", &output, 0);

    let conditional = r#"module spec.zn.p6cond;

func main(): i32 {
    let c: cstring = null;
    let n: usize = c == null ? 0 : len(c);
    return i32(n);
}
"#;
    let output = run_strict_host("p6cond", conditional);
    assert_exit("p6cond", &output, 0);
}

/// A nullable raw member or element store reports before the write; a
/// narrowed one stays accepted.
#[test]
fn should_reject_a_nullable_raw_store_target() {
    let local = r#"module spec.zn.p7local;

func write(borrow p: raw i32?): void {
    unsafe {
        p[0] = 1;
    }
}

func main(): i32 { return 0; }
"#;
    assert_rejected("p7local", local, "E10010", 1, &["value 'p'"]);

    let member = r#"module spec.zn.p7member;

struct P { x: i32; }

func write(borrow p: raw P?): void {
    unsafe {
        p.x = 1;
    }
}

func main(): i32 { return 0; }
"#;
    assert_rejected("p7member", member, "E10010", 1, &["value 'p'"]);
}

/// FFI declarations face TY-4 at the declaration, and a nameless call callee
/// still gets the possible-null report.
#[test]
fn should_report_ffi_types_and_nameless_call_results() {
    let unused_param = r#"module spec.zn.p8param;

ffi probe {
    func F(x: i32?): i32;
}

func main(): i32 { return 0; }
"#;
    assert_rejected("p8param", unused_param, "E10011", 1, &["nullable type"]);

    let unused_return = r#"module spec.zn.p8return;

ffi probe {
    func G(): i32?;
}

func main(): i32 { return 0; }
"#;
    assert_rejected("p8return", unused_return, "E10011", 1, &["nullable type"]);

    let used_return = r#"module spec.zn.p8usedreturn;

ffi probe {
    func G(): i32?;
}

func main(): i32 {
    unsafe {
        return G();
    }
}
"#;
    assert_rejected("p8usedreturn", used_return, "E10011", 1, &["nullable type"]);

    let lambda = r#"module spec.zn.p9lambda;

import { String, stringOf } from std.text;

func get(): String? {
    return stringOf("ab");
}

func main(): i32 {
    let g: str = (() => get())();
    return i32(len(g)) - 2;
}
"#;
    assert_rejected("p9lambda", lambda, "E10010", 1, &["value 'call()'"]);
}

/// A module conditional narrows a global cstring for its non-null arm.
#[test]
fn should_narrow_a_module_initializer_conditional() {
    let source = r#"module spec.zn.p1cond;

let c: cstring = null;
let n: usize = c == null ? 0 : len(c);

func main(): i32 {
    return i32(n);
}
"#;
    let output = run_strict_host("p1cond", source);
    assert_exit("p1cond", &output, 0);
}

/// A `borrow` declaration is authoritative even when the call omits the marker.
#[test]
fn should_judge_an_unmarked_borrow_call_as_a_write() {
    let source = r#"module spec.zn.unmarkedborrow;

import { String, stringOf } from std.text;

func clear(borrow s: String?): bool {
    s = null;
    return true;
}

func main(): i32 {
    let s: String? = stringOf("abcd");
    if (s != null && clear(s)) {
        return i32(len(s));
    }
    return 1;
}
"#;
    assert_rejected("unmarkedborrow", source, "E10010", 1, &["value 's'"]);
}

/// The dot `len` spelling in a module initializer checks its cstring receiver.
#[test]
fn should_judge_a_module_initializer_dot_len() {
    let global = r#"module spec.zn.moduledotlen;

let c: cstring = null;
let n: usize = c.len();

func main(): i32 {
    return i32(n);
}
"#;
    assert_rejected("moduledotlen", global, "E10010", 1, &["value 'c'"]);

    let field = r#"module spec.zn.moduledotlenfield;

struct W { c: cstring; }

let w: W = W { c: null };
let n: usize = w.c.len();

func main(): i32 {
    return i32(n);
}
"#;
    assert_rejected("moduledotlenfield", field, "E10010", 1, &["value 'w'"]);
}

/// A global narrowing fact is scoped to the branch or loop that proved it, and
/// a user call may write a global.
#[test]
fn should_scope_and_invalidate_global_narrowing() {
    let after_if = r#"module spec.zn.globalnarrowif;

let g: cstring = null;

func main(): i32 {
    if (g != null && true) { }
    return i32(len(g));
}
"#;
    assert_rejected("globalnarrowif", after_if, "E10010", 1, &["value 'g'"]);

    let after_loop = r#"module spec.zn.globalnarrowloop;

let g: cstring = null;

func main(): i32 {
    let n: i32 = 0;
    while (g != null && n < 1) {
        n = n + 1;
    }
    return i32(len(g));
}
"#;
    assert_rejected(
        "globalnarrowloop",
        after_loop,
        "E10010",
        1,
        &["value 'g'"],
    );

    let call_in_condition = r#"module spec.zn.globalcallcond;

import { toCString } from std.text;

let g: cstring = toCString("ab");

func reset(): bool {
    g = null;
    return true;
}

func main(): i32 {
    if (g != null && reset()) {
        return i32(len(g));
    }
    return 0;
}
"#;
    assert_rejected(
        "globalcallcond",
        call_in_condition,
        "E10010",
        1,
        &["value 'g'"],
    );

    let call_in_body = r#"module spec.zn.globalcallbody;

import { toCString } from std.text;

let g: cstring = toCString("ab");

func reset(): bool {
    g = null;
    return true;
}

func main(): i32 {
    if (g != null) {
        reset();
        return i32(len(g));
    }
    return 0;
}
"#;
    assert_rejected("globalcallbody", call_in_body, "E10010", 1, &["value 'g'"]);
}

/// A global narrowed by a guard that returns on the null path stays narrowed
/// afterward.
#[test]
fn should_keep_a_global_narrowed_after_an_early_return() {
    let source = r#"module spec.zn.globalearlyreturn;

import { toCString } from std.text;

let g: cstring = toCString("abcd");

func main(): i32 {
    if (g == null) {
        return 1;
    }
    return i32(len(g)) - 4;
}
"#;
    let output = run_strict_host("globalearlyreturn", source);
    assert_exit("globalearlyreturn", &output, 0);
}

/// Module-initializer short-circuit chains narrow their right operands.
#[test]
fn should_narrow_a_module_initializer_short_circuit() {
    let and = r#"module spec.zn.moduleand;

let c: cstring = null;
let b: bool = c != null && len(c) > 0;

func main(): i32 {
    if (b) { return 1; }
    return 0;
}
"#;
    let output = run_strict_host("moduleand", and);
    assert_exit("moduleand", &output, 0);

    let or = r#"module spec.zn.moduleor;

let c: cstring = null;
let b: bool = c == null || len(c) > 0;

func main(): i32 {
    if (b) { return 1; }
    return 0;
}
"#;
    let output = run_strict_host("moduleor", or);
    assert_exit("moduleor", &output, 1);
}

/// Module-initializer call arguments and template placeholders face their
/// concrete destinations.
#[test]
fn should_judge_module_initializer_call_and_template_arguments() {
    let call = r#"module spec.zn.modulecallarg;

import { String, stringOf } from std.text;

func get(): String? {
    return stringOf("ab");
}

func f(x: str): i32 {
    return i32(len(x));
}

let n: i32 = f(get());

func main(): i32 {
    return n - 2;
}
"#;
    assert_rejected("modulecallarg", call, "E10010", 1, &["value 'get()'"]);

    let template = r#"module spec.zn.moduletemplate;

import { String, stringOf } from std.text;

func get(): String? {
    return stringOf("ab");
}

let s: str = $"x{get()}";

func main(): i32 {
    return i32(len(s)) - 3;
}
"#;
    assert_rejected("moduletemplate", template, "E10010", 1, &["value 'get()'"]);
}

/// Module-initializer raw reads obey the unsafe gate.
#[test]
fn should_gate_module_initializer_raw_reads() {
    let index = r#"module spec.zn.moduleindex;

let a: i32 = 7;
let p: raw i32 = unsafe &a;
let n: i32 = p[0];

func main(): i32 { return n - 7; }
"#;
    assert_rejected("moduleindex", index, "E10602", 1, &["pointer arithmetic"]);

    let member = r#"module spec.zn.modulemember;

struct P { x: i32; }

let p: P = P { x: 7 };
let q: raw P = unsafe &p;
let n: i32 = q.x;

func main(): i32 { return n - 7; }
"#;
    assert_rejected("modulemember", member, "E10600", 1, &["dereference"]);

    let address = r#"module spec.zn.moduleaddr;

let a: i32 = 7;
let p: raw i32 = &a;

func main(): i32 { return 0; }
"#;
    assert_rejected("moduleaddr", address, "E10601", 1, &["address-of"]);
}

/// An outer local narrowing survives an inner short-circuit transient.
#[test]
fn should_keep_an_outer_narrowing_through_an_inner_short_circuit() {
    let source = r#"module spec.zn.outernarrow;

func f(s: str?): i32 {
    if (s != null) {
        if (s != null && len(s) > 0) { } else {
            return i32(len(s));
        }
        return i32(len(s));
    }
    return 0;
}

func main(): i32 {
    let s: str = "ab";
    return f(s) - 2;
}
"#;
    let output = run_strict_host("outernarrow", source);
    assert_exit("outernarrow", &output, 0);
}

/// A call through a function value can write a global.
#[test]
fn should_invalidate_global_narrowing_through_a_function_value() {
    let source = r#"module spec.zn.fnptrglobal;

import { toCString } from std.text;

let g: cstring = toCString("ab");

func reset(): bool {
    g = null;
    return true;
}

func main(): i32 {
    let f: func(): bool = reset;
    if (g != null && f()) {
        return i32(len(g));
    }
    return 0;
}
"#;
    assert_rejected("fnptrglobal", source, "E10010", 1, &["value 'g'"]);
}

/// The dot `len` spelling narrows in the branch where its guard holds.
#[test]
fn should_narrow_a_dot_len_receiver_in_the_guarded_branch() {
    let local = r#"module spec.zn.conddotlocal;

import { toCString } from std.text;

func main(): i32 {
    let c: cstring = toCString("ab");
    if (c != null && c.len() > 0) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_strict_host("conddotlocal", local);
    assert_exit("conddotlocal", &output, 1);

    let global = r#"module spec.zn.globaldotguard;

import { toCString } from std.text;

let g: cstring = toCString("ab");

func main(): i32 {
    if (g != null) {
        return i32(g.len()) - 2;
    }
    return 0;
}
"#;
    let output = run_strict_host("globaldotguard", global);
    assert_exit("globaldotguard", &output, 0);

    let conditional = r#"module spec.zn.globaldotcond;

import { toCString } from std.text;

let g: cstring = toCString("ab");

func main(): i32 {
    return (g != null) ? i32(g.len()) - 2 : 0;
}
"#;
    let output = run_strict_host("globaldotcond", conditional);
    assert_exit("globaldotcond", &output, 0);

    let module_and = r#"module spec.zn.moduleanddot;

import { toCString } from std.text;

let c: cstring = toCString("ab");
let n: i32 = (c != null && c.len() > 0) ? i32(c.len()) : 0;

func main(): i32 {
    return n - 2;
}
"#;
    let output = run_strict_host("moduleanddot", module_and);
    assert_exit("moduleanddot", &output, 0);
}

/// A callable value shadowing a module name uses its own parameter modes, and
/// a func-typed value may write a global.
#[test]
fn should_resolve_callable_identity_and_func_value_globals() {
    let local = r#"module spec.zn.f1rlocal;

import { String, stringOf } from std.text;

func clear(s: String): bool { return true; }
func reset(borrow s: String?): bool { s = null; return true; }

func main(): i32 {
    let s: String? = stringOf("abcd");
    let clear: func(borrow String?): bool = reset;
    if (s != null && clear(s)) {
        return i32(len(s));
    }
    return 1;
}
"#;
    assert_rejected("f1rlocal", local, "E10010", 1, &["value 's'"]);

    let field = r#"module spec.zn.f1rfield;

import { String, stringOf } from std.text;

struct S { clear: func(borrow String?): bool; }
func clear(s: String): bool { return true; }
func reset(borrow s: String?): bool { s = null; return true; }

func main(): i32 {
    let s: String? = stringOf("abcd");
    let st: S = S { clear: reset };
    if (s != null && st.clear(s)) {
        return i32(len(s));
    }
    return 1;
}
"#;
    assert_rejected("f1rfield", field, "E10010", 1, &["value 's'"]);

    let func_global = r#"module spec.zn.f1rglobal;

import { toCString } from std.text;

let g: cstring = toCString("abcd");
func resetG(): bool { g = null; return true; }

func main(): i32 {
    let f: func(): bool = resetG;
    if (g != null) {
        f();
        return i32(len(g));
    }
    return 0;
}
"#;
    assert_rejected("f1rglobal", func_global, "E10010", 1, &["value 'g'"]);
}

/// A fact restored after a short-circuit right operand is dropped when that
/// operand writes the owner.
#[test]
fn should_not_restore_a_fact_written_by_the_right_operand() {
    let local = r#"module spec.zn.l1rlocal;

import { String, stringOf } from std.text;

func reset(borrow s: String?): bool { s = null; return true; }

func main(): i32 {
    let s: String? = stringOf("abcd");
    if (s != null) {
        if (s != null && reset(s)) { }
        return i32(len(s));
    }
    return 0;
}
"#;
    assert_rejected("l1rlocal", local, "E10010", 1, &["value 's'"]);

    let global = r#"module spec.zn.l1rglobal;

import { toCString } from std.text;

let g: cstring = toCString("abcd");
func resetG(): bool { g = null; return true; }

func main(): i32 {
    if (g != null) {
        if (g != null && resetG()) { }
        return i32(len(g));
    }
    return 0;
}
"#;
    assert_rejected("l1rglobal", global, "E10010", 1, &["value 'g'"]);

    let module = r#"module spec.zn.l1rmodule;

import { toCString } from std.text;

let g: cstring = toCString("abcd");
func resetG(): bool { g = null; return true; }
let b: bool = g != null && (resetG() && len(g) > 0);

func main(): i32 {
    if (b) { return 1; }
    return 0;
}
"#;
    assert_rejected("l1rmodule", module, "E10010", 1, &["value 'g'"]);
}

/// A dot `len` receiver rooted at a call result has no place to narrow.
#[test]
fn should_reject_a_member_of_a_call_result_as_a_dot_len_receiver() {
    let module = r#"module spec.zn.m1rmodule;

struct W { c: cstring; }
func make(): W { return W { c: null }; }

let n: usize = make().c.len();

func main(): i32 {
    return i32(n);
}
"#;
    assert_rejected("m1rmodule", module, "E10010", 1, &["value 'c'"]);

    let body = r#"module spec.zn.m1rbody;

struct W { c: cstring; }
func make(): W { return W { c: null }; }

func main(): i32 {
    return i32(make().c.len());
}
"#;
    assert_rejected("m1rbody", body, "E10010", 1, &["value 'c'"]);
}

/// Module generic calls substitute their explicit type arguments, and a dot
/// method call checks its written arguments.
#[test]
fn should_check_module_generic_and_dot_method_arguments() {
    let generic = r#"module spec.zn.m3rgeneric;

import { String, stringOf } from std.text;

func get(): String? { return stringOf("ab"); }
func f<T>(x: T): i32 { return i32(len(x)); }

let n: i32 = f<str>(get());

func main(): i32 {
    return n - 2;
}
"#;
    assert_rejected("m3rgeneric", generic, "E10010", 1, &["value 'get()'"]);

    let nullable = r#"module spec.zn.m3rnullable;

func f<T>(x: T): i32 { return 1; }

let n: i32 = f<String?>(null);

func main(): i32 {
    return n - 1;
}
"#;
    let output = run_strict_host("m3rnullable", nullable);
    assert_exit("m3rnullable", &output, 0);

    let method = r#"module spec.zn.m3rmethod;

import { String, stringOf } from std.text;

struct W { x: i32; }
func make(): W { return W { x: 1 }; }
func get(): String? { return stringOf("ab"); }
func h(borrow recv: W, x: str): i32 { return i32(len(x)) + recv.x; }

let n: i32 = make().h(get());

func main(): i32 {
    return n - 3;
}
"#;
    assert_rejected("m3rmethod", method, "E10010", 1, &["value 'get()'"]);
}

/// Module-initializer raw arithmetic, casts, and volatile reads keep the body's
/// unsafe gate.
#[test]
fn should_gate_module_raw_arithmetic_casts_and_volatile() {
    let arithmetic = r#"module spec.zn.m4rarith;

let a: i32 = 7;
let p: raw i32 = unsafe &a;
let q: raw i32 = p + 1;

func main(): i32 { return 0; }
"#;
    assert_rejected("m4rarith", arithmetic, "E10602", 1, &["pointer arithmetic"]);

    let cast = r#"module spec.zn.m4rcast;

let a: i32 = 7;
let p: raw i32 = unsafe &a;
let q: raw u8 = raw u8(p);

func main(): i32 { return 0; }
"#;
    assert_rejected("m4rcast", cast, "E10603", 1, &["raw pointer cast"]);

    let volatile = r#"module spec.zn.m4rvolatile;

let a: u32 = 7;
let p: raw u32 = unsafe &a;
let v: u32 = readVolatile32(p);

func main(): i32 { return i32(v) - 7; }
"#;
    assert_rejected("m4rvolatile", volatile, "E10604", 1, &["volatile"]);
}

/// A raw pointer argument may alias a narrowed local; the call is a write.
#[test]
fn should_treat_a_raw_pointer_call_argument_as_a_local_write() {
    let source = r#"module spec.zn.l1rraw;

import { toCString } from std.text;

func clearRaw(p: raw cstring): bool {
    unsafe {
        *p = null;
        return true;
    }
}

func main(): i32 {
    let c: cstring = toCString("abcd");
    let p: raw cstring = unsafe &c;
    if (c != null && clearRaw(p)) {
        return i32(len(c));
    }
    return 0;
}
"#;
    assert_rejected("l1rraw", source, "E10010", 1, &["value 'c'"]);
}

/// Dot `len` on a conditional, dereference, or call-result element checks the
/// non-place cstring root.
#[test]
fn should_check_non_place_cstring_dot_len_roots() {
    let conditional = r#"module spec.zn.m1rcond;

import { toCString } from std.text;

func f(flag: bool): i32 {
    let a: cstring = toCString("ab");
    let b: cstring = null;
    return i32((flag ? a : b).len());
}

func main(): i32 {
    return f(false);
}
"#;
    assert_rejected("m1rcond", conditional, "E10010", 1, &["may be null"]);

    let deref = r#"module spec.zn.m1rderef;

func main(): i32 {
    let c: cstring = null;
    let p: raw cstring = unsafe &c;
    unsafe {
        return i32((*p).len());
    }
}
"#;
    assert_rejected("m1rderef", deref, "E10010", 1, &["may be null"]);

    let array = r#"module spec.zn.m1rarray;

struct W { cs: cstring[2]; }

func make(): W {
    return W { cs: [null, null] };
}

func main(): i32 {
    return i32(make().cs[0].len());
}
"#;
    assert_rejected("m1rarray", array, "E10010", 1, &["may be null"]);
}

/// A sibling field write does not invalidate a different field's narrowing.
#[test]
fn should_keep_a_narrowing_across_a_sibling_field_write() {
    let source = r#"module spec.zn.l1rsibling;

import { String, stringOf } from std.text;

struct W { a: String?; b: String?; }

func resetB(borrow b: String?): bool {
    b = null;
    return true;
}

func main(): i32 {
    let w: W = W { a: stringOf("ab"), b: stringOf("cd") };
    if (w.a != null) {
        if (w.a != null && resetB(w.b)) { }
        return i32(len(w.a)) - 2;
    }
    return 0;
}
"#;
    let output = run_strict_host("l1rsibling", source);
    assert_exit("l1rsibling", &output, 0);
}
