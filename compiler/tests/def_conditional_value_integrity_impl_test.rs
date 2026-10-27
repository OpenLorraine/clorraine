//! Implementation tests for the conditional materialization batch.
//!
//! The cases pin the emitted per-arm stores, deferred take and literal shapes,
//! source-order field stores, fresh/call-root release counts, the division
//! guard, and the one-report diagnostics. Each case emits a small Clorraine
//! program with the built `clc` binary, reads the generated C, and builds it
//! with the strict host flags so an accepted program cannot emit invalid C.
//! The determinism case re-emits accepted programs and compares the units byte
//! for byte.
//!
//! All temporary files live in a self-removing scratch directory.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one emitted program may run before the case fails as a hang.
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
            "clc-condvalimpl-{}-{tag}-{sequence}",
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

/// Writes `source` into the scratch directory and returns its path.
fn write_source(tag: &str, scratch: &ScratchDir, source: &str) -> PathBuf {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    source_path
}

/// Compiles `source_path` into `c_path` and returns the emitted text.
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

/// Emits one host-profile program and asserts the front end accepted it.
///
/// The scratch directory is returned so it outlives the emitted text and can
/// host the strict C build.
fn emit_host(tag: &str, source: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let emitted = emit_at(tag, &source_path, &scratch.path().join("out.c"));
    (scratch, emitted)
}

/// Emits the same program twice from one source path, returning both units.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let first = emit_at(tag, &source_path, &scratch.path().join("out0.c"));
    let second = emit_at(tag, &source_path, &scratch.path().join("out1.c"));
    (first, second)
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

/// Asserts the emitted line with `anchor` also contains `needle`.
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

/// Asserts `first` appears before `second` in the emitted text.
fn assert_before(tag: &str, emitted: &str, first: &str, second: &str) {
    let first_at = emitted
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: {first:?} is missing; emitted C:\n{emitted}"));
    let second_at = emitted
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: {second:?} is missing; emitted C:\n{emitted}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}; emitted C:\n{emitted}"
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

/// Builds the emitted C with the strict host flags and returns the program.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).unwrap_or_else(|err| {
        panic!("{tag}: cannot write the emitted C: {err}");
    });
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

/// Runs a built program with a time limit, returning its exit code.
fn run_with_timeout(tag: &str, program: &Path) -> i32 {
    let mut child = Command::new(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code().unwrap_or(-1),
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
}

/// Builds and runs one emitted program under the strict host flags.
fn run_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> i32 {
    let program = build_host_c(tag, scratch, emitted);
    run_with_timeout(tag, &program)
}

/// Emits `source` and asserts the front end rejected it with one `code` error.
fn reject_once(tag: &str, source: &str, code: &str, fragment: &str) {
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
    assert!(
        stderr.contains(fragment),
        "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

// ---------------------------------------------------------------------------
// Hidden deref-chain division guard
// ---------------------------------------------------------------------------

/// A raw-deref generic division emits the `E11210` guard before the C divide.
#[test]
fn should_guard_the_deref_chain_division() {
    let tag = "div471";
    let (_scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.div471;

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
"#,
    );
    assert_contains(
        tag,
        &emitted,
        "cl_trap(\"E11210\", \"integer division by zero\")",
    );
}

// ---------------------------------------------------------------------------
// Maybe-target duties
// ---------------------------------------------------------------------------

/// A concrete lend moved into an owning maybe-target reports one `E10034`.
#[test]
fn should_report_the_concrete_lend_once() {
    let tag = "lend472";
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
    reject_once(tag, owning, "E10034", "'s'");

    let view = r#"module spec.condval.lend472str;

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
    let (scratch, emitted) = emit_host("lend472str", view);
    assert_eq!(run_host_c("lend472str", &scratch, &emitted), 0);
}

/// A concrete local moved on a maybe-target path reports `E10020` at the use.
#[test]
fn should_report_the_maybe_moved_local_once() {
    let tag = "move473";
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
    reject_once(tag, owning, "E10020", "'s'");

    let view = r#"module spec.condval.move473str;

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
    let (scratch, emitted) = emit_host("move473str", view);
    assert_eq!(run_host_c("move473str", &scratch, &emitted), 0);
}

// ---------------------------------------------------------------------------
// Mixed read base
// ---------------------------------------------------------------------------

/// The mixed place/literal read base stores the literal into its own temporary
/// and releases the selected literal's owners.
#[test]
fn should_emit_the_mixed_read_base_per_arm() {
    let tag = "mixedbase475";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.mixedbase475;

import { String, stringOf } from std.text;

struct Inner { name: String; row: i32[1][1]; tag: String; }

func n2(): i32 { return 3; }

func main(): i32 {
    let other: Inner = Inner { name: stringOf("cc"), row: [[4]], tag: stringOf("dd") };
    let ok: bool = false;
    return (ok ? other : Inner { name: stringOf("aa"), row: [[n2()]], tag: stringOf("bb") }).row[0][0];
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_mixedbase475_main(void)\n{");
    assert_contains(tag, body, "__cl_tmp1.name = cl_std_text_stringOf(cl_str(\"aa\"));");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_tmp2), &(other.row), sizeof(__cl_tmp2));",
    );
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_tmp2), &(__cl_tmp1.row), sizeof(__cl_tmp2));",
    );
    assert_contains(tag, body, "if (__cl_tmp1.name) { FreeString(__cl_tmp1.name); }");
    assert_contains(tag, body, "if (__cl_tmp1.tag) { FreeString(__cl_tmp1.tag); }");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 3);
}

// ---------------------------------------------------------------------------
// Guarded duplicable literal
// ---------------------------------------------------------------------------

/// A guarded duplicable literal passes the hidden array destination to the
/// selected arm and never renders the call without it.
#[test]
fn should_emit_the_guarded_duplicable_literal_per_arm() {
    let tag = "dupliteral476";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.dupliteral476;

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
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_dupliteral476_main(void)\n{");
    assert_on_line_of(tag, body, "v = (g ?", "n1(__cl_tmp0.row)");
    assert_on_line_of(tag, body, "v = (g ?", "n2(__cl_tmp0.row)");
    assert_eq!(
        body.matches("cl_spec_condval_dupliteral476_n1(__cl_tmp0.row)").count(),
        1
    );
    assert_eq!(
        body.matches("cl_spec_condval_dupliteral476_n2(__cl_tmp0.row)").count(),
        1
    );
    assert_absent(tag, body, "cl_spec_condval_dupliteral476_n1()");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 3);
}

// ---------------------------------------------------------------------------
// Nested conditional store
// ---------------------------------------------------------------------------

/// A nested conditional arm stores each literal through its own row calls.
#[test]
fn should_emit_the_nested_conditional_store() {
    let tag = "nested477";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.nested477;

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
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_nested477_main(void)\n{");
    assert_contains(tag, body, "cl_spec_condval_nested477_row1(__cl_call_3)");
    assert_contains(tag, body, "cl_spec_condval_nested477_row2(__cl_call_4)");
    assert_contains(tag, body, "cl_spec_condval_nested477_row3(__cl_call_5)");
    assert_contains(tag, body, "__cl_arm_9.names = cl_spec_condval_nested477_name1();");
    assert_contains(tag, body, "__cl_arm_10.names = cl_spec_condval_nested477_name2();");
    assert_contains(tag, body, "__cl_arm_11.names = cl_spec_condval_nested477_name3();");
    assert_contains(tag, body, "cl_spec_condval_nested477_consumeT(__cl_owned0)");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0);
}

// ---------------------------------------------------------------------------
// Deferred guarded take
// ---------------------------------------------------------------------------

/// A guarded duplicable struct conditional take stays inside the `g` arm.
#[test]
fn should_defer_the_guarded_duplicable_take() {
    let tag = "deferred478";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.deferred478;

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
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_deferred478_main(void)\n{");
    assert_on_line_of(tag, body, "r = (g ?", "consumeT((ok ?");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 2);

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
    let (scratch, emitted) = emit_host("deferred478skip", skipped);
    assert_eq!(run_host_c("deferred478skip", &scratch, &emitted), 0);
}

// ---------------------------------------------------------------------------
// Source-order field stores
// ---------------------------------------------------------------------------

/// A literal whose declaration order differs from its source order stores its
/// fields in source order; the fill twin evaluates them once.
#[test]
fn should_emit_the_source_order_field_stores() {
    let tag = "order479a";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.order479a;

import { String, stringOf } from std.text;

struct Inner { names: String; }
struct Outer { inner: Inner; tag: i32; }

let trace: i32 = 0;

func tagOf(): i32 { trace = trace * 10 + 1; return 7; }
func n1(): String { trace = trace * 10 + 2; return stringOf("aa"); }
func useOuter(o: Outer): i32 { return 1; }

func main(): i32 {
    let g: bool = true;
    if (g && useOuter(Outer { tag: tagOf(), inner: Inner { names: n1() } }) == 1) {
        if (trace != 12) { return 9; }
        return 0;
    }
    return 8;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_order479a_main(void)\n{");
    assert_before(
        tag,
        body,
        "__cl_tmp0.tag = cl_spec_condval_order479a_tagOf()",
        "__cl_tmp0.inner.names = cl_spec_condval_order479a_n1()",
    );
    assert_contains(tag, body, "cl_spec_condval_order479a_useOuter(__cl_tmp0)");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0);

    let fill_tag = "order479fill";
    let (scratch, emitted) = emit_host(
        fill_tag,
        r#"module spec.condval.order479fill;

struct Inner { names: static str; }
struct Outer { inner: Inner; tag: i32; }

let trace: i32 = 0;

func tagOf(): i32 { trace = trace * 10 + 1; return 7; }
func n1(): static str { trace = trace * 10 + 2; return "aa"; }

func main(): i32 {
    let os: Outer[2] = [; Outer { tag: tagOf(), inner: Inner { names: n1() } }];
    if (trace != 12) { return 9; }
    return 0;
}
"#,
    );
    let body = function_body(
        fill_tag,
        &emitted,
        "cl_spec_condval_order479fill_main(void)\n{",
    );
    assert_follows(
        fill_tag,
        body,
        "__cl_fill_1.tag = cl_spec_condval_order479fill_tagOf();",
        "__cl_fill_1.inner = (ClInner){ cl_spec_condval_order479fill_n1() };",
    );
    assert_eq!(run_host_c(fill_tag, &scratch, &emitted), 0);
}

// ---------------------------------------------------------------------------
// Fresh conditional root
// ---------------------------------------------------------------------------

/// The call-projected root releases the unselected Holder fields and the
/// literal arm releases its own name.
#[test]
fn should_release_the_fresh_conditional_root() {
    let tag = "freshroot480";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.freshroot480;

import { String, stringOf } from std.text;

struct Inner { name: String; row: i32[1][1]; }
struct Holder { inner: Inner; tag: String; }

func mk(): Holder { return Holder { inner: Inner { name: stringOf("bb"), row: [[4]] }, tag: stringOf("tt") }; }

func main(): i32 {
    let ok: bool = true;
    return (ok ? Inner { name: stringOf("aa"), row: [[3]] } : mk().inner).row[0][0];
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_freshroot480_main(void)\n{");
    assert_contains(tag, body, "__cl_tmp2 = cl_spec_condval_freshroot480_mk();");
    assert_contains(
        tag,
        body,
        "if (__cl_tmp2.inner.name) { FreeString(__cl_tmp2.inner.name); }",
    );
    assert_contains(tag, body, "if (__cl_tmp2.tag) { FreeString(__cl_tmp2.tag); }");
    assert_contains(tag, body, "if (__cl_tmp1.name) { FreeString(__cl_tmp1.name); }");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 3);
}

// ---------------------------------------------------------------------------
// Conditional call-arm root
// ---------------------------------------------------------------------------

/// The selected call arm's root is bound and released after the view read.
#[test]
fn should_release_the_conditional_call_arm() {
    let tag = "callroot481";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.callroot481;

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
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_callroot481_main(void)\n{");
    assert_contains(
        tag,
        body,
        "p = (__cl_tmp0 ? (u8 *)(__cl_tmp1.tag.ptr) : (u8 *)(__cl_tmp2.tag.ptr));",
    );
    assert_contains(tag, body, "if (__cl_tmp1.name) { FreeString(__cl_tmp1.name); }");
    assert_contains(tag, body, "if (__cl_tmp1.extra) { FreeString(__cl_tmp1.extra); }");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0);
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

/// The accepted changed units re-emit byte for byte.
#[test]
fn should_reemit_the_changed_conditional_units() {
    let cases = [
        (
            "detmixed",
            r#"module spec.condval.detmixed;

import { String, stringOf } from std.text;

struct Inner { name: String; row: i32[1][1]; tag: String; }

func n2(): i32 { return 3; }

func main(): i32 {
    let other: Inner = Inner { name: stringOf("cc"), row: [[4]], tag: stringOf("dd") };
    let ok: bool = false;
    return (ok ? other : Inner { name: stringOf("aa"), row: [[n2()]], tag: stringOf("bb") }).row[0][0];
}
"#,
        ),
        (
            "detorder",
            r#"module spec.condval.detorder;

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
"#,
        ),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(
            first, second,
            "{tag}: the changed conditional unit is not byte-identical on re-emission"
        );
    }
}

// ---------------------------------------------------------------------------
// Conditional place arm
// ---------------------------------------------------------------------------

/// A duplicable place arm is copied without zeroing the source.
#[test]
fn should_copy_a_duplicable_place_arm_in_a_guarded_conditional() {
    let tag = "placearm475b";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.condval.placearm475b;

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
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_condval_placearm475b_main(void)\n{");
    assert_absent(tag, body, "cl_zero(&other)");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 4);
}
