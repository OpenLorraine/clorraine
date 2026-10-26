//! Runnable specification tests for the raw and conditional store/alias batch.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, and runs the binary. The memory cases run under AddressSanitizer
//! when `RDC_ASAN` is set and as ordinary strict builds otherwise, so a double
//! free still aborts and a leak is visible in the sanitizer run.
//!
//! The batch pins the store and alias rules that are decided from the address
//! used at run time:
//!
//! * a guarded struct-literal argument or receiver moves a raw-deref owning
//!   field instead of copying it;
//! * a conditional fixed-array assignment captures a take of its destination;
//! * the alias test covers subobject containment in both directions;
//! * a nested materialization keeps every live take capture;
//! * a raw self-alias restores the save instead of emptying the destination;
//! * a fixed-array parameter source keeps its adjusted address;
//! * an intermediate raw field is pinned before a side-effecting RHS;
//! * a post-store source read is rejected while the destination read stays
//!   accepted;
//! * a duplicable fixed-array self-read evaluates the RHS before the clear;
//! * an element store tests the run-time source/destination alias;
//! * a skipped `||` right does not zero the save.
//!
//! All temporary files live in a self-removing scratch directory.

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
            "clc-rawstorespec-{}-{tag}-{sequence}",
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

/// Asserts the front end rejected `source` with exactly one total error
/// carrying `code`, and that no translation unit was written.
fn assert_rejected_exactly_once(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "{tag}: expected exactly one error; stderr:\n{stderr}"
    );
    assert!(
        errors[0].contains(code),
        "{tag}: expected the {code} report; saw `{}`; stderr:\n{stderr}",
        errors[0]
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

/// Emits one host-profile program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty"
    );
    emitted
}

/// Compiles the emitted C with the host compiler under the strict flag set.
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
            "-no-pie",
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

/// Runs a memory-safety case under AddressSanitizer when `RDC_ASAN` is set,
/// otherwise as an ordinary strict host build.
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
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Guarded struct-literal raw move
// ---------------------------------------------------------------------------

/// A guarded call argument moves a raw-deref owning field and empties the
/// pointee, so the callee's release and the pointee's release cannot free the
/// same string twice.
#[test]
fn should_move_a_raw_deref_field_into_a_guarded_call_argument() {
    let source = r#"module spec.rawstore.guardedarg;

import { String, stringOf } from std.text;

struct H { s: String; }

func consume(take h: H): i32 {
    return i32(len(h.s));
}

func main(): i32 {
    let h: H = H { s: stringOf("aa") };
    let g: bool = true;
    unsafe {
        let p: raw H = &h;
        if (g && consume(H { s: (*p).s }) == 2) {
            return 0;
        }
    }
    return 9;
}
"#;
    let output = run_memory("guardedarg", source);
    assert_no_asan("guardedarg", &output);
    assert_exit("guardedarg", &output, 0);
}

/// A skipped guard leaves the pointee readable and leaks nothing.
#[test]
fn should_keep_the_pointee_readable_when_the_guarded_literal_is_skipped() {
    let source = r#"module spec.rawstore.guardedargskip;

import { String, stringOf } from std.text;

struct H { s: String; }

func consume(take h: H): i32 {
    return i32(len(h.s));
}

func main(): i32 {
    let h: H = H { s: stringOf("aa") };
    let g: bool = false;
    unsafe {
        let p: raw H = &h;
        if (g && consume(H { s: (*p).s }) == 2) {
            return 9;
        }
        return i32(len((*p).s)) - 2;
    }
}
"#;
    let output = run_memory("guardedargskip", source);
    assert_no_asan("guardedargskip", &output);
    assert_exit("guardedargskip", &output, 0);
}

// ---------------------------------------------------------------------------
// Conditional fixed-array take capture
// ---------------------------------------------------------------------------

/// A conditional fixed-array assignment captures a take of its destination,
/// so the saved old array is not released after the taken elements are freed.
#[test]
fn should_capture_a_take_of_a_conditional_fixed_array_destination() {
    let source = r#"module spec.rawstore.arraytake;

import { String, stringOf } from std.text;

struct A { items: String[2]; }

func eatRows(take rows: String[2]): i32 {
    return i32(len(rows[0])) - 2;
}

func main(): i32 {
    let a: A = A { items: [stringOf("aa"), stringOf("bb")] };
    let x: String[2] = [stringOf("xx"), stringOf("yy")];
    let y: String[2] = [stringOf("mm"), stringOf("nn")];
    let g: bool = true;
    unsafe {
        let p: raw A = &a;
        a.items = (g && eatRows(take (*p).items) == 0) ? x : y;
    }
    if (i32(len(a.items[0])) != 2) {
        return 9;
    }
    return 0;
}
"#;
    let output = run_memory("arraytake", source);
    assert_no_asan("arraytake", &output);
    assert_exit("arraytake", &output, 0);
}

// ---------------------------------------------------------------------------
// Nested take-capture stack
// ---------------------------------------------------------------------------

/// A nested conditional take materialized as a call argument keeps the outer
/// capture alive, so the outer save is not released twice.
#[test]
fn should_keep_the_outer_take_capture_through_a_nested_argument() {
    let source = r#"module spec.rawstore.nestedarg;

import { String, stringOf } from std.text;

func inner(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let z: String = stringOf("zzzzzz");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = (g && inner(take (g ? (*p) : z)) == 2) ? x : y;
    }
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("nestedarg", source);
    assert_no_asan("nestedarg", &output);
    assert_exit("nestedarg", &output, 0);
}

// ---------------------------------------------------------------------------
// Raw self-alias save restore
// ---------------------------------------------------------------------------

/// A raw self-alias restores the save into the destination instead of
/// emptying the destination through the pointee.
#[test]
fn should_restore_the_save_when_a_raw_pointer_aliases_the_destination() {
    let source = r#"module spec.rawstore.selfalias;

import { String, stringOf } from std.text;

func main(): i32 {
    let v: String = stringOf("seven77");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = g ? (*p) : y;
        return i32(len(v));
    }
}
"#;
    let output = run_memory("selfalias", source);
    assert_no_asan("selfalias", &output);
    assert_exit("selfalias", &output, 7);
}

/// The member twin restores the save of a raw self-alias field.
#[test]
fn should_restore_the_save_of_a_raw_self_alias_member() {
    let source = r#"module spec.rawstore.selfaliasmember;

import { String, stringOf } from std.text;

struct Box { name: String; }

func main(): i32 {
    let a: Box = Box { name: stringOf("seven77") };
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw Box = &a;
        a.name = g ? (*p).name : y;
        return i32(len(a.name));
    }
}
"#;
    let output = run_memory("selfaliasmember", source);
    assert_no_asan("selfaliasmember", &output);
    assert_exit("selfaliasmember", &output, 7);
}

// ---------------------------------------------------------------------------
// Skipped short-circuit take
// ---------------------------------------------------------------------------

/// A skipped `||` right does not zero the save, so the old value is released
/// exactly once by the conditional store.
#[test]
fn should_keep_the_save_when_the_short_circuit_skips_the_take() {
    let source = r#"module spec.rawstore.shortcircuit;

import { String, stringOf } from std.text;

func eat(take v: String): i32 {
    return 2;
}

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    v = (g || eat(take v) == 2) ? x : y;
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("shortcircuit", source);
    assert_no_asan("shortcircuit", &output);
    assert_exit("shortcircuit", &output, 0);
}

/// The false twin runs the take; the taken value is released by the callee and
/// the save is zeroed, so the store does not free it again.
#[test]
fn should_zero_the_save_when_the_short_circuit_runs_the_take() {
    let source = r#"module spec.rawstore.shortcircuittaken;

import { String, stringOf } from std.text;

func eat(take v: String): i32 {
    return 2;
}

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = false;
    v = (g || eat(take v) == 2) ? x : y;
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("shortcircuittaken", source);
    assert_no_asan("shortcircuittaken", &output);
    assert_exit("shortcircuittaken", &output, 0);
}

// ---------------------------------------------------------------------------
// Source/destination containment
// ---------------------------------------------------------------------------

/// A take of a super-object into a member destination clears the consumed
/// subobject, so the old member is not released a second time.
#[test]
fn should_contain_a_super_object_take_into_a_member_destination() {
    let source = r#"module spec.rawstore.containsup;

import { String, stringOf } from std.text;

struct Elem { s: String; }
struct Pair { head: i32; tail: Elem; }

func eatPair(take p: Pair): i32 {
    return 1;
}

func main(): i32 {
    let pair: Pair = Pair { head: 7, tail: Elem { s: stringOf("aa") } };
    let x: Elem = Elem { s: stringOf("xx") };
    let y: Elem = Elem { s: stringOf("yy") };
    let g: bool = true;
    unsafe {
        let p: raw Pair = &pair;
        pair.tail = (g && eatPair(take (*p)) == 1) ? x : y;
    }
    if (pair.head != 7) {
        return 9;
    }
    if (i32(len(pair.tail.s)) != 2) {
        return 8;
    }
    return 0;
}
"#;
    let output = run_memory("containsup", source);
    assert_no_asan("containsup", &output);
    assert_exit("containsup", &output, 0);
}

/// A take of a first member into a whole-struct destination clears only the
/// taken member of the save, so the un-taken owning sibling is released.
#[test]
fn should_contain_a_first_member_take_into_a_whole_struct_destination() {
    let source = r#"module spec.rawstore.containfirst;

import { String, stringOf } from std.text;

struct Pair { first: String; sibling: String; }

func eat(take s: String): i32 {
    return 1;
}

func main(): i32 {
    let pair: Pair = Pair { first: stringOf("aa"), sibling: stringOf("77") };
    let x: Pair = Pair { first: stringOf("xx"), sibling: stringOf("88") };
    let y: Pair = Pair { first: stringOf("yy"), sibling: stringOf("99") };
    let g: bool = true;
    unsafe {
        let p: raw Pair = &pair;
        pair = (g && eat(take (*p).first) == 1) ? x : y;
    }
    if (i32(len(pair.first)) != 2) {
        return 9;
    }
    if (i32(len(pair.sibling)) != 2) {
        return 8;
    }
    return 0;
}
"#;
    let output = run_memory("containfirst", source);
    assert_no_asan("containfirst", &output);
    assert_exit("containfirst", &output, 0);
}

// ---------------------------------------------------------------------------
// Fixed-array parameter alias guard
// ---------------------------------------------------------------------------

/// A take fixed-array parameter that aliases the raw store target keeps the
/// array address stable through the copy.
#[test]
fn should_guard_a_fixed_array_parameter_that_aliases_the_raw_target() {
    let source = r#"module spec.rawstore.arrayparam;

import { String, stringOf } from std.text;

struct S { items: String[2]; }

func copyInto<T>(p: raw T, take items: String[2]): i32 {
    unsafe {
        (*p).items = items;
        return i32(len((*p).items[0])) - 2;
    }
}

func main(): i32 {
    let a: S = S { items: [stringOf("aa"), stringOf("bb")] };
    unsafe {
        return copyInto<S>(&a, a.items);
    }
}
"#;
    let output = run_memory("arrayparam", source);
    assert_no_asan("arrayparam", &output);
    assert_exit("arrayparam", &output, 0);
}

/// The concrete fixed-array parameter alias keeps the array readable and
/// builds under the strict C flags.
#[test]
fn should_build_the_concrete_fixed_array_parameter_alias() {
    let source = r#"module spec.rawstore.arrayaddr;

import { String, stringOf } from std.text;

struct S { items: String[2]; }

func copyRaw(p: raw S, take items: String[2]): i32 {
    unsafe {
        (*p).items = items;
        return i32(len((*p).items[0])) - 2;
    }
}

func main(): i32 {
    let a: S = S { items: [stringOf("aa"), stringOf("bb")] };
    unsafe {
        return copyRaw(&a, a.items);
    }
}
"#;
    let output = run_memory("arrayaddr", source);
    assert_no_asan("arrayaddr", &output);
    assert_exit("arrayaddr", &output, 0);
}

// ---------------------------------------------------------------------------
// Intermediate raw-field pin
// ---------------------------------------------------------------------------

/// The intermediate raw field is pinned before a side-effecting RHS, so the
/// store reaches the pre-call field.
#[test]
fn should_pin_an_intermediate_raw_field_before_a_side_effecting_rhs() {
    let source = r#"module spec.rawstore.fieldpin;

struct Inner { x: i32; }
struct Outer { next: Inner; }

let calls: i32 = 0;

func change(p: raw Outer): i32 {
    unsafe {
        (*p).next = Inner { x: 9 };
    }
    calls = calls + 1;
    return 3;
}

func main(): i32 {
    let a: Outer = Outer { next: Inner { x: 0 } };
    unsafe {
        let p: raw Outer = &a;
        (*p).next.x = change(p);
    }
    if (a.next.x != 3) {
        return 9;
    }
    if (calls != 1) {
        return 8;
    }
    return a.next.x;
}
"#;
    let output = run_memory("fieldpin", source);
    assert_no_asan("fieldpin", &output);
    assert_exit("fieldpin", &output, 3);
}

// ---------------------------------------------------------------------------
// Post-store source read
// ---------------------------------------------------------------------------

/// The later read of the run-time source element is rejected once, while no
/// translation unit is written.
#[test]
fn should_reject_the_post_conditional_store_source_read() {
    let source = r#"module spec.rawstore.postsrc;

import { String, stringOf } from std.text;

struct Elem { name: String; }

let idx: i32 = 0;

func set_idx(): bool {
    idx = 1;
    return true;
}

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    a[idx] = set_idx() ? a[idx] : Elem { name: stringOf("zz") };
    let s: str = a[1].name;
    return i32(len(s)) - 2;
}
"#;
    assert_rejected_exactly_once("postsrc", source, "E10020", &["a[1]"]);
}

/// The destination read stays accepted after the pinned-index store.
#[test]
fn should_accept_the_post_conditional_store_destination_read() {
    let source = r#"module spec.rawstore.postdst;

import { String, stringOf } from std.text;

struct Elem { name: String; }

let idx: i32 = 0;

func set_idx(): bool {
    idx = 1;
    return true;
}

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    a[idx] = set_idx() ? a[idx] : Elem { name: stringOf("zz") };
    let s: str = a[0].name;
    if (i32(s[0]) != 98) {
        return 9;
    }
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("postdst", source);
    assert_no_asan("postdst", &output);
    assert_exit("postdst", &output, 0);
}

// ---------------------------------------------------------------------------
// Duplicable fixed-array self-read
// ---------------------------------------------------------------------------

/// A fixed-array literal reads the destination before the clear.
#[test]
fn should_evaluate_a_fixed_array_self_read_before_the_clear() {
    let source = r#"module spec.rawstore.selfread;

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [3, 4]];
    a[0] = [a[0][1], a[0][0]];
    if (a[0][0] != 2 || a[0][1] != 1) {
        return 9;
    }
    return 0;
}
"#;
    let output = run_memory("selfread", source);
    assert_no_asan("selfread", &output);
    assert_exit("selfread", &output, 0);
}

/// The fill twin evaluates its expression before the destination is cleared.
#[test]
fn should_evaluate_a_fixed_array_fill_self_read_before_the_clear() {
    let source = r#"module spec.rawstore.selffill;

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [3, 4]];
    a[0] = [; a[0][0] + a[0][1]];
    if (a[0][0] != 3 || a[0][1] != 3) {
        return 9;
    }
    return 0;
}
"#;
    let output = run_memory("selffill", source);
    assert_no_asan("selffill", &output);
    assert_exit("selffill", &output, 0);
}

// ---------------------------------------------------------------------------
// Runtime element alias
// ---------------------------------------------------------------------------

/// The element store tests the run-time source/destination address and keeps
/// the value when the selected pointer aliases the destination.
#[test]
fn should_keep_the_value_when_a_raw_element_source_aliases_the_destination() {
    let source = r#"module spec.rawstore.elementalias;

import { String, stringOf } from std.text;

let choose: bool = true;
let calls: i32 = 0;

func alt(s: raw String, t: raw String): raw String {
    calls = calls + 1;
    if (choose) {
        return s;
    }
    return t;
}

func main(): i32 {
    let xs: String[2] = [stringOf("aa"), stringOf("bb")];
    let other: String[2] = [stringOf("cc"), stringOf("dd")];
    unsafe {
        let p: raw String = &xs[0];
        let q: raw String = &other[0];
        xs[0] = *alt(p, q);
    }
    if (i32(len(xs[0])) != 2) {
        return 9;
    }
    if (calls != 1) {
        return 8;
    }
    return 0;
}
"#;
    let output = run_memory("elementalias", source);
    assert_no_asan("elementalias", &output);
    assert_exit("elementalias", &output, 0);
}

/// The non-aliasing element source still moves into the destination.
#[test]
fn should_move_a_non_aliasing_raw_element_source() {
    let source = r#"module spec.rawstore.elementmove;

import { String, stringOf } from std.text;

let choose: bool = false;

func alt(s: raw String, t: raw String): raw String {
    if (choose) {
        return s;
    }
    return t;
}

func main(): i32 {
    let xs: String[2] = [stringOf("aa"), stringOf("bb")];
    let other: String[2] = [stringOf("cc"), stringOf("dd")];
    unsafe {
        let p: raw String = &xs[0];
        let q: raw String = &other[0];
        xs[0] = *alt(p, q);
    }
    if (i32(len(xs[0])) != 2) {
        return 9;
    }
    if (i32(len(other[0])) != 0) {
        return 8;
    }
    return 0;
}
"#;
    let output = run_memory("elementmove", source);
    assert_no_asan("elementmove", &output);
    assert_exit("elementmove", &output, 0);
}
