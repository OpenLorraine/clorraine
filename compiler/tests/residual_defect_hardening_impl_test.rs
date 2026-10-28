//! Implementation tests for the emitted shape of the flat array literal and
//! the deferred row store.
//!
//! The specification suite proves *what* a program computes; these cases prove
//! *how* the emitter spells it. A no-context literal declares one flat array
//! with every extent, a deferred literal stores a conditional row as a nested
//! ternary whose arms use the row expression, and an owned row leaves its
//! source emptied inside the same comma expression. A row whose source is a
//! C-adjusted array parameter is addressed by the parameter name and emptied
//! element by element, because the adjusted pointer has no array size. A
//! conditional fill covers every row through the per-row store. Two emissions
//! of the same program are byte-identical.

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
            "clc-rdhimpl-{}-{tag}-{sequence}",
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

/// Asserts the text does not contain a superseded fragment.
fn assert_absent(tag: &str, text: &str, needle: &str) {
    assert!(
        !text.contains(needle),
        "{tag}: the emitted C still contains {needle:?}"
    );
}

/// The emitted text of one function definition, up to its closing brace.
///
/// The section ends at the first `}` in column zero, so an assertion stays
/// inside a single emitted function. `start` should include the definition's
/// `{` so a prototype is not matched first.
fn function_section<'a>(tag: &str, text: &'a str, start: &str) -> &'a str {
    let start_index = text
        .find(start)
        .unwrap_or_else(|| panic!("{tag}: the emitted C does not contain {start:?}"));
    let rest = &text[start_index..];
    let end = rest
        .find("\n}")
        .map(|offset| offset + 2)
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Compiles `source` and returns the compiler output.
///
/// Used by the rejection cases, which assert the diagnostic and that no
/// translation unit is written.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output) {
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
    let emitted_exists = c_path.exists();
    assert!(
        !(output.status.code() != Some(0) && emitted_exists),
        "{tag}: a rejected program must not produce a translation unit"
    );
    (scratch, output)
}

/// A no-context literal declares one flat array with every extent.
#[test]
fn should_declare_a_flat_array_when_a_literal_has_no_context() {
    let source = r#"module impl.rdh.flat;

func main(): i32 {
    let lit = [[1, 2], [3, 4]];
    return lit[1][0];
}
"#;
    let emitted = emit_host("rdhimpl-flat", source);
    assert_contains("rdhimpl-flat", &emitted, "i32 lit[2][2] = {0};");
    assert_contains("rdhimpl-flat", &emitted, "lit[1][0] = 3;");
    // A nested element type would print a one-extent declarator and a brace
    // row store, which C rejects.
    assert_absent("rdhimpl-flat", &emitted, "lit[1] = {");
}

/// A deferred conditional row is a nested ternary of row stores.
#[test]
fn should_store_a_conditional_row_through_the_row_expression() {
    let source = r#"module impl.rdh.row;

func main(): i32 {
    let ok: bool = true;
    let c: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1;
    return hit ? 7 : 9;
}
"#;
    let emitted = emit_host("rdhimpl-row", source);
    assert_contains(
        "rdhimpl-row",
        &emitted,
        "memcpy(__cl_tmp0[0], &(rows[0]), sizeof(__cl_tmp0[0]))",
    );
    assert_contains(
        "rdhimpl-row",
        &emitted,
        "memcpy(__cl_tmp0[0], &(rows[1]), sizeof(__cl_tmp0[0]))",
    );
    // The literal row stores its elements one by one.
    assert_contains("rdhimpl-row", &emitted, "__cl_tmp0[1][0] = 5");
    assert_contains("rdhimpl-row", &emitted, "__cl_tmp0[1][1] = 6");
    // The old scalar spellings wrote into the wrong row type.
    assert_absent("rdhimpl-row", &emitted, "__cl_tmp0[0] = (c ?");
    assert_absent("rdhimpl-row", &emitted, "__cl_tmp0[1] = {");
}

/// A conditional row with a literal arm keeps both arms' value in common.
#[test]
fn should_end_both_conditional_row_arms_in_the_same_value() {
    let source = r#"module impl.rdh.mixed;

func main(): i32 {
    let ok: bool = true;
    let c: bool = false;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [c ? rows[0] : [5, 6], rows[1]][0][0] == 5;
    return hit ? 7 : 9;
}
"#;
    let emitted = emit_host("rdhimpl-mixed", source);
    // The then arm copies the place and the else arm stores the literal's
    // elements; both tails yield zero so the ternary has one operand type.
    assert_contains(
        "rdhimpl-mixed",
        &emitted,
        "((memcpy(__cl_tmp0[0], &(rows[0]), sizeof(__cl_tmp0[0]))), 0)",
    );
    assert_contains(
        "rdhimpl-mixed",
        &emitted,
        "(__cl_tmp0[0][0] = 5, __cl_tmp0[0][1] = 6, 0)",
    );
    assert_contains(
        "rdhimpl-mixed",
        &emitted,
        "memcpy(__cl_tmp0[1], &(rows[1]), sizeof(__cl_tmp0[1]))",
    );
}

/// An owned conditional row empties the row it moved.
#[test]
fn should_empty_the_moved_row_when_the_row_is_owned() {
    let source = r#"module impl.rdh.owned;

import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = true;
    let c: bool = true;
    let rows: String[2][2] = [[stringOf("ab"), stringOf("c")], [stringOf("d"), stringOf("ef")]];
    let hit: bool = ok && len([c ? rows[0] : rows[1], [stringOf("gh"), stringOf("i")]][0][0]) == 2;
    return hit ? 7 : 9;
}
"#;
    let emitted = emit_host("rdhimpl-owned", source);
    assert_contains(
        "rdhimpl-owned",
        &emitted,
        "(memcpy(__cl_tmp0[0], &(rows[0]), sizeof(__cl_tmp0[0])), (cl_zero(&rows[0])))",
    );
    assert_contains(
        "rdhimpl-owned",
        &emitted,
        "(memcpy(__cl_tmp0[0], &(rows[1]), sizeof(__cl_tmp0[0])), (cl_zero(&rows[1])))",
    );
    // The fresh literal row calls the string constructor into each element.
    assert_contains("rdhimpl-owned", &emitted, "__cl_tmp0[1][0] = cl_std_text_stringOf");
    assert_contains("rdhimpl-owned", &emitted, "__cl_tmp0[1][1] = cl_std_text_stringOf");
}

/// A row moved from an array parameter is addressed by its name and walked.
#[test]
fn should_walk_an_array_parameter_when_a_row_expression_moves_it() {
    let source = r#"module impl.rdh.paramrow;

import { String, stringOf } from std.text;

func hit(take a: String[2], ok: bool): i32 {
    let flag: bool = ok && len([a, [stringOf("x"), stringOf("y")]][0][0]) == 4;
    return flag ? 1 : 0;
}

func main(): i32 {
    let src: String[2] = [stringOf("abcd"), stringOf("ef")];
    return hit(src, true);
}
"#;
    let emitted = emit_host("rdhimpl-param", source);
    // A C-adjusted array parameter is already the array's address, and the
    // destination row supplies the byte count.
    assert_contains(
        "rdhimpl-param",
        &emitted,
        "memcpy(__cl_tmp0[0], a, sizeof(__cl_tmp0[0]))",
    );
    // The move-out walks the parameter's elements; zeroing the adjusted
    // pointer would take the pointer's size, not the array's.
    assert_contains("rdhimpl-param", &emitted, "cl_zero(&a[0])");
    assert_contains("rdhimpl-param", &emitted, "cl_zero(&a[1])");
    assert_absent("rdhimpl-param", &emitted, "cl_zero(&a)");
}

/// A returned owned multi-extent place copies every extent and empties it.
#[test]
fn should_copy_every_extent_when_an_owned_place_returns() {
    let source = r#"module impl.rdh.ret6;

import { String, stringOf } from std.text;

func copy(take a: String[2][2]): String[2][2] {
    return a;
}

func main(): i32 {
    let src: String[2][2] = [[stringOf("ab"), stringOf("c")], [stringOf("de"), stringOf("f")]];
    let out: String[2][2] = copy(src);
    return i32(len(out[0][0]));
}
"#;
    let emitted = emit_host("rdhimpl-retowned", source);
    // One whole-array byte copy from the adjusted parameter name, sized from
    // the element type because `sizeof` would measure the pointer.
    assert_contains(
        "rdhimpl-retowned",
        &emitted,
        "memcpy(__cl_ret, a, (sizeof(__cl_ret[0]) * 2));",
    );
    // The source is emptied element by element, once.
    assert_contains("rdhimpl-retowned", &emitted, "cl_zero(&a[0][0]);");
    assert_contains("rdhimpl-retowned", &emitted, "cl_zero(&a[1][1]);");
    // The old per-extent spelling assigned an array in C.
    assert_absent("rdhimpl-retowned", &emitted, "__cl_ret[0] = a[0];");
}

/// A returned duplicable multi-extent place copies every extent and keeps it.
#[test]
fn should_copy_every_extent_when_a_duplicable_place_returns() {
    let source = r#"module impl.rdh.ret7;

func copy(take a: i32[2][2]): i32[2][2] {
    return a;
}

func main(): i32 {
    let src: i32[2][2] = [[1, 2], [3, 4]];
    let out: i32[2][2] = copy(src);
    return out[0][0] * 100 + out[1][1] * 10 + out[0][1];
}
"#;
    let emitted = emit_host("rdhimpl-retdup", source);
    assert_contains(
        "rdhimpl-retdup",
        &emitted,
        "memcpy(__cl_ret, a, (sizeof(__cl_ret[0]) * 2));",
    );
    // A duplicable source keeps its values.
    assert_absent("rdhimpl-retdup", &emitted, "cl_zero(&a[");
    assert_absent("rdhimpl-retdup", &emitted, "__cl_ret[0] = a[0];");
}

/// An owned call in a `while` condition moves into a loop temporary.
#[test]
fn should_release_a_while_condition_call_when_it_owns_the_result() {
    let source = r#"module impl.rdh.p3while;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    while (grow().count > 0 && i < 3) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("rdhimpl-p3while", source);
    // A materialized operand cannot live in the header, so the loop becomes
    // `while (1)` with the store, the previous-value drop, and the test inside.
    assert_contains("rdhimpl-p3while", &emitted, "while (1) {");
    let calls = emitted.matches("cl_impl_rdh_p3while_grow()").count();
    assert_eq!(calls, 1, "rdhimpl-p3while: the condition call runs once");
    assert_contains(
        "rdhimpl-p3while",
        &emitted,
        "if (!(((__cl_tmp0.count > 0) && (i < 3)))) break;",
    );
    // Each re-store drops the previous value and the last value is released.
    let drops = emitted
        .matches("if (__cl_tmp0.data) { FreeBuffer(__cl_tmp0.data); }")
        .count();
    assert_eq!(drops, 2, "rdhimpl-p3while: one re-store drop and one final release");
}

/// An unguarded `do while` holding an owned call takes the flag-first form.
#[test]
fn should_run_a_first_flagged_pass_when_an_unguarded_do_while_holds_an_owned_call() {
    let source = r#"module impl.rdh.p3dowhile;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
    } while (grow().count > 0 && i < 3);
    return i;
}
"#;
    let emitted = emit_host("rdhimpl-p3do", source);
    assert_contains("rdhimpl-p3do", &emitted, "i32 __cl_first_1 = 0;");
    assert_contains("rdhimpl-p3do", &emitted, "for (;;) {");
    assert_contains("rdhimpl-p3do", &emitted, "if (__cl_first_1) {");
    assert_contains("rdhimpl-p3do", &emitted, "__cl_first_1 = 1;");
    assert_contains(
        "rdhimpl-p3do",
        &emitted,
        "if (!(((__cl_tmp0.count > 0) && (i < 3)))) break;",
    );
}

/// An owned call in a `for` condition moves the test into the body.
#[test]
fn should_release_a_for_condition_call_when_it_owns_the_result() {
    let source = r#"module impl.rdh.p3for;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    for (i = 0; grow().count > 0 && i < 3; i = i + 1) {
    }
    return i;
}
"#;
    let emitted = emit_host("rdhimpl-p3for", source);
    // The step still prints in the header; the condition moved into the body.
    assert_contains("rdhimpl-p3for", &emitted, "for (; ; i = (i32)((u32)i + (u32)1)) {");
    assert_contains(
        "rdhimpl-p3for",
        &emitted,
        "if (!(((__cl_tmp0.count > 0) && (i < 3)))) break;",
    );
}

/// A condition call that owns nothing keeps the inline header spelling.
#[test]
fn should_keep_the_inline_spelling_when_a_condition_call_owns_nothing() {
    let source = r#"module impl.rdh.p3nonown;

func count(): i32 {
    return 1;
}

func main(): i32 {
    let i: i32 = 0;
    while (count() > 0 && i < 3) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("rdhimpl-p3nonown", source);
    assert_contains(
        "rdhimpl-p3nonown",
        &emitted,
        "while (((cl_impl_rdh_p3nonown_count() > 0) && (i < 3))) {",
    );
    assert_absent("rdhimpl-p3nonown", &emitted, "__cl_tmp");
}

/// An owned fixed-array condition materializes one temp and releases it once.
#[test]
fn should_materialize_and_release_an_owned_fixed_array_condition() {
    let source = r#"module impl.rdh.p3fixedstr;

import { String, stringOf } from std.text;

func mkstr(): String[2] {
    return [stringOf("ab"), stringOf("cd")];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && len(mkstr()[0]) == 2) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("rdhimpl-p3fixedstr", source);
    assert_contains(
        "rdhimpl-p3fixedstr",
        &emitted,
        "ClString __cl_tmp0[2] = {0};",
    );
    let calls = emitted
        .matches("cl_impl_rdh_p3fixedstr_mkstr(__cl_tmp0)")
        .count();
    assert_eq!(
        calls, 1,
        "rdhimpl-p3fixedstr: the fixed-array condition call writes one temp"
    );
    // The re-store empties the previous row in the expression form.
    assert_contains(
        "rdhimpl-p3fixedstr",
        &emitted,
        "(__cl_tmp0[0] ? (FreeString(__cl_tmp0[0]), __cl_tmp0[0] = NULL) : 0)",
    );
    // The final value is released once through the statement form.
    let releases = emitted
        .matches("if (__cl_tmp0[0]) { FreeString(__cl_tmp0[0]); }")
        .count();
    assert_eq!(
        releases, 1,
        "rdhimpl-p3fixedstr: the fixed-array temp has one final release"
    );
}

/// An owned conditional array element moves its chosen arm per branch.
#[test]
fn should_branch_store_an_owned_conditional_array_element() {
    let source = r#"module impl.rdh.p3condel;

import { stringOf } from std.text;

func main(): i32 {
    let c: bool = true;
    let x: i32 = i32(len([c ? stringOf("ab") : stringOf("c")][0]));
    return x;
}
"#;
    let emitted = emit_host("rdhimpl-p3condel", source);
    // The chosen arm moves into the element and is emptied; the un-chosen arm
    // stays owned for the statement-end release.
    assert_contains("rdhimpl-p3condel", &emitted, "if (c) {");
    assert_contains("rdhimpl-p3condel", &emitted, "__cl_tmp0 = cl_std_text_stringOf(cl_str(\"ab\"));");
    let moves = emitted
        .matches("__cl_rhs_1[0] = __cl_tmp0;")
        .count();
    assert_eq!(
        moves, 1,
        "rdhimpl-p3condel: the then arm moves into the element once"
    );
    // A plain conditional copy was the double-free shape.
    assert_absent("rdhimpl-p3condel", &emitted, "__cl_rhs_1[0] = (c ?");
}

/// An owned call trapped in an eagerly stored read still gets a temporary.
///
/// The condition indexes a literal holding `grow().count`; without its own
/// temporary the array returned by `grow` would be released by nothing. The
/// temporary is refilled each test with its previous value dropped first, and
/// the last value is released after the loop.
#[test]
fn should_release_a_condition_call_trapped_in_a_materialized_read() {
    let source = r#"module impl.rdh.p3read;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    while ([grow().count][0] > 0 && i < 3) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("rdhimpl-p3read", source);
    // The trapped call travels through its own temporary, refilled in place.
    assert_contains(
        "rdhimpl-p3read",
        &emitted,
        "__cl_tmp0 = cl_impl_rdh_p3read_grow();",
    );
    assert_contains("rdhimpl-p3read", &emitted, "__cl_tmp1[0] = __cl_tmp0.count;");
    let drops = emitted
        .matches("if (__cl_tmp0.data) { FreeBuffer(__cl_tmp0.data); }")
        .count();
    assert_eq!(
        drops, 2,
        "rdhimpl-p3read: one re-store drop and one final release"
    );
}

/// A `for` step holding a materialize drops to the flat label-and-goto form.
#[test]
fn should_drop_to_the_flat_form_when_a_for_step_holds_a_materialize() {
    let source = r#"module impl.rdh.p3forstep;

func mk(): i32[3] {
    return [0, 1, 2];
}

func idx(): i32 {
    return 1;
}

func main(): i32 {
    let i: i32 = 0;
    for (i = 0; i < 3; i = i + mk()[idx()]) {
    }
    return i;
}
"#;
    let emitted = emit_host("rdhimpl-p3forstep", source);
    // The step's guarded index cannot print in a C `for` header, so the whole
    // loop becomes labels and gotos.
    assert_contains("rdhimpl-p3forstep", &emitted, "goto __cl_b1;");
    assert_contains("rdhimpl-p3forstep", &emitted, "cl_trap(\"E11211\", \"index out of range\");");
    assert_absent("rdhimpl-p3forstep", &emitted, "for (; ;");
}

/// A call row runs once and carries no discarded comma tail.
#[test]
fn should_call_a_call_row_once_without_a_value_tail() {
    let source = r#"module impl.rdh.callrow;

func make(): i32[2] {
    return [7, 8];
}

func main(): i32 {
    let ok: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [make(), [5, 6]][0][0] == 7;
    return hit ? 7 : 9;
}
"#;
    let emitted = emit_host("rdhimpl-callrow", source);
    let calls = emitted
        .matches("cl_impl_rdh_callrow_make(__cl_tmp0[0])")
        .count();
    assert_eq!(
        calls, 1,
        "rdhimpl-callrow: the row call must run exactly once"
    );
    // A discarded `0` right operand would fail the strict unused-value build.
    assert_absent("rdhimpl-callrow", &emitted, "make(__cl_tmp0[0]), 0)");
}

/// A take-call row materializes exactly once and empties its source once.
#[test]
fn should_materialize_a_take_call_row_once() {
    let source = r#"module impl.rdh.takecallrow;

import { String, stringOf } from std.text;

func consume(take src: String): i32[2] {
    return [1, i32(len(src))];
}

func main(): i32 {
    let ok: bool = true;
    let src: String = stringOf("ab");
    let hit: bool = ok && [consume(src), [5, 6]][0][0] == 1;
    return hit ? 7 : 9;
}
"#;
    let emitted = emit_host("rdhimpl-takecall", source);
    let calls = emitted
        .matches("cl_impl_rdh_takecallrow_consume(src, __cl_tmp0)")
        .count();
    assert_eq!(
        calls, 1,
        "rdhimpl-takecall: the take call must run exactly once"
    );
    let moves = emitted.matches("(src = NULL)").count();
    assert_eq!(
        moves, 1,
        "rdhimpl-takecall: the source must empty exactly once"
    );
}

/// A conditional fill stores its row through the per-row store forms.
#[test]
fn should_store_each_row_when_a_fill_value_is_conditional() {
    let source = r#"module impl.rdh.fill;

func main(): i32 {
    let c: bool = false;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let g: i32[2][2] = [; c ? a : b];
    return g[1][0] * 10 + g[0][1];
}
"#;
    let emitted = emit_host("rdhimpl-fill", source);
    assert_contains("rdhimpl-fill", &emitted, "if (c) {");
    assert_contains(
        "rdhimpl-fill",
        &emitted,
        "memcpy(&(__cl_fillrow_1), &(a), sizeof(__cl_fillrow_1));",
    );
    assert_contains(
        "rdhimpl-fill",
        &emitted,
        "memcpy(&(__cl_fillrow_1), &(b), sizeof(__cl_fillrow_1));",
    );
    assert_contains(
        "rdhimpl-fill",
        &emitted,
        "memcpy(&(g[0]), &(__cl_fillrow_1), sizeof(g[0]));",
    );
    assert_contains(
        "rdhimpl-fill",
        &emitted,
        "memcpy(&(g[1]), &(__cl_fillrow_1), sizeof(g[1]));",
    );
}

/// A maybe-owning take parameter releases only in the owning instance.
#[test]
fn should_release_a_maybe_owning_parameter_only_when_the_instance_owns() {
    let source = r#"module impl.rdh.p4keep;

import { String, stringOf } from std.text;

func keep<T>(take a: T[2]): i32 {
    return 0;
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("c")];
    let n: i32 = keep<String>(xs);
    let ys: i32[2] = [1, 2];
    let m: i32 = keep<i32>(ys);
    return n + m;
}
"#;
    let emitted = emit_host("rdhimpl-p4keep", source);
    assert_contains(
        "rdhimpl-p4keep",
        &emitted,
        "static i32 cl_keep__i32(i32 a[2])",
    );
    // The `String` instance releases both elements; the `i32` instance emits
    // no release at all.
    let releases = emitted.matches("FreeString(a[").count();
    assert_eq!(
        releases, 2,
        "rdhimpl-p4keep: only the owning instance releases the parameter"
    );
}

/// A generic move copies in place without emptying a duplicable instance.
#[test]
fn should_copy_without_emptying_when_a_generic_move_is_duplicable() {
    let source = r#"module impl.rdh.p4dup;

import { String, stringOf } from std.text;

func dup<T>(take a: T): T {
    let b: T = a;
    return b;
}

func main(): i32 {
    let s: String = stringOf("abcd");
    let t: String = dup<String>(s);
    let u: i32 = dup<i32>(3);
    return u + i32(len(t));
}
"#;
    let emitted = emit_host("rdhimpl-p4dup", source);
    let string_body =
        function_section("rdhimpl-p4dup", &emitted, "static ClString cl_dup__String(ClString a)\n{");
    let int_body = function_section("rdhimpl-p4dup", &emitted, "static i32 cl_dup__i32(i32 a)\n{");
    // The owning instance moves out of `a`; the duplicable one copies.
    assert_contains("rdhimpl-p4dup", string_body, "b = a;");
    assert_contains("rdhimpl-p4dup", string_body, "a = NULL;");
    assert_contains("rdhimpl-p4dup", int_body, "b = a;");
    assert_absent("rdhimpl-p4dup", int_body, "a = NULL;");
}

/// A take argument place is emptied only in the owning instance.
#[test]
fn should_empty_a_take_argument_place_only_when_the_instance_owns() {
    let source = r#"module impl.rdh.p4caller;

import { String, stringOf } from std.text;

func sink<T>(take v: T): i32 {
    return 0;
}

func caller<T>(take a: T): i32 {
    return sink<T>(a);
}

func main(): i32 {
    let s: String = stringOf("abcd");
    let x: i32 = caller<String>(s);
    let y: i32 = caller<i32>(2);
    return x + y;
}
"#;
    let emitted = emit_host("rdhimpl-p4caller", source);
    let string_body = function_section(
        "rdhimpl-p4caller",
        &emitted,
        "static i32 cl_caller__String(ClString a)\n{",
    );
    let int_body =
        function_section("rdhimpl-p4caller", &emitted, "static i32 cl_caller__i32(i32 a)\n{");
    // The owning instance empties the caller's place after the callee took it;
    // the duplicable instance keeps its copy.
    assert_contains("rdhimpl-p4caller", string_body, "cl_sink__String(a);");
    assert_contains("rdhimpl-p4caller", string_body, "a = NULL;");
    assert_contains("rdhimpl-p4caller", int_body, "cl_sink__i32(a);");
    assert_absent("rdhimpl-p4caller", int_body, "a = NULL;");
}

/// A later use after a generic move reports E10020 exactly once.
#[test]
fn should_report_e10020_once_when_a_later_use_needs_duplicability() {
    let source = r#"module impl.rdh.p4reject;

import { String, stringOf } from std.text;

func sink(borrow v: String): i32 {
    return i32(len(v));
}

func reuse<T>(take a: T): i32 {
    let b: T = a;
    sink(borrow a);
    return 0;
}

func main(): i32 {
    let s: String = stringOf("abcd");
    return reuse<String>(s);
}
"#;
    let (_scratch, output) = compile_only("rdhimpl-p4reject", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "rdhimpl-p4reject: the owning instance must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let reports = stderr.matches("error E10020:").count();
    assert_eq!(
        reports, 1,
        "rdhimpl-p4reject: one report per function and binding; stderr:\n{stderr}"
    );
    assert_contains("rdhimpl-p4reject", &stderr, "'a' was moved and cannot be used");
}

/// A duplicable instance keeps the unused marker on a maybe-owning local.
#[test]
fn should_mark_a_duplicable_instance_local_unused() {
    let source = r#"module impl.rdh.p4unused;

func f<T>(take a: T): i32 {
    let b: T = a;
    return 0;
}

func main(): i32 {
    return f<i32>(3);
}
"#;
    let emitted = emit_host("rdhimpl-p4unused", source);
    // The `i32` instance emits no drop for `b`, so the local is unread and
    // must carry the `sizeof` marker or the strict C build fails.
    assert_contains("rdhimpl-p4unused", &emitted, "if (sizeof(b)) { }");
}

/// A view over a maybe-owning temporary cannot outlive its release.
#[test]
fn should_reject_a_view_outliving_a_maybe_owning_temporary() {
    let source = r#"module impl.rdh.p4loan;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func first<T>(mk: func(): T[2]): T {
    let view: T[] = slice(mk(), 0, 1);
    return view[0];
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(first<String>(mkString)));
}
"#;
    let (_scratch, output) = compile_only("rdhimpl-p4loan", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "rdhimpl-p4loan: the outliving view must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_contains("rdhimpl-p4loan", &stderr, "error E10030:");
    assert_contains(
        "rdhimpl-p4loan",
        &stderr,
        "borrow of a statement temporary does not live long enough",
    );
}

/// A generic body calling a generic emits the nested instance.
#[test]
fn should_declare_the_nested_instance_when_a_generic_calls_a_generic() {
    let source = r#"module impl.rdh.p4nested;

import { String, stringOf } from std.text;

func sink<T>(take v: T): i32 {
    return 0;
}

func pass<T>(mk: func(): T): i32 {
    return sink<T>(mk());
}

func mkString(): String {
    return stringOf("abcd");
}

func main(): i32 {
    return pass<String>(mkString);
}
"#;
    let emitted = emit_host("rdhimpl-p4nested", source);
    assert_contains(
        "rdhimpl-p4nested",
        &emitted,
        "static i32 cl_sink__String(",
    );
    assert_contains(
        "rdhimpl-p4nested",
        &emitted,
        "cl_sink__String(",
    );
}

/// Emitting the same deferred literal twice produces byte-identical C.
///
/// Both emissions read the same source path, so the embedded `#line` source
/// name is identical and any difference is emitter behavior.
#[test]
fn should_emit_identical_c_when_a_deferred_literal_is_emitted_twice() {
    let source = r#"module impl.rdh.repeat;

func main(): i32 {
    let ok: bool = true;
    let c: bool = false;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let hit: bool = ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1;
    return hit ? 7 : 9;
}
"#;
    let scratch = ScratchDir::new("rdhimpl-repeat");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("rdhimpl-repeat: cannot write the program: {err}"));
    let mut emissions = Vec::new();
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
            "rdhimpl-repeat: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        emissions.push(
            std::fs::read_to_string(&c_path)
                .unwrap_or_else(|err| panic!("rdhimpl-repeat: cannot read {name}: {err}")),
        );
    }
    assert!(
        !emissions[0].trim().is_empty(),
        "rdhimpl-repeat: the emitted C is empty"
    );
    assert_eq!(
        emissions[0], emissions[1],
        "rdhimpl-repeat: two emissions of the same program must be byte-identical"
    );
}
