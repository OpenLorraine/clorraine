//! Implementation tests for the arm-root and guarded-carrier shapes of a
//! conditional projection.
//!
//! The specification suite proves that the repaired programs run and are
//! sanitizer-clean; these cases prove how the compiler spells the repairs, so
//! a later change cannot silently drop a move-out, a root binding, or a
//! carrier: the per-instance arm move-out, the fresh root that binds because
//! it still owns un-taken storage, the literal arm's move-out, the guarded
//! `take` carrier that is filled on the evaluated path and emptied after the
//! call, and the literal receiver field whose adopting carrier gets no
//! reassigned-value release. The direct arm spellings are the untouched
//! control, and every program must re-emit byte-identically.

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
            "clc-cprimpl-{}-{tag}-{sequence}",
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

/// Compiles `source_path` into `c_path` and returns the emitted text, so the
/// determinism check can point both passes at one source path.
fn emit_at(tag: &str, source_path: &Path, c_path: &Path) -> String {
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
    let emitted = std::fs::read_to_string(c_path).unwrap_or_default();
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    emitted
}

/// Writes `source` into the scratch directory and returns its path.
fn write_source(tag: &str, scratch: &ScratchDir, source: &str) -> PathBuf {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    source_path
}

/// Emits one host-profile program and asserts the front end accepted it.
///
/// The scratch directory is returned so it outlives the emitted text.
fn emit_host(tag: &str, source: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let emitted = emit_at(tag, &source_path, &scratch.path().join("out.c"));
    (scratch, emitted)
}

/// Asserts the emitted C contains `needle`.
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

/// Asserts that `release` occurs after `move_out` in the emitted text.
fn assert_release_after(tag: &str, emitted: &str, move_out: &str, release: &str) {
    let moved = emitted
        .find(move_out)
        .unwrap_or_else(|| panic!("{tag}: {move_out:?} is missing; emitted C:\n{emitted}"));
    let released = emitted
        .rfind(release)
        .unwrap_or_else(|| panic!("{tag}: {release:?} is missing; emitted C:\n{emitted}"));
    assert!(
        released > moved,
        "{tag}: {release:?} must follow {move_out:?}; emitted C:\n{emitted}"
    );
}

/// The body of the function whose emitted signature is `signature`.
///
/// Nested blocks close with an indented brace, so the first bare `}` closes
/// the function itself.
fn function_body<'a>(tag: &str, emitted: &'a str, signature: &str) -> &'a str {
    let start = emitted
        .find(signature)
        .unwrap_or_else(|| panic!("{tag}: the emitted C does not contain {signature:?}"));
    let text = &emitted[start + signature.len()..];
    let end = text
        .find("\n}")
        .unwrap_or_else(|| panic!("{tag}: the body after {signature:?} has no closing brace"));
    &text[..end]
}

/// Asserts `second` follows `first` with only `#line` markers between them;
/// any other text means a statement intervenes.
fn assert_follows(tag: &str, emitted: &str, first: &str, second: &str) {
    let start = emitted
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: {first:?} is missing; emitted C:\n{emitted}"));
    let after = &emitted[start + first.len()..];
    let second_at = after.find(second).unwrap_or_else(|| {
        panic!("{tag}: {second:?} does not follow {first:?}; emitted C:\n{emitted}")
    });
    assert!(
        after[..second_at]
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("#line")),
        "{tag}: a statement intervenes between {first:?} and {second:?}; emitted C:\n{emitted}"
    );
}

/// Asserts the emitted line with `anchor` also contains `needle`.
///
/// A guarded operand is printed as one line, so this pins that `needle` runs
/// where the guard reaches it instead of being hoisted before the guard.
fn assert_on_line_of(tag: &str, emitted: &str, anchor: &str, needle: &str) {
    let line = emitted
        .lines()
        .find(|line| line.contains(anchor))
        .unwrap_or_else(|| panic!("{tag}: no emitted line contains {anchor:?}"));
    assert!(
        line.contains(needle),
        "{tag}: expected the line with {anchor:?} to contain {needle:?}; line:\n{line}"
    );
}

/// Emits one program twice from one source path, returning both texts.
///
/// Both emissions share the source path so the `#line` facts agree and the
/// comparison is about the emitted shape alone.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let first = emit_at(tag, &source_path, &scratch.path().join("out0.c"));
    let second = emit_at(tag, &source_path, &scratch.path().join("out1.c"));
    (first, second)
}

/// Compiles `source` and asserts the front end rejected it with exactly
/// `count` error reports, each carrying `code`, and wrote no translation unit.
///
/// The owner and return judgments reject before code generation, so their pins
/// run through the front end: the exact count keeps a fix from passing by
/// adding an unrelated report, and the missing translation unit proves the
/// rejection happened before emission.
fn assert_rejected(tag: &str, source: &str, code: &str, count: usize) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        count,
        "{tag}: expected {count} error(s), saw {}; stderr:\n{stderr}",
        errors.len()
    );
    for line in &errors {
        assert!(
            line.contains(code),
            "{tag}: expected the {code} report; saw `{line}`; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// A `T`-typed member selected between a `take` pair and a freshly built pair.
const ARM_DUTY_SOURCE: &str = r#"module spec.cpr.armduty;
import { String, stringOf } from std.text;
struct Pair<T> { v: T; w: T; }
func mkPair<T>(take x: T, take y: T): Pair<T> { return Pair<T> { v: x, w: y }; }
func pick<T>(c: bool, take a: Pair<T>, take x: T, take y: T): T {
    let s: T = (c ? a : mkPair<T>(x, y)).v;
    return s;
}

func main(): i32 {
    let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
    let q: Pair<String> = Pair<String> { v: stringOf("ef"), w: stringOf("gh") };
    let s: String = pick<String>(true, p, stringOf("ij"), stringOf("kl"));
    let t: String = pick<String>(false, q, stringOf("mn"), stringOf("op"));
    return i32(len(s)) + i32(len(t)) - 4;
}
"#;

/// A `String` local selected between a place arm and a fresh root with an
/// owning sibling field.
const ARM_FRESH_SOURCE: &str = r#"module spec.cpr.armfresh;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): i32 {
    let s: String = c ? a.v : mkBox().v;
    return i32(len(s));
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b) - 4;
}
"#;

/// A `String` local selected between a place arm and a struct literal.
const ARM_LITERAL_SOURCE: &str = r#"module spec.cpr.armliteral;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func use(c: bool, take a: Box): i32 {
    let s: String = (c ? a : Box { v: stringOf("mk"), w: stringOf("qqqq") }).v;
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b);
}
"#;

/// The same literal arm reached through a `&&` guard and read by `.len()`.
const ARM_LITERAL_GUARD_SOURCE: &str = r#"module spec.cpr.armliteralguard;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && (c ? a : Box { v: stringOf("mk"), w: stringOf("qqqq") }).v.len() > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a, true) + use(false, b, true) - 2;
}
"#;

/// A guarded `take` whose argument is a conditional projection.
const GUARD_TAKES_SOURCE: &str = r#"module spec.cpr.guardtakes;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && consume((c ? a : mkBox()).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a, true) + use(false, b, true) - 2;
}
"#;

/// The guarded `take` spelling re-executed by a loop in `main`.
const GUARD_TAKES_LOOP_SOURCE: &str = r#"module spec.cpr.guardtakesloop;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && consume((c ? a : mkBox()).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
        let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
        n = n + use(true, a, true) + use(false, b, true);
        i = i + 1;
    }
    return n - 6;
}
"#;

/// A callable-field call on a struct literal whose field projects a fresh root.
const FIELD_ROOT_SOURCE: &str = r#"module spec.cpr.fieldroot;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(g: bool): i32 {
    if (g && H { call: addOne, s: mkBox().v }.call(5) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return use(true) + use(true) - 2;
}
"#;

/// The direct arm spellings: a moved member, a fresh member read, and an
/// unguarded literal receiver.
const ARM_CONTROLS_SOURCE: &str = r#"module spec.cpr.armcontrols;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box): i32 {
    let s: String = a.v;
    let u: String = mkBox().v;
    return i32(len(s)) + i32(len(u));
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    let g: bool = H { call: addOne, s: mkBox().v }.call(5) > 0;
    return use(true, a) + use(false, b) + (g ? 1 : 0) - 9;
}
"#;

/// A generic arm moves the selected member out per instantiation.
///
/// In the `String` instantiation the place arm's member is copied into its own
/// adopting temporary and zeroed in the parameter, the fresh arm's member is
/// moved out of the built pair the same way, the store selects between the two
/// temporaries, and the pair releases its un-moved `w` field only after the
/// move-out.
#[test]
fn should_move_the_generic_arms_member_out_per_instance() {
    let tag = "armduty";
    let (_scratch, emitted) = emit_host(tag, ARM_DUTY_SOURCE);
    let signature = "cl_pick__String(bool c, ClPair__String a, ClString x, ClString y)\n{";
    let body = function_body(tag, &emitted, signature);
    assert_follows(tag, body, "__cl_tmp2 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp0.v;", "__cl_tmp0.v = NULL;");
    assert_contains(tag, body, "s = __cl_tmp2;");
    assert_contains(tag, body, "s = __cl_tmp3;");
    assert_release_after(
        tag,
        body,
        "__cl_tmp3 = __cl_tmp0.v;",
        "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }",
    );
}

/// A fresh root that still owns un-taken storage binds its own temporary: the
/// call result fills an owned binding, the member moves out, and the binding
/// releases its `w` field after the store.
#[test]
fn should_bind_the_fresh_root_that_owns_residual_storage() {
    let tag = "armfresh";
    let (_scratch, emitted) = emit_host(tag, ARM_FRESH_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_armfresh_use(bool c, ClBox a)\n{");
    assert_contains(tag, body, "ClBox __cl_tmp1 = {0};");
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_armfresh_mkBox();");
    assert_follows(tag, body, "s = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_release_after(
        tag,
        body,
        "s = __cl_tmp1.v;",
        "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }",
    );
}

/// A literal arm moves its member out and releases its own sibling field.
///
/// The plain spelling fills an owned binding, moves the member out and zeroes
/// it there, and releases the un-moved `w` after the store; the guarded
/// spelling keeps the literal inside the `g &&` guard line and still frees its
/// field once at the statement end.
#[test]
fn should_move_out_of_a_literal_projection_arm() {
    let tag = "armliteral";
    let (_plain, plain) = emit_host(tag, ARM_LITERAL_SOURCE);
    let body = function_body(tag, &plain, "cl_spec_cpr_armliteral_use(bool c, ClBox a)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp2 = (ClBox){ cl_std_text_stringOf(cl_str(\"mk\")), ",
    );
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp2.v;", "__cl_tmp2.v = NULL;");
    assert_release_after(
        tag,
        body,
        "__cl_tmp3 = __cl_tmp2.v;",
        "if (__cl_tmp2.w) { FreeString(__cl_tmp2.w); }",
    );

    let tag = "armliteralguard";
    let (_guarded, guarded) = emit_host(tag, ARM_LITERAL_GUARD_SOURCE);
    let signature = "cl_spec_cpr_armliteralguard_use(bool c, ClBox a, bool g)\n{";
    let body = function_body(tag, &guarded, signature);
    assert_on_line_of(
        tag,
        body,
        "if ((g && ((",
        "(ClBox){ cl_std_text_stringOf(cl_str(\"mk\")), cl_std_text_stringOf(cl_str(\"qqqq\")) }",
    );
    assert_contains(tag, body, "), __cl_tmp0)).v)");
    assert_contains(tag, body, "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }");
}

/// A guarded `take` fills one shared carrier and empties it after the call.
///
/// The carrier holds the chosen member only on the path its guard reaches: the
/// place arm copies and zeroes its member into the carrier inside the `&&`
/// expression, the fresh arm binds its call root and moves the member out next
/// to it, and the call reads the carrier. The post-call move-out empties it, so
/// a later execution never releases the transferred member again.
#[test]
fn should_fill_and_empty_the_guarded_take_carrier() {
    for (tag, source) in [
        ("guardtakes", GUARD_TAKES_SOURCE),
        ("guardtakesloop", GUARD_TAKES_LOOP_SOURCE),
    ] {
        let (_scratch, emitted) = emit_host(tag, source);
        let signature = format!("cl_spec_cpr_{tag}_use(bool c, ClBox a, bool g)\n{{");
        let body = function_body(tag, &emitted, &signature);
        assert_contains(tag, body, "ClString __cl_tmp1 = NULL;");
        assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
        let fresh_fill = format!(
            "(__cl_tmp0 = cl_spec_cpr_{tag}_mkBox())), __cl_tmp0.v), (__cl_tmp0.v = NULL))"
        );
        assert_contains(tag, body, &fresh_fill);
        let call_and_empty =
            format!("(__cl_tmp2 = cl_spec_cpr_{tag}_consume(__cl_tmp1)), (__cl_tmp1 = NULL))");
        assert_contains(tag, body, &call_and_empty);
        assert_absent(tag, body, "if (__cl_tmp1) { FreeString(__cl_tmp1); }");
    }
}

/// A literal receiver field binds its projected root inside the guard.
///
/// The field's value projects a fresh call root, so the guard expression binds
/// the root, moves the member through the adopting carrier the field stores,
/// and releases the root's un-moved `w` field. The adopting carrier gets no
/// reassigned-value release — the field owns the value — while the receiver's
/// own reassigned-value release stays.
#[test]
fn should_bind_the_receiver_fields_root_inside_the_guard() {
    let tag = "fieldroot";
    let (_scratch, emitted) = emit_host(tag, FIELD_ROOT_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_fieldroot_use(bool g)\n{");
    assert_on_line_of(
        tag,
        body,
        "if ((g && ((",
        "(__cl_tmp0 = cl_spec_cpr_fieldroot_mkBox())",
    );
    assert_contains(tag, body, "(__cl_tmp1 = __cl_tmp0.v), (__cl_tmp0.v = NULL)");
    assert_contains(
        tag,
        body,
        "(ClH){ cl_spec_cpr_fieldroot_addOne, __cl_tmp1 }",
    );
    assert_contains(
        tag,
        body,
        "(__cl_tmp2.s ? (FreeString(__cl_tmp2.s), __cl_tmp2.s = NULL) : 0)",
    );
    assert_absent(tag, body, "__cl_tmp1 ?");
    assert_contains(tag, body, "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }");
}

/// The direct arm spellings keep their pinned shapes: the moved `take`
/// member, the fresh member read through an owned scope binding, and the
/// unguarded literal receiver.
#[test]
fn should_keep_the_direct_arm_spellings() {
    let tag = "armcontrols";
    let (_scratch, emitted) = emit_host(tag, ARM_CONTROLS_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_armcontrols_use(bool c, ClBox a)\n{");
    assert_follows(tag, body, "s = a.v;", "a.v = NULL;");
    assert_contains(tag, body, "__cl_owned0 = cl_spec_cpr_armcontrols_mkBox();");
    assert_follows(tag, body, "u = __cl_owned0.v;", "__cl_owned0.v = NULL;");
    assert_contains(tag, body, "if (__cl_owned0.w) { FreeString(__cl_owned0.w); }");
}

/// Every pinned program re-emits byte-identically: two emissions of one file
/// must produce the same text, so the shapes never depend on run order.
#[test]
fn should_re_emit_every_projection_program_byte_identically() {
    let cases: [(&str, &str); 8] = [
        ("armduty", ARM_DUTY_SOURCE),
        ("armfresh", ARM_FRESH_SOURCE),
        ("armliteral", ARM_LITERAL_SOURCE),
        ("armliteralguard", ARM_LITERAL_GUARD_SOURCE),
        ("guardtakes", GUARD_TAKES_SOURCE),
        ("guardtakesloop", GUARD_TAKES_LOOP_SOURCE),
        ("fieldroot", FIELD_ROOT_SOURCE),
        ("armcontrols", ARM_CONTROLS_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

/// A guarded callable receiver whose array field adopts a fresh holder's field
/// and whose guarded statement re-executes in a loop.
const GUARDED_ARRAY_FIELD_SOURCE: &str = r#"module spec.cpr.statereuse;
import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;
struct Holder { data: Array<String>; tag: String; }
struct H { call: func(i32): i32; s: Array<String>; }
func mkHolder(): Holder {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    return Holder { data: a, tag: stringOf("tt") };
}
func addOne(x: i32): i32 { return x + 1; }
func use(g: bool, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        if (g && H { call: addOne, s: mkHolder().data }.call(5) > 0) {
            n = n + 1;
        }
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return use(true, 3) - 3;
}
"#;

/// A guarded receiver's adopted array field declares its carrier and fills it
/// inside the guard, and no statement-form release of the carrier is hoisted.
///
/// The array's drop has no expression form, so the emitter could otherwise
/// print a statement-form release of the carrier's previous value before the
/// guarded `if`. The adopting carrier's consumer — the receiver's field — owns
/// the value, so no such release may run: a re-executed guard would free the
/// storage the receiver's own release already freed.
#[test]
fn should_not_hoist_the_adopting_carriers_statement_form_drop() {
    let tag = "statereuse";
    let (_scratch, emitted) = emit_host(tag, GUARDED_ARRAY_FIELD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_statereuse_use(bool g, i32 k)\n{");
    assert_contains(tag, body, "ClArray__String __cl_tmp1 = {0};");
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = __cl_tmp0.data)");
    assert_absent(tag, body, "if (__cl_tmp1.data) {");
    assert_absent(tag, body, "cl_buffer_ptr(__cl_tmp1.data)");
}

/// A generic take projection and a generic return projection.
const GENERIC_TAKE_SOURCE: &str = r#"module spec.cpr.generictake;
import { String, stringOf } from std.text;
struct Pair<T> { v: T; w: T; }
func mkPair<T>(take x: T, take y: T): Pair<T> { return Pair<T> { v: x, w: y }; }
func hold<T>(take s: T): i32 { return 1; }
func eager<T>(c: bool, take a: Pair<T>, take x: T, take y: T): i32 {
    return hold<T>((c ? a : mkPair<T>(x, y)).v);
}
func guarded<T>(c: bool, take a: Pair<T>, take x: T, take y: T, g: bool): i32 {
    if (g && hold<T>((c ? a : mkPair<T>(x, y)).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
    let q: Pair<String> = Pair<String> { v: stringOf("ef"), w: stringOf("gh") };
    let a: Pair<i32> = Pair<i32> { v: 1, w: 2 };
    let b: Pair<i32> = Pair<i32> { v: 3, w: 4 };
    return eager<String>(true, p, stringOf("ij"), stringOf("kl"))
         + eager<i32>(true, a, 5, 6)
         + guarded<String>(true, q, stringOf("mn"), stringOf("op"), true)
         + guarded<i32>(true, b, 7, 8, true);
}
"#;

/// The generic take moves the chosen member per instance.
///
/// The `String` instances fill the carrier from the chosen arm and empty that
/// arm's member before the call, and the post-call move-out empties the
/// carrier; the `i32` instances copy, so their arm keeps its value. The guarded
/// spelling fills the same carrier inside its guard line.
#[test]
fn should_move_a_generic_take_projection_per_instance() {
    let tag = "generictake";
    let (_scratch, emitted) = emit_host(tag, GENERIC_TAKE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_eager__String(bool c, ClPair__String a, ClString x, ClString y)\n{",
    );
    assert_contains(tag, body, "ClString __cl_tmp1 = NULL;");
    assert_follows(tag, body, "__cl_tmp1 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp1 = __cl_tmp0.v;", "__cl_tmp0.v = NULL;");
    assert_follows(tag, body, "cl_hold__String(__cl_tmp1);", "__cl_tmp1 = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_eager__i32(bool c, ClPair__i32 a, i32 x, i32 y)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = a.v;");
    assert_absent(tag, body, "a.v = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_guarded__String(bool c, ClPair__String a, ClString x, ClString y, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
    assert_contains(tag, body, "(__cl_tmp1 = NULL)");
}

/// A generic return projection and its concrete spelling.
const GENERIC_RETURN_SOURCE: &str = r#"module spec.cpr.genericreturn;
import { String, stringOf } from std.text;
struct Pair<T> { v: T; w: T; }
func mkPair<T>(take x: T, take y: T): Pair<T> { return Pair<T> { v: x, w: y }; }
func pick<T>(c: bool, take a: Pair<T>, take x: T, take y: T): T {
    return (c ? a : mkPair<T>(x, y)).v;
}

func main(): i32 {
    let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
    let a: Pair<i32> = Pair<i32> { v: 1, w: 2 };
    return i32(len(pick<String>(true, p, stringOf("ij"), stringOf("kl"))))
         + pick<i32>(true, a, 5, 6);
}
"#;

/// The generic return moves the chosen member per instance.
///
/// The `String` instance copies the chosen member into the return slot and
/// empties the arm that produced it — the place arm's field and the fresh
/// root's selected field alike — so the root releases only its un-moved field;
/// the `i32` instance copies and keeps its arm's value.
#[test]
fn should_move_a_generic_return_projection_per_instance() {
    let tag = "genericreturn";
    let (_scratch, emitted) = emit_host(tag, GENERIC_RETURN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_pick__String(bool c, ClPair__String a, ClString x, ClString y)\n{",
    );
    assert_contains(tag, body, "ClString __cl_tmp2 = NULL;");
    assert_follows(tag, body, "__cl_tmp2 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp0.v;", "__cl_tmp0.v = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_pick__i32(bool c, ClPair__i32 a, i32 x, i32 y)\n{",
    );
    assert_absent(tag, body, "a.v = NULL;");
}

/// The three nested conditional take spellings.
const NESTED_THEN_SOURCE: &str = r#"module spec.cpr.nestedthen;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, d: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume((c ? (d ? a : mkBox()) : b).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const NESTED_ELSE_SOURCE: &str = r#"module spec.cpr.nestedelse;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, d: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume((c ? b : (d ? a : mkBox())).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const NESTED_BOTH_SOURCE: &str = r#"module spec.cpr.nestedboth;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func mkBox2(): Box { return Box { v: stringOf("ef"), w: stringOf("gh") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, d: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume((c ? (d ? mkBox() : mkBox2()) : b).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

/// A nested conditional take empties the chosen member at every level.
///
/// Each guard line moves the leaf the inner condition selects into the shared
/// carrier and empties that leaf's member; when only the outer level moved, the
/// inner level's root kept and released the transferred storage again.
#[test]
fn should_move_a_nested_conditional_take_at_every_level() {
    let tag = "nestedthen";
    let (_scratch, emitted) = emit_host(tag, NESTED_THEN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedthen_use(bool c, bool d, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(d ? (__cl_tmp1 = a.v, (a.v = NULL))");
    assert_on_line_of(tag, body, "if ((g && ((", ": (__cl_tmp1 = b.v, (b.v = NULL))");

    let tag = "nestedelse";
    let (_scratch, emitted) = emit_host(tag, NESTED_ELSE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedelse_use(bool c, bool d, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(c ? (__cl_tmp1 = b.v, (b.v = NULL))");
    assert_on_line_of(tag, body, "if ((g && ((", "(d ? (__cl_tmp1 = a.v, (a.v = NULL))");

    let tag = "nestedboth";
    let (_scratch, emitted) = emit_host(tag, NESTED_BOTH_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedboth_use(bool c, bool d, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "__cl_tmp0.v), (__cl_tmp0.v = NULL)");
    assert_on_line_of(tag, body, "if ((g && ((", "__cl_tmp1.v), (__cl_tmp1.v = NULL)");
    assert_on_line_of(tag, body, "if ((g && ((", ": (__cl_tmp2 = b.v, (b.v = NULL))");
}

/// A conditional projection in a callable receiver's field, both spellings.
const FIELD_COND_GUARDED_SOURCE: &str = r#"module spec.cpr.fieldcondguarded;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && H { call: addOne, s: (c ? a : mkBox()).v }.call(5) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const FIELD_COND_EAGER_SOURCE: &str = r#"module spec.cpr.fieldcondeager;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box): i32 {
    let r: i32 = H { call: addOne, s: (c ? a : mkBox()).v }.call(5);
    return r;
}

func main(): i32 {
    return 0;
}
"#;

/// A receiver field takes the chosen member from its arm's carrier.
///
/// The guarded spelling fills the carrier inside the guard line, moves each
/// arm's member out, and builds the receiver from the carrier; the eager
/// spelling fills one temporary per arm and stores the chosen one into the
/// field, emptying that temporary.
#[test]
fn should_move_a_conditional_projection_into_a_receiver_field() {
    let tag = "fieldcondguarded";
    let (_scratch, emitted) = emit_host(tag, FIELD_COND_GUARDED_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_fieldcondguarded_use(bool c, ClBox a, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
    assert_contains(
        tag,
        body,
        "(ClH){ cl_spec_cpr_fieldcondguarded_addOne, __cl_tmp1 }",
    );

    let tag = "fieldcondeager";
    let (_scratch, emitted) = emit_host(tag, FIELD_COND_EAGER_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_fieldcondeager_use(bool c, ClBox a)\n{",
    );
    assert_follows(tag, body, "__cl_tmp1 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp2.v;", "__cl_tmp2.v = NULL;");
    assert_follows(tag, body, "__cl_owned0.s = __cl_tmp1;", "__cl_tmp1 = NULL;");
    assert_follows(tag, body, "__cl_owned0.s = __cl_tmp3;", "__cl_tmp3 = NULL;");
}

/// A conditional of projections in a plain literal field, an array element, an
/// eager take argument, and an owning return.
const FIELD_COND_PLAIN_SOURCE: &str = r#"module spec.cpr.fieldcondplain;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct Out { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): i32 {
    let o: Out = Out { v: c ? a.v : mkBox().v, w: stringOf("z") };
    return i32(len(o.v)) - 2;
}

func main(): i32 {
    return 0;
}
"#;

const ELEMENT_COND_SOURCE: &str = r#"module spec.cpr.elementcond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): i32 {
    let arr: String[2] = [c ? a.v : mkBox().v, stringOf("z")];
    return i32(len(arr[0])) - 2;
}

func main(): i32 {
    return 0;
}
"#;

const TAKE_COND_SOURCE: &str = r#"module spec.cpr.takecond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, take a: Box): i32 {
    return consume(c ? a.v : mkBox().v);
}

func main(): i32 {
    return 0;
}
"#;

const RETURN_COND_SOURCE: &str = r#"module spec.cpr.returncond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): String {
    return c ? a.v : mkBox().v;
}

func main(): i32 {
    return 0;
}
"#;

/// Every remaining consuming position binds the fresh arm's root.
///
/// The literal field stores the chosen member into its field, the array
/// element stores it into the element, the eager take moves it into the
/// argument temporary, and the return moves it into the return slot; each
/// position also releases the bound root's un-moved field.
#[test]
fn should_bind_the_conditional_field_and_element_roots() {
    let tag = "fieldcondplain";
    let (_scratch, emitted) = emit_host(tag, FIELD_COND_PLAIN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_fieldcondplain_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_fieldcondplain_mkBox();");
    assert_follows(tag, body, "o.v = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");

    let tag = "elementcond";
    let (_scratch, emitted) = emit_host(tag, ELEMENT_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_elementcond_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_elementcond_mkBox();");
    assert_follows(tag, body, "arr[0] = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");

    let tag = "takecond";
    let (_scratch, emitted) = emit_host(tag, TAKE_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_takecond_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_takecond_mkBox();");
    assert_follows(tag, body, "__cl_tmp2 = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");

    let tag = "returncond";
    let (_scratch, emitted) = emit_host(tag, RETURN_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_returncond_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_returncond_mkBox();");
    assert_follows(tag, body, "__cl_ret = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");
}

/// A deferred bare conditional of projections in a guarded field and a `?:`
/// arm element.
const GUARDED_FIELD_COND_SOURCE: &str = r#"module spec.cpr.guardedfieldcond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && H { call: addOne, s: c ? a.v : mkBox().v }.call(5) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const ARM_ELEM_COND_SOURCE: &str = r#"module spec.cpr.armelemcond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func lenElem(names: String[2]): i32 { return i32(len(names[0])) - 2; }
func use(c: bool, take a: Box, g: bool): i32 {
    return g ? lenElem([c ? a.v : mkBox().v, stringOf("z")]) : 0;
}

func main(): i32 {
    return 0;
}
"#;

/// A deferred bare conditional fills an adopting carrier on its evaluated path.
///
/// The guarded receiver field fills the carrier inside its guard line and
/// builds the receiver from it; the `?:` arm element fills the carrier in the
/// branch that evaluates the arm and copies it into the element. Each fill
/// moves the chosen member and empties its source, and the fresh arm's root
/// releases only its un-moved field.
#[test]
fn should_fill_the_deferred_conditional_carriers() {
    let tag = "guardedfieldcond";
    let (_scratch, emitted) = emit_host(tag, GUARDED_FIELD_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_guardedfieldcond_use(bool c, ClBox a, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
    assert_contains(
        tag,
        body,
        "(ClH){ cl_spec_cpr_guardedfieldcond_addOne, __cl_tmp1 }",
    );
    assert_contains(tag, body, "__cl_tmp0.v = NULL");

    let tag = "armelemcond";
    let (_scratch, emitted) = emit_host(tag, ARM_ELEM_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_armelemcond_use(bool c, ClBox a, bool g)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = a.v;");
    assert_follows(
        tag,
        body,
        "__cl_tmp0 = cl_spec_cpr_armelemcond_mkBox();",
        "__cl_tmp1 = __cl_tmp0.v;",
    );
    assert_follows(tag, body, "__cl_owned0[0] = __cl_tmp1;", "__cl_tmp1 = NULL;");
}

/// An indexed conditional field of a duplicable value.
const DUPLICABLE_INDEX_FIELD_SOURCE: &str = r#"module spec.cpr.dupindexfield;
struct Out { f: i32; w: i32; }
func use(c: bool, a: i32[2], b: i32[2]): i32 {
    let o: Out = Out { f: c ? a[1] : b[0], w: 1 };
    return o.f - 2;
}

func main(): i32 {
    return 0;
}
"#;

/// A duplicable indexed conditional field keeps the inline ternary spelling.
///
/// The value copies, so the field needs no root binding and no per-arm
/// consume: the condition is not saved into a temporary, no empty branch
/// wraps the store, and the field is written from the ternary directly.
#[test]
fn should_keep_the_inline_ternary_for_a_duplicable_index_field() {
    let tag = "dupindexfield";
    let (_scratch, emitted) = emit_host(tag, DUPLICABLE_INDEX_FIELD_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_dupindexfield_use(bool c, i32 a[2], i32 b[2])\n{",
    );
    assert_contains(tag, body, "o = (ClOut){ (c ? a[1] : b[0]), 1 };");
    assert_absent(tag, body, "if (__cl_tmp0) {");
}

/// A bound literal with an owned sibling field source.
const LITERAL_FIELD_MOVE_SOURCE: &str = r#"module spec.cpr.literalfieldmove;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct Out2 { v: String; w: Box; }
struct Out { v: String; w: String; }
func consume(take s: String): i32 { return i32(len(s)); }
func eager(c: bool, take a: Box, take b: Box): i32 {
    return consume(c ? a.v : Out2 { v: stringOf("zz"), w: b }.v) - 2;
}
func store(c: bool, take a: Box, take b: Box): i32 {
    let o: Out = Out { v: c ? a.v : Out2 { v: stringOf("zz"), w: b }.v, w: stringOf("q") };
    return i32(len(o.v)) - 2;
}
func guarded(c: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume(c ? a.v : Out2 { v: stringOf("zz"), w: b }.v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

/// Binding a literal arm moves its owned field source, never copies it.
///
/// The eager and guarded spellings fill the literal's frame field from the
/// source temporary and empty both the source and the temporary, so the
/// literal's release owns the moved value and the source's own cleanup finds
/// nothing. A brace-list copy would leave the source and the literal both
/// owning the same storage.
#[test]
fn should_move_a_bound_literals_owned_field_source() {
    let tag = "literalfieldmove";
    let (_scratch, emitted) = emit_host(tag, LITERAL_FIELD_MOVE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_literalfieldmove_eager(bool c, ClBox a, ClBox b)\n{",
    );
    assert_follows(tag, body, "__cl_tmp1 = b;", "cl_zero(&b);");
    assert_follows(tag, body, "__cl_tmp2.w = __cl_tmp1;", "cl_zero(&__cl_tmp1);");

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_literalfieldmove_guarded(bool c, ClBox a, ClBox b, bool g)\n{",
    );
    assert_contains(tag, body, "(__cl_tmp0 = b), (cl_zero(&b))");
}

/// A nested literal field and an array-literal field element holding places.
const NESTED_LITERAL_MOVE_SOURCE: &str = r#"module spec.cpr.nestedliteralmove;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct Inner { v: String; w: String; }
struct Outer { inner: Inner; tag: String; }
struct OutA { v: String; w: String[2]; }
struct Out { v: String; w: String; }
func consume(take s: String): i32 { return i32(len(s)); }
func guarded(c: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume(c ? a.v : Outer { inner: Inner { v: stringOf("zz"), w: b.w }, tag: stringOf("t") }.inner.v) > 0) {
        return 1;
    }
    return 0;
}
func eagerField(c: bool, take a: Box, take b: Box): i32 {
    let o: Out = Out { v: c ? a.v : Outer { inner: Inner { v: stringOf("zz"), w: b.w }, tag: stringOf("t") }.inner.v, w: stringOf("q") };
    return i32(len(o.v)) - 2;
}
func arrayField(c: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume(c ? a.v : OutA { v: stringOf("zz"), w: [b.w, stringOf("q")] }.v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

/// Binding a literal moves a place reached inside a nested field or element.
///
/// The nested literal's `w` field moves `b.w` before the outer literal
/// materializes, and the array field's first element moves `b.w` the same
/// way: each source empties at its own depth, so the literal holds the only
/// reference and the source's cleanup finds nothing.
#[test]
fn should_move_a_nested_literals_field_sources() {
    let tag = "nestedliteralmove";
    let (_scratch, emitted) = emit_host(tag, NESTED_LITERAL_MOVE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedliteralmove_guarded(bool c, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp0 = b.w), (b.w = NULL)");
    assert_contains(
        tag,
        body,
        "(ClInner){ cl_std_text_stringOf(cl_str(\"zz\")), __cl_tmp0 }",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedliteralmove_eagerField(bool c, ClBox a, ClBox b)\n{",
    );
    assert_follows(tag, body, "__cl_tmp1 = b.w;", "b.w = NULL;");
    assert_follows(tag, body, "__cl_tmp2.inner.w = __cl_tmp1;", "__cl_tmp1 = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedliteralmove_arrayField(bool c, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp0 = b.w), (b.w = NULL)");
    assert_contains(tag, body, "{ __cl_tmp0, cl_std_text_stringOf(cl_str(\"q\")) }");
}

// ---------------------------------------------------------------------------
// The call-wrapped owner walk, the view-element argument and field positions,
// the `str[1]` return, and the mixed-return one-report rule. A rejected
// program pins its front-end judgment; an accepted program pins the emitted
// adoption shape and re-emits byte-identically.
// ---------------------------------------------------------------------------

/// A call-wrapped conditional view used while both parameter backings live.
///
/// The accepted safe twin keeps the call inline over both arms; only a write
/// to an arm's backing while the view is live reports.
const WRAP_SAFE_SOURCE: &str = r#"module spec.cpr.wrapsafe;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: u8[] = bytes((c ? b : d).v);
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, true);
}
"#;

/// A call-wrapped conditional view whose first arm's backing is written.
const OWNER_FIRST_SOURCE: &str = r#"module spec.cpr.ownerfirst;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: u8[] = bytes((c ? b : d).v);
    b.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, true);
}
"#;

/// The two-arm spelling `bytes(c ? b.v : d.v)` whose second arm is written.
const OWNER_TWOARM_SOURCE: &str = r#"module spec.cpr.ownertwoarm;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: u8[] = bytes(c ? b.v : d.v);
    d.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, false);
}
"#;

/// A chained view-returning callee whose second arm's backing is written.
const OWNER_CHAINED_SOURCE: &str = r#"module spec.cpr.ownerchained;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func peek(x: str): str { return x; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: str = peek((c ? b : d).v);
    d.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, false);
}
"#;

/// A view-element array passed as an argument.
const ELEM_ARG_SOURCE: &str = r#"module spec.cpr.elemarg;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(names: str[1]): i32 { return i32(len(names[0])); }

func main(): i32 {
    return use([mk().v]) - 2;
}
"#;

/// A view-element array stored into a struct-literal field.
const ELEM_FIELD_SOURCE: &str = r#"module spec.cpr.elemfield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func main(): i32 {
    let h: Holder = Holder { names: [mk().v] };
    return i32(len(h.names[0])) - 2;
}
"#;

/// A `str[1]` return whose element derives from a by-value `String` parameter.
const ARRAY_RETURN_SOURCE: &str = r#"module spec.cpr.arrayreturn;

import { String, stringOf } from std.text;

func f(s: String): str[1] { return [s]; }

func main(): i32 {
    let s: String = stringOf("ab");
    let a: str[1] = f(s);
    return i32(len(a[0])) - 2;
}
"#;

/// The same `str[1]` return deriving from a parameter member.
const ARRAY_RETURN_MEMBER_SOURCE: &str = r#"module spec.cpr.arrayreturnmember;

import { String } from std.text;

struct Box { v: String; }

func f(h: Box): str[1] { return [h.v]; }

func main(): i32 { return 0; }
"#;

/// A mixed conditional view return over a local and a fresh root.
const ONE_REPORT_MIXED_SOURCE: &str = r#"module spec.cpr.onereportmixed;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    return (c ? a : mkBox()).v;
}

func main(): i32 { return 0; }
"#;

/// A conditional view return over a local and a parameter.
const ONE_REPORT_LOCALPARAM_SOURCE: &str = r#"module spec.cpr.onereportlocalparam;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool, a: str): str {
    let b: Box = Box { v: stringOf("cd") };
    return (c ? b.v : a);
}

func main(): i32 { return 0; }
"#;

/// A conditional view return over two locals.
const ONE_REPORT_LOCALONLY_SOURCE: &str = r#"module spec.cpr.onereportlocalonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    let b: Box = Box { v: stringOf("ef") };
    return (c ? a : b).v;
}

func main(): i32 { return 0; }
"#;

/// A write to either arm's backing while a call-wrapped conditional view is
/// live reports the plain twin's mutation error exactly once.
///
/// `bytes(b.v)` reports one `E10031` at the write because the view loans `b`.
/// The call-wrapped conditional must record every arm's owner, so each of the
/// three programs reports exactly that one error at its mutation and emits no
/// translation unit; the accepted safe twin keeps its inline call over both
/// arms.
#[test]
fn should_reject_a_call_wrapped_conditional_view_mutation() {
    let tag = "wrapsafe";
    let (_scratch, emitted) = emit_host(tag, WRAP_SAFE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_wrapsafe_f(ClBox *b, ClBox *d, bool c)\n{");
    assert_contains(tag, body, "s = cl_std_text_bytes((c ? (*b) : (*d)).v);");

    assert_rejected("ownerfirst", OWNER_FIRST_SOURCE, "E10031", 1);
    assert_rejected("ownertwoarm", OWNER_TWOARM_SOURCE, "E10031", 1);
    assert_rejected("ownerchained", OWNER_CHAINED_SOURCE, "E10031", 1);
}

/// A view-element array in an argument or field position adopts its temporary
/// element's backing in the destination frame.
///
/// The fresh `mk()` root binds as one owned local, the element view is built
/// from that local's member, and the local's fields release at the frame's
/// end, so no moved-string temporary is left unreleased.
#[test]
fn should_adopt_the_backing_of_a_view_element_array_in_argument_and_field_positions() {
    let tag = "elemarg";
    let (_scratch, emitted) = emit_host(tag, ELEM_ARG_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_elemarg_main(void)\n{");
    assert_contains(tag, body, "ClBox __cl_owned0 = {0};");
    assert_contains(tag, body, "__cl_owned0 = cl_spec_cpr_elemarg_mk();");
    assert_contains(tag, body, "__cl_owned1[0] = cl_string_view(__cl_owned0.v);");
    assert_release_after(
        tag,
        body,
        "__cl_owned1[0] = cl_string_view(__cl_owned0.v);",
        "if (__cl_owned0.w) { FreeString(__cl_owned0.w); }",
    );
    assert_absent(tag, body, "ClString __cl_tmp1 = NULL;");

    let tag = "elemfield";
    let (_scratch, emitted) = emit_host(tag, ELEM_FIELD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_elemfield_main(void)\n{");
    assert_contains(tag, body, "ClBox __cl_owned0 = {0};");
    assert_contains(tag, body, "__cl_owned0 = cl_spec_cpr_elemfield_mk();");
    assert_contains(tag, body, "h.names[0] = cl_string_view(__cl_owned0.v);");
    assert_release_after(
        tag,
        body,
        "h.names[0] = cl_string_view(__cl_owned0.v);",
        "if (__cl_owned0.w) { FreeString(__cl_owned0.w); }",
    );
    assert_absent(tag, body, "ClString __cl_tmp1 = NULL;");
}

/// A `str[1]` return matches its single-view twin: the accepted parameter
/// spelling returns the view without consuming the source, and the member
/// spelling reports the twin's single `E10036`.
#[test]
fn should_return_a_view_element_array_without_consuming_its_source() {
    let tag = "arrayreturn";
    let (_scratch, emitted) = emit_host(tag, ARRAY_RETURN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_arrayreturn_f(ClString s, ClStr __cl_ret[1])\n{",
    );
    assert_contains(tag, body, "__cl_ret[0] = cl_string_view(s);");
    assert_absent(tag, body, "s = NULL;");
    assert_absent(tag, body, "FreeString(s)");

    assert_rejected("arrayreturnmember", ARRAY_RETURN_MEMBER_SOURCE, "E10036", 1);
}

/// A mixed local/generated view return reports exactly one error — the
/// generated-owner escape — while a local-only backing keeps one local-owner
/// report.
#[test]
fn should_report_the_mixed_view_return_once() {
    assert_rejected("onereportmixed", ONE_REPORT_MIXED_SOURCE, "E10030", 1);
    assert_rejected("onereportlocalparam", ONE_REPORT_LOCALPARAM_SOURCE, "E10035", 1);
    assert_rejected("onereportlocalonly", ONE_REPORT_LOCALONLY_SOURCE, "E10035", 1);
}

/// The new view-element programs re-emit byte-identically.
#[test]
fn should_re_emit_the_new_view_element_programs_byte_identically() {
    let cases: [(&str, &str); 4] = [
        ("wrapsafe", WRAP_SAFE_SOURCE),
        ("elemarg", ELEM_ARG_SOURCE),
        ("elemfield", ELEM_FIELD_SOURCE),
        ("arrayreturn", ARRAY_RETURN_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}
