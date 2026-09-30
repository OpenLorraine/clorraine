//! Implementation tests for the emitted shape of owned conditional stores and
//! deferred take-call temporaries.
//!
//! The specification suite proves *what* a program computes; these cases prove
//! *how* the emitter spells it. An owned conditional stores the chosen arm
//! through a branch, empties that arm's source, and releases the destination's
//! saved old value exactly once; a self-naming arm restores the save instead.
//! A deferred take-call result releases its temporary when the value is
//! discarded and empties the temporary when a destination adopts it. A `take`
//! argument that is a conditional travels through a non-releasing argument
//! temporary, because the callee's parameter cleanup owns the value. A
//! conditional return branches into the result, and a struct literal with an
//! owned conditional field stores every field in source order.
//!
//! Fixed-array values have no C assignment, so a row store copies bytes
//! through one shared core (a literal stores element by element and a
//! conditional branches per arm), a duplicable fixed-array argument or
//! projection materializes into a named local, and a struct literal with a
//! fixed-array field stores its fields instead of using an aggregate
//! initializer.

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
            "clc-cviimpl-{}-{tag}-{sequence}",
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

/// The text after the first occurrence of `needle`.
///
/// Fails the case when the needle is absent, so a shape assertion always names
/// the construct it expected to find.
fn after<'a>(tag: &str, text: &'a str, needle: &str) -> &'a str {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("{tag}: the emitted C does not contain {needle:?}"));
    &text[start + needle.len()..]
}

/// Asserts the text contains a fragment.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: the emitted C does not contain {needle:?}"
    );
}

/// A fresh owned binding takes the chosen arm through a branch and empties it.
#[test]
fn should_branch_store_an_owned_conditional_into_a_fresh_binding() {
    let source = r#"module impl.cvi.branch;
import { String, stringOf } from std.text;
func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: String = ok ? a : b;
    return i32(len(c));
}
"#;
    let emitted = emit_host("cviimpl-branch", source);
    assert_contains("cviimpl-branch", &emitted, "if (ok) {");
    assert_contains("cviimpl-branch", &emitted, "c = a;");
    assert_contains("cviimpl-branch", &emitted, "a = NULL;");
    assert_contains("cviimpl-branch", &emitted, "} else {");
    assert_contains("cviimpl-branch", &emitted, "c = b;");
    assert_contains("cviimpl-branch", &emitted, "b = NULL;");
    assert!(
        !emitted.contains("c = (ok ?"),
        "cviimpl-branch: an owned conditional must not render as a value expression"
    );
}

/// Replacing a live owned destination saves the old value and releases it once.
#[test]
fn should_save_and_release_the_old_value_when_an_owned_conditional_replaces_it() {
    let source = r#"module impl.cvi.replace;
import { String, stringOf } from std.text;
func main(): i32 {
    let ok: bool = true;
    let b: String = stringOf("cde");
    let s: String = stringOf("ab");
    s = ok ? s : b;
    return i32(len(s));
}
"#;
    let emitted = emit_host("cviimpl-replace", source);
    assert_contains("cviimpl-replace", &emitted, "ClString __cl_save_1 = s;");
    let tail = after("cviimpl-replace", &emitted, "} else {");
    assert_contains(
        "cviimpl-replace",
        tail,
        "if (__cl_save_1) { FreeString(__cl_save_1); }",
    );
}

/// An arm that names the destination restores the saved value and empties the
/// save, so the single release after the branch is a no-op.
#[test]
fn should_restore_a_self_naming_arm_and_empty_the_saved_value() {
    let source = r#"module impl.cvi.self;
import { String, stringOf } from std.text;
func main(): i32 {
    let ok: bool = true;
    let b: String = stringOf("cde");
    let s: String = stringOf("ab");
    s = ok ? s : b;
    return i32(len(s));
}
"#;
    let emitted = emit_host("cviimpl-self", source);
    assert_contains("cviimpl-self", &emitted, "s = __cl_save_1;");
    let tail = after("cviimpl-self", &emitted, "s = __cl_save_1;");
    assert_contains("cviimpl-self", tail, "__cl_save_1 = NULL;");
}

/// An owned aggregate arm moves with the aggregate zero, not a null handle.
#[test]
fn should_zero_an_owned_aggregate_arm_with_the_aggregate_zero() {
    let source = r#"module impl.cvi.aggregate;
import { String, stringOf } from std.text;
struct Box { name: String; tag: i32; }
func main(): i32 {
    let ok: bool = true;
    let a: Box = Box { name: stringOf("ab"), tag: 1 };
    let b: Box = Box { name: stringOf("cde"), tag: 1 };
    let c: Box = ok ? a : b;
    return i32(len(c.name)) + c.tag;
}
"#;
    let emitted = emit_host("cviimpl-agg", source);
    assert_contains("cviimpl-agg", &emitted, "c = a;");
    assert_contains("cviimpl-agg", &emitted, "cl_zero(&a);");
    assert_contains("cviimpl-agg", &emitted, "c = b;");
    assert_contains("cviimpl-agg", &emitted, "cl_zero(&b);");
}

/// A nested conditional branches inside the chosen outer arm only.
#[test]
fn should_evaluate_only_the_chosen_arm_of_a_nested_conditional() {
    let source = r#"module impl.cvi.nested;
import { String, stringOf } from std.text;
func mk(): String { return stringOf("wxyz"); }
func main(): i32 {
    let ok1: bool = true;
    let ok2: bool = false;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: String = ok1 ? (ok2 ? a : b) : mk();
    return i32(len(c));
}
"#;
    let emitted = emit_host("cviimpl-nested", source);
    let outer = after("cviimpl-nested", &emitted, "if (ok1) {");
    assert_contains("cviimpl-nested", outer, "if (ok2) {");
    assert_contains("cviimpl-nested", outer, "c = a;");
    assert_contains("cviimpl-nested", outer, "a = NULL;");
    assert_contains("cviimpl-nested", outer, "c = b;");
    assert_contains("cviimpl-nested", outer, "b = NULL;");
    let else_arm = after("cviimpl-nested", outer, "} else {");
    assert_contains("cviimpl-nested", else_arm, "c = cl_impl_cvi_nested_mk();");
}

/// A discarded deferred take-call result releases its temporary at the
/// statement's end.
#[test]
fn should_release_a_discarded_transfer_temporary_after_the_statement() {
    let source = r#"module impl.cvi.discard;
import { String, stringOf } from std.text;
func sink(take s: String): String { return s; }
func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    ok ? sink(s) : t;
    return i32(len(t));
}
"#;
    let emitted = emit_host("cviimpl-discard", source);
    assert_contains(
        "cviimpl-discard",
        &emitted,
        "__cl_tmp0 = cl_impl_cvi_discard_sink(s)",
    );
    let tail = after("cviimpl-discard", &emitted, "(void)((ok ?");
    assert_contains(
        "cviimpl-discard",
        tail,
        "if (__cl_tmp0) { FreeString(__cl_tmp0); }",
    );
}

/// A binding that adopts a deferred take-call result empties the temporary, so
/// the statement release is a no-op.
#[test]
fn should_adopt_a_transfer_temporary_in_a_binding_and_empty_it() {
    let source = r#"module impl.cvi.adopt;
import { String, stringOf } from std.text;
func sink(take s: String): String { return s; }
func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    let r: String = ok ? sink(s) : t;
    return i32(len(r));
}
"#;
    let emitted = emit_host("cviimpl-adopt", source);
    assert_contains("cviimpl-adopt", &emitted, "r = __cl_tmp0;");
    let tail = after("cviimpl-adopt", &emitted, "r = __cl_tmp0;");
    assert_contains("cviimpl-adopt", tail, "__cl_tmp0 = NULL;");
}

/// A `take` conditional argument travels through a non-releasing temporary:
/// the callee owns the value, so the caller releases nothing.
#[test]
fn should_pass_a_take_conditional_through_a_non_releasing_argument_temporary() {
    let source = r#"module impl.cvi.takearg;
import { String, stringOf } from std.text;
func eat(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    return eat(ok ? a : b);
}
"#;
    let emitted = emit_host("cviimpl-takearg", source);
    assert_contains("cviimpl-takearg", &emitted, "__cl_tmp0 = a;");
    assert_contains("cviimpl-takearg", &emitted, "a = NULL;");
    assert_contains("cviimpl-takearg", &emitted, "__cl_tmp0 = b;");
    assert_contains("cviimpl-takearg", &emitted, "eat(__cl_tmp0);");
    assert!(
        !emitted.contains("FreeString(__cl_tmp0)"),
        "cviimpl-takearg: the callee owns the argument, so the caller must not release it"
    );
}

/// A conditional return branches into the result and empties the chosen arm.
#[test]
fn should_branch_store_a_conditional_into_the_return_result() {
    let source = r#"module impl.cvi.ret;
import { String, stringOf } from std.text;
func sink(take s: String): String { return s; }
func pick(ok: bool, take s: String, take t: String): String {
    return ok ? sink(s) : t;
}
func main(): i32 {
    return i32(len(pick(false, stringOf("ab"), stringOf("cde"))));
}
"#;
    let emitted = emit_host("cviimpl-ret", source);
    assert_contains("cviimpl-ret", &emitted, "ClString __cl_save_1 = __cl_ret;");
    assert_contains("cviimpl-ret", &emitted, "__cl_ret = __cl_tmp0;");
    assert_contains("cviimpl-ret", &emitted, "__cl_tmp0 = NULL;");
    assert_contains("cviimpl-ret", &emitted, "__cl_ret = t;");
    assert_contains("cviimpl-ret", &emitted, "t = NULL;");
    assert_contains("cviimpl-ret", &emitted, "return __cl_ret;");
}

/// A struct literal with an owned conditional field stores its fields in
/// source order instead of rendering an aggregate initializer.
#[test]
fn should_store_the_fields_of_a_struct_literal_with_a_conditional_field() {
    let source = r#"module impl.cvi.field;
import { String, stringOf } from std.text;
struct Box { name: String; tag: i32; }
func sink(take s: String): String { return s; }
func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let t: String = stringOf("cde");
    let c: Box = Box { name: ok ? sink(s) : t, tag: 1 };
    return i32(len(c.name)) + c.tag;
}
"#;
    let emitted = emit_host("cviimpl-field", source);
    assert_contains("cviimpl-field", &emitted, "c.name = __cl_tmp0;");
    assert_contains("cviimpl-field", &emitted, "c.name = t;");
    let tail = after("cviimpl-field", &emitted, "c.name = t;");
    assert_contains("cviimpl-field", tail, "c.tag = 1;");
    assert!(
        !emitted.contains("(ClBox){"),
        "cviimpl-field: a conditional field must not be rendered in an aggregate initializer"
    );
}

/// A composite conditional arm empties every storage source it copied into
/// the destination, so the statement-end release is a no-op.
#[test]
fn should_empty_a_composite_arm_source_after_the_store() {
    let source = r#"module impl.cvi.composite;
import { String, stringOf } from std.text;
struct Box { name: String; tag: i32; }
func sink(take s: String): String { return s; }
func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let b: Box = Box { name: stringOf("cde"), tag: 1 };
    let c: Box = ok ? Box { name: sink(s), tag: 2 } : b;
    return i32(len(c.name)) + c.tag;
}
"#;
    let emitted = emit_host("cviimpl-composite", source);
    assert_contains("cviimpl-composite", &emitted, "ClBox __cl_arm");
    let arm = after("cviimpl-composite", &emitted, "ClBox __cl_arm");
    assert_contains("cviimpl-composite", arm, ".name = __cl_tmp0;");
    assert_contains("cviimpl-composite", arm, "c = __cl_arm");
    let store_at = arm
        .find(".name = __cl_tmp0;")
        .unwrap_or_else(|| panic!("cviimpl-composite: the field is not stored"));
    let stored = &arm[store_at..];
    let zero_at = stored
        .find("__cl_tmp0 = NULL;")
        .unwrap_or_else(|| panic!("cviimpl-composite: the source temporary is not emptied"));
    let move_at = stored
        .find("c = __cl_arm")
        .unwrap_or_else(|| panic!("cviimpl-composite: the local is not moved in"));
    assert!(
        zero_at < move_at,
        "cviimpl-composite: the source must be emptied before the local is moved in"
    );
}

/// A field read from a fresh call result is copied without being emptied: it
/// has no caller storage, and re-rendering the call would duplicate its side
/// effect (and produce an invalid assignment to an rvalue).
#[test]
fn should_copy_a_field_of_a_call_result_without_emptying_it() {
    let source = r#"module impl.cvi.callfield;
import { String, stringOf } from std.text;
struct Box { name: String; tag: i32; }
func mk(): Box { return Box { name: stringOf("ab"), tag: 1 }; }
func main(): i32 {
    let ok: bool = true;
    let b: String = stringOf("cde");
    let c: String = ok ? mk().name : b;
    return i32(len(c));
}
"#;
    let emitted = emit_host("cviimpl-callfield", source);
    assert_contains("cviimpl-callfield", &emitted, "c = cl_impl_cvi_callfield_mk().name;");
    assert!(
        !emitted.contains("mk().name = NULL"),
        "cviimpl-callfield: a field of a call result has no caller storage to empty"
    );
}

/// A composite arm whose conditional field reads the destination evaluates
/// that condition once, because the field is stored by the branch store.
#[test]
fn should_store_a_nested_condition_once_in_a_composite_arm() {
    let source = r#"module impl.cvi.condread;
import { String, stringOf } from std.text;
struct Box { name: String; tag: i32; }
func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let d: Box = Box { name: stringOf("zz"), tag: 9 };
    let c: Box = Box { name: stringOf("qq"), tag: 0 };
    c = ok ? Box { name: (c.tag == 0) ? a : b, tag: 1 } : d;
    return i32(len(c.name)) + c.tag;
}
"#;
    let emitted = emit_host("cviimpl-condread", source);
    assert_contains("cviimpl-condread", &emitted, "ClBox __cl_arm");
    let arm = after("cviimpl-condread", &emitted, "ClBox __cl_arm");
    assert_contains("cviimpl-condread", arm, ".name = a;");
    assert_contains("cviimpl-condread", arm, ".name = b;");
    assert_contains("cviimpl-condread", arm, "c = __cl_arm");
    assert_eq!(
        arm.matches("c.tag == 0").count(),
        1,
        "cviimpl-condread: the condition must be evaluated once, for the branch store only"
    );
}

/// A nested condition that consumes the destination empties the saved old
/// value, because the callee already owns it.
#[test]
fn should_empty_the_save_when_a_nested_condition_consumes_the_destination() {
    let source = r#"module impl.cvi.condconsume;
import { String, stringOf } from std.text;
func sink(take s: String): String { return s; }
func main(): i32 {
    let outer: bool = true;
    let b: String = stringOf("cde");
    let s: String = stringOf("ab");
    s = outer ? (len(sink(s)) > 0 ? b : b) : b;
    return i32(len(s));
}
"#;
    let emitted = emit_host("cviimpl-condconsume", source);
    let tail = after("cviimpl-condconsume", &emitted, "ClString __cl_save_1 = s;");
    let call_at = tail
        .find("cl_impl_cvi_condconsume_sink(s);")
        .unwrap_or_else(|| panic!("cviimpl-condconsume: the consuming call is absent"));
    let consumed = &tail[call_at..];
    let zero_at = consumed
        .find("__cl_save_1 = NULL;")
        .unwrap_or_else(|| panic!("cviimpl-condconsume: the saved value is not emptied"));
    let drop_at = consumed
        .find("if (__cl_save_1)")
        .unwrap_or_else(|| panic!("cviimpl-condconsume: the saved value has no release"));
    assert!(
        zero_at < drop_at,
        "cviimpl-condconsume: the consumed destination's saved value must be emptied before its release"
    );
}

/// A `take` conditional argument inside a short-circuit operand is filled and
/// consumed only on the evaluated path, so a skipped path runs no arm.
#[test]
fn should_defer_a_take_conditional_argument_to_the_evaluated_path() {
    let source = r#"module impl.cvi.guardarg;
import { String, stringOf } from std.text;
func eat(take s: String): i32 { return i32(len(s)); }
func boom(take s: String): String { return s; }
func main(): i32 {
    let ok: bool = true;
    let s: String = stringOf("ab");
    let c: String = stringOf("cde");
    if (false && eat(ok ? boom(s) : c) > 0) {
        return 1;
    }
    return 7;
}
"#;
    let emitted = emit_host("cviimpl-guardarg", source);
    let main = after(
        "cviimpl-guardarg",
        &emitted,
        "cl_impl_cvi_guardarg_main(void)\n{",
    );
    let guard_at = main
        .find("if ((0 &&")
        .unwrap_or_else(|| panic!("cviimpl-guardarg: the guarded operand is not embedded"));
    let before = &main[..guard_at];
    assert!(
        !before.contains("boom(") && !before.contains("eat("),
        "cviimpl-guardarg: the guarded operands must not run before the guard"
    );
    let guarded = after("cviimpl-guardarg", main, "if ((0 &&");
    assert_contains("cviimpl-guardarg", guarded, "eat(__cl_tmp");
    assert_contains("cviimpl-guardarg", guarded, "(c = NULL)");
}

/// A struct literal with a fixed-array field stores every field, because a C
/// aggregate initializer cannot name an array field.
#[test]
fn should_store_every_field_when_a_struct_literal_has_a_fixed_array_field() {
    let source = r#"module impl.cvi.mixedfield;
import { String, stringOf } from std.text;
struct Box { name: String; nums: i32[2]; }
func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: Box = Box { name: ok ? a : b, nums: [1, 2] };
    return i32(len(c.name)) + c.nums[0];
}
"#;
    let emitted = emit_host("cviimpl-mixed", source);
    assert_contains("cviimpl-mixed", &emitted, "c.nums[0] = 1;");
    assert_contains("cviimpl-mixed", &emitted, "c.nums[1] = 2;");
    assert!(
        !emitted.contains("(ClBox){"),
        "cviimpl-mixed: the literal must store its fields instead of using an aggregate initializer"
    );
}

/// A struct literal whose field reads the target is built in a fresh local
/// first, so the old target survives until every field has been evaluated.
#[test]
fn should_build_a_target_reading_struct_literal_in_a_fresh_local() {
    let source = r#"module impl.cvi.readtarget;
import { String, stringOf } from std.text;
struct Box { name: String; tag: i32; }
func main(): i32 {
    let ok: bool = true;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: Box = Box { name: stringOf("zz"), tag: 1 };
    c = Box { name: ok ? a : b, tag: c.tag + 1 };
    return i32(len(c.name)) + c.tag;
}
"#;
    let emitted = emit_host("cviimpl-readtarget", source);
    let main = after(
        "cviimpl-readtarget",
        &emitted,
        "cl_impl_cvi_readtarget_main(void)\n{",
    );
    let head = main
        .find("ClBox __cl_lit")
        .unwrap_or_else(|| panic!("cviimpl-readtarget: the literal is not built in a local"));
    let body = &main[head..];
    let field_store = body
        .find(".name = a;")
        .unwrap_or_else(|| panic!("cviimpl-readtarget: the conditional field is not stored"));
    let old_drop = body
        .find("if (c.name) { FreeString(c.name); }")
        .unwrap_or_else(|| panic!("cviimpl-readtarget: the old target is not released"));
    assert!(
        field_store < old_drop,
        "cviimpl-readtarget: every field must be evaluated before the old target is released"
    );
    assert_contains("cviimpl-readtarget", body, "c = __cl_lit");
}

/// An arm whose call consumed the destination empties the saved old value
/// instead of releasing the value the callee already owns.
#[test]
fn should_empty_the_saved_value_when_an_arm_consumes_the_destination() {
    let source = r#"module impl.cvi.consumed;
import { String, stringOf } from std.text;
func sink(take s: String): String { return s; }
func main(): i32 {
    let ok: bool = true;
    let b: String = stringOf("cde");
    let s: String = stringOf("ab");
    s = ok ? sink(s) : b;
    return i32(len(s));
}
"#;
    let emitted = emit_host("cviimpl-consumed", source);
    let tail = after("cviimpl-consumed", &emitted, "s = __cl_tmp0;");
    let zero_at = tail
        .find("__cl_save_1 = NULL;")
        .unwrap_or_else(|| panic!("cviimpl-consumed: the saved value is not emptied"));
    let drop_at = tail
        .find("if (__cl_save_1)")
        .unwrap_or_else(|| panic!("cviimpl-consumed: the saved value has no release"));
    assert!(
        zero_at < drop_at,
        "cviimpl-consumed: the consumed destination's saved value must be emptied before its release"
    );
}

/// The same struct-literal rule moves a place arm and empties it.
#[test]
fn should_store_a_struct_literal_conditional_field_from_a_place_arm() {
    let source = r#"module impl.cvi.fieldplace;
import { String, stringOf } from std.text;
struct Box { name: String; tag: i32; }
func main(): i32 {
    let ok: bool = false;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: Box = Box { name: ok ? a : b, tag: 1 };
    return i32(len(c.name)) + c.tag;
}
"#;
    let emitted = emit_host("cviimpl-fplace", source);
    let tail = after("cviimpl-fplace", &emitted, "c.name = a;");
    assert_contains("cviimpl-fplace", tail, "a = NULL;");
    assert_contains("cviimpl-fplace", tail, "c.name = b;");
    assert_contains("cviimpl-fplace", tail, "b = NULL;");
    let tag = after("cviimpl-fplace", tail, "c.name = b;");
    assert_contains("cviimpl-fplace", tag, "c.tag = 1;");
}

/// A constant-index row store copies the array bytes instead of assigning,
/// through the same core the run-time path uses.
#[test]
fn should_byte_copy_a_row_when_a_constant_index_stores_a_fixed_array() {
    let source = r#"module impl.cvi.rowconst;
func main(): i32 {
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let value: i32[2] = [5, 6];
    rows[0] = value;
    return rows[0][0];
}
"#;
    let emitted = emit_host("cviimpl-rowconst", source);
    assert_contains(
        "cviimpl-rowconst",
        &emitted,
        "memcpy(&(rows[0]), &(value), sizeof(rows[0]));",
    );
}

/// A run-time-index row store guards the index first, then copies the bytes
/// through the dereferenced element pointer.
#[test]
fn should_byte_copy_a_row_when_a_runtime_index_stores_a_fixed_array() {
    let source = r#"module impl.cvi.rowruntime;
func runtimeIndex(): i32 {
    return 1;
}
func main(): i32 {
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let value: i32[2] = [5, 6];
    let i: i32 = runtimeIndex();
    rows[i] = value;
    return rows[1][0];
}
"#;
    let emitted = emit_host("cviimpl-rowruntime", source);
    let guarded = after(
        "cviimpl-rowruntime",
        &emitted,
        "cl_trap(\"E11211\", \"index out of range\");",
    );
    assert_contains(
        "cviimpl-rowruntime",
        guarded,
        "memcpy(&((*(&(rows[i])))), &(value), sizeof((*(&(rows[i])))));",
    );
}

/// A run-time-index row literal stores element by element; a brace list is
/// not a C assignment value.
#[test]
fn should_store_an_array_literal_element_by_element_at_a_runtime_index() {
    let source = r#"module impl.cvi.rowliteral;
func runtimeIndex(): i32 {
    return 1;
}
func main(): i32 {
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let x: i32 = 7;
    let y: i32 = 8;
    let i: i32 = runtimeIndex();
    rows[i] = [x, y];
    return rows[1][0];
}
"#;
    let emitted = emit_host("cviimpl-rowliteral", source);
    assert_contains("cviimpl-rowliteral", &emitted, "(*(&(rows[i])))[0] = x;");
    assert_contains("cviimpl-rowliteral", &emitted, "(*(&(rows[i])))[1] = y;");
    assert!(
        !emitted.contains("= { x, y }"),
        "cviimpl-rowliteral: an array literal must not render as a brace list in a store"
    );
}

/// A run-time-index conditional copies each arm's bytes inside its branch.
#[test]
fn should_branch_store_each_row_arm_when_a_runtime_index_takes_a_conditional() {
    let source = r#"module impl.cvi.rowcond;
func runtimeIndex(): i32 {
    return 1;
}
func main(): i32 {
    let ok: bool = true;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let a: i32[2] = [5, 6];
    let b: i32[2] = [7, 8];
    let i: i32 = runtimeIndex();
    rows[i] = ok ? a : b;
    return rows[1][0];
}
"#;
    let emitted = emit_host("cviimpl-rowcond", source);
    let branch = after("cviimpl-rowcond", &emitted, "if (ok) {");
    assert_contains(
        "cviimpl-rowcond",
        branch,
        "memcpy(&((*(&(rows[i])))), &(a), sizeof((*(&(rows[i])))));",
    );
    let alternate = after("cviimpl-rowcond", branch, "} else {");
    assert_contains(
        "cviimpl-rowcond",
        alternate,
        "memcpy(&((*(&(rows[i])))), &(b), sizeof((*(&(rows[i])))));",
    );
}

/// A conditional array argument with call arms materializes into a named
/// local; the call writes through its hidden destination before the copy.
#[test]
fn should_materialize_a_conditional_call_argument_into_a_local() {
    let source = r#"module impl.cvi.argcall;
func mkA(): i32[2] {
    return [1, 2];
}
func mkB(): i32[2] {
    return [3, 4];
}
func eat(a: i32[2]): i32 {
    return a[0] + a[1];
}
func main(): i32 {
    let ok: bool = true;
    return eat(ok ? mkA() : mkB());
}
"#;
    let emitted = emit_host("cviimpl-argcall", source);
    assert_contains("cviimpl-argcall", &emitted, "i32 __cl_owned0[2] = {0};");
    assert_contains("cviimpl-argcall", &emitted, "if (ok) {");
    assert_contains("cviimpl-argcall", &emitted, "cl_impl_cvi_argcall_mkA(__cl_arm");
    assert_contains("cviimpl-argcall", &emitted, "cl_impl_cvi_argcall_mkB(__cl_arm");
    assert_contains("cviimpl-argcall", &emitted, "eat(__cl_owned0);");
}

/// A conditional array argument with literal arms stores each chosen arm's
/// elements into the materialized local.
#[test]
fn should_materialize_a_conditional_literal_argument_element_by_element() {
    let source = r#"module impl.cvi.argliteral;
func eat(a: i32[2]): i32 {
    return a[0] + a[1];
}
func main(): i32 {
    let ok: bool = true;
    return eat(ok ? [10, 20] : [30, 40]);
}
"#;
    let emitted = emit_host("cviimpl-argliteral", source);
    assert_contains("cviimpl-argliteral", &emitted, "i32 __cl_owned0[2] = {0};");
    assert_contains("cviimpl-argliteral", &emitted, "__cl_owned0[0] = 10;");
    assert_contains("cviimpl-argliteral", &emitted, "__cl_owned0[1] = 20;");
    assert_contains("cviimpl-argliteral", &emitted, "__cl_owned0[0] = 30;");
    assert_contains("cviimpl-argliteral", &emitted, "__cl_owned0[1] = 40;");
    assert_contains("cviimpl-argliteral", &emitted, "eat(__cl_owned0);");
}

/// An array literal behind a projection materializes into a compiler
/// temporary, so the element read names a real place.
#[test]
fn should_materialize_an_array_literal_projection_into_a_temporary() {
    let source = r#"module impl.cvi.projection;
func pick(x: i32): i32 {
    return x;
}
func main(): i32 {
    let a: i32 = 2;
    let b: i32 = 3;
    let first: i32 = [a, b][0];
    return first * 10 + pick([a, b][1]);
}
"#;
    let emitted = emit_host("cviimpl-projection", source);
    assert_contains("cviimpl-projection", &emitted, "i32 __cl_tmp0[2] = {0};");
    assert_contains("cviimpl-projection", &emitted, "__cl_tmp0[0] = a;");
    assert_contains("cviimpl-projection", &emitted, "__cl_tmp0[1] = b;");
    assert_contains("cviimpl-projection", &emitted, "first = __cl_tmp0[0];");
    assert_contains("cviimpl-projection", &emitted, "pick(__cl_tmp1[1])");
}

/// A struct-literal argument with a fixed-array field materializes into a
/// named local and stores its fields.
#[test]
fn should_materialize_a_struct_literal_argument_with_a_fixed_array_field() {
    let source = r#"module impl.cvi.argstruct;
struct Box { arr: i32[2]; tag: i32; }
func eat(box: Box): i32 {
    return box.arr[0] + box.arr[1] + box.tag;
}
func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return eat(Box { arr: ok ? a : b, tag: 1 });
}
"#;
    let emitted = emit_host("cviimpl-argstruct", source);
    assert_contains("cviimpl-argstruct", &emitted, "ClBox __cl_owned0 = {0};");
    let branch = after("cviimpl-argstruct", &emitted, "if (ok) {");
    assert_contains(
        "cviimpl-argstruct",
        branch,
        "memcpy(&(__cl_owned0.arr), &(a), sizeof(__cl_owned0.arr));",
    );
    assert_contains("cviimpl-argstruct", branch, "__cl_owned0.tag = 1;");
    assert_contains("cviimpl-argstruct", &emitted, "eat(__cl_owned0);");
}

/// A returned struct literal with a fixed-array field stores its fields into
/// the return local before the release ladder.
#[test]
fn should_store_the_fields_of_a_returned_struct_literal_with_a_fixed_array_field() {
    let source = r#"module impl.cvi.retliteral;
struct Box { arr: i32[2]; tag: i32; }
func make(a: i32[2]): Box {
    return Box { arr: a, tag: 2 };
}
func main(): i32 {
    let a: i32[2] = [1, 2];
    let box: Box = make(a);
    return box.arr[0] + box.arr[1] + box.tag;
}
"#;
    let emitted = emit_host("cviimpl-retliteral", source);
    assert_contains(
        "cviimpl-retliteral",
        &emitted,
        "memcpy(&(__cl_return_1.arr), &(a), sizeof(__cl_return_1.arr));",
    );
    assert_contains("cviimpl-retliteral", &emitted, "__cl_return_1.tag = 2;");
    assert_contains("cviimpl-retliteral", &emitted, "return __cl_return_1;");
}
