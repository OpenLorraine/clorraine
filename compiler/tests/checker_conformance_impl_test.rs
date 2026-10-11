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

/// A view argument crosses an uncontracted `cstring` parameter as its pointer.
///
/// The prototype declares `char *`, so the crossing spells the cast instead of
/// passing the view struct (red at the phase base: the strict build failed).
#[test]
fn should_cross_a_view_at_a_cstring_parameter() {
    let source = r#"module spec.cc.impl.cstringviewpin;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let s: str = "/definitely/not/here";
    unsafe {
        let f: raw u8 = Open(s, 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    let emitted = emit_host("cstringviewpin", source);
    assert_contains("cstringviewpin", &emitted, "Open((char *)(s).ptr, 0)");
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

