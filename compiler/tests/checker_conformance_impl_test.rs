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
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga".to_owned(),
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
    // The `raw u8` return spells the pre-existing const-discard shape (a
    // const pointer at a mutable destination): the pin records the emission
    // deliberately, not a build regression; the shape is a filed row.
    assert_contains("ptrpin", &emitted, "return s.ptr;");
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

/// An import alias that repeats an own binding keeps the own binding's key.
///
/// `import { b as g }` beside `let g: i32` binds the name twice, and the
/// initializer's `g.pick(7)` keys the module's own `i32`, so the emitted call
/// reaches a declared and defined instance. The import's `Box` must leave no
/// trace: a recorded instance of it would declare a specialization the
/// emitter never calls.
#[test]
fn should_declare_the_own_binding_at_an_aliased_receiver_collision() {
    let collision = r#"module spec.cc.impl.aliascollision;

import { b as g } from spec.cc.impl.aliasreceiverholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let g: i32 = 3;

let h: i32 = g.pick(7);

func main(): i32 {
    return (h - 7) + (g - 3);
}
"#;
    let emitted = emit_host_pair("aliascollision", ALIAS_RECEIVER_HOLDER_SOURCE, collision);
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

/// A take argument empties a borrowed element before the next store.
///
/// `eat(v.name)` moves the element's member to the callee, so the element is
/// emptied right after the call — the top-level field spelling's shape — and
/// the store that follows sees an empty place. Before the place type reached
/// the move-out, the element kept the value the callee owned and the store's
/// overwrite release freed it twice. The top-level spelling already held its
/// shape before the element places followed it, so its half of this pin is
/// the unchanged control.
#[test]
fn should_empty_a_borrowed_element_after_a_take_argument() {
    let take = "cl_spec_cc_impl_elemtake_eat((*__cl_tmp1).name);";
    let empty = "(*__cl_tmp1).name = NULL;";
    let emitted = emit_host("elemtake", ELEMENT_TAKE_SOURCE);
    assert_contains("elemtake", &emitted, take);
    let take_at = emitted
        .find(take)
        .unwrap_or_else(|| panic!("elemtake: the take call is missing"));
    let empty_at = emitted
        .find(empty)
        .unwrap_or_else(|| panic!("elemtake: the element is never emptied"));
    assert!(
        take_at < empty_at,
        "elemtake: the take must empty the element before the next store"
    );
    assert_eq!(
        count(&emitted, empty),
        2,
        "elemtake: expected the take's emptying and the store's release"
    );

    // The top-level spelling already emptied its field before the element
    // places followed it, so these shapes guard it unchanged.
    let emitted = emit_host("elemtakectl", ELEMENT_TAKE_CONTROL_SOURCE);
    assert_contains("elemtakectl", &emitted, "eat(a.name);");
    assert_contains("elemtakectl", &emitted, "a.name = NULL;");
}
