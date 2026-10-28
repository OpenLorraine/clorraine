//! Runnable specification tests for the zero-defect critical/major batch.
//!
//! Each case compiles a small Clorraine program, inspects or builds the emitted
//! host C under the fixed strict warning set, and runs it where the behavior is
//! observable. Memory cases run under AddressSanitizer when `RDC_ASAN` is set.
//!
//! The batch pins the conditional-store and take integrity rows, the view
//! returns, and the initializer/evaluation-order rows.

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
            "clc-zerocrit-{}-{tag}-{sequence}",
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

/// Compiles one emitted host translation unit under the strict warning set.
fn build_strict_host(tag: &str, emitted: &str) {
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
}

/// Two takes of one saved destination consume different subranges; each
/// subrange is cleared at its own move-out, so the save release frees neither
/// consumed value again.
#[test]
fn should_clear_every_subrange_consumed_from_a_saved_destination() {
    let source = r#"module spec.zc.take545;

import { String, stringOf } from std.text;

struct Pair { first: String; sibling: String; }

func eat(take s: String): i32 { return i32(len(s)); }

func main(): i32 {
    let pair: Pair = Pair { first: stringOf("ab"), sibling: stringOf("cd") };
    let x: Pair = Pair { first: stringOf("ee"), sibling: stringOf("ff") };
    let y: Pair = Pair { first: stringOf("gg"), sibling: stringOf("hh") };
    let g: bool = true;
    pair = (g && eat(take pair.first) > 0 && eat(take pair.sibling) > 0) ? x : y;
    return i32(len(pair.first)) - 2;
}
"#;
    let emitted = emit_host("take545", source);
    let output = run_strict_host("take545", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "take545: both consumed subranges must be cleared before the save release; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    if std::env::var_os("RDC_ASAN").is_some() {
        let sanitized = run_sanitized_host("take545", &emitted);
        let stderr = String::from_utf8_lossy(&sanitized.stderr);
        assert_eq!(
            sanitized.status.code(),
            Some(0),
            "take545: the sanitized run must stay clean; stderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("LeakSanitizer") && !stderr.contains("AddressSanitizer"),
            "take545: the save must not free a consumed handle twice; stderr:\n{stderr}"
        );
    }
}

/// A fresh owned temporary stored into a borrowed view slot dies with the
/// frame and reports the concrete `E10030`.
#[test]
fn should_reject_a_statement_temporary_stored_into_a_borrowed_view_slot() {
    let source = r#"module spec.zc.slottemporary;

import { String, stringOf } from std.text;

func put<T>(borrow slot: T): void { slot = stringOf("ab"); }

func main(): i32 {
    let slot: str = "zz";
    put<str>(borrow slot);
    return i32(slot[0]) - 97;
}
"#;
    assert_rejected("slottemporary", source, "E10030", 1, &["statement temporary"]);
}

/// A foreign view stored into a borrowed view slot escapes into the caller's
/// slot; the generic instance reports the same `E10033` as the concrete twin.
#[test]
fn should_reject_a_foreign_view_stored_into_a_borrowed_view_slot() {
    let generic = r#"module spec.zc.slot550;

import { String, stringOf } from std.text;

func put<T>(c: bool, h: T, borrow slot: T): void {
    slot = c ? h : h;
}

func main(): i32 {
    let b: String = stringOf("aa");
    let h: str = b;
    let slot: str = "zz";
    put<str>(true, h, borrow slot);
    b = stringOf("bb");
    return i32(slot[0]) - 97;
}
"#;
    assert_rejected("slot550", generic, "E10033", 1, &["'slot'"]);
    let concrete = r#"module spec.zc.slot550c;

import { String, stringOf } from std.text;

func put(c: bool, h: str, borrow slot: str): void {
    slot = c ? h : h;
}

func main(): i32 {
    let b: String = stringOf("aa");
    let h: str = b;
    let slot: str = "zz";
    put(true, h, borrow slot);
    b = stringOf("bb");
    return i32(slot[0]) - 97;
}
"#;
    assert_rejected("slot550c", concrete, "E10033", 1, &["'slot'"]);
}

/// A type-parameter return becomes a view at its view instances and owes the
/// same one-candidate rule as the concrete twin; a single candidate stays
/// accepted.
#[test]
fn should_apply_the_one_candidate_rule_to_a_type_parameter_view_return() {
    let two = r#"module spec.zc.return556;

func f<T>(s: str, t: str): T { return t; }

func main(): i32 {
    let r: str = f<str>("ab", "cd");
    return i32(len(r)) - 2;
}
"#;
    assert_rejected("return556", two, "E10036", 1, &[]);
    let concrete = r#"module spec.zc.return556c;

func f(s: str, t: str): str { return t; }

func main(): i32 {
    let r: str = f("ab", "cd");
    return i32(len(r)) - 2;
}
"#;
    assert_rejected("return556c", concrete, "E10036", 1, &[]);
    let one = r#"module spec.zc.return556one;

func f<T>(s: str): T { return s; }

func main(): i32 {
    let r: str = f<str>("ab");
    return i32(len(r)) - 2;
}
"#;
    let emitted = emit_host("return556one", one);
    let output = run_strict_host("return556one", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "return556one: the single view candidate must stay accepted; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let static_return = r#"module spec.zc.return556static;

func f<T>(s: str): T { return "ab"; }

func main(): i32 {
    let r: str = f<str>("cd");
    return i32(len(r)) - 2;
}
"#;
    let emitted = emit_host("return556static", static_return);
    let output = run_strict_host("return556static", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "return556static: a static view return needs no candidate; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A literal element, a take argument, and a read materialization thread the
/// destination into the per-instance judgment, so a view instance copies a
/// concrete lend arm instead of reporting the owning-instance move.
#[test]
fn should_accept_view_instances_of_literal_argument_and_read_maybe_targets() {
    let element = r#"module spec.zc.maybe547element;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func mk<T>(c: bool, s: String, v: str): Wrap<T> {
    return Wrap<T> { items: [c ? v : s, v] };
}

func main(): i32 {
    let s: String = stringOf("ab");
    let w: Wrap<str> = mk<str>(true, s, "cd");
    return i32(len(w.items[0])) - 2;
}
"#;
    let emitted = emit_host("maybe547element", element);
    let output = run_strict_host("maybe547element", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "maybe547element: the view element must copy the lend arm; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let argument = r#"module spec.zc.maybe547argument;

import { String, stringOf } from std.text;

func eat<T>(take v: T): i32 { return i32(len(v)); }

func pick<T>(c: bool, s: String, v: str): i32 {
    return eat<T>(take (c ? v : s));
}

func main(): i32 {
    let s: String = stringOf("ab");
    return pick<str>(true, s, "cd") - 2;
}
"#;
    let emitted = emit_host("maybe547argument", argument);
    let output = run_strict_host("maybe547argument", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "maybe547argument: the view take must copy the lend arm; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let read = r#"module spec.zc.maybe547read;

import { String, stringOf } from std.text;

func pick<T>(c: bool, p: T[2], q: T[2]): i32 {
    let n: i32 = i32(len((c ? p : q)[0]));
    return n;
}

func main(): i32 {
    let p: str[2] = ["ab", "cd"];
    let q: str[2] = ["ef", "gh"];
    return pick<str>(true, p, q) - 2;
}
"#;
    let emitted = emit_host("maybe547read", read);
    let output = run_strict_host("maybe547read", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "maybe547read: the view read must copy the lend arm; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A raw out-slot store is duplicable and stays accepted; only a view slot
/// carries the escape duty.
#[test]
fn should_accept_a_raw_out_slot_store() {
    let source = r#"module spec.zc.rawslot;

func put<T>(p: T, borrow slot: T): void { slot = p; }

func main(): i32 {
    let a: i32 = 1;
    let p: raw i32 = unsafe &a;
    let q: raw i32 = unsafe &a;
    put<raw i32>(p, borrow q);
    return 0;
}
"#;
    let emitted = emit_host("rawslot", source);
    let output = run_strict_host("rawslot", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "rawslot: a raw out-slot store must stay accepted; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A pure dot receiver evaluates before a later argument's statement fill.
#[test]
fn should_pin_a_pure_dot_receiver_before_a_later_fill() {
    let source = r#"module spec.zc.receiverorder;

struct B { v: i32; }
struct Outer { xs: i32[2]; n: i32; }
func set(p: raw B): i32[2] { unsafe { (*p).v = 9; } return [1, 2]; }
func use(self: B, o: Outer): i32 { return self.v; }
func main(): i32 {
    let b: B = B { v: 5 };
    let p: raw B = unsafe &b;
    let r: i32 = b.use(Outer { xs: set(p), n: 2 });
    return r;
}
"#;
    let emitted = emit_host("receiverorder", source);
    let output = run_strict_host("receiverorder", &emitted);
    assert_eq!(
        output.status.code(),
        Some(5),
        "receiverorder: the receiver read must precede the later fill; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A maybe-owning return judges a concrete lend arm per instance: an owning
/// instance reports the lend move its concrete twin reports, while a view
/// instance copies the arm.
#[test]
fn should_judge_a_lend_arm_of_a_maybe_owning_return_per_instance() {
    let generic = r#"module spec.zc.return546;

import { String, stringOf } from std.text;

func pick<T>(c: bool, take h: T, s: String): T {
    return c ? h : s;
}

func main(): i32 {
    let s: String = stringOf("cd");
    let r: String = pick<String>(true, stringOf("ab"), s);
    return i32(len(r)) - 2;
}
"#;
    assert_rejected("return546", generic, "E10034", 1, &["'s'"]);
    let concrete = r#"module spec.zc.return546c;

import { String, stringOf } from std.text;

func pick(c: bool, take h: String, s: String): String {
    return c ? h : s;
}

func main(): i32 {
    let s: String = stringOf("cd");
    let r: String = pick(true, stringOf("ab"), s);
    return i32(len(r)) - 2;
}
"#;
    assert_rejected("return546c", concrete, "E10034", 1, &["'s'"]);
    let two_candidates = r#"module spec.zc.return546v;

import { String, stringOf } from std.text;

func pick<T>(c: bool, h: T, s: String): T {
    return c ? h : s;
}

func main(): i32 {
    let s: String = stringOf("cd");
    let r: str = pick<str>(true, "ab", s);
    return i32(len(r)) - 2;
}
"#;
    assert_rejected("return546v", two_candidates, "E10036", 1, &[]);
    let one = r#"module spec.zc.return546one;

func pick<T>(c: bool, h: T, n: i32): T {
    return c ? h : h;
}

func main(): i32 {
    let r: str = pick<str>(true, "ab", 1);
    return i32(len(r)) - 2;
}
"#;
    let emitted = emit_host("return546one", one);
    let output = run_strict_host("return546one", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "return546one: the single view candidate must stay accepted; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A view destination that reads one member of a stored initializer literal
/// releases the literal's residual sibling fields and keeps the read
/// member's storage for the view.
#[test]
fn should_release_the_residual_fields_of_a_stored_initializer_literal() {
    let source = r#"module spec.zc.leak553;

import { String, stringOf } from std.text;

struct S { xs: i32[2]; a: String; b: String; }

let g: str = S { xs: [1, 2], a: stringOf("ab"), b: stringOf("cd") }.a;

func main(): i32 {
    return i32(len(g)) - 2;
}
"#;
    let emitted = emit_host("leak553", source);
    assert!(
        emitted.contains("__cl_itmp1.a = NULL;")
            && emitted.contains("FreeString(__cl_itmp1.b)"),
        "leak553: the residual sibling must release and the read member must move out; emitted C:\n{emitted}"
    );
    let output = run_strict_host("leak553", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "leak553: the view must read the kept member; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    if std::env::var_os("RDC_ASAN").is_some() {
        let sanitized = run_sanitized_host("leak553", &emitted);
        let stderr = String::from_utf8_lossy(&sanitized.stderr);
        assert_eq!(
            sanitized.status.code(),
            Some(0),
            "leak553: the sanitized run must stay clean; stderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("LeakSanitizer") && !stderr.contains("AddressSanitizer"),
            "leak553: the residual sibling must not leak and the view backing must stay alive; stderr:\n{stderr}"
        );
    }
}

/// A literal member widened into a view stored in another aggregate keeps its
/// backing alive; only the resolved read root is released.
#[test]
fn should_not_free_a_view_backing_widened_from_a_literal_member() {
    let source = r#"module spec.zc.viewmember;

import { String, stringOf } from std.text;

struct A { xs: i32[2]; a: String; }
struct S { items: str[2]; tag: String; }

let g: str = S { items: [A { xs: [1, 2], a: stringOf("ab") }.a, "zz"],
                  tag: stringOf("tt") }.items[0];

func main(): i32 { return i32(g[0]) - 97; }
"#;
    let emitted = emit_host("viewmember", source);
    let output = run_strict_host("viewmember", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "viewmember: the widened literal member must stay alive; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    if std::env::var_os("RDC_ASAN").is_some() {
        let sanitized = run_sanitized_host("viewmember", &emitted);
        let stderr = String::from_utf8_lossy(&sanitized.stderr);
        assert_eq!(
            sanitized.status.code(),
            Some(0),
            "viewmember: the sanitized run must stay clean; stderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("LeakSanitizer") && !stderr.contains("AddressSanitizer"),
            "viewmember: the view backing must not be freed; stderr:\n{stderr}"
        );
    }
}

/// A conditional initializer arm that reads through a resolved inner local
/// reads the member the move-out empties, not a same-named nested member.
#[test]
fn should_read_the_resolved_member_on_a_conditional_initializer_arm() {
    let source = r#"module spec.zc.condnested;

import { String, stringOf } from std.text;

struct C { c: String; }
struct B { ys: i32[2]; inner: C; c: String; }
struct A { inner: B; }
func yes(): bool { return true; }

let g: str = yes() ? A { inner: B { ys: [1, 2], inner: C { c: stringOf("zz") }, c: stringOf("ab") } }.inner.c
                   : A { inner: B { ys: [3, 4], inner: C { c: stringOf("yy") }, c: stringOf("cd") } }.inner.c;

func main(): i32 { return i32(g[0]) - 97; }
"#;
    let emitted = emit_host("condnested", source);
    let output = run_strict_host("condnested", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "condnested: the conditional arm must read the resolved member; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    if std::env::var_os("RDC_ASAN").is_some() {
        let sanitized = run_sanitized_host("condnested", &emitted);
        let stderr = String::from_utf8_lossy(&sanitized.stderr);
        assert_eq!(
            sanitized.status.code(),
            Some(0),
            "condnested: the sanitized run must stay clean; stderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("LeakSanitizer") && !stderr.contains("AddressSanitizer"),
            "condnested: the read member must match the move-out; stderr:\n{stderr}"
        );
    }
}

/// A containing literal whose field is a store-needing inner local keeps its
/// other field initializers, including their effects, when a view reads a
/// member through the inner local.
#[test]
fn should_keep_a_containing_literal_s_other_field_effects() {
    let source = r#"module spec.zc.containingliteral;

import { String, stringOf } from std.text;

struct Inner { xs: i32[2]; name: String; }
struct Outer { inner: Inner; other: i32; extra: i32; }

let order: i32 = 0;
func h(): i32 { order = order * 10 + 2; return 7; }
func k(): i32 { order = order * 10 + 3; return 8; }

let g: str = Outer { inner: Inner { xs: [1, 2], name: stringOf("ab") }, other: h(), extra: k() }.inner.name;

func main(): i32 { return order - 23; }
"#;
    let emitted = emit_host("containingliteral", source);
    let output = run_strict_host("containingliteral", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "containingliteral: both sibling effects must run; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A module initializer with an impure fixed-array field followed by a
/// statement-filled field fills the array element-wise at its source
/// position, so the emitted C builds and the effects keep their order.
#[test]
fn should_fill_an_initializer_fixed_array_field_before_a_later_field() {
    let nested = r#"module spec.zc.initarrayorder;

struct Inner { ys: i32[2]; }
struct S { xs: i32[2]; inner: Inner; }

let c: bool = true;
func f(): i32 { return 1; }
let s: S = S { xs: c ? [f(), 2] : [3, 4], inner: Inner { ys: [1, 2] } };

func main(): i32 { return s.xs[0] + s.inner.ys[1] - 3; }
"#;
    let emitted = emit_host("initarrayorder", nested);
    let output = run_strict_host("initarrayorder", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "initarrayorder: the fixed-array field must fill without a C array assignment; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let order = r#"module spec.zc.initarraytrace;

struct S { xs: i32[2]; other: i32; }

let order: i32 = 0;
func f(): i32 { order = order * 10 + 1; return 1; }
func g(): i32 { order = order * 10 + 2; return 7; }

let c: bool = true;
let s: S = S { xs: c ? [f(), 2] : [3, 4], other: g() };

func main(): i32 { return order - 12 + s.xs[0] - 1 + s.other - 7; }
"#;
    let emitted = emit_host("initarraytrace", order);
    let output = run_strict_host("initarraytrace", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "initarraytrace: the fixed-array field's effect must precede the later field's; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A module initializer stores an effectful field at its source position, so
/// a later field's statement fill cannot hoist above it.
#[test]
fn should_evaluate_initializer_fields_in_source_order() {
    let source = r#"module spec.zc.order555;

struct Inner { xs: i32[2]; }
struct Outer { a: i32; b: Inner; }

let order: i32 = 0;
func f(): i32 { order = order * 10 + 1; return 1; }
func mkarr(): i32[2] { order = order * 10 + 2; return [7, 8]; }

let o: Outer = Outer { a: f(), b: Inner { xs: mkarr() } };

func main(): i32 {
    return order - 12;
}
"#;
    let emitted = emit_host("order555", source);
    let output = run_strict_host("order555", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "order555: the initializer fields must run in source order; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A later argument whose literal has no C value expression is filled at its
/// own position, after the effects of every earlier argument.
#[test]
fn should_evaluate_earlier_arguments_before_a_later_literal_fill() {
    let source = r#"module spec.zc.order552;

let order: i32 = 0;

struct Outer { xs: i32[2]; n: i32; }

func a(): i32 { order = order * 10 + 1; return 1; }
func mk(): i32 { order = order * 10 + 2; return 7; }
func useTwo(x: i32, o: Outer): i32 { order = order * 10 + 3; return x + o.n; }

func main(): i32 {
    let r: i32 = useTwo(a(), Outer { xs: [mk(), 8], n: 2 });
    return order - 123 + r - 3;
}
"#;
    let emitted = emit_host("order552", source);
    let output = run_strict_host("order552", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "order552: the arguments must run in source order (got order {}); stderr:\n{}",
        0,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A fill expression evaluates once, not once per covered element, including
/// a nested-row fill that evaluates once per row.
#[test]
fn should_evaluate_an_array_fill_once() {
    let statement = r#"module spec.zc.fill551;

let calls: i32 = 0;

func mk(): i32 {
    calls = calls + 1;
    return 1;
}

func main(): i32 {
    let xs: i32[3] = [; mk()];
    return xs[0] + calls * 10;
}
"#;
    let emitted = emit_host("fill551", statement);
    let output = run_strict_host("fill551", &emitted);
    assert_eq!(
        output.status.code(),
        Some(11),
        "fill551: the statement fill must run once; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let nested = r#"module spec.zc.fill551b;

let calls: i32 = 0;

func mk(): i32 {
    calls = calls + 1;
    return 1;
}

func main(): i32 {
    let m: i32[2][3] = [; [; mk()]];
    return m[0][0] + calls * 10;
}
"#;
    let emitted = emit_host("fill551b", nested);
    let output = run_strict_host("fill551b", &emitted);
    assert_eq!(
        output.status.code(),
        Some(11),
        "fill551b: the nested-row fill expression must run once for the whole fill; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts the front end rejected `source` with the given code and count.
fn assert_rejected(tag: &str, source: &str, code: &str, expected: usize, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        expected,
        "{tag}: expected {expected} error(s); stderr:\n{stderr}"
    );
    assert!(
        errors[0].contains(code),
        "{tag}: expected the {code} report; saw `{}`; stderr:\n{stderr}",
        errors[0]
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

/// A plain (borrow) parameter selected by a conditional may not be moved into
/// an owned raw-deref destination; the store reports the lend rule once.
#[test]
fn should_reject_a_borrowed_parameter_moved_into_a_raw_destination() {
    let source = r#"module spec.zc.store544;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(c: bool, p: String, q: String): i32 {
    let b: Box = Box { v: stringOf("zz") };
    let h: raw Box = unsafe &b;
    unsafe { (*h).v = c ? p : q; }
    return i32(len(b.v)) - 2;
}

func main(): i32 {
    return pick(true, stringOf("ab"), stringOf("cd"));
}
"#;
    assert_rejected("store544", source, "E10034", 1, &["p"]);
}

/// Builds and runs one emitted host translation unit under the strict set.
fn run_strict_host(tag: &str, emitted: &str) -> Output {
    let scratch = ScratchDir::new(&format!("{tag}-run"));
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
    Command::new(&program)
        .output()
        .expect("failed to run the host program")
}

/// Builds and runs one emitted host translation unit under AddressSanitizer
/// with leak detection.
fn run_sanitized_host(tag: &str, emitted: &str) -> Output {
    let scratch = ScratchDir::new(&format!("{tag}-asan"));
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-fsanitize=address",
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
        "{tag}: the sanitized host build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new(&program)
        .env("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")
        .output()
        .expect("failed to run the sanitized host program")
}

/// Only an integer literal zero divisor is a guaranteed fault. A cast of a
/// literal zero and `%` trap; `~0` and a float zero divide normally.
#[test]
fn should_trap_only_an_integer_literal_zero_divisor() {
    let cast = r#"module spec.zc.zerocast;

func main(): i32 {
    let a: i32 = 8;
    let r: i32 = a / i32(0);
    return r;
}
"#;
    let emitted = emit_host("zerocast", cast);
    assert!(
        emitted.contains("cl_trap(\"E11210\""),
        "zerocast: an integer cast zero must trap; emitted C:\n{emitted}"
    );
    let built = run_strict_host("zerocast", &emitted);
    assert!(
        !built.status.success(),
        "zerocast: the trap must abort; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(
        String::from_utf8_lossy(&built.stderr).contains("E11210"),
        "zerocast: the runtime must name the trap; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let bitnot = r#"module spec.zc.zerobitnot;

func main(): i32 {
    let a: i32 = 8;
    let r: i32 = a / ~0;
    return r + 8;
}
"#;
    let emitted = emit_host("zerobitnot", bitnot);
    let output = run_strict_host("zerobitnot", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "zerobitnot: the division must run normally; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let float = r#"module spec.zc.zerofloat;

func main(): i32 {
    let a: f64 = 1.0;
    let r: f64 = a / f64(0);
    if (r > 1000.0) { return 1; }
    return 0;
}
"#;
    let emitted = emit_host("zerofloat", float);
    assert!(
        !emitted.contains("E11210"),
        "zerofloat: a float zero divide is Inf/NaN, not a trap; emitted C:\n{emitted}"
    );
    let output = run_strict_host("zerofloat", &emitted);
    assert_eq!(
        output.status.code(),
        Some(1),
        "zerofloat: the division must produce Inf; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let narrowed = r#"module spec.zc.zeronarrow;

func main(): i32 {
    let x: u8 = 4;
    let r: u8 = x / u8(256);
    return i32(r);
}
"#;
    let emitted = emit_host("zeronarrow", narrowed);
    assert!(
        emitted.contains("cl_trap(\"E11210\""),
        "zeronarrow: a narrowing cast of 256 to u8 is zero; emitted C:\n{emitted}"
    );
    let built = run_strict_host("zeronarrow", &emitted);
    assert!(
        !built.status.success(),
        "zeronarrow: the trap must abort; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let float_cast = r#"module spec.zc.zerofloatcast;

func main(): i32 {
    let y: i32 = 4;
    let r: i32 = y / i32(0.0);
    return r;
}
"#;
    let emitted = emit_host("zerofloatcast", float_cast);
    assert!(
        emitted.contains("cl_trap(\"E11210\""),
        "zerofloatcast: an integer cast of float zero must trap; emitted C:\n{emitted}"
    );

    let rem = r#"module spec.zc.zerorem;

func main(): i32 {
    let a: i32 = 8;
    let r: i32 = a % i32(0);
    return r;
}
"#;
    let emitted = emit_host("zerorem", rem);
    assert!(
        emitted.contains("cl_trap(\"E11210\""),
        "zerorem: an integer cast zero remainder must trap; emitted C:\n{emitted}"
    );
}

/// A runtime-indexed store of a struct literal with a fixed-array call field
/// builds the literal in a temporary so the call receives its hidden
/// destination, then stores and empties the temporary.
#[test]
fn should_store_a_runtime_indexed_literal_with_a_fixed_array_call_field() {
    let source = r#"module spec.zc.store548;

struct S { xs: i32[2]; }

func mkArr(): i32[2] { return [7, 8]; }

func main(): i32 {
    let i: usize = 1;
    let arr: S[2] = [S { xs: [1, 2] }, S { xs: [3, 4] }];
    arr[i] = S { xs: mkArr() };
    return arr[1].xs[0] - 7;
}
"#;
    let emitted = emit_host("store548", source);
    let output = run_strict_host("store548", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "store548: the stored literal must hold the call result; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A literal zero divisor in a non-constant division is a runtime fault, and
/// the emitted C must not contain the bare division the strict build rejects.
#[test]
fn should_trap_a_literal_zero_divisor_without_emitting_the_division() {
    let source = r#"module spec.zc.zero549;

struct H { n: i32; }

func main(): i32 {
    let h: H = H { n: 4 };
    let p: raw H = unsafe &h;
    let r: i32 = (unsafe (*p).n) / 0;
    return r;
}
"#;
    let emitted = emit_host("zero549", source);
    assert!(
        emitted.contains("E11210"),
        "zero549: the emitted C must carry the integer-division trap"
    );
    assert!(
        !emitted.contains("/ 0"),
        "zero549: the emitted C must not contain a bare division by zero"
    );
    build_strict_host("zero549", &emitted);
}
