//! Emitted-shape and internal-boundary tests for the take, move-out, and
//! nullable borrow out-slot fixes.
//!
//! The specification suite states the observable behavior; these cases pin the
//! translation-unit text the behavior depends on: the deferred guard bind for a
//! guarded member take, the root call and selected index slot of an aggregate
//! field, the slice element move-out before the row release, the non-zero slice
//! start, and the rejected concrete element-move limit. Each case emits a small
//! program with the built compiler and inspects the emitted C.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one program may run before the case fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

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
            "clc-dbcimpl-{}-{tag}-{sequence}",
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
/// Returns the emitted translation unit. These cases are about the text the
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

/// Compiles a program and returns the compiler output and the C path.
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

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_program(tag, &program, &[])
}

/// Emits and runs one host program without the strict warning set, built with
/// AddressSanitizer when `RDC_ASAN` is set.
///
/// A shape with a known pre-existing warning still needs its memory behavior
/// pinned, so the case builds without `-Werror`. The sanitizer is opt-in
/// because the in-container full-suite sanitizer run is timing-flaky in this
/// environment and is not a gate; host sanitizer probes are the deterministic
/// memory oracle. The ordinary build still aborts in the C library on a double
/// release, so a regression fails the exit-code assertion either way. Returns
/// the program output.
fn run_memory_relaxed(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut build = Command::new("cc");
    build.args(["-std=c99", "-O0", "-g"]);
    if std::env::var_os("RDC_ASAN").is_some() {
        build.arg("-fsanitize=address");
    }
    let built = build
        .args([
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the relaxed build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    if std::env::var_os("RDC_ASAN").is_some() {
        run_program(tag, &program, &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")])
    } else {
        run_program(tag, &program, &[])
    }
}

/// Runs a built program with a time limit and the given environment values.
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_program(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{tag}: the program did not finish within {RUN_TIMEOUT:?}");
            }
            Err(err) => panic!("{tag}: cannot wait for the program: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program output: {err}"))
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

/// Asserts the front end rejects the program with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
    assert_rejected_with(tag, source, code, &[]);
}

/// Asserts the front end rejects the program with `code`, that the message
/// contains every fragment, and that no translation unit is written.
fn assert_rejected_with(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: the diagnostic does not contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// A guarded member take binds the call root inside its guard.
///
/// The root call runs only on the evaluated guard path, so the bind must sit
/// inside the `&&` right operand: the temporary receives the call result, the
/// callee reads the selected member, and the member is emptied on that path.
/// No eagerly assigned owned local may appear.
#[test]
fn should_bind_the_root_call_when_a_guarded_member_take_selects_it() {
    let source = r#"module impl.closeout.guardt;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let flag: bool = false;
    let hit: bool = flag && sink(mkv().a) == 1;
    return hit ? 1 : 0;
}
"#;
    let emitted = emit_host("guard-bind", source);
    assert_before(
        "guard-bind",
        &emitted,
        "(flag && (",
        "__cl_tmp0 = cl_impl_closeout_guardt_mkv()",
    );
    assert_before(
        "guard-bind",
        &emitted,
        "__cl_tmp0 = cl_impl_closeout_guardt_mkv()",
        "_sink(__cl_tmp0.a)",
    );
    assert_contains(
        "guard-bind",
        &emitted,
        "_sink(__cl_tmp0.a)), (__cl_tmp0.a = NULL)",
    );
    assert_absent("guard-bind", &emitted, "__cl_owned0 = ");
}

/// An aggregate field over a call result pins the index and empties its slot.
///
/// The root call is emitted before the field read, the run-time index is stored
/// once, and the selected slot is emptied after the value moves into the field.
#[test]
fn should_pin_the_index_and_empty_the_selected_slot_when_an_aggregate_field_reads_a_call_result() {
    let source = r#"module impl.closeout.indexbox;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func pack<T>(make: func(): T[2], i: i32): Box<T> {
    return Box<T> { v: make()[i] };
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let b: Box<String> = pack<String>(mkString, 1);
    return i32(len(b.v));
}
"#;
    let emitted = emit_host("aggregate-index", source);
    assert_before("aggregate-index", &emitted, "make(", "__cl_tmp0[__cl_tmp1]");
    assert_contains("aggregate-index", &emitted, "__cl_tmp1 = i;");
    assert_before(
        "aggregate-index",
        &emitted,
        "__cl_tmp2 = __cl_tmp0[__cl_tmp1];",
        "__cl_tmp0[__cl_tmp1] = NULL;",
    );
}

/// An aggregate field at a constant index still emits the root call first.
///
/// No index temporary is needed for a constant, but the call must run before
/// the read and the selected slot must be emptied.
#[test]
fn should_empty_slot_zero_when_an_aggregate_field_reads_a_call_result_at_a_constant_index() {
    let source = r#"module impl.closeout.constantbox;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func pack<T>(make: func(): T[2]): Box<T> {
    return Box<T> { v: make()[0] };
}

func mkString(): String[2] {
    return [stringOf("abc"), stringOf("de")];
}

func main(): i32 {
    let b: Box<String> = pack<String>(mkString);
    return i32(len(b.v));
}
"#;
    let emitted = emit_host("constant-index", source);
    assert_before("constant-index", &emitted, "make(", "__cl_tmp0[0]");
    assert_before(
        "constant-index",
        &emitted,
        "make(",
        "__cl_tmp0[0] = NULL;",
    );
}

/// A slice element moves out before the view's rows are released.
///
/// The binding receives the element, the slot the view selected is emptied, and
/// the row releases run afterward; the emitted C must name no unknown-typed
/// symbol.
#[test]
fn should_move_the_slice_element_before_the_row_release_when_a_statement_temporary_is_bound() {
    let source = r#"module impl.closeout.slicestore;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func storeSlice<T>(make: func(): T[2], i: i32): T {
    let s: T = slice(make(), 0, 2)[i];
    return s;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(storeSlice<String>(mkString, 1)));
}
"#;
    let emitted = emit_host("slice-store", source);
    assert_absent("slice-store", &emitted, "Unknown");
    assert_before(
        "slice-store",
        &emitted,
        "s = (__cl_tmp1).ptr[__cl_tmp2];",
        "(__cl_tmp1).ptr[__cl_tmp2] = NULL;",
    );
    assert_contains(
        "slice-store",
        &emitted,
        "if (__cl_tmp0[0]) { FreeString(__cl_tmp0[0]); }",
    );
}

/// A non-zero slice start empties the slot the view selected, not slot zero.
///
/// The view's pointer already carries the start, so the same slot-zero idiom
/// resolves to the second row; the first row is released by the cleanup.
#[test]
fn should_empty_the_viewed_slot_when_the_slice_start_is_non_zero() {
    let source = r#"module impl.closeout.sliceoffset;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func storeSlice<T>(make: func(): T[2], i: i32): T {
    let s: T = slice(make(), 1, 2)[i];
    return s;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(storeSlice<String>(mkString, 0)));
}
"#;
    let emitted = emit_host("slice-offset", source);
    assert_absent("slice-offset", &emitted, "Unknown");
    assert_contains(
        "slice-offset",
        &emitted,
        "(__cl_tmp1).ptr[__cl_tmp2] = NULL;",
    );
    assert_contains("slice-offset", &emitted, "__cl_tmp0[0] = NULL;");
}

/// Concrete element moves through a slice view keep the element-move limit.
///
/// The generic spellings move their element out; a concrete element selected
/// through a slice view is still rejected before any C is written.
#[test]
fn should_reject_a_concrete_slice_element_move_when_the_element_is_bound() {
    let source = r#"module impl.closeout.slicereject;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func storeSlice(make: func(): String[2], i: i32): String {
    let s: String = slice(make(), 0, 2)[i];
    return s;
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected("slice-reject", source, "E10026");
}

/// The runnable unguarded take keeps its release-once behavior.
///
/// The unguarded form binds the call result to an owned local, empties the
/// taken member, and releases the remaining field; the case pins the exit code
/// so a release regression (which aborts in the C library) cannot pass.
#[test]
fn should_release_once_when_an_unguarded_take_reads_a_call_result() {
    let source = r#"module impl.closeout.membertake;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    return sink(mkv().a);
}
"#;
    let output = run_host("member-take", source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "member-take: expected exit 1; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A nested access through a multi-extent slice view must not free caller
/// storage.
///
/// The slice view over a multi-extent source does not carry row typing, so an
/// element access through it stays a borrowed read: it must not move out or
/// release storage the caller still owns, because a double release aborts even
/// without the sanitizer. The emitted access already carries a pre-existing
/// scalar-typing warning, so the case builds without the strict warning set and
/// pins memory safety only.
#[test]
fn should_not_free_caller_storage_when_a_nested_multi_extent_slice_access_runs() {
    let source = r#"module impl.closeout.slicenested;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let xs: String[2][2] = [
        [stringOf("a"), stringOf("bc")],
        [stringOf("def"), stringOf("ghij")]
    ];
    let n: i32 = i32(len(slice(xs, 0, 2)[1][0]));
    return n;
}
"#;
    let output = run_memory_relaxed("slice-nested", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "slice-nested: the program aborted (double free); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("AddressSanitizer"),
        "slice-nested: the sanitizer reported an error; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The plain `len` call result gains an owner and is released once.
///
/// The intrinsic only borrows its argument, so an unnamed string call result
/// has no binding to drop and would leak. The emitted unit must store the call
/// into a named local, read the length through that local, and keep the
/// scope search to exactly one cleanup path. The exit code proves the value
/// survived the read; a release regression aborts in the C library.
#[test]
fn should_bind_and_release_once_when_len_reads_a_string_call_result() {
    let source = r#"module impl.closeout.exprlen;

import { String, stringOf } from std.text;

func main(): i32 {
    return i32(len(stringOf("abc")));
}
"#;
    let emitted = emit_host("expr-len", source);
    assert_contains(
        "expr-len",
        &emitted,
        "__cl_owned0 = cl_std_text_stringOf",
    );
    assert_contains("expr-len", &emitted, "(cl_string_view(__cl_owned0)).len");
    assert_absent(
        "expr-len",
        &emitted,
        "(cl_string_view(cl_std_text_stringOf",
    );
    let output = run_host("expr-len-run", source);
    assert_eq!(
        output.status.code(),
        Some(3),
        "expr-len-run: expected exit 3; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The dot-`len` receiver on a call result gains the same owner.
///
/// `xs.len()` is the receiver-taking spelling of the intrinsic, so its
/// receiver must travel through the same binding; otherwise the string
/// returned by the call leaks. The case pins the binding and the read through
/// it, and the pinned exit code proves the receiver is alive for the read.
#[test]
fn should_bind_the_receiver_when_the_dot_len_form_reads_a_call_result() {
    let source = r#"module impl.closeout.exprdot;

import { String, stringOf } from std.text;

func main(): i32 {
    return i32(stringOf("a").len());
}
"#;
    let emitted = emit_host("expr-dot", source);
    assert_contains(
        "expr-dot",
        &emitted,
        "__cl_owned0 = cl_std_text_stringOf",
    );
    assert_contains("expr-dot", &emitted, "(cl_string_view(__cl_owned0)).len");
    assert_absent(
        "expr-dot",
        &emitted,
        "(cl_string_view(cl_std_text_stringOf",
    );
    let output = run_host("expr-dot-run", source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "expr-dot-run: expected exit 1; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A guarded `len` argument is filled inside the guard and released once.
///
/// The right side of `&&` runs only on the taken path, so the argument's
/// binding cannot be a statement before the branch: the emitted condition must
/// fill the materialized temporary in the same expression that measures it,
/// and the statement's cleanup releases whatever the taken path filled. The
/// case pins the fill-inside-the-condition shape and the pinned exit code.
#[test]
fn should_defer_the_fill_to_the_guard_when_a_guarded_len_call_reads_a_call_result() {
    let source = r#"module impl.closeout.exprguard;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func make(): String {
    calls = calls + 1;
    return stringOf("abc");
}

func guarded(flag: bool): i32 {
    let hit: bool = flag && len(make()) == 3;
    if (hit && calls == 1) {
        return 3;
    }
    if (!hit && calls == 0) {
        return 5;
    }
    return 99;
}

func main(): i32 {
    return guarded(true);
}
"#;
    let emitted = emit_host("expr-guard", source);
    assert_contains(
        "expr-guard",
        &emitted,
        "__cl_tmp0 = cl_impl_closeout_exprguard_make()",
    );
    assert_contains(
        "expr-guard",
        &emitted,
        "(cl_string_view(__cl_tmp0)).len == 3",
    );
    assert_absent(
        "expr-guard",
        &emitted,
        "(cl_string_view(cl_impl_closeout_exprguard_make())",
    );
    let output = run_host("expr-guard-run", source);
    assert_eq!(
        output.status.code(),
        Some(3),
        "expr-guard-run: expected exit 3; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A duplicable argument needs no owner and stays inline.
///
/// A fixed-array literal and a static string own nothing to release, so the
/// argument stays inline and no owned local appears. An emitted local that is
/// never read would also be rejected by the strict warning set, so the case
/// pins both the absence of the binding and a successful strict build.
#[test]
fn should_leave_a_duplicable_argument_inline_when_len_reads_it() {
    let source = r#"module impl.closeout.exprdup;

func main(): i32 {
    return i32(len([1, 2])) + i32(len("hi"));
}
"#;
    let emitted = emit_host("expr-dup", source);
    assert_absent("expr-dup", &emitted, "__cl_owned");
    let output = run_host("expr-dup-run", source);
    assert_eq!(
        output.status.code(),
        Some(4),
        "expr-dup-run: expected exit 4; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A may-lend call result is not bound: its argument's owner releases it.
///
/// A generic callee that returns its lend parameter hands back storage the
/// caller already owns through the argument. Binding that result into a second
/// owned local would release the same storage twice, so the result stays an
/// inline read of the argument's binding. The exit code proves the alias is
/// intact and the storage is released once.
#[test]
fn should_not_bind_a_may_lend_result_when_len_reads_a_reborrow_call() {
    let source = r#"module impl.closeout.exprlend;

import { String, stringOf } from std.text;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func outer<T>(x: T): T {
    return apply(x, [x] (y: T) => y);
}

func main(): i32 {
    return i32(len(outer<String>(stringOf("abc"))));
}
"#;
    let emitted = emit_host("expr-lend", source);
    assert_absent("expr-lend", &emitted, "__cl_owned1");
    assert_contains(
        "expr-lend",
        &emitted,
        "(cl_string_view(cl_outer__String(__cl_owned0))).len",
    );
    let output = run_host("expr-lend-run", source);
    assert_eq!(
        output.status.code(),
        Some(3),
        "expr-lend-run: expected exit 3; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A conditional argument with a place-rooted arm stays an inline read.
///
/// Binding such a conditional would adopt and empty the place arm, and the
/// borrow argument needs only a read. The case pins the absence of the binding
/// and the pinned answer on the taken path, where the other arm is a may-lend
/// call that must not be bound either.
#[test]
fn should_not_bind_a_conditional_argument_when_an_arm_roots_in_a_place() {
    let source = r#"module impl.closeout.exprcond;

import { String, stringOf } from std.text;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func outer<T>(x: T): T {
    return apply(x, [x] (y: T) => y);
}

func main(): i32 {
    let t: String = stringOf("abcd");
    return i32(len(true ? t : outer<String>(t)));
}
"#;
    let emitted = emit_host("expr-cond", source);
    assert_absent("expr-cond", &emitted, "__cl_owned");
    let output = run_host("expr-cond-run", source);
    assert_eq!(
        output.status.code(),
        Some(4),
        "expr-cond-run: expected exit 4; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A take-call result measured across loop iterations is released once.
///
/// The take call materializes its result in a statement temporary that already
/// carries a release, so the intrinsic argument must read that temporary
/// instead of copying it into a second owner; a second owner frees the same
/// bytes when the loop statement runs again. The case pins the absence of the
/// alias assignment and the two-iteration answer.
#[test]
fn should_release_the_take_result_once_when_len_reads_it_across_loop_iterations() {
    let source = r#"module impl.closeout.exprloop;

import { String, stringOf } from std.text;

func sink(take s: String): String {
    return s;
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        n = n + i32(len(sink(stringOf("ab"))));
        i = i + 1;
    }
    return n;
}
"#;
    let emitted = emit_host("expr-loop", source);
    assert_absent("expr-loop", &emitted, " = __cl_tmp");
    let output = run_host("expr-loop-run", source);
    assert_eq!(
        output.status.code(),
        Some(4),
        "expr-loop-run: expected exit 4; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A nested conditional whose place arm is one level deep stays inline.
///
/// The chosen arm of a conditional may itself be a conditional; when any
/// reachable arm roots in a place, adopting the outer value would empty that
/// place and the later read of it would run on freed storage. The case pins
/// the absence of the binding on a shape whose place sits one level down, and
/// the answer proves the local string is still readable after the measurement.
#[test]
fn should_not_bind_a_nested_conditional_argument_when_a_deep_arm_roots_in_a_place() {
    let source = r#"module impl.closeout.exprcond2;

import { String, stringOf } from std.text;

func mk1(): String {
    return stringOf("x");
}

func mk2(): String {
    return stringOf("yz");
}

func measure(a: bool, b: bool, s: String): i32 {
    return i32(len(a ? mk1() : (b ? s : mk2())));
}

func main(): i32 {
    let s: String = stringOf("abcd");
    let n: i32 = measure(false, true, s);
    return n * 10 + i32(len(s));
}
"#;
    let emitted = emit_host("expr-cond2", source);
    assert_absent("expr-cond2", &emitted, "__cl_owned");
    let output = run_host("expr-cond2-run", source);
    assert_eq!(
        output.status.code(),
        Some(44),
        "expr-cond2-run: expected exit 44; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A nullable borrow out-slot lowers per kind, and the call passes its address.
///
/// A `String?` slot is `ClString *`, a `File?` slot `BPTR *`, a `str?` slot
/// `ClStr *`, and a slice `T[]?` slot `ClSlice_i32 *`; each call takes the
/// address of the caller's exact place. The value parameter keeps its own
/// spelling, so the fix changes only the out-slot.
#[test]
fn should_spell_nullable_borrow_out_slots_as_pointers_when_the_kinds_are_passed() {
    let source = r#"module impl.closeout.outslot;

import { String, stringOf } from std.text;
import { File, openFile, MODE_OLDFILE } from std.io;
import { slice } from std.mem;

func fillString(take v: String, borrow slot: String?): void {
    slot = v;
}

func fillStr(v: str, borrow slot: str?): void {
    slot = v;
}

func fillFile(take v: File, borrow slot: File?): void {
    slot = v;
}

func fillSlice(v: i32[], borrow slot: i32[]?): void {
    slot = v;
}

func main(): i32 {
    let owned: String? = null;
    fillString(stringOf("xyz"), borrow owned);
    let view: str? = null;
    fillStr("abc", borrow view);
    let a: i32[2] = [1, 2];
    let xs: i32[]? = slice(a, 0, 2);
    fillSlice(slice(a, 0, 2), borrow xs);
    let f: File? = openFile("/dev/null", MODE_OLDFILE);
    if (f == null) {
        return 8;
    }
    let g: File? = openFile("/dev/null", MODE_OLDFILE);
    if (g == null) {
        return 9;
    }
    fillFile(g, borrow f);
    return 0;
}
"#;
    let emitted = emit_host("outslot-shapes", source);
    assert_contains("outslot-shapes", &emitted, "(ClString v, ClString *slot)");
    assert_contains("outslot-shapes", &emitted, "(ClStr v, ClStr *slot)");
    assert_contains("outslot-shapes", &emitted, "(BPTR v, BPTR *slot)");
    assert_contains(
        "outslot-shapes",
        &emitted,
        "(ClSlice_i32 v, ClSlice_i32 *slot)",
    );
    assert_contains("outslot-shapes", &emitted, ", &(owned))");
    assert_contains("outslot-shapes", &emitted, "fillStr(cl_str(\"abc\"), &(view))");
    assert_contains("outslot-shapes", &emitted, ", &(xs))");
    assert_contains("outslot-shapes", &emitted, ", &(f))");
    // The null initializer of the `str?` local is the zero view, not a null
    // pointer, and the null test reads the view's pointer member.
    assert_contains("outslot-shapes", &emitted, "view = cl_zero_ClStr();");
    assert_absent("outslot-shapes", &emitted, "(ClString v, ClString slot)");
}

/// A non-nullable duplicable struct lend stays by value.
///
/// The deliberate by-value deviation for a duplicable struct is unchanged by
/// the nullable out-slot fix: the parameter is `ClPoint`, the call passes the
/// value without an address, and no pointer spelling appears. The declaration
/// warns that the lend is unnecessary, which is the documented behavior.
#[test]
fn should_keep_a_duplicable_struct_lend_by_value_when_the_borrow_is_not_nullable() {
    let source = r#"module impl.closeout.structlend;

struct Point {
    x: i32;
    y: i32;
}

func bump(borrow p: Point): void {
    p.x = p.x + 1;
}

func main(): i32 {
    let p: Point = Point { x: 1, y: 2 };
    bump(p);
    return p.x;
}
"#;
    let emitted = emit_host("struct-lend", source);
    assert_contains("struct-lend", &emitted, "bump(ClPoint p)");
    assert_contains("struct-lend", &emitted, "bump(p);");
    assert_absent("struct-lend", &emitted, "ClPoint *p");
}

/// A nullable view initializes to its zero struct and narrows through `ptr`.
///
/// A view is an emitted pointer-and-length struct, so `null` becomes the zero
/// view and `== null` tests the view's pointer member; assigning or comparing
/// the struct with `NULL` would not build as C.
#[test]
fn should_use_the_zero_view_and_the_pointer_test_when_a_nullable_view_is_narrowed() {
    let source = r#"module impl.closeout.viewnull;

func main(): i32 {
    let view: str? = null;
    if (view == null) {
        return 9;
    }
    return i32(len(view));
}
"#;
    let emitted = emit_host("view-null", source);
    assert_contains("view-null", &emitted, "view = cl_zero_ClStr();");
    assert_contains("view-null", &emitted, "((view).ptr == NULL)");
    assert_absent("view-null", &emitted, "view = NULL;");
    assert_absent("view-null", &emitted, "(ClStr){0}");
}

/// An out-slot argument that is not an exact-typed place is rejected.
///
/// The slot lowers to a pointer into the caller's storage, so a widened place
/// (`String` into `str?` or `String?`) or a non-place exact-typed expression
/// (a call result) cannot carry the callee's write back and is rejected with
/// `E10200`; the non-place case names the exact-place rule in its help.
#[test]
fn should_reject_when_a_widened_or_non_place_argument_reaches_a_nullable_out_slot() {
    let widened_str = r#"module impl.closeout.outslotwide1;

import { String, stringOf } from std.text;

func fill(borrow slot: str?): void {
    slot = null;
}

func main(): i32 {
    let owned: String = stringOf("abc");
    fill(borrow owned);
    return 0;
}
"#;
    let widened_string = r#"module impl.closeout.outslotwide2;

import { String, stringOf } from std.text;

func fill(borrow slot: String?): void {
    slot = null;
}

func main(): i32 {
    let owned: String = stringOf("abc");
    fill(borrow owned);
    return 0;
}
"#;
    let non_place = r#"module impl.closeout.outslotnonplace;

import { File, openFile, MODE_OLDFILE } from std.io;

func fill(borrow slot: File?): void {
    slot = null;
}

func main(): i32 {
    fill(borrow openFile("/dev/null", MODE_OLDFILE));
    return 0;
}
"#;
    assert_rejected("outslot-wide-str", widened_str, "E10200");
    assert_rejected("outslot-wide-string", widened_string, "E10200");
    let (_scratch, output, c_path) = compile_only("outslot-nonplace", non_place);
    assert_ne!(
        output.status.code(),
        Some(0),
        "outslot-nonplace: the front end accepted a call result as an out-slot"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error E10200:"),
        "outslot-nonplace: expected E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("pass a place of the exact slot type"),
        "outslot-nonplace: expected the exact-place help; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "outslot-nonplace: a rejected program must not produce a translation unit"
    );
}

/// An explicit instance that makes a `borrow T` parameter nullable passes the
/// address of the caller's slot.
///
/// The instance substitution fixes the parameter type, so the callee signature
/// is `ClString *slot` and the call must take the slot's address; the write
/// reaches the caller's storage and the run answers three.
#[test]
fn should_pass_the_address_when_an_explicit_instance_makes_the_slot_nullable() {
    let source = r#"module impl.closeout.instslot;

import { String, stringOf } from std.text;

func fill<T>(take v: T, borrow slot: T): void {
    slot = v;
}

func main(): i32 {
    let slot: String? = null;
    fill<String?>(stringOf("xyz"), borrow slot);
    if (slot == null) {
        return 9;
    }
    return i32(len(slot));
}
"#;
    let emitted = emit_host("inst-slot", source);
    assert_contains("inst-slot", &emitted, "(ClString v, ClString *slot)");
    assert_contains("inst-slot", &emitted, ", &(slot))");
    let output = run_host("inst-slot-run", source);
    assert_eq!(
        output.status.code(),
        Some(3),
        "inst-slot-run: expected exit 3; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An `ffi` borrow parameter stays by value even when it names a slot.
///
/// The FFI prototype ignores modes and nullability, so the emitter must not
/// take the address of the argument: the extern sees the declared ABI type by
/// value. A non-duplicable struct argument would otherwise fail the strict C
/// build, and a nullable handle would silently pass a stack address.
#[test]
fn should_keep_ffi_arguments_by_value_when_the_parameter_is_a_borrow_slot() {
    let source = r#"module impl.closeout.ffislot;

import { String, stringOf } from std.text;

struct Entity {
    name: String;
}

ffi lib {
    func use(borrow e: Entity): void;
    func rawSlot(borrow slot: String?): void;
}

func main(): i32 {
    let e: Entity = Entity { name: stringOf("x") };
    use(e);
    let slot: String? = null;
    rawSlot(borrow slot);
    return 0;
}
"#;
    let emitted = emit_host("ffi-slot", source);
    assert_contains("ffi-slot", &emitted, "void use(ClEntity);");
    assert_contains("ffi-slot", &emitted, "use(e);");
    assert_absent("ffi-slot", &emitted, "&(e)");
    assert_contains("ffi-slot", &emitted, "rawSlot(slot);");
    assert_absent("ffi-slot", &emitted, "&(slot)");
}

/// A null view reaches call arguments, conditional arms, and global
/// initializers through the emitted zero-view helper.
///
/// A view is a struct, so `NULL` does not type-check; the helper keeps the
/// emitted C inside the C99 subset in every expression position. The program
/// measures the argument and the conditional, so the run answers four.
#[test]
fn should_use_the_zero_view_helper_when_a_null_view_reaches_arguments_and_conditionals() {
    let source = r#"module impl.closeout.nullviews;

let g: str? = null;

func check(s: str?): i32 {
    if (s == null) {
        return 1;
    }
    return i32(len(s));
}

func main(): i32 {
    let flag: bool = false;
    let one: i32 = check(null);
    let view: str? = flag ? null : "abc";
    if (view == null) {
        return 7;
    }
    return one + i32(len(view));
}
"#;
    let emitted = emit_host("null-views", source);
    assert_contains("null-views", &emitted, "g = cl_zero_ClStr();");
    assert_contains("null-views", &emitted, "check(cl_zero_ClStr())");
    assert_contains(
        "null-views",
        &emitted,
        "(flag ? cl_zero_ClStr() : cl_str(\"abc\"))",
    );
    assert_absent("null-views", &emitted, "(ClStr){0}");
    let output = run_host("null-views-run", source);
    assert_eq!(
        output.status.code(),
        Some(4),
        "null-views-run: expected exit 4; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Two instances that differ only by type-argument nullability get distinct
/// symbols.
///
/// `pick<String>` and `pick<String?>` are distinct instances, so the instance
/// key must distinguish nullability; a collision would emit the same C
/// definition twice.
#[test]
fn should_key_instance_symbols_by_nullability_when_only_type_argument_nullability_differs() {
    let source = r#"module impl.closeout.instkey;

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
    let emitted = emit_host("inst-key", source);
    assert_contains("inst-key", &emitted, "cl_pick__String(");
    assert_contains("inst-key", &emitted, "cl_pick__StringQ(");
    let output = run_host("inst-key-run", source);
    assert_eq!(
        output.status.code(),
        Some(4),
        "inst-key-run: expected exit 4; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A nested null conditional arm renders the zero view.
///
/// The nested arm's null literal is typed by the outer result, so the emitted
/// expression uses `cl_zero_ClStr()` rather than a null pointer that would not
/// build against the view struct.
#[test]
fn should_render_the_zero_view_when_a_nested_conditional_arm_is_null() {
    let source = r#"module impl.closeout.nestnull;

func main(): i32 {
    let a: bool = true;
    let b: bool = false;
    let view: str? = a ? (b ? null : "abc") : "z";
    if (view == null) {
        return 7;
    }
    return i32(len(view));
}
"#;
    let emitted = emit_host("nest-null", source);
    assert_contains("nest-null", &emitted, "cl_zero_ClStr()");
    assert_absent("nest-null", &emitted, "? NULL");
    let output = run_host("nest-null-run", source);
    assert_eq!(
        output.status.code(),
        Some(3),
        "nest-null-run: expected exit 3; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A deferred return releases the condition's materialized temporary.
///
/// The do-while condition materializes an array after each completed pass. The
/// body's deferred return on the second pass leaves the loop without testing
/// the condition again, so the previous pass's array is still owned and must
/// be released on that return. The emitted release sits between the deferred
/// return's assignment and its return.
#[test]
fn should_release_the_condition_temporary_when_a_deferred_return_leaves_the_loop() {
    let source = r#"module impl.closeout.deferrelease;

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
        defer {
            if (i == 2) {
                return i;
            }
        }
    } while (grow().count > 0 && i < 2);
    return 0;
}
"#;
    let emitted = emit_host("defer-release", source);
    assert_contains(
        "defer-release",
        &emitted,
        "    __cl_ret = i;\n    if (__cl_tmp0.data) { FreeBuffer(__cl_tmp0.data); }\n    __cl_tmp0.data = NULL;\n    return __cl_ret;",
    );
    let output = run_host("defer-release-run", source);
    assert_eq!(
        output.status.code(),
        Some(2),
        "defer-release-run: expected exit 2; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A return after the loop gets no appended condition release.
///
/// The walk that sends the condition's releases to the body's returns,
/// including a deferred return that never fires, must stay inside the loop.
/// The post-loop return keeps exactly the exit path's release before it; no
/// second release is appended to the return itself.
#[test]
fn should_not_append_a_condition_release_to_a_return_after_the_loop() {
    let source = r#"module impl.closeout.deferoutside;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func drain(): i32 {
    let i: i32 = 5;
    do {
        i = i + 1;
        defer {
            if (i == 99) {
                return i;
            }
        }
    } while (grow().count > 0 && i < 5);
    return i * 10;
}

func main(): i32 {
    return drain();
}
"#;
    let emitted = emit_host("defer-outside", source);
    let cleaned: String = emitted
        .lines()
        .filter(|line| !line.trim_start().starts_with("#line"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_contains(
        "defer-outside",
        &cleaned,
        "    __cl_tmp0.data = NULL;\n    __cl_ret = (i32)((u32)i * (u32)10);\n    return __cl_ret;",
    );
    let output = run_host("defer-outside-run", source);
    assert_eq!(
        output.status.code(),
        Some(60),
        "defer-outside-run: expected exit 60; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A nested lambda that lists the outer capture relays it through the inner
/// environment.
///
/// Each lambda owns its own environment struct. The outer environment is built
/// from the enclosing local, the inner environment is built from the outer
/// environment's member, and the inner body reads its own copy. The program
/// runs and answers four.
#[test]
fn should_relay_an_outer_capture_into_a_nested_environment() {
    let source = r#"module impl.closeout.capturenest;

func main(): i32 {
    let n: i32 = 4;
    return ([n] () => ([n] () => n)())();
}
"#;
    let emitted = emit_host("capture-nest", source);
    assert_contains(
        "capture-nest",
        &emitted,
        "return cl_lambda_0(&(((struct ClEnv_0){ n })));",
    );
    assert_contains(
        "capture-nest",
        &emitted,
        "return cl_lambda_1(&(((struct ClEnv_1){ (__env->n) })));",
    );
    assert_contains("capture-nest", &emitted, "return (__env->n);");
    let output = run_host("capture-nest-run", source);
    assert_eq!(
        output.status.code(),
        Some(4),
        "capture-nest-run: expected exit 4; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A capture-less lambda spells module names as plain module symbols.
///
/// A module constant and a module function sit above every scope floor, so the
/// lambda needs no capture and stays a plain function pointer: it takes the
/// parameter alone, with no environment, and its body calls the sanitized
/// module function and reads the sanitized module constant. The program runs
/// and answers forty-two.
#[test]
fn should_spell_module_names_inside_a_captureless_lambda() {
    let source = r#"module impl.closeout.captureenv;

const base: i32 = 40;

func addTwo(v: i32): i32 {
    return v + 2;
}

func main(): i32 {
    let f: func(i32): i32 = (v: i32) => addTwo(base) + v;
    return f(0);
}
"#;
    let emitted = emit_host("capture-env", source);
    assert_contains("capture-env", &emitted, "static i32 cl_lambda_0(i32 v)");
    assert_contains(
        "capture-env",
        &emitted,
        "return (i32)((u32)cl_impl_closeout_captureenv_addTwo(cl_impl_closeout_captureenv_base) + (u32)v);",
    );
    let output = run_host("capture-env-run", source);
    assert_eq!(
        output.status.code(),
        Some(42),
        "capture-env-run: expected exit 42; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The unlisted-capture rejection names the read and suggests the listing.
///
/// The diagnostic states the offending name in the message and repeats it in
/// the help line, so a reader can paste the suggestion into the capture list.
#[test]
fn should_report_the_unlisted_name_and_its_suggestion_in_the_diagnostic_text() {
    let source = r#"module impl.closeout.capturetext;

func outer(n: i32, x: i32): i32 {
    return ([x] () => n + x)();
}

func main(): i32 {
    return outer(1, 2);
}
"#;
    assert_rejected_with(
        "capture-text",
        source,
        "E10405",
        &[
            "error E10405: lambda reads 'n' without listing it",
            "help: add 'n' to the capture list",
        ],
    );
}

/// A read of a `take` capture reports only the take rejection.
///
/// The capture is listed, so the read is not an unlisted enclosing read: the
/// listing itself is invalid (E10403), and the help must not suggest adding a
/// name that is already there.
#[test]
fn should_report_only_the_take_rejection_when_a_take_capture_is_read() {
    let source = r#"module impl.closeout.capturetake;

import { stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("x");
    ([take s] () => s)();
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("capture-take", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "capture-take: the front end accepted a `take` capture"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error E10403:"),
        "capture-take: expected E10403; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("E10405"),
        "capture-take: a listed take capture must not also report E10405; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "capture-take: a rejected program must not produce a translation unit"
    );
}
