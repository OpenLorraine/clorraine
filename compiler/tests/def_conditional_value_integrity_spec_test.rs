//! Runnable specification tests for the conditional materialization batch.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! and runs the binary. The memory cases run under AddressSanitizer when
//! `RDC_ASAN` is set and as ordinary strict builds otherwise, so a leak shows in
//! the sanitizer run and a bad release still aborts.
//!
//! The batch pins the conditional read-base, nested-literal, maybe-target,
//! deferred-take, and fresh-root rules:
//!
//! * a generic field-projected take later move reports one `E10020`;
//! * a hidden generic deref-chain division emits the `E11210` guard;
//! * a concrete lend moved into a maybe-owning conditional is rejected while
//!   the view target copies;
//! * a concrete local moved on a maybe-target path reports `E10020` at the
//!   later use while the view target copies;
//! * a mixed place/literal conditional read base binds a per-arm destination;
//! * a guarded duplicable literal read base stores the selected arm only;
//! * nested conditional arms recurse in literal stores and guarded takes;
//! * a guarded duplicable struct conditional take defers to the evaluated path;
//! * nested literal fields evaluate in source order, not declaration order;
//! * a fresh conditional root releases its unselected fields;
//! * a selected call arm's root is released exactly once.
//!
//! All temporary files live in a self-removing scratch directory.

use std::os::unix::process::ExitStatusExt;
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
            "clc-condvalspec-{}-{tag}-{sequence}",
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

/// Asserts the front end rejected `source` with exactly one error carrying
/// `code`, and that no translation unit was written.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str]) {
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
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
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

/// Asserts the run produced no address or leak sanitizer report.
fn assert_no_sanitizer(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    for marker in ["AddressSanitizer", "LeakSanitizer"] {
        assert!(
            !stderr.contains(marker),
            "{tag}: the run reported a memory error; stderr:\n{stderr}"
        );
    }
}

/// Asserts a runtime fault printed `expected` first and did not die of SIGFPE.
///
/// The host runtime aborts after the fault line, so the status is a signal; the
/// contract is the exact first line and the absence of an arithmetic trap.
fn assert_trap(tag: &str, output: &Output, expected: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first = stderr.lines().next().unwrap_or_default();
    assert_eq!(
        first, expected,
        "{tag}: unexpected first stderr line; full stderr:\n{stderr}"
    );
    assert_ne!(
        output.status.signal(),
        Some(8),
        "{tag}: the program died of SIGFPE instead of the runtime guard"
    );
    assert!(
        !output.status.success(),
        "{tag}: a runtime fault must exit nonzero"
    );
}

// ---------------------------------------------------------------------------
// DEF-470: the stale generic field-projected take later move
// ---------------------------------------------------------------------------

/// A generic field-projected take later whole use reports one `E10020`.
///
/// The row was closed stale before the batch; this is the green HEAD pin that
/// keeps the diagnostic in place while the rest of the batch changes ownership.
#[test]
fn should_reject_the_stale_generic_field_projected_take() {
    let source = r#"module spec.condval.stale470;

import { String, stringOf } from std.text;

struct H { name: String; }

func go(c: bool, take h: H): i32 {
    let r: String? = c ? h.name : null;
    let s: String = h.name;
    return i32(len(s));
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return go(true, take h);
}
"#;
    assert_rejected_once("stale470", source, "E10020", &["h.name"]);
}

// ---------------------------------------------------------------------------
// DEF-471: the hidden generic deref-chain division
// ---------------------------------------------------------------------------

/// A division through a raw-deref generic field emits the `E11210` guard.
///
/// The guard sees the pointee type through the deref chain; a missing arm
/// leaves an unknown type and the run dies of SIGFPE instead.
#[test]
fn should_guard_a_generic_deref_chain_division() {
    let source = r#"module spec.condval.div471;

struct Inner { n: i32; }
struct H { p: raw Inner; }

func go<T>(h: T, d: i32): void {
    unsafe { (*h.p).n /= d; }
}

func main(): i32 {
    let inner: Inner = Inner { n: 1 };
    unsafe {
        let h: H = H { p: &inner };
        go<H>(h, 0);
    }
    return 0;
}
"#;
    let output = run_host("div471", source);
    assert_trap(
        "div471",
        &output,
        "error E11210: integer division by zero",
    );
}

// ---------------------------------------------------------------------------
// DEF-472: the concrete lend in a maybe-target conditional
// ---------------------------------------------------------------------------

/// A concrete lend moved into an owning maybe-target instance reports one
/// `E10034` naming the lend; the view target control copies instead.
#[test]
fn should_reject_a_concrete_lend_maybe_target_move() {
    let owning = r#"module spec.condval.lend472;

import { String, stringOf } from std.text;

func pick<T>(c: bool, take h: T, s: String): i32 {
    let r: T = c ? h : s;
    return 0;
}

func main(): i32 {
    let h: String = stringOf("aa");
    let s: String = stringOf("bb");
    return pick<String>(true, take h, s);
}
"#;
    assert_rejected_once("lend472", owning, "E10034", &["s"]);
}

/// The `T = str` maybe-target control copies the view and runs clean.
#[test]
fn should_accept_the_view_maybe_target_control() {
    let source = r#"module spec.condval.lend472str;

import { String, stringOf } from std.text;

func pick<T>(c: bool, take h: T, s: String): i32 {
    let r: T = c ? h : s;
    return 0;
}

func main(): i32 {
    let h: str = "aa";
    let s: String = stringOf("bb");
    return pick<str>(true, take h, s);
}
"#;
    let output = run_memory("lend472str", source);
    assert_no_sanitizer("lend472str", &output);
    assert_exit("lend472str", &output, 0);
}

// ---------------------------------------------------------------------------
// DEF-473: the maybe-target take and later concrete local use
// ---------------------------------------------------------------------------

/// A concrete local moved on a maybe-target path reports `E10020` at its later
/// use; the view target control keeps the local usable.
#[test]
fn should_reject_a_maybe_target_later_use_of_a_moved_local() {
    let owning = r#"module spec.condval.move473;

import { String, stringOf } from std.text;

func pick<T>(c: bool, take h: T): i32 {
    let s: String = stringOf("cd");
    let r: T = c ? h : s;
    let n: i32 = i32(len(s));
    return n - 2;
}

func main(): i32 {
    let h: String = stringOf("ab");
    return pick<String>(true, take h);
}
"#;
    assert_rejected_once("move473", owning, "E10020", &["s"]);
}

/// The `T = str` maybe-target control keeps the moved local usable.
#[test]
fn should_accept_the_view_maybe_target_later_use_control() {
    let source = r#"module spec.condval.move473str;

import { String, stringOf } from std.text;

func pick<T>(c: bool, take h: T): i32 {
    let s: String = stringOf("cd");
    let r: T = c ? h : s;
    let n: i32 = i32(len(s));
    return n - 2;
}

func main(): i32 {
    let h: str = "ab";
    return pick<str>(true, take h);
}
"#;
    let output = run_memory("move473str", source);
    assert_no_sanitizer("move473str", &output);
    assert_exit("move473str", &output, 0);
}

// ---------------------------------------------------------------------------
// DEF-475: the owned mixed conditional read base
// ---------------------------------------------------------------------------

/// A mixed place/literal conditional read base binds a per-arm destination and
/// releases the selected literal's owners.
#[test]
fn should_bind_the_mixed_conditional_read_base() {
    let source = r#"module spec.condval.mixedbase475;

import { String, stringOf } from std.text;

struct Inner { name: String; row: i32[1][1]; tag: String; }

func n2(): i32 { return 3; }

func main(): i32 {
    let other: Inner = Inner { name: stringOf("cc"), row: [[4]], tag: stringOf("dd") };
    let ok: bool = false;
    return (ok ? other : Inner { name: stringOf("aa"), row: [[n2()]], tag: stringOf("bb") }).row[0][0];
}
"#;
    let output = run_memory("mixedbase475", source);
    assert_no_sanitizer("mixedbase475", &output);
    assert_exit("mixedbase475", &output, 3);
}

// ---------------------------------------------------------------------------
// DEF-476: the guarded duplicable literal read base
// ---------------------------------------------------------------------------

/// A guarded duplicable conditional literal read base stores the selected
/// array-returning call arm and leaves the other arm uncalled.
#[test]
fn should_store_the_guarded_duplicable_literal_per_arm() {
    let source = r#"module spec.condval.dupliteral476;

struct Inner { row: i32[1][1]; }

let calls: i32 = 0;

func n1(): i32[1][1] { calls = calls + 1; return [[1]]; }
func n2(): i32[1][1] { calls = calls + 1; return [[3]]; }

func main(): i32 {
    let g: bool = true;
    let ok: bool = false;
    let v: i32 = g ? i32((ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]) : 0;
    if (calls != 1) { return 9; }
    return v;
}
"#;
    let output = run_host("dupliteral476", source);
    assert_exit("dupliteral476", &output, 3);
}

// ---------------------------------------------------------------------------
// DEF-477: nested conditional arms
// ---------------------------------------------------------------------------

/// A conditional arm that is itself a conditional recurses in a literal store
/// and a guarded take; only the selected arm runs.
#[test]
fn should_recurse_into_nested_conditional_arms() {
    let source = r#"module spec.condval.nested477;

import { String, stringOf } from std.text;

struct Inner { row: i32[1][1]; }
struct O { names: String; }

let calls: i32 = 0;
let last: i32 = 0;

func row1(): i32[1][1] { calls = calls + 1; last = 1; return [[1]]; }
func row2(): i32[1][1] { calls = calls + 1; last = 2; return [[2]]; }
func row3(): i32[1][1] { calls = calls + 1; last = 3; return [[3]]; }
func name1(): String { calls = calls + 1; last = 1; return stringOf("a"); }
func name2(): String { calls = calls + 1; last = 2; return stringOf("bb"); }
func name3(): String { calls = calls + 1; last = 3; return stringOf("ccc"); }

func consumeT(take o: O): i32 { return 0; }

func main(): i32 {
    let ok1: bool = false;
    let ok2: bool = true;
    let o: Inner = ok1 ? Inner { row: row1() } : (ok2 ? Inner { row: row2() } : Inner { row: row3() });
    if (calls != 1) { return 9; }
    if (last != 2) { return 8; }
    let r: i32 = consumeT(take (ok1 ? O { names: name1() } : (ok2 ? O { names: name2() } : O { names: name3() })));
    if (calls != 2) { return 7; }
    if (last != 2) { return 6; }
    return 0;
}
"#;
    let output = run_host("nested477", source);
    assert_exit("nested477", &output, 0);
}

// ---------------------------------------------------------------------------
// DEF-478: the guarded duplicable struct conditional take
// ---------------------------------------------------------------------------

/// A duplicable struct conditional take inside a guarded operand evaluates only
/// on the selected path; both twins keep the trace exact.
#[test]
fn should_defer_a_guarded_duplicable_take() {
    let taken = r#"module spec.condval.deferred478;

struct O { names: static str; }

let calls: i32 = 0;

func n1(): static str { calls = calls + 1; return "aa"; }
func n2(): static str { calls = calls + 1; return "b"; }

func consumeT(take o: O): i32 { return i32(len(o.names)); }

func main(): i32 {
    let g: bool = true;
    let ok: bool = true;
    let r: i32 = g ? consumeT(take (ok ? O { names: n1() } : O { names: n2() })) : 0;
    if (calls != 1) { return 9; }
    return r;
}
"#;
    let output = run_memory("deferred478", taken);
    assert_no_sanitizer("deferred478", &output);
    assert_exit("deferred478", &output, 2);

    let skipped = r#"module spec.condval.deferred478skip;

struct O { names: static str; }

let calls: i32 = 0;

func n1(): static str { calls = calls + 1; return "aa"; }
func n2(): static str { calls = calls + 1; return "b"; }

func consumeT(take o: O): i32 { return i32(len(o.names)); }

func main(): i32 {
    let g: bool = false;
    let ok: bool = true;
    let r: i32 = g ? consumeT(take (ok ? O { names: n1() } : O { names: n2() })) : 0;
    if (calls != 0) { return 9; }
    return r;
}
"#;
    let output = run_memory("deferred478skip", skipped);
    assert_no_sanitizer("deferred478skip", &output);
    assert_exit("deferred478skip", &output, 0);
}

// ---------------------------------------------------------------------------
// DEF-479: nested literal field source order
// ---------------------------------------------------------------------------

/// Nested literal fields evaluate in source order even when the struct
/// declaration order differs; the owning and scalar twins pin both paths.
#[test]
fn should_emit_nested_literal_fields_in_source_order() {
    let owning = r#"module spec.condval.order479a;

import { String, stringOf } from std.text;

struct Inner { names: String; }
struct Outer { inner: Inner; tag: i32; }

let trace: i32 = 0;

func tagOf(): i32 { trace = trace * 10 + 1; return 7; }
func n1(): String { trace = trace * 10 + 2; return stringOf("aa"); }

func main(): i32 {
    let g: bool = true;
    let other: Outer = Outer { inner: Inner { names: stringOf("other") }, tag: 0 };
    let o: Outer = g ? Outer { tag: tagOf(), inner: Inner { names: n1() } } : other;
    if (trace != 12) { return 9; }
    return 0;
}
"#;
    let output = run_host("order479a", owning);
    assert_exit("order479a", &output, 0);

    let scalar = r#"module spec.condval.order479b;

struct Inner { n: i32; }
struct Outer { inner: Inner; tag: i32; }

let trace: i32 = 0;

func tagOf(): i32 { trace = trace * 10 + 1; return 7; }
func n2(): i32 { trace = trace * 10 + 3; return 2; }

func main(): i32 {
    let g: bool = true;
    let other: Outer = Outer { inner: Inner { n: 0 }, tag: 0 };
    let o: Outer = g ? Outer { tag: tagOf(), inner: Inner { n: n2() } } : other;
    if (trace != 13) { return 9; }
    return 0;
}
"#;
    let output = run_host("order479b", scalar);
    assert_exit("order479b", &output, 0);
}

// ---------------------------------------------------------------------------
// DEF-480: the fresh conditional root
// ---------------------------------------------------------------------------

/// A fresh conditional root releases the selected projection's remaining
/// owners; the literal arm and the call-projected arm are both pinned.
#[test]
fn should_bind_and_release_the_fresh_conditional_root() {
    let literal = r#"module spec.condval.freshroot480;

import { String, stringOf } from std.text;

struct Inner { name: String; row: i32[1][1]; }
struct Holder { inner: Inner; tag: String; }

func mk(): Holder { return Holder { inner: Inner { name: stringOf("bb"), row: [[4]] }, tag: stringOf("tt") }; }

func main(): i32 {
    let ok: bool = true;
    return (ok ? Inner { name: stringOf("aa"), row: [[3]] } : mk().inner).row[0][0];
}
"#;
    let output = run_memory("freshroot480", literal);
    assert_no_sanitizer("freshroot480", &output);
    assert_exit("freshroot480", &output, 3);

    let call = r#"module spec.condval.freshroot480b;

import { String, stringOf } from std.text;

struct Inner { name: String; row: i32[1][1]; }
struct Holder { inner: Inner; tag: String; }

func mk(): Holder { return Holder { inner: Inner { name: stringOf("bb"), row: [[4]] }, tag: stringOf("tt") }; }

func main(): i32 {
    let ok: bool = false;
    return (ok ? Inner { name: stringOf("aa"), row: [[3]] } : mk().inner).row[0][0];
}
"#;
    let output = run_memory("freshroot480b", call);
    assert_no_sanitizer("freshroot480b", &output);
    assert_exit("freshroot480b", &output, 4);
}

// ---------------------------------------------------------------------------
// DEF-481: the conditional call-arm root
// ---------------------------------------------------------------------------

/// A conditional read base over owning call arms releases the selected call
/// root exactly once; the view read stays valid.
#[test]
fn should_bind_and_release_a_conditional_call_arm() {
    let source = r#"module spec.condval.callroot481;

import { String, stringOf } from std.text;

struct S { name: String; tag: static str; extra: String; }

func mkA(): S { return S { name: stringOf("aa"), tag: "aa", extra: stringOf("xx") }; }
func mkB(): S { return S { name: stringOf("bb"), tag: "bb", extra: stringOf("yy") }; }

func main(): i32 {
    let ok: bool = true;
    unsafe {
        let p = (ok ? mkA() : mkB()).tag.ptr;
        if (p == null) { return 9; }
    }
    return 0;
}
"#;
    let output = run_memory("callroot481", source);
    assert_no_sanitizer("callroot481", &output);
    assert_exit("callroot481", &output, 0);
}

// ---------------------------------------------------------------------------
// Conditional place arm
// ---------------------------------------------------------------------------

/// A guarded conditional that selects a duplicable place arm copies it and
/// leaves the source usable; the concrete twin copies.
#[test]
fn should_copy_a_duplicable_conditional_place_arm() {
    let source = r#"module spec.condval.placearm475b;

struct Inner { row: i32[1][1]; }

func n1(): i32[1][1] { return [[3]]; }

func main(): i32 {
    let other: Inner = Inner { row: [[4]] };
    let g: bool = true;
    let ok: bool = true;
    let v: i32 = g ? ((ok ? other : Inner { row: n1() }).row[0][0]) : 0;
    if (other.row[0][0] != 4) { return 9; }
    return v;
}
"#;
    let output = run_memory("placearm475b", source);
    assert_no_sanitizer("placearm475b", &output);
    assert_exit("placearm475b", &output, 4);
}

// ---------------------------------------------------------------------------
// Destination generations
// ---------------------------------------------------------------------------

/// A later owning maybe-target move keeps the moved-value judgment even when
/// an earlier move of the same source selected a view destination.
#[test]
fn should_judge_a_maybe_move_at_its_destination_generation() {
    let two_store = r#"module spec.condval.twostoremin;

import { String, stringOf } from std.text;

func pick<T, U>(c: bool, take u: U, take s: T): i32 {
    let q: U = c ? u : s;
    let r: T = s;
    let n: i32 = i32(len(s));
    return n - 2;
}

func main(): i32 {
    let u: str = "zz";
    let s: String = stringOf("cd");
    return pick<String, str>(false, take u, take s);
}
"#;
    assert_rejected_once("twostoremin", two_store, "E10020", &["s"]);

    let take_view = r#"module spec.condval.takeafterview;

import { String, stringOf } from std.text;

func pass<V>(take v: V): void { }

func pick<T, U>(c: bool, take u: U, take s: T): i32 {
    let q: U = c ? u : s;
    pass<T>(take s);
    let n: i32 = i32(len(s));
    return n - 2;
}

func main(): i32 {
    let u: str = "zz";
    let s: String = stringOf("cd");
    return pick<String, str>(false, take u, take s);
}
"#;
    assert_rejected_once("takeafterview", take_view, "E10020", &["s"]);

    let dedupe = r#"module spec.condval.dedupe2;

import { String, stringOf } from std.text;

func pick<T, U>(c: bool, c2: bool, take h: T, take h2: U): i32 {
    let s: String = stringOf("cd");
    let a: U = c2 ? h2 : s;
    let n1: i32 = i32(len(s));
    let b: T = c ? h : s;
    let n2: i32 = i32(len(s));
    return n1 + n2 - 4;
}

func main(): i32 {
    let h: String = stringOf("aa");
    let h2: str = "ee";
    return pick<String, str>(true, true, take h, take h2);
}
"#;
    assert_rejected_once("dedupe2", dedupe, "E10020", &["s"]);
}

/// A re-initialized source drops the old view destination, so its later use is
/// accepted.
#[test]
fn should_drop_a_stale_destination_on_reinitialization() {
    let source = r#"module spec.condval.staledest;

import { String, stringOf } from std.text;

func pick<T, U>(c: bool, c2: bool, take h: T, take h2: U): i32 {
    let s: String = stringOf("cd");
    let a: U = c2 ? h2 : s;
    s = stringOf("ef");
    let b: T = c ? h : s;
    let n: i32 = i32(len(s));
    return n - 2;
}

func main(): i32 {
    let h: str = "aa";
    let h2: String = stringOf("bb");
    return pick<str, String>(true, true, take h, take h2);
}
"#;
    let output = run_memory("staledest", source);
    assert_no_sanitizer("staledest", &output);
    assert_exit("staledest", &output, 0);
}
