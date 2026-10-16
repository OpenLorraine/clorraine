//! Implementation tests for the module initializer integrity batch.
//!
//! The specification suite proves what an initializer computes; these cases
//! prove how the compiler emits its resolved callables: a lifted lambda with
//! its declaration and definition, a func-typed global called through its own
//! storage, a generic call reaching its declared instance, and a callable-field
//! call passing its receiver.

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

/// Asserts the first `s = NULL;` after `call_at` follows the call directly.
///
/// The emitted C may carry `#line` markers between the call and the next
/// statement, so the lines after the call's own line may consist of those
/// markers and blank lines; any other text means a statement intervenes before
/// the move-out.
fn assert_move_out_follows(tag: &str, emitted: &str, call_at: usize) {
    let after = &emitted[call_at..];
    let line_end = after.find('\n').map_or(after.len(), |index| index + 1);
    let rest = &after[line_end..];
    let null_at = rest
        .find("s = NULL;")
        .unwrap_or_else(|| panic!("{tag}: no move-out follows the call; emitted C:\n{emitted}"));
    assert!(
        rest[..null_at]
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("#line")),
        "{tag}: a statement intervenes before the move-out; emitted C:\n{emitted}"
    );
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


/// An initializer lambda lifts to a module-level function.
///
/// The initializer's call reaches the lifted identity, and the lifted function
/// is declared and defined exactly as a body lambda's is.
#[test]
fn should_lift_an_initializer_lambda() {
    let source = r#"module spec.mii.impl.lambdacallee;

let g: i32 = (() => 3)();

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("lambdacallee", source);
    assert_contains("lambdacallee", &emitted, "static i32 cl_lambda_0(void);");
    assert_contains("lambdacallee", &emitted, "static i32 cl_lambda_0(void)\n{");
    assert_contains(
        "lambdacallee",
        &emitted,
        "cl_spec_mii_impl_lambdacallee_g = cl_lambda_0();",
    );
}

/// A func-typed global call in an initializer spells the global's symbol.
///
/// The call goes through the global's own storage, so the written name never
/// reaches the C file as a free function.
#[test]
fn should_spell_a_func_typed_global_call_in_an_initializer() {
    let source = r#"module spec.mii.impl.funcglobal;

func ready(): i32 {
    return 7;
}

let f: func(): i32 = ready;
let g: i32 = f();

func main(): i32 {
    return g - 7;
}
"#;
    let emitted = emit_host("funcglobal", source);
    assert_contains(
        "funcglobal",
        &emitted,
        "cl_spec_mii_impl_funcglobal_f = cl_spec_mii_impl_funcglobal_ready;",
    );
    assert_contains(
        "funcglobal",
        &emitted,
        "cl_spec_mii_impl_funcglobal_g = (cl_spec_mii_impl_funcglobal_f)();",
    );
}

/// A generic initializer call reaches its declared instance.
///
/// The instance machinery walks initializer expressions, so the call and the
/// definition share one key and no implicit declaration remains.
#[test]
fn should_declare_a_generic_initializer_call_instance() {
    let source = r#"module spec.mii.impl.genericcall;

func pick<T>(a: T, b: T): T {
    return a;
}

let g: i32 = pick<i32>(1, 2);

func main(): i32 {
    return g - 1;
}
"#;
    let emitted = emit_host("genericcall", source);
    assert_contains("genericcall", &emitted, "static i32 cl_pick__i32(i32 a, i32 b);");
    assert_contains("genericcall", &emitted, "static i32 cl_pick__i32(i32 a, i32 b)\n{");
    assert_contains(
        "genericcall",
        &emitted,
        "cl_spec_mii_impl_genericcall_g = cl_pick__i32(1, 2);",
    );
}

/// A callable-field call in an initializer passes the field read.
///
/// The receiver's global is spelled and the field read is the callable, so the
/// call renders through the receiver exactly as the body's field-call path
/// renders it.
#[test]
fn should_pass_a_field_receiver_in_an_initializer() {
    let source = r#"module spec.mii.impl.fieldreceiver;

func ready(): i32 {
    return 7;
}

struct Box { get: func(): i32; }

let b: Box = Box { get: ready };
let g: i32 = b.get();

func main(): i32 {
    return g - 7;
}
"#;
    let emitted = emit_host("fieldreceiver", source);
    assert_contains(
        "fieldreceiver",
        &emitted,
        "cl_spec_mii_impl_fieldreceiver_g = (cl_spec_mii_impl_fieldreceiver_b.get)();",
    );
}

/// A lambda-valued func-typed global binds the lifted identity.
///
/// The global's initializer is a lambda value, so the lifted function is its
/// C value exactly as a body lambda's would be, and the later call reaches it
/// through the global.
#[test]
fn should_bind_a_lambda_valued_global_to_the_lifted_identity() {
    let source = r#"module spec.mii.impl.lambdavalue;

import { String, stringOf } from std.text;

let f: func(String): i32 = (s: String) => i32(len(s));
let g: i32 = f(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("lambdavalue", source);
    assert_contains(
        "lambdavalue",
        &emitted,
        "cl_spec_mii_impl_lambdavalue_f = cl_lambda_0;",
    );
    assert_contains(
        "lambdavalue",
        &emitted,
        "ClString __cl_itmp0 = cl_std_text_stringOf(cl_str(\"abc\"));",
    );
    assert_contains(
        "lambdavalue",
        &emitted,
        "cl_spec_mii_impl_lambdavalue_g = (cl_spec_mii_impl_lambdavalue_f)(__cl_itmp0);",
    );
    assert_contains(
        "lambdavalue",
        &emitted,
        "if (__cl_itmp0) { FreeString(__cl_itmp0); }",
    );
}

/// A struct literal with an array field stores statement-form.
///
/// The literal binds into an init local, the array field copies with
/// `memcpy` from the source place, and the local assigns to the global — the
/// body's literal-store shape.
#[test]
fn should_store_a_struct_literal_statement_form() {
    let source = r#"module spec.mii.impl.structstore;

struct BoxA { arr: i32[2]; tag: i32; }

let garr: i32[2] = [1, 2];
let boxed: BoxA = BoxA { arr: garr, tag: 3 };

func main(): i32 {
    return boxed.arr[0] + boxed.arr[1] + boxed.tag - 6;
}
"#;
    let emitted = emit_host("structstore", source);
    assert_contains("structstore", &emitted, "ClBoxA __cl_itmp0 = {0};");
    assert_contains(
        "structstore",
        &emitted,
        "memcpy(&(__cl_itmp0.arr), &(cl_spec_mii_impl_structstore_garr), sizeof(__cl_itmp0.arr));",
    );
    assert_contains("structstore", &emitted, "__cl_itmp0.tag = 3;");
    assert_contains(
        "structstore",
        &emitted,
        "cl_spec_mii_impl_structstore_boxed = __cl_itmp0;",
    );
}

/// A call-filled array field writes through its hidden destination first.
///
/// The fixed-array call fills an init local, the field copies from it with
/// `memcpy`, and the literal assigns to the global; no compound literal
/// carries an array expression.
#[test]
fn should_store_a_call_filled_array_field() {
    let source = r#"module spec.mii.impl.structstorecall;

struct BoxA { arr: i32[2]; }

func dup(): i32[2] {
    return [5, 6];
}

let boxed: BoxA = BoxA { arr: dup() };

func main(): i32 {
    return boxed.arr[0] + boxed.arr[1] - 11;
}
"#;
    let emitted = emit_host("structstorecall", source);
    assert_contains(
        "structstorecall",
        &emitted,
        "cl_spec_mii_impl_structstorecall_dup(__cl_itmp0);",
    );
    assert_contains(
        "structstorecall",
        &emitted,
        "memcpy(&(__cl_itmp1.arr), &(__cl_itmp0), sizeof(__cl_itmp1.arr));",
    );
    assert_contains(
        "structstorecall",
        &emitted,
        "cl_spec_mii_impl_structstorecall_boxed = __cl_itmp1;",
    );
}

/// A template at a `str` global reaches it through the view constructor.
#[test]
fn should_wrap_a_template_at_a_str_global() {
    let source = r#"module spec.mii.impl.templateview;

let n: i32 = 4;
let label: str = $"n={n}";

func main(): i32 {
    return i32(len(label)) - 3;
}
"#;
    let emitted = emit_host("templateview", source);
    assert_contains(
        "templateview",
        &emitted,
        "cl_spec_mii_impl_templateview_label = cl_string_view(cl_string_concat(",
    );
}

/// A String-valued call at a `str` global reaches it through the view wrapper.
#[test]
fn should_wrap_a_string_call_at_a_str_global() {
    let source = r#"module spec.mii.impl.strviewcall;

import { stringOf } from std.text;

let s: str = stringOf("abc");

func main(): i32 {
    return i32(len(s)) - 3;
}
"#;
    let emitted = emit_host("strviewcall", source);
    assert_contains(
        "strviewcall",
        &emitted,
        "cl_spec_mii_impl_strviewcall_s = cl_string_view(cl_std_text_stringOf(",
    );
}

/// A qualified field `.len` folds to the field's declared extent.
#[test]
fn should_fold_a_field_len_in_an_initializer() {
    let source = r#"module spec.mii.impl.foldlen;

struct BoxB { arr: i32[2]; }

let b: BoxB = BoxB { arr: [1, 2] };
let n: usize = b.arr.len();

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let emitted = emit_host("foldlen", source);
    assert_contains("foldlen", &emitted, "cl_spec_mii_impl_foldlen_n = 2u;");
}

/// A literal receiver's fold materializes nothing.
///
/// The extent is a structural property, so the initializer emits the folded
/// constant and no receiver stores at all — exactly as the body's fold
/// drops them.
#[test]
fn should_fold_a_literal_receiver_without_receiver_stores() {
    let source = r#"module spec.mii.impl.literalfold;

struct BoxA { arr: i32[2]; tag: i32; }

let n: usize = BoxA { arr: [1, 2], tag: 3 }.arr.len();

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let emitted = emit_host("literalfold", source);
    assert_contains("literalfold", &emitted, "cl_spec_mii_impl_literalfold_n = 2u;");
    assert_eq!(
        count(&emitted, "__cl_itmp"),
        0,
        "literalfold: the fold must not materialize the receiver"
    );
}

/// A free `len` of a literal receiver folds without materializing it.
///
/// Both spellings fold through one rule: the fixed-array extent is a
/// structural property, so the initializer emits the constant and no
/// receiver stores.
#[test]
fn should_fold_a_free_len_of_a_literal_receiver() {
    let source = r#"module spec.mii.impl.freelen;

struct BoxA { arr: i32[2]; tag: i32; }

let n: usize = len(BoxA { arr: [1, 2], tag: 3 }.arr);

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let emitted = emit_host("freelen", source);
    assert_contains("freelen", &emitted, "cl_spec_mii_impl_freelen_n = 2u;");
    assert_eq!(
        count(&emitted, "__cl_itmp"),
        0,
        "freelen: the fold must not materialize the receiver"
    );
}

/// An owned member projection binds its temporary and releases it.
///
/// The call result binds into an init local, the projection reads it, and the
/// release pass frees the owned member at the init function's end.
#[test]
fn should_release_a_member_projection_temporary() {
    let source = r#"module spec.mii.impl.memberrelease;

import { String, stringOf } from std.text;

struct Box { count: i32; name: String; }

func mkBox(): Box {
    return Box { count: 3, name: stringOf("n") };
}

let g: i32 = mkBox().count;

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("memberrelease", source);
    assert_contains(
        "memberrelease",
        &emitted,
        "ClBox __cl_itmp0 = cl_spec_mii_impl_memberrelease_mkBox();",
    );
    assert_contains(
        "memberrelease",
        &emitted,
        "cl_spec_mii_impl_memberrelease_g = __cl_itmp0.count;",
    );
    assert_contains(
        "memberrelease",
        &emitted,
        "if (__cl_itmp0.name) { FreeString(__cl_itmp0.name); }",
    );
}

/// An owned element projection releases both element owners.
#[test]
fn should_release_an_element_projection_temporary() {
    let source = r#"module spec.mii.impl.elementrelease;

import { String, stringOf } from std.text;

func two(): String[2] {
    return [stringOf("a"), stringOf("bb")];
}

let g: usize = len(two()[0]);

func main(): i32 {
    return i32(g) - 1;
}
"#;
    let emitted = emit_host("elementrelease", source);
    assert_contains(
        "elementrelease",
        &emitted,
        "ClString __cl_itmp0[2] = {0};",
    );
    assert_contains(
        "elementrelease",
        &emitted,
        "cl_spec_mii_impl_elementrelease_two(__cl_itmp0);",
    );
    assert_contains(
        "elementrelease",
        &emitted,
        "if (__cl_itmp0[0]) { FreeString(__cl_itmp0[0]); }",
    );
    assert_contains(
        "elementrelease",
        &emitted,
        "if (__cl_itmp0[1]) { FreeString(__cl_itmp0[1]); }",
    );
}

/// A conditional arm's owned value binds under its guard and releases.
///
/// Only the chosen arm stores, and the releases run in reverse creation
/// order with a null guard, so the untaken arm's zero local frees nothing.
#[test]
fn should_release_a_guarded_arm_temporary() {
    let source = r#"module spec.mii.impl.armrelease;

import { stringOf } from std.text;

let c: bool = true;
let g: i32 = i32(len(c ? stringOf("a") : stringOf("bb")));

func main(): i32 {
    return g - 1;
}
"#;
    let emitted = emit_host("armrelease", source);
    let then_store = "if (__cl_itmp0) { __cl_itmp1 = cl_std_text_stringOf(cl_str(\"a\")); }";
    let else_store = "if (!__cl_itmp0) { __cl_itmp2 = cl_std_text_stringOf(cl_str(\"bb\")); }";
    assert_contains("armrelease", &emitted, then_store);
    assert_contains("armrelease", &emitted, else_store);
    let else_release = "    if (__cl_itmp2) { FreeString(__cl_itmp2); }";
    let then_release = "    if (__cl_itmp1) { FreeString(__cl_itmp1); }";
    let else_at = emitted
        .find(else_release)
        .unwrap_or_else(|| panic!("armrelease: the second arm's release is missing"));
    let then_at = emitted
        .find(then_release)
        .unwrap_or_else(|| panic!("armrelease: the first arm's release is missing"));
    assert!(
        else_at < then_at,
        "armrelease: the releases do not run in reverse creation order"
    );
}

/// A lend argument's owned temporary binds and releases after the call.
#[test]
fn should_release_a_lend_argument_temporary() {
    let source = r#"module spec.mii.impl.argrelease;

import { String, stringOf } from std.text;

func first(xs: String): i32 {
    return i32(len(xs));
}

let g: i32 = first(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("argrelease", source);
    assert_contains(
        "argrelease",
        &emitted,
        "ClString __cl_itmp0 = cl_std_text_stringOf(cl_str(\"abc\"));",
    );
    assert_contains(
        "argrelease",
        &emitted,
        "cl_spec_mii_impl_argrelease_g = cl_spec_mii_impl_argrelease_first(__cl_itmp0);",
    );
    assert_contains(
        "argrelease",
        &emitted,
        "if (__cl_itmp0) { FreeString(__cl_itmp0); }",
    );
}

/// A `take` argument's temporary does not release.
///
/// The callee consumes and frees the value, so the init function leaves its
/// local unreleased — a release here would double-free.
#[test]
fn should_not_release_a_take_argument_temporary() {
    let source = r#"module spec.mii.impl.takearg;

import { String, stringOf } from std.text;

func consume(take xs: String): i32 {
    return i32(len(xs));
}

let g: i32 = consume(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("takearg", source);
    assert_contains(
        "takearg",
        &emitted,
        "cl_spec_mii_impl_takearg_g = cl_spec_mii_impl_takearg_consume(cl_std_text_stringOf(cl_str(\"abc\")));",
    );
    assert_eq!(
        count(&emitted, "__cl_itmp"),
        0,
        "takearg: a consumed argument binds no init local, so nothing releases it"
    );
}

/// A body `sizeof` of a written slice type emits its typedef and use.
///
/// The written type argument joins the typedef set exactly as a value
/// position would, so the slice struct is defined before the function that
/// measures it.
#[test]
fn should_define_a_body_slice_type_argument_typedef() {
    let source = r#"module spec.mii.impl.bodyslice;

func size(): usize {
    let n: usize = sizeof(i32[]);
    return n;
}

func main(): i32 {
    if (size() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("bodyslice", source);
    assert_contains(
        "bodyslice",
        &emitted,
        "typedef struct { i32 *ptr; usize len; } ClSlice_i32;",
    );
    assert_contains("bodyslice", &emitted, "n = sizeof(ClSlice_i32);");
}

/// A written slice type argument in a store index emits its typedef.
///
/// The store's run-time index carries the written type's `sizeof`, so the
/// typedef is defined before the function that stores through it.
#[test]
fn should_define_a_store_index_slice_type_argument_typedef() {
    let source = r#"module spec.mii.impl.storeslice;

let position: usize = 0;

func main(): i32 {
    let xs: i32[4] = [1, 2, 3, 4];
    xs[position + sizeof(i32[]) - sizeof(i32[])] = 9;
    return xs[0] - 9;
}
"#;
    let emitted = emit_host("storeslice", source);
    assert_contains(
        "storeslice",
        &emitted,
        "typedef struct { i32 *ptr; usize len; } ClSlice_i32;",
    );
    assert_contains(
        "storeslice",
        &emitted,
        "__cl_tmp0 = ((cl_spec_mii_impl_storeslice_position + sizeof(ClSlice_i32)) - sizeof(ClSlice_i32));",
    );
}

/// The expression-callee initializer spellings and their func-value control.
///
/// `mk()` returns a func value and `c ? addOne : addOne` selects one, so the
/// two calls go through a value the emitted C must carry; the func-typed
/// global `f` is the plain-receiver control.
const EXPR_CALLEE_SOURCE: &str = r#"module spec.mii.impl.exprcallee;

func addOne(a: i32): i32 {
    return a + 1;
}

func mk(): func(i32): i32 {
    return addOne;
}

let c: bool = true;
let f: func(i32): i32 = addOne;
let h: i32 = mk()(5);
let j: i32 = (c ? addOne : addOne)(5);
let k: i32 = f(5);

func main(): i32 {
    return (h - 6) + (j - 6) + (k - 6);
}
"#;

/// An expression callee in an initializer is the call's callable value.
///
/// A call-result callee and a conditional over func values both lower to the
/// call's receiver, so the emitted C calls the value the callee produced
/// instead of collapsing the call into its arguments; the func-typed global
/// beside them keeps the plain receiver spelling.
#[test]
fn should_call_an_expression_callee_in_an_initializer() {
    let emitted = emit_host("exprcallee", EXPR_CALLEE_SOURCE);
    assert_contains(
        "exprcallee",
        &emitted,
        "cl_spec_mii_impl_exprcallee_h = (cl_spec_mii_impl_exprcallee_mk())(5);",
    );
    assert_contains(
        "exprcallee",
        &emitted,
        "cl_spec_mii_impl_exprcallee_j = ((cl_spec_mii_impl_exprcallee_c ? cl_spec_mii_impl_exprcallee_addOne : cl_spec_mii_impl_exprcallee_addOne))(5);",
    );
    assert_contains(
        "exprcallee",
        &emitted,
        "cl_spec_mii_impl_exprcallee_k = (cl_spec_mii_impl_exprcallee_f)(5);",
    );
    assert_eq!(
        count(&emitted, " = (5);"),
        0,
        "exprcallee: a collapsed call must not store its bare argument"
    );
}

/// An inferred generic initializer call reaches the keyed instance.
///
/// The inferred spelling derives its type arguments from the checked argument
/// types, so the call spells the same keyed symbol the explicit control does
/// and the declaration carries exactly one instance; the plain module symbol
/// must not appear.
#[test]
fn should_key_an_inferred_generic_initializer_call() {
    let source = r#"module spec.mii.impl.infergeneric;

func pick<T>(a: T, b: T): T {
    return a;
}

let g: i32 = pick(1, 2);
let h: i32 = pick<i32>(3, 4);

func main(): i32 {
    return (g - 1) + (h - 3);
}
"#;
    let emitted = emit_host("infergeneric", source);
    assert_contains(
        "infergeneric",
        &emitted,
        "static i32 cl_pick__i32(i32 a, i32 b);",
    );
    assert_contains(
        "infergeneric",
        &emitted,
        "cl_spec_mii_impl_infergeneric_g = cl_pick__i32(1, 2);",
    );
    assert_contains(
        "infergeneric",
        &emitted,
        "cl_spec_mii_impl_infergeneric_h = cl_pick__i32(3, 4);",
    );
    assert_eq!(
        count(&emitted, "static i32 cl_pick__i32(i32 a, i32 b)\n{"),
        1,
        "infergeneric: the instance must declare once"
    );
    assert_eq!(
        count(&emitted, "cl_spec_mii_impl_infergeneric_pick("),
        0,
        "infergeneric: the plain module symbol must not be emitted"
    );
}

/// The inferred view-builder initializer call spells its keyed instance.
///
/// The slice argument's element type keys the builder, the typed temporary
/// carries the keyed slice, and the explicit spelling beside it shares the
/// instance; the plain view-builder symbol must not appear.
#[test]
fn should_key_an_inferred_view_builder_initializer_call() {
    let source = r#"module spec.mii.impl.inferview;

import { slice } from std.mem;

let xs: i32[4] = [1, 2, 3, 4];

func viewOf<T>(v: T[]): T[] {
    return v;
}

let e: i32 = viewOf(slice(xs, 0, 4))[1];
let f: i32 = viewOf<i32>(slice(xs, 0, 4))[2];

func main(): i32 {
    return (e - 2) + (f - 3);
}
"#;
    let emitted = emit_host("inferview", source);
    assert_contains(
        "inferview",
        &emitted,
        "static ClSlice_i32 cl_viewOf__i32(ClSlice_i32 v);",
    );
    assert_contains(
        "inferview",
        &emitted,
        "ClSlice_i32 __cl_itmp0 = cl_viewOf__i32(((ClSlice_i32){ (cl_spec_mii_impl_inferview_xs) + (0), (usize)((4) - (0)) }));",
    );
    assert_eq!(
        count(&emitted, "cl_spec_mii_impl_inferview_viewOf("),
        0,
        "inferview: the plain view-builder symbol must not be emitted"
    );
}

/// The guarded `&&` bind beside its unguarded twin.
const GUARD_BIND_SOURCE: &str = r#"module spec.mii.impl.guardbind;

import { String, stringOf, bytes } from std.text;

let yes: bool = true;

func firstByte(s: String): u8 {
    return bytes(s)[0];
}

let hit: bool = yes && (firstByte(stringOf("ab")) == 97);
let plain: bool = firstByte(stringOf("ab")) == 97;

func main(): i32 {
    if (hit && plain) {
        return 0;
    }
    return 1;
}
"#;

/// A guarded owned bind releases inside its decision block.
///
/// The `&&` right side declares its owned temporary inside the block that
/// guards it, so the block ends with the null-guarded release in reverse
/// order; the unguarded bind beside it keeps the end-of-init release, and the
/// guarded bind has exactly one release outside nothing.
#[test]
fn should_release_a_guarded_initializer_bind_inside_its_block() {
    let emitted = emit_host("guardbind", GUARD_BIND_SOURCE);
    assert_contains(
        "guardbind",
        &emitted,
        concat!(
            "    if (cl_spec_mii_impl_guardbind_yes) {\n",
            "    ClString __cl_itmp1 = cl_std_text_stringOf(cl_str(\"ab\"));\n",
            "        __cl_itmp0 = (cl_spec_mii_impl_guardbind_firstByte(__cl_itmp1) == 97);\n",
            "    if (__cl_itmp1) { FreeString(__cl_itmp1); }\n",
            "    __cl_itmp1 = NULL;\n",
            "    }",
        ),
    );
    assert_eq!(
        count(&emitted, "if (__cl_itmp1)"),
        1,
        "guardbind: the guarded bind must release once, inside its block"
    );
    assert_contains(
        "guardbind",
        &emitted,
        "if (__cl_itmp2) { FreeString(__cl_itmp2); }",
    );
}

/// A nullable view conditional builds each arm's own view.
///
/// The module initializer wraps the owned arm and gives the null arm the zero
/// view; the body binds the owned handle into an init-scope owner first and
/// guards its store the same way, so the null arm never reaches the view
/// constructor. The whole-value wrapper must appear in neither program.
#[test]
fn should_build_a_nullable_view_conditional_per_arm() {
    let module_source = r#"module spec.mii.impl.viewarm;

import { stringOf } from std.text;

let c: bool = false;
let s: str? = c ? stringOf("abc") : null;

func main(): i32 {
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}
"#;
    let module_emitted = emit_host("viewarm", module_source);
    assert_contains(
        "viewarm",
        &module_emitted,
        "cl_spec_mii_impl_viewarm_s = (cl_spec_mii_impl_viewarm_c ? cl_string_view(cl_std_text_stringOf(cl_str(\"abc\"))) : cl_zero_ClStr());",
    );
    assert_eq!(
        count(&module_emitted, "cl_string_view(("),
        0,
        "viewarm: the whole-conditional view wrapper must not be emitted"
    );
    let body_source = r#"module spec.mii.impl.viewarmbody;

import { stringOf } from std.text;

func code(c: bool): i32 {
    let s: str? = c ? stringOf("abc") : null;
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}

func main(): i32 {
    return code(false);
}
"#;
    let body_emitted = emit_host("viewarmbody", body_source);
    assert_contains(
        "viewarmbody",
        &body_emitted,
        "s = (__cl_owned0 ? cl_string_view(__cl_owned0) : cl_zero_ClStr());",
    );
    assert_eq!(
        count(&body_emitted, "cl_string_view(("),
        0,
        "viewarmbody: the whole-conditional view wrapper must not be emitted"
    );
}

/// The resolved-call and guarded-bind programs re-emit byte-identically.
///
/// Both programs re-emit from one source path so their `#line` facts agree,
/// and the re-emitted text keeps the receiver-call and in-block release
/// shapes.
#[test]
fn should_re_emit_the_resolved_call_and_guard_programs_byte_identically() {
    for (tag, source, shape) in [
        (
            "exprcallee",
            EXPR_CALLEE_SOURCE,
            "cl_spec_mii_impl_exprcallee_h = (cl_spec_mii_impl_exprcallee_mk())(5);",
        ),
        (
            "guardbind",
            GUARD_BIND_SOURCE,
            "if (__cl_itmp1) { FreeString(__cl_itmp1); }",
        ),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
        assert_contains(tag, &first, shape);
    }
}

/// A receiver-bound generic dot call keys the receiver's type.
///
/// `xs.mergeBox(yi, zj)` passes `xs` as the call's argument position zero, so
/// the inferred instance binds `A` from the receiver instead of shifting the
/// written arguments into its place; the receiver-blind key must not appear.
/// The nullable-slice twin is the spurious-instance guard: `value.prep(...)`
/// keys `T` from its receiver, so no dead `slice`-keyed instance is recorded.
#[test]
fn should_key_a_receiver_bound_generic_dot_call() {
    let source = r#"module spec.mii.impl.recvkey;

struct Box<T> { tag: i32; }

func mergeBox<A, B>(self: Box<A>, x: Box<B>, y: Box<B>): i32 {
    return x.tag + y.tag;
}

let xs: Box<str> = Box<str> { tag: 1 };
let yi: Box<i32> = Box<i32> { tag: 2 };
let zj: Box<i32> = Box<i32> { tag: 3 };
let r: i32 = xs.mergeBox(yi, zj);

func main(): i32 {
    return r - 5;
}
"#;
    let emitted = emit_host("recvkey", source);
    assert_contains(
        "recvkey",
        &emitted,
        "static i32 cl_mergeBox__str_i32(ClBox__str self, ClBox__i32 x, ClBox__i32 y);",
    );
    assert_contains(
        "recvkey",
        &emitted,
        "cl_spec_mii_impl_recvkey_r = cl_mergeBox__str_i32(cl_spec_mii_impl_recvkey_xs, cl_spec_mii_impl_recvkey_yi, cl_spec_mii_impl_recvkey_zj);",
    );
    assert_eq!(
        count(&emitted, "cl_mergeBox__i32_i32"),
        0,
        "recvkey: the receiver-blind instance key must not be emitted"
    );

    let guard = r#"module spec.mii.impl.recvguard;

func prep<T>(take v: T, borrow slot: T[]?): void {
    slot = null;
}

func main(): i32 {
    let value: i32 = 1;
    let xs: i32[]? = null;
    value.prep(borrow xs);
    return 0;
}
"#;
    let emitted = emit_host("recvguard", guard);
    assert_contains(
        "recvguard",
        &emitted,
        "static void cl_prep__i32(i32 v, ClSlice_i32 *slot);",
    );
    assert_eq!(
        count(&emitted, "__slice_i32Q"),
        0,
        "recvguard: a shifted argument must not key a dead slice instance"
    );
}
