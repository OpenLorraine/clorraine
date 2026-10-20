//! Implementation tests for the borrow/generic integrity batch.
//!
//! These cases pin the observable internals of the fixes: the emitted C shape
//! of an accepted nullable out-slot store, the diagnostic counts of a store
//! site that the checker visits on more than one pass, the C typedef and
//! symbol spellings of instances that differ only by nullability, and the
//! lowered pointer test of a view-versus-`null` comparison with the store and
//! read-back through the narrowed view. They complement the specification
//! suite, which pins the accepted and rejected behavior.

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
            "clc-borrowgenericimpl-{}-{tag}-{sequence}",
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

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// Asserts the text contains a fragment.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: the emitted C does not contain {needle:?}"
    );
}

/// Asserts the first fragment appears before the second one in the text.
fn assert_before(tag: &str, text: &str, first: &str, second: &str) {
    let first_at = text
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {first:?}"));
    let second_at = text
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {second:?}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}"
    );
}

/// Counts the occurrences of a fragment.
fn count_occurrences(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

/// A nullable out-slot store is emitted as a pointer write with one release.
///
/// The callee receives the address of the caller's slot, so the definition
/// declares a pointer parameter and the call site passes the slot's address.
/// The store releases the callee-owned incoming value, zeroes the slot, stores
/// the new value through the pointer, and empties the moved source, in that
/// order. A store that skipped the release would leak the incoming value; one
/// that released after the store would free the value it just wrote.
#[test]
fn should_emit_a_pointer_store_when_a_nullable_out_slot_receives_a_value() {
    let source = r#"module spec.bgi.impl.outslot;

import { String, stringOf } from std.text;

func fillString(take v: String, borrow slot: String?): void {
    slot = v;
}

func main(): i32 {
    let slot: String? = null;
    fillString(stringOf("xyz"), borrow slot);
    if (slot == null) {
        return 9;
    }
    return i32(len(slot));
}
"#;
    let emitted = emit_host("out-slot-store", source);
    assert_contains("out-slot-store", &emitted, "ClString *slot)");
    assert_contains("out-slot-store", &emitted, "(*slot) = v;");
    assert_contains("out-slot-store", &emitted, "&(slot)");
    assert_before(
        "out-slot-store",
        &emitted,
        "FreeString((*slot))",
        "(*slot) = v;",
    );
    assert_before("out-slot-store", &emitted, "(*slot) = v;", "v = NULL;");
}

/// A parameter-typed store inside a loop reports each diagnostic once.
///
/// The checker walks a loop body on a speculative pass and on the reporting
/// pass, so a duty recorded at the store would be judged twice if the site did
/// not dedupe. The program is rejected with exactly one `E10034` for the store
/// and exactly one `E10020` for the source the loop's second iteration may
/// have moved; a duplicated count means the site was judged once per pass.
#[test]
fn should_report_one_slot_error_when_a_loop_body_stores_into_a_parameter_typed_slot() {
    let source = r#"module spec.bgi.impl.loopstore;

import { String, stringOf } from std.text;

func fill<T>(take v: T, borrow slot: T, flag: bool): void {
    let i: i32 = 0;
    while (i < 1) {
        i = i + 1;
        if (flag) {
            slot = v;
        }
    }
}

func main(): i32 {
    let s: String = stringOf("ab");
    fill<String>(stringOf("xyz"), borrow s, true);
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("loop-store", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "loop-store: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        1,
        "loop-store: expected exactly one E10034; stderr:\n{stderr}"
    );
    assert_eq!(
        count_occurrences(&stderr, "error E10020:"),
        1,
        "loop-store: expected exactly one E10020; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "loop-store: a rejected program must not produce a translation unit"
    );
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

/// Struct instances that differ only by nullability get their own C typedefs.
///
/// The marker is part of the key, so the two instances never share a
/// definition; the strict host build sees one complete type per instance. The
/// field spelling itself is unchanged: a nullable handle is still the same C
/// pointer type.
#[test]
fn should_name_distinct_type_definitions_when_struct_instances_differ_only_by_nullability() {
    let source = r#"module spec.bgi.impl.keysstructs;

import { String, stringOf } from std.text;

struct Wrap<T> {
    v: T;
}

func main(): i32 {
    let a: Wrap<String> = Wrap<String> { v: stringOf("ab") };
    let b: Wrap<String?> = Wrap<String?> { v: stringOf("cde") };
    if (b.v == null) {
        return 9;
    }
    return i32(len(a.v)) + i32(len(b.v));
}
"#;
    let emitted = emit_host("keys-structs-shape", source);
    assert_contains(
        "keys-structs-shape",
        &emitted,
        "typedef struct ClWrap__String ClWrap__String;",
    );
    assert_contains(
        "keys-structs-shape",
        &emitted,
        "typedef struct ClWrap__StringQ ClWrap__StringQ;",
    );
    assert_eq!(
        count_occurrences(&emitted, "struct ClWrap__String {"),
        1,
        "keys-structs-shape: expected one non-nullable definition"
    );
    assert_eq!(
        count_occurrences(&emitted, "struct ClWrap__StringQ {"),
        1,
        "keys-structs-shape: expected one nullable definition"
    );
}

/// Top-level function-instance symbols keep their exact spellings.
///
/// A nullable top-level type argument already keyed the marker before the key
/// change, so the symbol spelling must stay byte-identical. The pinned
/// prototypes catch any drift in the key rendering.
#[test]
fn should_keep_top_level_function_instance_symbols_unchanged_when_nullability_differs() {
    let source = r#"module spec.bgi.impl.keyspicktop;

import { String, stringOf } from std.text;

func pick<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let one: i32 = pick<String>(stringOf("ab"), stringOf("cd"));
    let two: i32 = pick<String?>(stringOf("ef"), null);
    return one + two + 4;
}
"#;
    let emitted = emit_host("keys-pick-top-shape", source);
    assert_contains(
        "keys-pick-top-shape",
        &emitted,
        "static i32 cl_pick__String(ClString a, ClString b)",
    );
    assert_contains(
        "keys-pick-top-shape",
        &emitted,
        "static i32 cl_pick__StringQ(ClString a, ClString b)",
    );
}

/// A nested nullable argument carries its marker inside the instance symbol.
///
/// The marker applies where the nullable type sits, so the nested pair keys
/// `cl_pick__Wrap__String` and `cl_pick__Wrap__StringQ` while the top-level
/// pair keys `cl_pick__String` and `cl_pick__StringQ` — the three-argument
/// form would collide if the marker were only checked at the top level.
///
/// Limitation: the marker is not distinguishable from a user identifier that
/// ends in `Q`, so a type named `StringQ` would key the same spelling as
/// `String?`. That ambiguity is a known property of the marker spelling,
/// shared with the function-instance symbols, and is recorded rather than
/// fixed here.
#[test]
fn should_name_the_nested_instance_with_its_own_nullable_marker() {
    let source = r#"module spec.bgi.impl.keysnested;

import { String, stringOf } from std.text;

struct Wrap<T> {
    v: T;
}

func pick<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let a1: Wrap<String> = Wrap<String> { v: stringOf("ab") };
    let a2: Wrap<String> = Wrap<String> { v: stringOf("cd") };
    let one: i32 = pick<Wrap<String>>(a1, a2);
    let b1: Wrap<String?> = Wrap<String?> { v: stringOf("ef") };
    let b2: Wrap<String?> = Wrap<String?> { v: null };
    let two: i32 = pick<Wrap<String?>>(b1, b2);
    let s1: i32 = pick<String>(stringOf("gh"), stringOf("ij"));
    let s2: i32 = pick<String?>(stringOf("kl"), null);
    return one + two + s1 + s2 + 4;
}
"#;
    let emitted = emit_host("keys-nested-shape", source);
    assert_contains(
        "keys-nested-shape",
        &emitted,
        "static i32 cl_pick__Wrap__String(ClWrap__String a, ClWrap__String b)",
    );
    assert_contains(
        "keys-nested-shape",
        &emitted,
        "static i32 cl_pick__Wrap__StringQ(ClWrap__StringQ a, ClWrap__StringQ b)",
    );
    assert_contains(
        "keys-nested-shape",
        &emitted,
        "static i32 cl_pick__StringQ(ClString a, ClString b)",
    );
}

/// A nested nullable-instance program re-emits byte-identically.
///
/// The marker must not depend on emission order or map iteration, so two
/// emissions over the same source path produce the same translation unit.
#[test]
fn should_re_emit_identically_when_a_nested_nullable_instance_is_emitted_twice() {
    let source = r#"module spec.bgi.impl.keysdeterminism;

import { String, stringOf } from std.text;

struct Wrap<T> {
    v: T;
}

func main(): i32 {
    let a1: Wrap<String> = Wrap<String> { v: stringOf("ab") };
    let b1: Wrap<String?> = Wrap<String?> { v: stringOf("cd") };
    if (b1.v == null) {
        return 9;
    }
    return i32(len(a1.v)) + i32(len(b1.v));
}
"#;
    let (first, second) = emit_twice("keys-reemit", source);
    assert_eq!(
        first, second,
        "keys-reemit: two emissions of one program must be byte-identical"
    );
}

/// A func-typed parameter named like a module lender keeps its masked
/// spelling on the signature and the call.
///
/// The parameter is emitted as `give_0` in both the prototype and the
/// definition, and the body calls it through that C local; the same-named
/// module function keeps its own symbol and must not be reached. The
/// function-shadowing pins in the runtime and call-value implementation
/// suites carry the same masked spellings.
#[test]
fn should_name_a_colliding_func_typed_parameter_with_its_masked_spelling() {
    let source = r#"module spec.bgi.impl.p4maskfunc;

import { String, stringOf } from std.text;

func give(borrow v: str): str {
    return v;
}

func copier(borrow v: String): String {
    return stringOf("y");
}

func demo(give: func(borrow String): String): i32 {
    let s: String = stringOf("x");
    return i32(len(give(s)));
}

func main(): i32 {
    return demo(copier);
}
"#;
    let emitted = emit_host("p4-mask-func", source);
    assert_contains("p4-mask-func", &emitted, "(ClString (*give_0)(ClString))");
    assert_contains("p4-mask-func", &emitted, "give_0(s);");
    assert!(
        !emitted.contains("cl_spec_bgi_impl_p4maskfunc_give(s)"),
        "p4-mask-func: the parameter call must not reach the module lender"
    );
}

/// A colliding non-func parameter agrees on its masked spelling everywhere.
///
/// The prototype, the definition, and the body all spell the parameter
/// `give_0`, so the strict build cannot see a signature/body mismatch; the
/// module call in the sibling function still resolves to the lender's symbol.
#[test]
fn should_agree_on_a_colliding_parameter_spelling_across_its_signature_and_body() {
    let source = r#"module spec.bgi.impl.p4masksibling;

func give(v: i32): i32 {
    return v + 1;
}

func useModule(): i32 {
    return give(41);
}

func sibling(give: i32): i32 {
    return give;
}

func main(): i32 {
    return useModule() + sibling(1);
}
"#;
    let emitted = emit_host("p4-mask-sibling", source);
    assert_contains(
        "p4-mask-sibling",
        &emitted,
        "static i32 cl_spec_bgi_impl_p4masksibling_sibling(i32 give_0);",
    );
    assert_contains("p4-mask-sibling", &emitted, "return give_0;");
    assert_contains(
        "p4-mask-sibling",
        &emitted,
        "return cl_spec_bgi_impl_p4masksibling_give(41);",
    );
}

/// Parameter C names stay unique when a masked spelling meets a literal one.
///
/// The masked parameter takes `give_0` and the literally-named sibling takes
/// the de-duplicated `give_0_1`, in the signature and in the body, so two
/// distinct IR bindings can never share a C identifier.
#[test]
fn should_de_duplicate_parameter_names_when_a_masked_spelling_collides() {
    let source = r#"module spec.bgi.impl.p4paramdedup;

func give(v: i32): i32 {
    return v + 1;
}

func demo(give: i32, give_0: i32): i32 {
    return give + give_0;
}

func main(): i32 {
    return demo(1, 2);
}
"#;
    let emitted = emit_host("p4-param-dedup", source);
    assert_contains("p4-param-dedup", &emitted, "(i32 give_0, i32 give_0_1)");
    assert_contains(
        "p4-param-dedup",
        &emitted,
        "return (i32)((u32)give_0 + (u32)give_0_1);",
    );
}

/// A view-versus-`null` comparison lowers to a pointer test in both orders.
///
/// The emitter renders the emptiness test on the view's `ptr` field whatever
/// the operand order, so `xs == null` and `null == ys` produce the same shape
/// over their own operand. The `!=` form is the negated test, and the element
/// store and read-back inside the narrowed branch keep the pointer spelling
/// with the bounds trap between them.
#[test]
fn should_emit_pointer_null_tests_when_a_view_is_compared_with_null() {
    let source = r#"module spec.bgi.impl.viewshape;

import { slice } from std.mem;

func check(borrow xs: u8[]?, borrow ys: u8[]?): i32 {
    let n: i32 = 0;
    if (xs == null) {
        n = n + 1;
    }
    if (null == ys) {
        n = n + 2;
    }
    if (xs != null) {
        xs[0] = 7;
        n = n + i32(xs[0]);
    }
    return n;
}

func main(): i32 {
    let data: u8[2] = [1, 2];
    let live: u8[]? = slice(data, 0, 2);
    let empty: u8[]? = null;
    return check(borrow live, borrow empty);
}
"#;
    let emitted = emit_host("view-shape", source);
    assert_contains("view-shape", &emitted, "(((*xs)).ptr == NULL)");
    assert_contains("view-shape", &emitted, "(((*ys)).ptr == NULL)");
    assert_contains("view-shape", &emitted, "(((*xs)).ptr != NULL)");
    assert_contains("view-shape", &emitted, "((*xs)).ptr[0] = 7u;");
    assert_eq!(
        count_occurrences(&emitted, "((*xs)).ptr[0]"),
        2,
        "view-shape: expected the element store and its read-back"
    );
}

/// Two slice views report the operator rule once per operator use.
///
/// Equality and ordering on a pair of slice views have no defined meaning, so
/// each use reports `E10222` on the view operand; the acceptance for the
/// `null` comparison must not weaken either report.
#[test]
fn should_report_view_pair_rejections_once_per_operator_use() {
    let source = r#"module spec.bgi.impl.viewpair;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let b: u8[2] = [3, 4];
    let xs: u8[] = slice(a, 0, 2);
    let ys: u8[] = slice(b, 0, 2);
    if (xs == ys) {
        return 1;
    }
    if (xs < ys) {
        return 2;
    }
    return 0;
}
"#;
    let (scratch, output, c_path) = compile_only("view-pair", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "view-pair: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10222:"),
        2,
        "view-pair: expected one report per operator use; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("operator '==' is not defined"),
        "view-pair: expected the equality report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("operator '<' is not defined"),
        "view-pair: expected the ordering report; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "view-pair: a rejected program must not produce a translation unit"
    );
    drop(scratch);
}

/// A generic conditional store reports once at the owning instance.
///
/// The report is the concrete store site (`:6:5`) and no translation unit is
/// written. A body with no owning instantiation is accepted and emits, and the
/// duplicable whole-move instance runs.
#[test]
fn should_report_the_generic_conditional_store_once_at_the_instance() {
    let tag = "generic-conditional-store";
    let source = r#"module spec.bgiimpl.genericconditional;

import { String, stringOf } from std.text;

func pick<T>(c: bool, h: T): i32 {
    let r: String? = c ? h : null;
    if (r == null) { return 1; }
    return 0;
}

func main(): i32 {
    let h: String = stringOf("ab");
    return pick<String>(true, h);
}
"#;
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the owning instance must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        1,
        "{tag}: expected one E10034; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'h'"),
        "{tag}: the report must name the lend parameter; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(":6:5"),
        "{tag}: the report must sit at the store site; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected instance must not produce a translation unit"
    );

    let definition = r#"module spec.bgiimpl.genericdefinition;

func pick<T>(c: bool, h: T): i32 {
    let r: String? = c ? h : null;
    if (r == null) { return 1; }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("generic-definition", definition);
    assert_eq!(
        output.status.code(),
        Some(0),
        "generic-definition: the definition must not be reported; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        c_path.exists(),
        "generic-definition: an accepted definition must emit"
    );

    let duplicable = r#"module spec.bgiimpl.genericduplicable;

struct P { x: i32; }

func pick<T>(h: T): i32 {
    let r: T = h;
    return r.x;
}

func main(): i32 {
    let h: P = P { x: 4 };
    return pick<P>(h) - 4;
}
"#;
    let emitted = emit_host("generic-duplicable", duplicable);
    assert_contains("generic-duplicable", &emitted, "cl_pick__P");
}

/// The batch-P instance duties report once per selected lend arm.
///
/// The generic take, raw field, struct-literal field, and projected assignment
/// report one `E10034`; the nested projection reports two (one per inner lend
/// arm). A duplicable instance re-emits byte-identically.
#[test]
fn should_report_the_generic_conditional_duties_once_per_selected_arm() {
    let rejected = |tag: &str, source: &str, expected: usize| {
        let (_scratch, output, c_path) = compile_only(tag, source);
        assert_ne!(
            output.status.code(),
            Some(0),
            "{tag}: the front end accepted a program that must be rejected"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            count_occurrences(&stderr, "error E10034:"),
            expected,
            "{tag}: unexpected E10034 count; stderr:\n{stderr}"
        );
        assert!(
            !c_path.exists(),
            "{tag}: a rejected program must not produce a translation unit"
        );
    };

    let take = r#"module impl.bgi.ptake;

import { String, stringOf } from std.text;

struct H { name: String; tag: i32; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, h: T): i32 { return sink(take (c ? h.name : null)); }

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: 1 };
    return f<H>(true, h);
}
"#;
    rejected("ptake", take, 1);

    let raw = r#"module impl.bgi.praw;

import { String, stringOf } from std.text;

struct H { name: String; tag: i32; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, xs: T): i32 { return sink(take (c ? (unsafe xs.name) : null)); }

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: 1 };
    let p: raw H = unsafe &h;
    return f<raw H>(true, p);
}
"#;
    rejected("praw", raw, 1);

    let field = r#"module impl.bgi.pfield;

import { String, stringOf } from std.text;

struct H { name: String; }
struct S { name: String?; tag: i32; }

func f<T>(c: bool, h: T): i32 {
    let s: S = S { name: c ? h.name : null, tag: 1 };
    if (s.name == null) { return 9; }
    return i32(len(s.name)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    rejected("pfield", field, 1);

    let projection = r#"module impl.bgi.pprojection;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, d: bool, h1: T, h2: T): String? {
    return c ? (d ? h1 : h2).name : null;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    let r: String? = f<H>(true, true, h1, h2);
    if (r == null) { return 9; }
    return i32(len(r)) - 2;
}
"#;
    rejected("pprojection", projection, 2);

    let accepted = r#"module impl.bgi.paccepted;

struct HD { p: raw u8; }

func sink(take s: raw u8?): i32 { return 0; }

func f<T>(c: bool, h: T): i32 { return sink(take (c ? h.p : null)); }

func main(): i32 {
    let v: u8 = 1;
    let h: HD = HD { p: unsafe &v };
    return f<HD>(true, h);
}
"#;
    let (first, second) = emit_twice("pgenericaccepted", accepted);
    assert_eq!(
        first, second,
        "pgenericaccepted: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// A bare projection over a type-parameter conditional reports every inner
/// lend arm, and a duplicable member later read stays accepted.
#[test]
fn should_report_each_bare_projection_arm_at_every_consumed_position() {
    let take = r#"module impl.bgi.pbaretake;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(d: bool, h1: T, h2: T): i32 { return sink(take ((d ? h1 : h2).name)); }

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    return f<H>(true, h1, h2);
}
"#;
    let (_scratch, output, c_path) = compile_only("pbaretake", take);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pbaretake: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        2,
        "pbaretake: expected two E10034; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pbaretake: a rejected program must not produce a translation unit"
    );

    let binding = r#"module impl.bgi.pbarebinding;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(d: bool, h1: T, h2: T): i32 {
    let s: String? = (d ? h1 : h2).name;
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    return f<H>(true, h1, h2);
}
"#;
    let (_scratch, output, c_path) = compile_only("pbarebinding", binding);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pbarebinding: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        2,
        "pbarebinding: expected two E10034; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pbarebinding: a rejected program must not produce a translation unit"
    );

    let accepted = r#"module impl.bgi.pbaredup;

import { String, stringOf } from std.text;

struct H { name: String; tag: raw u8; }

func f<T>(c: bool, h: T): i32 {
    let s: raw u8? = c ? h.tag : null;
    if (h.tag == null) { return 0; }
    return 1;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: null };
    return f<H>(true, h);
}
"#;
    let (first, second) = emit_twice("pbaredup", accepted);
    assert_eq!(
        first, second,
        "pbaredup: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

// ---------------------------------------------------------------------------
// Generic conditional wrappers and moved state (Batch Q)
// ---------------------------------------------------------------------------

/// The generic conditional take fills one carrier the instance types.
///
/// Each arm moves the selected member out of the `take` parameter into a
/// `String` carrier, and the post-call move-out empties the carrier. The bare
/// and `unsafe`-wrapped spellings emit the same shape and re-emit
/// byte-identically.
#[test]
fn should_emit_the_per_arm_move_out_for_a_conditional_take() {
    let bare = r#"module impl.bgi.pqtake;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, take h: T): i32 { return sink(take (c ? h.name : null)); }

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let (first, second) = emit_twice("pqtake", bare);
    assert_eq!(
        first, second,
        "pqtake: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert_contains(
        "pqtake",
        &first,
        "ClString __cl_tmp0 = NULL;",
    );
    assert!(
        count_occurrences(&first, "h.name = NULL;") >= 3,
        "pqtake: expected the two arm move-outs and the cleanup; unit:\n{first}"
    );
    assert_contains("pqtake", &first, "__cl_tmp0 = h.name;");
    assert_contains("pqtake", &first, "__cl_tmp0 = NULL;");

    let wrapped = r#"module impl.bgi.pqtakeunsafe;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, take h: T): i32 {
    return sink(take (unsafe (c ? h.name : null)));
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let (first, second) = emit_twice("pqtakeunsafe", wrapped);
    assert_eq!(
        first, second,
        "pqtakeunsafe: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert_contains("pqtakeunsafe", &first, "ClString __cl_tmp0 = NULL;");
    assert!(
        count_occurrences(&first, "h.name = NULL;") >= 3,
        "pqtakeunsafe: expected the two arm move-outs and the cleanup; unit:\n{first}"
    );
}

/// The owning array element and the wrapped generic source report once.
#[test]
fn should_report_the_generic_array_and_wrapped_source_once() {
    let element = r#"module impl.bgi.pqarrayelem;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, h: T): i32 {
    let arr: String[1] = [c ? h.name : h.name];
    return i32(len(arr[0])) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let (_scratch, output, c_path) = compile_only("pqarrayelem", element);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pqarrayelem: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        1,
        "pqarrayelem: expected one E10034; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot release or store borrowed parameter 'h'"),
        "pqarrayelem: expected the lend report; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pqarrayelem: a rejected program must not produce a translation unit"
    );

    let wrapped = r#"module impl.bgi.pqwrapped;

import { String, stringOf } from std.text;

struct H { name: String; }
struct S { name: String?; tag: i32; }

func f<T>(c: bool, h: T): i32 {
    let s: S = S { name: unsafe (c ? h.name : null), tag: 1 };
    if (s.name == null) { return 9; }
    return i32(len(s.name)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let (_scratch, output, c_path) = compile_only("pqwrapped", wrapped);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pqwrapped: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        1,
        "pqwrapped: expected one E10034; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pqwrapped: a rejected program must not produce a translation unit"
    );
}

/// The nested write reports at the write and at the later read.
///
/// Both use sites report `E10020` naming the moved field, in the generic body
/// and in its concrete twin. The whole-field re-initialization control keeps
/// its assignments and re-emits byte-identically.
#[test]
fn should_report_a_nested_write_below_a_moved_field_at_each_use() {
    let generic = r#"module impl.bgi.pqnestedwrite;

import { String, stringOf } from std.text;

struct Inner { x: i32; s: String; }
struct H { name: Inner; }

func f<T>(take h: T): i32 {
    let n: Inner = h.name;
    h.name.x = 7;
    return i32(len(h.name.s)) + n.x;
}

func main(): i32 {
    let h: H = H { name: Inner { x: 1, s: stringOf("ab") } };
    return f<H>(h);
}
"#;
    let (_scratch, output, c_path) = compile_only("pqnestedwrite", generic);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pqnestedwrite: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10020:"),
        2,
        "pqnestedwrite: expected the write and the later read; stderr:\n{stderr}"
    );
    assert_eq!(
        count_occurrences(&stderr, "'h.name' was moved and cannot be used"),
        2,
        "pqnestedwrite: expected both reports to name the moved field; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pqnestedwrite: a rejected program must not produce a translation unit"
    );

    let concrete = r#"module impl.bgi.pqnestedwriteconc;

import { String, stringOf } from std.text;

struct Inner { x: i32; s: String; }
struct H { name: Inner; }

func f(take h: H): i32 {
    let n: Inner = h.name;
    h.name.x = 7;
    return i32(len(h.name.s)) + n.x;
}

func main(): i32 {
    let h: H = H { name: Inner { x: 1, s: stringOf("ab") } };
    return f(h);
}
"#;
    let (_scratch, output, c_path) = compile_only("pqnestedwriteconc", concrete);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pqnestedwriteconc: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10020:"),
        2,
        "pqnestedwriteconc: expected the write and the later read; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pqnestedwriteconc: a rejected program must not produce a translation unit"
    );

    let control = r#"module impl.bgi.pqnestedinit;

import { String, stringOf } from std.text;

struct Inner { x: i32; s: String; }
struct H { name: Inner; }

func f(take h: H): i32 {
    let n: Inner = h.name;
    h.name = Inner { x: 7, s: stringOf("cd") };
    return i32(len(h.name.s)) + n.x;
}

func main(): i32 {
    let h: H = H { name: Inner { x: 1, s: stringOf("ab") } };
    return f(h) - 3;
}
"#;
    let (first, second) = emit_twice("pqnestedinit", control);
    assert_eq!(
        first, second,
        "pqnestedinit: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert_contains("pqnestedinit", &first, "h.name = (ClInner)");

    // A run-time element write clears the narrowing of every element at that
    // position, so the later constant-index read reports the null use.
    let runtime_index = r#"module impl.bgi.pqruntimeindex;

import { String, stringOf } from std.text;

struct Cell { v: String?; }
struct H { cells: Cell[2]; }

func main(): i32 {
    let h: H = H { cells: [Cell { v: stringOf("aa") }, Cell { v: null }] };
    let j: i32 = 1;
    if (h.cells[0].v != null) {
        h.cells[j].v = null;
        return i32(len(h.cells[0].v));
    }
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("pqruntimeindex", runtime_index);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pqruntimeindex: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10010:"),
        1,
        "pqruntimeindex: expected the cleared narrowing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("may be null"),
        "pqruntimeindex: expected the null-use report; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pqruntimeindex: a rejected program must not produce a translation unit"
    );
}

/// The null-first and nested conditional takes emit the concrete carrier.
///
/// The null arm may lead, and an owning arm may itself be a conditional. The
/// temporary stays a `String` carrier (never the opaque `i32` fallback), the
/// selected member moves out, and the unit re-emits byte-identically.
#[test]
fn should_emit_the_null_first_and_nested_conditional_takes() {
    let reversed = r#"module impl.bgi.pqtakefirstnull;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, take h: T): i32 { return sink(take (c ? null : h.name)); }

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(false, h);
}
"#;
    let (first, second) = emit_twice("pqtakefirstnull", reversed);
    assert_eq!(
        first, second,
        "pqtakefirstnull: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert_contains("pqtakefirstnull", &first, "ClString __cl_tmp0 = NULL;");
    assert_contains("pqtakefirstnull", &first, "__cl_tmp0 = h.name;");

    let nested = r#"module impl.bgi.pqtakenested;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, d: bool, take h: T): i32 {
    return sink(take (c ? h.name : (d ? h.name : null)));
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, true, h);
}
"#;
    let (first, second) = emit_twice("pqtakenested", nested);
    assert_eq!(
        first, second,
        "pqtakenested: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert_contains("pqtakenested", &first, "ClString __cl_tmp0 = NULL;");
    assert!(
        count_occurrences(&first, "h.name = NULL;") >= 4,
        "pqtakenested: expected both nested arms and the cleanups; unit:\n{first}"
    );
    assert_contains("pqtakenested", &first, "__cl_tmp0 = NULL;");

    let mixed = r#"module impl.bgi.pqtakemixed;

import { String, stringOf } from std.text;

struct H { name: String; }
struct HN { name: String?; }

func sink(take s: String?): i32 { return 0; }

func f<T, U>(c: bool, take h: T, take g: U): i32 {
    return sink(take (c ? h.name : g.name));
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    let g: HN = HN { name: stringOf("bb") };
    return f<H, HN>(true, h, g);
}
"#;
    let (first, second) = emit_twice("pqtakemixed", mixed);
    assert_eq!(
        first, second,
        "pqtakemixed: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert_contains("pqtakemixed", &first, "ClString __cl_tmp0 = NULL;");
    assert!(
        count_occurrences(&first, "h.name = NULL;") >= 2,
        "pqtakemixed: expected the h arm and the cleanup; unit:\n{first}"
    );
    assert!(
        count_occurrences(&first, "g.name = NULL;") >= 2,
        "pqtakemixed: expected the g arm and the cleanup; unit:\n{first}"
    );
}

/// The unsafe-wrapped generic conditional reports once at a binding, a
/// `const`, and a whole assignment.
#[test]
fn should_report_the_wrapped_generic_binding_and_assignment_once() {
    let binding = r#"module impl.bgi.pqunsafebind;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, h: T): i32 {
    let s: String? = unsafe (c ? h.name : null);
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let (_scratch, output, c_path) = compile_only("pqunsafebind", binding);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pqunsafebind: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        1,
        "pqunsafebind: expected one E10034; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pqunsafebind: a rejected program must not produce a translation unit"
    );

    let assignment = r#"module impl.bgi.pqunsafeassign;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, h: T): i32 {
    let s: String? = null;
    s = unsafe (c ? h.name : null);
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let (_scratch, output, c_path) = compile_only("pqunsafeassign", assignment);
    assert_ne!(
        output.status.code(),
        Some(0),
        "pqunsafeassign: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        1,
        "pqunsafeassign: expected one E10034; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "pqunsafeassign: a rejected program must not produce a translation unit"
    );
}

/// A generic dot call emits its substituted return type.
///
/// The callee is `pick<T>(...): T` at `T = String`, so the result temporary is
/// the `String` handle, never `ClT`. The explicit-argument route reports the
/// direct-call codes once.
#[test]
fn should_emit_the_generic_dot_calls_substituted_result() {
    let returned = r#"module spec.bgi.impl.dotreturn;

import { String, stringOf } from std.text;

struct Wrap<T> { v: T; }

func pick<T>(take a: Wrap<T>, take b: T, take c: T): T { return a.v; }

func main(): i32 {
    let s1: String = stringOf("ab");
    let s2: String = stringOf("cd");
    let s3: String = stringOf("ef");
    let w: Wrap<String> = Wrap<String> { v: s1 };
    let x: String = w.pick(s2, s3);
    return i32(len(x)) - 2;
}
"#;
    let (first, second) = emit_twice("dotreturn", returned);
    assert_eq!(
        first, second,
        "dotreturn: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert!(
        !first.contains("ClT "),
        "dotreturn: the result temporary kept the declared parameter type"
    );
    assert!(
        first.contains("ClString __cl_tmp"),
        "dotreturn: the result temporary is not the substituted String"
    );

    let mismatch = r#"module spec.bgi.impl.dotexplicit;

struct Box { v: i32; }

func pick<T>(take b: Box, take x: T): T { return x; }

func main(): i32 {
    let b: Box = Box { v: 1 };
    let x: str = b.pick<str>(5);
    return i32(x[0]) - 53;
}
"#;
    let (_scratch, output, c_path) = compile_only("dotexplicit", mismatch);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10200:"),
        1,
        "dotexplicit: expected one conversion report; stderr:\n{stderr}"
    );
    assert!(!c_path.exists());
}

/// A generic dot call reports the direct call's inference and bound codes.
///
/// An uninferable parameter is `E10301` once, and a concrete argument that
/// violates a callable bound is `E10303` once. Neither writes a translation
/// unit.
#[test]
fn should_report_a_generic_dot_calls_inference_and_bound() {
    let unbound = r#"module spec.bgi.impl.dotunbound;

func fail<T>(x: i32): T { return 0; }

func main(): i32 {
    let n: i32 = 1.fail();
    return n;
}
"#;
    let (_scratch, output, c_path) = compile_only("dotunbound", unbound);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10301:"),
        1,
        "dotunbound: expected one inference report; stderr:\n{stderr}"
    );
    assert!(!c_path.exists());

    let bound = r#"module spec.bgi.impl.dotbound;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    return 1.apply(5) - 5;
}
"#;
    let (_scratch, output, c_path) = compile_only("dotbound", bound);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10303:"),
        1,
        "dotbound: expected one bound report; stderr:\n{stderr}"
    );
    assert!(!c_path.exists());
}
