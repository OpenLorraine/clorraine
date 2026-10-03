//! Runnable specification tests for the generic-lend and move-out defect fixes.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The chosen value travels in
//! `main`'s return code, so a case that prints nothing states its answer as the
//! exit code.
//!
//! A case whose program is accepted but whose emitted C the host compiler
//! rejects states the defect directly: the emitted translation must build. A
//! case that runs also pins release-once behavior, because a double release
//! aborts in the C library before the exit code is seen.
//!
//! The element-read cases pin that a fixed-array temporary gives up exactly the
//! slot the callee took or the binding selected; emptying the wrong slot
//! double-releases, and copying a slot the temporary still releases leaves a
//! dangling value. The memory helper adds leak and use-after-free detection
//! when `RDC_ASAN` is set, and otherwise runs the same program as an ordinary
//! strict host build.

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
            "clc-dbcspec-{}-{tag}-{sequence}",
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

/// Counts the reports of one diagnostic code in the compiler's stderr.
fn error_count(stderr: &str, code: &str) -> usize {
    stderr.matches(&format!("error {code}:")).count()
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

/// Returns the part of the text that starts at the marker.
fn after_marker<'a>(tag: &str, text: &'a str, marker: &str) -> &'a str {
    let at = text
        .find(marker)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {marker:?}"));
    &text[at..]
}

/// Returns the C function body that starts at `start`.
///
/// `start` includes the declarator and the opening brace so a forward
/// declaration with the same signature does not match. The body ends at the
/// first closing brace in column zero.
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

/// A generic lend returned as an owned value reports `E10034` under an owning
/// instance.
///
/// `apply<String>` returns its read-only parameter, which would hand the
/// caller's string to a new owner. The concrete spelling of the same move
/// already rejects with the borrowed-parameter diagnostic, so the generic
/// spelling must name the parameter and write no translation unit.
#[test]
fn should_reject_when_a_generic_lend_is_returned_under_an_owning_instance() {
    let source = r#"module spec.dbc.st1;

import { String, stringOf } from std.text;

func apply<T>(x: T): T {
    return x;
}

func main(): i32 {
    let s: String = apply<String>(stringOf("abc"));
    return i32(len(s));
}
"#;
    assert_rejected_with(
        "dbc01",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'x'"],
    );
}

/// Forwarding a lend through a lend parameter reports `E10034` exactly once.
///
/// `outer` hands its lend `x` to `apply`'s lend parameter, which is not itself
/// a move; only the return inside `apply` moves the lend into an owned result.
/// One report per move site keeps a forwarding chain from multiplying the
/// diagnostic.
#[test]
fn should_report_once_when_a_generic_lend_is_forwarded_through_a_lend_parameter() {
    let source = r#"module spec.dbc.st2;

import { String, stringOf } from std.text;

func apply<T>(x: T): T {
    return x;
}

func outer<T>(x: T): T {
    return apply<T>(x);
}

func main(): i32 {
    let s: String = outer<String>(stringOf("abc"));
    return i32(len(s));
}
"#;
    let (_scratch, output, c_path) = compile_only("dbc02", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "dbc02: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        error_count(&stderr, "E10034"),
        1,
        "dbc02: expected exactly one E10034; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "dbc02: a rejected program must not produce a translation unit"
    );
}

/// A duplicable instance copies the lend and returns the argument.
///
/// `apply<i32>` has a duplicable type argument, so the lend binds a copy and
/// the return moves that copy; the program is accepted and `main` returns the
/// argument it passed.
#[test]
fn should_copy_when_a_generic_lend_is_duplicable() {
    let source = r#"module spec.dbc.st3;

func apply<T>(x: T): T {
    return x;
}

func main(): i32 {
    return apply<i32>(41);
}
"#;
    let output = run_host("dbc03", source);
    assert_exit("dbc03", &output, 41);
}

/// A generic lend stored through a take parameter reports `E10034`.
///
/// `f` hands its read-only lend `v` to `Array.push`'s take parameter, which
/// releases the lend under an owning instance. The move site names `v` and no
/// translation unit is written.
#[test]
fn should_reject_when_a_generic_lend_is_stored_through_a_take_parameter() {
    let source = r#"module spec.dbc.st4;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

struct Holder<T> {
    items: Array<T>;
}

func f<T>(take h: Holder<T>, v: T): void {
    h.items.push(v);
}

func main(): i32 {
    let h: Holder<String> = Holder<String> { items: arrayNew<String>() };
    f<String>(take h, stringOf("abc"));
    return 0;
}
"#;
    assert_rejected_with(
        "dbc04",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'v'"],
    );
}

/// A concrete lend stored through a take parameter keeps its rejection.
///
/// The same holder shape spelled with concrete types still reports `E10034`;
/// the generic duty must not change the concrete spelling's behavior.
#[test]
fn should_reject_when_a_concrete_lend_is_stored_through_a_take_parameter() {
    let source = r#"module spec.dbc.st5;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

struct Holder {
    items: Array<String>;
}

func f(take h: Holder, v: String): void {
    h.items.push(v);
}

func main(): i32 {
    let h: Holder = Holder { items: arrayNew<String>() };
    f(take h, stringOf("abc"));
    return 0;
}
"#;
    assert_rejected_with(
        "dbc05",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'v'"],
    );
}

/// Moving a take parameter into the return value is legal.
///
/// `g` owns its `take` parameter, so returning it transfers that ownership to
/// the caller; the program is accepted and the returned string measures three
/// bytes. A release of the moved value would abort or change the exit code.
#[test]
fn should_move_when_a_take_parameter_is_returned_under_an_owning_instance() {
    let source = r#"module spec.dbc.st6;

import { String, stringOf } from std.text;

func g<T>(take v: T): T {
    return v;
}

func main(): i32 {
    let s: String = g<String>(stringOf("abc"));
    return i32(len(s));
}
"#;
    let output = run_host("dbc06", source);
    assert_exit("dbc06", &output, 3);
}

/// A runtime-indexed element take empties the slot the callee took.
///
/// `takeAt` asks `consume` to take `mk()[i]` with `i` one; the callee receives
/// the second element and the caller's temporary must empty that same slot, so
/// the remaining element is released once and the consumed string measures
/// three bytes.
#[test]
fn should_empty_the_taken_slot_when_a_runtime_index_selects_it() {
    let source = r#"module spec.dbc.st7;

import { String, stringOf } from std.text;

func consume<T>(take v: T): i32 {
    return i32(len(v));
}

func takeAt<T>(mk: func(): T[2], i: i32): i32 {
    return consume<T>(mk()[i]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return takeAt<String>(mkString, 1);
}
"#;
    let output = run_memory("dbc07", source);
    assert_exit("dbc07", &output, 3);
}

/// A guarded runtime-indexed take keeps short-circuit evaluation.
///
/// With the guard true the call takes the second element, whose length makes
/// the comparison true, so `main` returns seven. With the guard false the call
/// never runs and `main` returns zero; nothing is released on either path
/// twice.
#[test]
fn should_keep_the_short_circuit_when_a_guarded_runtime_index_is_taken() {
    let source_true = r#"module spec.dbc.st8;

import { String, stringOf } from std.text;

func consume<T>(take v: T): i32 {
    return i32(len(v));
}

func guardAt<T>(mk: func(): T[2], i: i32, flag: bool): i32 {
    let hit: bool = flag && consume<T>(mk()[i]) == 3;
    return hit ? 7 : 0;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return guardAt<String>(mkString, 1, true);
}
"#;
    let output_true = run_memory("dbc08t", source_true);
    assert_exit("dbc08t", &output_true, 7);

    let source_false = r#"module spec.dbc.st8;

import { String, stringOf } from std.text;

func consume<T>(take v: T): i32 {
    return i32(len(v));
}

func guardAt<T>(mk: func(): T[2], i: i32, flag: bool): i32 {
    let hit: bool = flag && consume<T>(mk()[i]) == 3;
    return hit ? 7 : 0;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return guardAt<String>(mkString, 1, false);
}
"#;
    let output_false = run_memory("dbc08f", source_false);
    assert_exit("dbc08f", &output_false, 0);
}

/// A runtime-indexed element stored into a binding moves out of the temporary.
///
/// `storeAt` binds `mk()[i]` with `i` one to an owned local and returns it. The
/// statement temporary must empty the selected slot before it releases the
/// rows, so the returned string is still valid and measures three bytes.
#[test]
fn should_move_the_element_when_a_runtime_index_is_stored_into_a_binding() {
    let source = r#"module spec.dbc.st9;

import { String, stringOf } from std.text;

func storeAt<T>(mk: func(): T[2], i: i32): T {
    let s: T = mk()[i];
    return s;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(storeAt<String>(mkString, 1)));
}
"#;
    let output = run_memory("dbc09", source);
    assert_exit("dbc09", &output, 3);
}

/// A runtime-indexed element returned directly moves out of the temporary.
///
/// `first` returns `mk()[i]` with `i` one; the return path must empty the
/// selected slot before the statement temporary releases the rows, so the
/// caller receives the second element and measures three bytes.
#[test]
fn should_move_the_element_when_a_runtime_index_is_returned() {
    let source = r#"module spec.dbc.st10;

import { String, stringOf } from std.text;

func first<T>(mk: func(): T[2], i: i32): T {
    return mk()[i];
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(first<String>(mkString, 1)));
}
"#;
    let output = run_memory("dbc10", source);
    assert_exit("dbc10", &output, 3);
}

/// A constant-index element take keeps its emitted shape.
///
/// `zeroAt` takes `mk()[0]`; the callee receives the first element and the
/// caller's move-out still renders the constant slot, so the emitted C calls
/// the take with `__cl_tmp0[0]` and empties that slot. The consumed string
/// measures two bytes.
#[test]
fn should_keep_the_shape_when_a_constant_index_take_empties_the_slot() {
    let source = r#"module spec.dbc.st11;

import { String, stringOf } from std.text;

func consume<T>(take v: T): i32 {
    return i32(len(v));
}

func zeroAt<T>(mk: func(): T[2]): i32 {
    return consume<T>(mk()[0]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return zeroAt<String>(mkString);
}
"#;
    let emitted = emit_host("dbc11", source);
    assert_contains("dbc11", &emitted, "cl_consume__String(__cl_tmp0[0])");
    assert_contains("dbc11", &emitted, "__cl_tmp0[0] = NULL;");
    let output = run_memory("dbc11run", source);
    assert_exit("dbc11run", &output, 2);
}

/// A constant-index element store or return moves before the temporary release.
///
/// `storeFirst` binds `mk()[0]` to an owned local and `retFirst` returns
/// `mk()[0]` directly. Both must empty slot zero before the statement
/// temporary releases the rows, so the emitted C zeroes `__cl_tmp0[0]` before
/// the first `FreeString(__cl_tmp0[0])` in each function and the program exits
/// with the summed lengths of the first rows.
#[test]
fn should_move_the_element_when_a_constant_index_store_or_return_releases_the_temporary() {
    let source = r#"module spec.dbc.st20;

import { String, stringOf } from std.text;

func storeFirst<T>(mk: func(): T[2]): T {
    let s: T = mk()[0];
    return s;
}

func retFirst<T>(mk: func(): T[2]): T {
    return mk()[0];
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    let a: String = storeFirst<String>(mkString);
    let b: String = retFirst<String>(mkString);
    return i32(len(a)) + i32(len(b));
}
"#;
    let emitted = emit_host("dbc20", source);

    let store_body = function_section(
        "dbc20",
        &emitted,
        "static ClString cl_storeFirst__String(void (*mk)(ClString __cl_ret[2]))\n{",
    );
    // The base call fills the destination through a block temporary, so the
    // zero-before-release check anchors after that fill.
    let store_after = after_marker("dbc20", store_body, "cl_zero(&__cl_call_");
    assert_before(
        "dbc20",
        store_after,
        "__cl_tmp0[0] = NULL;",
        "FreeString(__cl_tmp0[0])",
    );

    let ret_body = function_section(
        "dbc20",
        &emitted,
        "static ClString cl_retFirst__String(void (*mk)(ClString __cl_ret[2]))\n{",
    );
    let ret_after = after_marker("dbc20", ret_body, "cl_zero(&__cl_call_");
    assert_before(
        "dbc20",
        ret_after,
        "__cl_tmp0[0] = NULL;",
        "FreeString(__cl_tmp0[0])",
    );

    let output = run_memory("dbc20run", source);
    assert_exit("dbc20run", &output, 4);
}

/// Storing a generic lend into an owning binding or aggregate reports `E10034`.
///
/// `keep` stores its lend `x` in a local and `pack` stores `x` in a struct
/// literal; both would give the caller's value a second owner under `String`.
/// The same binding shape under `i32` is accepted and returns seven, because a
/// duplicable value is copied.
#[test]
fn should_reject_when_a_generic_lend_is_stored_in_an_owning_binding_or_aggregate() {
    let bind_source = r#"module spec.dbc.st21;

import { String, stringOf } from std.text;

func keep<T>(x: T): T {
    let y: T = x;
    return y;
}

func main(): i32 {
    let s: String = keep<String>(stringOf("abc"));
    return i32(len(s));
}
"#;
    assert_rejected_with(
        "dbc21bind",
        bind_source,
        "E10034",
        &["cannot release or store borrowed parameter 'x'"],
    );

    let aggregate_source = r#"module spec.dbc.st21;

import { String, stringOf } from std.text;

struct Box<T> {
    v: T;
}

func pack<T>(x: T): Box<T> {
    return Box<T> { v: x };
}

func main(): i32 {
    let b: Box<String> = pack<String>(stringOf("abc"));
    return i32(len(b.v));
}
"#;
    assert_rejected_with(
        "dbc21agg",
        aggregate_source,
        "E10034",
        &["cannot release or store borrowed parameter 'x'"],
    );

    let duplicable_source = r#"module spec.dbc.st21;

func keep<T>(x: T): T {
    let y: T = x;
    return y;
}

func main(): i32 {
    return keep<i32>(7);
}
"#;
    let output = run_host("dbc21int", duplicable_source);
    assert_exit("dbc21int", &output, 7);
}

/// A concrete-typed lend returned in a maybe-owning position reports `E10034`.
///
/// `f` returns its string lend where the result type is the type parameter, so
/// the position may own at another instantiation. An owning instantiation is
/// judged the same way as the fully concrete spelling: the lend is stored in
/// the owned result and the borrowed-parameter diagnostic is reported.
#[test]
fn should_reject_when_a_concrete_typed_lend_escapes_a_maybe_owning_position() {
    let source = r#"module spec.dbc.st22;

import { String, stringOf } from std.text;

func f<T>(v: String): T {
    return v;
}

func main(): i32 {
    let s: String = f<String>(stringOf("abc"));
    return i32(len(s));
}
"#;
    assert_rejected_with(
        "dbc22",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'v'"],
    );
}

/// A lend element taken through a take parameter reports `E10034`.
///
/// `at` lends the fixed array `p` and passes `p[0]` to `sink`'s take
/// parameter; the element is part of a borrowed parameter and may not be
/// released by the callee. The partial move is judged by the element type, so
/// an owning instantiation reports the borrowed-parameter diagnostic.
#[test]
fn should_reject_when_a_generic_lend_element_is_taken() {
    let source = r#"module spec.dbc.st23;

import { String, stringOf } from std.text;

func sink<T>(take s: T): i32 {
    return 0;
}

func at<T>(p: T[2]): i32 {
    return sink<T>(p[0]);
}

func main(): i32 {
    let xs: String[2] = [stringOf("ab"), stringOf("cd")];
    return at<String>(xs);
}
"#;
    assert_rejected_with(
        "dbc23",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
    );
}

/// Reassigning a nullable out-param slot stays accepted under an owning
/// instance.
///
/// A nullable borrow parameter is an in-out ownership slot, so the callee may
/// replace the value it received and the out-param exemption keeps
/// `fill<String>` free of the borrowed-parameter diagnostic. The slot starts
/// empty, the reassignment clears it again, and the run is clean.
#[test]
fn should_run_when_a_generic_out_parameter_slot_is_reassigned_under_an_owning_instance() {
    let source = r#"module spec.dbc.st24;

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
    let output = run_memory("dbc24", source);
    assert_exit("dbc24", &output, 1);
}

/// A multi-level runtime index materializes every level and empties the exact
/// slot.
///
/// `storeAt2` binds `mk()[i][j]` and `takeAt2` passes `mk()[i][j]` to a take
/// parameter, both with `i` and `j` one. Every level is evaluated once, the
/// selected row element is the only one given up, and each path returns the
/// length of `"ghi"`, so `main` exits with six.
#[test]
fn should_empty_the_exact_slot_when_a_multi_level_runtime_index_selects_it() {
    let source = r#"module spec.dbc.st25;

import { String, stringOf } from std.text;

func consume<T>(take v: T): i32 {
    return i32(len(v));
}

func mkGrid(): String[2][2] {
    return [[stringOf("ab"), stringOf("cde")], [stringOf("f"), stringOf("ghi")]];
}

func storeAt2<T>(mk: func(): T[2][2], i: i32, j: i32): T {
    let s: T = mk()[i][j];
    return s;
}

func takeAt2<T>(mk: func(): T[2][2], i: i32, j: i32): i32 {
    return consume<T>(mk()[i][j]);
}

func main(): i32 {
    let a: i32 = i32(len(storeAt2<String>(mkGrid, 1, 1)));
    let b: i32 = takeAt2<String>(mkGrid, 1, 1);
    return a + b;
}
"#;
    let output = run_memory("dbc25", source);
    assert_exit("dbc25", &output, 6);
}

/// A moved lend read again reports the store and the later use.
///
/// `reuse` stores its lend `x` in the local `y` and then reads `x` again.
/// Under an owning instance the store is `E10034` and the later read of the
/// moved binding is `E10020`; each duty reports once, so the compiler writes
/// exactly one report of each and no translation unit.
#[test]
fn should_report_the_lend_and_the_later_use_when_a_moved_lend_is_read_again() {
    let source = r#"module spec.dbc.st26;

import { String, stringOf } from std.text;

func sink<T>(borrow s: T): i32 {
    return 0;
}

func reuse<T>(x: T): i32 {
    let y: T = x;
    sink<T>(borrow y);
    sink<T>(borrow x);
    return 0;
}

func main(): i32 {
    return reuse<String>(stringOf("abc"));
}
"#;
    let (_scratch, output, c_path) = compile_only("dbc26", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "dbc26: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        error_count(&stderr, "E10034"),
        1,
        "dbc26: expected exactly one E10034; stderr:\n{stderr}"
    );
    assert_eq!(
        error_count(&stderr, "E10020"),
        1,
        "dbc26: expected exactly one E10020; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "dbc26: a rejected program must not produce a translation unit"
    );
}

/// A returned fixed-array row moves out of the statement temporary.
///
/// `row` returns `m()[i]` where the element is itself a fixed array. The
/// hidden return destination receives a byte copy of the selected row and the
/// temporary's release must not free that row, so the caller reads the second
/// row and measures four bytes.
#[test]
fn should_move_the_row_when_a_fixed_array_row_is_returned() {
    let source = r#"module spec.dbc.st28;

import { String, stringOf } from std.text;

func row<T>(m: func(): T[2][2], i: i32): T[2] {
    return m()[i];
}

func mkGrid(): String[2][2] {
    return [[stringOf("a"), stringOf("bb")], [stringOf("ccc"), stringOf("dddd")]];
}

func main(): i32 {
    return i32(len(row<String>(mkGrid, 1)[1]));
}
"#;
    let output = run_memory("dbc28", source);
    assert_exit("dbc28", &output, 4);
}

/// A lend moved out of a raw element read is rejected under an owning instance.
///
/// `raw T` indexing yields the pointee, so moving it into the owned return is
/// the same lent escape the concrete spelling rejects with `E10034`.
#[test]
fn should_reject_when_a_lend_raw_element_is_moved_under_an_owning_instance() {
    let source = r#"module spec.dbc.st29;

import { String, stringOf } from std.text;

func f<T>(p: raw T): T {
    return unsafe(p[0]);
}

func main(): i32 {
    let s: String = stringOf("ab");
    let r: raw String = unsafe(&s);
    let out: String = f<String>(r);
    return i32(len(out));
}
"#;
    assert_rejected("dbc29", source, "E10034");
}

/// A projection base evaluates before its index.
///
/// `mkString` sets the global `idx` to one, and both readers select
/// `mk()[idx]`: the base call runs first, so the take and the store each
/// receive the second row element and measure three bytes (`33`).
#[test]
fn should_evaluate_the_base_before_the_index_when_a_projection_is_read() {
    let source = r#"module spec.dbc.st30;

import { String, stringOf } from std.text;

let idx: i32 = 0;

func mkString(): String[2] {
    idx = 1;
    return [stringOf("ab"), stringOf("cde")];
}

func takeFirst<T>(take s: T): T {
    return s;
}

func viaTake<T>(mk: func(): T[2]): T {
    return takeFirst<T>(mk()[idx]);
}

func viaStore<T>(mk: func(): T[2]): T {
    let s: T = mk()[idx];
    return s;
}

func main(): i32 {
    return i32(len(viaTake<String>(mkString))) * 10 + i32(len(viaStore<String>(mkString)));
}
"#;
    let output = run_memory("dbc30", source);
    assert_exit("dbc30", &output, 33);
}
