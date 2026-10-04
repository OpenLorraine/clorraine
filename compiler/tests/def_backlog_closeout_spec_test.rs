//! Runnable specification tests for take and move-out behavior at call and
//! slice boundaries.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. The chosen answer
//! travels in `main`'s return code, so a case that prints nothing states its
//! result as the exit code.
//!
//! The suite covers three shapes:
//!
//! * a `take` argument that reads a member of an aggregate returned by a call,
//!   both guarded (`flag && sink(mkv().a)`) and unguarded;
//! * an aggregate literal whose field reads an indexed element of an
//!   array-returning call (`Box<T> { v: mk()[i] }`), with a run-time index and
//!   a constant index, spelled concretely and under a `String` substitution;
//! * an element read through a `slice(...)` view over a statement temporary,
//!   bound to a local, taken by a callee, and, for concrete element types,
//!   rejected.
//!
//! A case whose program is accepted but whose emitted C the host compiler
//! rejects states the defect directly: the emitted translation must build. A
//! case that runs also pins release-once behavior, because a double release
//! aborts in the C library before the exit code is seen.
//!
//! The unguarded member take and the concrete slice rejections already behave
//! as required, so they act as regression pins. The guarded member take and
//! the aggregate-literal index cases are the expectations that fail until the
//! move-out reaches those shapes.
//!
//! The memory helper runs the same program as an ordinary strict host build
//! and adds leak and use-after-free detection when `RDC_ASAN` is set.

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
            "clc-closeoutspec-{}-{tag}-{sequence}",
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
/// Returns the emitted translation unit. A rejection is a failure of the case,
/// not a result to assert, because the run cases are about runtime behavior.
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

/// Compiles the emitted C with the host compiler under the strict flag set.
///
/// Panics with the compiler's stderr when the build fails. Returns the built
/// program path.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
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
    program
}

/// Runs a built program with a time limit and the given environment values.
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_with_timeout(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
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

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let program = build_host_c(tag, &scratch, &emitted);
    run_with_timeout(tag, &program, &[])
}

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
///
/// A leaked allocation fails the run and a double free aborts, so a clean exit
/// pins release-once for the case.
fn run_asan(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let emitted = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emitted.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-fsanitize=address",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the sanitizer build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(
        tag,
        &program,
        &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")],
    )
}

/// Runs a memory-safety case: under AddressSanitizer when `RDC_ASAN` is set,
/// otherwise as an ordinary strict host build.
///
/// The plain build still aborts in the C library on a double release, so the
/// exit code stays observable; the sanitizer run adds leak and use-after-free
/// detection for the close-out probes.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
}

/// Asserts the program exited with exactly `expected`, reporting its stderr.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit code {expected}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts the run produced no AddressSanitizer report.
///
/// The check is meaningful under `RDC_ASAN`; a plain strict build has no
/// sanitizer output to find. A leak or use-after-free that aborts the run
/// fails the exit-code assertion first.
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
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
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
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

/// Returns the compiler temporary that the run-time index parameter `i` was
/// pinned into.
///
/// The pin keeps the index expression evaluated once, so the element read and
/// the slot emptying share one value. A translation without the pin fails the
/// case with a clear message.
fn pinned_index_temp<'a>(tag: &str, text: &'a str) -> &'a str {
    for line in text.lines() {
        let line = line.trim();
        if let Some((lhs, rhs)) = line.split_once(" = ") {
            if rhs == "i;" && lhs.starts_with("__cl_tmp") {
                return lhs;
            }
        }
    }
    panic!("{tag}: the run-time index is not pinned into a compiler temporary")
}

/// Asserts the emitted aggregate-literal field calls the array-returning
/// function before it empties the slot the run-time index selected.
///
/// The function is passed as a callable parameter named `make`, so `make(`
/// only matches the call site. The pinned index appears exactly once and is
/// the index of the emptied slot.
fn assert_indexed_box_shape(tag: &str, source: &str) {
    let emitted = emit_host(tag, source);
    let index = pinned_index_temp(tag, &emitted);
    assert_eq!(
        count_occurrences(&emitted, &format!("{index} = i;")),
        1,
        "{tag}: the run-time index must be pinned exactly once"
    );
    assert_before(tag, &emitted, "make(", &format!("[{index}] = NULL;"));
}

/// A guarded member take keeps the call result's siblings alive: the guard
/// zeroes the taken member and releases the rest exactly once.
///
/// The true path takes `mkv().a` (two bytes) and answers seven; the false path
/// never evaluates the call and answers zero. The emitted guard must zero
/// `.a` after the take, or the result's cleanup releases the member a second
/// time. Under AddressSanitizer the true path also proves the sibling `b` is
/// released once.
#[test]
fn should_keep_a_guarded_member_take_clean_when_the_guard_is_true_or_false() {
    let source_true = r#"module closeout.guardedmember;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("ab"), b: stringOf("cde") };
}

func sink(take v: String): i32 {
    return i32(len(v));
}

func guarded(flag: bool): i32 {
    let hit: bool = flag && sink(mkv().a) == 2;
    return hit ? 7 : 0;
}

func main(): i32 {
    return guarded(true);
}
"#;
    let emitted = emit_host("guarded-member", source_true);
    assert_contains("guarded-member", &emitted, ".a = NULL;");

    let output_true = run_memory("guarded-member-true", source_true);
    assert_exit("guarded-member-true", &output_true, 7);
    assert_no_asan("guarded-member-true", &output_true);

    let source_false = r#"module closeout.guardedmember;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("ab"), b: stringOf("cde") };
}

func sink(take v: String): i32 {
    return i32(len(v));
}

func guarded(flag: bool): i32 {
    let hit: bool = flag && sink(mkv().a) == 2;
    return hit ? 7 : 0;
}

func main(): i32 {
    return guarded(false);
}
"#;
    let output_false = run_memory("guarded-member-false", source_false);
    assert_exit("guarded-member-false", &output_false, 0);
    assert_no_asan("guarded-member-false", &output_false);
}

/// An unguarded member take transfers the member and releases the result once.
///
/// This is a regression pin: the unguarded form already binds the call result,
/// empties the taken field, and releases the sibling field. The run answers
/// two, and the sanitizer path would report a leak or a second release.
#[test]
fn should_transfer_the_member_when_an_unguarded_take_reads_a_call_result() {
    let source = r#"module closeout.membertake;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("ab"), b: stringOf("cde") };
}

func sink(take v: String): i32 {
    return i32(len(v));
}

func main(): i32 {
    return sink(mkv().a);
}
"#;
    let emitted = emit_host("member-take", source);
    assert_contains("member-take", &emitted, ".a = NULL;");

    let output = run_memory("member-take-run", source);
    assert_exit("member-take-run", &output, 2);
    assert_no_asan("member-take-run", &output);
}

/// An aggregate-literal field reads a call result through a run-time index.
///
/// `pack` fills `Box<T> { v: make()[i] }`; the array-returning call must run
/// before the selected element moves into the field, the index must be read
/// exactly once, and the emptied slot must be the one the index selected. The
/// instantiation measures the second element, so `main` answers three. Both
/// the generic literal spelling and the concrete `Box<String>` spelling must
/// behave the same way.
#[test]
fn should_move_the_indexed_element_when_an_aggregate_field_reads_a_call_result() {
    let generic_source = r#"module closeout.indexbox;

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
    assert_indexed_box_shape("aggregate-index-generic", generic_source);
    let generic_output = run_memory("aggregate-index-generic-run", generic_source);
    assert_exit("aggregate-index-generic-run", &generic_output, 3);
    assert_no_asan("aggregate-index-generic-run", &generic_output);

    let concrete_source = r#"module closeout.indexboxconcrete;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func pack<T>(make: func(): T[2], i: i32): Box<String> {
    return Box<String> { v: make()[i] };
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let b: Box<String> = pack<String>(mkString, 1);
    return i32(len(b.v));
}
"#;
    assert_indexed_box_shape("aggregate-index-concrete", concrete_source);
    let concrete_output = run_memory("aggregate-index-concrete-run", concrete_source);
    assert_exit("aggregate-index-concrete-run", &concrete_output, 3);
    assert_no_asan("aggregate-index-concrete-run", &concrete_output);
}

/// An aggregate-literal field reads a call result at a constant index zero.
///
/// The root call must still be emitted before the move, and the first slot
/// must be emptied before the array temporary is released. The first element
/// measures three bytes, so `main` answers three; the sanitizer path would
/// report a missing root call as a crash or a leak.
#[test]
fn should_move_slot_zero_when_an_aggregate_field_reads_a_call_result_at_a_constant_index() {
    let source = r#"module closeout.constantbox;

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
    assert_before("constant-index", &emitted, "make(", "__cl_tmp0[0] = NULL;");

    let output = run_memory("constant-index-run", source);
    assert_exit("constant-index-run", &output, 3);
    assert_no_asan("constant-index-run", &output);
}

/// A slice view over a statement temporary moves its element into a binding.
///
/// `storeSlice` binds `slice(make(), 0, 2)[i]` with `i` one. The selected
/// element must move out before the statement temporary releases the rows, so
/// the returned string is still valid and measures three bytes; the sanitizer
/// path would report a read of freed memory otherwise.
#[test]
fn should_move_the_element_when_a_slice_statement_temporary_is_bound() {
    let source = r#"module closeout.slicestore;

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
    let output = run_memory("slice-store", source);
    assert_exit("slice-store", &output, 3);
    assert_no_asan("slice-store", &output);
}

/// A slice view with a non-zero start moves its element into a binding.
///
/// `storeSlice` binds `slice(make(), 1, 2)[i]` with `i` zero, which selects
/// the second row element. Counting rows, not scalars, finds `"cde"`; the
/// element must move out before the temporary releases the rows, so `main`
/// answers three.
#[test]
fn should_move_the_element_when_the_slice_start_is_non_zero() {
    let source = r#"module closeout.slicestoreoffset;

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
    let output = run_memory("slice-store-offset", source);
    assert_exit("slice-store-offset", &output, 3);
    assert_no_asan("slice-store-offset", &output);
}

/// A slice view over a statement temporary passes its element to a `take`
/// parameter.
///
/// `takeSlice` calls `consume<T>(slice(make(), 0, 2)[i])` with `i` one, so the
/// callee receives the second element and owns it. The element must move out
/// before the temporary releases the rows: a copy would be freed twice, which
/// aborts, and the emitted C must name no unknown-typed callee. The consumed
/// string measures three bytes, so `main` answers three.
#[test]
fn should_transfer_the_element_when_a_slice_statement_temporary_is_taken() {
    let source = r#"module closeout.slicetake;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func consume<T>(take v: T): i32 {
    return i32(len(v));
}

func takeSlice<T>(make: func(): T[2], i: i32): i32 {
    return consume<T>(slice(make(), 0, 2)[i]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return takeSlice<String>(mkString, 1);
}
"#;
    let emitted = emit_host("slice-take", source);
    assert!(
        !emitted.contains("Unknown"),
        "slice-take: the emitted C must not name an unknown-typed symbol"
    );

    let output = run_memory("slice-take-run", source);
    assert_exit("slice-take-run", &output, 3);
    assert_no_asan("slice-take-run", &output);
}

/// Concrete element moves through a slice view stay rejected.
///
/// This is a regression pin: the element-move limit reports `E10026` for a
/// concrete `String` element whether the element is taken by a callee or bound
/// to an owned local, and a rejected program writes no translation unit. The
/// generic spellings above are the forms the fixes widen; the concrete
/// spellings must keep the existing limit.
#[test]
fn should_reject_a_concrete_slice_element_move_when_the_element_is_taken_or_stored() {
    let take_source = r#"module closeout.slicerejecttake;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func consume(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("cde")];
    let view: String[] = slice(xs, 0, 2);
    return consume(take view[1]);
}
"#;
    assert_rejected("slice-reject-take", take_source, "E10026");

    let store_source = r#"module closeout.slicerejectstore;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("cde")];
    let view: String[] = slice(xs, 0, 2);
    let s: String = view[1];
    return i32(len(s));
}
"#;
    assert_rejected("slice-reject-store", store_source, "E10026");
}

/// Aggregate-literal fields evaluate in source order.
///
/// The language evaluates an aggregate literal's fields left to right, so an
/// earlier field's call runs before a later field's projection of a fresh call
/// result. The program records the two calls in a global counter and answers
/// with the counter; running the later field's call first would answer
/// twenty-one instead of twelve. The case pins the order and the value under
/// the strict host build: the member read leaves the call result's sibling
/// field to the statement cleanup, a pre-existing leak outside this case.
#[test]
fn should_evaluate_aggregate_fields_left_to_right_when_a_later_field_consumes_a_projection() {
    let source = r#"module closeout.fieldorder;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

struct Box {
    w: String;
    v: String;
}

let trace: i32 = 0;

func first(): String {
    trace = trace * 10 + 1;
    return stringOf("w");
}

func second(): Pair {
    trace = trace * 10 + 2;
    return Pair { a: stringOf("ab"), b: stringOf("cde") };
}

func main(): i32 {
    let b: Box = Box { w: first(), v: second().a };
    if (i32(len(b.v)) != 2) {
        return 99;
    }
    return trace;
}
"#;
    let output = run_host("field-order", source);
    assert_exit("field-order", &output, 12);
}

/// A `len` call on an owned string-literal result releases the string once.
///
/// `i32(len(stringOf("abc")))` passes a heap string straight into the intrinsic
/// that measures it. The string has no named owner, so the caller must bind it
/// and release it when the statement ends or the function returns; lowering the
/// call inline with no owner leaks the bytes. The program answers three, and
/// the sanitizer path reports a leak whenever the release is missing.
#[test]
fn should_release_the_string_when_len_reads_a_string_of_call_result() {
    let source = r#"module spec.closeout.st8;

import { String, stringOf } from std.text;

func main(): i32 {
    return i32(len(stringOf("abc")));
}
"#;
    let output = run_memory("closeout-st8-literal", source);
    assert_exit("closeout-st8-literal", &output, 3);
    assert_no_asan("closeout-st8-literal", &output);
}

/// A `len` call on a runtime-indexed store result releases the stored string.
///
/// `storeAt` moves the selected array element out of the temporary and returns
/// it, so the caller receives a freshly owned one-character string that no
/// local names. Measuring it through `len` must bind that string and release
/// it exactly once; the program answers one, and the sanitizer path reports a
/// leak whenever the release is missing.
#[test]
fn should_release_the_stored_string_when_len_reads_a_call_result() {
    let source = r#"module spec.closeout.st8;

import { String, stringOf } from std.text;

func storeAt<T>(mk: func(): T[2], i: i32): T {
    let s: T = mk()[i];
    return s;
}

func mkString(): String[2] {
    return [stringOf("a"), stringOf("b")];
}

func main(): i32 {
    return i32(len(storeAt<String>(mkString, 1)));
}
"#;
    let output = run_memory("closeout-st8-store", source);
    assert_exit("closeout-st8-store", &output, 1);
    assert_no_asan("closeout-st8-store", &output);
}

/// The dot spelling of `len` on a call result releases the string once.
///
/// `stringOf("a").len()` asks a one-character heap string for its length. The
/// receiver has no named owner, so the caller must bind it and release it when
/// the statement ends or the function returns; an inline receiver leaks the
/// bytes. The program answers one, and the sanitizer path reports a leak
/// whenever the release is missing.
#[test]
fn should_release_the_receiver_when_the_dot_len_form_reads_a_call_result() {
    let source = r#"module spec.closeout.st9;

import { String, stringOf } from std.text;

func main(): i32 {
    return i32(stringOf("a").len());
}
"#;
    let output = run_memory("closeout-st9", source);
    assert_exit("closeout-st9", &output, 1);
    assert_no_asan("closeout-st9", &output);
}

/// A guarded `len` call stays lazy and releases the string on the taken path.
///
/// The condition `flag && len(make()) == 3` must not evaluate `make()` when the
/// flag is false, so short-circuit evaluation stays lazy (a counter global
/// proves the call never ran). When the flag is true the call must run exactly
/// once, measure three bytes, and its ownerless heap string must be released
/// when the function returns; the sanitizer path reports a leak otherwise. The
/// true path answers three and the false path answers five.
#[test]
fn should_keep_the_call_lazy_when_a_guarded_len_call_reads_a_call_result() {
    let source_true = r#"module spec.closeout.st10;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func make(): String {
    calls = calls + 1;
    return stringOf("abc");
}

func guarded(flag: bool): i32 {
    let hit: bool = flag && len(make()) == 3;
    if (hit) {
        if (calls == 1) {
            return 3;
        }
        return 98;
    }
    if (calls == 0) {
        return 5;
    }
    return 99;
}

func main(): i32 {
    return guarded(true);
}
"#;
    let output_true = run_memory("closeout-st10-true", source_true);
    assert_exit("closeout-st10-true", &output_true, 3);
    assert_no_asan("closeout-st10-true", &output_true);

    let source_false = r#"module spec.closeout.st10;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func make(): String {
    calls = calls + 1;
    return stringOf("abc");
}

func guarded(flag: bool): i32 {
    let hit: bool = flag && len(make()) == 3;
    if (hit) {
        if (calls == 1) {
            return 3;
        }
        return 98;
    }
    if (calls == 0) {
        return 5;
    }
    return 99;
}

func main(): i32 {
    return guarded(false);
}
"#;
    let output_false = run_memory("closeout-st10-false", source_false);
    assert_exit("closeout-st10-false", &output_false, 5);
    assert_no_asan("closeout-st10-false", &output_false);
}

/// A local string place passed to `len` keeps its value and its clean run.
///
/// This is a regression pin: the argument is an existing local binding, not an
/// ownerless call result, so measuring it needs no temporary owner and must
/// neither disturb the binding nor release it early. The program answers three
/// and the sanitizer path stays clean.
#[test]
fn should_keep_the_local_string_when_len_reads_a_place_argument() {
    let source = r#"module spec.closeout.st11;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("abc");
    let n: i32 = i32(len(s));
    return n;
}
"#;
    let output = run_memory("closeout-st11", source);
    assert_exit("closeout-st11", &output, 3);
    assert_no_asan("closeout-st11", &output);
}

/// A conditional argument with one place-rooted arm is read in place.
///
/// The language passes an intrinsic argument by borrow, so measuring
/// `flag ? s : stringOf("x")` must not adopt and empty either arm: the chosen
/// place stays the caller's and the chosen fresh string is read where it
/// stands. Adopting the place arm would empty the caller's binding and the
/// later read of it would run on freed storage. The taken path answers three
/// and the false path answers one, and the local string is measured again
/// after the conditional to prove it was not emptied.
#[test]
fn should_read_the_chosen_arm_when_len_reads_a_conditional_with_a_place_arm() {
    let source = r#"module spec.closeout.st8cond;

import { String, stringOf } from std.text;

func measure(flag: bool, s: String): i32 {
    return i32(len(flag ? s : stringOf("x"))) * 10 + i32(len(s));
}

func main(): i32 {
    let s: String = stringOf("abc");
    let a: i32 = measure(true, s);
    let b: i32 = measure(false, s);
    return a + b;
}
"#;
    let output = run_host("closeout-st8-cond", source);
    assert_exit("closeout-st8-cond", &output, 46);
}

/// A non-null `String?` out-slot receives the callee's value and releases the
/// old one exactly once.
///
/// `fill<String>` takes the new string and assigns it through a writable
/// out-slot that already owns the old string. The callee owns the incoming
/// value on entry, so the assignment must release the old string in the
/// caller's storage and leave the new string there. The caller then measures
/// the new string, so `main` answers three. An out-slot passed by value would
/// leave the caller with the old string, and the callee and the caller would
/// each release it, aborting in the C library or reporting a double free under
/// AddressSanitizer.
#[test]
fn should_write_through_when_a_non_null_nullable_string_out_slot_is_assigned() {
    let source = r#"module spec.closeout.st12;

import { String, stringOf } from std.text;

func fill<T>(take v: T, borrow slot: String?): void {
    slot = v;
}

func main(): i32 {
    let slot: String? = stringOf("ab");
    fill<String>(stringOf("xyz"), borrow slot);
    if (slot == null) {
        return 9;
    }
    return i32(len(slot));
}
"#;
    let output = run_memory("closeout-st12", source);
    assert_exit("closeout-st12", &output, 3);
    assert_no_asan("closeout-st12", &output);
}

/// A non-null `File?` out-slot receives the callee's handle and closes each
/// handle exactly once.
///
/// `fill<File>` takes a freshly opened handle and assigns it through a writable
/// out-slot that already owns another open handle. The callee owns the incoming
/// handle on entry, so the assignment must close the old handle in the caller's
/// storage and leave the new handle there. The caller then falls off the scope
/// and closes the new handle once, so `main` answers zero. A by-value out-slot
/// would leave the caller the already-closed old handle and both sides would
/// close it again, aborting in the C library or reporting a use-after-free
/// under AddressSanitizer.
#[test]
fn should_write_through_when_a_non_null_nullable_file_out_slot_is_assigned() {
    let source = r#"module spec.closeout.st13;

import { File, openFile, MODE_OLDFILE } from std.io;

func fill<T>(take v: T, borrow slot: File?): void {
    slot = v;
}

func main(): i32 {
    let first: File? = openFile("/dev/null", MODE_OLDFILE);
    if (first == null) {
        return 8;
    }
    let second: File? = openFile("/dev/null", MODE_OLDFILE);
    if (second == null) {
        return 9;
    }
    let slot: File? = first;
    fill<File>(second, borrow slot);
    if (slot == null) {
        return 10;
    }
    return 0;
}
"#;
    let output = run_memory("closeout-st13", source);
    assert_exit("closeout-st13", &output, 0);
    assert_no_asan("closeout-st13", &output);
}

/// A `str?` out-slot receives the callee's view so the caller can read it.
///
/// `fill` assigns its `str` argument into a `str?` place. The call must pass
/// the address of the caller's storage, so the view is written through and the
/// caller measures three bytes after the call. A `str?` slot passed by value
/// would leave the caller's slot empty and `main` would answer nine; the
/// emitted translation must also build as strict C, because a nullable view has
/// no by-value spelling.
#[test]
fn should_write_through_when_a_nullable_str_out_slot_is_assigned() {
    let source = r#"module spec.closeout.st14;

func fill(v: str, borrow slot: str?): void {
    slot = v;
}

func main(): i32 {
    let slot: str? = null;
    fill("abc", borrow slot);
    if (slot == null) {
        return 9;
    }
    return i32(len(slot));
}
"#;
    let output = run_memory("closeout-st14-write", source);
    assert_exit("closeout-st14-write", &output, 3);
    assert_no_asan("closeout-st14-write", &output);
}

/// A `String` place that would widen into a `str?` out-slot is rejected.
///
/// An out-slot needs an exact-typed place because the callee writes back into
/// the caller's storage; a converted temporary cannot carry that write. A
/// `String` place needs a view conversion to reach a `str?` slot, so the
/// argument is rejected with `E10200` and no translation unit is written.
#[test]
fn should_reject_when_a_string_place_widens_into_a_str_out_slot() {
    let source = r#"module spec.closeout.st14;

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
    assert_rejected_with("closeout-st14-wide-str-slot", source, "E10200", &[]);
}

/// A `String` place that would widen into a `String?` out-slot is rejected.
///
/// The argument needs only the nullability widening, but an out-slot must
/// still be an exact-typed place: the callee writes back into the caller's
/// storage, and a converted temporary cannot carry that write. The argument is
/// rejected with `E10200` and no translation unit is written.
#[test]
fn should_reject_when_a_string_place_widens_into_a_string_out_slot() {
    let source = r#"module spec.closeout.st14;

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
    assert_rejected_with("closeout-st14-wide-string-slot", source, "E10200", &[]);
}

/// A null-start out-slot stays accepted and may be reassigned to null.
///
/// The callee owns the incoming value on entry, but a null slot has nothing to
/// release, and writing null leaves the caller's slot empty. This is the
/// established out-parameter exemption: the callee may reassign the slot
/// without the borrowed-parameter diagnostic, and `main` answers one after
/// observing the null slot.
#[test]
fn should_keep_the_null_slot_when_a_null_start_out_slot_is_reassigned_to_null() {
    let source = r#"module spec.closeout.st15;

import { String, stringOf } from std.text;

func fill<T>(v: T, borrow slot: String?): void {
    slot = null;
}

func main(): i32 {
    let slot: String? = null;
    fill<String>(stringOf("xyz"), borrow slot);
    if (slot == null) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_memory("closeout-st15-nullstart", source);
    assert_exit("closeout-st15-nullstart", &output, 1);
    assert_no_asan("closeout-st15-nullstart", &output);
}

/// A writable lend of a non-nullable duplicable struct stays by value.
///
/// `bump` increments its `Point` parameter, but a duplicable struct is passed
/// by value, so the write affects only the callee's copy and `main` still reads
/// one. The declaration warns that the lend is unnecessary; the behavior is
/// deliberate and must not change when nullable out-slots become pointers.
#[test]
fn should_keep_the_caller_value_when_a_non_nullable_duplicable_struct_is_lent() {
    let source = r#"module spec.closeout.st15;

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
    let output = run_memory("closeout-st15-byvalue", source);
    assert_exit("closeout-st15-byvalue", &output, 1);
    assert_no_asan("closeout-st15-byvalue", &output);
}

/// A widened place reaching an out-slot through a generic call is rejected.
///
/// An explicit instance fixes the callee's parameter type, so the same
/// exact-place rule as a direct call applies: a `String` place cannot widen
/// into a `String?` slot, because the callee writes through the argument.
#[test]
fn should_reject_when_a_widened_place_reaches_an_out_slot_through_a_generic_call() {
    let source = r#"module spec.closeout.p3f1;

import { String, stringOf } from std.text;

func fill<T>(take v: T, borrow slot: String?): void {
    slot = v;
}

func main(): i32 {
    let owned: String = stringOf("abc");
    fill<String>(stringOf("xyz"), borrow owned);
    return 0;
}
"#;
    assert_rejected("closeout-p3f1", source, "E10200");
}

/// A widened place reaching an out-slot through a dot-call is rejected.
///
/// The dot-call route carries the callee's parameter modes, so the out-slot
/// rule applies to the receiver-taking call as well.
#[test]
fn should_reject_when_a_widened_place_reaches_an_out_slot_through_a_dot_call() {
    let source = r#"module spec.closeout.p3f1dot;

import { String, stringOf } from std.text;

func fill<T>(take v: T, borrow slot: String?): void {
    slot = v;
}

func main(): i32 {
    let moved: String = stringOf("xyz");
    let owned: String = stringOf("abc");
    moved.fill(borrow owned);
    return 0;
}
"#;
    assert_rejected("closeout-p3f1dot", source, "E10200");
}

/// An explicit instance that makes a generic slot nullable writes through.
///
/// The instance substitution fixes the parameter type at the call site, so the
/// callee receives a pointer to the caller's slot, replaces the value, and the
/// caller measures the new string.
#[test]
fn should_write_through_when_an_explicit_instance_makes_a_generic_slot_nullable() {
    let source = r#"module spec.closeout.p3f2;

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
    let output = run_memory("closeout-p3f2", source);
    assert_exit("closeout-p3f2", &output, 3);
    assert_no_asan("closeout-p3f2", &output);
}

/// A null view runs through an argument, a conditional, and a global slot.
///
/// The zero-view form keeps every expression position type-correct; the
/// argument and the conditional are measured and the run is clean.
#[test]
fn should_run_when_a_null_view_is_passed_and_measured() {
    let source = r#"module spec.closeout.p3f4;

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
    let output = run_memory("closeout-p3f4", source);
    assert_exit("closeout-p3f4", &output, 4);
    assert_no_asan("closeout-p3f4", &output);
}

/// A wrong-typed place reaching an out-slot inside a generic body is rejected.
///
/// The body is accepted while the slot type is still a parameter, so the
/// instantiation audit applies the exact-place rule once `outer<str?>` fixes
/// the slot to `str?`; the `i32` place cannot receive the write.
#[test]
fn should_reject_when_a_wrong_typed_place_reaches_an_out_slot_inside_a_generic_body() {
    let source = r#"module spec.closeout.p3r21;

func fill<T>(take v: T, borrow slot: T): void {
    slot = v;
}

func outer<T>(take v: T): i32 {
    let i: i32 = 7;
    fill<T>(v, borrow i);
    return i;
}

func main(): i32 {
    return outer<str?>("xyz");
}
"#;
    assert_rejected("closeout-p3r21", source, "E10200");
}

/// A widened dot-call receiver reaching an out-slot is rejected.
///
/// The receiver is the first parameter of the receiver-taking call, so a
/// nullable borrow receiver takes the same exact-place rule as a written
/// argument: a `String` receiver cannot widen into a `String?` slot.
#[test]
fn should_reject_when_a_widened_receiver_reaches_an_out_slot_through_a_dot_call() {
    let source = r#"module spec.closeout.p3r22;

import { String, stringOf } from std.text;

func fill(borrow slot: String?): void {
    slot = null;
}

func main(): i32 {
    let owned: String = stringOf("abc");
    owned.fill();
    return i32(len(owned));
}
"#;
    assert_rejected("closeout-p3r22", source, "E10200");
}

/// A valid parameter-typed nullable slot still accepts a dot-call.
///
/// The declared slot type mentions a type parameter, so the exact-place rule
/// waits for the instantiation; the `i32[]?` place is exact and the call is
/// accepted with the receiver as the taken value.
#[test]
fn should_run_when_a_dot_call_passes_an_exact_place_to_a_parameter_typed_slot() {
    let source = r#"module spec.closeout.p3r26;

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
    let output = run_memory("closeout-p3r26", source);
    assert_exit("closeout-p3r26", &output, 0);
    assert_no_asan("closeout-p3r26", &output);
}

/// Two instances that differ only by type-argument nullability both build.
///
/// `pick<String>` and `pick<String?>` are distinct instances: the nullable
/// argument keys a distinct symbol, so the two emitted definitions do not
/// collide.
#[test]
fn should_build_when_two_instances_differ_only_by_type_argument_nullability() {
    let source = r#"module spec.closeout.p3r24;

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
    let output = run_memory("closeout-p3r24", source);
    assert_exit("closeout-p3r24", &output, 4);
    assert_no_asan("closeout-p3r24", &output);
}

/// A nested null conditional arm is measured through the zero view.
///
/// The chosen nested arm is the string, so the run measures three; the null
/// arm would have been an invalid C comparison without the zero-view form.
#[test]
fn should_run_when_a_nested_conditional_arm_is_null() {
    let source = r#"module spec.closeout.p3r25;

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
    let output = run_memory("closeout-p3r25", source);
    assert_exit("closeout-p3r25", &output, 3);
    assert_no_asan("closeout-p3r25", &output);
}

/// A generic caller instantiating a generic callee with the same parameter
/// name still passes the address of its exact place.
///
/// The caller's type parameter and the callee's share the name `T`; the
/// callee's instance arguments must decide its slot type, so the write reaches
/// the caller's slot and the run measures three.
#[test]
fn should_write_through_when_a_generic_caller_instantiates_a_generic_callee() {
    let source = r#"module spec.closeout.p3r23;

import { String, stringOf } from std.text;

func inner<T>(take v: T, borrow slot: T): void {
    slot = v;
}

func outer<T>(take v: T): i32 {
    let slot: String? = null;
    inner<String?>(v, borrow slot);
    if (slot == null) {
        return 9;
    }
    return i32(len(slot));
}

func main(): i32 {
    return outer<String>(stringOf("xyz"));
}
"#;
    let output = run_memory("closeout-p3r23", source);
    assert_exit("closeout-p3r23", &output, 3);
    assert_no_asan("closeout-p3r23", &output);
}

/// A deferred return from a do-while body releases the live condition
/// temporary.
///
/// The condition `grow().count > 0 && i < 2` materializes an array at the
/// bottom of each completed pass. The body registers a `defer` that returns on
/// the second pass, and that return leaves the loop without ever evaluating
/// the condition again. The array materialized after the first pass is still
/// owned at that point, so the return must release it. The program answers
/// two, and the sanitizer path reports a leak whenever the release is missing.
#[test]
fn should_release_the_condition_temporary_when_a_deferred_return_leaves_a_do_while_body() {
    let source = r#"module spec.closeout.st16;

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
    let output = run_memory("closeout-st16-deferred", source);
    assert_exit("closeout-st16-deferred", &output, 2);
    assert_no_asan("closeout-st16-deferred", &output);
}

/// A deferred return from a while body keeps the condition release it already
/// carries and adds no second one.
///
/// A `while` lowers its condition before the body, so the condition's owned
/// temporary already has its release when the body registers the defer; the
/// return inside the defer must keep that release unchanged. The program
/// answers two, and the sanitizer path stays clean.
#[test]
fn should_stay_clean_when_a_deferred_return_leaves_a_while_body() {
    let source = r#"module spec.closeout.st18;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func main(): i32 {
    let i: i32 = 0;
    while (grow().count > 0 && i < 2) {
        i = i + 1;
        defer {
            if (i == 2) {
                return i;
            }
        }
    }
    return 0;
}
"#;
    let output = run_memory("closeout-st18-deferred", source);
    assert_exit("closeout-st18-deferred", &output, 2);
    assert_no_asan("closeout-st18-deferred", &output);
}

/// A deferred return that never fires leaves every loop exit's release alone.
///
/// Both loops register a deferred block whose return never runs, so the exits
/// are `break` and the false condition, and each exit is followed by a return
/// after the loop. The condition still materializes an array temporary, so
/// every path must release it exactly once: a duplicated release aborts in the
/// C library and a missing one leaks under AddressSanitizer. The first helper
/// answers twenty and the second answers sixty, so `main` answers eighty.
#[test]
fn should_keep_one_release_per_exit_when_a_deferred_return_is_not_taken() {
    let source = r#"module spec.closeout.st19;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32> {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    return a;
}

func leaveByBreak(): i32 {
    let i: i32 = 0;
    do {
        i = i + 1;
        defer {
            if (i == 99) {
                return i;
            }
        }
        if (i == 2) {
            break;
        }
    } while (grow().count > 0 && i < 5);
    return i * 10;
}

func leaveByFallThrough(): i32 {
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
    return leaveByBreak() + leaveByFallThrough();
}
"#;
    let output = run_memory("closeout-st19-paths", source);
    assert_exit("closeout-st19-paths", &output, 80);
    assert_no_asan("closeout-st19-paths", &output);
}

/// A lambda that lists every enclosing name it reads is accepted.
///
/// Both enclosing locals appear in the capture list and the immediate
/// invocation reads both, so the program compiles and runs and answers the
/// sum, eight.
#[test]
fn should_accept_a_lambda_that_lists_every_enclosing_name_it_reads() {
    let source = r#"module spec.closeout.st21;

func main(): i32 {
    let n: i32 = 7;
    let x: i32 = 1;
    return ([x, n] () => n + x)();
}
"#;
    let output = run_host("closeout-st21-listed", source);
    assert_exit("closeout-st21-listed", &output, 8);
}

/// A capture-less lambda may read module-level names.
///
/// A module constant and a module function sit above every lambda scope floor,
/// so a lambda with an empty capture list reads them without listing them. The
/// program answers forty-two.
#[test]
fn should_accept_a_captureless_lambda_that_reads_module_names() {
    let source = r#"module spec.closeout.st22;

const base: i32 = 40;

func addTwo(v: i32): i32 {
    return v + 2;
}

func main(): i32 {
    let f: func(i32): i32 = (v: i32) => addTwo(base) + v;
    return f(0);
}
"#;
    let output = run_host("closeout-st22-module", source);
    assert_exit("closeout-st22-module", &output, 42);
}

/// A nested lambda that lists the outer capture it reads is accepted.
///
/// The inner lambda reads the outer lambda's capture, so it must list the name
/// at its own level; with the listing present the program runs and answers
/// four.
#[test]
fn should_accept_a_nested_lambda_that_lists_the_outer_capture() {
    let source = r#"module spec.closeout.st23;

func main(): i32 {
    let n: i32 = 4;
    return ([n] () => ([n] () => n)())();
}
"#;
    let output = run_host("closeout-st23-nested-listed", source);
    assert_exit("closeout-st23-nested-listed", &output, 4);
}

/// A nested lambda that reads the outer capture without listing it is
/// rejected: the capture list must be repeated at each level.
#[test]
fn should_reject_a_nested_lambda_that_reads_the_outer_capture_without_listing_it() {
    let source = r#"module spec.closeout.st23;

func main(): i32 {
    let n: i32 = 4;
    return ([n] () => ([] () => n)())();
}
"#;
    assert_rejected("closeout-st23-nested-unlisted", source, "E10405");
}

/// Moving a captured binding still reports E10401.
#[test]
fn should_still_reject_moving_a_captured_binding() {
    let source = r#"module spec.closeout.st24a;

import { stringOf } from std.text;

func consume(take s: String): void {
}

func main(): i32 {
    let s: String = stringOf("x");
    ([s] () => consume(s))();
    return 0;
}
"#;
    assert_rejected_with(
        "closeout-st24-move",
        source,
        "E10401",
        &["cannot move captured 's'"],
    );
}

/// Writing a captured binding without a borrow capture still reports E10402.
#[test]
fn should_still_reject_writing_a_captured_binding_without_a_borrow_capture() {
    let source = r#"module spec.closeout.st24b;

func main(): i32 {
    let counter: i32 = 0;
    ([counter] () => { counter = counter + 1; })();
    return counter;
}
"#;
    assert_rejected_with(
        "closeout-st24-write",
        source,
        "E10402",
        &["cannot write captured 'counter' without '[borrow counter]'"],
    );
}

/// A take capture still reports E10403.
#[test]
fn should_still_reject_a_take_capture() {
    let source = r#"module spec.closeout.st24c;

import { stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("x");
    ([take s] () => s)();
    return 0;
}
"#;
    assert_rejected_with(
        "closeout-st24-take",
        source,
        "E10403",
        &["'take' captures are not supported"],
    );
}
