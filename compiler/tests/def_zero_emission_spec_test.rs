//! Specification tests for the zero-defect emission and ABI batch.
//!
//! Every case derives from a filed row's reproduction and the frozen
//! specification: an accepted program must emit a strict C99 translation unit
//! and run, a rejected program must report its frozen diagnostic and write no
//! C, and a stale row pins the already-correct behavior.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-zeroemission-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Compiles one program with optional extra flags.
fn compile_with(tag: &str, source: &str, extra: &[&str]) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let mut command = Command::new(env!("CARGO_BIN_EXE_clc"));
    command.args(["--profile", "host"]);
    command.args(extra);
    command.args([
        "-o",
        c_path.to_str().expect("C path"),
        source_path.to_str().expect("source path"),
    ]);
    let output = command.output().expect("failed to spawn clc");
    (scratch, output, c_path)
}

/// Compiles one host program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_with(tag, source, &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Builds one emitted translation unit under the strict host warning set.
fn build_strict(tag: &str, emitted: &str) -> (ScratchDir, PathBuf) {
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
    let (_scratch, program) = build_strict(tag, &emitted);
    Command::new(&program)
        .output()
        .expect("failed to spawn the built program")
}

/// Asserts one diagnostic count and optional message fragments.
fn assert_rejected(tag: &str, source: &str, code: &str, expected: usize, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_with(tag, source, &[]);
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
    assert!(!c_path.exists(), "{tag}: a rejected program must not write C");
}

/// Asserts one exit code.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: unexpected exit code; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Counts non-overlapping occurrences of `needle` in `text`.
fn count(text: &str, needle: &str) -> usize {
    text.match_indices(needle).count()
}

// ---------------------------------------------------------------------------
// a constant-bounds `slice` of a runtime view.
// ---------------------------------------------------------------------------

/// `slice(v, 0, 0)` on a runtime view emits no always-false guard.
///
/// The upper bound is the constant zero, so `0 > v.len` is false for every
/// unsigned length; emitting it trips `-Werror=type-limits`. The zero-length
/// slice is accepted, builds strictly, and reads back as empty.
#[test]
fn should_not_emit_an_always_false_guard_for_a_zero_high_bound() {
    let source = r#"module spec.ze.zero_slice;

import { slice } from std.mem;

func useView(v: u8[]): i32 {
    let w: u8[] = slice(v, 0, 0);
    return i32(len(w));
}

func main(): i32 {
    let xs: u8[3] = [1, 2, 3];
    return useView(slice(xs, 0, 3));
}
"#;
    let emitted = emit_host("zero_slice", source);
    assert!(
        !emitted.contains("(usize)(0) >"),
        "zero_slice: the always-false upper-bound comparison must not be emitted; C:\n{emitted}"
    );
    let output = run_strict_host("zero_slice", source);
    assert_exit("zero_slice", &output, 0);
}

// ---------------------------------------------------------------------------
// a closure-call argument needing the `str` widening.
// ---------------------------------------------------------------------------

/// A `String` argument to a `str` lambda parameter widens to the view.
///
/// The named-call path widens the owned handle; the closure-call path must do
/// the same, or the emitted `ClString` argument fails the strict build.
#[test]
fn should_widen_a_string_argument_at_a_closure_call() {
    let source = r#"module spec.ze.closure_widen;

import { stringOf } from std.text;

func main(): i32 {
    let n: i32 = ((s: str) => i32(len(s)))(stringOf("hello"));
    return n - 5;
}
"#;
    let emitted = emit_host("closure_widen", source);
    assert!(
        emitted.contains("cl_string_view("),
        "closure_widen: the owned handle must widen to a view; C:\n{emitted}"
    );
    let output = run_strict_host("closure_widen", source);
    assert_exit("closure_widen", &output, 0);
}

// ---------------------------------------------------------------------------
// a constant zero divisor inside a lambda body.
// ---------------------------------------------------------------------------

/// A literal zero divisor is a runtime fault, never a bare C division.
///
/// The value renderer turns the literal division into the E11210 trap, so the
/// translation unit builds under `-Werror=div-by-zero` and the program aborts
/// with the frozen fault line.
#[test]
fn should_emit_the_division_trap_for_a_literal_zero_in_a_lambda() {
    let source = r#"module spec.ze.lambda_zero;

func main(): i32 {
    let f: func(): i32 = () => 1 / 0;
    return f();
}
"#;
    let emitted = emit_host("lambda_zero", source);
    assert!(
        !emitted.contains("1 / 0") && !emitted.contains("1/0"),
        "lambda_zero: the bare division must not be emitted; C:\n{emitted}"
    );
    assert!(
        emitted.contains("E11210"),
        "lambda_zero: the literal divisor must lower to the trap; C:\n{emitted}"
    );
    let output = run_strict_host("lambda_zero", source);
    assert_ne!(output.status.code(), Some(0), "lambda_zero: the trap must abort");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("E11210"),
        "lambda_zero: the trap must print E11210; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// a fixed-array field built from an array literal.
// ---------------------------------------------------------------------------

/// A module-initializer fixed-array field emits one division guard.
///
/// The walk evaluates the element and the store core also checks it; the
/// second guard is redundant. The program is correct and the strict build
/// passes either way, so the pin counts the emitted guard.
#[test]
fn should_emit_one_division_guard_for_an_initializer_array_field() {
    let source = r#"module spec.ze.one_guard;

struct BoxA { arr: i32[2]; }

let d: i32 = 1;
let b: BoxA = BoxA { arr: [12 / d, 2] };

func main(): i32 {
    return b.arr[0] + b.arr[1] - 14;
}
"#;
    let emitted = emit_host("one_guard", source);
    assert_eq!(
        count(&emitted, "integer division by zero"),
        1,
        "one_guard: the initializer must emit one division guard; C:\n{emitted}"
    );
    let output = run_strict_host("one_guard", source);
    assert_exit("one_guard", &output, 0);
}

// ---------------------------------------------------------------------------
// / : recorded instances and field expansion.
// ---------------------------------------------------------------------------

/// A `sizeof` type argument expands the instance's nested field types.
///
/// `sizeOf<Box<i32>>()` names `ClBox__i32`, whose field `Pair<T>` becomes
/// `Pair<i32>`; the nested `ClPair__i32` must be defined before the struct.
#[test]
fn should_expand_a_recorded_type_argument_instance() {
    let source = r#"module spec.ze.instance_expand;

struct Pair<T> { n: T; }
struct Box<T> { p: Pair<T>; }

func sizeOf<T>(): usize { return sizeof(T[]); }

func main(): i32 {
    let n: usize = sizeOf<Box<i32>>();
    let m: usize = sizeof(Box<i32>[]);
    return i32(n - m);
}
"#;
    let emitted = emit_host("instance_expand", source);
    assert!(
        emitted.contains("struct ClPair__i32"),
        "instance_expand: the nested Pair instance must be defined; C:\n{emitted}"
    );
    let output = run_strict_host("instance_expand", source);
    assert_exit("instance_expand", &output, 0);
}

/// A module-initializer `sizeof(Wrap<i32>[])` records the instance.
///
/// The initializer spelling reaches the same type argument as the body twin,
/// so `ClWrap__i32` must be defined for the slice typedef.
#[test]
fn should_record_an_initializer_type_argument_instance() {
    let source = r#"module spec.ze.initializer_instance;

struct Wrap<T> { n: T; }

let n: usize = sizeof(Wrap<i32>[]);

func main(): i32 {
    let m: usize = sizeof(Wrap<i32>[]);
    return i32(n - m);
}
"#;
    let emitted = emit_host("initializer_instance", source);
    assert!(
        emitted.contains("struct ClWrap__i32"),
        "initializer_instance: the initializer instance must be defined; C:\n{emitted}"
    );
    let output = run_strict_host("initializer_instance", source);
    assert_exit("initializer_instance", &output, 0);
}

// ---------------------------------------------------------------------------
// the concrete `len(cstring)` value form.
// ---------------------------------------------------------------------------

/// A `cstring` length requires a null check and then reads the byte count.
///
/// The unchecked spelling reports `E10010`; the narrowed spelling emits the
/// runtime helper and runs.
#[test]
fn should_require_a_null_check_for_a_cstring_length() {
    let unchecked = r#"module spec.ze.cstring_len_unchecked;

import { toCString } from std.text;

func main(): i32 {
    let s: cstring = toCString("ab");
    let n: usize = len(s);
    return i32(n) - 2;
}
"#;
    assert_rejected(
        "cstring_len_unchecked",
        unchecked,
        "E10010",
        1,
        &["may be null"],
    );

    let checked = r#"module spec.ze.cstring_len_checked;

import { toCString } from std.text;

func main(): i32 {
    let s: cstring = toCString("ab");
    if (s != null) {
        let n: usize = len(s);
        return i32(n) - 2;
    }
    return 1;
}
"#;
    let emitted = emit_host("cstring_len_checked", checked);
    assert!(
        emitted.contains("cl_cstring_len("),
        "cstring_len_checked: the length must read the runtime helper; C:\n{emitted}"
    );
    let output = run_strict_host("cstring_len_checked", checked);
    assert_exit("cstring_len_checked", &output, 0);
}

// ---------------------------------------------------------------------------
// the raw unsafe gate through a generic instance.
// ---------------------------------------------------------------------------

/// A raw member read inside a generic body keeps the `unsafe` requirement.
///
/// The concrete `p.foo` reports `E10600`; the instance at `raw S` must report
/// the same rule at the body, not accept and emit `x->foo`.
#[test]
fn should_keep_the_raw_unsafe_gate_at_a_generic_instance() {
    let source = r#"module spec.ze.raw_gate;

struct S { foo: i32; }

func get<T>(x: T): i32 { return x.foo; }

func main(): i32 {
    let s: S = S { foo: 7 };
    let p: raw S = unsafe &s;
    return get<raw S>(p) - 7;
}
"#;
    assert_rejected(
        "raw_gate",
        source,
        "E10600",
        1,
        &["dereference of a raw pointer requires 'unsafe'"],
    );
}

// ---------------------------------------------------------------------------
// guarded literal spellings.
// ---------------------------------------------------------------------------

/// The guarded literal spellings build under the strict warning set.
///
/// The guarded conditional-root field, the guarded element conditional with
/// call-filled arms, and the guarded ternary literal are the derived probes
/// for the row. Each selects its value and runs.
#[test]
fn should_build_the_guarded_literal_spellings() {
    let source = r#"module spec.ze.guarded_literals;

struct Box { row: i32[2]; n: i32; }
struct Holder { names: str[1]; }

func id(x: i32): i32 { return x; }

func main(): i32 {
    let g: bool = true;
    let c: bool = true;
    let a: Box = Box { row: [1, 2], n: 3 };
    let b: Box = Box { row: [4, 5], n: 6 };
    let s: str = "AA";
    let r1: bool = g && i32(len(Holder { names: [s] }.names[0])) > 0;
    let r2: bool = g && (c ? a : b).row[0] > 0;
    let r3: bool = g && (c ? [id(1), id(2)] : [3, 4])[0] > 0;
    let r4: bool = g && (c ? 1 : 2) > 0;
    return (r1 ? 1 : 0) + (r2 ? 1 : 0) + (r3 ? 1 : 0) + (r4 ? 1 : 0) - 4;
}
"#;
    let output = run_strict_host("guarded_literals", source);
    assert_exit("guarded_literals", &output, 0);
}

// ---------------------------------------------------------------------------
// / : once-only array fills.
// ---------------------------------------------------------------------------

/// A call fill evaluates once and copies its value to every element.
#[test]
fn should_evaluate_a_call_fill_once() {
    let source = r#"module spec.ze.call_fill;

let calls: i32 = 0;

func f(): i32 {
    calls = calls + 1;
    return 2;
}

func main(): i32 {
    let xs: i32[3] = [; f()];
    return xs[0] + xs[1] + xs[2] - 6 + calls - 1;
}
"#;
    let output = run_strict_host("call_fill", source);
    assert_exit("call_fill", &output, 0);
}

/// A deferred-arm update fill evaluates its update once before the stores.
#[test]
fn should_evaluate_a_deferred_update_fill_once() {
    let source = r#"module spec.ze.deferred_fill;

func take3(xs: i32[3]): i32 { return xs[0] + xs[1] + xs[2]; }

func main(): i32 {
    let i: i32 = 1;
    let n: i32 = take3([; i++]);
    return n - 3 + (i - 2);
}
"#;
    let output = run_strict_host("deferred_fill", source);
    assert_exit("deferred_fill", &output, 0);
}

// ---------------------------------------------------------------------------
// the loop-condition mixed-arm receiver.
// ---------------------------------------------------------------------------

/// The mixed-arm receiver in a loop condition builds under the strict flags.
///
/// The fresh arm's value owns a `String`, so the deferred spelling nests a
/// fill and a designator read in one C expression; GCC 15 flags the order as
/// a sequence-point violation. The receiver must settle per iteration, and
/// both arms must run their selected callable.
#[test]
fn should_settle_a_mixed_arm_receiver_in_a_loop_condition() {
    let source = r#"module spec.ze.loop_receiver;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addTen(x: i32): i32 { return x + 10; }
func neg(x: i32): i32 { return 0 - x; }
func mkH(): H { return H { call: addTen, s: stringOf("mk") }; }

func use(c: bool, take a: H, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && (c ? mkH() : a).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    let a1: H = H { call: neg, s: stringOf("aa") };
    let n: i32 = use(true, a1, 1) - 1;
    let a2: H = H { call: neg, s: stringOf("bb") };
    return n + use(false, a2, 1);
}
"#;
    let output = run_strict_host("loop_receiver", source);
    assert_exit("loop_receiver", &output, 0);
}

// ---------------------------------------------------------------------------
// a raw deref source is used.
// ---------------------------------------------------------------------------

/// A raw source pointer used by a deref store is not reported unused.
#[test]
fn should_not_warn_for_a_raw_source_pointer() {
    let source = r#"module spec.ze.raw_source;

func main(): i32 {
    let a: i32 = 1;
    let b: i32 = 2;
    let q: raw i32 = unsafe &a;
    let r: raw i32 = unsafe &b;
    unsafe { *q = *r; }
    return a - 2;
}
"#;
    let (_scratch, output, _c_path) = compile_with("raw_source", source, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("W10000"),
        "raw_source: the dereferenced pointer is used; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = run_strict_host("raw_source", source);
    assert_exit("raw_source", &output, 0);
}

// ---------------------------------------------------------------------------
// a rolled-back generated owner's display subject.
// ---------------------------------------------------------------------------

/// A re-taken parked owner id still renders the generated-owner phrase.
///
/// The deferred pass declares a generated owner, the exit-edge check rolls it
/// back, and a later local takes its id. The escape report must name the
/// compiler-generated owner, never the later binding.
#[test]
fn should_render_the_parked_generated_owner_phrase() {
    let source = r#"module spec.ze.parked_owner;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(a: str, c: bool): str {
    defer {
        let l: Box = Box { v: stringOf("cd") };
        return (c ? l : mkBox()).v;
    }
    if (c) { return a; }
    let dummy: i32 = 0;
    let later: Box = Box { v: stringOf("ef") };
    return a;
}

func main(): i32 { return 0; }
"#;
    let (_scratch, output, c_path) = compile_with("parked_owner", source, &[]);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10030:").count(),
        1,
        "parked_owner: expected one E10030; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("borrow of a compiler-generated owner"),
        "parked_owner: the subject must be the generated phrase; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("borrow of 'later'"),
        "parked_owner: the later binding must not name the parked owner; stderr:\n{stderr}"
    );
    assert!(!c_path.exists());
}

// ---------------------------------------------------------------------------
// raw pointer `.ptr` and `.len`.
// ---------------------------------------------------------------------------

/// A member of a raw pointer's scalar pointee is an unknown member.
///
/// `raw u8` is a pointer, not a view; `.ptr` and `.len` have no member
/// surface, and accepting them emits a struct-member arrow on a pointer.
#[test]
fn should_reject_ptr_and_len_members_on_a_raw_scalar_pointer() {
    let ptr = r#"module spec.ze.raw_ptr_member;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let p: raw u8 = unsafe &a[0];
    let q: raw u8 = unsafe p.ptr;
    return i32(q[0]) - 1;
}
"#;
    assert_rejected(
        "raw_ptr_member",
        ptr,
        "E10212",
        1,
        &["'ptr' is not a member of 'raw u8'"],
    );

    let len = r#"module spec.ze.raw_len_member;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let p: raw u8 = unsafe &a[0];
    let n: usize = unsafe p.len;
    return i32(n) - 2;
}
"#;
    assert_rejected(
        "raw_len_member",
        len,
        "E10212",
        1,
        &["'len' is not a member of 'raw u8'"],
    );
}

// ---------------------------------------------------------------------------
// a compound-initializer field at a nullable handle.
// ---------------------------------------------------------------------------

/// A struct-literal field at `File?` takes the typed null value.
///
/// The store path lowers a null-shaped value to the destination's zero; the
/// compound-initializer path must do the same instead of initializing the
/// handle from a `void *`.
#[test]
fn should_lower_a_compound_initializer_field_to_the_typed_null() {
    let source = r#"module spec.ze.compound_null;

import { File } from std.io;

struct Box { f: File?; }

func main(): i32 {
    let n: File? = null;
    let b: Box = Box { f: n };
    return 0;
}
"#;
    let emitted = emit_host("compound_null", source);
    assert!(
        !emitted.contains("(ClBox){ n }"),
        "compound_null: the field must reach the typed-null lowering; C:\n{emitted}"
    );
    let output = run_strict_host("compound_null", source);
    assert_exit("compound_null", &output, 0);
}

// ---------------------------------------------------------------------------
// `sizeof` folds enums, handles, and views.
// ---------------------------------------------------------------------------

/// The constant `sizeof` fold covers enums, handles, and view types.
///
/// `sizeof(E)` is the backing scalar width, a struct with an enum field
/// measures that field, a handle measures its pointer, and an omitted-extent
/// array measures the view struct. Each fold is accepted and the value
/// positions agree at run time.
#[test]
fn should_fold_sizeof_for_enums_handles_and_views() {
    let source = r#"module spec.ze.sizeof_types;

import { String, stringOf } from std.text;

enum E : u8 { A, B }
struct S { e: E; n: i32; }

const enum_size: usize = sizeof(E);
const struct_size: usize = sizeof(S);
const handle_size: usize = sizeof(String);
const view_size: usize = sizeof(u8[]);

func main(): i32 {
    let e: usize = sizeof(E);
    let s: usize = sizeof(S);
    let h: usize = sizeof(String);
    let v: usize = sizeof(u8[]);
    let pad: u8[struct_size] = [; 0];
    if (i32(len(pad)) != i32(s)) { return 1; }
    return i32(enum_size - e) + i32(struct_size - s)
        + i32(handle_size - h) + i32(view_size - v);
}
"#;
    let output = run_strict_host("sizeof_types", source);
    assert_exit("sizeof_types", &output, 0);
}

// ---------------------------------------------------------------------------
// `offsetof` outside the acceptance checks.
// ---------------------------------------------------------------------------

/// `offsetof` emits the real padded offset and does not require a binding.
#[test]
fn should_emit_the_real_offsetof_value() {
    let source = r#"module spec.ze.offsetof_value;

struct S { a: i32; b: i32; }

const k: usize = offsetof(S, b);

func main(): i32 {
    let m: usize = offsetof(S, b);
    return i32(k - 4) + i32(m - 4);
}
"#;
    let emitted = emit_host("offsetof_value", source);
    assert!(
        emitted.contains("offsetof(ClS, b)"),
        "offsetof_value: the value position must use the real C offset; C:\n{emitted}"
    );
    let output = run_strict_host("offsetof_value", source);
    assert_exit("offsetof_value", &output, 0);
}

// ---------------------------------------------------------------------------
// an eager take deref argument keeps argument order.
// ---------------------------------------------------------------------------

/// An earlier argument evaluates before the take carrier's pointer fill.
///
/// `first()` reads the global index, the carrier's `nextp` increments it, and
/// the call's parameter must receive the pre-increment value. The take
/// dereference is an `unsafe` expression; the program builds and exits 0.
#[test]
fn should_keep_argument_order_before_a_take_deref_carrier() {
    let source = r#"module spec.ze.take_order;

import { String, stringOf } from std.text;

struct S { s: String; }

let idx: i32 = 0;

func nextp(p: raw S, q: raw S): raw S {
    unsafe {
        idx = idx + 1;
        if (idx == 1) { return p; }
        return q;
    }
}

func first(): i32 { return idx; }
func pair(n: i32, take x: S): i32 { return n; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    let b: S = S { s: stringOf("bb") };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let r: i32 = pair(first(), take (unsafe *nextp(p, q)));
        return r + (idx - 1) * 10;
    }
}
"#;
    let output = run_strict_host("take_order", source);
    assert_exit("take_order", &output, 0);
}

// ---------------------------------------------------------------------------
// a mixed all-fresh guarded read base.
// ---------------------------------------------------------------------------

/// A guarded all-fresh conditional read builds when both arms are structs.
#[test]
fn should_build_a_guarded_all_fresh_conditional_read() {
    let source = r#"module spec.ze.fresh_conditional;

struct Inner { row: i32[2][2]; }

func mkInner(): Inner { return Inner { row: [[1, 2], [3, 4]] }; }
func n2(): i32[2][2] { return [[5, 6], [7, 8]]; }

func main(): i32 {
    let g: bool = true;
    let ok: bool = true;
    let v: i32 = g ? (ok ? mkInner() : Inner { row: n2() }).row[0][0] : 0;
    return v - 1;
}
"#;
    let output = run_strict_host("fresh_conditional", source);
    assert_exit("fresh_conditional", &output, 0);
}

// ---------------------------------------------------------------------------
// null-shaped values in scalar casts.
// ---------------------------------------------------------------------------

/// A null-shaped value cannot be cast to an integer.
///
/// The shape is a `void *`; the cast would emit a pointer-to-integer
/// conversion the strict build rejects. The cast reports `E10201`.
#[test]
fn should_reject_a_null_shaped_value_in_an_integer_cast() {
    let int_cast = r#"module spec.ze.null_int_cast;

func main(): i32 {
    let x = null;
    return i32(x);
}
"#;
    assert_rejected(
        "null_int_cast",
        int_cast,
        "E10201",
        1,
        &["cannot cast 'null' to 'i32'"],
    );

    let size_cast = r#"module spec.ze.null_size_cast;

func main(): i32 {
    let x = null;
    let n: usize = usize(x);
    return i32(n);
}
"#;
    assert_rejected(
        "null_size_cast",
        size_cast,
        "E10201",
        1,
        &["cannot cast 'null' to 'usize'"],
    );
}

// ---------------------------------------------------------------------------
// Loop-condition split residuals.
// ---------------------------------------------------------------------------

/// A nested short-circuit right operand is not evaluated by the prefill.
#[test]
fn should_not_prefill_a_nested_short_circuit_right_operand() {
    let source = r#"module spec.ze.nested_short_circuit;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

let calls: i32 = 0;

func addOne(x: i32): i32 { return x + 1; }

func mkH(): H {
    calls = calls + 1;
    return H { call: addOne, s: stringOf("x") };
}

func truthy(): bool { return true; }

func run(g: bool): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < 1 && (g || mkH().call(5) > 0)) {
        n = n + 1;
        i = i + 1;
    }
    return n + calls * 10;
}

func main(): i32 {
    return run(true) - 1;
}
"#;
    let output = run_strict_host("nested_short_circuit", source);
    assert_exit("nested_short_circuit", &output, 0);
}

/// A fill that changes the condition cannot select the other arm.
#[test]
fn should_freeze_a_loop_condition_before_filling_an_arm() {
    let source = r#"module spec.ze.frozen_condition;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

let flag: bool = true;

func addTen(x: i32): i32 { return x + 10; }
func neg(x: i32): i32 { return 0 - x; }

func mkA(): H {
    flag = false;
    return H { call: addTen, s: stringOf("a") };
}

func use(take a: H, k: i32): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < k && (flag ? mkA() : a).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    let a: H = H { call: neg, s: stringOf("aa") };
    return use(a, 1) - 1;
}
"#;
    let output = run_strict_host("frozen_condition", source);
    assert_exit("frozen_condition", &output, 0);
}

/// A conditional initializer arm keeps its runtime guards.
#[test]
fn should_keep_a_conditional_initializer_arm_guard() {
    let source = r#"module spec.ze.conditional_init_guard;

struct A { arr: i32[2]; }

let d: i32 = 0;
let b: A = true ? A { arr: [12 / d, 2] } : A { arr: [1, 2] };

func main(): i32 {
    return b.arr[0];
}
"#;
    let output = run_strict_host("conditional_init_guard", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "conditional_init_guard: the zero divisor must trap"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("E11210"),
        "conditional_init_guard: the guard must print E11210; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// `offsetof` argument validation.
// ---------------------------------------------------------------------------

/// An unknown `offsetof` field is rejected before emission.
#[test]
fn should_reject_an_unknown_offsetof_field() {
    let source = r#"module spec.ze.offsetof_unknown;

struct S { a: i32; b: i32; }

func main(): i32 {
    let m: usize = offsetof(S, nope);
    return i32(m);
}
"#;
    assert_rejected(
        "offsetof_unknown",
        source,
        "E10212",
        1,
        &["'nope' is not a member of 'S'"],
    );
}

/// A non-name `offsetof` field is rejected before emission.
#[test]
fn should_reject_a_non_name_offsetof_field() {
    let source = r#"module spec.ze.offsetof_non_name;

struct S { a: i32; b: i32; }

func main(): i32 {
    let m: usize = offsetof(S, 1 + 2);
    return i32(m);
}
"#;
    assert_rejected(
        "offsetof_non_name",
        source,
        "E10212",
        1,
        &["offsetof requires a field name"],
    );
}
