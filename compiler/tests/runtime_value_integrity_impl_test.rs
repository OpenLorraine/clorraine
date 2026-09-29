//! Implementation tests for the emitted shape of statement temporaries and
//! conditional fixed-array values.
//!
//! The specification suite proves *what* a runtime value program computes;
//! these cases prove *how* the emitter spells it. A statement that stores an
//! owning value in a compiler temporary releases that temporary in place when
//! the statement ends, and the release names the temporary directly. A value
//! that travels to a binding (a `take` call result or an aggregate-literal
//! adoption) is not released at the statement's end, because the binding owns
//! it. A `for` loop over an ownerless iterable reads one materialized
//! temporary, and an owning iterable is released after the loop. A
//! fixed-array conditional is stored through a branch that evaluates only the
//! chosen arm, with the arm's own guards inside that branch, in both the
//! assignment and the return positions; an owned reassignment saves the old
//! target first and releases it once after the branch.

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
            "clc-rviimpl-{}-{tag}-{sequence}",
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

/// A call result element read stores the call in a temporary and the read
/// releases the temporary right after the statement.
#[test]
fn should_release_an_owning_statement_temporary_after_its_statement() {
    let source = r#"module impl.rvi.temp;
import { String, stringOf } from std.text;
func mk(): String[2] { return [stringOf("ab"), stringOf("cde")]; }
func main(): i32 {
    let n: i32 = i32(len(mk()[0]));
    return n;
}
"#;
    let emitted = emit_host("rviimpl-temp", source);
    assert_contains("rviimpl-temp", &emitted, "cl_impl_rvi_temp_mk(__cl_tmp0);");
    assert_contains(
        "rviimpl-temp",
        &emitted,
        "n = (i32)((cl_string_view(__cl_tmp0[0])).len);",
    );
    // The release runs after the read and drops both elements.
    let tail = after("rviimpl-temp", &emitted, "n = (i32)((cl_string_view");
    let release = after("rviimpl-temp", tail, "if (__cl_tmp0[0]) { FreeString(__cl_tmp0[0]); }");
    assert_contains(
        "rviimpl-temp",
        release,
        "if (__cl_tmp0[1]) { FreeString(__cl_tmp0[1]); }",
    );
}

/// A return expression that reads a statement temporary releases it, and no
/// second owner is bound for the element the temporary owns.
#[test]
fn should_release_a_return_expression_temporary_without_a_second_owner() {
    let source = r#"module impl.rvi.ret;
import { String, stringOf } from std.text;
func mk(): String[2] { return [stringOf("ab"), stringOf("cde")]; }
func vlen(s: str): i32 { return i32(len(s)); }
func main(): i32 { return vlen(mk()[1]); }
"#;
    let emitted = emit_host("rviimpl-ret", source);
    assert_contains("rviimpl-ret", &emitted, "cl_impl_rvi_ret_mk(__cl_tmp0);");
    assert!(
        !emitted.contains("__cl_owned"),
        "rviimpl-ret: the element of a released temporary must not be bound again"
    );
    let tail = after("rviimpl-ret", &emitted, "cl_impl_rvi_ret_vlen(");
    assert_contains(
        "rviimpl-ret",
        tail,
        "if (__cl_tmp0[0]) { FreeString(__cl_tmp0[0]); }",
    );
    assert_contains(
        "rviimpl-ret",
        tail,
        "if (__cl_tmp0[1]) { FreeString(__cl_tmp0[1]); }",
    );
}

/// A `take` call result travels to a binding, so its temporary is not released
/// at the statement's end.
#[test]
fn should_not_release_a_take_call_result_temporary() {
    let source = r#"module impl.rvi.taketemp;
import { String, stringOf } from std.text;
func sink(take s: String): String { return s; }
func main(): i32 {
    let s: String = stringOf("x");
    let r: String = sink(s);
    return i32(len(r));
}
"#;
    let emitted = emit_host("rviimpl-take", source);
    assert_contains("rviimpl-take", &emitted, "__cl_tmp0 = cl_impl_rvi_taketemp_sink(s);");
    let tail = after("rviimpl-take", &emitted, "r = __cl_tmp0;");
    assert!(
        !tail.contains("__cl_tmp0"),
        "rviimpl-take: the binding owns the take result, so its temporary must not be released"
    );
}

/// An aggregate-literal adoption temporary moves its value into the literal,
/// so it is not released at the statement's end either.
#[test]
fn should_not_release_a_literal_adoption_temporary() {
    let source = r#"module impl.rvi.literal;
import { String, stringOf } from std.text;
func main(): i32 {
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: String[2] = [a, b];
    return i32(len(c[0]));
}
"#;
    let emitted = emit_host("rviimpl-literal", source);
    assert_contains("rviimpl-literal", &emitted, "__cl_tmp0 = a;");
    assert_contains("rviimpl-literal", &emitted, "a = NULL;");
    let tail = after("rviimpl-literal", &emitted, "c[1] = __cl_tmp1;");
    assert!(
        !tail.contains("__cl_tmp0") && !tail.contains("__cl_tmp1"),
        "rviimpl-literal: the literal owns the adopted elements, so their temporaries must not be released"
    );
}

/// A call-rooted element loop reads one materialized iterable and releases it
/// after the loop.
#[test]
fn should_materialize_a_call_rooted_iterable_and_release_it_after_the_loop() {
    let source = r#"module impl.rvi.iterable;
import { String, stringOf } from std.text;
func mk(): String[2] { return [stringOf("ab"), stringOf("cde")]; }
func vlen(s: str): i32 { return i32(len(s)); }
func main(): i32 {
    let total: i32 = 0;
    for (borrow s in mk()) {
        total = total + vlen(s);
    }
    return total;
}
"#;
    let emitted = emit_host("rviimpl-iterable", source);
    // The iterable is stored once into a temporary before the loop.
    let before_loop = after("rviimpl-iterable", &emitted, "cl_impl_rvi_iterable_mk(");
    assert_contains("rviimpl-iterable", before_loop, "__cl_tmp2");
    let tail = after(
        "rviimpl-iterable",
        &emitted,
        "for (; (__cl_tmp0 < 2); __cl_tmp0 = (__cl_tmp0 + 1u))",
    );
    assert_contains("rviimpl-iterable", tail, "if (__cl_tmp2[0]) { FreeString(__cl_tmp2[0]); }");
    assert_contains("rviimpl-iterable", tail, "if (__cl_tmp2[1]) { FreeString(__cl_tmp2[1]); }");
}

/// A fixed-array conditional initializer stores the destination through a
/// branch, and an arm's runtime guard stays inside that branch.
#[test]
fn should_choose_a_branch_and_keep_arm_guards_inside_it_in_the_assignment_position() {
    let source = r#"module impl.rvi.cond;
func main(): i32 {
    let ok: bool = true;
    let i: i32 = 1;
    let rows: i32[2][2] = [[1, 2], [3, 4]];
    let flat: i32[2] = [5, 6];
    let c: i32[2] = ok ? rows[i] : flat;
    return c[0] + c[1];
}
"#;
    let emitted = emit_host("rviimpl-cond", source);
    let branch = after("rviimpl-cond", &emitted, "if (ok) {");
    assert_contains("rviimpl-cond", branch, "cl_trap(\"E11211\", \"index out of range\");");
    assert_contains("rviimpl-cond", branch, "memcpy(&(c), &(rows[i]), sizeof(c));");
    let else_branch = after("rviimpl-cond", branch, "} else {");
    assert_contains("rviimpl-cond", else_branch, "memcpy(&(c), &(flat), sizeof(c));");
}

/// A fixed-array conditional return writes the chosen arm into the hidden
/// result parameter through a branch.
#[test]
fn should_choose_a_branch_in_the_return_position() {
    let source = r#"module impl.rvi.retcond;
func pick(ok: bool): i32[2] {
    let a: i32[2] = [1, 2];
    let b: i32[2] = [3, 4];
    return ok ? a : b;
}
func main(): i32 {
    let r: i32[2] = pick(false);
    return r[0] + r[1];
}
"#;
    let emitted = emit_host("rviimpl-retcond", source);
    let branch = after("rviimpl-retcond", &emitted, "if (ok) {");
    assert_contains("rviimpl-retcond", branch, "__cl_ret[0] = a[0];");
    assert_contains("rviimpl-retcond", branch, "__cl_ret[1] = a[1];");
    let else_branch = after("rviimpl-retcond", branch, "} else {");
    assert_contains("rviimpl-retcond", else_branch, "__cl_ret[0] = b[0];");
    assert_contains("rviimpl-retcond", else_branch, "__cl_ret[1] = b[1];");
}

/// A conditional that reassigns an owned array saves the old target, moves the
/// chosen arm in, and releases the saved old value once after the branch.
#[test]
fn should_save_and_release_the_old_target_in_a_conditional_reassignment() {
    let source = r#"module impl.rvi.condreassign;
import { String, stringOf } from std.text;
func vlen(s: str): i32 { return i32(len(s)); }
func main(): i32 {
    let ok: bool = false;
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    let t: String[2] = [stringOf("fg"), stringOf("hij")];
    s = ok ? s : t;
    return vlen(s[1]);
}
"#;
    let emitted = emit_host("rviimpl-condreassign", source);
    assert_contains(
        "rviimpl-condreassign",
        &emitted,
        "memcpy(&(__cl_save_1), &(s), sizeof(s));",
    );
    // An arm that names the target restores the saved value and empties the
    // save, so the later release is a no-op.
    let then_arm = after("rviimpl-condreassign", &emitted, "if (ok) {");
    assert_contains(
        "rviimpl-condreassign",
        then_arm,
        "memcpy(&(s), &(__cl_save_1), sizeof(s));",
    );
    assert_contains("rviimpl-condreassign", then_arm, "cl_zero(&__cl_save_1);");
    // The other arm moves the chosen source out.
    let else_arm = after("rviimpl-condreassign", then_arm, "} else {");
    assert_contains("rviimpl-condreassign", else_arm, "memcpy(&(s), &(t), sizeof(s));");
    assert_contains("rviimpl-condreassign", else_arm, "cl_zero(&t);");
    // The saved old value is released once after the branch.
    let after_branch = after("rviimpl-condreassign", else_arm, "}\n");
    assert_contains(
        "rviimpl-condreassign",
        after_branch,
        "if (__cl_save_1[0]) { FreeString(__cl_save_1[0]); }",
    );
}

/// A statement temporary is released on every path that leaves its statement,
/// not only on the first return edge.
#[test]
fn should_release_a_statement_temporary_on_every_path() {
    let source = r#"module impl.rvi.paths;
import { String, stringOf } from std.text;
func mk(): String[2] { return [stringOf("ab"), stringOf("cde")]; }
func vlen(s: str): i32 { return i32(len(s)); }
func main(): i32 {
    if (vlen(mk()[0]) == 99) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit_host("rviimpl-paths", source);
    let branch = after("rviimpl-paths", &emitted, "if ((cl_impl_rvi_paths_vlen(");
    assert_contains(
        "rviimpl-paths",
        branch,
        "if (__cl_tmp0[0]) { FreeString(__cl_tmp0[0]); }",
    );
    let fallthrough = after("rviimpl-paths", branch, "} else {");
    assert_contains(
        "rviimpl-paths",
        fallthrough,
        "if (__cl_tmp0[0]) { FreeString(__cl_tmp0[0]); }",
    );
}

/// A loop header temporary releases its previous value before the loop stores
/// the next one.
#[test]
fn should_release_a_loop_header_temporary_before_rematerializing_it() {
    let source = r#"module impl.rvi.loophead;
import { String, stringOf } from std.text;
func mk(): String[2] { return [stringOf("ab"), stringOf("cde")]; }
func vlen(s: str): i32 { return i32(len(s)); }
func main(): i32 {
    let i: i32 = 0;
    while (vlen(mk()[0]) == 2 && i < 3) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("rviimpl-loophead", source);
    let head = after("rviimpl-loophead", &emitted, "while (1) {");
    let drop = head
        .find("if (__cl_tmp0[0]) { FreeString(__cl_tmp0[0]); }")
        .unwrap_or_else(|| panic!("rviimpl-loophead: the loop head does not release the old temporary"));
    let store = head
        .find("cl_impl_rvi_loophead_mk(__cl_tmp0);")
        .unwrap_or_else(|| panic!("rviimpl-loophead: the loop head does not re-materialize the temporary"));
    assert!(
        drop < store,
        "rviimpl-loophead: the old value must be released before the new one is stored"
    );
}

/// A runtime receiver index is bounds-guarded before the statement call.
#[test]
fn should_guard_a_field_receiver_in_the_statement_call() {
    let source = r#"module impl.rvi.receiver;
struct Ops { run: func(i32): i32; }
func bump(x: i32): i32 { return x + 1; }
func main(): i32 {
    let ops: Ops[2] = [Ops { run: bump }, Ops { run: bump }];
    let i: i32 = 5;
    ops[i].run(1);
    return 0;
}
"#;
    let emitted = emit_host("rviimpl-receiver", source);
    let trap = emitted
        .find("if ((i < 0) || ((usize)(i) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");")
        .unwrap_or_else(|| panic!("rviimpl-receiver: no receiver bounds trap"));
    let call = emitted
        .find("(ops[i].run)(1);")
        .unwrap_or_else(|| panic!("rviimpl-receiver: no field call"));
    assert!(
        trap < call,
        "rviimpl-receiver: the receiver trap must run before the call"
    );
}

/// A runtime receiver index is bounds-guarded before a fixed-array call
/// assignment.
#[test]
fn should_guard_a_field_receiver_in_the_fixed_array_assign() {
    let source = r#"module impl.rvi.recvassign;
struct Maker { make: func(i32): i32[2]; }
func pair(x: i32): i32[2] { return [x, x + 1]; }
func main(): i32 {
    let ms: Maker[2] = [Maker { make: pair }, Maker { make: pair }];
    let i: i32 = 5;
    let a: i32[2] = ms[i].make(3);
    return a[1];
}
"#;
    let emitted = emit_host("rviimpl-recvassign", source);
    let trap = emitted
        .find("if ((i < 0) || ((usize)(i) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");")
        .unwrap_or_else(|| panic!("rviimpl-recvassign: no receiver bounds trap"));
    let call = emitted
        .find("(ms[i].make)(3, __cl_call_1);")
        .unwrap_or_else(|| panic!("rviimpl-recvassign: no field call with a destination"));
    assert!(
        trap < call,
        "rviimpl-recvassign: the receiver trap must run before the call"
    );
}

/// A runtime receiver index is bounds-guarded before a fixed-array return
/// call, which writes through the hidden destination.
#[test]
fn should_guard_a_field_receiver_in_the_fixed_array_return() {
    let source = r#"module impl.rvi.recvreturn;
struct Maker { make: func(i32): i32[2]; }
func pair(x: i32): i32[2] { return [x, x + 1]; }
func pick(ms: Maker[2], i: i32): i32[2] {
    return ms[i].make(3);
}
func main(): i32 {
    let ms: Maker[2] = [Maker { make: pair }, Maker { make: pair }];
    let r: i32[2] = pick(ms, 5);
    return r[0];
}
"#;
    let emitted = emit_host("rviimpl-recvreturn", source);
    let trap = emitted
        .find("if ((i < 0) || ((usize)(i) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");")
        .unwrap_or_else(|| panic!("rviimpl-recvreturn: no receiver bounds trap"));
    let call = emitted
        .find("(ms[i].make)(3, __cl_ret);")
        .unwrap_or_else(|| panic!("rviimpl-recvreturn: no returned field call with a destination"));
    assert!(
        trap < call,
        "rviimpl-recvreturn: the receiver trap must run before the call"
    );
}

/// A func value with a fixed-array return declares the hidden destination and
/// a scalar-returning func value stays unchanged.
#[test]
fn should_render_the_hidden_destination_in_a_func_declarator() {
    let source = r#"module impl.rvi.funcptr;
struct Ops { run: func(i32): i32; }
struct Maker { make: func(i32): i32[2]; }
func bump(x: i32): i32 { return x + 1; }
func pair(x: i32): i32[2] { return [x, x + 1]; }
func main(): i32 {
    let g: func(i32): i32[2] = pair;
    let ops: Ops = Ops { run: bump };
    let m: Maker = Maker { make: pair };
    let a: i32[2] = g(3);
    return a[0] + ops.run(1) + m.make(3)[0];
}
"#;
    let emitted = emit_host("rviimpl-funcptr", source);
    assert_contains("rviimpl-funcptr", &emitted, "void (*make)(i32, i32 [2]);");
    assert_contains(
        "rviimpl-funcptr",
        &emitted,
        "void (*g)(i32, i32 [2]) = NULL;",
    );
    assert_contains("rviimpl-funcptr", &emitted, "i32 (*run)(i32);");
}

/// A call through a func value passes the hidden destination, and a discarded
/// call writes into a released discard array.
#[test]
fn should_pass_a_destination_to_a_func_value_call() {
    let source = r#"module impl.rvi.funccall;
func pair(x: i32): i32[2] { return [x, x + 1]; }
func main(): i32 {
    let g: func(i32): i32[2] = pair;
    let a: i32[2] = g(3);
    g(3);
    return a[0] + a[1];
}
"#;
    let emitted = emit_host("rviimpl-funccall", source);
    assert_contains("rviimpl-funccall", &emitted, "g(3, __cl_call_1);");
    let discard = after("rviimpl-funccall", &emitted, "i32 __cl_discard_1[2] = {0};");
    assert_contains("rviimpl-funccall", discard, "g(3, __cl_discard_1);");
}
