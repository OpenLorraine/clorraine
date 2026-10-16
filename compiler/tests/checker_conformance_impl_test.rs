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
