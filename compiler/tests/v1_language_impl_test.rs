//! Implementation tests for the v1 language completion internals.
//!
//! These tests cover the emitter and checker edges the runnable specification
//! suite does not pin directly: that a generic struct instantiation dedupes to
//! one emitted C struct, that nested arguments compose into one symbol, and
//! that the generics diagnostics report through the real compiler binary. They
//! are not the specification oracle; a defect here is an implementation bug.

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
            "clc-v1lang-impl-{}-{tag}-{sequence}",
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

/// Compiles one source for the host profile and returns the emitted C.
///
/// The front end must accept the program; a rejection is a test failure, not a
/// result to assert.
fn emit(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("cannot write the snippet");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Builds one source with the `clorraine` driver for the host profile.
///
/// The driver compiles the emitted C with the host C compiler, so a zero exit
/// proves the translation unit is valid C.
fn build(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("cannot write the snippet");
    Command::new(env!("CARGO_BIN_EXE_clorraine"))
        .args(["build", "main.clo"])
        .current_dir(scratch.path())
        .output()
        .expect("failed to spawn the clorraine driver")
}

/// Compiles one source and returns the compiler's stderr.
fn diagnostics(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("cannot write the snippet");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    String::from_utf8_lossy(&compile.stderr).into_owned()
}

/// The number of times `needle` appears in `haystack`.
fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn one_c_struct_per_distinct_instantiation_is_emitted() {
    let source = r#"module tests.v1.impl.dedupe;

struct Pair<T> {
    first: T;
    second: T;
}

func main(): i32 {
    let a = Pair<i32> { first: 1, second: 2 };
    let b = Pair<i32> { first: 3, second: 4 };
    return a.first + b.second;
}
"#;
    let emitted = emit("dedupe", source);
    assert_eq!(
        count(&emitted, "struct ClPair__i32 {"),
        1,
        "two uses of Pair<i32> must share one emitted struct"
    );
    assert!(
        !emitted.contains("struct ClPair {"),
        "the bare generic template must not be emitted"
    );
}

#[test]
fn nested_arguments_compose_into_a_single_symbol() {
    let source = r#"module tests.v1.impl.nested;

struct Pair<T> {
    first: T;
    second: T;
}

func main(): i32 {
    let inner = Pair<i32> { first: 1, second: 2 };
    let outer = Pair<Pair<i32>> { first: inner, second: inner };
    return outer.first.first;
}
"#;
    let emitted = emit("nested", source);
    assert_eq!(
        count(&emitted, "struct ClPair__Pair__i32 {"),
        1,
        "the nested key must compose from the inner key"
    );
}

#[test]
fn a_type_argument_violating_a_bound_reports_e10303() {
    let source = r#"module tests.v1.impl.bound;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    return apply<i32, i32>(5, 3);
}
"#;
    let stderr = diagnostics("bound", source);
    assert!(
        stderr.contains("error E10303"),
        "expected E10303, got:\n{stderr}"
    );
}

#[test]
fn a_generic_field_feeds_a_callable_bound() {
    // The field of `Pair<i32>` is `i32`, so the concrete `addOne` satisfies
    // `F: func(T): T` without an explicit type argument.
    let source = r#"module tests.v1.impl.field_bound;

struct Pair<T> {
    first: T;
    second: T;
}

func addOne(x: i32): i32 { return x + 1; }
func apply<T, F: func(T): T>(x: T, f: F): T { return f(x); }

func main(): i32 {
    let p = Pair<i32> { first: 5, second: 2 };
    return apply(p.first, addOne);
}
"#;
    let emitted = emit("field-bound", source);
    // The `F` instance argument is a `func` type, whose key carries its whole
    // signature, so the monomorphized symbol names `fn_i32__i32`.
    assert!(
        emitted.contains("cl_apply__i32_fn_i32__i32(p.first,"),
        "the substituted field must flow into the monomorphized generic call"
    );
}

#[test]
fn a_callable_field_of_a_generic_struct_checks_its_substituted_signature() {
    // A callable field declared `func(T): T` becomes `func(i32): i32` on the
    // `Holder<i32>` instance, so the dot-call takes an `i32` argument.
    let source = r#"module tests.v1.impl.field_callable;

struct Holder<T> {
    run: func(T): T;
    value: T;
}

func id(x: i32): i32 { return x; }

func main(): i32 {
    let h = Holder<i32> { run: id, value: 5 };
    return h.run(h.value);
}
"#;
    let emitted = emit("field-callable", source);
    assert!(
        emitted.contains("(h.run)(h.value)"),
        "the callable field must be invoked with the substituted argument type"
    );
}

#[test]
fn returning_a_callable_only_value_reports_e10304() {
    let source = r#"module tests.v1.impl.callable;

func keep<T, F: func(T): T>(x: T, f: F): F {
    return f;
}

func main(): i32 {
    return 0;
}
"#;
    let stderr = diagnostics("callable", source);
    assert!(
        stderr.contains("error E10304"),
        "expected E10304, got:\n{stderr}"
    );
}

#[test]
fn a_struct_that_embeds_itself_reports_e10305() {
    let source = r#"module tests.v1.impl.recursive;

struct Wrapper<T> {
    inner: T;
}

struct Nest<T> {
    next: Nest<Wrapper<T>>;
}

func main(): i32 {
    return 0;
}
"#;
    let stderr = diagnostics("recursive", source);
    assert!(
        stderr.contains("error E10305"),
        "expected E10305, got:\n{stderr}"
    );
}

#[test]
fn an_embedded_generic_struct_is_defined_before_its_user() {
    let source = r#"module tests.v1.impl.embed_order;

struct Pair<T> {
    first: T;
    second: T;
}

struct Box<T> {
    value: Pair<T>;
}

func main(): i32 {
    let b = Box<i32> { value: Pair<i32> { first: 1, second: 2 } };
    return b.value.first;
}
"#;
    let emitted = emit("embed-order", source);
    let embedded = emitted
        .find("struct ClPair__i32 {")
        .expect("the embedded instantiation must be defined");
    let user = emitted
        .find("struct ClBox__i32 {")
        .expect("the embedding instantiation must be defined");
    assert!(
        embedded < user,
        "the embedded struct must be defined before its user"
    );
}

#[test]
fn a_struct_recursing_through_a_generic_field_reports_e10305() {
    let source = r#"module tests.v1.impl.recursive_generic;

struct Box<T> {
    inner: T;
}

struct S {
    b: Box<S>;
}

func main(): i32 {
    return 0;
}
"#;
    let stderr = diagnostics("recursive-generic", source);
    assert!(
        stderr.contains("error E10305"),
        "expected E10305, got:\n{stderr}"
    );
}

#[test]
fn a_capturing_lambda_lifts_to_an_environment_function() {
    let source = r#"module tests.v1.impl.lift;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    let base: i32 = 2;
    return apply(1, [base] (x: i32) => x + base);
}
"#;
    let emitted = emit("lift", source);
    assert_eq!(
        count(&emitted, "struct ClEnv_0 {"),
        1,
        "the capturing lambda must emit one environment struct"
    );
    assert!(
        emitted.contains("cl_lambda_0"),
        "the lambda must lift to a cl_lambda_0 function"
    );
    assert!(
        emitted.contains("ClClosure_0"),
        "the capturing value must be the two-word ClClosure_0 struct"
    );
    assert!(
        emitted.contains("(f).fn((f).env,"),
        "the callable-bound instance must call the closure through its two words"
    );
    assert!(
        emitted.contains("cl_apply__i32_closure_0"),
        "the closure must monomorphize the generic body to a closure-keyed instance"
    );
}

#[test]
fn a_borrow_capture_holds_a_pointer_and_writes_through() {
    let source = r#"module tests.v1.impl.borrow_capture;

func main(): i32 {
    let counter: i32 = 0;
    ([borrow counter] (n: i32) => { counter = counter + n; })(5);
    return counter;
}
"#;
    let emitted = emit("borrow-capture", source);
    assert!(
        emitted.contains("struct ClEnv_0 {"),
        "the borrow capture must emit an environment struct"
    );
    assert!(
        emitted.contains("i32 *counter;"),
        "a [borrow] capture must store a pointer, not a copy"
    );
    assert!(
        emitted.contains("(*__env->counter)"),
        "the lifted body must write through the environment pointer"
    );
    assert!(
        emitted.contains("cl_lambda_0(&"),
        "a directly invoked closure must be called with the environment address"
    );
}

#[test]
fn distinct_lambda_sites_get_distinct_instances() {
    let source = r#"module tests.v1.impl.sites;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    let a: i32 = 1;
    let b: i32 = 2;
    let first = apply(1, [a] (x: i32) => x + a);
    let second = apply(1, [b] (x: i32) => x + b);
    return first + second;
}
"#;
    let emitted = emit("sites", source);
    assert_eq!(
        count(&emitted, "struct ClEnv_0 {"),
        1,
        "the first closure site must emit its own environment"
    );
    assert_eq!(
        count(&emitted, "struct ClEnv_1 {"),
        1,
        "the second closure site must not share the first environment"
    );
    assert_eq!(
        count(&emitted, "cl_apply__i32_closure_0("),
        3,
        "the first site must have one prototype, one definition, and one call"
    );
    assert_eq!(
        count(&emitted, "cl_apply__i32_closure_1("),
        3,
        "the second site must have one prototype, one definition, and one call"
    );
}

#[test]
fn a_block_body_lambda_infers_its_return_type() {
    let source = r#"module tests.v1.impl.block_return;

func apply2(x: i32, f: func(i32): i32): i32 {
    return f(f(x));
}

func main(): i32 {
    return apply2(3, (x: i32) => { return x + 1; });
}
"#;
    let emitted = emit("block-return", source);
    assert!(
        emitted.contains("i32 cl_lambda_0("),
        "a block body's `return x + 1` must infer an i32 return, not void"
    );
}

#[test]
fn a_captureless_lambda_needs_no_environment() {
    let source = r#"module tests.v1.impl.captureless;

func apply2(x: i32, f: func(i32): i32): i32 {
    return f(f(x));
}

func main(): i32 {
    return apply2(3, (x: i32) => x + 1);
}
"#;
    let emitted = emit("captureless", source);
    assert!(
        emitted.contains("cl_lambda_0"),
        "a capture-less lambda must still lift to a callable function"
    );
    assert!(
        !emitted.contains("ClEnv_"),
        "a capture-less lambda is a plain function pointer with no environment"
    );
    assert!(
        !emitted.contains("ClClosure_"),
        "a capture-less lambda must not use the two-word closure struct"
    );
}

#[test]
fn a_volatile_module_global_keeps_the_c_qualifier() {
    let source = r#"module tests.v1.impl.volatile_global;

volatile let ticks: u32 = 0;

func main(): i32 {
    ticks = ticks + 1;
    return i32(ticks);
}
"#;
    let emitted = emit("volatile-global", source);
    assert!(
        emitted.contains("static volatile u32"),
        "a `volatile let` must emit a C `volatile` static, not a plain one"
    );
}

#[test]
fn the_volatile_intrinsics_emit_a_volatile_access() {
    let source = r#"module tests.v1.impl.volatile_forms;

func demo(): i32 {
    let value: u16 = 0;
    unsafe {
        let p: raw u16 = &value;
        writeVolatile16(p, 0x1234);
        return i32(readVolatile16(p));
    }
}

func main(): i32 {
    return demo();
}
"#;
    let emitted = emit("volatile-forms", source);
    assert!(
        count(&emitted, "*(volatile u16 *)") >= 2,
        "both the read and the write must reach a `volatile u16` pointee access"
    );
}

#[test]
fn a_field_write_through_a_const_raw_global_is_allowed() {
    // A `const raw` register base is an immutable pointer, but writing its
    // pointee is a hardware access, not a write to the binding.
    let source = r#"module tests.v1.impl.raw_base;

struct Regs {
    dmacon: volatile u16;
}

const BASE: raw Regs = raw Regs(0x00DFF000);

func main(): i32 {
    unsafe {
        BASE.dmacon = 1;
    }
    return 0;
}
"#;
    let emitted = emit("raw-base", source);
    assert!(
        emitted.contains("->dmacon"),
        "the register write must dereference the raw base"
    );
}

#[test]
fn a_field_write_through_a_raw_global_needs_unsafe() {
    let source = r#"module tests.v1.impl.raw_base_unsafe;

struct Regs {
    dmacon: volatile u16;
}

const BASE: raw Regs = raw Regs(0x00DFF000);

func main(): i32 {
    BASE.dmacon = 1;
    return 0;
}
"#;
    let stderr = diagnostics("raw-base-unsafe", source);
    assert!(
        stderr.contains("E10600"),
        "a field write through a raw global must require 'unsafe'; got:\n{stderr}"
    );
}

#[test]
fn a_generic_struct_handle_field_is_dropped() {
    let source = r#"module tests.v1.impl.generic_drop;

import { String, stringOf } from std.text;

struct Box<T> {
    value: T;
}

func main(): i32 {
    let b = Box<String> { value: stringOf("x") };
    return 0;
}
"#;
    let emitted = emit("generic-drop", source);
    assert!(
        emitted.contains("FreeString(b.value)"),
        "the generic handle field must be released; got:\n{emitted}"
    );
}

#[test]
fn a_nested_generic_zero_value_emits_nested_zero_states() {
    let source = r#"module tests.v1.impl.nested_zero;

struct Pair<T> {
    first: T;
    second: T;
}

func main(): i32 {
    let z = Pair<Pair<i32>> {};
    return z.first.first;
}
"#;
    let emitted = emit("nested-zero", source);
    assert!(
        emitted.contains("(ClPair__Pair__i32){ {0}, {0} }"),
        "the nested zero value must use nested zero states; got:\n{emitted}"
    );
}

#[test]
fn a_plain_zero_value_stays_valid() {
    let source = r#"module tests.v1.impl.plain_zero;

struct Point {
    x: i32;
    y: i32;
}

func main(): i32 {
    let p = Point {};
    return p.x;
}
"#;
    let emitted = emit("plain-zero", source);
    assert!(
        emitted.contains("(ClPoint){ 0, 0 }"),
        "a non-generic zero value must still emit its zero fields; got:\n{emitted}"
    );
}

#[test]
fn a_non_generic_owning_struct_is_unchanged() {
    // A non-generic struct that owns a handle still drops its field and emits
    // no generic instantiation symbol.
    let source = r#"module tests.v1.impl.non_generic_drop;

import { String, stringOf } from std.text;

struct Label {
    name: String;
}

func main(): i32 {
    let l = Label { name: stringOf("x") };
    return 0;
}
"#;
    let emitted = emit("non-generic-drop", source);
    assert!(
        emitted.contains("FreeString(l.name)"),
        "a non-generic owning field must still drop; got:\n{emitted}"
    );
    assert!(
        !emitted.contains("ClLabel__"),
        "a non-generic struct must not gain a generic symbol"
    );
}

#[test]
fn a_generic_handle_field_builds() {
    // The dropped generic handle field must produce valid C: the driver
    // compiles the translation unit with the host C compiler.
    let source = r#"module tests.v1.impl.generic_drop_build;

import { String, stringOf } from std.text;

struct Box<T> {
    value: T;
}

func main(): i32 {
    let b = Box<String> { value: stringOf("x") };
    return 0;
}
"#;
    let output = build("generic-drop-build", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the generic handle field must build; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_nested_generic_zero_value_builds() {
    // The nested zero state must produce valid C, not `{ 0, 0 }`.
    let source = r#"module tests.v1.impl.nested_zero_build;

struct Pair<T> {
    first: T;
    second: T;
}

func main(): i32 {
    let z = Pair<Pair<i32>> {};
    return z.first.first;
}
"#;
    let output = build("nested-zero-build", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the nested generic zero value must build; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
