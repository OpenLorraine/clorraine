//! Runnable specification tests for the semantic parity batch.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! and runs the binary. The memory cases run under AddressSanitizer when
//! `RDC_ASAN` is set and as ordinary strict builds otherwise.
//!
//! The batch pins the null-arm contexts, substituted view-array returns, nested
//! element resolution, nullable conditional reports, nested projections, and
//! view-member move/read forms.
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
            "clc-sempspec-{}-{tag}-{sequence}",
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

/// Asserts the front end rejected `source` with `expected` errors carrying
/// `code`, and that no translation unit was written.
fn assert_rejected(tag: &str, source: &str, code: &str, expected: usize, fragments: &[&str]) {
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
        expected,
        "{tag}: expected {expected} error(s); stderr:\n{stderr}"
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

/// Asserts the front end rejected `source` with exactly one `code` error.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    assert_rejected(tag, source, code, 1, fragments);
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



// ---------------------------------------------------------------------------
// Null-arm contexts
// ---------------------------------------------------------------------------

/// The null rule is judged by the context: a bool `if` reports `E10012`, an
/// i32 `if` reports `E10218`, a cast/binary/annotated let reports `E10012`, an
/// unannotated let infers a nullable binding, and a nullable loop head reports
/// `E10012`.
#[test]
fn should_judge_the_null_arm_contexts() {
    let bool_if = r#"module spec.sempar.st23a;

func main(): i32 {
    let c: bool = true;
    if (c ? true : null) { return 1; }
    return 0;
}
"#;
    assert_rejected_once("st23a", bool_if, "E10012", &[]);

    let i32_if = r#"module spec.sempar.st23b;

func main(): i32 {
    let c: bool = true;
    let x: i32 = 1;
    if (c ? x : null) { return 1; }
    return 0;
}
"#;
    assert_rejected_once("st23b", i32_if, "E10218", &[]);

    let cast = r#"module spec.sempar.st23c;

func main(): i32 {
    let c: bool = true;
    let z: i32 = i32(c ? 1 : null);
    return z;
}
"#;
    assert_rejected_once("st23c", cast, "E10012", &[]);

    let binary = r#"module spec.sempar.st23d;

func main(): i32 {
    let c: bool = true;
    let z: i32 = (c ? 1 : null) + 1;
    return z;
}
"#;
    assert_rejected_once("st23d", binary, "E10012", &[]);

    let annotated = r#"module spec.sempar.st23e;

func main(): i32 {
    let c: bool = true;
    let y: i32 = c ? 1 : null;
    return y;
}
"#;
    assert_rejected_once("st23e", annotated, "E10012", &[]);

    let nullable = r#"module spec.sempar.st23f;

import { String, stringOf } from std.text;

func main(): i32 {
    let c: bool = true;
    let h: String = stringOf("aa");
    let y = c ? h : null;
    if (y == null) { return 9; }
    return i32(len(y)) - 2;
}
"#;
    let output = run_memory("st23f", nullable);
    assert_no_sanitizer("st23f", &output);
    assert_exit("st23f", &output, 0);

    let loop_head = r#"module spec.sempar.st23g;

func main(): i32 {
    let c: bool = true;
    let a: i32[2] = [1, 2];
    for (v in c ? a : null) {
        if (v == 1) { return 9; }
    }
    return 0;
}
"#;
    assert_rejected_once("st23g", loop_head, "E10012", &[]);
}

// ---------------------------------------------------------------------------
// Substituted view-array return
// ---------------------------------------------------------------------------

/// A substituted view-array return derives from the single lend parameter; the
/// concrete twin accepts and the safe control is ASAN clean.
#[test]
fn should_accept_a_substituted_view_array_return() {
    let source = r#"module spec.sempar.st24;

import { String, stringOf } from std.text;

struct H { name: String; }

func first<T>(borrow xs: T): str[1] {
    return [xs[0].name];
}

func main(): i32 {
    let hs: H[2] = [H { name: stringOf("aa") }, H { name: stringOf("bb") }];
    let s: str[1] = first<H[2]>(hs);
    return i32(len(s[0])) - 2;
}
"#;
    let output = run_memory("st24", source);
    assert_no_sanitizer("st24", &output);
    assert_exit("st24", &output, 0);
}

// ---------------------------------------------------------------------------
// Nested element resolution
// ---------------------------------------------------------------------------

/// A nested loop over an unknown substitution resolves the element through the
/// instance; no placeholder reaches the C and the program runs.
#[test]
fn should_resolve_a_nested_element_substitution() {
    let source = r#"module spec.sempar.st25;

func f<T>(xs: T[2]): i32 {
    let n: i32 = 0;
    for (v in xs) {
        for (w in v) {
            n = n + i32(w);
        }
    }
    return n;
}

func main(): i32 {
    let m: u8[2][2] = [[1, 2], [3, 4]];
    return i32(f<u8[2]>(m)) - 10;
}
"#;
    let output = run_memory("st25", source);
    assert_no_sanitizer("st25", &output);
    assert_exit("st25", &output, 0);
}

// ---------------------------------------------------------------------------
// Same-arm nullable conditional
// ---------------------------------------------------------------------------

/// Both nullable arms of a same-arm conditional report one `E10010` each at a
/// non-nullable `str` destination.
#[test]
fn should_report_each_nullable_arm_once() {
    let source = r#"module spec.sempar.st26;

import { String, stringOf } from std.text;

struct H { name: String?; }

func main(): i32 {
    let xs: H[2] = [H { name: stringOf("aa") }, H { name: null }];
    let c: bool = true;
    let s: str = c ? xs[0].name : xs[0].name;
    return i32(len(s)) - 2;
}
"#;
    assert_rejected("st26", source, "E10010", 2, &["xs"]);

    // The same conditional at a type-parameter destination is judged after
    // substitution: one E10010 per nullable arm, as the concrete twin does.
    let generic = r#"module spec.sempar.st26generic;

import { String, stringOf } from std.text;

struct H { name: String?; }

func pick<T>(c: bool, xs: H[2]): i32 {
    let r: T = c ? xs[0].name : xs[0].name;
    return i32(len(r)) - 2;
}

func main(): i32 {
    let xs: H[2] = [H { name: stringOf("aa") }, H { name: null }];
    return pick<str>(true, xs);
}
"#;
    assert_rejected("st26generic", generic, "E10010", 2, &["xs"]);
}

// ---------------------------------------------------------------------------
// Nested projection parity
// ---------------------------------------------------------------------------

/// A duplicable nested projection is accepted; an owning one reports one
/// `E10026`; the lend field form reports one `E10034`.
#[test]
fn should_judge_a_nested_projection_by_instance() {
    let duplicable = r#"module spec.sempar.st27a;

struct Inner { n: i32; }
struct H { inner: Inner; }

func g<T, U>(c: bool, h: T, d: U): U {
    return c ? h.inner.n : d;
}

func main(): i32 {
    let h: H = H { inner: Inner { n: 3 } };
    return g<H, i32>(true, h, 0) - 3;
}
"#;
    let output = run_memory("st27a", duplicable);
    assert_no_sanitizer("st27a", &output);
    assert_exit("st27a", &output, 0);

    let owning = r#"module spec.sempar.st27b;

import { String, stringOf } from std.text;

struct Inner { n: String; }
struct H { inner: Inner; }

func g<T, U>(c: bool, take h: T, take d: U): U {
    return c ? h.inner.n : d;
}

func main(): i32 {
    let h: H = H { inner: Inner { n: stringOf("aa") } };
    let d: String = stringOf("bb");
    let r: String = g<H, String>(true, take h, take d);
    return i32(len(r)) - 2;
}
"#;
    assert_rejected_once("st27b", owning, "E10026", &[]);

    let lend_field = r#"module spec.sempar.st27c;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct H { inner: Inner; }

func g<T, U>(c: bool, borrow h: T, take d: U): U {
    return c ? h.inner.name : d;
}

func main(): i32 {
    let h: H = H { inner: Inner { name: stringOf("aa") } };
    let d: String = stringOf("bb");
    let r: String = g<H, String>(true, h, take d);
    return i32(len(r)) - 2;
}
"#;
    assert_rejected_once("st27c", lend_field, "E10034", &[]);
}

// ---------------------------------------------------------------------------
// Static-only viewed position
// ---------------------------------------------------------------------------

/// A callee that views only a static field is accepted, including when its
/// caller returns that static view; the owned-field twin keeps the frozen
/// view-return rejection because an owned field is not a view-like lend.
#[test]
fn should_accept_a_static_only_viewed_position() {
    let safe = r#"module spec.sempar.st28;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func viewTag(borrow h: H): str {
    return h.tag;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: "aa" };
    let s: str = viewTag(h);
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("st28", safe);
    assert_no_sanitizer("st28", &output);
    assert_exit("st28", &output, 0);

    // The filed rejection sat at a caller that passed a local mixed aggregate
    // to a static-only viewed callee and returned the view. The call's result
    // is static storage, so the local does not back it.
    let caller = r#"module spec.sempar.st28caller;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func viewTag(borrow h: H): str {
    return h.tag;
}

func caller(): str {
    let h: H = H { name: stringOf("aa"), tag: "aa" };
    return viewTag(h);
}

func main(): i32 {
    let s: str = caller();
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("st28caller", caller);
    assert_no_sanitizer("st28caller", &output);
    assert_exit("st28caller", &output, 0);

    // The static proof must survive substitution: a generic callee at a struct
    // instance reads only the static field, exactly as its concrete twin does.
    let generic = r#"module spec.sempar.st28generic;

struct H { tag: static str; }

func viewTagG<T>(borrow h: T): str { return h.tag; }

func main(): i32 {
    let h: H = H { tag: "aa" };
    let s: str = viewTagG<H>(h);
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("st28generic", generic);
    assert_no_sanitizer("st28generic", &output);
    assert_exit("st28generic", &output, 0);

    let generic_owned = r#"module spec.sempar.st28genericowned;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func viewTagG<T>(borrow h: T): str { return h.tag; }

func main(): i32 {
    let h: H = H { name: stringOf("nn"), tag: "aa" };
    let s: str = viewTagG<H>(h);
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("st28genericowned", generic_owned);
    assert_no_sanitizer("st28genericowned", &output);
    assert_exit("st28genericowned", &output, 0);

    let generic_array = r#"module spec.sempar.st28genericarray;

struct H { tag: static str; }

func tagsG<T>(borrow h: T): str[2] { return [h.tag, h.tag]; }

func main(): i32 {
    let h: H = H { tag: "aa" };
    let s: str[2] = tagsG<H>(h);
    return i32(len(s[0])) - 2;
}
"#;
    let output = run_memory("st28genericarray", generic_array);
    assert_no_sanitizer("st28genericarray", &output);
    assert_exit("st28genericarray", &output, 0);

    let owned = r#"module spec.sempar.st28name;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func nameView(borrow h: H): str {
    return h.name;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: "aa" };
    let s: str = nameView(h);
    return i32(len(s)) - 2;
}
"#;
    // An owned field is not a view-like lend source: the frozen view-return
    // rule reports E10036, not a lend-store error. The static field above is
    // the safe control because its bytes never live in the borrowed aggregate.
    assert_rejected_once("st28name", owned, "E10036", &[]);
}

// ---------------------------------------------------------------------------
// Static-return helper
// ---------------------------------------------------------------------------

/// A helper that returns static storage is accepted in the direct and slot
/// forms; the row closes stale with a green pin.
#[test]
fn should_accept_a_static_return_helper() {
    let source = r#"module spec.sempar.st29;

func always(xs: str[2]): str {
    return "ZZ";
}

func main(): i32 {
    let xs: str[2] = ["aa", "bb"];
    let slot: str = always(xs);
    return i32(len(slot)) - 2;
}
"#;
    let output = run_memory("st29", source);
    assert_no_sanitizer("st29", &output);
    assert_exit("st29", &output, 0);
}

// ---------------------------------------------------------------------------
// Inline iterable alias return
// ---------------------------------------------------------------------------

/// A returned alias of an inline iterable temporary reports one `E10030`.
#[test]
fn should_reject_an_inline_iterable_alias_return() {
    let source = r#"module spec.sempar.st30;

import { String, stringOf } from std.text;

struct B { v: String; }

func make(borrow s: String): str {
    return s;
}

func go(borrow b: B): str {
    for (borrow e in [make(b.v)]) {
        return e;
    }
    return "";
}

func main(): i32 {
    let b: B = B { v: stringOf("aa") };
    let s: str = go(b);
    return i32(len(s)) - 2;
}
"#;
    assert_rejected_once("st30", source, "E10030", &["e"]);
}

// ---------------------------------------------------------------------------
// View-member move and read forms
// ---------------------------------------------------------------------------

/// The view-member projection over a type-parameter conditional is accepted in
/// the move and read forms, including a static tag read through a local arm;
/// a local arm's owned-field view still reports the frozen view-return code.
#[test]
fn should_accept_a_type_parameter_view_member_projection() {
    let moved = r#"module spec.sempar.st31;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { p: raw u8; }

func pack<T>(d: bool, borrow h1: T, borrow h2: T): S {
    return S { p: (d ? h1 : h2).tag.ptr };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let s: S = pack<H>(true, h1, h2);
        if (s.p == null) { return 9; }
    }
    return 0;
}
"#;
    let output = run_memory("st31", moved);
    assert_no_sanitizer("st31", &output);
    assert_exit("st31", &output, 0);

    // The same projection reads the static view's length and an element: both
    // are value copies, not moves of either lend arm.
    let len = r#"module spec.sempar.st31len;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { n: usize; }

func packLen<T>(d: bool, borrow h1: T, borrow h2: T): S {
    return S { n: (d ? h1 : h2).tag.len };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    let s: S = packLen<H>(true, h1, h2);
    if (s.n != 2) { return 9; }
    return 0;
}
"#;
    let output = run_memory("st31len", len);
    assert_no_sanitizer("st31len", &output);
    assert_exit("st31len", &output, 0);

    let byte = r#"module spec.sempar.st31byte;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { b: u8; }

func packByte<T>(d: bool, borrow h1: T, borrow h2: T): S {
    return S { b: (d ? h1 : h2).tag[0] };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    let s: S = packByte<H>(true, h1, h2);
    if (s.b != 97) { return 9; }
    return 0;
}
"#;
    let output = run_memory("st31byte", byte);
    assert_no_sanitizer("st31byte", &output);
    assert_exit("st31byte", &output, 0);

    let read = r#"module spec.sempar.st31read;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func readTag<T>(d: bool, borrow h1: T, borrow h2: T): raw u8 {
    return (d ? h1 : h2).tag.ptr;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let p: raw u8 = readTag<H>(true, h1, h2);
        if (p == null) { return 9; }
    }
    return 0;
}
"#;
    let output = run_memory("st31read", read);
    assert_no_sanitizer("st31read", &output);
    assert_exit("st31read", &output, 0);

    let local = r#"module spec.sempar.st31local;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { p: raw u8; }

func pack<T>(d: bool, borrow h1: T, borrow h2: T): S {
    let l: H = H { name: stringOf("ll"), tag: "ll" };
    return S { p: (d ? h1 : l).tag.ptr };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let s: S = pack<H>(true, h1, h2);
        if (s.p == null) { return 9; }
    }
    return 0;
}
"#;
    // A local aggregate's static tag still points at module-lifetime bytes;
    // the pointer copy does not borrow the local frame.
    let output = run_memory("st31local", local);
    assert_no_sanitizer("st31local", &output);
    assert_exit("st31local", &output, 0);

    // The unsafe twin views the local's owned field: a returned view must
    // derive from a parameter, not from a local, so the frozen view-return
    // code reports it. The concrete spelling reports the same code.
    let local_owned = r#"module spec.sempar.st31localowned;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func pack<T>(d: bool, borrow h1: T, borrow h2: T): str {
    let l: H = H { name: stringOf("ll"), tag: "ll" };
    return (d ? h1 : l).name;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    let s: str = pack<H>(true, h1, h2);
    return i32(len(s)) - 2;
}
"#;
    assert_rejected_once("st31localowned", local_owned, "E10035", &["l"]);
}

// ---------------------------------------------------------------------------
// Candidate and control regressions
// ---------------------------------------------------------------------------

/// A `borrow T` view-return candidate is counted after substitution: an
/// owning aggregate reports one E10036, while a single concrete lend beside
/// the borrow parameter stays accepted.
#[test]
fn should_judge_a_borrow_type_parameter_candidate_by_instance() {
    let owned = r#"module spec.sempar.st412owned;

import { String, stringOf } from std.text;

struct H { name: String; }

func getName<T>(borrow h: T): str { return h.name; }

func main(): i32 {
    let h: H = H { name: stringOf("ab") };
    let s: str = getName<H>(h);
    return i32(len(s)) - 2;
}
"#;
    assert_rejected_once("st412owned", owned, "E10036", &[]);

    let single = r#"module spec.sempar.st412single;

struct H { tag: static str; }

func g<T>(borrow xs: T, s: str): str { return s; }

func main(): i32 {
    let h: H = H { tag: "hh" };
    let r: str = g<H>(h, "ab");
    return i32(len(r)) - 2;
}
"#;
    let output = run_memory("st412single", single);
    assert_no_sanitizer("st412single", &output);
    assert_exit("st412single", &output, 0);

    let two_lends = r#"module spec.sempar.st412two;

import { slice } from std.mem;

func g<T>(borrow xs: T, s: str): str { return s; }

func main(): i32 {
    let b: u8[2] = [1, 2];
    let xs: u8[] = slice(b, 0, 2);
    let r: str = g<u8[]>(xs, "ab");
    return i32(len(r)) - 2;
}
"#;
    assert_rejected_once("st412two", two_lends, "E10036", &[]);

    // A plain type-parameter parameter becomes a second lend at the instance;
    // the concrete twin reports the two-candidate E10036 and so must the
    // generic spelling.
    let plain_second = r#"module spec.sempar.st412plain;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(borrow a: T, b: T): str[1] { return [a[0].name]; }

func main(): i32 {
    let hs: H[2] = [H { name: stringOf("aa") }, H { name: stringOf("bb") }];
    let s: str[1] = f<H[2]>(hs, hs);
    return i32(len(s[0])) - 2;
}
"#;
    assert_rejected_once("st412plain", plain_second, "E10036", &[]);
}

/// A static return in one branch does not hide a wrong-source view return in
/// another branch.
#[test]
fn should_reject_a_mixed_static_view_return() {
    let source = r#"module spec.sempar.st28mixed;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func pick(c: bool, borrow b: H): str {
    if (c) { return b.tag; }
    return b.name;
}

func main(): i32 {
    let b: H = H { name: stringOf("bb"), tag: "bb" };
    let s: str = pick(false, b);
    return i32(len(s)) - 2;
}
"#;
    assert_rejected_once("st28mixed", source, "E10036", &[]);
}

/// A nullable iterable is judged by flow narrowing: the narrowed conditional
/// accepts, while an unnarrowed nullable binding reports the possible-null
/// rule.
#[test]
fn should_judge_a_nullable_iterable_by_flow_narrowing() {
    let narrowed = r#"module spec.sempar.st411narrow;

import { slice } from std.mem;

func f(c: bool, a: u8[]?): i32 {
    if (a == null) { return 0; }
    let b: u8[2] = [1, 2];
    let v: u8[] = slice(b, 0, 2);
    let n: i32 = 0;
    for (x in c ? a : v) { n = n + i32(x); }
    return n;
}

func main(): i32 {
    let b: u8[2] = [1, 2];
    let v: u8[] = slice(b, 0, 2);
    return f(true, v) - 3;
}
"#;
    let output = run_memory("st411narrow", narrowed);
    assert_no_sanitizer("st411narrow", &output);
    assert_exit("st411narrow", &output, 0);

    let unnarrowed = r#"module spec.sempar.st411unnarrow;

import { slice } from std.mem;

func f(a: u8[]?): i32 {
    let n: i32 = 0;
    for (x in a) { n = n + i32(x); }
    return n;
}

func main(): i32 {
    let b: u8[2] = [1, 2];
    let v: u8[] = slice(b, 0, 2);
    return f(v);
}
"#;
    assert_rejected_once("st411unnarrow", unnarrowed, "E10010", &["a"]);
}

/// A conditional's declared non-null arm is not a possible-null use.
#[test]
fn should_skip_the_non_null_arm_of_a_nullable_conditional() {
    let source = r#"module spec.sempar.st415ok;

import { String, stringOf } from std.text;

func f(c: bool, s: String?, t: String): i32 {
    if (s == null) { return 0; }
    let r: str = c ? s : t;
    return i32(len(r)) - 2;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    let t: String = stringOf("cd");
    return f(true, s, t);
}
"#;
    let output = run_memory("st415ok", source);
    assert_no_sanitizer("st415ok", &output);
    assert_exit("st415ok", &output, 0);
}

/// A copied inline-iterable element return creates no loan and stays
/// accepted; the view-return alias remains the E10030 case.
#[test]
fn should_accept_a_copied_inline_iterable_element() {
    let source = r#"module spec.sempar.st513copy;

func f(): i32 {
    for (borrow e in [1, 2, 3]) { return e; }
    return 9;
}

func main(): i32 { return f() - 1; }
"#;
    let output = run_memory("st513copy", source);
    assert_no_sanitizer("st513copy", &output);
    assert_exit("st513copy", &output, 0);
}

/// A take parameter's intrinsic view-member projection is a value copy, not a
/// partial move.
#[test]
fn should_accept_a_take_view_member_projection() {
    let source = r#"module spec.sempar.st522take;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { p: raw u8; }

func pack<T>(d: bool, take h1: T, take h2: T): S {
    return S { p: (d ? h1 : h2).tag.ptr };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let s: S = pack<H>(true, take h1, take h2);
        if (s.p == null) { return 9; }
        if (s.p[0] != 97) { return 8; }
    }
    return 0;
}
"#;
    let output = run_memory("st522take", source);
    assert_no_sanitizer("st522take", &output);
    assert_exit("st522take", &output, 0);
}
