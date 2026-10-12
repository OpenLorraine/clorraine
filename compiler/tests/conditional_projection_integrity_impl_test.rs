//! Implementation tests for the per-arm ownership shapes of a conditional
//! projection.
//!
//! The specification suite proves what the consuming shapes compute and that
//! they are sanitizer-clean; these cases prove how the compiler spells them:
//! the chosen place arm's member moves into an adopting temporary and is
//! zeroed in its own storage, a fresh arm's call root binds and empties before
//! the store, the guarded intrinsic argument keeps the place arm inline and
//! binds only the fresh root, and the accepted view control re-emits
//! byte-identically.

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
            "clc-cpiimpl-{}-{tag}-{sequence}",
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
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    emitted
}

/// Asserts the emitted C contains `needle`, reporting the full text otherwise.
fn assert_contains(tag: &str, emitted: &str, needle: &str) {
    assert!(
        emitted.contains(needle),
        "{tag}: expected the emitted C to contain {needle:?}; emitted C:\n{emitted}"
    );
}

/// Asserts the emitted C does not contain `needle`.
fn assert_absent(tag: &str, emitted: &str, needle: &str) {
    assert!(
        !emitted.contains(needle),
        "{tag}: expected the emitted C to lack {needle:?}; emitted C:\n{emitted}"
    );
}

/// Asserts `second` follows `first` with only `#line` markers between them.
///
/// The emitter prints a `#line` fact before most statements, so two adjacent
/// source statements may have marker lines between them; any other text means
/// a statement intervenes.
fn assert_follows(tag: &str, emitted: &str, first: &str, second: &str) {
    let start = emitted
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: {first:?} is missing; emitted C:\n{emitted}"));
    let after = &emitted[start + first.len()..];
    let second_at = after
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: {second:?} does not follow {first:?}; emitted C:\n{emitted}"));
    assert!(
        after[..second_at]
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("#line")),
        "{tag}: a statement intervenes between {first:?} and {second:?}; emitted C:\n{emitted}"
    );
}

/// Emits one program twice from one source path, returning both texts.
///
/// Both emissions share the same source path so the `#line` facts agree and
/// the comparison is about the emitted shape alone.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut texts = Vec::new();
    for pass in 0..2 {
        let c_path = scratch.path().join(format!("out{pass}.c"));
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
        texts.push(std::fs::read_to_string(&c_path).unwrap_or_default());
    }
    (texts.remove(0), texts.remove(0))
}

/// The storing case: a projection of a mixed conditional consumed by a store
/// of a two-field struct whose first field owns storage.
const STORE_SOURCE: &str = r#"module spec.cpiimpl.armstore;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = true;
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    let s: String = (c ? a : mkBox()).v;
    return i32(len(s)) + i32(len(a.w)) - 6;
}
"#;

/// A store moves the place arm's member into an adopting temporary.
///
/// The selected member is copied into the temporary and then zeroed in the
/// place arm's own storage, so the place releases only its sibling field and
/// the destination owns the value exactly once.
#[test]
fn should_move_the_place_arms_member_into_an_adopting_temporary() {
    let tag = "armstoreplace";
    let emitted = emit_host(tag, STORE_SOURCE);
    assert_follows(tag, &emitted, "__cl_tmp1 = a.v;", "a.v = NULL;");
}

/// A store binds the fresh arm's call root and empties the moved member.
///
/// The call result binds to a statement temporary, the selected member moves
/// into an adopting temporary, and the zeroed member leaves only the sibling
/// field for the root's own release.
#[test]
fn should_bind_and_empty_the_fresh_arms_root_before_the_store() {
    let tag = "armstorefresh";
    let emitted = emit_host(tag, STORE_SOURCE);
    assert_contains(
        tag,
        &emitted,
        "__cl_tmp2 = cl_spec_cpiimpl_armstore_mkBox();",
    );
    assert_follows(tag, &emitted, "__cl_tmp3 = __cl_tmp2.v;", "__cl_tmp2.v = NULL;");
}

/// A store selects between the two adopting temporaries and releases the
/// saved destination.
///
/// Each arm's branch writes its own temporary, the store selects between
/// them, and the destination's previous value is saved and released once.
#[test]
fn should_select_between_the_two_adopting_temporaries() {
    let tag = "armstoreselect";
    let emitted = emit_host(tag, STORE_SOURCE);
    assert_contains(tag, &emitted, "ClString __cl_save_1 = s;");
    assert_contains(tag, &emitted, "s = __cl_tmp1;");
    assert_contains(tag, &emitted, "s = __cl_tmp3;");
    assert_contains(tag, &emitted, "if (__cl_save_1) { FreeString(__cl_save_1); }");
}

/// The fresh arm's root releases its un-moved field after the store.
///
/// The root temporary holds only the sibling field once the member moved out,
/// so the statement-end release frees that field and leaves the moved member
/// to the destination.
#[test]
fn should_release_the_fresh_arms_un_moved_field_after_the_store() {
    let tag = "armstorerelease";
    let emitted = emit_host(tag, STORE_SOURCE);
    let moved_out = emitted
        .find("__cl_tmp3 = __cl_tmp2.v;")
        .expect("the fresh arm's member moves out");
    let released = emitted
        .rfind("if (__cl_tmp2.w) { FreeString(__cl_tmp2.w); }")
        .expect("the fresh root's sibling field releases");
    assert!(
        released > moved_out,
        "{tag}: the root's sibling release must follow the move-out; emitted C:\n{emitted}"
    );
}

/// The storing case: a guarded intrinsic argument of a mixed conditional
/// projection with a single-field struct.
const GUARD_SOURCE: &str = r#"module spec.cpiimpl.armguard;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("mk") };
}

func main(): i32 {
    let c: bool = true;
    let b: Box = Box { v: stringOf("ab") };
    if (c && i32(len((c ? b : mk()).v)) > 0) {
        return i32(len(b.v)) - 2;
    }
    return 99;
}
"#;

/// A guarded intrinsic argument keeps the place arm inline and binds the
/// fresh root.
///
/// The projection is not materialized into a released temporary: the place
/// arm stays a read of the caller's field, and only the fresh arm's call root
/// binds through the guard's deferred fill and releases once on the path that
/// evaluates it.
#[test]
fn should_keep_the_guarded_place_arm_inline_and_bind_the_fresh_root() {
    let tag = "armguard";
    let emitted = emit_host(tag, GUARD_SOURCE);
    assert_contains(tag, &emitted, "(c ? b : ");
    assert_contains(
        tag,
        &emitted,
        "__cl_tmp0 = cl_spec_cpiimpl_armguard_mk()",
    );
    assert_contains(tag, &emitted, "__cl_tmp0)).v");
    assert_absent(tag, &emitted, "= b.v;");
    assert_contains(tag, &emitted, "if (__cl_tmp0.v) { FreeString(__cl_tmp0.v); }");
}

/// The projecting return case: a function returns a projection of a mixed
/// conditional.
const RETURN_SOURCE: &str = r#"module spec.cpiimpl.armreturn;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func project(c: bool): String {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    return (c ? a : mkBox()).v;
}

func main(): i32 {
    return i32(len(project(true))) - 2;
}
"#;

/// A return moves the selected member out of the chosen arm's root.
///
/// The returned value receives the adopting temporary, the place arm's member
/// is emptied before the function's exit cleanup, and the fresh root releases
/// its un-moved field on the return path.
#[test]
fn should_move_the_member_out_for_a_projecting_return() {
    let tag = "armreturn";
    let emitted = emit_host(tag, RETURN_SOURCE);
    assert_follows(tag, &emitted, "__cl_tmp1 = a.v;", "a.v = NULL;");
    assert_contains(
        tag,
        &emitted,
        "__cl_tmp2 = cl_spec_cpiimpl_armreturn_mkBox();",
    );
    assert_follows(tag, &emitted, "__cl_tmp3 = __cl_tmp2.v;", "__cl_tmp2.v = NULL;");
    assert_contains(tag, &emitted, "__cl_ret = ");
}

/// The accepted view control: a view bound to a projection of a mixed
/// conditional with a place arm.
const VIEW_CONTROL_SOURCE: &str = r#"module spec.cpiimpl.armviewcontrol;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box {
    return Box { v: stringOf("mk"), w: stringOf("wwww") };
}

func main(): i32 {
    let c: bool = false;
    let a: Box = Box { v: stringOf("aa"), w: stringOf("wwww") };
    let s: str = (c ? a : mkBox()).v;
    return i32(len(s)) + i32(len(a.w)) - 6;
}
"#;

/// A view control re-emits byte-identically.
///
/// The accepted view binding reads through the selected root without moving
/// its member, so two emissions of the same source are the same translation
/// unit.
#[test]
fn should_emit_a_byte_identical_unit_for_the_view_control() {
    let tag = "armviewcontrol";
    let (first, second) = emit_twice(tag, VIEW_CONTROL_SOURCE);
    assert!(
        first == second,
        "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// Compiles `source` and returns the compiler's stderr, asserting rejection.
///
/// A rejected program must exit non-zero and leave no emitted translation
/// unit, so a pin can hold the report's exact code, message, and location.
fn compile_rejected(tag: &str, source: &str) -> String {
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
        "{tag}: expected a rejection; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The guarded struct-literal receiver case: a callable-field call on a
/// literal whose owning field is a fresh call.
const GUARDED_LITERAL_SOURCE: &str = r#"module spec.cpiimpl.guardliteral;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func main(): i32 {
    if (true && H { call: addOne, s: stringOf("x") }.call(5) > 0) {
        return 1;
    }
    return 0;
}
"#;

/// A guarded struct-literal receiver fills one temporary on the taken path.
///
/// The literal assigns into a statement temporary where the guard runs — the
/// skipped path never constructs it — the call reads through the temporary,
/// and the statement's release frees the literal's owning field once.
#[test]
fn should_fill_a_guarded_struct_literal_receiver_once() {
    let tag = "guardliteral";
    let emitted = emit_host(tag, GUARDED_LITERAL_SOURCE);
    assert_contains(
        tag,
        &emitted,
        "__cl_tmp0 = (ClH){ cl_spec_cpiimpl_guardliteral_addOne, cl_std_text_stringOf(cl_str(\"x\")) }",
    );
    assert_contains(tag, &emitted, "(__cl_tmp0.call)(5)");
    assert_contains(
        tag,
        &emitted,
        "__cl_tmp0.s ? (FreeString(__cl_tmp0.s), __cl_tmp0.s = NULL) : 0",
    );
    assert_contains(tag, &emitted, "if (__cl_tmp0.s) { FreeString(__cl_tmp0.s); }");
}

/// The conditional dot-`len` receiver case: each arm is a fresh two-field
/// call result.
const DOT_LEN_SOURCE: &str = r#"module spec.cpiimpl.dotlen;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(n: i32): Box {
    return Box { v: stringOf("mm"), w: stringOf("ww") };
}

func main(): i32 {
    let c: bool = true;
    let n: usize = (c ? mk(1) : mk(2)).v.len();
    return i32(n) - 2;
}
"#;

/// A conditional dot-`len` receiver binds each arm's root.
///
/// Each arm's call root fills its own temporary in its branch, the read takes
/// the length through the selected root's member, and the taken root releases
/// its un-moved sibling exactly once.
#[test]
fn should_bind_each_root_of_a_conditional_dot_len_receiver() {
    let tag = "dotlen";
    let emitted = emit_host(tag, DOT_LEN_SOURCE);
    assert_contains(tag, &emitted, "__cl_tmp1 = cl_spec_cpiimpl_dotlen_mk(1);");
    assert_contains(tag, &emitted, "__cl_tmp2 = cl_spec_cpiimpl_dotlen_mk(2);");
    assert_contains(
        tag,
        &emitted,
        "(cl_string_view((__cl_tmp0 ? __cl_tmp1 : __cl_tmp2).v)).len",
    );
    assert_contains(tag, &emitted, "if (__cl_tmp2.w) { FreeString(__cl_tmp2.w); }");
}

/// The conditional view's shared loan case: a view of a mixed conditional's
/// place arm with a write to that member afterwards.
const LOAN_SOURCE: &str = r#"module spec.cpiimpl.placearmloan;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("mm") };
}

func main(): i32 {
    let c: bool = true;
    let b: Box = Box { v: stringOf("bb") };
    let s: str = (c ? b : mk()).v;
    b.v = stringOf("zz");
    return i32(len(s));
}
"#;

/// A conditional view's place arm reports the shared loan at the mutation.
///
/// The report names the place, uses the frozen message, and points at the
/// write — exactly one report, like the named spelling.
#[test]
fn should_report_the_shared_loan_at_the_mutation() {
    let tag = "placearmloan";
    let stderr = compile_rejected(tag, LOAN_SOURCE);
    assert!(
        stderr.contains("error E10031: cannot modify 'b' while it is shared-borrowed"),
        "{tag}: expected the shared-loan report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:15:5"),
        "{tag}: expected the report at the mutation; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("error E10031").count(),
        1,
        "{tag}: expected exactly one report; stderr:\n{stderr}"
    );
}

/// The accepted `len(cond.v)` control case: the intrinsic-argument spelling
/// of a conditional projection.
const LEN_CONTROL_SOURCE: &str = r#"module spec.cpiimpl.lencontrol;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(n: i32): Box {
    return Box { v: stringOf("mm"), w: stringOf("ww") };
}

func main(): i32 {
    let c: bool = true;
    let n: usize = len((c ? mk(1) : mk(2)).v);
    return i32(n) - 2;
}
"#;

/// The `len(cond.v)` control binds the whole conditional root and re-emits
/// byte-identically.
///
/// The argument keeps its named spelling: one owned local holds the chosen
/// conditional root, so the length reads through the bound root, and two
/// emissions of the same source are the same translation unit.
#[test]
fn should_keep_the_len_argument_control_byte_identical() {
    let tag = "lencontrol";
    let (first, second) = emit_twice(tag, LEN_CONTROL_SOURCE);
    assert!(
        first == second,
        "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
    assert_contains(tag, &first, "__cl_owned0 = cl_spec_cpiimpl_lencontrol_mk(1);");
}
