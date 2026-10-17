//! Implementation tests for the checker conformance batch.
//!
//! The specification suite proves what the programs compute; these cases prove
//! how the compiler emits the fixed shapes: the initializer's single-evaluation
//! stores for an impure dividend and an impure slice index base, the slice
//! typedefs an inline argument, a lifted specialization, and a generic
//! struct-instance element need (with the instance declared first), and the
//! owning index base's unchanged release.

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
            "clc-criimpl-{}-{tag}-{sequence}",
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
        "{tag}: the emitted C is empty"
    );
    emitted
}

/// Emits one hosted-amiga program and asserts the front end accepted it.
///
/// The cross toolchain is not needed for an emission pin: the profile only
/// selects the ABI typedefs and the null spelling, both visible in the
/// emitted C. The spec suite's hosted-amiga helpers build the same emission.
fn emit_hosted_amiga(tag: &str, source: &str) -> String {
    emit_profile(tag, source, "hosted-amiga")
}

/// Emits one freestanding program and asserts the front end accepted it.
///
/// The freestanding profile has no operating system surface, so an emission
/// pin checks the keyed symbol and the absent placeholder without building a
/// target binary. The freestanding suite owns the link and run legs.
fn emit_freestanding(tag: &str, source: &str) -> String {
    emit_profile(tag, source, "freestanding")
}

/// Emits one program for `profile` and asserts the front end accepted it.
fn emit_profile(tag: &str, source: &str, profile: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
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
        "{tag}: the emitted C is empty"
    );
    emitted
}

/// Runs one host-profile program and asserts the front end rejected it.
///
/// Returns the diagnostics' text. The phase's changes are diagnostic-only, so
/// a rejected program must not write a translation unit.
fn reject_host(tag: &str, source: &str) -> String {
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
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program wrote a translation unit"
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Asserts the emitted C contains `needle`, reporting the full text otherwise.
fn assert_contains(tag: &str, emitted: &str, needle: &str) {
    assert!(
        emitted.contains(needle),
        "{tag}: expected the emitted C to contain {needle:?}; emitted C:\n{emitted}"
    );
}

/// Counts non-overlapping occurrences of `needle`.
fn count(emitted: &str, needle: &str) -> usize {
    emitted.matches(needle).count()
}

/// Emits one program twice from the same source path, returning both texts.
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



/// The accepted `cstring` instance of the deferred argument conversion.
const DEFERRED_FFI_OK_SOURCE: &str = r#"module spec.cc.impl.deferredokffi;

import { toCString } from std.text;

ffi probe {
    func Open(name: cstring, mode: i32): raw u8;
}

func convert<T>(x: T): raw u8 {
    unsafe {
        return Open(x, 0);
    }
}

func main(): i32 {
    let c: cstring = toCString("/definitely/not/here");
    unsafe {
        let f: raw u8 = convert<cstring>(c);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;

/// The accepted `str` instance of the deferred argument conversion.
const DEFERRED_VIEW_SOURCE: &str = r#"module spec.cc.impl.deferredokview;

func span(s: str): i32 {
    return i32(len(s));
}

func sink<T>(x: T): i32 {
    return span(x);
}

func main(): i32 {
    let s: str = "abcd";
    return sink<str>(s);
}
"#;

/// The accepted `String` widening instance of the deferred argument conversion.
const DEFERRED_OWNED_SOURCE: &str = r#"module spec.cc.impl.deferredokowned;

import { String, stringOf } from std.text;

func span(s: str): i32 {
    return i32(len(s));
}

func sink<T>(x: T): i32 {
    return span(x);
}

func main(): i32 {
    return sink<String>(stringOf("abc"));
}
"#;

/// The accepted identity round trip over `str`, `String`, and `cstring`.
const DEFERRED_IDENTITY_SOURCE: &str = r#"module spec.cc.impl.deferredokident;

import { stringOf, toCString } from std.text;

func keep<T>(take x: T): T {
    let y: T = x;
    return y;
}

func main(): i32 {
    let s: str = "abcd";
    let t: str = keep<str>(s);
    if (len(t) != 4) {
        return 1;
    }
    let u: String = keep<String>(stringOf("abc"));
    if (len(u) != 3) {
        return 2;
    }
    let c: cstring = toCString("ab");
    let d: cstring = keep<cstring>(c);
    if (d == null) {
        return 3;
    }
    return 0;
}
"#;

/// The accepted `ffi` prototype and integer-use control.
const DEFERRED_FFI_RESULT_OK_SOURCE: &str = r#"module spec.cc.impl.ffiresultok;

ffi probe {
    func Open(name: cstring, mode: i32): raw u8;
    func Read(file: raw u8, buf: raw u8, count: i32): i32;
    func Close(file: raw u8): void;
    func Count(): i32;
}

func sum(): i32 {
    let n: i32 = 0;
    unsafe {
        n = Count() + 1;
    }
    return n;
}

func main(): i32 {
    unsafe {
        let f: raw u8 = Open("/dev/null", 0);
        if (f == null) {
            return 1;
        }
        let a: u8[4] = [0, 0, 0, 0];
        let n: i32 = Read(f, &a[0], 4);
        Close(f);
        if (n == 0) {
            return 7;
        }
        return sum();
    }
}
"#;

/// The initializer stores an impure dividend once.
///
/// The guard and the division read the stored local, so the call appears
/// exactly once (red at the phase base: the guard re-rendered the call).
#[test]
fn should_store_an_impure_dividend_once_in_an_initializer() {
    let source = r#"module spec.cc.impl.initdividend;

let calls: i32 = 0;

func d(): i32 {
    calls = calls + 1;
    return 2;
}

let g: i32 = d() / 1;

func main(): i32 {
    return calls * 10 + g;
}
"#;
    let emitted = emit_host("initdividend", source);
    assert_contains("initdividend",
        &emitted,
        "i32 __cl_itmp0 = cl_spec_cc_impl_initdividend_d();",
    );
    assert_contains("initdividend", &emitted, "(__cl_itmp0 == -2147483648");
    assert_contains("initdividend",
        &emitted,
        "cl_spec_cc_impl_initdividend_g = (__cl_itmp0 / 1);",
    );
    assert_eq!(count(&emitted, "cl_spec_cc_impl_initdividend_d();"), 1);
}

/// The initializer stores an impure slice index base once.
///
/// The bounds guard and the element access read the stored local, so the call
/// behind the base appears exactly once (red at the phase base: the guard
/// re-rendered the call).
#[test]
fn should_store_an_impure_index_base_once_in_an_initializer() {
    let source = r#"module spec.cc.impl.initbase;

import { slice } from std.mem;

let calls: i32 = 0;
let xs: i32[4] = [1, 2, 3, 4];

func v(s: i32[]): i32[] {
    calls = calls + 1;
    return s;
}

let sl: i32[] = slice(xs, 0, 4);
let e: i32 = v(sl)[1];

func main(): i32 {
    return calls * 10 + e;
}
"#;
    let emitted = emit_host("initbase", source);
    assert_contains("initbase",
        &emitted,
        "ClSlice_i32 __cl_itmp0 = cl_spec_cc_impl_initbase_v(cl_spec_cc_impl_initbase_sl);",
    );
    assert_contains("initbase", &emitted, "((usize)(1) >= (usize)((__cl_itmp0).len))");
    assert_contains("initbase",
        &emitted,
        "cl_spec_cc_impl_initbase_e = (__cl_itmp0).ptr[1];",
    );
    assert_eq!(
        count(
            &emitted,
            "cl_spec_cc_impl_initbase_v(cl_spec_cc_impl_initbase_sl)"
        ),
        1
    );
}

/// An inline slice argument defines its typedef and use.
///
/// The builder's compound literal names the view type, so the typedef is
/// declared before the function that builds it.
#[test]
fn should_define_an_inline_argument_slice_typedef() {
    let source = r#"module spec.cc.impl.inlinearg;

import { slice } from std.mem;
import { stringOf } from std.text;

func mkS(): String[2] {
    return [stringOf("ab"), stringOf("cd")];
}

func main(): i32 {
    return i32(len(slice(mkS(), 0, 2)));
}
"#;
    let emitted = emit_host("inlinearg", source);
    assert_contains("inlinearg",
        &emitted,
        "typedef struct { ClString *ptr; usize len; } ClSlice_String;",
    );
    assert_contains("inlinearg",
        &emitted,
        "((ClSlice_String){ (__cl_tmp0) + (0), (usize)((2) - (0)) })",
    );
}

/// A lifted-lambda specialization defines its written slice typedef.
///
/// The specialization's body measures the view, so the typedef precedes the
/// lifted function that uses it.
#[test]
fn should_define_a_lifted_specialization_slice_typedef() {
    let source = r#"module spec.cc.impl.lambdaspec;

func makeSize<T>(): func(): usize {
    return () => sizeof(T[]);
}

func main(): i32 {
    let size: func(): usize = makeSize<i32>();
    if (size() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("lambdaspec", source);
    assert_contains("lambdaspec",
        &emitted,
        "typedef struct { i32 *ptr; usize len; } ClSlice_i32;",
    );
    assert_contains("lambdaspec", &emitted, "static usize cl_lambda_0__i32(void)");
    assert_contains("lambdaspec", &emitted, "return sizeof(ClSlice_i32);");
}

/// A generic struct-instance element declares its instance first.
///
/// The slice typedef names the instance type, so the instance's own typedef
/// precedes the view typedef that references it.
#[test]
fn should_declare_a_generic_struct_instance_before_its_slice_typedef() {
    let source = r#"module spec.cc.impl.structelement;

struct Wrap<T> {
    value: T;
}

func sizeOf<T>(): usize {
    return sizeof(T[]);
}

func main(): i32 {
    if (sizeOf<Wrap<i32>>() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("structelement", source);
    let instance = emitted
        .find("typedef struct ClWrap__i32 ClWrap__i32;")
        .expect("structelement: the instance typedef is missing");
    let slice = emitted
        .find("typedef struct { ClWrap__i32 *ptr; usize len; } ClSlice_Wrap__i32;")
        .expect("structelement: the slice typedef is missing");
    assert!(
        instance < slice,
        "structelement: the instance typedef must precede the slice typedef"
    );
    assert_contains("structelement", &emitted, "return sizeof(ClSlice_Wrap__i32);");
}

/// An owning index base keeps its single call and release.
///
/// The call-result fixed array behind the run-time index stores once and its
/// elements release at the init function's end, exactly as before the fix.
#[test]
fn should_release_an_owning_index_base_once() {
    let source = r#"module spec.cc.impl.initownbase;

import { stringOf } from std.text;

let position: usize = 0;
let calls: i32 = 0;

func mk(): String[2] {
    calls = calls + 1;
    return [stringOf("ab"), stringOf("cd")];
}

let n: usize = len(mk()[position]);

func main(): i32 {
    return calls * 10 + i32(n);
}
"#;
    let emitted = emit_host("initownbase", source);
    assert_eq!(
        count(&emitted, "cl_spec_cc_impl_initownbase_mk(__cl_itmp0)"),
        1
    );
    assert_contains("initownbase", &emitted, "FreeString(__cl_itmp0[0])");
    assert_contains("initownbase", &emitted, "FreeString(__cl_itmp0[1])");
}

/// A changed initializer shape re-emits byte-identically.
///
/// The store addition is deterministic: the same source emits the same
/// translation unit on a second run.
#[test]
fn should_re_emit_a_stored_initializer_identically() {
    let source = r#"module spec.cc.impl.reemit;

import { slice } from std.mem;

let calls: i32 = 0;
let xs: i32[4] = [1, 2, 3, 4];

func v(s: i32[]): i32[] {
    calls = calls + 1;
    return s;
}

let sl: i32[] = slice(xs, 0, 4);
let e: i32 = v(sl)[1];

func main(): i32 {
    return calls * 10 + e;
}
"#;
    let (first, second) = emit_twice("reemit", source);
    assert_eq!(
        first, second,
        "reemit: the initializer store re-emitted differently"
    );
    assert_contains(
        "reemit",
        &first,
        "ClSlice_i32 __cl_itmp0 = cl_spec_cc_impl_reemit_v(cl_spec_cc_impl_reemit_sl);",
    );
}

/// The conditional-wrapped projection folds through the substituted base.
///
/// Both arms name the same field of the instance's struct, so the fold emits
/// the extent as a constant (red at the phase base: the instance returned
/// zero).
#[test]
fn should_fold_the_conditional_projection_in_the_instance() {
    let source = r#"module spec.cc.impl.condfold;

struct B {
    s: i32[7];
}

func cond<T>(b: T, flag: bool): usize {
    return len(flag ? b.s : b.s);
}

func main(): i32 {
    return i32(cond(B { s: [1, 2, 3, 4, 5, 6, 7] }, true));
}
"#;
    let emitted = emit_host("condfold", source);
    assert_contains("condfold", &emitted, "static usize cl_cond__B(ClB b, bool flag)");
    assert_contains("condfold", &emitted, "return 7;");
}

/// An opaque projection resolves through the instance's substituted type.
///
/// The read stays accepted at the declaration; the instance resolves it to the
/// struct field, so the emitted member access builds.
#[test]
fn should_resolve_an_opaque_projection_in_the_instance() {
    let source = r#"module spec.cc.impl.memok;

struct W {
    foo: i32;
}

func get<T>(x: T): i32 {
    return x.foo;
}

func main(): i32 {
    return get(W { foo: 7 });
}
"#;
    let emitted = emit_host("memok", source);
    assert_contains("memok", &emitted, "static i32 cl_get__W(ClW x)");
    assert_contains("memok", &emitted, "return x.foo;");
}

/// A scalar-element array instance keeps its loop shape.
#[test]
fn should_keep_the_scalar_element_loop_shape() {
    let source = r#"module spec.cc.impl.iterok;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let s: i32[4] = [1, 2, 3, 4];
    return each<i32[4]>(s);
}
"#;
    let emitted = emit_host("iterok", source);
    assert_contains("iterok", &emitted, "static i32 cl_each__arr4_i32(i32 xs[4])");
    assert_contains("iterok", &emitted, "i32 v = 0;");
    assert_contains("iterok", &emitted, "v = xs[__cl_tmp0];");
    assert_contains("iterok", &emitted, "(__cl_tmp0 < 4)");
}

/// A borrow element over a scalar array instance keeps its aliasing shape.
#[test]
fn should_keep_the_borrow_element_loop_shape() {
    let source = r#"module spec.cc.impl.iterborrow;

func eachB<T>(xs: T): i32 {
    let n: i32 = 0;
    for (borrow v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: i32[4] = [1, 2, 3, 4];
    return eachB<i32[4]>(a);
}
"#;
    let emitted = emit_host("iterborrow", source);
    assert_contains(
        "iterborrow",
        &emitted,
        "static i32 cl_eachB__arr4_i32(i32 xs[4])",
    );
    assert_contains("iterborrow", &emitted, "i32 *__cl_tmp1 = NULL;");
    assert_contains("iterborrow", &emitted, "__cl_tmp1 = &(xs[__cl_tmp0]);");
}

/// A struct-element instance binds the concrete element type.
///
/// The element placeholder resolves to the substituted argument's one-step
/// element, so the binding and its borrow address temporary carry the struct
/// (red at the phase base: the binding was the unknown fallback and failed the
/// strict build).
#[test]
fn should_bind_a_struct_element_in_the_instance() {
    let source = r#"module spec.cc.impl.iterstructpin;

struct S {
    x: i32;
}

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: S[2] = [S { x: 1 }, S { x: 2 }];
    return each<S[2]>(a);
}
"#;
    let emitted = emit_host("iterstructpin", source);
    assert_contains("iterstructpin", &emitted, "static i32 cl_each__arr2_S(ClS xs[2])");
    assert_contains("iterstructpin", &emitted, "ClS v = {0};");
    assert_contains("iterstructpin", &emitted, "ClS *__cl_tmp1 = NULL;");
}

/// A float-element instance binds the float element type.
///
/// The binding carries the substituted element, so the accumulation is a float
/// sum and the instance computes the concrete result.
#[test]
fn should_bind_a_float_element_in_the_instance() {
    let source = r#"module spec.cc.impl.iterf32pin;

func sumf<T>(xs: T): f32 {
    let s: f32 = 0.0;
    for (v in xs) {
        s = s + v;
    }
    return s;
}

func main(): i32 {
    let a: f32[2] = [1.5, 2.5];
    return i32(sumf<f32[2]>(a) * 10.0);
}
"#;
    let emitted = emit_host("iterf32pin", source);
    assert_contains(
        "iterf32pin",
        &emitted,
        "static float cl_sumf__arr2_f32(float xs[2])",
    );
    assert_contains("iterf32pin", &emitted, "float v = 0;");
    assert_contains("iterf32pin", &emitted, "s = (s + v);");
}

/// A `static str` literal at a `cstring` parameter stays the C string.
#[test]
fn should_keep_a_cstring_literal_unwrapped() {
    let source = r#"module spec.cc.impl.cstringlitpin;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    unsafe {
        let f: raw u8 = Open("/definitely/not/here", 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    let emitted = emit_host("cstringlitpin", source);
    assert_contains(
        "cstringlitpin",
        &emitted,
        r#"Open("/definitely/not/here", 0)"#,
    );
}

/// The exact argument counts in both call forms keep their call shapes.
///
/// The dot form passes the receiver as the free function's first argument and
/// the direct form passes the written arguments unchanged.
#[test]
fn should_keep_the_exact_argument_count_call_shapes() {
    let source = r#"module spec.cc.impl.arityokpin;

func addOne(a: i32): i32 {
    return a + 1;
}

func addTwo(a: i32, b: i32): i32 {
    return a + b;
}

func main(): i32 {
    let x: i32 = 10;
    let y: i32 = x.addOne();
    return addTwo(y, 5);
}
"#;
    let emitted = emit_host("arityokpin", source);
    assert_contains(
        "arityokpin",
        &emitted,
        "y = cl_spec_cc_impl_arityokpin_addOne(x);",
    );
    assert_contains(
        "arityokpin",
        &emitted,
        "return cl_spec_cc_impl_arityokpin_addTwo(y, 5);",
    );
}

/// A callable field keeps the field-wins call shape.
#[test]
fn should_keep_the_callable_field_call_shape() {
    let source = r#"module spec.cc.impl.memberfieldpin;

struct S {
    foo: func(i32): i32;
}

func foo(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let s: S = S { foo: foo };
    return s.foo(1);
}
"#;
    let emitted = emit_host("memberfieldpin", source);
    assert_contains("memberfieldpin", &emitted, "return (s.foo)(1);");
}

/// The widened `toCString` returns the view's pointer and widens an owned
/// argument through the view at the call site.
#[test]
fn should_emit_the_widened_to_c_string_shapes() {
    let source = r#"module spec.cc.impl.convpin;

import { stringOf, toCString } from std.text;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let s: str = "/definitely/not/here";
    let c: cstring = toCString(s);
    let owned: String = stringOf("/definitely/not/here");
    let d: cstring = toCString(owned);
    unsafe {
        let f: raw u8 = Open(c, 0);
        let g: raw u8 = Open(d, 0);
        if (f == null) {
            return 7;
        }
        if (g == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    let emitted = emit_host("convpin", source);
    assert_contains("convpin", &emitted, "char * cl_std_text_toCString(ClStr s)");
    assert_contains("convpin", &emitted, "return (char *)(s.ptr);");
    assert_contains("convpin", &emitted, "c = cl_std_text_toCString(s);");
    assert_contains(
        "convpin",
        &emitted,
        "d = cl_std_text_toCString(cl_string_view(owned));",
    );
}

/// The null comparisons keep their delegation shape.
#[test]
fn should_keep_the_null_shaped_comparison_shapes() {
    let source = r#"module spec.cc.impl.nullcomparepin;

func main(): i32 {
    let x = null;
    if (x == null) {
        return 1;
    }
    if (x != null) {
        return 2;
    }
    return 0;
}
"#;
    let emitted = emit_host("nullcomparepin", source);
    assert_contains("nullcomparepin", &emitted, "void * x = NULL;");
    assert_contains("nullcomparepin", &emitted, "(x == NULL)");
    assert_contains("nullcomparepin", &emitted, "(x != NULL)");
}

/// A copy between null-shaped locals keeps its assignment shape.
#[test]
fn should_keep_the_null_shaped_copy_shape() {
    let source = r#"module spec.cc.impl.nullcopypin;

func main(): i32 {
    let x = null;
    let y = x;
    if (y == null) {
        return 4;
    }
    return 0;
}
"#;
    let emitted = emit_host("nullcopypin", source);
    assert_contains("nullcopypin", &emitted, "void * y = NULL;");
    assert_contains("nullcopypin", &emitted, "y = x;");
}

/// Nullable declarations keep their zero initializers.
#[test]
fn should_keep_the_nullable_declaration_shapes() {
    let source = r#"module spec.cc.impl.nullnullablepin;

import { File } from std.io;

func main(): i32 {
    let f: File? = null;
    if (f != null) {
        return 1;
    }
    let p: raw u8? = null;
    if (p == null) {
        return 3;
    }
    return 0;
}
"#;
    let emitted = emit_host("nullnullablepin", source);
    assert_contains("nullnullablepin", &emitted, "BPTR f = NULL;");
    assert_contains("nullnullablepin", &emitted, "u8 *p = NULL;");
}

// ===== Phase 5: diagnostic hygiene =====

/// The moved field update keeps one report at its own site.
///
/// The file's path is a scratch directory, so the pin asserts the line and
/// column spelling inside the message's `-->` line.
#[test]
fn should_report_a_moved_update_once_with_its_site() {
    let source = r#"module spec.cc.impl.statededupepin;

import { String, stringOf } from std.text;

struct P { s: String; n: i32; }

func main(): i32 {
    let p = P { s: stringOf("ab"), n: 1 };
    let q = p;
    let y = p.n++;
    return q.n + y;
}
"#;
    let stderr = reject_host("statededupepin", source);
    assert_eq!(
        stderr.matches("error E10020:").count(),
        1,
        "statededupepin: expected one E10020; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'p' was moved and cannot be used"),
        "statededupepin: the moved message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:10:13"),
        "statededupepin: the report moved off its site; stderr:\n{stderr}"
    );
}

/// The capture move keeps one report at its own site and names the capture
/// rule, not the lend rule.
#[test]
fn should_report_a_capture_move_once_with_its_site() {
    let source = r#"module spec.cc.impl.capturepin;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        let y: T = x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    let stderr = reject_host("capturepin", source);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "capturepin: expected one E10401; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("E10034"),
        "capturepin: the lend rule still reports; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:7:20"),
        "capturepin: the report moved off its site; stderr:\n{stderr}"
    );
}

/// A generated owner renders as a phrase, never its compiler spelling.
#[test]
fn should_render_a_generated_owner_as_a_phrase() {
    let source = r#"module spec.cc.impl.escapeownerpin;

import { String, stringOf } from std.text;

struct Box {
    v: String;
}

func use(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: str = "zz";
    let i: i32 = 0;
    while (i < 2) {
        let t: str = (Box { v: stringOf("ab") }).v;
        if (i == 0) { s = t; }
        i = i + 1;
    }
    return use(s);
}
"#;
    let stderr = reject_host("escapeownerpin", source);
    assert!(
        stderr.contains("borrow of a compiler-generated owner does not live long enough"),
        "escapeownerpin: the phrase is missing; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("__cl_"),
        "escapeownerpin: the compiler spelling leaks; stderr:\n{stderr}"
    );
}

/// The legal update keeps its read-modify-write emission, byte-identically.
#[test]
fn should_keep_the_legal_update_emission() {
    let source = r#"module spec.cc.impl.stateokpin;

func main(): i32 {
    let a: i32[2] = [1, 2];
    a[0]++;
    let n: i32 = 3;
    n++;
    return a[0] + n;
}
"#;
    let (first, second) = emit_twice("stateokpin", source);
    assert_contains("stateokpin", &first, "a[0] = (i32)((u32)a[0] + (u32)1);");
    assert_eq!(first, second, "stateokpin: re-emission is not byte-identical");
}

/// A duplicable capture move keeps the copied environment emission.
#[test]
fn should_keep_the_duplicable_capture_emission() {
    let source = r#"module spec.cc.impl.captureduppin;

func make<T>(x: T): void {
    ([x] () => {
        let y: T = x;
    })();
}

func main(): i32 {
    make<i32>(5);
    return 0;
}
"#;
    let emitted = emit_host("captureduppin", source);
    assert_contains("captureduppin", &emitted, "struct ClEnv_0__i32");
    assert_contains("captureduppin", &emitted, "cl_lambda_0__i32");
}

// ===== Element placeholder, inference, and bounds =====

/// A loop-element member read passed to a generic callee.
///
/// `id(v.y)` over `S[2]` resolves the projected place at the instance, so the
/// argument keys the substituted field type; the re-emission pin reuses it.
const PROJECTED_ELEMENT_SOURCE: &str = r#"module spec.cc.impl.elemprojkey;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + id(v.y);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;

/// A projected place argument keys the substituted field type.
///
/// The member read on the element placeholder resolves through the instance's
/// substituted struct, so the inferred `id` instance keys `i32` and the keyed
/// declaration and call emit; the opaque key must not appear (red at the
/// phase base: the call keyed `cl_id__unknown` and the strict build failed
/// with an implicit declaration).
#[test]
fn should_key_a_projected_element_argument_in_the_instance() {
    let emitted = emit_host("elemprojkey", PROJECTED_ELEMENT_SOURCE);
    assert_contains("elemprojkey", &emitted, "static i32 cl_id__i32(i32 x);");
    assert_contains(
        "elemprojkey",
        &emitted,
        "total = (i32)((u32)total + (u32)cl_id__i32(v.y));",
    );
    assert_eq!(
        count(&emitted, "cl_id__unknown"),
        0,
        "elemprojkey: the opaque key must not be emitted"
    );
}

/// A capture of a loop element types its environment from the element.
///
/// The captured binding's checked type is the element placeholder, so the
/// lifted lambda's environment carries the instance's struct and the member
/// read resolves; the int-typed field must not appear (red at the phase base:
/// `incompatible types when initializing type 'int' using type 'ClS'` and
/// `request for member 'y'`).
#[test]
fn should_type_the_element_capture_environment_in_the_instance() {
    let source = r#"module spec.cc.impl.elemcapture;

struct S { y: i32; }

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + ([v] () => v.y)();
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;
    let emitted = emit_host("elemcapture", source);
    assert_contains(
        "elemcapture",
        &emitted,
        "struct ClEnv_0__arr2_S {\n    ClS v;\n};",
    );
    assert_eq!(
        count(&emitted, "    i32 v;\n"),
        0,
        "elemcapture: the environment field must not be int-typed"
    );
}

/// An inferred element argument keys the substituted element.
///
/// The argument is the loop element itself, so the `inner` instance maps the
/// placeholder to the argument's one-step element and the keyed declaration
/// and call emit; the opaque key must not appear (red at the phase base: the
/// call keyed `cl_inner__unknown` and the strict build failed with an
/// implicit declaration).
#[test]
fn should_key_a_placeholder_element_argument_in_the_instance() {
    let source = r#"module spec.cc.impl.eleminfer;

func inner<T>(x: T): T { return x; }

func outer<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + inner(v);
    }
    return n;
}

func main(): i32 {
    let a: i32[2] = [1, 2];
    return outer<i32[2]>(a);
}
"#;
    let emitted = emit_host("eleminfer", source);
    assert_contains("eleminfer", &emitted, "static i32 cl_inner__i32(i32 x);");
    assert_contains(
        "eleminfer",
        &emitted,
        "n = (i32)((u32)n + (u32)cl_inner__i32(v));",
    );
    assert_eq!(
        count(&emitted, "cl_inner__unknown"),
        0,
        "eleminfer: the opaque key must not be emitted"
    );
}

/// A runtime row sum whose guard decides on the substituted row.
const ROW_GUARD_SOURCE: &str = r#"module spec.cc.impl.rowguard;

func rowsum<T>(xs: T, i: usize): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[i];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let i: usize = 5;
    return rowsum<i32[2][3]>(rows, i);
}
"#;

/// A runtime index of a substituted row carries its bounds guard.
///
/// The guard condition decides on the substituted row extent, so an index of
/// 5 traps at the frozen bounds check, the in-bounds control carries the same
/// guard, and the constant index 5 reports the frozen range error before
/// emission (red at the phase base: no guard emitted and the constant
/// accepted).
#[test]
fn should_guard_a_runtime_row_index_in_the_instance() {
    let emitted = emit_host("rowguard", ROW_GUARD_SOURCE);
    assert_contains(
        "rowguard",
        &emitted,
        "if ((usize)(i) >= (usize)(3)) cl_trap(\"E11211\", \"index out of range\");",
    );

    let in_bounds = ROW_GUARD_SOURCE
        .replace("let i: usize = 5;", "let i: usize = 2;")
        .replace("impl.rowguard;", "impl.rowguardok;");
    let emitted = emit_host("rowguardok", &in_bounds);
    assert_contains(
        "rowguardok",
        &emitted,
        "if ((usize)(i) >= (usize)(3)) cl_trap(\"E11211\", \"index out of range\");",
    );

    let constant = r#"module spec.cc.impl.rowguardconst;

func rowsum<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[5];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return rowsum<i32[2][3]>(rows);
}
"#;
    let stderr = reject_host("rowguardconst", constant);
    assert_eq!(
        stderr.matches("error E10216:").count(),
        1,
        "rowguardconst: expected one E10216; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("index 5 is out of range for 'i32[3]'"),
        "rowguardconst: the range message is missing; stderr:\n{stderr}"
    );
}

/// The projected-element and row-guard programs re-emit byte-identically.
///
/// Both re-emit from one source path so their `#line` facts agree, and the
/// re-emitted text keeps the keyed call and the bounds guard.
#[test]
fn should_re_emit_the_element_and_row_programs_byte_identically() {
    for (tag, source, shape) in [
        (
            "elemprojkey",
            PROJECTED_ELEMENT_SOURCE,
            "total = (i32)((u32)total + (u32)cl_id__i32(v.y));",
        ),
        (
            "rowguard",
            ROW_GUARD_SOURCE,
            "if ((usize)(i) >= (usize)(3)) cl_trap(\"E11211\", \"index out of range\");",
        ),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(first, second, "{tag}: the program re-emitted differently");
        assert_contains(tag, &first, shape);
    }
}

// ===== Member typing: enum value form and view members =====

/// A view pointer member read through its typed member and conversions.
///
/// `.ptr` carries the view's byte pointer, so a `raw u8` destination takes it
/// directly and a `cstring` destination spells the sanctioned pointer
/// conversion; the stdlib's `toCString` body keeps the same cast.
const VIEW_POINTER_SOURCE: &str = r#"module spec.cc.impl.ptrpin;

import { toCString } from std.text;

func ptr(s: str): raw u8 {
    return s.ptr;
}

func cstr(s: str): cstring {
    return s.ptr;
}

func main(): i32 {
    let s: str = "abc";
    let p: raw u8 = ptr(s);
    let c: cstring = cstr(s);
    let d: cstring = toCString(s);
    unsafe {
        if (p == null) {
            return 1;
        }
    }
    if (c == null) {
        return 2;
    }
    if (d == null) {
        return 3;
    }
    return 0;
}
"#;

/// A view pointer member read is typed and its conversions judge.
///
/// The member's checked type is the byte pointer, so a `raw u8` destination
/// takes the read directly, a `cstring` destination spells the pointer
/// conversion, and an integer destination reports the frozen conversion rule
/// (red at the phase base: the member was untyped, the integer return was
/// accepted, and the strict build failed with `int-conversion`).
#[test]
fn should_type_a_view_pointer_member_for_its_conversions() {
    let emitted = emit_host("ptrpin", VIEW_POINTER_SOURCE);
    assert_contains("ptrpin", &emitted, "static u8 * cl_spec_cc_impl_ptrpin_ptr(ClStr s)");
    // The `raw u8` return spells the sanctioned member cast: a text view's
    // pointer member is a `const u8 *` in the C while a mutable raw
    // destination holds a byte pointer.
    assert_contains("ptrpin", &emitted, "return (u8 *)(s.ptr);");
    // The local `cstring` return and the stdlib `toCString` body both spell
    // the sanctioned pointer conversion.
    assert_eq!(
        count(&emitted, "return (char *)(s.ptr);"),
        2,
        "ptrpin: the sanctioned pointer conversions are missing"
    );

    let integer = r#"module spec.cc.impl.ptrintpin;

func span(s: str): usize {
    return s.ptr;
}

func main(): i32 {
    let s: str = "abc";
    return i32(span(s));
}
"#;
    let stderr = reject_host("ptrintpin", integer);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "ptrintpin: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'raw u8' to 'usize' implicitly"),
        "ptrintpin: the conversion message is missing; stderr:\n{stderr}"
    );
}

/// A view length member read keeps its typed scalar shape.
///
/// `.len` is the view's element count typed `usize`, so the return needs no
/// conversion and the member renders as the struct field.
const VIEW_LENGTH_SOURCE: &str = r#"module spec.cc.impl.lenpin;

func span(s: str): usize {
    return s.len;
}

func main(): i32 {
    let s: str = "abcd";
    return i32(span(s)) - 4;
}
"#;

/// A view length member stays typed and unconverted.
#[test]
fn should_keep_a_view_length_member_typed() {
    let emitted = emit_host("lenpin", VIEW_LENGTH_SOURCE);
    assert_contains("lenpin", &emitted, "static usize cl_spec_cc_impl_lenpin_span(ClStr s)");
    assert_contains("lenpin", &emitted, "return s.len;");
    assert_eq!(
        count(&emitted, "cannot convert"),
        0,
        "lenpin: the length member must need no conversion"
    );
}

/// A value-form enum member read reports the unknown member at both paths.
///
/// The concrete read and the instance read each report exactly one `E10212`
/// naming the enum, because the frozen member table spells the member
/// `Type.MEMBER` (red at the phase base: both were accepted and the strict
/// build failed with `request for member 'A' in something not a structure or
/// union`). The scoped and qualified controls keep their emission.
#[test]
fn should_reject_a_value_form_enum_member_read_at_both_paths() {
    let concrete = r#"module spec.cc.impl.enumpin;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.A;
    let n = i32(e.A);
    return n - 1;
}
"#;
    let stderr = reject_host("enumpin", concrete);
    assert_eq!(
        stderr.matches("error E10212:").count(),
        1,
        "enumpin: expected one E10212; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'A' is not a member of 'E'"),
        "enumpin: the unknown-member message is missing; stderr:\n{stderr}"
    );

    let instance = r#"module spec.cc.impl.enumpininst;

enum E : u8 { A, B }

func read<T>(x: T): i32 {
    return i32(x.A);
}

func main(): i32 {
    let e: E = E.B;
    return read<E>(e);
}
"#;
    let stderr = reject_host("enumpininst", instance);
    assert_eq!(
        stderr.matches("error E10212:").count(),
        1,
        "enumpininst: expected one E10212; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'A' is not a member of 'E'"),
        "enumpininst: the unknown-member message is missing; stderr:\n{stderr}"
    );

    let scoped = r#"module spec.cc.impl.enumctl;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.B;
    return i32(e);
}
"#;
    let emitted = emit_host("enumctl", scoped);
    assert_contains("enumctl", &emitted, "e = E_B;");
    assert_contains("enumctl", &emitted, "return (i32)(e);");

    let qualified = r#"module spec.cc.impl.enumqual;

func main(): i32 {
    let owned: String = std.text.stringOf("abc");
    let v: u8[] = std.text.bytes(owned);
    return i32(len(v));
}
"#;
    let emitted = emit_host("enumqual", qualified);
    assert_contains("enumqual", &emitted, "cl_std_text_stringOf(cl_str(\"abc\"))");
}

/// The pointer and length member programs re-emit byte-identically.
///
/// Both re-emit from one source path so their `#line` facts agree, and the
/// re-emitted text keeps the typed member spellings.
#[test]
fn should_re_emit_the_view_member_programs_byte_identically() {
    for (tag, source, shape) in [
        ("ptrpin", VIEW_POINTER_SOURCE, "return (char *)(s.ptr);"),
        ("lenpin", VIEW_LENGTH_SOURCE, "return s.len;"),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(first, second, "{tag}: the program re-emitted differently");
        assert_contains(tag, &first, shape);
    }
}

// ===== Deferred conversion duties and ffi result typing =====

/// A parameter-typed argument's conversion is judged at its instance.
///
/// A written argument whose type is a parameter cannot be judged at the
/// declaration; the instance audit reports the frozen conversion once, at the
/// argument that cannot convert. Both argument routes judge the same way: the
/// `ffi` declaration and the ordinary function. Without the judgment the
/// accepted program passed the view struct where the `char *` parameter is
/// declared and the strict build failed with `incompatible type for argument
/// 1`.
#[test]
fn should_report_a_deferred_argument_conversion_at_its_instance() {
    let ffi = r#"module spec.cc.impl.deferredffi;

ffi probe {
    func cl_probe_convert(name: cstring, mode: i32): raw u8;
}

func convert<T>(x: T): raw u8 {
    unsafe {
        return cl_probe_convert(x, 0);
    }
}

func main(): i32 {
    let s: str = "abc";
    unsafe {
        let f: raw u8 = convert<str>(s);
        if (f == null) {
            return 1;
        }
    }
    return 0;
}
"#;
    let stderr = reject_host("deferredffi", ffi);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "deferredffi: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'str' to 'cstring' implicitly"),
        "deferredffi: the conversion message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:9:33"),
        "deferredffi: the report must name the deferred argument; stderr:\n{stderr}"
    );

    let direct = r#"module spec.cc.impl.deferreddirect;

func consume(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func convert<T>(x: T): i32 {
    return consume(x);
}

func main(): i32 {
    let s: str = "abc";
    return convert<str>(s);
}
"#;
    let stderr = reject_host("deferreddirect", direct);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "deferreddirect: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'str' to 'cstring' implicitly"),
        "deferreddirect: the conversion message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:11:20"),
        "deferreddirect: the report must name the deferred argument; stderr:\n{stderr}"
    );
}

/// A parameter-typed return and declaration are judged at their instances.
///
/// The return type and the declared type are concrete, so the substituted
/// value faces the frozen conversion and reports once per site (without the
/// judgment the accepted programs failed the strict build with `incompatible
/// types when returning` and `incompatible types when assigning`).
#[test]
fn should_report_a_deferred_return_and_declaration_conversion_at_its_instance() {
    let ret = r#"module spec.cc.impl.deferredret;

func pick<T>(x: T): i32 {
    return x;
}

func main(): i32 {
    let s: str = "abc";
    return pick<str>(s);
}
"#;
    let stderr = reject_host("deferredret", ret);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "deferredret: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'str' to 'i32' implicitly"),
        "deferredret: the conversion message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:4:12"),
        "deferredret: the report must name the returned value; stderr:\n{stderr}"
    );

    let decl = r#"module spec.cc.impl.deferreddecl;

func keep<T>(x: T): i32 {
    let y: i32 = x;
    return y;
}

func main(): i32 {
    let s: str = "abc";
    return keep<str>(s);
}
"#;
    let stderr = reject_host("deferreddecl", decl);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "deferreddecl: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'str' to 'i32' implicitly"),
        "deferreddecl: the conversion message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:4:18"),
        "deferreddecl: the report must name the declared value; stderr:\n{stderr}"
    );
}

/// A null-shaped value at a concrete destination reports the null rule.
///
/// The null shape has no concrete value surface to convert, so the instance
/// judgment reports the frozen null rule once and never the conversion (the
/// accepted programs previously failed the strict build with `returning 'void
/// *'` and `assignment to 'i32' from 'void *'`).
#[test]
fn should_report_a_deferred_null_shape_at_its_instance() {
    let ret = r#"module spec.cc.impl.deferrednullret;

func make<T>(): T {
    let x = null;
    return x;
}

func main(): i32 {
    return i32(make<i32>()) + 1;
}
"#;
    let stderr = reject_host("deferrednullret", ret);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "deferrednullret: expected one E10012; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'null' is not allowed here"),
        "deferrednullret: the null message is missing; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("E10200"),
        "deferrednullret: the null shape must not report as a conversion; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:5:12"),
        "deferrednullret: the report must name the returned value; stderr:\n{stderr}"
    );

    let decl = r#"module spec.cc.impl.deferrednulldecl;

func store<T>(): i32 {
    let x = null;
    let y: T = x;
    return 0;
}

func main(): i32 {
    return store<i32>() + 1;
}
"#;
    let stderr = reject_host("deferrednulldecl", decl);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "deferrednulldecl: expected one E10012; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'null' is not allowed here"),
        "deferrednulldecl: the null message is missing; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("E10200"),
        "deferrednulldecl: the null shape must not report as a conversion; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:5:16"),
        "deferrednulldecl: the report must name the declared value; stderr:\n{stderr}"
    );
}

/// The legal conversion instances keep their emitted calls.
///
/// The instance audit reports only instances whose conversion is not implicit,
/// so a `cstring` instance at an `ffi` `cstring` parameter, a `str` instance
/// at a `str` parameter, and a `String` instance widening to `str` all emit
/// their substituted calls: no cast appears at the parameter and the handle
/// wraps in the view conversion.
#[test]
fn should_keep_the_legal_conversion_instances_emitting_their_calls() {
    let emitted = emit_host("deferredokffi", DEFERRED_FFI_OK_SOURCE);
    assert_contains("deferredokffi", &emitted, "APTR Open(char *, LONG);");
    assert_contains("deferredokffi", &emitted, "static u8 * cl_convert__cstring(char *x)");
    assert_contains("deferredokffi", &emitted, "return Open(x, 0);");

    let emitted = emit_host("deferredokview", DEFERRED_VIEW_SOURCE);
    assert_contains(
        "deferredokview",
        &emitted,
        "static i32 cl_sink__str(ClStr x);",
    );
    assert_contains(
        "deferredokview",
        &emitted,
        "return cl_spec_cc_impl_deferredokview_span(x);",
    );

    let emitted = emit_host("deferredokowned", DEFERRED_OWNED_SOURCE);
    assert_contains(
        "deferredokowned",
        &emitted,
        "return cl_spec_cc_impl_deferredokowned_span(cl_string_view(x));",
    );
}

/// An `ffi` result keeps its declared type in the emitted prototypes.
///
/// The declared signature is the ABI: the prototype renders the declared
/// parameter and result types, and a valid integer use emits the call without
/// a cast.
#[test]
fn should_keep_an_ffi_result_typed_in_its_prototypes() {
    let emitted = emit_host("ffiresultok", DEFERRED_FFI_RESULT_OK_SOURCE);
    assert_contains("ffiresultok", &emitted, "APTR Open(char *, LONG);");
    assert_contains("ffiresultok", &emitted, "LONG Read(APTR, APTR, LONG);");
    assert_contains("ffiresultok", &emitted, "void Close(APTR);");
}

/// An `ffi` integer result carries its type into arithmetic.
///
/// With the declared result typed, the operands are `i32` and the sum emits
/// the explicit 32-bit wrap (before the typing the operands were unresolved
/// and the sum rendered bare).
#[test]
fn should_type_an_ffi_result_in_its_arithmetic() {
    let source = r#"module spec.cc.impl.ffiresultsum;

ffi probe {
    func Count(): i32;
    func Other(): i32;
    func More(): i32;
}

func total(): i32 {
    let n: i32 = 0;
    unsafe {
        n = Count() + Other() + More();
    }
    return n;
}

func main(): i32 {
    return total();
}
"#;
    let emitted = emit_host("ffiresultsum", source);
    assert_contains("ffiresultsum", &emitted, "LONG Count(void);");
    assert_contains(
        "ffiresultsum",
        &emitted,
        "n = (i32)((u32)(i32)((u32)Count() + (u32)Other()) + (u32)More());",
    );
}

/// The accepted instances re-emit identically.
#[test]
fn should_re_emit_the_deferred_conversion_instances_identically() {
    for (tag, source) in [
        ("reemitident", DEFERRED_IDENTITY_SOURCE),
        ("reemitffiok", DEFERRED_FFI_OK_SOURCE),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(first, second, "{tag}: the program re-emitted differently");
    }
}

// ===== Deferred dot-call receivers and the null-shape destination gate =====

/// A parameter-typed dot-call receiver reports once at its instance.
///
/// The receiver is a written argument at index zero, so its deferred
/// conversion is recorded like an argument's and judged at the instance, at
/// the receiver's span. The `raw u8` instance reports the frozen conversion
/// too, where the emitted cast had kept the program building. The legal
/// `cstring` instance emits the substituted call unchanged.
#[test]
fn should_report_a_deferred_dot_call_receiver_at_its_instance() {
    let receiver = r#"module spec.cc.impl.receiverduty;

func sink(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func conv<T>(x: T): i32 {
    return x.sink();
}

func main(): i32 {
    let s: str = "abc";
    return conv<str>(s);
}
"#;
    let stderr = reject_host("receiverduty", receiver);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "receiverduty: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'str' to 'cstring' implicitly"),
        "receiverduty: the conversion message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:11:12"),
        "receiverduty: the report must name the receiver; stderr:\n{stderr}"
    );

    let raw = r#"module spec.cc.impl.receiverraw;

func sink(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func conv<T>(x: T): i32 {
    return x.sink();
}

func main(): i32 {
    let a: u8[4] = [65, 66, 67, 0];
    let p: raw u8 = unsafe &a[0];
    return conv<raw u8>(p);
}
"#;
    let stderr = reject_host("receiverraw", raw);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "receiverraw: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'raw u8' to 'cstring' implicitly"),
        "receiverraw: the conversion message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:11:12"),
        "receiverraw: the report must name the receiver; stderr:\n{stderr}"
    );

    let legal = r#"module spec.cc.impl.receiverlegal;

import { toCString } from std.text;

func sink(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func conv<T>(x: T): i32 {
    return x.sink();
}

func main(): i32 {
    let c: cstring = toCString("abc");
    return conv<cstring>(c);
}
"#;
    let emitted = emit_host("receiverlegal", legal);
    assert_contains("receiverlegal", &emitted, "static i32 cl_conv__cstring(char *x)");
    assert_contains(
        "receiverlegal",
        &emitted,
        "return cl_spec_cc_impl_receiverlegal_sink(x);",
    );
}

/// A null shape at a concrete destination keeps a single report.
///
/// The destination gate leaves a concrete destination to the position's own
/// rule, so the declaration and the argument each report exactly one `E10012`,
/// at the position's own span (the instance conversion recorded the same site
/// again before the gate, twice at the same span for the argument).
#[test]
fn should_not_duplicate_a_null_report_at_a_concrete_destination() {
    let decl = r#"module spec.cc.impl.nullconcretedecl;

func store<T>(x: T): i32 {
    let n = null;
    let y: i32 = n;
    return y;
}

func main(): i32 {
    let s: str = "abc";
    return store<str>(s);
}
"#;
    let stderr = reject_host("nullconcretedecl", decl);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "nullconcretedecl: expected one E10012; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'null' is not allowed here"),
        "nullconcretedecl: the null message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:5:5"),
        "nullconcretedecl: the report must keep the position's span; stderr:\n{stderr}"
    );

    let argument = r#"module spec.cc.impl.nullconcretearg;

func want(v: i32): i32 {
    return v;
}

func pass<T>(x: T): i32 {
    let n = null;
    return want(n);
}

func main(): i32 {
    let s: str = "abc";
    return pass<str>(s);
}
"#;
    let stderr = reject_host("nullconcretearg", argument);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "nullconcretearg: expected one E10012; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'null' is not allowed here"),
        "nullconcretearg: the null message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:9:17"),
        "nullconcretearg: the report must keep the position's span; stderr:\n{stderr}"
    );
}

// ===== The typed nulls and the captured store =====

/// A null-shaped value lowers to the destination view's own empty value.
///
/// The store, the argument, and the conditional carry the destination's zero
/// view (`cl_zero_ClStr()`), never the shape's own pointer (the accepted
/// programs previously failed the strict build with `incompatible types when
/// assigning to 'ClStr' from 'void *'` and `incompatible type for argument 1`).
#[test]
fn should_lower_a_null_shape_to_the_destination_view_value() {
    let store = r#"module spec.cc.impl.nullviewstore;

func main(): i32 {
    let x = null;
    let s: str? = x;
    if (s == null) {
        return 0;
    }
    return i32(len(s)) + 1;
}
"#;
    let emitted = emit_host("nullviewstore", store);
    assert_contains("nullviewstore", &emitted, "ClStr s = {0};");
    assert_contains("nullviewstore", &emitted, "s = cl_zero_ClStr();");
    assert_eq!(
        count(&emitted, "s = x;"),
        0,
        "nullviewstore: the store must not copy the shape's pointer"
    );

    let argument = r#"module spec.cc.impl.nullviewarg;

func show(s: str?): i32 {
    if (s == null) {
        return 0;
    }
    return i32(len(s)) + 1;
}

func main(): i32 {
    let x = null;
    return show(x);
}
"#;
    let emitted = emit_host("nullviewarg", argument);
    assert_contains(
        "nullviewarg",
        &emitted,
        "return cl_spec_cc_impl_nullviewarg_show(cl_zero_ClStr());",
    );

    // The conditional keeps its per-arm rendering: the null-shaped shortcut
    // never takes precedence over it (the same shape the residue suite pins).
    let conditional = r#"module spec.cc.impl.nullcondstore;

func fill(c: bool, borrow slot: str?): void {
    slot = c ? null : null;
}

func main(): i32 {
    let out: str? = null;
    fill(true, borrow out);
    if (out == null) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("nullcondstore", conditional);
    assert_contains(
        "nullcondstore",
        &emitted,
        "(*slot) = (c ? cl_zero_ClStr() : cl_zero_ClStr());",
    );
}

/// The hosted-amiga handle destination takes the profile's zero null.
///
/// A `File?` is the `BPTR` word on that ABI, so the store and the return
/// spell the integer `0`, never the shape's `void *` (the accepted program
/// previously failed the cross build with `assignment to 'BPTR' … from 'void
/// *'` and `returning 'void *' … 'BPTR'`).
#[test]
fn should_lower_a_null_shape_to_the_hosted_amiga_zero() {
    let source = r#"module spec.cc.impl.nullhandle;

import { File } from std.io;

func pass(): File? {
    let x = null;
    return x;
}

func main(): i32 {
    let x = null;
    let fl: File? = x;
    if (fl != null) {
        return 1;
    }
    let f2: File? = pass();
    if (f2 != null) {
        return 2;
    }
    return 0;
}
"#;
    let emitted = emit_hosted_amiga("nullhandle", source);
    assert_contains(
        "nullhandle",
        &emitted,
        "static BPTR cl_spec_cc_impl_nullhandle_pass(void)",
    );
    assert_eq!(
        count(&emitted, "fl = 0;"),
        4,
        "nullhandle: the hoisted init, the store, its drop, and the return \
         cleanup must each spell the integer 0"
    );
    assert_eq!(
        count(&emitted, "return x;"),
        0,
        "nullhandle: the return must not carry the shape's pointer"
    );
    assert_eq!(
        count(&emitted, "fl = x;"),
        0,
        "nullhandle: the store must not carry the shape's pointer"
    );
}

/// A writable capture stored into a nullable out-slot reports once.
///
/// The store would move the value out of the capture environment while the
/// caller still owns it, so the instance judgment reports the frozen capture
/// rule and the run no longer double-frees (the accepted program previously
/// aborted with `free(): double free detected` at exit 134). The non-capture
/// `take` store keeps its accepted shape.
#[test]
fn should_reject_a_writable_capture_stored_into_a_nullable_out_slot() {
    let source = r#"module spec.cc.impl.captureslot;

import { String, stringOf } from std.text;

func wrap<T>(borrow out: T, x: T): void {
    ([borrow x, borrow out] () => {
        out = x;
    })();
}

func main(): i32 {
    let out: String? = null;
    let s: String = stringOf("ab");
    wrap<String?>(borrow out, s);
    return 0;
}
"#;
    let stderr = reject_host("captureslot", source);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "captureslot: expected one E10401; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot move captured 'x' in a lambda"),
        "captureslot: the capture message is missing; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("E10034"),
        "captureslot: the lend-store report must be absent; stderr:\n{stderr}"
    );

    let control = r#"module spec.cc.impl.captureslotctl;

import { String, stringOf } from std.text;

func put<T>(borrow d: T, take s: T): void {
    d = s;
}

func main(): i32 {
    let d: String? = null;
    let s: String = stringOf("ab");
    put<String?>(borrow d, s);
    if (d == null) {
        return 1;
    }
    return i32(len(d)) + 1;
}
"#;
    let emitted = emit_host("captureslotctl", control);
    assert_contains(
        "captureslotctl",
        &emitted,
        "cl_put__StringQ(&(d), s);",
    );
    assert_contains("captureslotctl", &emitted, "(*d) = s;");
}

/// The typed-null lowering and the capture control re-emit identically.
#[test]
fn should_re_emit_the_null_lowering_and_the_capture_control_identically() {
    let store = r#"module spec.cc.impl.nullviewstore;

func main(): i32 {
    let x = null;
    let s: str? = x;
    if (s == null) {
        return 0;
    }
    return i32(len(s)) + 1;
}
"#;
    let (first, second) = emit_twice("reemitnullview", store);
    assert_eq!(
        first, second,
        "reemitnullview: the program re-emitted differently"
    );
    assert_contains("reemitnullview", &first, "s = cl_zero_ClStr();");

    let control = r#"module spec.cc.impl.captureslotctl;

import { String, stringOf } from std.text;

func put<T>(borrow d: T, take s: T): void {
    d = s;
}

func main(): i32 {
    let d: String? = null;
    let s: String = stringOf("ab");
    put<String?>(borrow d, s);
    if (d == null) {
        return 1;
    }
    return i32(len(d)) + 1;
}
"#;
    let (first, second) = emit_twice("reemitcapturectl", control);
    assert_eq!(
        first, second,
        "reemitcapturectl: the program re-emitted differently"
    );
    assert_contains("reemitcapturectl", &first, "(*d) = s;");
}

// ===== The null-shaped conditional arm and the capture view store =====

/// A null-shaped conditional arm stores the destination's empty value.
///
/// The conditional stores per arm at a non-duplicable handle destination, so
/// the arm's null shape takes the destination's `NULL` and the source is not
/// zeroed again: the hoisted declaration is the local's only `x = NULL;`
/// spelling (before the per-arm destination typing the arm copied `x` and the
/// strict build failed with `variable 'x' set but not used`).
#[test]
fn should_store_a_null_shaped_conditional_arm_as_the_destination_value() {
    let source = r#"module spec.cc.impl.nullcondhandle;

import { File } from std.io;

func main(): i32 {
    let c: bool = true;
    let x = null;
    let g: File? = c ? x : null;
    if (g == null) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("nullcondhandle", source);
    assert_eq!(
        count(&emitted, "g = NULL;"),
        4,
        "nullcondhandle: both arms, the hoisted initialization, and the drop \
         must each store the destination's null"
    );
    assert_eq!(
        count(&emitted, "x = NULL;"),
        2,
        "nullcondhandle: the declaration and the source's own initialization \
         spell the shape's zero; the arm's redundant zero is absent"
    );
    assert_eq!(
        count(&emitted, "g = x;"),
        0,
        "nullcondhandle: the arm must not copy the shape's pointer"
    );
}

/// A capture's view-widening store emits the view conversion.
///
/// The store copies the borrowed capture into the view, so it lowers through
/// the `String`-to-`str` widening and stays accepted.
#[test]
fn should_emit_the_capture_view_widening_store() {
    let source = r#"module spec.cc.impl.captureviewstore;

import { String, stringOf } from std.text;

func make<T>(x: T): i32 {
    return ([borrow x] () => {
        let y: str = "q";
        y = x;
        return i32(len(y));
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    return make<String>(s) + i32(len(s)) + 1;
}
"#;
    let emitted = emit_host("captureviewstore", source);
    assert_contains(
        "captureviewstore",
        &emitted,
        "y = cl_string_view((*__env->x));",
    );
}

// ===== The null-shaped array element at the destination value =====

/// A null-shaped array element stores the destination's empty value.
///
/// The element stores the destination's own null on the host and its integer
/// zero on the alternate profile, and the source local is marked unused
/// instead of being read (before the walk skipped the nested children, the
/// host strict build failed with `variable 'n' set but not used`).
#[test]
fn should_store_a_null_shaped_array_element_at_the_destination_value() {
    let host = r#"module spec.cc.impl.nullarrayhost;

import { File } from std.io;

func main(): i32 {
    let n = null;
    let xs: File[2] = [n, n];
    if (xs[0] == null && xs[1] == null) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("nullarrayhost", host);
    assert_contains("nullarrayhost", &emitted, "xs[0] = NULL;");
    assert_contains("nullarrayhost", &emitted, "xs[1] = NULL;");
    assert_eq!(
        count(&emitted, "xs[0] = n;") + count(&emitted, "xs[1] = n;"),
        0,
        "nullarrayhost: the elements must not copy the shape's pointer"
    );

    let amiga = r#"module spec.cc.impl.nullarrayamiga;

import { File } from std.io;

func main(): i32 {
    let n = null;
    let xs: File[2] = [n, n];
    if (xs[0] == null && xs[1] == null) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_hosted_amiga("nullarrayamiga", amiga);
    assert_contains("nullarrayamiga", &emitted, "xs[0] = 0;");
    assert_contains("nullarrayamiga", &emitted, "xs[1] = 0;");
    assert_eq!(
        count(&emitted, "xs[0] = n;") + count(&emitted, "xs[1] = n;"),
        0,
        "nullarrayamiga: the elements must not copy the shape's pointer"
    );
}

// ===== The projected-argument instance keys =====

/// Emits a two-file host program and asserts the front end accepted it.
///
/// A module initializer can name an imported binding as a dot call's
/// receiver, so the receiver's module is written beside the program and
/// passed on the same command line. Returns the emitted translation unit.
fn emit_host_pair(tag: &str, holder: &str, main: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
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
        "{tag}: the emitted C is empty"
    );
    emitted
}

/// Runs a two-file host program and asserts the front end rejected it.
///
/// The holder module and the program are written beside each other, as a build
/// passes them. Returns the diagnostics' text; a rejected program must not
/// write a translation unit.
fn reject_host_pair(tag: &str, holder: &str, main: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program wrote a translation unit"
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Emits a two-file program twice from the same source paths.
///
/// The holder and the program keep one path across both runs, so a translation
/// unit that differs only in its source locations still compares equal. Returns
/// both emitted texts.
fn emit_twice_pair(tag: &str, holder: &str, main: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
        let output = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            main_path.display().to_string(),
            holder_path.display().to_string(),
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

/// Asserts one keyed instance is declared, defined, and called exactly once.
///
/// `prefix` is the declaration prefix shared by every instance of the same
/// generic callee, `signature` is the wanted instance's C signature without
/// its trailing `;` or body, and `call` is its call expression. An underived
/// key misses the declaration and the definition; a wrongly derived key shows
/// up as a second instance under `prefix`.
fn assert_keyed_instance(tag: &str, emitted: &str, prefix: &str, signature: &str, call: &str) {
    assert_contains(tag, emitted, &format!("{signature};"));
    assert_contains(tag, emitted, &format!("{signature}\n{{"));
    assert_eq!(
        count(emitted, signature),
        2,
        "{tag}: expected exactly one declaration and one definition of {signature:?}"
    );
    assert_eq!(
        count(emitted, prefix),
        2,
        "{tag}: expected exactly one instance under {prefix:?}"
    );
    assert_eq!(
        count(emitted, call),
        1,
        "{tag}: expected exactly one call {call:?}"
    );
}

/// The holder module of the aliased-receiver programs.
const DOTRECEIVER_HOLDER_SOURCE: &str = r#"module spec.cc.impl.dotreceiverholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

/// The borrowed element's owning member argument program.
const TEMPROOTED_SOURCE: &str = r#"module spec.cc.impl.temprooted;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

let hits: i32 = 0;

func keep<T>(x: T): i32 {
    hits = hits + 1;
    return 1;
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + keep(v.name);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cd"), y: 2 }];
    return (each<S[2]>(a) - 2) + (hits - 2);
}
"#;

/// The conditional argument program.
const CONDITIONAL_SOURCE: &str = r#"module spec.cc.impl.conditional;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(a: T, b: T, c: bool): i32 {
    return id(c ? a.y : b.y);
}

func main(): i32 {
    let a: S = S { y: 1 };
    let b: S = S { y: 2 };
    return (pick<S>(a, b, true) - 1) + (pick<S>(a, b, false) - 2);
}
"#;

/// An aliased imported receiver of an initializer dot call declares its
/// instance.
///
/// `bb.pick(7)` fixes the call's type parameter from the receiver — the
/// holder's exported binding, named through an alias — so the emitter spells
/// the keyed instance `cl_pick__Box` and passes the resolved binding at
/// argument position zero. Before the receiver head resolved through the
/// module scope, the call had no declaration and no definition: the strict
/// build failed on an implicit declaration of `cl_pick__Box`.
#[test]
fn should_declare_an_aliased_receiver_dot_call_instance() {
    let aliased = r#"module spec.cc.impl.dotreceiver;

import { b as bb } from spec.cc.impl.dotreceiverholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let g: i32 = bb.pick(7);

func main(): i32 {
    return g - 7;
}
"#;
    let emitted = emit_host_pair("dotreceiver", DOTRECEIVER_HOLDER_SOURCE, aliased);
    assert_keyed_instance(
        "dotreceiver",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__Box(ClBox v, i32 w)",
        "cl_pick__Box(cl_spec_cc_impl_dotreceiverholder_b, 7)",
    );
}

/// A borrowed element's owning member passed to a generic call declares its
/// instance.
///
/// `keep(v.name)` inside the borrow loop reads the element through the
/// element materialization; the substituted member type fixes the key to the
/// owning `String` instance, which the emitter declares, defines, and calls.
/// Before the resolver followed the temporary root, the call had no
/// declaration and no definition: the strict build failed on an implicit
/// declaration of `cl_keep__String`.
#[test]
fn should_declare_a_temp_rooted_member_argument_instance() {
    let emitted = emit_host("temprooted", TEMPROOTED_SOURCE);
    assert_keyed_instance(
        "temprooted",
        &emitted,
        "static i32 cl_keep__",
        "static i32 cl_keep__String(ClString x)",
        "cl_keep__String((*__cl_tmp1).name)",
    );
}

/// A conditional argument whose arms agree declares one instance.
///
/// `id(c ? a.y : b.y)` reads both arms through the instance substitution;
/// they resolve to the same `i32`, so the selection keys one instance and the
/// call carries the condition. Before the resolver followed a
/// parameter-rooted place base, the call had no declaration and no
/// definition: the strict build failed on an implicit declaration of
/// `cl_id__i32`.
#[test]
fn should_declare_a_conditional_argument_instance() {
    let emitted = emit_host("conditional", CONDITIONAL_SOURCE);
    assert_keyed_instance(
        "conditional",
        &emitted,
        "static i32 cl_id__",
        "static i32 cl_id__i32(i32 x)",
        "cl_id__i32((c ? a.y : b.y))",
    );
}

/// The projected-argument controls keep their keyed instances.
///
/// The body twin, the unaliased initializer twin, the concrete-parameter
/// twin, and the by-value index twin already derived their keys before the
/// resolver changes, so these shapes hold on both sides — they guard the
/// controls against a regression rather than discriminate the fix.
#[test]
fn should_keep_the_projected_argument_control_instances() {
    let body = r#"module spec.cc.impl.dotreceiverbody;

import { b as bb } from spec.cc.impl.dotreceiverholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

func body(): i32 {
    return bb.pick(7);
}

let g: i32 = body();

func main(): i32 {
    return g - 7;
}
"#;
    let emitted = emit_host_pair("dotreceiverbody", DOTRECEIVER_HOLDER_SOURCE, body);
    assert_keyed_instance(
        "dotreceiverbody",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__Box(ClBox v, i32 w)",
        "cl_pick__Box(cl_spec_cc_impl_dotreceiverholder_b, 7)",
    );

    let direct = r#"module spec.cc.impl.dotreceiverdirect;

import { b } from spec.cc.impl.dotreceiverholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let g: i32 = b.pick(7);

func main(): i32 {
    return g - 7;
}
"#;
    let emitted = emit_host_pair("dotreceiverdirect", DOTRECEIVER_HOLDER_SOURCE, direct);
    assert_keyed_instance(
        "dotreceiverdirect",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__Box(ClBox v, i32 w)",
        "cl_spec_cc_impl_dotreceiverdirect_g = cl_pick__Box(cl_spec_cc_impl_dotreceiverholder_b, 7);",
    );

    let concrete = r#"module spec.cc.impl.tempconcrete;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func keep<T>(x: T): i32 {
    return 1;
}

func eachS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + keep(v.name);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cd"), y: 2 }];
    return eachS(a);
}
"#;
    let emitted = emit_host("tempconcrete", concrete);
    assert_keyed_instance(
        "tempconcrete",
        &emitted,
        "static i32 cl_keep__",
        "static i32 cl_keep__String(ClString x)",
        "cl_keep__String((*__cl_tmp1).name)",
    );

    let reference = r#"module spec.cc.impl.tempref;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func rowsum<T>(xs: T, i: usize): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + id(v[i].y);
    }
    return n;
}

func main(): i32 {
    let rows: S[2][3] = [
        [S { y: 1 }, S { y: 2 }, S { y: 3 }],
        [S { y: 4 }, S { y: 5 }, S { y: 6 }]
    ];
    let i: usize = 1;
    return rowsum<S[2][3]>(rows, i);
}
"#;
    let emitted = emit_host("tempref", reference);
    assert_keyed_instance(
        "tempref",
        &emitted,
        "static i32 cl_id__",
        "static i32 cl_id__i32(i32 x)",
        "cl_id__i32(v[i].y)",
    );
}

/// The projected-argument instances re-emit byte-identically.
///
/// The resolver derives every key from the recorded values, so a second
/// emission of the same source produces the same translation unit.
#[test]
fn should_re_emit_the_projected_argument_programs_identically() {
    let (first, second) = emit_twice("reemittemprooted", TEMPROOTED_SOURCE);
    assert_eq!(
        first, second,
        "reemittemprooted: the program re-emitted differently"
    );
    assert_contains(
        "reemittemprooted",
        &first,
        "cl_keep__String((*__cl_tmp1).name)",
    );

    let (first, second) = emit_twice("reemitconditional", CONDITIONAL_SOURCE);
    assert_eq!(
        first, second,
        "reemitconditional: the program re-emitted differently"
    );
    assert_contains(
        "reemitconditional",
        &first,
        "cl_id__i32((c ? a.y : b.y))",
    );
}

// ===== The receiver head's scope =====

/// The holder module of the aliased-receiver collision and shadow programs.
const ALIAS_RECEIVER_HOLDER_SOURCE: &str = r#"module spec.cc.impl.aliasreceiverholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

/// An initializer dot-call receiver keeps the module's own binding's key.
///
/// With the import renamed to `other` and the module's own `g`, the
/// initializer's `g.pick(7)` keys the module's own `i32`, so the emitted call
/// reaches a declared and defined instance. The import's `Box` must leave no
/// trace: a recorded instance of it would declare a specialization the
/// emitter never calls.
#[test]
fn should_declare_the_own_binding_at_an_initializer_dot_receiver() {
    let initializer = r#"module spec.cc.impl.aliascollision;

import { b as other } from spec.cc.impl.aliasreceiverholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let g: i32 = 3;

let h: i32 = g.pick(7) + other.n;

func main(): i32 {
    return (h - 12) + (g - 3);
}
"#;
    let emitted = emit_host_pair("aliascollision", ALIAS_RECEIVER_HOLDER_SOURCE, initializer);
    assert_keyed_instance(
        "aliascollision",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__i32(i32 v, i32 w)",
        "cl_pick__i32(cl_spec_cc_impl_aliascollision_g, 7)",
    );
    assert_eq!(
        count(&emitted, "cl_pick__Box"),
        0,
        "aliascollision: the import's type must not be recorded"
    );
}

/// A body binding that shadows an import alias leaves no instance behind.
///
/// The local `bb: i32` shadows the imported `bb`, so `bb.pick(7)` keys the
/// own `i32` instance, which the emitter declares, defines, and calls. The
/// import's struct type must not add a specialization: a recorded instance of
/// it would sit in the translation unit uncalled.
#[test]
fn should_not_record_a_shadowed_alias_receiver_in_a_body() {
    let shadowed = r#"module spec.cc.impl.shadowedalias;

import { b as bb } from spec.cc.impl.aliasreceiverholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

func run(): i32 {
    let bb: i32 = 5;
    return bb.pick(7);
}

func main(): i32 {
    return run() - 7;
}
"#;
    let emitted = emit_host_pair("shadowedalias", ALIAS_RECEIVER_HOLDER_SOURCE, shadowed);
    assert_keyed_instance(
        "shadowedalias",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__i32(i32 v, i32 w)",
        "cl_pick__i32(bb, 7)",
    );
    assert_eq!(
        count(&emitted, "cl_pick__Box"),
        0,
        "shadowedalias: the import's type must not add a dead instance"
    );
}

// ===== The lambda body's own scope =====

/// The holder module of the initializer-lambda programs.
const LAMBDA_HOLDER_SOURCE: &str = r#"module spec.cc.impl.exprholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

/// An initializer lambda whose parameter shadows an import leaves no phantom
/// instance.
///
/// `(bb: i32) => bb.pick(7)` binds `bb` to the lambda's parameter, so the
/// lifted body's call keys the `i32` instance, which the emitter declares,
/// defines, and calls — in the expression-bodied form and the block-bodied
/// twin alike. The import's struct type must not add a specialization the
/// emitter never calls.
#[test]
fn should_not_record_a_shadowed_lambda_parameter_receiver() {
    let expression = r#"module spec.cc.impl.exprlambdashadow;

import { b as bb } from spec.cc.impl.exprholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let f: func(i32): i32 = (bb: i32) => bb.pick(7);

func main(): i32 {
    return f(3) - 7;
}
"#;
    let emitted = emit_host_pair("exprlambdashadow", LAMBDA_HOLDER_SOURCE, expression);
    assert_keyed_instance(
        "exprlambdashadow",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__i32(i32 v, i32 w)",
        "cl_pick__i32(bb, 7)",
    );
    assert_eq!(
        count(&emitted, "cl_pick__Box"),
        0,
        "exprlambdashadow: the import's type must not add a dead instance"
    );

    let block = r#"module spec.cc.impl.blocklambdashadow;

import { b as bb } from spec.cc.impl.exprholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let f: func(i32): i32 = (bb: i32) => {
    return bb.pick(7);
};

func main(): i32 {
    return f(3) - 7;
}
"#;
    let emitted = emit_host_pair("blocklambdashadow", LAMBDA_HOLDER_SOURCE, block);
    assert_keyed_instance(
        "blocklambdashadow",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__i32(i32 v, i32 w)",
        "cl_pick__i32(bb, 7)",
    );
    assert_eq!(
        count(&emitted, "cl_pick__Box"),
        0,
        "blocklambdashadow: the import's type must not add a dead instance"
    );
}

// ===== The element store's drop and the element conversions =====

/// Emits a host program and asserts one report of `code` carries `fragment`.
///
/// The stderr must name the report exactly once and carry the fragment, so a
/// per-instance judgment is pinned the way the concrete twin reports it.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragment: &str) {
    let stderr = reject_host(tag, source);
    assert_eq!(
        stderr.matches(code).count(),
        1,
        "{tag}: expected one {code}; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(fragment),
        "{tag}: expected {fragment:?}; stderr:\n{stderr}"
    );
}

/// The borrowed element store program.
const ELEMENT_STORE_SOURCE: &str = r#"module spec.cc.impl.elemstore;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = stringOf("zzzz");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return each<S[2]>(a) - 8;
}
"#;

/// The concrete twin of the borrowed element store program.
const ELEMENT_STORE_CONTROL_SOURCE: &str = r#"module spec.cc.impl.elemstorectl;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func eachS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = stringOf("zzzz");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return eachS(a) - 8;
}
"#;

/// The loop element return over an `i32` slice.
const ELEMENT_RETURN_CONTROL_SOURCE: &str = r#"module spec.cc.impl.convret;

import { slice } from std.mem;

func first<T>(xs: T[]): i32 {
    for (borrow e in xs) {
        return e;
    }
    return 0;
}

func main(): i32 {
    let a: i32[3] = [7, 8, 9];
    let s: i32[] = slice(a, 0, 3);
    return first<i32>(s);
}
"#;

/// The loop element `let` over an `i32` slice.
const ELEMENT_LET_CONTROL_SOURCE: &str = r#"module spec.cc.impl.convlet;

import { slice } from std.mem;

func sum<T>(xs: T[]): i32 {
    let total: i32 = 0;
    for (borrow e in xs) {
        let y: i32 = e;
        total = total + y;
    }
    return total;
}

func main(): i32 {
    let a: i32[3] = [7, 8, 9];
    let s: i32[] = slice(a, 0, 3);
    return sum<i32>(s);
}
"#;

/// The conditional element member position over a concrete array.
const CONDITIONAL_MEMBER_CONTROL_SOURCE: &str = r#"module spec.cc.impl.convcond;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick(xs: S[3], i: usize, c: bool): i32 {
    return id(c ? xs[i] : xs[i]).y;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick(a, i, true);
}
"#;

/// The conditional element member argument over an opaque array.
const CONDITIONAL_ARGUMENT_CONTROL_SOURCE: &str = r#"module spec.cc.impl.convcondarg;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, c: bool): i32 {
    return id(c ? xs[0].y : xs[0].y);
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return pick<S[2]>(a, true);
}
"#;

/// The borrowed element store releases the owner it overwrites.
///
/// `v.name = stringOf("zzzz")` inside `each<T>`'s borrow loop overwrites the
/// element's owning `String`, so the emitted store releases the old owner and
/// empties the place first, exactly as the concrete twin's store does. Before
/// the destination's resolved type reached the drop decision, the release was
/// missing and the overwritten owner leaked.
#[test]
fn should_release_the_overwritten_owner_in_a_borrowed_element_store() {
    let store = "    if ((*__cl_tmp1).name) { FreeString((*__cl_tmp1).name); }\n    \
                 (*__cl_tmp1).name = NULL;\n    \
                 (*__cl_tmp1).name = cl_std_text_stringOf(cl_str(\"zzzz\"));";
    let emitted = emit_host("elemstore", ELEMENT_STORE_SOURCE);
    assert_contains("elemstore", &emitted, store);
    assert_eq!(
        count(
            &emitted,
            "(*__cl_tmp1).name = cl_std_text_stringOf(cl_str(\"zzzz\"));"
        ),
        1,
        "elemstore: expected one element store"
    );

    let emitted = emit_host("elemstorectl", ELEMENT_STORE_CONTROL_SOURCE);
    assert_contains("elemstorectl", &emitted, store);
}

/// A converted element reports the conversion once at each spelling.
///
/// A place, conditional, and direct element return, an element `let`, an
/// element argument, and the loop element return and `let` each substitute an
/// `S` element into a concrete `i32` destination, so each instance reports the
/// frozen conversion exactly once, as the same program with the array type
/// carried concretely reports it. Before the per-instance judgment, all seven
/// were accepted and the strict build failed on the C type mismatch.
#[test]
fn should_report_the_element_conversion_once_at_each_spelling() {
    let place_return = r#"module spec.cc.impl.elemconvplaceret;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, i: usize): i32 {
    return id(xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvplaceret",
        place_return,
        "error E10200:",
        "cannot convert 'S' to 'i32' implicitly",
    );

    let conditional_return = r#"module spec.cc.impl.elemconvcondret;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, i: usize, c: bool): i32 {
    return id(c ? xs[i] : xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i, true);
}
"#;
    assert_rejected_once(
        "elemconvcondret",
        conditional_return,
        "error E10200:",
        "cannot convert 'S' to 'i32' implicitly",
    );

    let direct_return = r#"module spec.cc.impl.elemconvdirectret;

struct S { y: i32; }

func pick<T>(xs: T, i: usize): i32 {
    return xs[i];
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvdirectret",
        direct_return,
        "error E10200:",
        "cannot convert 'S' to 'i32' implicitly",
    );

    let element_let = r#"module spec.cc.impl.elemconvlet;

struct S { y: i32; }

func pick<T>(xs: T, i: usize): i32 {
    let v: i32 = xs[i];
    return v;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvlet",
        element_let,
        "error E10200:",
        "cannot convert 'S' to 'i32' implicitly",
    );

    let element_argument = r#"module spec.cc.impl.elemconvarg;

struct S { y: i32; }

func use(x: i32): i32 { return x; }

func pick<T>(xs: T, i: usize): i32 {
    return use(xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvarg",
        element_argument,
        "error E10200:",
        "cannot convert 'S' to 'i32' implicitly",
    );

    let loop_return = r#"module spec.cc.impl.elemconvloopret;

struct S { y: i32; }

func first<T>(xs: T): i32 {
    for (v in xs) {
        return v;
    }
    return 0;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return first<S[3]>(a);
}
"#;
    assert_rejected_once(
        "elemconvloopret",
        loop_return,
        "error E10200:",
        "cannot convert 'S' to 'i32' implicitly",
    );

    let loop_let = r#"module spec.cc.impl.elemconvlooplet;

struct S { y: i32; }

func first<T>(xs: T): i32 {
    for (v in xs) {
        let w: i32 = v;
        return w;
    }
    return 0;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return first<S[3]>(a);
}
"#;
    assert_rejected_once(
        "elemconvlooplet",
        loop_let,
        "error E10200:",
        "cannot convert 'S' to 'i32' implicitly",
    );
}

/// The matching element positions keep their accepted instances.
///
/// The loop element return and `let` over an `i32` slice and the conditional
/// element member positions convert an element into its own type, so each
/// instance stays accepted and emits its keyed call. These shapes already held
/// before the per-instance judgment — they guard the accepted side of the
/// conversion duties rather than discriminate them.
#[test]
fn should_keep_the_matching_element_control_instances() {
    let emitted = emit_host("convret", ELEMENT_RETURN_CONTROL_SOURCE);
    assert_keyed_instance(
        "convret",
        &emitted,
        "static i32 cl_first__",
        "static i32 cl_first__i32(ClSlice_i32 xs)",
        "cl_first__i32(s)",
    );

    let emitted = emit_host("convlet", ELEMENT_LET_CONTROL_SOURCE);
    assert_keyed_instance(
        "convlet",
        &emitted,
        "static i32 cl_sum__",
        "static i32 cl_sum__i32(ClSlice_i32 xs)",
        "cl_sum__i32(s)",
    );

    let emitted = emit_host("convcond", CONDITIONAL_MEMBER_CONTROL_SOURCE);
    assert_contains("convcond", &emitted, "static ClS cl_id__S(ClS x);");
    assert_contains("convcond", &emitted, "static ClS cl_id__S(ClS x)\n{");
    assert_contains("convcond", &emitted, "return cl_id__S((c ? ");

    let emitted = emit_host("convcondarg", CONDITIONAL_ARGUMENT_CONTROL_SOURCE);
    assert_keyed_instance(
        "convcondarg",
        &emitted,
        "static i32 cl_id__",
        "static i32 cl_id__i32(i32 x)",
        "cl_id__i32((c ? ",
    );
    assert_contains(
        "convcondarg",
        &emitted,
        "static i32 cl_pick__arr2_S(ClS xs[2], bool c)",
    );
}

/// The element surfaces re-emit byte-identically.
///
/// The store's drop decision and the conversion duties read the substituted
/// instance types, so a second emission of the same source produces the same
/// translation unit.
#[test]
fn should_re_emit_the_element_surfaces_identically() {
    let (first, second) = emit_twice("reemitelemstore", ELEMENT_STORE_SOURCE);
    assert_eq!(
        first, second,
        "reemitelemstore: the program re-emitted differently"
    );
    assert_contains(
        "reemitelemstore",
        &first,
        "if ((*__cl_tmp1).name) { FreeString((*__cl_tmp1).name); }",
    );

    let (first, second) = emit_twice("reemitconvret", ELEMENT_RETURN_CONTROL_SOURCE);
    assert_eq!(
        first, second,
        "reemitconvret: the program re-emitted differently"
    );
    assert_contains("reemitconvret", &first, "cl_first__i32(s)");
}

// ===== The element place's moves =====

/// The borrowed element move-store program.
const ELEMENT_MOVE_SOURCE: &str = r#"module spec.cc.impl.elemmove;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func eachM<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        let s: String = stringOf("qq");
        v.name = s;
        total = total + i32(len(bytes(v.name)));
        v.name = stringOf("r");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return eachM<S[2]>(a) - 6;
}
"#;

/// The concrete twin of the borrowed element move-store program.
const ELEMENT_MOVE_CONTROL_SOURCE: &str = r#"module spec.cc.impl.elemmovectl;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func eachMS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        let s: String = stringOf("qq");
        v.name = s;
        total = total + i32(len(bytes(v.name)));
        v.name = stringOf("r");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return eachMS(a) - 6;
}
"#;

/// The borrowed element take-then-store program.
const ELEMENT_TAKE_SOURCE: &str = r#"module spec.cc.impl.elemtake;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func each<T>(xs: T): i32 {
    for (borrow v in xs) {
        eat(v.name);
        v.name = stringOf("zzzz");
        return i32(len(bytes(v.name)));
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    return each<S[1]>(a) - 4;
}
"#;

/// The top-level field twin of the take-then-store program.
const ELEMENT_TAKE_CONTROL_SOURCE: &str = r#"module spec.cc.impl.elemtakectl;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func main(): i32 {
    let a: S = S { name: stringOf("a") };
    eat(a.name);
    a.name = stringOf("zzzz");
    return i32(len(bytes(a.name))) - 4;
}
"#;

/// A named owner stored into a borrowed element empties the source.
///
/// `v.name = s` stores the `String` `s` into the element, so the emitted move
/// releases the element's replaced owner, stores, and empties `s` — the
/// sequence the concrete twin emits. Before the element destination was
/// routed as an owning store, the source stayed intact and the overwrite
/// release freed the moved value twice.
#[test]
fn should_empty_a_named_source_stored_into_a_borrowed_element() {
    let sequence = "    if ((*__cl_tmp1).name) { FreeString((*__cl_tmp1).name); }\n    \
                    (*__cl_tmp1).name = NULL;\n    \
                    (*__cl_tmp1).name = s;\n    \
                    s = NULL;";
    let emitted = emit_host("elemmove", ELEMENT_MOVE_SOURCE);
    assert_contains("elemmove", &emitted, sequence);

    let emitted = emit_host("elemmovectl", ELEMENT_MOVE_CONTROL_SOURCE);
    assert_contains("elemmovectl", &emitted, sequence);
}

/// A take argument of a borrowed element reports the partial-move limit.
///
/// `eat(v.name)` consumes a member of a borrowed element: the element view
/// never owns its element and its member is not a top-level field of an owned
/// local or `take` parameter, so the move is the v1 element-move limit and the
/// generic instance reports the same `E10026` its concrete twin reports — no
/// translation unit is written. The top-level field spelling keeps its
/// accepted move-and-empty shape, so the double free the formerly accepted
/// generic spelling caused is gone by rejection.
#[test]
fn should_report_a_borrowed_element_take_as_the_partial_move_limit() {
    let stderr = reject_host("elemtake", ELEMENT_TAKE_SOURCE);
    assert!(
        stderr.contains("E10026"),
        "elemtake: expected the element-move limit; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.starts_with("error "))
            .count(),
        1,
        "elemtake: expected one report; stderr:\n{stderr}"
    );

    // The top-level spelling already emptied its field before the element
    // places followed it, so this shape guards it unchanged.
    let emitted = emit_host("elemtakectl", ELEMENT_TAKE_CONTROL_SOURCE);
    assert_contains("elemtakectl", &emitted, "eat(a.name);");
    assert_contains("elemtakectl", &emitted, "a.name = NULL;");
}

/// The aliasing element store program.
const ELEMENT_ALIAS_SOURCE: &str = r#"module spec.cc.impl.elemalias;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = dup(borrow xs[0].name);
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab"), y: 1 }];
    return each<S[1]>(a) - 2;
}
"#;

/// The concrete twin of the aliasing element store program.
const ELEMENT_ALIAS_CONTROL_SOURCE: &str = r#"module spec.cc.impl.elemaliasctl;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func eachS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = dup(borrow xs[0].name);
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cdef"), y: 2 }];
    return eachS(a) - 4;
}
"#;

/// A store to a borrowed element saves it before an aliasing right-hand side.
///
/// `v.name = dup(borrow xs[0].name)` reads the target's storage through the
/// array root, so the emitted store saves the replaced value, evaluates the
/// right-hand side, stores, and releases the saved value — for the generic
/// spelling and the concrete pointer store alike. Before the aliasing read
/// deferred the release, the target was emptied first and the right-hand side
/// read the emptied storage.
#[test]
fn should_save_a_borrowed_element_before_an_aliasing_right_hand_side() {
    let save = "= (*__cl_tmp1).name;";
    let store = "(*__cl_tmp1).name = cl_spec_cc_impl_elemalias_dup(xs[0].name);";
    let emitted = emit_host("elemalias", ELEMENT_ALIAS_SOURCE);
    assert_contains("elemalias", &emitted, save);
    assert_contains("elemalias", &emitted, store);
    let save_at = emitted
        .find(save)
        .unwrap_or_else(|| panic!("elemalias: the element is never saved"));
    let store_at = emitted
        .find(store)
        .unwrap_or_else(|| panic!("elemalias: the aliasing store is missing"));
    assert!(
        save_at < store_at,
        "elemalias: the element must be saved before the aliasing store"
    );
    assert_eq!(
        count(&emitted, "__cl_save_"),
        4,
        "elemalias: expected the save, its release, and the emptied release slot"
    );

    let emitted = emit_host("elemaliasctl", ELEMENT_ALIAS_CONTROL_SOURCE);
    assert_contains("elemaliasctl", &emitted, save);
    assert_contains(
        "elemaliasctl",
        &emitted,
        "(*__cl_tmp1).name = cl_spec_cc_impl_elemaliasctl_dup(xs[0].name);",
    );
}

/// The local-container aliasing element store program.
const ELEMENT_LOCAL_SOURCE: &str = r#"module spec.cc.impl.elemlocal;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func each<T>(take v: T): i32 {
    let a: T = v;
    for (borrow e in a) {
        e.name = dup(borrow a[0].name);
    }
    return i32(len(bytes(a[0].name))) - 2;
}

func main(): i32 {
    let s: S[1] = [S { name: stringOf("ab"), y: 1 }];
    return each<S[1]>(s);
}
"#;

/// The concrete twin of the local-container aliasing store program.
const ELEMENT_LOCAL_CONTROL_SOURCE: &str = r#"module spec.cc.impl.elemlocalctl;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab"), y: 1 }];
    for (borrow v in a) {
        v.name = dup(borrow a[0].name);
    }
    return i32(len(bytes(a[0].name))) - 2;
}
"#;

/// A borrowed element of a local container saves before an aliasing store.
///
/// The element belongs to the local `a`, so `dup(borrow a[0].name)` reads the
/// target's storage through that local; the emitted store saves the replaced
/// value, evaluates the right-hand side, stores, and releases the saved value
/// — for the generic spelling and the concrete pointer store alike. Before
/// the local container was recognized, the target was emptied first and the
/// right-hand side read the emptied storage.
#[test]
fn should_save_a_local_container_element_before_an_aliasing_right_hand_side() {
    let save = "= (*__cl_tmp1).name;";
    let store = "(*__cl_tmp1).name = cl_spec_cc_impl_elemlocal_dup(a[0].name);";
    let emitted = emit_host("elemlocal", ELEMENT_LOCAL_SOURCE);
    assert_contains("elemlocal", &emitted, save);
    assert_contains("elemlocal", &emitted, store);
    let save_at = emitted
        .find(save)
        .unwrap_or_else(|| panic!("elemlocal: the element is never saved"));
    let store_at = emitted
        .find(store)
        .unwrap_or_else(|| panic!("elemlocal: the aliasing store is missing"));
    assert!(
        save_at < store_at,
        "elemlocal: the element must be saved before the aliasing store"
    );
    assert_eq!(
        count(&emitted, "__cl_save_"),
        4,
        "elemlocal: expected the save, its release, and the emptied release slot"
    );

    let emitted = emit_host("elemlocalctl", ELEMENT_LOCAL_CONTROL_SOURCE);
    assert_contains("elemlocalctl", &emitted, save);
    assert_contains(
        "elemlocalctl",
        &emitted,
        "(*__cl_tmp1).name = cl_spec_cc_impl_elemlocalctl_dup(a[0].name);",
    );
}

// ===== The sibling-fixed null argument and the inverted conversions =====

/// Asserts one report of `code` names `fragment` at `site`.
///
/// The site is the `line:column` suffix the compiler prints after the file
/// name, so a report that lands on a different use site fails.
fn assert_rejected_once_at_site(tag: &str, source: &str, code: &str, fragment: &str, site: &str) {
    let stderr = reject_host(tag, source);
    assert_eq!(
        stderr.matches(code).count(),
        1,
        "{tag}: expected one {code}; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(fragment),
        "{tag}: expected {fragment:?}; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(site),
        "{tag}: expected the report at {site:?}; stderr:\n{stderr}"
    );
}

/// The inferred `str?` sibling instance.
const NULLABLE_VIEW_SOURCE: &str = r#"module spec.cc.impl.inferrednullok;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func main(): i32 {
    let s: str? = null;
    let x = null;
    return pair(s, x) - 7;
}
"#;

/// The explicit `String?` sibling instance.
const NULLABLE_OWNED_SOURCE: &str = r#"module spec.cc.impl.inferrednulloke;

import { String, stringOf } from std.text;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    let x = null;
    return pair<String?>(s, x) - 7;
}
"#;

/// The legal instances of the inverted conversion routes.
const LEGAL_INSTANCES_SOURCE: &str = r#"module spec.cc.impl.instconvok;

import { String, stringOf, toCString } from std.text;

func pick<T>(): T {
    return 5;
}

func consume2<U>(c: U): i32 {
    return 1;
}

func convert<T>(x: T): i32 {
    return consume2<cstring>(x);
}

func sink<U>(x: U): i32 {
    return 1;
}

func outer<T, U>(a: T): i32 {
    return sink<U>(a);
}

func inner<T>(x: T): i32 {
    return 1;
}

func outer2<U>(c: cstring): i32 {
    return inner<U>(c);
}

func main(): i32 {
    let r: i32 = pick<i32>();
    if (r != 5) {
        return 1;
    }
    let c: cstring = toCString("ab");
    if (convert<cstring>(c) != 1) {
        return 2;
    }
    if (outer<i32, i32>(7) != 1) {
        return 3;
    }
    let s: str = "xy";
    if (outer<str, str>(s) != 1) {
        return 4;
    }
    let h: String = stringOf("abc");
    if (outer<String, str>(h) != 1) {
        return 5;
    }
    if (outer<cstring, cstring>(c) != 1) {
        return 6;
    }
    if (outer2<cstring>(c) != 1) {
        return 7;
    }
    return 0;
}
"#;

/// A null-shaped argument at a sibling-fixed parameter reports at its site.
///
/// `pair(1, x)` fixes the parameter concretely from its sibling, so the
/// null-shaped `x` reports the frozen null rule exactly as it would at a
/// concrete parameter — once per use site (`pair(x, x)` reports two), with the
/// explicit twin unchanged.
#[test]
fn should_report_the_sibling_fixed_null_argument_at_its_use_site() {
    let sibling_fixed = r#"module spec.cc.impl.inferrednull;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair(1, x);
}
"#;
    assert_rejected_once_at_site(
        "inferrednull",
        sibling_fixed,
        "error E10012:",
        "'null' is not allowed here",
        "main.clo:9:20",
    );

    let reversed = r#"module spec.cc.impl.inferrednullrev;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair(x, 1);
}
"#;
    assert_rejected_once_at_site(
        "inferrednullrev",
        reversed,
        "error E10012:",
        "'null' is not allowed here",
        "main.clo:9:17",
    );

    let two_uses = r#"module spec.cc.impl.inferrednullxx;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair(x, x);
}
"#;
    let stderr = reject_host("inferrednullxx", two_uses);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        2,
        "inferrednullxx: each null use reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:9:17") && stderr.contains("main.clo:9:20"),
        "inferrednullxx: the reports name both use sites; stderr:\n{stderr}"
    );

    let explicit = r#"module spec.cc.impl.inferrednulltwin;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair<i32>(1, x);
}
"#;
    assert_rejected_once_at_site(
        "inferrednulltwin",
        explicit,
        "error E10012:",
        "'null' is not allowed here",
        "main.clo:9:25",
    );
}

/// The inverted conversion routes report at the written value's site.
///
/// The return route, the callee route, the mixed pair, and the parameter
/// route each report one frozen conversion at their instance — the same code
/// and site their concrete twins report.
#[test]
fn should_report_the_inverted_conversion_routes_at_their_sites() {
    let return_route = r#"module spec.cc.impl.instconvret;

func pick<T>(): T {
    return 5;
}

func main(): i32 {
    let c: cstring = pick<cstring>();
    if (c == null) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once_at_site(
        "instconvret",
        return_route,
        "error E10200:",
        "cannot convert 'i32' to 'cstring' implicitly",
        "main.clo:4:12",
    );

    let argument_route = r#"module spec.cc.impl.instconvcallee;

func consume2<U>(c: U): i32 {
    return 0;
}

func convert<T>(x: T): i32 {
    return consume2<cstring>(x);
}

func main(): i32 {
    let s: str = "abc";
    return convert<str>(s);
}
"#;
    assert_rejected_once_at_site(
        "instconvcallee",
        argument_route,
        "error E10200:",
        "cannot convert 'str' to 'cstring' implicitly",
        "main.clo:8:30",
    );

    let caller_parameter = r#"module spec.cc.impl.instconvmixed;

func sink<U>(x: U): i32 {
    return 0;
}

func outer<T, U>(a: T): i32 {
    return sink<U>(a);
}

func main(): i32 {
    return outer<i32, cstring>(1) + 1;
}
"#;
    assert_rejected_once_at_site(
        "instconvmixed",
        caller_parameter,
        "error E10200:",
        "cannot convert 'i32' to 'cstring' implicitly",
        "main.clo:8:20",
    );

    let concrete_caller = r#"module spec.cc.impl.instconvparam;

import { toCString } from std.text;

func inner<T>(x: T): i32 {
    if (x == null) {
        return 1;
    }
    return 0;
}

func outer<U>(c: cstring): i32 {
    return inner<U>(c);
}

func main(): i32 {
    let c: cstring = toCString("x");
    return outer<raw u8>(c);
}
"#;
    assert_rejected_once_at_site(
        "instconvparam",
        concrete_caller,
        "error E10200:",
        "cannot convert 'cstring' to 'raw u8' implicitly",
        "main.clo:13:21",
    );
}

/// An inferred call whose concrete parameter receives a caller parameter.
///
/// `inner(x, x)` infers its type parameter from the caller's `raw u8` and
/// passes it to the concrete `cstring` parameter, so the instance reports the
/// frozen conversion at the written argument — the same code its direct twin
/// reports.
#[test]
fn should_report_the_inferred_concrete_parameter_conversion() {
    let source = r#"module spec.cc.impl.inferredconcrete;

import { toCString } from std.text;

func inner<T>(y: cstring, x: T): i32 {
    if (y == null) {
        return 1;
    }
    return 0;
}

func outer<U>(x: U): i32 {
    return inner(x, x);
}

func main(): i32 {
    let a: u8[4] = [65, 66, 67, 0];
    let p: raw u8 = unsafe &a[0];
    return outer<raw u8>(p);
}
"#;
    assert_rejected_once_at_site(
        "inferredconcrete",
        source,
        "error E10200:",
        "cannot convert 'raw u8' to 'cstring' implicitly",
        "main.clo:13:18",
    );
}

/// The legal instances of both rows keep their accepted emissions.
///
/// The nullable sibling instances resolve their own keyed calls, and the
/// legal inverted-route instances resolve theirs, so nothing about the new
/// judgments rejects a conforming pair.
#[test]
fn should_keep_the_nullable_and_legal_route_instances() {
    let emitted = emit_host("implinferrednullok", NULLABLE_VIEW_SOURCE);
    assert_contains("implinferrednullok", &emitted, "cl_pair__strQ");

    let emitted = emit_host("implinferrednulloke", NULLABLE_OWNED_SOURCE);
    assert_contains("implinferrednulloke", &emitted, "cl_pair__StringQ");

    let emitted = emit_host("implinstconvok", LEGAL_INSTANCES_SOURCE);
    assert_contains("implinstconvok", &emitted, "cl_pick__i32");
    assert_contains("implinstconvok", &emitted, "cl_consume2__cstring");
}

/// The accepted-instance programs re-emit byte-identically.
///
/// The new judgments read the instance's substituted types, so a second
/// emission of the same source produces the same text.
#[test]
fn should_re_emit_the_conversion_control_programs_identically() {
    let (first, second) = emit_twice("reemitnullroute", NULLABLE_VIEW_SOURCE);
    assert_eq!(
        first, second,
        "reemitnullroute: the program re-emitted differently"
    );
    assert_contains("reemitnullroute", &first, "cl_pair__strQ");

    let (first, second) = emit_twice("reemitlegalroute", LEGAL_INSTANCES_SOURCE);
    assert_eq!(
        first, second,
        "reemitlegalroute: the program re-emitted differently"
    );
    assert_contains("reemitlegalroute", &first, "cl_consume2__cstring");
}

/// The caller-parameter-resolved nullable sibling program.
const CALLER_PARAMETER_NULLABLE_SOURCE: &str = r#"module spec.cc.impl.parsibling;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func outer<U>(x: U): i32 {
    let n = null;
    return pair(x, n) - 7;
}

func main(): i32 {
    let s: str? = null;
    return outer<str?>(s);
}
"#;

/// The never-instantiated generic body program.
const CALLER_PARAMETER_NOINST_SOURCE: &str = r#"module spec.cc.impl.parsiblingnoinst;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func outer<U>(x: U): i32 {
    let n = null;
    return pair(x, n) - 7;
}

func main(): i32 {
    return 0;
}
"#;

/// A caller-parameter-resolved nullable instance keys its own copy.
///
/// The nullable instance of the sibling pair emits its keyed specialization,
/// while a body that is never instantiated emits no specialization at all —
/// the resolution is judged where the instance exists.
#[test]
fn should_key_the_caller_parameter_resolved_nullable_instance() {
    let emitted = emit_host("implparsibling", CALLER_PARAMETER_NULLABLE_SOURCE);
    assert_contains("implparsibling", &emitted, "cl_pair__strQ");

    let emitted = emit_host("implparsiblingnoinst", CALLER_PARAMETER_NOINST_SOURCE);
    assert_eq!(
        count(&emitted, "cl_pair__"),
        0,
        "implparsiblingnoinst: an un-instantiated body emits no specialization"
    );
}

// ===== Member typing on concrete paths =====

/// A view pointer member read casts at every raw destination.
///
/// The member's C field is a `const u8 *` while a `raw u8` destination holds
/// a plain byte pointer, so the local store, the `raw u8` return, the raw
/// struct-literal field, and the raw argument each spell the sanctioned cast;
/// the program re-emits byte-identically.
const RAW_CAST_SOURCE: &str = r#"module spec.cc.impl.castpin;

struct Holder {
    c: raw u8;
}

func sink(p: raw u8): i32 {
    unsafe {
        if (p == null) { return 1; }
        return i32(*p) - 97;
    }
}

func ptr(s: str): raw u8 {
    return s.ptr;
}

func main(): i32 {
    let s: str = "abc";
    let p: raw u8 = s.ptr;
    let h: Holder = Holder { c: s.ptr };
    if (ptr(s) == null) { return 2; }
    if (p == null) { return 3; }
    if (h.c == null) { return 4; }
    return sink(s.ptr);
}
"#;

#[test]
fn should_cast_the_view_pointer_member_at_every_raw_destination() {
    let emitted = emit_host("castpin", RAW_CAST_SOURCE);
    assert_contains("castpin", &emitted, "p = (u8 *)(s.ptr);");
    assert_contains("castpin", &emitted, "return (u8 *)(s.ptr);");
    assert_contains(
        "castpin",
        &emitted,
        "h = (ClHolder){ (u8 *)(s.ptr) };",
    );
    assert_contains(
        "castpin",
        &emitted,
        "cl_spec_cc_impl_castpin_sink((u8 *)(s.ptr))",
    );
    assert_eq!(
        count(&emitted, "(u8 *)("),
        4,
        "castpin: expected one cast per raw destination"
    );

    let (first, second) = emit_twice("castpin", RAW_CAST_SOURCE);
    assert_eq!(first, second, "castpin: the program re-emitted differently");
}

/// A generic instance's pointer member argument casts like a direct call.
///
/// The `id(v.ptr)` argument's recorded type is opaque in the generic body, so
/// the emitter resolves the callee's type argument from the substituted
/// argument values — the same resolution the call symbol uses — and the keyed
/// instance's call spells the member cast. The direct call stays the control.
#[test]
fn should_cast_a_generic_instances_pointer_member_argument() {
    let generic = r#"module spec.cc.impl.genargcast;

func id<T>(x: T): T {
    return x;
}

func each<T>(v: T): raw u8 {
    return id(v.ptr);
}

func main(): i32 {
    let s: str = "ab";
    let p: raw u8 = each<str>(s);
    unsafe {
        if (p == null) { return 1; }
        return i32(*p) - 97;
    }
}
"#;
    let emitted = emit_host("genargcast", generic);
    assert_contains("genargcast", &emitted, "cl_id__raw_u8((u8 *)(v.ptr))");

    let direct = r#"module spec.cc.impl.directargcast;

func sink(p: raw u8): i32 {
    unsafe {
        if (p == null) { return 1; }
        return i32(*p) - 97;
    }
}

func main(): i32 {
    let s: str = "abc";
    return sink(s.ptr);
}
"#;
    let emitted = emit_host("directargcast", direct);
    assert_contains(
        "directargcast",
        &emitted,
        "cl_spec_cc_impl_directargcast_sink((u8 *)(s.ptr))",
    );
}

/// A module initializer spells the same member conversions a store does.
///
/// The module-scope direct store of a view's `.ptr` at a raw destination and
/// at a `cstring` destination renders through the store conversion, so the
/// emitted initializer carries both casts; the explicit-cast control keeps
/// its own spelling.
#[test]
fn should_cast_the_module_initializer_pointer_member_store() {
    let source = r#"module spec.cc.impl.modstore;

let s: str = "abc";
let g: raw u8 = s.ptr;
let c: cstring = s.ptr;
let e: raw u8 = unsafe raw u8(s.ptr);

func main(): i32 {
    return 0;
}
"#;
    let emitted = emit_host("modstore", source);
    assert_contains(
        "modstore",
        &emitted,
        "cl_spec_cc_impl_modstore_g = (u8 *)(cl_spec_cc_impl_modstore_s.ptr);",
    );
    assert_contains(
        "modstore",
        &emitted,
        "cl_spec_cc_impl_modstore_c = (char *)(cl_spec_cc_impl_modstore_s.ptr);",
    );
    assert_contains(
        "modstore",
        &emitted,
        "cl_spec_cc_impl_modstore_e = (u8 *)(cl_spec_cc_impl_modstore_s.ptr);",
    );
}

/// An unknown member on a concrete base reports at the base's span.
///
/// The fixed array, the `cstring`, and the `raw E` each report the registered
/// unknown-member rule with the base as written and no translation unit
/// written.
#[test]
fn should_report_unknown_members_at_the_base_span() {
    let fixed_array = r#"module spec.cc.impl.fixedmem;

func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let p = a.ptr;
    return 0;
}
"#;
    let stderr = reject_host("fixedmem", fixed_array);
    assert!(stderr.contains("'ptr' is not a member of 'u8[4]'"), "fixedmem: {stderr}");
    assert!(stderr.contains("main.clo:5:13"), "fixedmem: {stderr}");

    let c_string = r#"module spec.cc.impl.cstringmem;

import { toCString } from std.text;

func main(): i32 {
    let c: cstring = toCString("abc");
    let p = c.ptr;
    return 0;
}
"#;
    let stderr = reject_host("cstringmem", c_string);
    assert!(stderr.contains("'ptr' is not a member of 'cstring'"), "cstringmem: {stderr}");
    assert!(stderr.contains("main.clo:7:13"), "cstringmem: {stderr}");

    let raw_enum = r#"module spec.cc.impl.rawenumm;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.A;
    let p: raw E = unsafe &e;
    unsafe {
        return i32(p.A);
    }
}
"#;
    let stderr = reject_host("rawenumm", raw_enum);
    assert!(stderr.contains("'A' is not a member of 'raw E'"), "rawenumm: {stderr}");
    assert!(stderr.contains("main.clo:9:20"), "rawenumm: {stderr}");
}

// ===== Call identity and written arguments =====

/// A dot call's written argument is judged at the parameter the receiver fixes.
///
/// The receiver supplies the callee's first argument, so the written arguments
/// must convert to the type it fixes; the report names the written value, not
/// the receiver. The take and borrow spellings each report once at their own
/// argument, the generic caller reports at the value it passes, and the direct
/// inferred and explicit twins keep their existing codes and sites. The
/// matching take dot call is accepted and emits its keyed instance.
#[test]
fn should_report_a_dot_call_written_argument_at_its_resolved_parameter() {
    let take_dot = r#"module spec.cc.impl.dotwrittenarg;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let g: i32 = b.pick(7);
    return g - 1;
}
"#;
    assert_rejected_once_at_site(
        "dotwrittenarg",
        take_dot,
        "error E10200:",
        "cannot convert 'i32' to 'Box' implicitly",
        "main.clo:11:25",
    );

    let borrow_dot = r#"module spec.cc.impl.dotwrittenborrow;

struct Box { x: i32; }

func put<T>(borrow out: T, v: T): void {
    out = v;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    b.put(7);
    return b.x - 1;
}
"#;
    assert_rejected_once_at_site(
        "dotwrittenborrow",
        borrow_dot,
        "error E10200:",
        "cannot convert 'i32' to 'Box' implicitly",
        "main.clo:11:11",
    );

    let generic_i32 = r#"module spec.cc.impl.dotgenerici32;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func outer<T>(b: Box, c: T): i32 {
    return b.pick(c);
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    return outer<i32>(b, 7) - 1;
}
"#;
    assert_rejected_once_at_site(
        "dotgenerici32",
        generic_i32,
        "error E10200:",
        "cannot convert 'i32' to 'Box' implicitly",
        "main.clo:10:19",
    );

    // The direct twins already judged their written argument against the
    // inferred or explicit type argument; they keep the same code and site.
    let direct_inferred = r#"module spec.cc.impl.dotwritteninferred;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let g: i32 = pick(b, 7);
    return g - 1;
}
"#;
    assert_rejected_once_at_site(
        "dotwritteninferred",
        direct_inferred,
        "error E10200:",
        "cannot convert 'i32' to 'Box' implicitly",
        "main.clo:11:26",
    );

    let direct_explicit = r#"module spec.cc.impl.dotwrittenexplicit;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let g: i32 = pick<Box>(b, 7);
    return g - 1;
}
"#;
    assert_rejected_once_at_site(
        "dotwrittenexplicit",
        direct_explicit,
        "error E10200:",
        "cannot convert 'i32' to 'Box' implicitly",
        "main.clo:11:31",
    );

    // A matching receiver and argument emit the instance the receiver fixes.
    let matching = r#"module spec.cc.impl.dotcontrol;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 2;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let c: Box = Box { x: 2 };
    let g: i32 = b.pick(c);
    return g - 2;
}
"#;
    let emitted = emit_host("dotcontrol", matching);
    assert_keyed_instance(
        "dotcontrol",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__Box(ClBox v, ClBox w)",
        "cl_pick__Box(b, c)",
    );
}

/// A generic caller whose written argument matches the receiver is accepted.
///
/// The dot call fixes the callee's parameter from the receiver, so `c: Box`
/// matches `w: Box` even though the enclosing function's own type parameter is
/// the caller's `i32` instance. Judging the callee's parameter against the
/// caller's instance rejected both the receiver and the argument; the call
/// site's own inference leaves this caller clean and keys the `Box` instance.
#[test]
fn should_accept_a_generic_caller_whose_argument_matches_the_receiver() {
    let generic_box = r#"module spec.cc.impl.dotgenericbox;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func outer<T>(b: Box, c: Box): i32 {
    return b.pick(c);
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let c: Box = Box { x: 2 };
    return outer<i32>(b, c) - 1;
}
"#;
    let emitted = emit_host("dotgenericbox", generic_box);
    assert_keyed_instance(
        "dotgenericbox",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__Box(ClBox v, ClBox w)",
        "cl_pick__Box(b, c)",
    );
}

/// The holder module the declaration/import collision programs import from.
///
/// It exports `v` for the renamed shapes and `g` for the direct spelling, so
/// the colliding local name is always `g`.
const DECL_COLLISION_HOLDER: &str = r#"module spec.cc.impl.declcollisionholder;

export let v: i32 = 5;
export let g: i32 = 5;
"#;

/// Builds a module that binds `g` through an import and its own declaration.
///
/// The import renames `v` to `g`, so the report's site is the imported source
/// name while its message names the local binding. Returns the program text
/// and the one-based line of the import statement.
fn declaration_collision_program(
    module: &str,
    declaration: &str,
    import_first: bool,
) -> (String, usize) {
    let import = "import { v as g } from spec.cc.impl.declcollisionholder;";
    let main = "func main(): i32 {\n    return 0;\n}";
    if import_first {
        (
            format!("module {module};\n\n{import}\n\n{declaration}\n\n{main}\n"),
            3,
        )
    } else {
        let import_line = declaration.lines().count() + 4;
        (
            format!("module {module};\n\n{declaration}\n\n{import}\n\n{main}\n"),
            import_line,
        )
    }
}

/// Asserts one `E11103` names `fragment` at `site` and no duplicate declaration
/// is reported.
fn assert_collision_report(tag: &str, stderr: &str, fragment: &str, site: &str) {
    assert_eq!(
        stderr.matches("error E11103:").count(),
        1,
        "{tag}: expected one E11103; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(fragment),
        "{tag}: expected {fragment:?}; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(site),
        "{tag}: expected the report at {site:?}; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("E11101"),
        "{tag}: the declaration stays a single declaration; stderr:\n{stderr}"
    );
}

/// One name bound by both an import and a declaration is the collision report.
///
/// The import pass reports the local binding once, at the import item's name,
/// for every declaration kind and in either order; the module declaration is
/// not a duplicate declaration. The renamed-import control is accepted and its
/// read still reaches the holder's binding.
#[test]
fn should_report_a_declaration_import_collision_at_the_import_item() {
    let direct = r#"module spec.cc.impl.declcollisiondirect;

import { g } from spec.cc.impl.declcollisionholder;

let g: i32 = 7;

func main(): i32 {
    return g - 7;
}
"#;
    let stderr = reject_host_pair("declcollision-direct", DECL_COLLISION_HOLDER, direct);
    assert_collision_report(
        "declcollision-direct",
        &stderr,
        "duplicate import of 'g'",
        "main.clo:3:10",
    );

    // Every declaration kind the module loader registers, in both orders. A
    // `comptime` declaration also reports its staged-feature code; the
    // collision is the finding under test, so only that code is pinned.
    let kinds: &[(&str, &str)] = &[
        ("let", "let g: i32 = 7;"),
        ("func", "func g(): i32 { return 7; }"),
        ("struct", "struct g { x: i32; }"),
        ("const", "const g: i32 = 7;"),
        ("enum", "enum g : u8 { A, B }"),
        (
            "handle",
            r#"ffi hlib {
    func Close(take p: g): void;
}
handle g freedBy Close;"#,
        ),
        ("ffi-block", "ffi g { func h(x: i32): i32; }"),
        ("ffi-func", "ffi hlib { func g(x: i32): i32; }"),
        ("intrinsic", "intrinsic func g(): i32;"),
        ("comptime", "comptime func g(): i32 { return 7; }"),
    ];
    for (index, (kind, declaration)) in kinds.iter().enumerate() {
        for import_first in [true, false] {
            let order = if import_first { "importfirst" } else { "declfirst" };
            let tag = format!("declcollision-{kind}-{order}");
            let module = format!(
                "spec.cc.impl.declcollision{index}{}",
                if import_first { "i" } else { "d" }
            );
            let (main, import_line) =
                declaration_collision_program(&module, declaration, import_first);
            let stderr = reject_host_pair(&tag, DECL_COLLISION_HOLDER, &main);
            let site = format!("main.clo:{import_line}:10");
            assert_collision_report(&tag, &stderr, "duplicate import of 'g'", &site);
        }
    }

    let control = r#"module spec.cc.impl.declcollisioncontrol;

import { v as g } from spec.cc.impl.declcollisionholder;

let h: i32 = 2;

func main(): i32 {
    return g + h - 7;
}
"#;
    let emitted = emit_host_pair("declcollision-control", DECL_COLLISION_HOLDER, control);
    assert_contains(
        "declcollision-control",
        &emitted,
        "cl_spec_cc_impl_declcollisionholder_v",
    );
}

// ===== The direct-name shadow controls =====

/// The holder module the direct-name shadow programs import from.
///
/// The exported `b` is a `Box`, so a shadow that resolved to the import would
/// instantiate the iteration over a non-iterable struct instead of the
/// body's fixed array.
const SHADOW_CONTROL_HOLDER: &str = r#"module spec.cc.impl.shadowholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

/// The direct-name shadow controls keep their own instance and no dead one.
///
/// A local, a parameter, a generic-body local, and an expression- or
/// block-bodied lambda parameter may shadow an import, and the innermost scope
/// decides the dot receiver. Each fixture emits the array instance it actually
/// calls, emits no instance for the shadowed import's struct, and re-emits
/// byte-identically. The no-import baseline pins the same loop shape without a
/// shadow. These shapes already held before the phase's change, so they are
/// controls rather than discriminators.
#[test]
fn should_keep_the_direct_name_shadow_controls_without_dead_instances() {
    let local = r#"module spec.cc.impl.shadowlocal;

import { b } from spec.cc.impl.shadowholder;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) { n = n + i32(v); }
    return n;
}

func body(): i32 {
    let b: i32[2] = [3, 4];
    return b.each();
}

let g: i32 = body();

func main(): i32 {
    return g - 7;
}
"#;
    let (first, second) = emit_twice_pair("shadowlocal", SHADOW_CONTROL_HOLDER, local);
    assert_eq!(
        first, second,
        "shadowlocal: the program re-emitted differently"
    );
    assert_keyed_instance(
        "shadowlocal",
        &first,
        "static i32 cl_each__",
        "static i32 cl_each__arr2_i32(i32 xs[2])",
        "cl_each__arr2_i32(b)",
    );
    assert_eq!(
        count(&first, "cl_each__Box"),
        0,
        "shadowlocal: the shadowed import must add no instance"
    );

    let parameter = r#"module spec.cc.impl.shadowparam;

import { b } from spec.cc.impl.shadowholder;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) { n = n + i32(v); }
    return n;
}

func body(b: i32[2]): i32 {
    return b.each();
}

func main(): i32 {
    let a: i32[2] = [3, 4];
    return body(a) - 7;
}
"#;
    let (first, second) = emit_twice_pair("shadowparam", SHADOW_CONTROL_HOLDER, parameter);
    assert_eq!(
        first, second,
        "shadowparam: the program re-emitted differently"
    );
    assert_keyed_instance(
        "shadowparam",
        &first,
        "static i32 cl_each__",
        "static i32 cl_each__arr2_i32(i32 xs[2])",
        "cl_each__arr2_i32(b)",
    );
    assert_eq!(
        count(&first, "cl_each__Box"),
        0,
        "shadowparam: the shadowed import must add no instance"
    );

    let generic = r#"module spec.cc.impl.shadowgeneric;

import { b } from spec.cc.impl.shadowholder;

func drain<T>(xs: T): i32 { let n: i32 = 0; for (v in xs) { n = n + 1; } return n; }

func run<T>(x: T): i32 { let b: i32[2] = [3, 4]; return b.drain() + 0; }

func main(): i32 { let a: i32[2] = [3, 4]; return run<i32[2]>(a) - 2; }
"#;
    let (first, second) = emit_twice_pair("shadowgeneric", SHADOW_CONTROL_HOLDER, generic);
    assert_eq!(
        first, second,
        "shadowgeneric: the program re-emitted differently"
    );
    assert_keyed_instance(
        "shadowgeneric",
        &first,
        "static i32 cl_drain__",
        "static i32 cl_drain__arr2_i32(i32 xs[2])",
        "cl_drain__arr2_i32(b)",
    );
    assert_eq!(
        count(&first, "cl_drain__Box"),
        0,
        "shadowgeneric: the shadowed import must add no instance"
    );

    let lambda_expression = r#"module spec.cc.impl.shadowlambdaexpr;

import { b } from spec.cc.impl.shadowholder;

func drain<T>(xs: T): i32 { let n: i32 = 0; for (v in xs) { n = n + 1; } return n; }

let f: func(i32[2]): i32 = (b: i32[2]) => b.drain();

func main(): i32 { let a: i32[2] = [3, 4]; return f(a) - 2; }
"#;
    let (first, second) =
        emit_twice_pair("shadowlambdaexpr", SHADOW_CONTROL_HOLDER, lambda_expression);
    assert_eq!(
        first, second,
        "shadowlambdaexpr: the program re-emitted differently"
    );
    assert_keyed_instance(
        "shadowlambdaexpr",
        &first,
        "static i32 cl_drain__",
        "static i32 cl_drain__arr2_i32(i32 xs[2])",
        "cl_drain__arr2_i32(b)",
    );
    assert_eq!(
        count(&first, "cl_drain__Box"),
        0,
        "shadowlambdaexpr: the shadowed import must add no instance"
    );

    let lambda_block = r#"module spec.cc.impl.shadowlambdablock;

import { b } from spec.cc.impl.shadowholder;

func drain<T>(xs: T): i32 { let n: i32 = 0; for (v in xs) { n = n + 1; } return n; }

let f: func(i32[2]): i32 = (b: i32[2]) => { return b.drain(); };

func main(): i32 { let a: i32[2] = [3, 4]; return f(a) - 2; }
"#;
    let (first, second) = emit_twice_pair("shadowlambdablock", SHADOW_CONTROL_HOLDER, lambda_block);
    assert_eq!(
        first, second,
        "shadowlambdablock: the program re-emitted differently"
    );
    assert_keyed_instance(
        "shadowlambdablock",
        &first,
        "static i32 cl_drain__",
        "static i32 cl_drain__arr2_i32(i32 xs[2])",
        "cl_drain__arr2_i32(b)",
    );
    assert_eq!(
        count(&first, "cl_drain__Box"),
        0,
        "shadowlambdablock: the shadowed import must add no instance"
    );

    let baseline = r#"module spec.cc.impl.shadowcontrol;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) { n = n + i32(v); }
    return n;
}

func body(): i32 {
    let b: i32[2] = [3, 4];
    return b.each();
}

func main(): i32 {
    return body() - 7;
}
"#;
    let (first, second) = emit_twice("shadowcontrol", baseline);
    assert_eq!(
        first, second,
        "shadowcontrol: the program re-emitted differently"
    );
    assert_keyed_instance(
        "shadowcontrol",
        &first,
        "static i32 cl_each__",
        "static i32 cl_each__arr2_i32(i32 xs[2])",
        "cl_each__arr2_i32(b)",
    );
}

// ===== Qualified call identity and the written-value span =====

/// A self-qualified `ffi` call spells the declaration's symbol and keeps its
/// modes.
///
/// The symbol is the `ffi` declaration's own simple name, never the dotted path
/// the call writes. A `take` initializer call hands the value to the callee and
/// binds no owner for it, so the initializer releases nothing after the call; a
/// `take` body argument is moved out of the caller's slot; a `borrow` body call
/// leaves the caller's release in place.
#[test]
fn should_emit_a_self_qualified_ffi_symbol_and_take_mode() {
    let initializer = r#"module spec.cc.impl.selfffitake;

import { String, stringOf } from std.text;

ffi own {
    func probe_free_len(take s: String): i32;
}

let g: i32 = spec.cc.impl.selfffitake.probe_free_len(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("selfffitake", initializer);
    assert_contains(
        "selfffitake",
        &emitted,
        "cl_spec_cc_impl_selfffitake_g = probe_free_len(cl_std_text_stringOf(cl_str(\"abc\")));",
    );
    assert_eq!(
        count(&emitted, "FreeString(__cl_itmp0)"),
        0,
        "selfffitake: the callee owns a taken value, so the initializer must not \
         release it; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "spec.cc.impl.selfffitake.probe_free_len"),
        0,
        "selfffitake: the call must spell the declaration's simple symbol; emitted C:\n{emitted}"
    );

    let body = r#"module spec.cc.impl.selfffibody;

import { String, stringOf } from std.text;

ffi own {
    func probe_free_len2(take s: String): i32;
    func cl_string_len(borrow s: String): usize;
}

func main(): i32 {
    let n: i32 = spec.cc.impl.selfffibody.probe_free_len2(stringOf("abc"));
    let m: usize = spec.cc.impl.selfffibody.cl_string_len(stringOf("abcd"));
    return (n - 3) + (i32(m) - 4);
}
"#;
    let emitted = emit_host("selfffibody", body);
    assert_contains("selfffibody", &emitted, "probe_free_len2(__cl_owned0)");
    assert_contains("selfffibody", &emitted, "m = cl_string_len(__cl_owned1);");
    assert_eq!(
        count(&emitted, "spec.cc.impl.selfffibody."),
        0,
        "selfffibody: the calls must spell the declarations' simple symbols; emitted C:\n{emitted}"
    );
    let call = emitted
        .find("probe_free_len2(__cl_owned0)")
        .unwrap_or_else(|| panic!("selfffibody: the take call is missing; emitted C:\n{emitted}"));
    assert!(
        emitted[call..]
            .lines()
            .take(4)
            .any(|line| line.contains("__cl_owned0 = NULL;")),
        "selfffibody: the taken argument is emptied after the call; emitted C:\n{emitted}"
    );
}

/// A self-qualified inferred generic call declares its instance once.
///
/// The inferred call's key is the same one a plain call derives, so the
/// translation unit declares and defines the instance it calls. The generic
/// body's qualified call keys its own instance the same way.
#[test]
fn should_declare_a_self_qualified_inferred_generic_instance() {
    let direct = r#"module spec.cc.impl.selfgeneric;

func pick<T>(take a: T, take b: T): i32 {
    return 1;
}

func main(): i32 {
    let a: i32 = 5;
    let b: i32 = 6;
    return spec.cc.impl.selfgeneric.pick(a, b) - 1;
}
"#;
    let emitted = emit_host("implselfgeneric", direct);
    assert_keyed_instance(
        "implselfgeneric",
        &emitted,
        "static i32 cl_pick__",
        "static i32 cl_pick__i32(i32 a, i32 b)",
        "cl_pick__i32(a, b)",
    );

    let relayed = r#"module spec.cc.impl.selfgenericbody;

func sink<T>(take v: T): i32 {
    return 1;
}

func relay<T>(take v: T): i32 {
    return spec.cc.impl.selfgenericbody.sink(v);
}

func main(): i32 {
    return relay<i32>(5) - 1;
}
"#;
    let emitted = emit_host("implselfgenericbody", relayed);
    assert_keyed_instance(
        "implselfgenericbody",
        &emitted,
        "static i32 cl_sink__",
        "static i32 cl_sink__i32(i32 v)",
        "cl_sink__i32(v)",
    );
}

/// A module-path binding receiver calls its member through the binding.
///
/// `m.n.f(...)` names the module's binding `n` as the receiver and the free
/// function `f` as the callee, so the emitter spells the function's symbol and
/// passes the binding's place first. The initializer form lowers the same way.
#[test]
fn should_emit_a_module_path_binding_member_call() {
    let body = r#"module spec.cc.impl.globaldotbody;

struct N { v: i32; }

func peek(borrow n: N, x: i32): i32 {
    return n.v + x;
}

let n: N = N { v: 1 };

func main(): i32 {
    return spec.cc.impl.globaldotbody.n.peek(5) - 6;
}
"#;
    let emitted = emit_host("globaldotbody", body);
    assert_contains(
        "globaldotbody",
        &emitted,
        "cl_spec_cc_impl_globaldotbody_peek(cl_spec_cc_impl_globaldotbody_n, 5)",
    );
    assert_eq!(
        count(&emitted, "spec.cc.impl.globaldotbody."),
        0,
        "globaldotbody: the call must not spell the qualified path; emitted C:\n{emitted}"
    );

    let initializer = r#"module spec.cc.impl.globaldotinit;

struct N { v: i32; }

func peek(borrow n: N, x: i32): i32 {
    return n.v + x;
}

let n: N = N { v: 1 };

let g: i32 = spec.cc.impl.globaldotinit.n.peek(5);

func main(): i32 {
    return g - 6;
}
"#;
    let emitted = emit_host("globaldotinit", initializer);
    assert_contains(
        "globaldotinit",
        &emitted,
        "cl_spec_cc_impl_globaldotinit_g = cl_spec_cc_impl_globaldotinit_peek(",
    );
    assert_contains(
        "globaldotinit",
        &emitted,
        "cl_spec_cc_impl_globaldotinit_peek(cl_spec_cc_impl_globaldotinit_n, 5)",
    );
    assert_eq!(
        count(&emitted, "spec.cc.impl.globaldotinit.n.peek"),
        0,
        "globaldotinit: the call must not spell the qualified path; emitted C:\n{emitted}"
    );
}

/// A pending dot-call argument reports at its written value.
///
/// The `take` marker in front of a pending argument is not part of the judged
/// value: the report starts at the value, exactly as the direct call's report
/// starts there.
#[test]
fn should_report_a_marked_dot_argument_at_its_written_value() {
    let source = r#"module spec.cc.impl.markedpending;

struct S { x: i32; }

func g<T>(take s: S, take v: T, take w: T): i32 {
    return 1;
}

func outer<T>(s: S): i32 {
    return s.g(take 7, take "x");
}

func main(): i32 {
    let s: S = S { x: 0 };
    return outer<i32>(s) - 1;
}
"#;
    let stderr = reject_host("implmarkedpending", source);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "implmarkedpending: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'static str' to 'i32' implicitly"),
        "implmarkedpending: expected the conversion message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:10:29"),
        "implmarkedpending: expected the report at the written value; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("main.clo:10:24"),
        "implmarkedpending: the report must not start at the marker; stderr:\n{stderr}"
    );
}

// ===== Substitution edges: conditional judgments, element keys, and element members =====

/// A conditional whose arms still name type parameters is judged at its instance.
///
/// The declaration cannot decide whether the two arms agree, so the site is
/// recorded and re-judged once per concrete instantiation. The report is the
/// concrete twin's: one conversion at the else arm, naming the substituted arm
/// types and carrying no translation unit. At the base compiler the program is
/// accepted and the emitter writes a C conditional whose arms have different
/// types.
#[test]
fn should_report_an_opaque_conditional_at_its_else_arm() {
    let source = r#"module spec.cc.impl.condopaqueinstance;

func id<T>(x: T): T { return x; }

func choose<T, U>(c: bool, a: T, b: U): i32 {
    return i32(id(c ? a : b));
}

func main(): i32 {
    return choose<i32, str>(true, 3, "x") - 3;
}
"#;
    assert_rejected_once_at_site(
        "implcondopaque",
        source,
        "error E10200:",
        "cannot convert 'str' to 'i32' implicitly",
        "main.clo:6:27",
    );
}

/// A null arm at a non-nullable substituted destination reports at the null.
///
/// The generic callee fixes the argument's parameter to `i32` after the
/// argument has been typed, so the conditional's null arm is judged when the
/// parameter resolves. The report is the frozen null rule at the `null`, and no
/// conversion report is added. At the base compiler the program is accepted
/// and the emitter writes a pointer/integer conditional.
#[test]
fn should_report_an_opaque_conditional_null_arm_at_the_null() {
    let source = r#"module spec.cc.impl.condnullarminstance;

func id<T>(x: T): T { return x; }

func pick(c: bool, x: i32): i32 {
    return i32(id(c ? x : null));
}

func main(): i32 { return pick(true, 5) - 5; }
"#;
    let stderr = reject_host("implcondnullarm", source);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "implcondnullarm: the null arm reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'null' is not allowed here"),
        "implcondnullarm: expected the null rule message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:6:27"),
        "implcondnullarm: the report must land on the null; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("error E10200:"),
        "implcondnullarm: the null rule owns the report; stderr:\n{stderr}"
    );
}

/// A direct conditional's null arm faces every destination kind.
///
/// A let initializer, a return, a struct field, and a call argument each pass
/// their type into the conditional, so the same null arm reports once at each
/// own `null`. The count and the four sites are the pin; no conversion report
/// belongs to these shapes. At the base compiler each context is accepted and
/// the emitted C fails to build.
#[test]
fn should_report_a_direct_conditional_null_at_each_context() {
    let source = r#"module spec.cc.impl.condctxpin;

struct S { v: i32; }

func ret(c: bool, x: i32): i32 {
    return c ? x : null;
}

func arg(v: i32): i32 { return v; }

func main(): i32 {
    let c: bool = true;
    let x: i32 = 5;
    let a: i32 = c ? x : null;
    let s: S = S { v: c ? x : null };
    let b: i32 = arg(c ? x : null);
    return a + s.v + b + ret(c, x) - 20;
}
"#;
    let stderr = reject_host("implcondctx", source);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        4,
        "implcondctx: each direct context reports once; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("'null' is not allowed here").count(),
        4,
        "implcondctx: each report names the null rule; stderr:\n{stderr}"
    );
    for site in [
        "main.clo:6:20",
        "main.clo:14:26",
        "main.clo:15:31",
        "main.clo:16:30",
    ] {
        assert!(
            stderr.contains(site),
            "implcondctx: the null at {site} must report; stderr:\n{stderr}"
        );
    }
    assert!(
        !stderr.contains("error E10200:"),
        "implcondctx: the null rule owns these reports; stderr:\n{stderr}"
    );
}

/// The concrete twin and the accepted conditional controls keep their shapes.
///
/// The concrete disagreeing call already reported one conversion at its else
/// arm before the instance judgment existed. The identical-arm and non-null
/// sibling conditionals, and the legal nullable destinations, are accepted and
/// re-emit byte-identically. These shapes already held before the change, so
/// they are controls rather than discriminators.
#[test]
fn should_keep_the_conditional_concrete_twin_and_controls() {
    let concrete = r#"module spec.cc.impl.condconcretepin;

func f(v: i32): i32 { return v; }

func main(): i32 {
    let c: bool = true;
    return f(c ? 5 : "x");
}
"#;
    assert_rejected_once_at_site(
        "implcondconcrete",
        concrete,
        "error E10200:",
        "cannot convert 'static str' to 'i32' implicitly",
        "main.clo:7:22",
    );

    let control = r#"module spec.cc.impl.condcontrolpin;

func id<T>(x: T): T { return x; }

func choose<T, U>(c: bool, a: T, b: U): i32 {
    return i32(id<T>(c ? a : a));
}

func pick(c: bool, x: i32): i32 {
    return i32(id<i32>(c ? x : 5));
}

func main(): i32 {
    return choose<i32, str>(true, 3, "x") + pick(true, 5) - 8;
}
"#;
    let (first, second) = emit_twice("implcondcontrol", control);
    assert_eq!(
        count(&first, "cl_id__i32("),
        4,
        "implcondcontrol: one keyed instance with two calls is expected; emitted C:\n{first}"
    );
    assert_eq!(
        first, second,
        "implcondcontrol: the same source must re-emit byte-identically"
    );

    let nullable = r#"module spec.cc.impl.condnullablepin;

func pass(x: str?): i32 {
    if (x == null) {
        return 0;
    }
    return 9;
}

func forward(c: bool, s: str): str? {
    return c ? s : null;
}

func pick(c: bool, s: str): i32 {
    let a: str? = c ? s : null;
    let r: i32 = pass(c ? s : null);
    let f: str? = forward(c, s);
    if (a == null) {
        return 1;
    }
    if (r != 9) {
        return 2;
    }
    if (f == null) {
        return 3;
    }
    return pass(a) + pass(f) - 18;
}

func main(): i32 {
    return pick(true, "ab");
}
"#;
    let (first, second) = emit_twice("implcondnullable", nullable);
    assert_eq!(
        first, second,
        "implcondnullable: the legal nullable destinations must re-emit byte-identically"
    );
}

/// A nested element instance is keyed from the substituted element.
///
/// The row type is `str[2]`, so the first indexed level is a `str` view and the
/// second is its byte: the callee instance must be keyed `u8` rather than leave
/// the generated element placeholder in the symbol. The translation unit
/// declares, defines, and calls exactly one such instance, carries no
/// placeholder text, re-emits byte-identically, and keys the same symbol on the
/// hosted-amiga profile. At the base compiler the key is
/// `cl_id__#element of #element of T`, which the strict C build rejects.
#[test]
fn should_key_a_nested_element_instance_from_the_substituted_element() {
    let source = r#"module spec.cc.impl.elemkeypin;

func id<T>(x: T): T { return x; }

func f<T>(xs: T): i32 {
    for (v in xs) {
        return i32(id(v[0]));
    }
    return 0;
}

func main(): i32 {
    let a: str[2] = ["ab", "c"];
    return f<str[2]>(a) - 97;
}
"#;
    let (first, second) = emit_twice("implelemkey", source);
    assert_keyed_instance(
        "implelemkey",
        &first,
        "static u8 cl_id__",
        "static u8 cl_id__u8(u8 x)",
        "cl_id__u8((v).ptr[0])",
    );
    assert!(
        !first.contains("#element"),
        "implelemkey: the translation unit must not carry an element placeholder; emitted C:\n{first}"
    );
    assert_eq!(
        first, second,
        "implelemkey: the same source must re-emit byte-identically"
    );

    let amiga = emit_hosted_amiga("implelemkeyamiga", source);
    assert_keyed_instance(
        "implelemkeyamiga",
        &amiga,
        "static u8 cl_id__",
        "static u8 cl_id__u8(u8 x)",
        "cl_id__u8((v).ptr[0])",
    );
    assert!(
        !amiga.contains("#element"),
        "implelemkeyamiga: the hosted-amiga translation unit must not carry an element placeholder; emitted C:\n{amiga}"
    );

    let freestanding = emit_freestanding("implelemkeyfs", source);
    assert_keyed_instance(
        "implelemkeyfs",
        &freestanding,
        "static u8 cl_id__",
        "static u8 cl_id__u8(u8 x)",
        "cl_id__u8((v).ptr[0])",
    );
    assert!(
        !freestanding.contains("#element"),
        "implelemkeyfs: the freestanding translation unit must not carry an element placeholder; emitted C:\n{freestanding}"
    );
}

/// The element `len` control does not index the substituted element.
///
/// `v.len` reads the view's length, so the callee instance is keyed by the
/// length's `usize` and never mints a nested element placeholder. The shape
/// already held before the element-chain change, so it is a control.
#[test]
fn should_keep_the_nested_element_len_control() {
    let source = r#"module spec.cc.impl.elemkeylenpin;

func id<T>(x: T): T { return x; }

func f<T>(xs: T): i32 {
    for (v in xs) {
        return i32(id(v.len));
    }
    return 0;
}

func main(): i32 {
    let a: str[2] = ["ab", "c"];
    return f<str[2]>(a) - 2;
}
"#;
    let emitted = emit_host("implelemkeylen", source);
    assert_keyed_instance(
        "implelemkeylen",
        &emitted,
        "static usize cl_id__",
        "static usize cl_id__usize(usize x)",
        "cl_id__usize(v.len)",
    );
    assert!(
        !emitted.contains("#element"),
        "implelemkeylen: the translation unit must not carry an element placeholder; emitted C:\n{emitted}"
    );
}

/// A substituted element member read widens at a view destination.
///
/// The generic body reads a `String` member from the opaque element and stores
/// it in a `str` view. The instance must emit the concrete twin's shim-view
/// widening, so the generic and concrete assignments are the same text. At the
/// base compiler the generic body assigns the handle directly (`ClString` into
/// `ClStr`), which the strict C build rejects.
#[test]
fn should_widen_a_substituted_element_member_at_a_view_destination() {
    let generic = r#"module spec.cc.impl.elemviewpin;

import { String, stringOf } from std.text;

struct S { name: String; }

func first<T>(xs: T): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab") }];
    return first<S[1]>(a) - 2;
}
"#;
    let concrete = r#"module spec.cc.impl.elemviewconcretepin;

import { String, stringOf } from std.text;

struct S { name: String; }

func first(xs: S[1]): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab") }];
    return first(a) - 2;
}
"#;
    let widening = "s = cl_string_view(xs[0].name);";
    let generic_c = emit_host("implelemview", generic);
    let concrete_c = emit_host("implelemviewconcrete", concrete);
    assert_eq!(
        count(&generic_c, widening),
        1,
        "implelemview: the substituted member read must widen exactly once; emitted C:\n{generic_c}"
    );
    assert_eq!(
        count(&concrete_c, widening),
        1,
        "implelemviewconcrete: the concrete twin must widen exactly once; emitted C:\n{concrete_c}"
    );
    let generic_line = generic_c
        .lines()
        .find(|line| line.trim() == widening)
        .expect("implelemview: the generic assignment line");
    let concrete_line = concrete_c
        .lines()
        .find(|line| line.trim() == widening)
        .expect("implelemviewconcrete: the concrete assignment line");
    assert_eq!(
        generic_line, concrete_line,
        "implelemview: the generic and concrete assignments must be byte-identical"
    );
}

/// A member read at a non-view destination keeps its plain store.
///
/// Only an owned `String` read into a `str` view takes the shim-view widening.
/// An `i32` member read into an `i32` destination stays a plain store and never
/// grows a view constructor. The shape already held before the widening change,
/// so it is a control.
#[test]
fn should_keep_the_element_member_nonview_destination() {
    let source = r#"module spec.cc.impl.membernonviewpin;

struct S { v: i32; w: i32; }

func first<T>(xs: T): i32 {
    let n: i32 = xs[0].v;
    return n;
}

func main(): i32 {
    let a: S[1] = [S { v: 3, w: 4 }];
    return first<S[1]>(a) - 3;
}
"#;
    let emitted = emit_host("implmembernonview", source);
    assert_contains("implmembernonview", &emitted, "n = xs[0].v;");
    assert!(
        !emitted.contains("cl_string_view(xs[0].v)"),
        "implmembernonview: a non-view destination must not widen; emitted C:\n{emitted}"
    );
}

// ===== Substitution edges: nullable members and conditional residue =====

/// A nullable substituted member guards its view widening.
///
/// The generic body reads a `String?` member from the opaque element and stores
/// it in a `str?` view. The instance must emit the concrete twin's guard, which
/// selects the destination's empty view for a null member instead of building a
/// view over the null handle. The generic and concrete assignment lines are
/// byte-identical, and the guard appears exactly once. At the base compiler the
/// generic line is the unguarded `cl_string_view(xs[0].name)`.
#[test]
fn should_guard_a_nullable_substituted_member_at_a_view_destination() {
    let generic = r#"module spec.cc.impl.nulmemberviewpin;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str? = xs[0].name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first<S[1]>(a) - 3;
}
"#;
    let concrete = r#"module spec.cc.impl.nulmemberviewconcretepin;

import { String } from std.text;

struct S { name: String?; }

func first(xs: S[1]): i32 {
    let s: str? = xs[0].name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first(a) - 3;
}
"#;
    let guarded = "s = (xs[0].name ? cl_string_view(xs[0].name) : cl_zero_ClStr());";
    let generic_c = emit_host("implnulmemberview", generic);
    let concrete_c = emit_host("implnulmemberviewconcrete", concrete);
    assert_eq!(
        count(&generic_c, guarded),
        1,
        "implnulmemberview: the substituted member read must guard exactly once; emitted C:\n{generic_c}"
    );
    assert_eq!(
        count(&concrete_c, guarded),
        1,
        "implnulmemberviewconcrete: the concrete twin must guard exactly once; emitted C:\n{concrete_c}"
    );
    let generic_line = generic_c
        .lines()
        .find(|line| line.trim() == guarded)
        .expect("implnulmemberview: the generic assignment line");
    let concrete_line = concrete_c
        .lines()
        .find(|line| line.trim() == guarded)
        .expect("implnulmemberviewconcrete: the concrete assignment line");
    assert_eq!(
        generic_line, concrete_line,
        "implnulmemberview: the generic and concrete guarded assignments must be byte-identical"
    );
}

/// A nullable substituted member at a non-nullable view reports the null rule.
///
/// The member converts to `str` (a `String?` still widens), so the nullability
/// is the failure: one `E10010` at the read, naming the root binding, and no
/// conversion report. The concrete twin reports the same rule at its own
/// statement. At the base compiler the generic spelling is accepted.
#[test]
fn should_report_a_nullable_substituted_member_at_a_nonnullable_destination() {
    let generic = r#"module spec.cc.impl.nulmemberrejectpin;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first<S[1]>(a);
}
"#;
    let stderr = reject_host("implnulmemberreject", generic);
    assert_eq!(
        stderr.matches("error E10010:").count(),
        1,
        "implnulmemberreject: the possible-null rule reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("value 'xs' may be null here"),
        "implnulmemberreject: the report must name the root binding; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:8:18"),
        "implnulmemberreject: the report must land on the read; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("error E10200:"),
        "implnulmemberreject: the conversion itself is legal; stderr:\n{stderr}"
    );

    let concrete = r#"module spec.cc.impl.nulmemberrejectconcretepin;

import { String } from std.text;

struct S { name: String?; }

func first(xs: S[1]): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first(a);
}
"#;
    let stderr = reject_host("implnulmemberrejectconcrete", concrete);
    assert_eq!(
        stderr.matches("error E10010:").count(),
        1,
        "implnulmemberrejectconcrete: the possible-null rule reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:8:5"),
        "implnulmemberrejectconcrete: the report must land on the destination; stderr:\n{stderr}"
    );
}

/// One conditional site reports its null arm once across two instances.
///
/// The conditionals sit in a generic body instantiated with `i32` and `u8`;
/// both instances reject the same null arm at the same non-nullable
/// destination. The site's first offending instance reports and the site ends,
/// so the translation unit has one `E10012` at the `null`. At the base compiler
/// each instance prints an identical report.
#[test]
fn should_report_a_two_instance_conditional_null_once() {
    let source = r#"module spec.cc.impl.twonullpin;

func choose<T>(c: bool, a: T): i32 {
    let y: T = c ? a : null;
    return 0;
}

func main(): i32 {
    let a: i32 = choose<i32>(true, 5);
    let u: u8 = 6;
    let b: i32 = choose<u8>(false, u);
    return a + b;
}
"#;
    let stderr = reject_host("impltwonull", source);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "impltwonull: the site reports once across its instances; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'null' is not allowed here"),
        "impltwonull: expected the null rule message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:4:24"),
        "impltwonull: the report must land on the null; stderr:\n{stderr}"
    );
}

/// A nested conditional reports at the one `null`, not at the inner conditional.
///
/// The inner `c ? x : null` takes its non-null sibling's type, so an enclosing
/// conditional does not see it as a null-shaped arm. The one `null` reports
/// once at its own span; the inner conditional's start carries no report. At
/// the base compiler the enclosing site reports the inner conditional again.
#[test]
fn should_report_a_nested_conditional_null_once() {
    let source = r#"module spec.cc.impl.nestednullpin;

func id<T>(x: T): T { return x; }

func f<T>(c1: bool, c2: bool, x: T): i32 {
    return i32(id(c2 ? (c1 ? x : null) : x));
}

func main(): i32 {
    return f<i32>(true, true, 5) - 5;
}
"#;
    let stderr = reject_host("implnestednull", source);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "implnestednull: the one null reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:6:34"),
        "implnestednull: the report must land on the null; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("main.clo:6:24"),
        "implnestednull: the inner conditional's start must not report; stderr:\n{stderr}"
    );
}

/// An unannotated conditional local keeps its sibling's type.
///
/// `let v = c ? x : null;` takes the non-null sibling's type, so the later
/// `return v` does not see the null shape. The one `null` reports once at its
/// own span; the return carries no report. At the base compiler the local
/// carries the null shape and the return reports it again.
#[test]
fn should_report_an_unannotated_conditional_null_once() {
    let source = r#"module spec.cc.impl.unannotnullpin;

func f<T>(c: bool, x: T): T {
    let v = c ? x : null;
    return v;
}

func main(): i32 {
    return f<i32>(true, 5) - 5;
}
"#;
    let stderr = reject_host("implunannotnull", source);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "implunannotnull: the one null reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:4:21"),
        "implunannotnull: the report must land on the null; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("main.clo:5:12"),
        "implunannotnull: the return must not report the local; stderr:\n{stderr}"
    );
}

/// Literal and array-literal arms take the destination's type.
///
/// The destination reaches both arm checks, so the integer literal arms carry
/// the `u8` suffix, the float arms keep `f32` values, and the array-literal arm
/// fills the fixed-array destination element by element. These shapes already
/// held before this correction, so they are controls.
#[test]
fn should_keep_the_literal_arm_controls() {
    let source = r#"module spec.cc.impl.literalarmspin;

func main(): i32 {
    let c: bool = true;
    let y: u8 = c ? 1 : 2;
    let f: f32 = c ? 1.5 : 2.5;
    let a: u8[2] = c ? [1, 2] : [3, 4];
    if (f != 1.5) {
        return 9;
    }
    return i32(y) + i32(a[0]) + i32(a[1]) - 4;
}
"#;
    let emitted = emit_host("implliteralarms", source);
    assert_contains("implliteralarms", &emitted, "y = (c ? 1u : 2u);");
    assert_contains("implliteralarms", &emitted, "f = (c ? 1.5 : 2.5);");
    assert_contains("implliteralarms", &emitted, "a[0] = 1u;");
    assert_contains("implliteralarms", &emitted, "a[0] = 3u;");
}

/// An out-of-range literal arm reports the fit rule at the literal.
///
/// The arm is typed by the scalar destination, so the report is `E10202` at the
/// literal rather than a conversion at the conditional. The shape already held
/// before this correction, so it is a control.
#[test]
fn should_report_a_literal_arm_fit_at_its_site() {
    let source = r#"module spec.cc.impl.literalfitpin;

func main(): i32 {
    let c: bool = true;
    let y: u8 = c ? 1 : 300;
    return i32(y);
}
"#;
    assert_rejected_once_at_site(
        "implliteralfit",
        source,
        "error E10202:",
        "literal does not fit 'u8'",
        "main.clo:5:25",
    );
}

// ===== Dotted member roots, FFI crossings, and conditional-arm residue =====

/// A dotted nullable member records its root binding.
///
/// The dotted read and the postfix read both root at `xs`, so the per-instance
/// audit names the same binding and reports one possible-null rule at the read.
/// At the base compiler the dotted spelling records no root and is accepted.
#[test]
fn should_report_a_dotted_nullable_member_at_a_nonnullable_destination() {
    let source = r#"module spec.cc.impl.dotrejectpin;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str = xs.name;
    return i32(len(s));
}

func main(): i32 {
    let a: S = S { name: null };
    return first<S>(a);
}
"#;
    let stderr = reject_host("impldotreject", source);
    assert_eq!(
        stderr.matches("error E10010:").count(),
        1,
        "impldotreject: the possible-null rule reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("value 'xs' may be null here"),
        "impldotreject: the report must name the dotted root; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:8:18"),
        "impldotreject: the report must land on the read; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("error E10200:"),
        "impldotreject: the conversion itself is legal; stderr:\n{stderr}"
    );
}

/// A dotted nullable member at a view destination widens under its guard.
///
/// The dotted read resolves to the concrete `String?` member, so the store
/// spells the same guarded widening the postfix read spells.
#[test]
fn should_guard_a_dotted_nullable_member_at_a_view_destination() {
    let source = r#"module spec.cc.impl.dotviewpin;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str? = xs.name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S = S { name: null };
    return first<S>(a) - 3;
}
"#;
    let emitted = emit_host("impldotview", source);
    let guarded = "s = (xs.name ? cl_string_view(xs.name) : cl_zero_ClStr());";
    assert_eq!(
        count(&emitted, guarded),
        1,
        "impldotview: the dotted read must guard exactly once; emitted C:\n{emitted}"
    );
}

/// A substituted member argument crosses an uncontracted FFI view as a pointer.
///
/// The crossing test reads the argument's resolved type, so the widened view
/// crosses as `(APTR)(...).ptr` exactly as a named `str` value does. The dotted
/// and postfix members both widen first; the raw and `cstring` arguments keep
/// their own crossings.
#[test]
fn should_cross_a_substituted_member_argument_to_an_ffi_view() {
    let source = r#"module spec.cc.impl.ffiviewpin;

import { String, stringOf } from std.text;

ffi lib {
    func RStr(s: str): i32;
    func Raw(p: raw u8): i32;
    func CText(s: cstring): i32;
}

struct S { name: String; }

func dotted<T>(xs: T): i32 {
    let n: i32 = 0;
    unsafe {
        n = RStr(xs.name);
    }
    return n;
}

func indexed<T>(xs: T): i32 {
    let n: i32 = 0;
    unsafe {
        n = RStr(xs[0].name);
    }
    return n;
}

func controls(t: str): i32 {
    let n: i32 = 0;
    let p: raw u8 = t.ptr;
    unsafe {
        n = CText("x") + Raw(p) + RStr(t);
    }
    return n;
}

func main(): i32 {
    let a: S = S { name: stringOf("hi") };
    let b: S[1] = [S { name: stringOf("hi") }];
    let t: str = "hi";
    return dotted<S>(a) + indexed<S[1]>(b) + controls(t) - 5;
}
"#;
    let emitted = emit_host("implffiview", source);
    assert_contains(
        "implffiview",
        &emitted,
        "RStr((APTR)(cl_string_view(xs.name)).ptr)",
    );
    assert_contains(
        "implffiview",
        &emitted,
        "RStr((APTR)(cl_string_view(xs[0].name)).ptr)",
    );
    assert_contains("implffiview", &emitted, "RStr((APTR)(t).ptr)");
    assert_contains("implffiview", &emitted, "CText(\"x\")");
    assert_contains("implffiview", &emitted, "Raw(p)");
}

/// A substituted member argument widens, and a nullable one guards.
///
/// The argument renderer resolves the member through the instance, so a
/// non-nullable `String` member widens to the `str` parameter and a `String?`
/// member guards its widening at a `str?` parameter. At the base compiler the
/// unresolved argument keeps its member read and the C build fails.
#[test]
fn should_widen_a_substituted_member_argument() {
    let source = r#"module spec.cc.impl.argwidenpin;

import { String, stringOf } from std.text;

func consume(s: str): i32 { return i32(len(s)); }

func maybe(s: str?): i32 {
    if (s == null) {
        return 0;
    }
    return i32(len(s));
}

struct S { name: String; }
struct N { name: String?; }

func nonnull<T>(xs: T): i32 {
    return consume(xs.name);
}

func nullable<T>(xs: T): i32 {
    return maybe(xs.name);
}

func main(): i32 {
    let a: S = S { name: stringOf("hi") };
    let b: N = N { name: stringOf("hi") };
    return nonnull<S>(a) + nullable<N>(b) - 4;
}
"#;
    let emitted = emit_host("implargwiden", source);
    assert_contains(
        "implargwiden",
        &emitted,
        "consume(cl_string_view(xs.name))",
    );
    assert_contains(
        "implargwiden",
        &emitted,
        "maybe((xs.name ? cl_string_view(xs.name) : cl_zero_ClStr()))",
    );
}

/// Both substituted conditional arms widen at a view result.
///
/// The instance resolves each arm's `String` member before the arm renderer, so
/// both arms take the shim-view widening and the assignment is byte-identical
/// to the concrete twin's. At the base compiler the unresolved arms keep the
/// handle values and the C build fails.
#[test]
fn should_widen_both_substituted_member_arms() {
    let generic = r#"module spec.cc.impl.condarmspin;

import { String, stringOf } from std.text;

struct S { name: String; }

func first<T>(xs: T, c: bool): i32 {
    let s: str = c ? xs[0].name : xs[1].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("hi") }, S { name: stringOf("hi") }];
    return first<S[2]>(a, true) - 2;
}
"#;
    let concrete = r#"module spec.cc.impl.condarmsconcretepin;

import { String, stringOf } from std.text;

struct S { name: String; }

func first(xs: S[2], c: bool): i32 {
    let s: str = c ? xs[0].name : xs[1].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("hi") }, S { name: stringOf("hi") }];
    return first(a, true) - 2;
}
"#;
    let widening = "s = (c ? cl_string_view(xs[0].name) : cl_string_view(xs[1].name));";
    let generic_c = emit_host("implcondarms", generic);
    let concrete_c = emit_host("implcondarmsconcrete", concrete);
    assert_eq!(
        count(&generic_c, widening),
        1,
        "implcondarms: both arms must widen exactly once; emitted C:\n{generic_c}"
    );
    assert_eq!(
        count(&concrete_c, widening),
        1,
        "implcondarmsconcrete: the concrete twin must widen exactly once; emitted C:\n{concrete_c}"
    );
    let generic_line = generic_c
        .lines()
        .find(|line| line.trim() == widening)
        .expect("implcondarms: the generic assignment line");
    let concrete_line = concrete_c
        .lines()
        .find(|line| line.trim() == widening)
        .expect("implcondarmsconcrete: the concrete assignment line");
    assert_eq!(
        generic_line, concrete_line,
        "implcondarms: the generic and concrete assignments must be byte-identical"
    );
}

/// A site whose two arms are null reports each arm once for its first instance.
///
/// The first instance's then and else arms report at their own nulls, and the
/// site ends, so two instances produce two reports rather than four. At the
/// base compiler each instance reports both arms.
#[test]
fn should_report_both_null_arms_once_per_instance() {
    let source = r#"module spec.cc.impl.bothnullpin;

func choose<T>(c: bool): i32 {
    let y: T = c ? null : null;
    return 0;
}

func main(): i32 {
    let a: i32 = choose<i32>(true);
    let u: u8 = 6;
    let b: i32 = choose<u8>(false);
    return a + b;
}
"#;
    let stderr = reject_host("implbothnull", source);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        2,
        "implbothnull: the first instance reports both arms once; stderr:\n{stderr}"
    );
    for site in ["main.clo:4:20", "main.clo:4:27"] {
        assert!(
            stderr.contains(site),
            "implbothnull: the null at {site} must report; stderr:\n{stderr}"
        );
    }
}
