//! Implementation tests for the store-emission integrity batch.
//!
//! These cases pin the observable internals of the conditional-store
//! judgment through a lend: the exact diagnostic sequence and message name per
//! shape, the per-instance duty that reports one owning instance at most
//! whatever the instantiation order, and the byte-identical re-emission of the
//! accepted shapes. They complement the specification suite, which pins the
//! accepted and rejected program behavior.
//!
//! The immutable-view group pins the write rule's observable internals: the
//! exact single diagnostic and its message name per write form (parameter,
//! local, field path, compound, update, module-level), and the accepted
//! controls' emission — the whole-view rebind stores the new view, and the
//! writable slice stores through its pointer — byte-identical across runs.
//!
//! The typedef-order group pins the emitted declaration order itself: each
//! typedef's byte position precedes every use of it in the same unit, and
//! re-emitting the reordered programs is byte-identical.
//!
//! The module-initializer group pins the init-function shapes: the array fill
//! statement order, the single materialized store of an impure operand, the
//! one-store condition of a module conditional, the constructed view in place
//! of the bare intrinsic name, the folded dot-`len`, and byte-identical
//! re-emission.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A slice-of-struct program whose emitted declaration order is pinned.
const ORDER_STRUCT_SLICE: &str = r#"module spec.sei.impl.orderstructslice;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func main(): i32 {
    let arr: P[2] = [P { x: 1, y: 2 }, P { x: 3, y: 4 }];
    let ps: P[] = slice(arr, 0, 2);
    ps[0].x = 5;
    return ps[0].x + ps[1].y;
}
"#;

/// A slice-of-enum program whose emitted declaration order is pinned.
const ORDER_ENUM_SLICE: &str = r#"module spec.sei.impl.orderenumslice;

import { slice } from std.mem;

enum E : u8 {
    A = 1,
    B = 2,
}

func main(): i32 {
    let arr: E[2] = [E.A, E.B];
    let es: E[] = slice(arr, 0, 2);
    return i32(es[1]);
}
"#;

/// A struct-with-enum-field program whose emitted declaration order is pinned.
const ORDER_STRUCT_ENUM: &str = r#"module spec.sei.impl.orderstructenum;

enum E : u8 {
    A = 1,
    B = 2,
}

struct S {
    e: E;
    n: i32;
}

func main(): i32 {
    let v: S = S { e: E.B, n: 4 };
    return i32(v.e) + v.n;
}
"#;

/// A closure-capturing-a-slice program whose emitted declaration order is
/// pinned.
const ORDER_CLOSURE_CAPTURE: &str = r#"module spec.sei.impl.orderclosure;

import { slice } from std.mem;

func main(): i32 {
    let a: i32[4] = [1, 2, 3, 4];
    let s0: i32[] = slice(a, 0, 2);
    return ([s0] () => i32(s0[1]))();
}
"#;

/// A filled module array whose init statement order is pinned.
const INIT_FILL: &str = r#"module spec.sei.impl.initfill;

let a: i32[3] = [1, 2; 7];

func main(): i32 {
    return a[2];
}
"#;

/// A module view whose impure offset must be stored exactly once.
const INIT_OPERAND_ONCE: &str = r#"module spec.sei.impl.initonce;

import { slice } from std.mem;

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 1;
}

func run(): i32 {
    let backing: i32[3] = [1, 2, 3];
    let v: i32[] = slice(backing, bump(), 3);
    return v[0];
}

func main(): i32 {
    let r: i32 = run();
    if (calls != 1) {
        return 9;
    }
    return r;
}
"#;

/// A module conditional whose condition must be stored exactly once.
const INIT_CONDITIONAL: &str = r#"module spec.sei.impl.initconditional;

let x: i32[2] = [10, 11];
let y: i32[2] = [20, 21];
let calls: i32 = 0;

func pick(): bool {
    calls = calls + 1;
    return true;
}

let a: i32[2] = pick() ? x : y;

func main(): i32 {
    if (calls != 1) {
        return 9;
    }
    return a[1];
}
"#;

/// A module array and its slice view, for the intrinsic-rendering pin.
const INIT_VIEW: &str = r#"module spec.sei.impl.initview;

import { slice } from std.mem;

func run(): i32 {
    let backing: i32[2] = [1, 2];
    let view: i32[] = slice(backing, 0, 2);
    return i32(len(view));
}

func main(): i32 {
    return run();
}
"#;

/// A module `len` of a fixed array, for the folded-count pin.
const INIT_LEN: &str = r#"module spec.sei.impl.initlen;

let backing: i32[3] = [1, 2, 3];
let n: usize = backing.len();

func main(): i32 {
    return i32(n);
}
"#;

/// The byte offset of `needle` in `unit`, failing with the unit when absent.
fn position_of(unit: &str, needle: &str) -> usize {
    unit.find(needle)
        .unwrap_or_else(|| panic!("the emitted unit has no {needle:?}:\n{unit}"))
}

/// How many times `needle` occurs in `unit`.
fn count_of(unit: &str, needle: &str) -> usize {
    unit.matches(needle).count()
}

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
            "clc-storeemitimpl-{}-{tag}-{sequence}",
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

/// Emits the same program twice into one directory and returns both units.
///
/// Both emissions share the source path, so their `#line` directives match and
/// a difference between the two strings is a difference in the translation
/// itself, not in the scratch directory name.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
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
        emitted.push(std::fs::read_to_string(&c_path).unwrap_or_default());
    }
    (emitted[0].clone(), emitted[1].clone())
}

/// The `error E…:` codes the compiler reported on stderr, in order.
fn error_codes(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("error ")?;
            let code = rest.split(':').next()?;
            code.starts_with('E').then(|| code.to_owned())
        })
        .collect()
}

/// Asserts the program was rejected with exactly the given error codes, that
/// the message contains every fragment, and that no translation unit exists.
fn assert_rejected(tag: &str, source: &str, expected: &[&str], fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let codes = error_codes(&output);
    assert_eq!(
        codes,
        expected,
        "{tag}: expected {expected:?}, saw {codes:?}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
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

/// Every rejected conditional-store shape reports exactly one code and names
/// the right binding.
///
/// The amended shapes (the concrete and generic conditional self-store) name
/// the target `slot`; the unchanged shapes keep their existing naming — the
/// target for a conditional with no lend arm and for the direct self-store,
/// the source for the direct and conditional read-only lends. A second code
/// anywhere would mean the site is double-reported; a wrong name would mean
/// the wrong side is judged.
#[test]
fn should_report_one_named_error_per_conditional_store_shape() {
    assert_rejected(
        "concrete-conditional-self",
        r#"module spec.sei.impl.concrete;

import { String, stringOf } from std.text;

func st(borrow slot: String, pick: bool): void {
    let other: String = stringOf("bbbb");
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    st(borrow s, false);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "generic-conditional-self",
        r#"module spec.sei.impl.generic;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    let o: String = stringOf("bbbb");
    st<String>(borrow s, false, take o);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "conditional-two-takes",
        r#"module spec.sei.impl.twotakes;

import { String, stringOf } from std.text;

func f(borrow slot: String, take a: String, take b: String, pick: bool): void {
    slot = pick ? a : b;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    f(borrow s, stringOf("bbbb"), stringOf("cccc"), true);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "direct-self",
        r#"module spec.sei.impl.directself;

import { String, stringOf } from std.text;

func copySelf(borrow slot: String): void {
    slot = slot;
}

func main(): i32 {
    let s: String = stringOf("abc");
    copySelf(borrow s);
    return 0;
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "read-only-source",
        r#"module spec.sei.impl.readonlysource;

import { String, stringOf } from std.text;

func f(borrow slot: String, other: String): void {
    slot = other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    let o: String = stringOf("bbbb");
    f(borrow s, o);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'other'"],
    );
    assert_rejected(
        "conditional-read-only-arm",
        r#"module spec.sei.impl.readonlyarm;

import { String, stringOf } from std.text;

func g<T>(borrow slot: T, other: T, c: bool): void {
    slot = c ? slot : other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    let o: String = stringOf("bbbb");
    g<String>(borrow s, o, false);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'other'"],
    );
}

/// One conditional store site reports once whatever the instantiation order.
///
/// The site is recorded once and the per-instance audit reports at most the
/// first owning instance; a nullable instance is accepted by the out-slot
/// carve-out. With the nullable instance first and the owning one second the
/// site must still report exactly once, and the owning-first order must not
/// change the count — a second report would mean the duty repeats, and a
/// missing one would mean the nullable instance suppressed the owning one.
#[test]
fn should_report_one_error_per_conditional_site_across_instances() {
    assert_rejected(
        "instances-nullable-first",
        r#"module spec.sei.impl.nullablefirst;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let b: String? = stringOf("cd");
    let ob: String? = stringOf("ee");
    st<String?>(borrow b, false, take ob);
    let a: String = stringOf("ab");
    let oa: String = stringOf("ff");
    st<String>(borrow a, false, take oa);
    return 0;
}
"#,
        &["E10034"],
        &["'slot'"],
    );
    assert_rejected(
        "instances-owning-first",
        r#"module spec.sei.impl.owningfirst;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let a: String = stringOf("ab");
    let oa: String = stringOf("ff");
    st<String>(borrow a, false, take oa);
    let b: String? = stringOf("cd");
    let ob: String? = stringOf("ee");
    st<String?>(borrow b, false, take ob);
    return 0;
}
"#,
        &["E10034"],
        &["'slot'"],
    );
}

/// The accepted shapes keep their emission, byte-identical across runs.
///
/// The duplicable instance compiles the conditional into one direct store of
/// the selected value, and the nullable shapes keep their save-and-null
/// conditional emission. Re-emitting each accepted program proves the
/// judgment change did not disturb the translation.
#[test]
fn should_keep_the_accepted_shapes_emission_byte_identical() {
    let duplicable = r#"module spec.sei.impl.duplicable;

func st<T>(borrow slot: T, pick: bool, take other: T): T {
    slot = pick ? slot : other;
    return slot;
}

func main(): i32 {
    let x: i32 = 5;
    return st<i32>(borrow x, false, 7);
}
"#;
    let (first, second) = emit_twice("reemission-duplicable", duplicable);
    assert_eq!(
        first, second,
        "reemission-duplicable: the two emissions differ"
    );
    assert!(
        first.contains("slot = (pick ? slot : other);"),
        "reemission-duplicable: the conditional store is not emitted as one direct store"
    );

    let nullable_out_slot = r#"module spec.sei.impl.nullableoutslot;

import { String, stringOf } from std.text;

func g(borrow slot: String?, pick: bool): void {
    let other: String? = stringOf("bbbb");
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String? = stringOf("aaaa");
    g(borrow s, true);
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}
"#;
    let (first, second) = emit_twice("reemission-nullable-out-slot", nullable_out_slot);
    assert_eq!(
        first, second,
        "reemission-nullable-out-slot: the two emissions differ"
    );

    let nullable_generic = r#"module spec.sei.impl.nullablegeneric;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String? = stringOf("aaaa");
    let o: String? = stringOf("bbbb");
    st<String?>(borrow s, true, take o);
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}
"#;
    let (first, second) = emit_twice("reemission-nullable-generic", nullable_generic);
    assert_eq!(
        first, second,
        "reemission-nullable-generic: the two emissions differ"
    );
}

/// Every rejected immutable-view write form reports exactly one named code.
///
/// The plain parameter, local, compound, and update forms name the binding;
/// the field form names the field path; the module-level form names the
/// global. A second code anywhere would mean the write is double-reported —
/// for example by both the immutable-view rule and the read-only rule — and a
/// wrong name would mean the wrong part of the place is judged. No rejected
/// form may produce a translation unit.
#[test]
fn should_report_one_named_error_per_immutable_view_write_form() {
    assert_rejected(
        "immutable-param",
        r#"module spec.sei.impl.ivparam;

func f(borrow s: str): i32 {
    s[0] = 65;
    return 0;
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#,
        &["E10042"],
        &["cannot write through immutable view 's'"],
    );
    assert_rejected(
        "immutable-local",
        r#"module spec.sei.impl.ivlocal;

func main(): i32 {
    let t: str = "abc";
    t[0] = 65;
    return 0;
}
"#,
        &["E10042"],
        &["cannot write through immutable view 't'"],
    );
    assert_rejected(
        "immutable-field",
        r#"module spec.sei.impl.ivfield;

struct S {
    tag: static str;
    n: i32;
}

func f(borrow s: S): void {
    s.tag[0] = 65;
}

func main(): i32 {
    let v: S = S { tag: "abc", n: 1 };
    f(borrow v);
    return 0;
}
"#,
        &["E10042"],
        &["cannot write through immutable view 's.tag'"],
    );
    assert_rejected(
        "immutable-compound",
        r#"module spec.sei.impl.ivcompound;

func f(borrow s: str): i32 {
    s[0] += 1;
    return 0;
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#,
        &["E10042"],
        &["cannot write through immutable view 's'"],
    );
    assert_rejected(
        "immutable-update",
        r#"module spec.sei.impl.ivupdate;

func f(borrow s: str): i32 {
    s[0]++;
    return 0;
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#,
        &["E10042"],
        &["cannot write through immutable view 's'"],
    );
    assert_rejected(
        "immutable-global",
        r#"module spec.sei.impl.ivglobal;

let g: str = "abc";

func main(): i32 {
    g[0] = 65;
    return 0;
}
"#,
        &["E10042"],
        &["cannot write through immutable view 'g'"],
    );
    assert_rejected(
        "immutable-value-update",
        r#"module spec.sei.impl.ivvalueupdate;

func f(borrow s: str): i32 {
    return i32(s[0]++);
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#,
        &["E10042"],
        &["cannot write through immutable view 's'"],
    );
    assert_rejected(
        "immutable-raw-field",
        r#"module spec.sei.impl.ivrawfield;

struct S {
    tag: static str;
    n: i32;
}

func f(borrow p: raw S): void {
    unsafe {
        p.tag[0] = 65;
    }
}

func main(): i32 {
    let v: S = S { tag: "abc", n: 1 };
    unsafe {
        f(borrow &v);
    }
    return 0;
}
"#,
        &["E10042"],
        &["cannot write through immutable view 'p.tag'"],
    );
}

/// The accepted immutable-view controls keep their emission, byte-identical
/// across runs.
///
/// The whole-view rebind compiles to a direct assignment of the new view, and
/// the writable slice write compiles to a store through the view's pointer.
/// Re-emitting each program proves the reject rule did not disturb the
/// accepted translations.
#[test]
fn should_keep_the_accepted_immutable_view_controls_emission_byte_identical() {
    let rebind = r#"module spec.sei.impl.ivrebind;

func main(): i32 {
    let t: str = "abc";
    t = "xyz";
    return i32(len(t));
}
"#;
    let (first, second) = emit_twice("reemission-str-rebind", rebind);
    assert_eq!(
        first, second,
        "reemission-str-rebind: the two emissions differ"
    );
    assert!(
        first.contains("t = cl_str(\"xyz\");"),
        "reemission-str-rebind: the rebind is not emitted as a view assignment"
    );

    let writable_slice = r#"module spec.sei.impl.ivwritable;

import { slice } from std.mem;

func f(borrow xs: u8[]): i32 {
    xs[0] = 65;
    return 0;
}

func main(): i32 {
    let a: u8[2] = [0, 0];
    return f(borrow slice(a, 0, 2));
}
"#;
    let (first, second) = emit_twice("reemission-writable-slice", writable_slice);
    assert_eq!(
        first, second,
        "reemission-writable-slice: the two emissions differ"
    );
    assert!(
        first.contains("(xs).ptr[0] = 65u;"),
        "reemission-writable-slice: the element store is not emitted through the view pointer"
    );
}

/// Every emitted typedef precedes its uses in the translation unit.
///
/// A slice of a user struct needs the struct's forward typedef before the
/// slice typedef and the definition after it; an enum typedef precedes the
/// slice typedef and the struct definition that name it; and the closure
/// environment that embeds a slice follows the slice typedef. Each assertion
/// compares byte offsets in one emitted unit, so a reordered phase fails with
/// the exact pair.
#[test]
fn should_emit_each_typedef_before_its_uses() {
    let (unit, _) = emit_twice("order-slice-of-struct", ORDER_STRUCT_SLICE);
    let forward = position_of(&unit, "typedef struct ClP ClP;");
    let slice = position_of(&unit, "} ClSlice_P;");
    let definition = position_of(&unit, "struct ClP {");
    assert!(
        forward < slice,
        "order-slice-of-struct: the slice typedef precedes the struct forward typedef"
    );
    assert!(
        slice < definition,
        "order-slice-of-struct: the struct definition precedes the slice typedef"
    );

    let (unit, _) = emit_twice("order-slice-of-enum", ORDER_ENUM_SLICE);
    let enum_typedef = position_of(&unit, "typedef u8 ClE;");
    let slice = position_of(&unit, "} ClSlice_E;");
    assert!(
        enum_typedef < slice,
        "order-slice-of-enum: the slice typedef precedes the enum typedef"
    );

    let (unit, _) = emit_twice("order-struct-with-enum", ORDER_STRUCT_ENUM);
    let enum_typedef = position_of(&unit, "typedef u8 ClE;");
    let definition = position_of(&unit, "struct ClS {");
    assert!(
        enum_typedef < definition,
        "order-struct-with-enum: the struct definition precedes the enum typedef"
    );

    let (unit, _) = emit_twice("order-closure-capture", ORDER_CLOSURE_CAPTURE);
    let slice = position_of(&unit, "} ClSlice_i32;");
    let environment = position_of(&unit, "struct ClEnv_0 {");
    assert!(
        slice < environment,
        "order-closure-capture: the closure environment precedes the slice typedef"
    );
}

/// Re-emitting the reordered programs is byte-identical.
///
/// The ordering change must be a pure declaration-order change: two emissions
/// of the same source, sharing the source path so their `#line` directives
/// match, must produce the same unit.
#[test]
fn should_keep_the_reordered_emission_byte_identical() {
    for (tag, source) in [
        ("reemission-order-struct-slice", ORDER_STRUCT_SLICE),
        ("reemission-order-enum-slice", ORDER_ENUM_SLICE),
        ("reemission-order-struct-enum", ORDER_STRUCT_ENUM),
        ("reemission-order-closure", ORDER_CLOSURE_CAPTURE),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(first, second, "{tag}: the two emissions differ");
    }
}

/// The module array fill stores elements in declaration order.
///
/// The literal's listed elements come first and the fill covers the tail; the
/// target must never take the invalid array assignment the old emission
/// produced.
#[test]
fn should_fill_module_arrays_in_declaration_order() {
    let (unit, _) = emit_twice("init-fill-order", INIT_FILL);
    let first = position_of(&unit, "cl_spec_sei_impl_initfill_a[0] = 1;");
    let second = position_of(&unit, "cl_spec_sei_impl_initfill_a[1] = 2;");
    let third = position_of(&unit, "cl_spec_sei_impl_initfill_a[2] = 7;");
    assert!(
        first < second,
        "init-fill-order: element 1 precedes element 0"
    );
    assert!(
        second < third,
        "init-fill-order: element 2 precedes element 1"
    );
    assert!(
        !unit.contains("cl_spec_sei_impl_initfill_a = {"),
        "init-fill-order: the target still takes an array assignment"
    );
}

/// An impure module intrinsic operand is stored once, before the view.
#[test]
fn should_materialize_an_impure_init_operand_once() {
    let (unit, _) = emit_twice("init-operand-once", INIT_OPERAND_ONCE);
    assert_eq!(
        count_of(&unit, "cl_spec_sei_impl_initonce_bump()"),
        1,
        "init-operand-once: the offset call does not run exactly once"
    );
    let store = position_of(&unit, "__cl_tmp0 = cl_spec_sei_impl_initonce_bump();");
    let view = position_of(&unit, "v = ((ClSlice_i32)");
    assert!(
        store < view,
        "init-operand-once: the view is constructed before the offset is stored"
    );
    assert!(
        unit[view..].contains("__cl_tmp0"),
        "init-operand-once: the construction does not read the stored offset"
    );
}

/// A module conditional's condition is stored once, before the elements.
#[test]
fn should_store_a_module_condition_once() {
    let (unit, _) = emit_twice("init-condition-once", INIT_CONDITIONAL);
    assert_eq!(
        count_of(&unit, "cl_spec_sei_impl_initconditional_pick()"),
        1,
        "init-condition-once: the condition does not run exactly once"
    );
    let store = position_of(
        &unit,
        "bool __cl_itmp0 = cl_spec_sei_impl_initconditional_pick();",
    );
    let first = position_of(&unit, "cl_spec_sei_impl_initconditional_a[0] = (__cl_itmp0 ? ");
    let second = position_of(&unit, "cl_spec_sei_impl_initconditional_a[1] = (__cl_itmp0 ? ");
    assert!(
        store < first && first < second,
        "init-condition-once: the element stores do not follow the stored condition in order"
    );
}

/// Module intrinsics render as constructed values in the init function.
///
/// The bare intrinsic name never reaches the unit: the `slice` construction
/// spells the view struct, and the dot-`len` of a fixed array folds to its
/// extent.
#[test]
fn should_render_module_intrinsics_in_the_init_function() {
    let (unit, _) = emit_twice("init-intrinsic-view", INIT_VIEW);
    assert_eq!(
        count_of(&unit, "slice("),
        0,
        "init-intrinsic-view: the bare intrinsic name is still emitted"
    );
    assert!(
        unit.contains("((ClSlice_i32)"),
        "init-intrinsic-view: the view is not constructed"
    );
    assert!(
        unit.contains("backing[0] = 1;")
            && unit.contains("backing[1] = 2;"),
        "init-intrinsic-view: the source array is not filled"
    );

    let (unit, _) = emit_twice("init-intrinsic-len", INIT_LEN);
    assert!(
        unit.contains("cl_spec_sei_impl_initlen_n = 3u;"),
        "init-intrinsic-len: the fixed-array count did not fold"
    );
}

/// Re-emitting the module-initializer programs is byte-identical.
#[test]
fn should_keep_the_module_initializer_emission_byte_identical() {
    for (tag, source) in [
        ("reemission-init-fill", INIT_FILL),
        ("reemission-init-operand-once", INIT_OPERAND_ONCE),
        ("reemission-init-conditional", INIT_CONDITIONAL),
        ("reemission-init-view", INIT_VIEW),
        ("reemission-init-len", INIT_LEN),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(first, second, "{tag}: the two emissions differ");
    }
}

/// A module `slice<T>` whose type argument disagrees with the source element
/// is rejected exactly as the function path rejects it.
#[test]
fn should_reject_a_slice_type_argument_mismatch_at_module_level() {
    assert_rejected(
        "slice-type-mismatch",
        r#"module spec.sei.impl.slicemismatch;

import { slice } from std.mem;

func run(): i32 {
    let backing: u8[2] = [1, 2];
    let v: i32[] = slice<i32>(backing, 0, 2);
    return v[0];
}

func main(): i32 {
    return run();
}
"#,
        &["E10200"],
        &["cannot convert 'u8[2]' to 'i32[]'"],
    );
}

/// A materialized module operand keeps its guard, and a fixed-array slice
/// operand keeps its decayed form.
///
/// The condition's bounds guard is a statement before the condition store, so
/// the run traps instead of reading past the array; the fixed-array slice
/// source keeps its ternary form, which the view construction uses through
/// the array-to-pointer decay, and the condition still runs exactly once.
#[test]
fn should_keep_module_operand_guards_and_array_decay() {
    let guarded = r#"module spec.sei.impl.guardedinit;

let backing: i32[3] = [1, 2, 3];
let x: i32[2] = [10, 11];
let y: i32[2] = [20, 21];
let idx: usize = 5;
let a: i32[2] = backing[idx] > 0 ? x : y;

func main(): i32 {
    return a[0];
}
"#;
    let (unit, _) = emit_twice("init-guarded-condition", guarded);
    let guard = position_of(&unit, "cl_trap(\"E11211\", \"index out of range\")");
    let store = position_of(
        &unit,
        "bool __cl_itmp0 = (cl_spec_sei_impl_guardedinit_backing",
    );
    assert!(
        guard < store,
        "init-guarded-condition: the condition store precedes its bounds guard"
    );

    let decay = r#"module spec.sei.impl.sliceconditional;

import { slice } from std.mem;

let a: i32[2] = [1, 2];
let b: i32[2] = [3, 4];

func pick(): bool {
    return true;
}

let v: i32[] = slice(pick() ? a : b, 0, 2);

func main(): i32 {
    return v[0];
}
"#;
    let (unit, _) = emit_twice("init-slice-conditional", decay);
    assert_eq!(
        count_of(&unit, "cl_spec_sei_impl_sliceconditional_pick()"),
        1,
        "init-slice-conditional: the condition does not run exactly once"
    );
    assert!(
        unit.contains(
            "(cl_spec_sei_impl_sliceconditional_pick() ? cl_spec_sei_impl_sliceconditional_a : cl_spec_sei_impl_sliceconditional_b)"
        ),
        "init-slice-conditional: the array operand does not keep its ternary form"
    );
    assert!(
        !unit.contains("__cl_itmp0[2]"),
        "init-slice-conditional: the array operand is materialized as an array temporary"
    );

    let impure = r#"module spec.sei.impl.impureindex;

let backing: i32[3] = [1, 2, 3];
let x: i32[2] = [10, 11];
let y: i32[2] = [20, 21];
let calls: i32 = 0;

func idx(): usize {
    calls = calls + 1;
    if (calls == 1) {
        return 0;
    }
    return 5;
}

let a: i32[2] = backing[idx()] > 0 ? x : y;

func main(): i32 {
    return a[0] + calls;
}
"#;
    let (unit, _) = emit_twice("init-impure-index", impure);
    assert_eq!(
        count_of(&unit, "cl_spec_sei_impl_impureindex_idx()"),
        1,
        "init-impure-index: the index does not run exactly once"
    );
    let store = position_of(
        &unit,
        "usize __cl_itmp0 = cl_spec_sei_impl_impureindex_idx();",
    );
    let guard = position_of(&unit, "(usize)(__cl_itmp0) >= (usize)(3)");
    let read = position_of(&unit, "cl_spec_sei_impl_impureindex_backing[__cl_itmp0]");
    assert!(
        store < guard && guard < read,
        "init-impure-index: the guard and the read do not share the stored index"
    );
}
