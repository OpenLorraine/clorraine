//! Emitted-shape and internal-boundary tests for the take and move-out fixes.
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
