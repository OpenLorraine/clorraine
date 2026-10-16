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
        "memcpy(&(__cl_return_1.arr), a, sizeof(__cl_return_1.arr));",
    );
    assert_contains("cviimpl-retliteral", &emitted, "__cl_return_1.tag = 2;");
    assert_contains("cviimpl-retliteral", &emitted, "return __cl_return_1;");
}

/// A run-time-index row store moves an owned row source and empties it, so
/// only the row releases the elements.
#[test]
fn should_move_an_owned_row_source_when_a_runtime_index_stores_it() {
    let source = r#"module impl.cvi.rowmov;
import { String, stringOf } from std.text;
func index(): i32 {
    return 1;
}
func main(): i32 {
    let rows: String[2][2] = [[stringOf("ab"), stringOf("c")], [stringOf("de"), stringOf("f")]];
    let value: String[2] = [stringOf("xyz"), stringOf("w")];
    let i: i32 = index();
    rows[i] = value;
    return i32(len(rows[1][1]));
}
"#;
    let emitted = emit_host("cviimpl-rowmov", source);
    assert_contains("cviimpl-rowmov", &emitted, "cl_zero(&value);");
    assert_contains(
        "cviimpl-rowmov",
        &emitted,
        "memcpy(&((*(&(rows[i])))), &(__cl_tmp0), sizeof((*(&(rows[i])))));",
    );
}

/// A run-time-index scalar store empties the moved source and releases the
/// element it replaces exactly once.
#[test]
fn should_move_an_owned_scalar_source_and_release_the_replaced_element() {
    let source = r#"module impl.cvi.scalarmov;
import { String, stringOf } from std.text;
func index(): i32 {
    return 1;
}
func main(): i32 {
    let rows: String[2] = [stringOf("ab"), stringOf("c")];
    let s: String = stringOf("xyz");
    let i: i32 = index();
    rows[i] = s;
    return i32(len(rows[1]));
}
"#;
    let emitted = emit_host("cviimpl-scalarmov", source);
    let moved = after("cviimpl-scalarmov", &emitted, "__cl_tmp0 = s;");
    assert_contains("cviimpl-scalarmov", moved, "s = NULL;");
    let dropped = emitted
        .find("if (*(&(rows[i]))) { FreeString(*(&(rows[i]))); }")
        .unwrap_or_else(|| panic!("cviimpl-scalarmov: the replaced element is not released"));
    let assigned = emitted
        .find("*(&(rows[i])) = __cl_tmp0;")
        .unwrap_or_else(|| panic!("cviimpl-scalarmov: the element is not assigned"));
    assert!(
        dropped < assigned,
        "cviimpl-scalarmov: the replaced element must be released before the new value is stored"
    );
}

/// An array literal behind a projection inside a short circuit is stored on
/// the guarded path only, not before the condition.
#[test]
fn should_store_a_guarded_array_literal_projection_on_its_own_path() {
    let source = r#"module impl.cvi.guardlit;
func boom(): i32 {
    return 1;
}
func main(): i32 {
    let ok: bool = false;
    if (ok && [boom(), 1][0] == 1) {
        return 9;
    }
    return 0;
}
"#;
    let emitted = emit_host("cviimpl-guardlit", source);
    let condition_at = emitted
        .find("if ((ok &&")
        .unwrap_or_else(|| panic!("cviimpl-guardlit: the short circuit is not emitted"));
    assert!(
        !emitted[..condition_at].contains("__cl_tmp0[0] ="),
        "cviimpl-guardlit: the literal must not be stored before the condition"
    );
    let guarded = &emitted[condition_at..];
    assert_contains("cviimpl-guardlit", guarded, "__cl_tmp0[0] = ");
}

/// A `take` array-literal argument materializes into a local before the call.
#[test]
fn should_materialize_a_take_array_literal_argument_into_a_local() {
    let source = r#"module impl.cvi.takelit;
func eat(take a: i32[2]): i32 {
    return a[0] + a[1];
}
func main(): i32 {
    return eat(take [1, 2]);
}
"#;
    let emitted = emit_host("cviimpl-takelit", source);
    assert_contains("cviimpl-takelit", &emitted, "i32 __cl_tmp0[2] = {0};");
    assert_contains("cviimpl-takelit", &emitted, "__cl_tmp0[0] = 1;");
    assert_contains("cviimpl-takelit", &emitted, "__cl_tmp0[1] = 2;");
    assert_contains("cviimpl-takelit", &emitted, "eat(__cl_tmp0);");
}

/// A conditional of struct literals with fixed-array fields stores each arm
/// per field instead of rendering a C ternary of aggregate initializers.
#[test]
fn should_store_each_arm_per_field_when_a_conditional_selects_struct_literals() {
    let source = r#"module impl.cvi.armstruct;
struct Box { arr: i32[2]; tag: i32; }
func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    let c: Box = ok ? Box { arr: a, tag: 1 } : Box { arr: b, tag: 2 };
    return c.arr[0] + c.arr[1] + c.tag;
}
"#;
    let emitted = emit_host("cviimpl-armstruct", source);
    let branch = after("cviimpl-armstruct", &emitted, "if (ok) {");
    assert_contains("cviimpl-armstruct", branch, "ClBox __cl_arm");
    assert_contains("cviimpl-armstruct", branch, "memcpy(&(__cl_arm");
    assert_contains("cviimpl-armstruct", branch, "c = __cl_arm");
    assert!(
        !emitted.contains("(ok ? (ClBox)"),
        "cviimpl-armstruct: a conditional of array-field literals must not render as a C ternary"
    );
}

/// A duplicable struct conditional passed as an argument materializes and
/// stores each arm per field, instead of rendering a C ternary of aggregate
/// initializers.
#[test]
fn should_materialize_a_conditional_struct_argument_into_a_local() {
    let source = r#"module impl.cvi.armarg;
struct Box { arr: i32[2]; tag: i32; }
func eat(b: Box): i32 {
    return b.arr[0] + b.arr[1] + b.tag;
}
func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return eat(ok ? Box { arr: a, tag: 1 } : Box { arr: b, tag: 2 });
}
"#;
    let emitted = emit_host("cviimpl-armarg", source);
    assert_contains("cviimpl-armarg", &emitted, "ClBox __cl_owned0 = {0};");
    let branch = after("cviimpl-armarg", &emitted, "if (ok) {");
    assert_contains("cviimpl-armarg", branch, "ClBox __cl_arm");
    assert_contains("cviimpl-armarg", branch, "memcpy(&(__cl_arm");
    assert_contains("cviimpl-armarg", branch, "eat(__cl_owned0);");
    assert!(
        !emitted.contains("(ok ? (ClBox)"),
        "cviimpl-armarg: a struct conditional argument must not render as a C ternary"
    );
}

/// The same conditional materializes for a `take` argument.
#[test]
fn should_materialize_a_conditional_struct_take_argument_into_a_local() {
    let source = r#"module impl.cvi.armtake;
struct Box { arr: i32[2]; tag: i32; }
func eat(take b: Box): i32 {
    return b.arr[0] + b.arr[1] + b.tag;
}
func main(): i32 {
    let ok: bool = true;
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return eat(ok ? Box { arr: a, tag: 1 } : Box { arr: b, tag: 2 });
}
"#;
    let emitted = emit_host("cviimpl-armtake", source);
    assert_contains("cviimpl-armtake", &emitted, "ClBox __cl_tmp0 = {0};");
    assert_contains("cviimpl-armtake", &emitted, "eat(__cl_tmp0);");
}

/// A struct-literal element of an array literal that stores its fields is
/// written per field into the element.
#[test]
fn should_store_a_struct_literal_element_field_by_field() {
    let source = r#"module impl.cvi.boxarr;
struct Box { arr: i32[2]; tag: i32; }
func main(): i32 {
    let a: i32[2] = [1, 2];
    let boxes: Box[2] = [Box { arr: a, tag: 3 }, Box { arr: a, tag: 4 }];
    return boxes[0].tag;
}
"#;
    let emitted = emit_host("cviimpl-boxarr", source);
    assert_contains("cviimpl-boxarr", &emitted, "boxes[0].tag = 3;");
    assert_contains("cviimpl-boxarr", &emitted, "boxes[1].tag = 4;");
    assert!(
        !emitted.contains("= { { "),
        "cviimpl-boxarr: an array of array-field struct literals must not use a brace list"
    );
}

/// A function type in return position nests the pointer declarator.
#[test]
fn should_nest_the_declarator_when_a_function_returns_a_function_pointer() {
    let source = r#"module impl.cvi.p3ret;
func double(x: i32): i32 { return x * 2; }
func make(): func(i32): i32 { return double; }
func main(): i32 {
    return make()(4);
}
"#;
    let emitted = emit_host("cviimpl-p3ret", source);
    assert_contains(
        "cviimpl-p3ret",
        &emitted,
        "static i32 (*cl_impl_cvi_p3ret_make(void))(i32)",
    );
    assert_contains(
        "cviimpl-p3ret",
        &emitted,
        "(cl_impl_cvi_p3ret_make())(4)",
    );
}

/// A function-pointer array element is called through the element value.
///
/// An indexed callee renders the element access as the callable, so the
/// emitted call wraps the access in parentheses; the base emitted only the
/// argument, which is a valid expression and therefore built (red at the
/// phase base: `a[0](3) - 6` rendered as `(3) - 6`).
#[test]
fn should_call_through_the_element_when_a_function_pointer_array_element_is_the_callee() {
    let source = r#"module impl.cvi.elemcallee;

func twice(x: i32): i32 {
    return x * 2;
}

struct Holder {
    arr: (func(i32): i32)[2];
}

func main(): i32 {
    let a: (func(i32): i32)[2] = [twice, twice];
    let h: Holder = Holder { arr: [twice, twice] };
    return (a[0](3) - 6) + (h.arr[0](5) - 10);
}
"#;
    let emitted = emit_host("cviimpl-elemcallee", source);
    assert_contains("cviimpl-elemcallee", &emitted, "(a[0])(3)");
    assert_contains("cviimpl-elemcallee", &emitted, "(h.arr[0])(5)");
}

/// A closure struct field and its lifted function share the hidden-destination
/// signature, and the closure call passes the destination after the
/// environment.
#[test]
fn should_share_the_destination_signature_between_a_closure_field_and_its_lambda() {
    let source = r#"module impl.cvi.p3closure;
func apply<F: func(i32): i32[2]>(x: i32, f: F): i32 {
    let r: i32[2] = f(x);
    return r[1];
}
func main(): i32 {
    let base: i32 = 10;
    return apply(5, [base] (x: i32) => { return [x, x + base]; });
}
"#;
    let emitted = emit_host("cviimpl-p3closure", source);
    assert_contains(
        "cviimpl-p3closure",
        &emitted,
        "void (*fn)(struct ClEnv_0 *__env, i32, i32 __cl_ret[2]);",
    );
    assert_contains(
        "cviimpl-p3closure",
        &emitted,
        "static void cl_lambda_0(struct ClEnv_0 *__env, i32 x, i32 __cl_ret[2])",
    );
    assert_contains(
        "cviimpl-p3closure",
        &emitted,
        "((f).fn((f).env, x, __cl_call_1));",
    );
}

/// A func-typed binding that returns a fixed array declares the hidden
/// destination and calls through it.
#[test]
fn should_declare_and_call_a_fixed_array_func_value_with_a_destination() {
    let source = r#"module impl.cvi.p3kind;
func pair(x: i32): i32[2] { return [x, x + 1]; }
func main(): i32 {
    let g: func(i32): i32[2] = pair;
    let a: i32[2] = g(3);
    return a[1];
}
"#;
    let emitted = emit_host("cviimpl-p3kind", source);
    assert_contains(
        "cviimpl-p3kind",
        &emitted,
        "void (*g)(i32, i32 __cl_ret[2]) = NULL;",
    );
    assert_contains("cviimpl-p3kind", &emitted, "g(3, __cl_call_1);");
}

/// A call by name through a lambda capture reaches the environment field, and a
/// module-level func-typed global reaches its own symbol.
#[test]
fn should_spell_a_captured_call_and_a_global_call_by_their_own_storage() {
    let source = r#"module impl.cvi.p3names;
func step(x: i32): i32 { return x + 100; }
let global_fn: func(i32): i32 = step;
func main(): i32 {
    let step: func(i32): i32 = (x: i32) => x + 1;
    let captured: i32 = ([step] (y: i32) => step(y))(4);
    return captured + global_fn(3);
}
"#;
    let emitted = emit_host("cviimpl-p3names", source);
    assert_contains("cviimpl-p3names", &emitted, "(__env->step)(y)");
    assert_contains(
        "cviimpl-p3names",
        &emitted,
        "cl_impl_cvi_p3names_global_fn(3)",
    );
}

/// A func-value call widens a `String` argument to the `str` view its own
/// signature expects.
#[test]
fn should_widen_a_string_argument_for_a_func_value_parameter() {
    let source = r#"module impl.cvi.p3widen;
import { String, stringOf } from std.text;
import { print } from std.io;
func main(): i32 {
    let s: String = stringOf("hi");
    let show: func(str): void = print;
    show(s);
    return 0;
}
"#;
    let emitted = emit_host("cviimpl-p3widen", source);
    assert_contains("cviimpl-p3widen", &emitted, "show(cl_string_view(s));");
}

/// A `borrow` parameter of a function type is the pointer to the value, and
/// the call site passes the address through a binding and through a field.
#[test]
fn should_pass_the_address_for_a_borrow_func_value_parameter() {
    let source = r#"module impl.cvi.p3borrow;
import { String, stringOf } from std.text;
struct Entity { name: String; }
struct Ops { run: func(borrow Entity): void; }
func nudge(borrow e: Entity): void { }
func main(): i32 {
    let e: Entity = Entity { name: stringOf("x") };
    let bump: func(borrow Entity): void = nudge;
    bump(e);
    let ops: Ops = Ops { run: nudge };
    ops.run(e);
    return 0;
}
"#;
    let emitted = emit_host("cviimpl-p3borrow", source);
    assert_contains(
        "cviimpl-p3borrow",
        &emitted,
        "static void cl_impl_cvi_p3borrow_nudge(ClEntity *e)",
    );
    assert_contains(
        "cviimpl-p3borrow",
        &emitted,
        "void (*bump)(ClEntity *) = NULL;",
    );
    assert_contains("cviimpl-p3borrow", &emitted, "bump(&(e));");
    assert_contains("cviimpl-p3borrow", &emitted, "void (*run)(ClEntity *);");
    assert_contains("cviimpl-p3borrow", &emitted, "(ops.run)(&(e));");
}

/// A func-type `borrow` parameter spells by value when the borrowed struct is
/// duplicable, so the func value's C type matches the named function it holds.
#[test]
fn should_spell_a_duplicable_borrow_func_parameter_by_value() {
    let source = r#"module impl.cvi.rv3borrow;
struct D { value: i32; }
func nudge(borrow e: D): void { e.value = 42; }
func main(): i32 {
    let d: D = D { value: 1 };
    let g: func(borrow D): void = nudge;
    g(d);
    return d.value;
}
"#;
    let emitted = emit_host("cviimpl-rv3borrow", source);
    assert_contains(
        "cviimpl-rv3borrow",
        &emitted,
        "static void cl_impl_cvi_rv3borrow_nudge(ClD e)",
    );
    assert_contains("cviimpl-rv3borrow", &emitted, "void (*g)(ClD) = NULL;");
    assert_contains("cviimpl-rv3borrow", &emitted, "g(d);");
    assert!(
        !emitted.contains("g(&(d))"),
        "cviimpl-rv3borrow: a duplicable borrow func parameter must not take an address"
    );
}

/// A fixed-array closure call inside a short-circuit operand writes through its
/// destination from the guard expression.
#[test]
fn should_pass_the_destination_from_a_guard_expression_to_a_closure_call() {
    let source = r#"module impl.cvi.rv3guard;
func main(): i32 {
    let flag: bool = true;
    if (flag && ((x: i32) => { return [x, x + 1]; })(5)[0] == 5) {
        return 7;
    }
    return 0;
}
"#;
    let emitted = emit_host("cviimpl-rv3guard", source);
    assert_contains(
        "cviimpl-rv3guard",
        &emitted,
        "((cl_lambda_0(5, __cl_tmp0)), (__cl_tmp0[0] == 5))",
    );
}

/// A discarded fixed-array call through a module-level func-typed global still
/// passes the hidden destination.
#[test]
fn should_pass_the_destination_to_a_discarded_global_func_call() {
    let source = r#"module impl.cvi.rv3global;
func pair(x: i32): i32[2] { return [x, x + 1]; }
let global_arr: func(i32): i32[2] = pair;
func main(): i32 {
    global_arr(3);
    return 0;
}
"#;
    let emitted = emit_host("cviimpl-rv3global", source);
    assert_contains(
        "cviimpl-rv3global",
        &emitted,
        "static void (*cl_impl_cvi_rv3global_global_arr)(i32, i32 __cl_ret[2]) = NULL;",
    );
    assert_contains(
        "cviimpl-rv3global",
        &emitted,
        "cl_impl_cvi_rv3global_global_arr(3, __cl_discard",
    );
}

/// A discarded fixed-array call through a callable-bound closure parameter
/// passes the hidden destination after the environment.
#[test]
fn should_pass_the_destination_to_a_discarded_closure_parameter_call() {
    let source = r#"module impl.cvi.rv3closurediscard;
func apply<F: func(i32): i32[2]>(f: F): i32 {
    f(5);
    return 0;
}
func main(): i32 {
    let base: i32 = 10;
    return apply([base] (x: i32) => { return [x, x + base]; });
}
"#;
    let emitted = emit_host("cviimpl-rv3closurediscard", source);
    assert_contains(
        "cviimpl-rv3closurediscard",
        &emitted,
        "((f).fn((f).env, 5, __cl_discard",
    );
}

/// A call-site marker on a callable field is checked against the field's
/// declared mode.
#[test]
fn should_reject_a_marker_that_contradicts_a_callable_field_mode() {
    let source = r#"module impl.cvi.rv3marker;
struct D { value: i32; }
struct Ops { run: func(D): void; }
func plain(e: D): void { }
func main(): i32 {
    let d: D = D { value: 1 };
    let ops: Ops = Ops { run: plain };
    ops.run(borrow d);
    return 0;
}
"#;
    let scratch = ScratchDir::new("cviimpl-rv3marker");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the program");
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
        "cviimpl-rv3marker: a contradicting field marker must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error E10039:"),
        "cviimpl-rv3marker: expected E10039; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "cviimpl-rv3marker: a rejected program must not produce a translation unit"
    );
}

/// A closure-typed parameter shadows a same-named module function inside a
/// generic body.
#[test]
fn should_prefer_a_closure_parameter_over_a_same_named_module_function() {
    let source = r#"module impl.cvi.rr3shadow;
func f(x: i32): i32[2] { return [x, x + 1]; }
func apply<F: func(i32): i32>(f: F): i32 {
    f(5);
    return 0;
}
func main(): i32 {
    let base: i32 = 10;
    return apply([base] (x: i32) => { return x + base; });
}
"#;
    let emitted = emit_host("cviimpl-rr3shadow", source);
    assert_contains("cviimpl-rr3shadow", &emitted, "((f_0).fn((f_0).env, 5));");
    assert!(
        !emitted.contains("cl_impl_cvi_rr3shadow_f(5)"),
        "cviimpl-rr3shadow: the parameter must shadow the module function"
    );
}

/// A direct capturing-lambda call with a fixed-array result takes the hidden
/// destination from a guard expression.
#[test]
fn should_type_a_capturing_lambda_call_in_a_guard_with_its_real_result() {
    let source = r#"module impl.cvi.rr3capguard;
func main(): i32 {
    let flag: bool = true;
    let base: i32 = 10;
    if (flag && ([base] (x: i32) => { return [x, x + base]; })(5)[0] == 15) {
        return 7;
    }
    return 0;
}
"#;
    let emitted = emit_host("cviimpl-rr3capguard", source);
    assert_contains(
        "cviimpl-rr3capguard",
        &emitted,
        "cl_lambda_0(&(((struct ClEnv_0){ base })), 5, __cl_tmp0)",
    );
    assert_contains("cviimpl-rr3capguard", &emitted, "(__cl_tmp0[0] == 15)");
}

/// A callable-bound parameter's fixed-array call in a guard takes the hidden
/// destination from the bound signature.
#[test]
fn should_type_a_bound_parameter_call_in_a_guard_with_its_bound_result() {
    let source = r#"module impl.cvi.rr3boundguard;
func apply<F: func(i32): i32[2]>(f: F, flag: bool): i32 {
    if (flag && f(5)[0] == 5) { return 7; }
    return 0;
}
func main(): i32 {
    let base: i32 = 10;
    return apply([base] (x: i32) => { return [x, x + base]; }, true);
}
"#;
    let emitted = emit_host("cviimpl-rr3boundguard", source);
    assert_contains(
        "cviimpl-rr3boundguard",
        &emitted,
        "((f).fn((f).env, 5, __cl_tmp0))",
    );
    assert_contains("cviimpl-rr3boundguard", &emitted, "(__cl_tmp0[0] == 5)");
}

/// A closure field spells a fixed-array parameter exactly as the lifted
/// function does, so the two-word value initializes with a compatible pointer.
#[test]
fn should_spell_a_closure_field_fixed_array_parameter_like_its_lambda() {
    let source = r#"module impl.cvi.rr3closarr;
func apply<F: func(i32[2]): i32>(f: F): i32 {
    let pair: i32[2] = [1, 2];
    return f(pair);
}
func main(): i32 {
    let base: i32 = 10;
    return apply([base] (pair: i32[2]) => pair[0] + pair[1] + base);
}
"#;
    let emitted = emit_host("cviimpl-rr3closarr", source);
    assert_contains(
        "cviimpl-rr3closarr",
        &emitted,
        "i32 (*fn)(struct ClEnv_0 *__env, i32 [2]);",
    );
    assert_contains(
        "cviimpl-rr3closarr",
        &emitted,
        "static i32 cl_lambda_0(struct ClEnv_0 *__env, i32 pair[2])",
    );
}

/// A fixed-array parameter is C-adjusted to a pointer, so a move on either
/// side copies through the parameter name and zeroes by element.
#[test]
fn should_copy_and_zero_when_fixed_array_parameters_move() {
    let source = r#"module impl.cvi.p4params;
import { String, stringOf } from std.text;

func take_into(take a: String[2], take b: String[2]): i32 {
    a = b;
    return i32(len(a[0]) + len(a[1]));
}

func copy_out(take a: String[2]): i32 {
    let c: String[2] = a;
    return i32(len(c[0]));
}

func main(): i32 {
    return take_into([stringOf("xy"), stringOf("z")], [stringOf("q"), stringOf("r")])
        + copy_out([stringOf("m"), stringOf("n")]);
}
"#;
    let emitted = emit_host("cviimpl-p4params", source);
    assert_contains("cviimpl-p4params", &emitted, "memcpy(a, b, (sizeof(a[0]) * 2));");
    assert_contains("cviimpl-p4params", &emitted, "cl_zero(&b[0]);");
    assert_contains("cviimpl-p4params", &emitted, "cl_zero(&b[1]);");
    assert_contains("cviimpl-p4params", &emitted, "memcpy(&(c), a, sizeof(c));");
    assert_contains("cviimpl-p4params", &emitted, "cl_zero(&a[0]);");
    assert_contains("cviimpl-p4params", &emitted, "cl_zero(&a[1]);");
    assert!(
        !emitted.contains("cl_zero(&a);"),
        "cviimpl-p4params: the adjusted parameter must not reach cl_zero directly"
    );
}

/// A fixed-array parameter returned to the caller is copied by bytes in one
/// operation and then emptied through the element walk.
#[test]
fn should_copy_the_returned_array_when_a_fixed_array_parameter_returns() {
    let source = r#"module impl.cvi.p4return;
import { String, stringOf } from std.text;

func give_back(take a: String[2]): String[2] {
    return a;
}

func main(): i32 {
    let r: String[2] = give_back([stringOf("ab"), stringOf("cde")]);
    return i32(len(r[1]));
}
"#;
    let emitted = emit_host("cviimpl-p4return", source);
    assert_contains(
        "cviimpl-p4return",
        &emitted,
        "memcpy(__cl_ret, a, (sizeof(__cl_ret[0]) * 2));",
    );
    assert_contains("cviimpl-p4return", &emitted, "cl_zero(&a[0]);");
    assert_contains("cviimpl-p4return", &emitted, "cl_zero(&a[1]);");
    // The previous per-extent spelling assigned an array, which C rejects.
    assert!(
        !emitted.contains("__cl_ret[0] = a[0];"),
        "cviimpl-p4return: a returned array must not use an array assignment"
    );
    assert!(
        !emitted.contains("cl_zero(&a);"),
        "cviimpl-p4return: the adjusted parameter must not reach cl_zero directly"
    );
}

/// A guarded do/while is flag-first: the body runs once, then the guards and
/// the test sit at the top where the fall-through and `continue` reach them.
#[test]
fn should_run_a_first_flagged_pass_when_a_do_while_condition_has_guards() {
    let source = r#"module impl.cvi.p4dowhile;
import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("ab"), stringOf("cd")];
}

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
    } while (vlen(mk()[0]) == 2 && i < 3);
    return i;
}
"#;
    let emitted = emit_host("cviimpl-p4dowhile", source);
    assert_contains("cviimpl-p4dowhile", &emitted, "i32 __cl_first_1 = 0;");
    assert_contains("cviimpl-p4dowhile", &emitted, "if (__cl_first_1) {");
    assert_contains("cviimpl-p4dowhile", &emitted, "if (!(((");
    assert_contains("cviimpl-p4dowhile", &emitted, "__cl_first_1 = 1;");
    assert!(
        !emitted.contains("do {"),
        "cviimpl-p4dowhile: a guarded do/while must not keep the bottom test"
    );
}

/// An unguarded do/while keeps the C `do`/`while`, where a `continue` already
/// reaches the test.
#[test]
fn should_keep_the_bottom_test_when_a_do_while_condition_has_no_guards() {
    let source = r#"module impl.cvi.p4plain;
func main(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
    } while (i < 5);
    return i;
}
"#;
    let emitted = emit_host("cviimpl-p4plain", source);
    assert_contains("cviimpl-p4plain", &emitted, "do {");
    assert_contains("cviimpl-p4plain", &emitted, "} while ((i < 5));");
    assert!(!emitted.contains("__cl_first"));
}

/// A growable guard operand whose element needs no drop releases its buffer
/// and resets its count as an expression before re-materializing.
#[test]
fn should_drop_a_growable_guard_operand_before_rematerializing_it() {
    let source = r#"module impl.cvi.p4grow;
import { Array, arrayNew } from std.mem;

func growable(): Array<i32>[2] {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    let b: Array<i32> = arrayNew<i32>();
    b.push(2);
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 3 && growable()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("cviimpl-p4grow", source);
    assert_contains(
        "cviimpl-p4grow",
        &emitted,
        "(__cl_tmp0[0].data ? (FreeBuffer(__cl_tmp0[0].data), __cl_tmp0[0].data = NULL, __cl_tmp0[0].count = 0) : 0)",
    );
    assert_contains(
        "cviimpl-p4grow",
        &emitted,
        "cl_impl_cvi_p4grow_growable(__cl_tmp0)",
    );
}

/// A growable guard operand whose element needs a drop releases the previous
/// value through the hoisted statement-form drop.
///
/// The element loop has no expression form, so the guard's re-store cannot
/// carry the drop; the enclosing construct emits it as a statement before the
/// guarded store. The element's release site therefore appears once in the
/// hoist and once in the scope cleanup, and the guard still stores through its
/// temporary.
#[test]
fn should_release_the_previous_element_when_a_growable_guard_element_needs_a_drop() {
    let source = r#"module impl.cvi.p4growstr;
import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("cviimpl-p4growstr", source);
    assert_contains(
        "cviimpl-p4growstr",
        &emitted,
        "(cl_impl_cvi_p4growstr_grow(__cl_tmp0))",
    );
    assert_contains("cviimpl-p4growstr", &emitted, "if (__cl_tmp0[0].data) {");
    let release_sites = emitted.matches("__cl_tmp0[0].count = 0;").count();
    assert!(
        release_sites >= 2,
        "cviimpl-p4growstr: the previous element must be released before the re-store \
         (found {release_sites} release sites)"
    );
    assert!(
        !emitted.contains("(__cl_tmp0[0].data ? (FreeBuffer("),
        "cviimpl-p4growstr: the element loop still has no expression form"
    );
}

/// A fixed-array assignment whose source is an adjusted parameter and whose
/// element is duplicable copies through the parameter name, not its slot.
#[test]
fn should_copy_through_the_name_when_a_duplicable_array_parameter_assigns() {
    let source = r#"module impl.cvi.rv4f1;
func sum(v: i32[3]): i32 {
    let c: i32[3] = v;
    return c[0] + c[1] + c[2];
}
func main(): i32 {
    let r: i32[3] = [2, 3, 4];
    return sum(r);
}
"#;
    let emitted = emit_host("cviimpl-rv4f1", source);
    assert_contains("cviimpl-rv4f1", &emitted, "memcpy(&(c), v, sizeof(c));");
    assert!(
        !emitted.contains("&(v), sizeof(c)"),
        "cviimpl-rv4f1: the parameter slot must not be the copy source"
    );
}

/// A generic instance of a fixed-array parameter move adjusts both the copy
/// source and the element-walk zero.
#[test]
fn should_adjust_both_sides_when_a_generic_array_parameter_moves() {
    let source = r#"module impl.cvi.rv4f3;
import { String, stringOf } from std.text;
func copy<T>(take a: T[2]): i32 {
    let c: T[2] = a;
    return i32(len(c[0]));
}
func main(): i32 {
    return copy<String>([stringOf("ab"), stringOf("cde")]);
}
"#;
    let emitted = emit_host("cviimpl-rv4f3", source);
    assert_contains("cviimpl-rv4f3", &emitted, "memcpy(&(c), a, sizeof(c));");
    assert_contains("cviimpl-rv4f3", &emitted, "cl_zero(&a[0]);");
    assert_contains("cviimpl-rv4f3", &emitted, "cl_zero(&a[1]);");
    assert!(
        !emitted.contains("cl_zero(&a);"),
        "cviimpl-rv4f3: the adjusted parameter must not reach cl_zero directly"
    );
}

/// A conditional fixed-array assignment with a parameter target uses the
/// adjusted address and byte count for the save, the arm copy, and the restore.
#[test]
fn should_adjust_the_sides_when_a_conditional_stores_an_array_parameter() {
    let source = r#"module impl.cvi.rv4f4;
import { String, stringOf } from std.text;
func choose(take a: String[2], take b: String[2], ok: bool): i32 {
    a = ok ? b : a;
    return i32(len(a[0]));
}
func main(): i32 {
    return choose([stringOf("ab"), stringOf("cd")], [stringOf("xyz"), stringOf("q")], true);
}
"#;
    let emitted = emit_host("cviimpl-rv4f4", source);
    assert_contains("cviimpl-rv4f4", &emitted, "memcpy(a, b, (sizeof(a[0]) * 2));");
    assert_contains("cviimpl-rv4f4", &emitted, "cl_zero(&b[0]);");
    assert_contains("cviimpl-rv4f4", &emitted, "memcpy(a, &(__cl_save");
    assert!(
        !emitted.contains("cl_zero(&a);"),
        "cviimpl-rv4f4: the adjusted parameter must not reach cl_zero directly"
    );
}

/// A deferred `take` move-out of an array parameter inside a guarded operand
/// empties the parameter's elements in one comma expression.
#[test]
fn should_walk_the_elements_when_a_guarded_move_out_names_an_array_parameter() {
    let source = r#"module impl.cvi.rv4f2;
import { String, stringOf } from std.text;
func eat(take p: String[2]): i32 {
    return i32(len(p[0]));
}
func pass(take a: String[2]): i32 {
    let i: i32 = 0;
    if (i < 2 && eat(a) > 0) {
        i = i + 1;
    }
    return i;
}
func main(): i32 {
    return pass([stringOf("ab"), stringOf("cde")]);
}
"#;
    let emitted = emit_host("cviimpl-rv4f2", source);
    assert_contains(
        "cviimpl-rv4f2",
        &emitted,
        "((cl_zero(&a[0])), (cl_zero(&a[1])))",
    );
    assert!(
        !emitted.contains("(cl_zero(&a))"),
        "cviimpl-rv4f2: the adjusted parameter must not reach cl_zero directly"
    );
}
