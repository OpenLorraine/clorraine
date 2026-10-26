//! Runnable specification tests for conditional-projection residue.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under a fixed strict warning
//! set, runs the binary under a time limit, and asserts the process result. The
//! memory helper runs a case as an ordinary strict host build and adds leak and
//! use-after-free detection when `RDC_ASAN` is set.
//!
//! The cases pin five accepted programs whose conditional- and projection-shaped
//! ownership paths leak, double-free, or read freed storage. When a conditional
//! selects a member, the chosen root must move that member out and release only
//! its remaining fields, so every owner releases exactly once. The same rule
//! covers a `take` argument behind a guard, a literal receiver field, and the
//! loop spellings that re-execute a projection. The last case holds the direct
//! spellings as an accepted control whose behavior must not change.

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
            "clc-cpr-{}-{tag}-{sequence}",
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

/// Compiles `source` for the host profile and returns the scratch directory,
/// the compiler output, and the emitted-C path.
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

/// Emits one host-profile program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> (ScratchDir, String) {
    let (scratch, output, c_path) = compile_only(tag, source);
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
    (scratch, emitted)
}

/// Asserts the front end rejected `source` once with `code` and wrote no C.
fn assert_rejected_once(tag: &str, source: &str, code: &str) {
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
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles the emitted C with the host compiler under the strict flag set,
/// adding AddressSanitizer when `asan` is true.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str, asan: bool) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut args = vec![
        "-std=c99".to_owned(),
        "-O0".to_owned(),
        "-g".to_owned(),
        "-Wall".to_owned(),
        "-Wextra".to_owned(),
        "-Werror".to_owned(),
    ];
    if asan {
        args.push("-fsanitize=address".to_owned());
        args.push("-no-pie".to_owned());
    }
    args.push("-o".to_owned());
    args.push(program.to_str().expect("program path").to_owned());
    args.push(c_path.to_str().expect("emitted C path").to_owned());
    let built = Command::new("cc")
        .args(&args)
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

/// Emits, builds, and runs one host program with the strict host build.
fn run_host(tag: &str, source: &str) -> Output {
    let (scratch, emitted) = emit_host(tag, source);
    let program = build_host_c(tag, &scratch, &emitted, false);
    run_with_timeout(tag, &program, &[])
}

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
fn run_asan(tag: &str, source: &str) -> Output {
    let (scratch, emitted) = emit_host(tag, source);
    let program = build_host_c(tag, &scratch, &emitted, true);
    run_with_timeout(
        tag,
        &program,
        &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")],
    )
}

/// Runs a memory-safety case: under AddressSanitizer when `RDC_ASAN` is set,
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

/// Asserts the front end rejected `source` with exactly `count` error reports,
/// each carrying `code`, and emitted no translation unit.
///
/// The exact count is the point: every error line must carry the expected code
/// so a fix cannot pass by adding an unrelated diagnostic, and a rejected
/// program must leave no emitted `.c` behind.
#[allow(dead_code)]
fn compile_rejects(tag: &str, source: &str, code: &str, count: usize) {
    let (_scratch, output, c_path) = compile_only(tag, source);
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

/// Asserts the front end rejected `source` with exactly one report carrying
/// `code` at `line:column`, that the report contains every phrase in
/// `fragments` and none of the markers in `absent`, and that it emitted no
/// translation unit.
///
/// This is the span-aware sibling of [`compile_rejects`]: the dedupe and
/// display rules decide where and how a rejection is carried, so a one-report
/// case pins the site as well as the code, and a fix that moves the report to
/// another expression must fail it. The scratch program is always named
/// `main.clo`, and a diagnostic renders its site on the line after its
/// message, so the location line is matched by its `:line:column` suffix.
fn compile_rejects_once_at(
    tag: &str,
    source: &str,
    code: &str,
    line: usize,
    column: usize,
    fragments: &[&str],
    absent: &[&str],
) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|report| report.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "{tag}: expected exactly one error, saw {}; stderr:\n{stderr}",
        errors.len()
    );
    assert!(
        errors[0].contains(code),
        "{tag}: expected the {code} report; saw `{}`; stderr:\n{stderr}",
        errors[0]
    );
    let suffix = format!(":{line}:{column}");
    let sites: Vec<&str> = stderr
        .lines()
        .filter(|report| report.trim_start().starts_with("--> "))
        .collect();
    assert_eq!(
        sites.len(),
        1,
        "{tag}: expected one report site, saw {}; stderr:\n{stderr}",
        sites.len()
    );
    assert!(
        sites[0].trim_end().ends_with(&suffix),
        "{tag}: expected the report at {suffix}; saw `{}`; stderr:\n{stderr}",
        sites[0].trim()
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    for marker in absent {
        assert!(
            !stderr.contains(marker),
            "{tag}: the marker {marker:?} must be absent; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

// ---------------------------------------------------------------------------
// armduty: moving a generic conditional arm's selected member into a value.
//
// The conditional mixes a `take` parameter (a place) with a freshly built pair.
// The `T`-typed member is a type parameter, so the move-out duty must be judged
// per instantiation: the chosen member moves into the consuming local, and the
// chosen root's un-moved field releases once. The program is accepted and runs
// with no memory report.
// ---------------------------------------------------------------------------

/// Moving the selected member of a generic conditional arm into a local value
/// keeps every owner released exactly once.
///
/// `(c ? a : mkPair<T>(x, y)).v` moves the chosen pair's `v` into `s`. For the
/// place arm the parameter `a` keeps its `w` field; for the fresh arm the built
/// pair's `w` field is released once. Both instantiations are `String`, whose
/// lengths cancel the trailing constant, so the run exits 0 and is clean.
#[test]
fn should_move_the_selected_member_of_a_generic_conditional_arm() {
    let source = r#"module spec.cpr.armduty;

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
    let output = run_memory("armduty", source);
    assert_no_asan("armduty", &output);
    assert_exit("armduty", &output, 0);
}

// ---------------------------------------------------------------------------
// armfresh: releasing the freshly created root behind a conditional projection.
//
// The conditional's fresh arm is a call that returns an owning struct, and its
// projected member is read into an owned local. The member moves out and the
// call result's un-moved fields release once, so the created storage never
// leaks. The named place arm keeps its own owner.
// ---------------------------------------------------------------------------

/// Releasing the fresh root behind a conditional projection keeps the run
/// leak-free.
///
/// `c ? a.v : mkBox().v` moves the chosen member into `s`. When the fresh arm
/// is chosen, the call result's sibling field must be released once; when the
/// place arm is chosen, the parameter's root keeps its field. The lengths cancel
/// the trailing constant, so the run exits 0 and is clean.
#[test]
fn should_release_the_fresh_root_of_a_conditional_projection() {
    let source = r#"module spec.cpr.armfresh;

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
    let output = run_memory("armfresh", source);
    assert_no_asan("armfresh", &output);
    assert_exit("armfresh", &output, 0);
}

// ---------------------------------------------------------------------------
// armliteral: releasing a literal arm of a conditional projection.
//
// When a conditional's arm is a struct literal that owns a fresh field, the
// projected member moves out and the literal's remaining field releases once.
// The plain store spelling and the guarded spelling follow the same rule.
// ---------------------------------------------------------------------------

/// Releasing a literal arm of a conditional projection keeps both spellings
/// memory-clean.
///
/// In the plain spelling `(c ? a : Box { ... }).v` moves the chosen member out;
/// when the literal is chosen, its sibling field releases once. In the guarded
/// spelling the same projection is reached through `g && ...` and the taken path
/// returns 1. Both programs are accepted, exit 0, and are clean.
#[test]
fn should_release_a_literal_arm_of_a_conditional_projection() {
    let plain = r#"module spec.cpr.armliteral;

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
    let output = run_memory("armliteral", plain);
    assert_no_asan("armliteral", &output);
    assert_exit("armliteral", &output, 0);

    let guarded = r#"module spec.cpr.armliteralguard;

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
    let output = run_memory("armliteralguard", guarded);
    assert_no_asan("armliteralguard", &output);
    assert_exit("armliteralguard", &output, 0);
}

// ---------------------------------------------------------------------------
// guardtakes: releasing a guarded `take` of a conditional projection.
//
// The guarded condition passes the projection's member to a `take` parameter,
// so ownership transfers into the callee on the taken path. The chosen root's
// remaining fields must still release once, and a loop that re-executes the
// take must release per execution rather than double-free.
// ---------------------------------------------------------------------------

/// Releasing a guarded `take` of a conditional projection keeps the single-shot
/// spelling memory-clean.
///
/// `g && consume((c ? a : mkBox()).v) > 0` transfers the chosen member into the
/// callee and the root's sibling field releases once. The lengths cancel the
/// trailing constant, so the run exits 0 and is clean.
#[test]
fn should_release_a_guarded_take_of_a_conditional_projection() {
    let source = r#"module spec.cpr.guardtakes;

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
    let output = run_memory("guardtakes", source);
    assert_no_asan("guardtakes", &output);
    assert_exit("guardtakes", &output, 0);
}

/// A guarded `take` of a conditional projection re-executed in a loop releases
/// once per execution.
///
/// Each iteration passes a fresh conditional projection to the guarded `take`,
/// so the chosen root's sibling field must release once each time and the
/// transferred member must not be released again on the next iteration. The
/// accumulated returns cancel the trailing constant, so the run exits 0 and is
/// clean.
#[test]
fn should_release_a_guarded_take_of_a_conditional_projection_in_a_loop() {
    let source = r#"module spec.cpr.guardtakesloop;

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
    let output = run_memory("guardtakesloop", source);
    assert_no_asan("guardtakesloop", &output);
    assert_exit("guardtakesloop", &output, 0);
}

// ---------------------------------------------------------------------------
// fieldroot: releasing the root of a literal receiver field.
//
// A struct-literal receiver initializes a field with a projection taken from a
// call result, and the receiver is called through a callable field. The call's
// taken path must release the receiver's owned field storage exactly once. A
// loop that rebuilds the receiver each iteration must release once per
// iteration rather than accumulate.
// ---------------------------------------------------------------------------

/// Releasing the root of a literal receiver field keeps the single-shot
/// spelling memory-clean.
///
/// `H { call: addOne, s: mkBox().v }.call(5)` owns the projected `s` field, so
/// the taken path releases that storage once. Both calls take the guarded path,
/// so their returns cancel the trailing constant and the run exits 0 clean.
#[test]
fn should_release_the_root_of_a_literal_receiver_field() {
    let source = r#"module spec.cpr.fieldroot;

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
    let output = run_memory("fieldroot", source);
    assert_no_asan("fieldroot", &output);
    assert_exit("fieldroot", &output, 0);
}

/// A literal receiver field rebuilt in a loop releases its root once per
/// iteration.
///
/// The loop evaluates the receiver three times, so each iteration's projected
/// field storage must release once and the runs must not accumulate. The
/// accumulated returns cancel the trailing constant, so the run exits 0 and is
/// clean.
#[test]
fn should_release_the_root_of_a_literal_receiver_field_in_a_loop() {
    let source = r#"module spec.cpr.fieldrootloop;

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
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        n = n + use(true);
        i = i + 1;
    }
    return n - 3;
}
"#;
    let output = run_memory("fieldrootloop", source);
    assert_no_asan("fieldrootloop", &output);
    assert_exit("fieldrootloop", &output, 0);
}

// ---------------------------------------------------------------------------
// armcontrols: accepted direct spellings that must not change.
//
// A `take` parameter's member moved into an owned local, a fresh member read,
// and an unguarded literal receiver all already release their storage
// correctly. Their behavior must stay accepted and memory-clean.
// ---------------------------------------------------------------------------

/// The direct arm and receiver spellings stay accepted and memory-clean.
///
/// The moved read `a.v` into an owned local, the fresh read `mkBox().v`, and the
/// unguarded callable receiver each already release their storage exactly once.
/// The accumulated lengths cancel the trailing constant, so the run exits 0 and
/// is clean.
#[test]
fn should_keep_the_direct_arm_and_receiver_spellings_accepted() {
    let source = r#"module spec.cpr.armcontrols;

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
    let output = run_memory("armcontrols", source);
    assert_no_asan("armcontrols", &output);
    assert_exit("armcontrols", &output, 0);
}

// ---------------------------------------------------------------------------
// callwrappedowner: the owner walk through a view-returning call.
//
// A call such as `bytes(...)` returns a view into storage its argument still
// owns, so binding the result records the argument's owners as loans. When the
// argument is a conditional over two lender parameters, every arm names an
// owner the view may read: whichever arm the call selects, a write to that
// arm's backing while the view is live is the plain spelling's mutation error.
// ---------------------------------------------------------------------------

/// A call-wrapped conditional view records every arm's owner, so a write to
/// either parameter backing reports the plain twin's mutation error.
///
/// `bytes(b.v)` reports `E10031` at the mutation because its view is a loan of
/// `b`. The call-wrapped conditional `(c ? b : d).v` must match on both
/// selections: each program writes the backing of the arm its call selects at
/// run time, and each reports exactly one `E10031` at the mutation and emits
/// no translation unit.
#[test]
fn should_record_every_arm_owner_of_a_call_wrapped_conditional_view() {
    let first = r#"module spec.cpr.callwrappedownerfirst;

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
    compile_rejects("callwrappedownerfirst", first, "E10031", 1);

    let second = r#"module spec.cpr.callwrappedownersecond;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: u8[] = bytes((c ? b : d).v);
    d.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, false);
}
"#;
    compile_rejects("callwrappedownersecond", second, "E10031", 1);

    let twin = r#"module spec.cpr.callwrappedownertwin;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, c: bool): i32 {
    let s: u8[] = bytes(b.v);
    b.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 { return 0; }
"#;
    compile_rejects("callwrappedownertwin", twin, "E10031", 1);
}

// ---------------------------------------------------------------------------
// elementpositions: view-array literals in argument and field positions.
//
// A view array whose element derives from a call temporary adopts the
// element's backing for the surrounding scope, exactly as the direct binding
// `let vs: str[1] = [mk().v];` does. The argument and struct-literal field
// positions must adopt the same backing; otherwise the temporary is never
// released and its storage leaks while the view stays readable.
// ---------------------------------------------------------------------------

/// A view-array literal in an argument or field position adopts its temporary
/// element backing, so the run is memory-clean.
///
/// `use([mk().v])` passes the array literal as an argument, and
/// `Holder { names: [mk().v] }` initializes a field of a local. Each element
/// derives from a call temporary, so each backing must be released after the
/// view's last use. Both programs are accepted, exit 0, and are clean.
#[test]
fn should_adopt_the_element_backing_of_view_arrays_in_argument_and_field_positions() {
    let argument = r#"module spec.cpr.elementpositionsarg;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(names: str[1]): i32 { return i32(len(names[0])); }

func main(): i32 {
    return use([mk().v]) - 2;
}
"#;
    let output = run_memory("elementpositionsarg", argument);
    assert_no_asan("elementpositionsarg", &output);
    assert_exit("elementpositionsarg", &output, 0);

    let field = r#"module spec.cpr.elementpositionsfield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func main(): i32 {
    let h: Holder = Holder { names: [mk().v] };
    return i32(len(h.names[0])) - 2;
}
"#;
    let output = run_memory("elementpositionsfield", field);
    assert_no_asan("elementpositionsfield", &output);
    assert_exit("elementpositionsfield", &output, 0);
}

// ---------------------------------------------------------------------------
// arrayreturn: the `str[N]` return spelling.
//
// A `str[1]` return of one view element must be judged exactly like the
// single-view return of the same derivation. A view derived from a handle
// parameter is accepted by the single spelling, so the array spelling is
// accepted too; a derivation that reports for the single spelling reports the
// same code once for the array.
// ---------------------------------------------------------------------------

/// A `str[1]` return matches its single-view twin's judgment.
///
/// `return [s]` derives its element from the by-value `String` parameter `s`,
/// exactly as the accepted `return s` derives the single view, so the program
/// is accepted, exits 0, and is clean. `return [h.v]` derives from the
/// parameter `h`, whose twin `return h.v` reports one `E10036` (zero view-like
/// lend candidates); the array spelling keeps exactly that one report and
/// adds no release-borrowed `E10034`.
#[test]
fn should_treat_a_view_array_return_like_its_single_view_twin() {
    let view = r#"module spec.cpr.arrayreturnparam;

import { String, stringOf } from std.text;

func f(s: String): str[1] { return [s]; }

func main(): i32 {
    let s: String = stringOf("ab");
    let a: str[1] = f(s);
    return i32(len(a[0])) - 2;
}
"#;
    let output = run_memory("arrayreturnparam", view);
    assert_no_asan("arrayreturnparam", &output);
    assert_exit("arrayreturnparam", &output, 0);

    let twin = r#"module spec.cpr.arrayreturnmembertwin;

import { String } from std.text;

struct Box { v: String; }

func f(h: Box): str { return h.v; }

func main(): i32 { return 0; }
"#;
    compile_rejects("arrayreturnmembertwin", twin, "E10036", 1);

    let member = r#"module spec.cpr.arrayreturnmember;

import { String } from std.text;

struct Box { v: String; }

func f(h: Box): str[1] { return [h.v]; }

func main(): i32 { return 0; }
"#;
    compile_rejects("arrayreturnmember", member, "E10036", 1);
}

// ---------------------------------------------------------------------------
// onereport: one report for the mixed local/generated view return.
//
// A return whose mixed conditional has a local-backed arm beside a
// generated-owner arm reports both the local-owner `E10035` and the
// generated-owner escape `E10030` at one site. The site keeps exactly one
// report: the local-owner judgment is withheld and the escape carries the
// rejection. A conditional whose local backing is the only escape (local plus
// parameter, or local arms alone) keeps its single `E10035`.
// ---------------------------------------------------------------------------

/// A mixed local/generated view return reports exactly one error: the
/// generated-owner escape.
///
/// `return (c ? a : mkBox()).v` returns a view of the local `a` on one path
/// and of the fresh `mkBox()` root on the other. The fresh root's release
/// makes the escape `E10030`; the local-owner `E10035` is withheld at that
/// site, so the program reports exactly one error. The controls
/// `return (c ? b.v : a)` (a local `b` and a parameter `a`) and the two-local
/// conditional keep their single `E10035`.
#[test]
fn should_report_once_at_a_mixed_local_and_generated_view_return() {
    let mixed = r#"module spec.cpr.onereportmixed;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    return (c ? a : mkBox()).v;
}

func main(): i32 { return 0; }
"#;
    compile_rejects("onereportmixed", mixed, "E10030", 1);

    let localparam = r#"module spec.cpr.onereportlocalparam;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool, a: str): str {
    let b: Box = Box { v: stringOf("cd") };
    return (c ? b.v : a);
}

func main(): i32 { return 0; }
"#;
    compile_rejects("onereportlocalparam", localparam, "E10035", 1);

    let localonly = r#"module spec.cpr.onereportlocalonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    let b: Box = Box { v: stringOf("ef") };
    return (c ? a : b).v;
}

func main(): i32 { return 0; }
"#;
    compile_rejects("onereportlocalonly", localonly, "E10035", 1);
}

// ---------------------------------------------------------------------------
// reverseorder: the reversed-order conditional view return (green control).
//
// The per-arm conversion emits the owned arm on its own path, and it covers
// both arm orders, so a conditional whose owned member arm comes second
// builds and runs. The control pins the order against a regression.
// ---------------------------------------------------------------------------

/// A conditional view return with the owned arm second stays accepted and
/// memory-clean.
///
/// `return c ? x : h.v` converts the owned member arm `h.v` on its own path
/// and returns the `str` parameter `x` on the other. The strict host build
/// accepts the emitted C, the runs select each arm once, and the program exits
/// 0 with no memory report.
#[test]
fn should_accept_the_reversed_order_of_a_conditional_view_return() {
    let source = r#"module spec.cpr.reverseorder;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(x: str, h: Box, c: bool): str {
    return c ? x : h.v;
}

func main(): i32 {
    let h: Box = Box { v: stringOf("abcd") };
    return i32(len(pick("zz", h, true))) + i32(len(pick("xyzw", h, false))) - 6;
}
"#;
    let output = run_memory("reverseorder", source);
    assert_no_asan("reverseorder", &output);
    assert_exit("reverseorder", &output, 0);
}

// ---------------------------------------------------------------------------
// ownertwins: chained view-returning callees and the two-arm member walk.
//
// A view-returning callee inherits its result's owners from the argument the
// view derives from, and a conditional argument contributes every arm's
// owner. The direct spelling `let s: str = (c ? b : d).v;` already loans both
// parameter backings; the chained spelling `peek((c ? b : d).v)` and the
// two-arm member spelling `bytes(c ? b.v : d.v)` must match it, so a write to
// the selected arm's backing while the view is live reports `E10031` once.
// ---------------------------------------------------------------------------

/// A chained view-returning callee and a two-arm member walk record every
/// arm's owner, so a write to the selected backing reports once.
///
/// The direct twin `let s: str = (c ? b : d).v;` reports the mutation; the
/// chained call through `peek(x: str): str` must report the same judgment for
/// the same write. The two-arm walk `bytes(c ? b.v : d.v)` writes the second
/// arm's backing, which the first-arm-only walk misses. Each program reports
/// exactly one `E10031` at the mutation and emits no translation unit.
#[test]
fn should_record_the_chained_and_two_arm_view_owners() {
    let chained = r#"module spec.cpr.ownertwinschained;

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
    compile_rejects("ownertwinschained", chained, "E10031", 1);

    let twoarm = r#"module spec.cpr.ownertwinstwoarm;

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
    compile_rejects("ownertwinstwoarm", twoarm, "E10031", 1);

    let direct = r#"module spec.cpr.ownertwinsdirect;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: str = (c ? b : d).v;
    d.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, false);
}
"#;
    compile_rejects("ownertwinsdirect", direct, "E10031", 1);
}

// ---------------------------------------------------------------------------
// statereuse: releasing an adopted array field on every guarded execution.
//
// A callable receiver's `Array<String>` field adopts a projection of a fresh
// holder, and the array's drop has no expression form, so a release of the
// previous carrier value is hoisted before the guarded construct. A statement
// that runs again — a loop iteration, or a skipped second evaluation — must
// not free the adopted storage the receiver's own release already freed. The
// `?:` arm spelling keeps the same carrier and must stay clean too.
// ---------------------------------------------------------------------------

/// A guarded callable receiver's adopted array field releases exactly once per
/// statement execution.
///
/// `H { call: addOne, s: mkHolder().data }.call(5)` inside the `g &&` guard
/// adopts the fresh holder's `data` field, and the receiver's own release is
/// the sole owner of that array. The loop executes the statement three times,
/// so a release of the previous carrier value hoisted before the guard would
/// free the adopted storage before the next fill; each call takes the guarded
/// path and the accumulated returns cancel the trailing constant. The `?:` arm
/// spelling re-executes the same carrier and must stay clean as well.
#[test]
fn should_release_a_guarded_array_field_on_every_statement_execution() {
    let guarded = r#"module spec.cpr.statereuse;

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
    let output = run_memory("statereuse", guarded);
    assert_no_asan("statereuse", &output);
    assert_exit("statereuse", &output, 0);

    let arm = r#"module spec.cpr.armstatreuse;

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

func use(c: bool, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        let r: i32 = c ? H { call: addOne, s: mkHolder().data }.call(5) : 0;
        n = n + (r - 6);
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return use(true, 3);
}
"#;
    let output = run_memory("armstatreuse", arm);
    assert_no_asan("armstatreuse", &output);
    assert_exit("armstatreuse", &output, 0);
}

// ---------------------------------------------------------------------------
// generictake: the generic take projection moves its member per instance.
//
// The take argument `(c ? a : mkPair<T>(x, y)).v` is judged per instantiation:
// an owning `T` moves the chosen member into the callee (the source is
// emptied), while a duplicable `T` copies it. The eager and the guarded
// spellings share one carrier, and the loop re-executes both per iteration.
// ---------------------------------------------------------------------------

/// A generic take projection moves its chosen member per instance.
///
/// `hold<T>((c ? a : mkPair<T>(x, y)).v)` transfers the selected member to the
/// callee: the `String` instances move the source out of the chosen root and
/// the `i32` instances copy, so every run is leak-free and neither arm's root
/// frees the transferred value. Both the eager and the guarded spelling run in
/// a loop, and the accumulated returns cancel the trailing constant.
#[test]
fn should_release_a_generic_take_projection_per_instance() {
    let source = r#"module spec.cpr.generictake;

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
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
        n = n + eager<String>(true, p, stringOf("ij"), stringOf("kl"));
        let q: Pair<String> = Pair<String> { v: stringOf("ef"), w: stringOf("gh") };
        n = n + eager<String>(false, q, stringOf("mn"), stringOf("op"));
        let r: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
        n = n + guarded<String>(true, r, stringOf("ij"), stringOf("kl"), true);
        let s: Pair<String> = Pair<String> { v: stringOf("ef"), w: stringOf("gh") };
        n = n + guarded<String>(false, s, stringOf("mn"), stringOf("op"), true);
        let a: Pair<i32> = Pair<i32> { v: 1, w: 2 };
        n = n + eager<i32>(true, a, 5, 6);
        let b: Pair<i32> = Pair<i32> { v: 3, w: 4 };
        n = n + eager<i32>(false, b, 7, 8);
        let c2: Pair<i32> = Pair<i32> { v: 1, w: 2 };
        n = n + guarded<i32>(true, c2, 5, 6, true);
        let d2: Pair<i32> = Pair<i32> { v: 3, w: 4 };
        n = n + guarded<i32>(false, d2, 7, 8, true);
        i = i + 1;
    }
    return n - 24;
}
"#;
    let output = run_memory("generictake", source);
    assert_no_asan("generictake", &output);
    assert_exit("generictake", &output, 0);
}

// ---------------------------------------------------------------------------
// nestedtake: a nested conditional take moves at every level.
//
// A conditional arm that is itself a conditional carries the same projection,
// so each level must move its own chosen member. When only the outer level
// moved, the inner arm's root kept and released the transferred storage, a
// double free. The then-nested, else-nested, and both-nested spellings all run
// every selection the four combinations name.
// ---------------------------------------------------------------------------

/// A nested conditional take empties the chosen member at every level.
///
/// `consume((c ? (d ? a : mkBox()) : b).v)` and its two mirror spellings move
/// the member the selected leaf names: the projection is rebuilt over each
/// nested arm, so the chosen root releases only its un-moved fields. Every
/// selection returns 1 and the four calls cancel the trailing constant.
#[test]
fn should_release_a_nested_conditional_take_at_every_level() {
    let then_nested = r#"module spec.cpr.nestedthen;

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
    let n: i32 = 0;
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(true, true, a, b, true);
    let a2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(true, false, a2, b2, true);
    let a3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(false, true, a3, b3, true);
    let a4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(false, false, a4, b4, true);
    return n - 4;
}
"#;
    let output = run_memory("nestedthen", then_nested);
    assert_no_asan("nestedthen", &output);
    assert_exit("nestedthen", &output, 0);

    let else_nested = r#"module spec.cpr.nestedelse;

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
    let n: i32 = 0;
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(true, true, a, b, true);
    let a2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(true, false, a2, b2, true);
    let a3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(false, true, a3, b3, true);
    let a4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(false, false, a4, b4, true);
    return n - 4;
}
"#;
    let output = run_memory("nestedelse", else_nested);
    assert_no_asan("nestedelse", &output);
    assert_exit("nestedelse", &output, 0);

    let both_nested = r#"module spec.cpr.nestedboth;

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
    let n: i32 = 0;
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(true, true, a, b, true);
    let a2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(true, false, a2, b2, true);
    let a3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(false, true, a3, b3, true);
    let a4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    n = n + use(false, false, a4, b4, true);
    return n - 4;
}
"#;
    let output = run_memory("nestedboth", both_nested);
    assert_no_asan("nestedboth", &output);
    assert_exit("nestedboth", &output, 0);
}

// ---------------------------------------------------------------------------
// genericreturn: the generic return projection moves its member per instance.
//
// A returned projection `(c ? a : mkPair<T>(x, y)).v` moves the selected
// member into the return slot: the place arm's field empties, the fresh root
// releases only its un-moved field, and a duplicable instance copies. The loop
// re-executes both arms for `String` and `i32`.
// ---------------------------------------------------------------------------

/// A generic return projection moves its chosen member per instance.
///
/// `return (c ? a : mkPair<T>(x, y)).v` gives the caller exactly one owner:
/// the `String` instances move the selected member out of the chosen root and
/// the root's sibling releases, the `i32` instances copy, and the accumulated
/// lengths and values cancel the trailing constant.
#[test]
fn should_release_a_generic_return_projection_per_instance() {
    let source = r#"module spec.cpr.genericreturn;

import { String, stringOf } from std.text;

struct Pair<T> { v: T; w: T; }

func mkPair<T>(take x: T, take y: T): Pair<T> { return Pair<T> { v: x, w: y }; }

func pick<T>(c: bool, take a: Pair<T>, take x: T, take y: T): T {
    return (c ? a : mkPair<T>(x, y)).v;
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
        let q: Pair<String> = Pair<String> { v: stringOf("ef"), w: stringOf("gh") };
        let s: String = pick<String>(true, p, stringOf("ij"), stringOf("kl"));
        let t: String = pick<String>(false, q, stringOf("mn"), stringOf("op"));
        let a: Pair<i32> = Pair<i32> { v: 1, w: 2 };
        let b: Pair<i32> = Pair<i32> { v: 3, w: 4 };
        let u: i32 = pick<i32>(true, a, 5, 6);
        let v: i32 = pick<i32>(false, b, 7, 8);
        n = n + (i32(len(s)) + i32(len(t)) + u + v - 12);
        i = i + 1;
    }
    return n;
}
"#;
    let output = run_memory("genericreturn", source);
    assert_no_asan("genericreturn", &output);
    assert_exit("genericreturn", &output, 0);
}

// ---------------------------------------------------------------------------
// fieldcond: a conditional projection in an aggregate-literal field.
//
// A callable receiver whose field projects a conditional must move the chosen
// member into the field and empty its source, in the guarded and the eager
// spelling alike; a plain literal field whose value is a conditional of
// projections must bind each fresh arm's root so its un-moved fields release.
// ---------------------------------------------------------------------------

/// A callable receiver's conditional projection field moves the chosen member.
///
/// `H { call: addOne, s: (c ? a : mkBox()).v }.call(5)` fills the field from
/// the chosen arm and empties that arm's member, so the receiver's own release
/// is the sole owner. The guarded spelling defers the fill to the guarded
/// expression, the eager spelling consumes through branch statements, and every
/// selection returns 6.
#[test]
fn should_release_a_conditional_projection_field_of_a_callable_receiver() {
    let guarded = r#"module spec.cpr.fieldcondguarded;

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
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a, true) + use(false, b, true) - 2;
}
"#;
    let output = run_memory("fieldcondguarded", guarded);
    assert_no_asan("fieldcondguarded", &output);
    assert_exit("fieldcondguarded", &output, 0);

    let eager = r#"module spec.cpr.fieldcondeager;

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
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b) - 12;
}
"#;
    let output = run_memory("fieldcondeager", eager);
    assert_no_asan("fieldcondeager", &output);
    assert_exit("fieldcondeager", &output, 0);
}

/// A literal field holding a conditional of projections releases each fresh
/// arm's root.
///
/// `Out { v: c ? a.v : mkBox().v, ... }` moves the chosen member into the field
/// through the statement-form store, and the fresh arm's root releases only its
/// un-moved field, so the run is leak-free while the selected value is read.
#[test]
fn should_release_a_conditional_field_of_a_struct_literal() {
    let source = r#"module spec.cpr.fieldcondplain;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Out { v: String; w: String; }

func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool, take a: Box): i32 {
    let o: Out = Out { v: c ? a.v : mkBox().v, w: stringOf("z") };
    return i32(len(o.v)) - 2;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b);
}
"#;
    let output = run_memory("fieldcondplain", source);
    assert_no_asan("fieldcondplain", &output);
    assert_exit("fieldcondplain", &output, 0);
}

// ---------------------------------------------------------------------------
// condpositions: a conditional of projections in the remaining consuming
// positions.
//
// An array-literal element, an eager `take` argument, and an owning return all
// consume the selected member, so each must bind the fresh arm's root and
// release only its un-moved field. Before the fix each position leaked the
// root's sibling storage on every fresh-arm selection.
// ---------------------------------------------------------------------------

/// A conditional projection element, take argument, and return release their
/// fresh root's residual, in the eager and the deferred position alike.
///
/// `[c ? a.v : mkBox().v, ...]` stores the chosen member into the array,
/// `consume(c ? a.v : mkBox().v)` transfers it to the callee, and
/// `return c ? a.v : mkBox().v` transfers it to the caller; each position
/// releases the fresh root's un-moved field once. A guarded receiver field and
/// a `?:` arm element fill one adopting carrier per arm on their evaluated
/// path, so the same rule holds where the literal is conditionally evaluated.
/// Every selection exits 0 and is clean.
#[test]
fn should_release_a_conditional_projection_in_the_remaining_consuming_positions() {
    let element = r#"module spec.cpr.elementcond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool, take a: Box): i32 {
    let arr: String[2] = [c ? a.v : mkBox().v, stringOf("z")];
    return i32(len(arr[0])) - 2;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b);
}
"#;
    let output = run_memory("elementcond", element);
    assert_no_asan("elementcond", &output);
    assert_exit("elementcond", &output, 0);

    let take = r#"module spec.cpr.takecond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }

func use(c: bool, take a: Box): i32 {
    return consume(c ? a.v : mkBox().v);
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b) - 4;
}
"#;
    let output = run_memory("takecond", take);
    assert_no_asan("takecond", &output);
    assert_exit("takecond", &output, 0);

    let returned = r#"module spec.cpr.returncond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool, take a: Box): String {
    return c ? a.v : mkBox().v;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    let s: String = use(true, a);
    let t: String = use(false, b);
    return i32(len(s)) + i32(len(t)) - 4;
}
"#;
    let output = run_memory("returncond", returned);
    assert_no_asan("returncond", &output);
    assert_exit("returncond", &output, 0);

    let deferred_field = r#"module spec.cpr.guardedfieldcond;

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
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a, true) + use(false, b, true) - 2;
}
"#;
    let output = run_memory("guardedfieldcond", deferred_field);
    assert_no_asan("guardedfieldcond", &output);
    assert_exit("guardedfieldcond", &output, 0);

    let deferred_element = r#"module spec.cpr.armelemcond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func lenElem(names: String[2]): i32 { return i32(len(names[0])) - 2; }

func use(c: bool, take a: Box, g: bool): i32 {
    return g ? lenElem([c ? a.v : mkBox().v, stringOf("z")]) : 0;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a, true) + use(false, b, true);
}
"#;
    let output = run_memory("armelemcond", deferred_element);
    assert_no_asan("armelemcond", &output);
    assert_exit("armelemcond", &output, 0);
}

// ---------------------------------------------------------------------------
// guardedspells: the guarded take and receiver-field spellings beyond the
// committed cases.
//
// The committed guarded cases cover the taken `&&` path with both arms. These
// programs add the skipped guard, the `||` spelling (both the evaluated right
// side and the short circuit), the fresh-arm-only argument, and the loop
// condition, so every guarded evaluation path has a pinned run.
// ---------------------------------------------------------------------------

/// The skipped guard, `||`, fresh-arm-only, and loop-condition guarded take
/// spellings stay accepted and memory-clean.
///
/// A skipped `&&` guard evaluates no arm, the `||` spelling evaluates its right
/// side only when the left is false, the fresh-arm-only argument binds its call
/// root, and the loop condition re-fills the carrier per iteration. The
/// accumulated returns cancel the trailing constant.
#[test]
fn should_cover_the_guarded_take_spellings_beyond_the_pinned_cases() {
    let source = r#"module spec.cpr.guardtakespells;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }

func useSkip(c: bool, g: bool): i32 {
    if (g && consume((c ? Box { v: stringOf("xx"), w: stringOf("yy") } : mkBox()).v) > 0) {
        return 1;
    }
    return 0;
}

func useOr(c: bool, g: bool): i32 {
    if (g || consume((c ? Box { v: stringOf("xx"), w: stringOf("yy") } : mkBox()).v) > 0) {
        return 1;
    }
    return 0;
}

func useFresh(g: bool): i32 {
    if (g && consume(mkBox().v) > 0) {
        return 1;
    }
    return 0;
}

func useLoop(c: bool, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && consume((c ? Box { v: stringOf("xx"), w: stringOf("yy") } : mkBox()).v) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return useSkip(true, true) + useSkip(true, false)
         + useOr(true, false) + useOr(true, true)
         + useFresh(true)
         + useLoop(true, 3) + useLoop(false, 3)
         - 10;
}
"#;
    let output = run_memory("guardtakespells", source);
    assert_no_asan("guardtakespells", &output);
    assert_exit("guardtakespells", &output, 0);
}

/// A callable receiver field in a loop condition releases its root per
/// evaluation.
///
/// `while (n < k && H { call: addOne, s: mkBox().v }.call(5) > 0)` rebuilds the
/// receiver on every condition evaluation, and each fresh root releases its
/// un-moved field while the field's own release owns the member.
#[test]
fn should_release_a_callable_receiver_field_in_a_loop_condition() {
    let source = r#"module spec.cpr.fieldrootcond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }

func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }

func use(k: i32): i32 {
    let n: i32 = 0;
    while (n < k && H { call: addOne, s: mkBox().v }.call(5) > 0) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    return use(3) - 3;
}
"#;
    let output = run_memory("fieldrootcond", source);
    assert_no_asan("fieldrootcond", &output);
    assert_exit("fieldrootcond", &output, 0);
}

// ---------------------------------------------------------------------------
// literalfieldmove: a bound literal's owned field sources move into it.
//
// A conditionally evaluated struct-literal arm keeps its field sources in
// place while its arm is lowered, so binding the literal as an owner must
// move them: otherwise the literal's release and the source's own cleanup
// free the same storage twice. The eager take, the store, and the guarded
// take spellings all move the sibling field.
// ---------------------------------------------------------------------------

/// Binding a literal arm moves its owned sibling field source into it.
///
/// `c ? a.v : Out2 { v: stringOf("zz"), w: b }.v` binds the literal's root in
/// the eager take, the store, and the guarded take alike: the sibling field's
/// source empties as the literal takes it, so the literal's release is its
/// sole owner and the source's cleanup is a no-op. Every selection is
/// leak-free, and the accumulated results cancel the trailing constant.
#[test]
fn should_move_a_literal_arms_owned_field_sources() {
    let source = r#"module spec.cpr.literalfieldmove;

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
    let n: i32 = 0;
    let a1: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b1: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + eager(true, a1, b1);
    let a2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b2: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + eager(false, a2, b2);
    let a3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b3: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + store(true, a3, b3);
    let a4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b4: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + store(false, a4, b4);
    let a5: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b5: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + guarded(true, a5, b5, true);
    let a6: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b6: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + guarded(false, a6, b6, true);
    return n - 2;
}
"#;
    let output = run_memory("literalfieldmove", source);
    assert_no_asan("literalfieldmove", &output);
    assert_exit("literalfieldmove", &output, 0);
}

// ---------------------------------------------------------------------------
// nestedliteralmove: a bound literal moves a place reached at any depth.
//
// A nested literal field and an array-literal field element that name a place
// must move that place before the outer literal materializes: otherwise the
// outer literal's release and the source's cleanup free the same storage. The
// guarded take, the eager field, and the array-field element spellings all
// reach the leaf.
// ---------------------------------------------------------------------------

/// Binding a literal moves a place reached inside a nested field or element.
///
/// `Outer { inner: Inner { v: …, w: b.w }, … }.inner.v` binds the outer
/// literal's root, and the inner `w` moves `b.w` at its own depth; the array
/// field `[b.w, …]` moves its element the same way. Every selection is
/// leak-free, and the accumulated results cancel the trailing constant.
#[test]
fn should_move_a_nested_literals_field_sources() {
    let source = r#"module spec.cpr.nestedliteralmove;

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
    let n: i32 = 0;
    let a1: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b1: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + guarded(true, a1, b1, true);
    let a2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b2: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + guarded(false, a2, b2, true);
    let a3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b3: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + eagerField(true, a3, b3);
    let a4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b4: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + eagerField(false, a4, b4);
    let a5: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b5: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + arrayField(true, a5, b5, true);
    let a6: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b6: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    n = n + arrayField(false, a6, b6, true);
    return n - 4;
}
"#;
    let output = run_memory("nestedliteralmove", source);
    assert_no_asan("nestedliteralmove", &output);
    assert_exit("nestedliteralmove", &output, 0);
}

// ---------------------------------------------------------------------------
// carrierescape: an aggregate that carries views must not leave the frame.
//
// A struct whose view-like field names storage the frame adopted cannot be
// returned to the caller: the backing is released with the frame, so the
// caller would read freed storage. The carrier mirrors the view-typed twins:
// one `E10030` when the backing is compiler-generated, and one `E10035` when
// the carrier's view derives from a named local. The same-frame spellings
// stay accepted and memory-clean.
// ---------------------------------------------------------------------------

/// A returned carrier whose adopted backing dies with the frame reports one
/// generated-owner escape.
///
/// `Holder { names: [mk().v] }` adopts the fresh `mk()` root for the frame;
/// returning the carrier hands the caller views of storage the return
/// releases, so the let, the assign, and the direct literal spellings each
/// report exactly one `E10030` and emit no translation unit. Each caller
/// reads the backing's bytes, so an accepted program would surface the read
/// of freed storage under AddressSanitizer.
#[test]
fn should_reject_a_returned_carrier_whose_backing_dies_with_the_frame() {
    let let_carrier = r#"module spec.cpr.carrierretlet;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func f(): Holder {
    let h: Holder = Holder { names: [mk().v] };
    return h;
}

func main(): i32 {
    let h: Holder = f();
    return i32(h.names[0][0]) - 97;
}
"#;
    compile_rejects("carrierretlet", let_carrier, "E10030", 1);

    let assign_carrier = r#"module spec.cpr.carrierretassign;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func f(): Holder {
    let h: Holder = Holder { names: ["zz"] };
    h = Holder { names: [mk().v] };
    return h;
}

func main(): i32 {
    let h: Holder = f();
    return i32(h.names[0][0]) - 97;
}
"#;
    compile_rejects("carrierretassign", assign_carrier, "E10030", 1);

    let literal_carrier = r#"module spec.cpr.carrierretlit;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func f(): Holder {
    return Holder { names: [mk().v] };
}

func main(): i32 {
    let h: Holder = f();
    return i32(h.names[0][0]) - 97;
}
"#;
    compile_rejects("carrierretlit", literal_carrier, "E10030", 1);
}

/// A carrier return whose view derives from a named local keeps the
/// local-owner report, exactly as the view-typed twin does.
///
/// `let names: str[1] = [mk().v]; return names;` reports one `E10035` for
/// the local view binding, and a view array read out of a local carrier
/// (`h.names`) reports the same single local-owner judgment. Neither emits
/// a translation unit.
#[test]
fn should_keep_the_local_owner_report_of_a_carrier_return() {
    let view_twin = r#"module spec.cpr.carrierviewtwin;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func f(): str[1] {
    let names: str[1] = [mk().v];
    return names;
}

func main(): i32 { return 0; }
"#;
    compile_rejects("carrierviewtwin", view_twin, "E10035", 1);

    let local_carrier = r#"module spec.cpr.carrierlocaltwin;

import { String, stringOf } from std.text;

struct Holder { names: str[1]; }

func f(): str[1] {
    let h: Holder = Holder { names: ["zz"] };
    let names: str[1] = h.names;
    return names;
}

func main(): i32 { return 0; }
"#;
    compile_rejects("carrierlocaltwin", local_carrier, "E10035", 1);
}

/// A carrier stored into a destination that outlives the statement's block
/// keeps its backing alive to the destination's use.
///
/// The block, `if`, and loop spellings all assign a fresh-backed carrier to a
/// binding declared in an enclosing block; the adopted backing binds to the
/// function frame, so the read after the block is valid and the runs exit 0
/// with no memory report.
#[test]
fn should_store_a_carrier_backing_into_the_destination_frame() {
    let block_store = r#"module spec.cpr.blockcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func main(): i32 {
    let h: Holder = Holder { names: ["ab"] };
    {
        h = Holder { names: [mk().v] };
    }
    return i32(h.names[0][0]) - 97;
}
"#;
    let output = run_memory("blockcarrier", block_store);
    assert_no_asan("blockcarrier", &output);
    assert_exit("blockcarrier", &output, 0);

    let if_store = r#"module spec.cpr.ifcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func main(): i32 {
    let c: bool = true;
    let h: Holder = Holder { names: ["ab"] };
    if (c) {
        h = Holder { names: [mk().v] };
    } else {
        h = Holder { names: ["zz"] };
    }
    return i32(h.names[0][0]) - 97;
}
"#;
    let output = run_memory("ifcarrier", if_store);
    assert_no_asan("ifcarrier", &output);
    assert_exit("ifcarrier", &output, 0);

    let loop_store = r#"module spec.cpr.loopcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func main(): i32 {
    let h: Holder = Holder { names: ["ab"] };
    let i: i32 = 0;
    while (i < 2) {
        h = Holder { names: [mk().v] };
        i = i + 1;
    }
    return i32(h.names[0][0]) - 97;
}
"#;
    let output = run_memory("loopcarrier", loop_store);
    assert_no_asan("loopcarrier", &output);
    assert_exit("loopcarrier", &output, 0);
}

// ---------------------------------------------------------------------------
// guardedelement: a conditionally evaluated element source keeps its call on
// the evaluated path.
//
// The element's call counts its runs; the guarded programs select the arm
// that never runs it, so the count stays zero (and the loop condition's
// short circuit keeps the count at one per iteration). An eager materialize
// would run the call before the guard and the count would expose it.
// ---------------------------------------------------------------------------

/// A carrier field whose element source sits under a `?:` arm keeps its call
/// on the selected path.
///
/// `g ? Holder { names: [mk().v] } : Holder { names: ["zz"] }` with `g` false
/// returns the static arm, so `mk()` never runs and the count stays zero.
#[test]
fn should_not_run_a_conditional_carrier_element_before_its_guard() {
    let source = r#"module spec.cpr.condfieldcall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func f(g: bool): i32 {
    let h: Holder = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };
    return i32(len(h.names[0])) - 2;
}

func main(): i32 {
    let r: i32 = f(false);
    return calls * 10 + r;
}
"#;
    let output = run_memory("condfieldcall", source);
    assert_no_asan("condfieldcall", &output);
    assert_exit("condfieldcall", &output, 0);
}

/// A view-element array argument keeps its call on the guarded path.
///
/// The `&&`, `?:`, and loop-condition spellings all short-circuit around the
/// argument with the counter false or exhausted, so `mk()` runs zero times
/// (three times for the loop's three taken iterations) and the programs exit
/// with the expected count.
#[test]
fn should_not_run_a_guarded_array_argument_before_its_guard() {
    let and_argument = r#"module spec.cpr.guardargcall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(names: str[1]): i32 { return i32(len(names[0])); }

func f(g: bool): i32 {
    if (g && use([mk().v]) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let r: i32 = f(false);
    return calls * 10 + r;
}
"#;
    let output = run_memory("guardargcall", and_argument);
    assert_no_asan("guardargcall", &output);
    assert_exit("guardargcall", &output, 0);

    let cond_argument = r#"module spec.cpr.condargcall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(names: str[1]): i32 { return i32(len(names[0])); }

func f(g: bool): i32 {
    return g ? use([mk().v]) : 0;
}

func main(): i32 {
    let r: i32 = f(false);
    return calls * 10 + r;
}
"#;
    let output = run_memory("condargcall", cond_argument);
    assert_no_asan("condargcall", &output);
    assert_exit("condargcall", &output, 0);

    let loop_argument = r#"module spec.cpr.loopargcall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(names: str[1]): i32 { return i32(len(names[0])); }

func h(k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && use([mk().v]) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    let r: i32 = h(3);
    return calls * 10 + r - 3;
}
"#;
    let output = run_memory("loopargcall", loop_argument);
    assert_no_asan("loopargcall", &output);
    assert_exit("loopargcall", &output, 30);
}

/// A struct-literal argument whose view-like field derives from a call keeps
/// its backing alive for the call.
///
/// `use(Holder { names: [mk().v] })` passes the literal containing the view
/// array; its temporary element's backing adopts for the frame, so the callee
/// reads live storage and the run exits 0 with no memory report.
#[test]
fn should_adopt_a_view_element_field_of_a_struct_literal_argument() {
    let source = r#"module spec.cpr.holderargcall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(h: Holder): i32 { return i32(h.names[0][0]); }

func main(): i32 {
    return use(Holder { names: [mk().v] }) - 97;
}
"#;
    let output = run_memory("holderargcall", source);
    assert_no_asan("holderargcall", &output);
    assert_exit("holderargcall", &output, 0);
}

// ---------------------------------------------------------------------------
// carrierviacall: a returned carrier whose field projects through a
// view-returning call.
//
// The call's result reborrows the storage of its argument, so a carrier built
// from `first(Holder { names: [mk().v] }.names)` holds views into the inner
// literal's adopted backing exactly as the direct projection does. The return
// must report the generated-owner escape once; the direct-projection twin is
// the control.
// ---------------------------------------------------------------------------

/// A carrier returned through a view-returning call reports the same
/// generated-owner escape as its direct-projection twin.
///
/// `first(...)` reborrows the argument's single view-element field, whose
/// temporary root the frame adopted; returning the carrier hands the caller
/// views of storage the return releases, so the program reports exactly one
/// `E10030` and emits no translation unit.
#[test]
fn should_reject_a_returned_carrier_through_a_view_call() {
    let source = r#"module spec.cpr.carrierretcall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func first(a: str[1]): str { return a[0]; }

func f(): Holder {
    let h: Holder = Holder { names: [first(Holder { names: [mk().v] }.names)] };
    return h;
}

func main(): i32 {
    let h: Holder = f();
    return i32(h.names[0][0]) - 97;
}
"#;
    compile_rejects("carrierretcall", source, "E10030", 1);
}

// ---------------------------------------------------------------------------
// guardedelementtaken: the taken path of a guarded element source.
//
// A guarded array argument defers its fill to the evaluated path; the taken
// path must still release the element's backing exactly once, and the count
// folds into the exit code so an eager or an absent run both fail.
// ---------------------------------------------------------------------------

/// A guarded array argument releases its element backing on the taken path.
///
/// With the guard true each spelling runs the element's call once, uses the
/// view, and releases the backing when the statement ends: the `&&` and `?:`
/// programs exit 0 with the call counted once and no memory report.
#[test]
fn should_release_a_guarded_array_argument_on_its_taken_path() {
    let and_taken = r#"module spec.cpr.guardargtaken;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(names: str[1]): i32 { return i32(len(names[0])); }

func f(g: bool): i32 {
    if (g && use([mk().v]) > 0) {
        return calls;
    }
    return calls;
}

func main(): i32 {
    return f(true) - 1;
}
"#;
    let output = run_memory("guardargtaken", and_taken);
    assert_no_asan("guardargtaken", &output);
    assert_exit("guardargtaken", &output, 0);

    let cond_taken = r#"module spec.cpr.condargtaken;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(names: str[1]): i32 { return i32(len(names[0])); }

func f(g: bool): i32 {
    let r: i32 = g ? use([mk().v]) : 0;
    return calls * 10 + r;
}

func main(): i32 {
    return f(true) - 12;
}
"#;
    let output = run_memory("condargtaken", cond_taken);
    assert_no_asan("condargtaken", &output);
    assert_exit("condargtaken", &output, 0);
}

// ---------------------------------------------------------------------------
// guardedelementagg: the guarded struct-literal argument.
//
// A struct-literal argument containing a view-element field runs its element's
// call only on the guard's path and releases the backing when the statement
// ends. The skipped spellings count zero calls; the taken spellings count one
// and read the correct byte.
// ---------------------------------------------------------------------------

/// A guarded struct-literal argument runs its field's call only on the
/// evaluated path, once, and releases the backing.
///
/// `g && use(Holder { names: [mk().v] })`, the `?:` spelling, and the `||`
/// spelling each skip the call when their guard short-circuits and run it
/// exactly once on the taken path. The taken programs also read the backing's
/// byte, so a lost or leaked backing fails the exit code or the memory report.
#[test]
fn should_not_run_a_guarded_struct_literal_argument_before_its_guard() {
    let and_skipped = r#"module spec.cpr.guardaggandskipped;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(h.names[0][0]); }

func f(g: bool): i32 {
    if (g && use(Holder { names: [mk().v] }) > 0) {
        return calls;
    }
    return calls;
}

func main(): i32 {
    return f(false);
}
"#;
    let output = run_memory("guardaggandskipped", and_skipped);
    assert_no_asan("guardaggandskipped", &output);
    assert_exit("guardaggandskipped", &output, 0);

    let and_taken = r#"module spec.cpr.guardaggtaken;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(h.names[0][0]); }

func f(g: bool): i32 {
    if (g && use(Holder { names: [mk().v] }) > 0) {
        return calls;
    }
    return calls;
}

func main(): i32 {
    return f(true) - 1;
}
"#;
    let output = run_memory("guardaggtaken", and_taken);
    assert_no_asan("guardaggtaken", &output);
    assert_exit("guardaggtaken", &output, 0);

    let cond_skipped = r#"module spec.cpr.guardaggcondskipped;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(h.names[0][0]); }

func f(g: bool): i32 {
    let v: i32 = g ? use(Holder { names: [mk().v] }) : 0;
    return calls * 10 + v;
}

func main(): i32 {
    return f(false);
}
"#;
    let output = run_memory("guardaggcondskipped", cond_skipped);
    assert_no_asan("guardaggcondskipped", &output);
    assert_exit("guardaggcondskipped", &output, 0);

    let cond_taken = r#"module spec.cpr.guardaggcondtaken;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(h.names[0][0]); }

func f(g: bool): i32 {
    let v: i32 = g ? use(Holder { names: [mk().v] }) : 0;
    return calls * 10 + v;
}

func main(): i32 {
    return f(true) - 107;
}
"#;
    let output = run_memory("guardaggcondtaken", cond_taken);
    assert_no_asan("guardaggcondtaken", &output);
    assert_exit("guardaggcondtaken", &output, 0);

    let or_skipped = r#"module spec.cpr.guardaggorskipped;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(h.names[0][0]); }

func f(g: bool): i32 {
    if (g || use(Holder { names: [mk().v] }) > 0) {
        return calls;
    }
    return calls;
}

func main(): i32 {
    return f(true);
}
"#;
    let output = run_memory("guardaggorskipped", or_skipped);
    assert_no_asan("guardaggorskipped", &output);
    assert_exit("guardaggorskipped", &output, 0);

    let or_taken = r#"module spec.cpr.guardaggortaken;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(h.names[0][0]); }

func f(g: bool): i32 {
    if (g || use(Holder { names: [mk().v] }) > 0) {
        return calls;
    }
    return calls;
}

func main(): i32 {
    return f(false) - 1;
}
"#;
    let output = run_memory("guardaggortaken", or_taken);
    assert_no_asan("guardaggortaken", &output);
    assert_exit("guardaggortaken", &output, 0);
}

// ---------------------------------------------------------------------------
// chainonereport: a the reborrow chain of a mixed conditional view return.
//
// The direct spelling `(c ? a : mkBox()).v` reports one generated-owner
// escape. A returned view through a reborrow callee (or the `bytes` view
// builder) derives from the same conditional, so it reports the same single
// `E10030`.
// ---------------------------------------------------------------------------

/// A reborrow-chained mixed conditional view return reports one `E10030`,
/// like its direct spelling.
///
/// `peek((c ? a : mkBox()).v)` and `bytes((c ? a : mkBox()).v)` both reborrow
/// the conditional's selected storage; each reports exactly one
/// generated-owner escape and emits no translation unit.
#[test]
fn should_report_the_chained_reborrow_view_return_once() {
    let chained = r#"module spec.cpr.chainonereport;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func peek(x: str): str { return x; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    return peek((c ? a : mkBox()).v);
}

func main(): i32 { return 0; }
"#;
    compile_rejects("chainonereport", chained, "E10030", 1);

    let bytes = r#"module spec.cpr.chainonereportbytes;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(c: bool): u8[] {
    let a: Box = Box { v: stringOf("cd") };
    return bytes((c ? a : mkBox()).v);
}

func main(): i32 { return 0; }
"#;
    compile_rejects("chainonereportbytes", bytes, "E10030", 1);
}

// ---------------------------------------------------------------------------
// guardednestkit: a guarded aggregate whose fixed-array field holds a struct
// literal that names its own view-element field.
//
// The nested literal must store field by field at every depth: a compound
// initializer would brace-elide the inner element's own fixed-array field,
// which fails the strict build and drops the stored view's length. The taken
// paths fold the read byte into the exit code; the skipped paths keep the
// element call at zero.
// ---------------------------------------------------------------------------

/// A guarded nested aggregate stores its inner view-element field field by
/// field on the taken path and skips it on the skipped path.
///
/// `Kit { parts: [Part { name: [mk().v] }] }` nests a fixed-array-of-struct
/// field inside a struct-literal element. Under `?:` and `&&` the taken path
/// runs the element's call once, reads the backing's byte (folded into the
/// exit code), and is memory-clean; the skipped path runs it zero times.
#[test]
fn should_store_a_nested_guarded_aggregate_field_by_field() {
    let cond_taken = r#"module spec.cpr.kitcondtaken;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Part { name: str[1]; }
struct Kit { parts: Part[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(k: Kit): i32 { return i32(k.parts[0].name[0][0]); }

func f(g: bool): i32 {
    let r: i32 = g ? use(Kit { parts: [Part { name: [mk().v] }] }) : 0;
    return calls * 10 + r;
}

func main(): i32 {
    return f(true) - 107;
}
"#;
    let output = run_memory("kitcondtaken", cond_taken);
    assert_no_asan("kitcondtaken", &output);
    assert_exit("kitcondtaken", &output, 0);

    let cond_skipped = r#"module spec.cpr.kitcondskipped;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Part { name: str[1]; }
struct Kit { parts: Part[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(k: Kit): i32 { return i32(k.parts[0].name[0][0]); }

func f(g: bool): i32 {
    let r: i32 = g ? use(Kit { parts: [Part { name: [mk().v] }] }) : 0;
    return calls * 10 + r;
}

func main(): i32 {
    return f(false);
}
"#;
    let output = run_memory("kitcondskipped", cond_skipped);
    assert_no_asan("kitcondskipped", &output);
    assert_exit("kitcondskipped", &output, 0);

    let and_taken = r#"module spec.cpr.kitandtaken;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Part { name: str[1]; }
struct Kit { parts: Part[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(k: Kit): i32 { return i32(k.parts[0].name[0][0]); }

func f(g: bool): i32 {
    if (g && use(Kit { parts: [Part { name: [mk().v] }] }) > 0) {
        return calls;
    }
    return calls;
}

func main(): i32 {
    return f(true) - 1;
}
"#;
    let output = run_memory("kitandtaken", and_taken);
    assert_no_asan("kitandtaken", &output);
    assert_exit("kitandtaken", &output, 0);

    let and_skipped = r#"module spec.cpr.kitandskipped;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Part { name: str[1]; }
struct Kit { parts: Part[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(k: Kit): i32 { return i32(k.parts[0].name[0][0]); }

func f(g: bool): i32 {
    if (g && use(Kit { parts: [Part { name: [mk().v] }] }) > 0) {
        return calls;
    }
    return calls;
}

func main(): i32 {
    return f(false);
}
"#;
    let output = run_memory("kitandskipped", and_skipped);
    assert_no_asan("kitandskipped", &output);
    assert_exit("kitandskipped", &output, 0);
}

// ---------------------------------------------------------------------------
// globalstore: a view stored into a global outlives its backing.
//
// A global binding lives for the whole program, so a view stored there must
// name module-level static storage. A view derived from a fresh call result or
// from a parameter does not outlive the global: the stored view dangles once
// the statement's temporary or the caller's frame ends. The frozen escape
// rule reports `E10033` once at each store, and the parameter-backed spelling
// is the control — a parameter does not outlive a global either, so it
// reports the same single error.
// ---------------------------------------------------------------------------

/// A fresh or parameter-backed view stored into a global binding reports one
/// escape error per store.
///
/// `names = mk().v;` stores a view of the fresh call result into the global
/// `names`: the call result's storage dies at the end of the store statement,
/// while the global keeps reading it for the rest of the program. The array
/// spelling `names = [mk().v];` derives each element the same way, so it
/// reports the same one `E10033`. The parameter-backed spelling `names = p.v;`
/// is the control: the parameter's frame ends before the global's, so it
/// reports the same code. Every spelling is rejected with exactly one error
/// and emits no translation unit.
#[test]
fn should_reject_a_fresh_or_parameter_backed_view_stored_into_a_global() {
    let view = r#"module spec.cpr.globalstoreview;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str = "start";

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func store(): void { names = mk().v; }

func main(): i32 {
    store();
    return i32(len(names)) - 2;
}
"#;
    compile_rejects("globalstoreview", view, "E10033", 1);

    let array = r#"module spec.cpr.globalstorearray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str[1] = ["start"];

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func store(): void { names = [mk().v]; }

func main(): i32 {
    store();
    return i32(len(names[0])) - 2;
}
"#;
    compile_rejects("globalstorearray", array, "E10033", 1);

    let parameter = r#"module spec.cpr.globalstoreparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str = "start";

func store(p: Box): void { names = p.v; }

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    store(b);
    return i32(len(names)) - 2;
}
"#;
    compile_rejects("globalstoreparam", parameter, "E10033", 1);
}

// ---------------------------------------------------------------------------
// outslotstore: a view stored into a writable parameter slot is judged against
// the frame the write reaches.
//
// A `borrow` slot holds storage the caller keeps: the caller's frame outlives
// the callee, so a view derived from a parameter is accepted and stays
// readable, while a view derived from a fresh call result or from a callee
// local dies before the caller's slot does and reports the loan-outlives error
// `E10030`. The nullable spelling is the by-address slot the caller reads; the
// non-nullable spelling receives the same judgment.
// ---------------------------------------------------------------------------

/// A fresh or local-backed view stored into a writable parameter slot reports
/// the loan-outlives error once, while the parameter-backed twins stay
/// accepted and memory-clean.
///
/// `slot = mk().v;` stores a view of the call result into the caller's slot;
/// the result's storage dies at the end of the statement, before the callee
/// returns. `slot = b.v;` stores a view of the callee's local `b`, which dies
/// when the callee returns while the caller's slot still reads it. Both report
/// one `E10030` and emit no translation unit. The parameter-backed twins
/// `slot = p.v;` differ by destination kind: a non-nullable writable view slot
/// is caller-retained, so it rejects the parameter-backed store with `E10033`;
/// a nullable `borrow T?` ownership slot reports the foreign-store `E10034`
/// once, and the caller reads the written view.
#[test]
fn should_reject_a_fresh_or_local_backed_view_stored_into_a_parameter_slot() {
    let fresh = r#"module spec.cpr.outslotfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func fill(borrow slot: str): void { slot = mk().v; }

func main(): i32 {
    let out: str = "aa";
    fill(borrow out);
    return i32(len(out)) - 2;
}
"#;
    compile_rejects("outslotfresh", fresh, "E10030", 1);

    let twin = r#"module spec.cpr.outslotpartwin;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(borrow slot: str, p: Box): i32 {
    slot = p.v;
    return i32(slot[0]);
}

func main(): i32 {
    let out: str = "aa";
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    return fill(borrow out, b) - 120;
}
"#;
    compile_rejects("outslotpartwin", twin, "E10033", 1);

    let wide = r#"module spec.cpr.outslotpartwide;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(borrow slot: str?, p: Box): void { slot = p.v; }

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    fill(borrow out, b);
    if (out == null) {
        return 9;
    }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("outslotpartwide", wide, "E10034", 1);

    let local = r#"module spec.cpr.outslotlocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(borrow slot: str): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    slot = b.v;
}

func main(): i32 {
    let out: str = "aa";
    fill(borrow out);
    return i32(len(out)) - 2;
}
"#;
    compile_rejects("outslotlocal", local, "E10030", 1);
}

// ---------------------------------------------------------------------------
// fieldstore: an array of views stored into a global field or a parameter slot.
//
// The whole-array spellings are judged per element. A global's field lives for
// the whole program, so a fresh element — a view of a call result — escapes
// its origin and reports `E10033`; the writable parameter slot is the caller's
// frame, where the same fresh element dies first and reports `E10030`.
// ---------------------------------------------------------------------------

/// A fresh view element stored into a global struct field reports `E10033`
/// once, and the same element stored into a parameter slot reports `E10030`
/// once.
///
/// `g.names = [mk().v];` stores a view-array literal whose element derives
/// from a call result; the result's storage dies at the end of the store
/// statement while the global field reads the element for the rest of the
/// program. `slot = [mk().v];` stores the same element through a writable
/// parameter slot, whose storage is the caller's frame: the backing still dies
/// before the slot's last read. Both stores are rejected with one error and
/// emit no translation unit.
#[test]
fn should_reject_a_fresh_view_element_stored_into_a_global_field_or_parameter_slot() {
    let global = r#"module spec.cpr.fieldstoreglobal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func store(): void { g.names = [mk().v]; }

func main(): i32 {
    store();
    return i32(len(g.names[0])) - 2;
}
"#;
    compile_rejects("fieldstoreglobal", global, "E10033", 1);

    let slot = r#"module spec.cpr.fieldstoreout;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func fill(borrow slot: str[1]): void { slot = [mk().v]; }

func main(): i32 {
    let out: str[1] = ["AA"];
    fill(borrow out);
    return i32(len(out[0])) - 2;
}
"#;
    compile_rejects("fieldstoreout", slot, "E10030", 1);
}

// ---------------------------------------------------------------------------
// localelement: a local view-array element accepts a backing from its own
// frame.
//
// A local `str[N]` dies with the frame that owns its backing, so an element
// store is judged against the backing's scope rather than rejected outright:
// a view of a local or of a parameter covers the array's own scope and is
// accepted. A global or a field outlives the frame, so its element store keeps
// reporting `E10033`.
// ---------------------------------------------------------------------------

/// A local-backed view stored into a local view-array element is accepted and
/// memory-clean, while the global and field spellings keep their escape
/// report.
///
/// `xs[1] = c ? b.v : a;` stores the widened conditional — the local `b`'s
/// member on one path, the `str` parameter `a` on the other — into a local
/// `str[2]`. Both backings outlive the array's own last read, so the store is
/// accepted, exits 0, and is clean. The named spelling `xs[1] = v;` takes the
/// same judgment. The same conditional stored into a global array element or a
/// global struct field element still reports one `E10033`, because the global
/// outlives the local backing.
#[test]
fn should_accept_a_local_backed_view_stored_into_a_local_array_element() {
    let conditional = r#"module spec.cpr.localelemcond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(c: bool, a: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = c ? b.v : a;
    return i32(xs[1][0]);
}

func main(): i32 {
    return pick(true, "ppp") + pick(false, "qqqq") - 233;
}
"#;
    let output = run_memory("localelemcond", conditional);
    assert_no_asan("localelemcond", &output);
    assert_exit("localelemcond", &output, 0);

    let named = r#"module spec.cpr.localelemnamed;

func use(v: str): i32 {
    let xs: str[2] = ["AA", "BB"];
    xs[1] = v;
    return i32(xs[1][0]);
}

func main(): i32 {
    return use("xyz") - 120;
}
"#;
    let output = run_memory("localelemnamed", named);
    assert_no_asan("localelemnamed", &output);
    assert_exit("localelemnamed", &output, 0);

    let global = r#"module spec.cpr.localelemglobal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str[2] = ["AA", "BB"];

func pick(c: bool, a: str): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    names[1] = c ? b.v : a;
}

func main(): i32 {
    pick(true, "ppp");
    return 0;
}
"#;
    compile_rejects("localelemglobal", global, "E10033", 1);

    let field = r#"module spec.cpr.localelemfield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func pick(c: bool, a: str): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    g.names[1] = c ? b.v : a;
}

func main(): i32 {
    pick(true, "ppp");
    return 0;
}
"#;
    compile_rejects("localelemfield", field, "E10033", 1);
}

// ---------------------------------------------------------------------------
// widenedstore: the handle-to-view widening on a local backing is judged too.
//
// `names = b.v;` and `slot = b.v;` store a `String` member into a `str?`
// destination, so the stored value is a view derived from the local `b`. The
// widening must not hide the escape: the global spelling reports `E10033` and
// the parameter-slot spelling reports `E10030`, each exactly once.
// ---------------------------------------------------------------------------

/// A `String` member widened into a `str?` global or parameter slot is judged
/// by its local backing, one report per site.
///
/// `names = b.v;` widens the local `b`'s member into the global `str?`
/// `names`, so the stored view escapes with the local frame and reports one
/// `E10033`. `slot = b.v;` writes the same widened view through a nullable
/// parameter slot: the caller's frame outlives the callee, but the local `b`
/// still dies first, so the store reports one `E10030`. Neither program emits
/// a translation unit.
#[test]
fn should_reject_a_local_backed_widened_view_stored_into_a_global_or_parameter_slot() {
    let global = r#"module spec.cpr.wideglobal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str? = null;

func store(): void {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    names = b.v;
}

func main(): i32 {
    store();
    if (names == null) {
        return 9;
    }
    return i32(names[0]) - 97;
}
"#;
    compile_rejects("wideglobal", global, "E10033", 1);

    let slot = r#"module spec.cpr.wideout;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    slot = b.v;
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    if (out == null) {
        return 9;
    }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("wideout", slot, "E10030", 1);
}

// ---------------------------------------------------------------------------
// loopstore: a backing from a loop iteration or a nested block does not cover
// the array's scope.
//
// The local element store is accepted only when the backing's scope covers the
// array's. A backing created inside a loop iteration dies at the end of that
// iteration, and a backing declared in a nested block dies at the block's end;
// a later read of the element reaches freed storage, so each store reports
// `E10030` once.
// ---------------------------------------------------------------------------

/// A loop-iteration or nested-block backing stored into a local view-array
/// element reports the loan-outlives error once.
///
/// `while (i < 3) { xs[1] = mk().v; }` stores a view of the iteration's call
/// result; the result dies when the iteration's statement ends, so a later
/// read of `xs[1]` reaches freed storage. The nested-block spelling stores a
/// view of a local declared inside the block, which dies at the block's end
/// while the array reads it afterwards. Each store reports exactly one
/// `E10030` and emits no translation unit.
#[test]
fn should_reject_a_loop_or_nested_scope_backing_stored_into_a_local_array_element() {
    let loop_store = r#"module spec.cpr.loopstoreelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    let i: i32 = 0;
    while (i < 3) {
        xs[1] = mk().v;
        i = i + 1;
    }
    return i32(xs[1][0]);
}

func main(): i32 {
    return run() - 97;
}
"#;
    compile_rejects("loopstoreelem", loop_store, "E10030", 1);

    let nested_store = r#"module spec.cpr.nestedstoreelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        xs[1] = b.v;
    }
    return i32(xs[1][0]);
}

func main(): i32 {
    return run() - 120;
}
"#;
    compile_rejects("nestedstoreelem", nested_store, "E10030", 1);
}

// ---------------------------------------------------------------------------
// staticelement: the accepted element stores reach strict C.
//
// `g.names = [s];` stores a static-backed element into a global struct field:
// the element's owner is module-level storage, so the store is accepted and
// must lower element by element (a compound initializer would be ill-formed C
// for a view element). The parameter-backed out-slot twin `slot = [p];` writes
// a parameter view through the caller's slot and is accepted the same way.
// ---------------------------------------------------------------------------

/// A static-backed element stored into a global field and a parameter-backed
/// element stored through an out-slot both build strictly and run clean.
///
/// `g.names = [s];` stores the module-level static view `s`; the element-wise
/// destination store must pass the strict warning set and the stored view must
/// stay readable, so the run exits 0 with no memory report. The twin
/// `slot = [p];` writes the parameter `p`'s view through the caller's
/// `str[1]` slot, and the caller reads the stored byte back.
#[test]
fn should_accept_a_static_or_parameter_backed_element_store_into_a_global_field_or_out_slot() {
    let global = r#"module spec.cpr.staticfield;

let s: str = "st";

struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func store(): void { g.names = [s]; }

func main(): i32 {
    store();
    return i32(g.names[0][0]) - 115;
}
"#;
    let output = run_memory("staticfield", global);
    assert_no_asan("staticfield", &output);
    assert_exit("staticfield", &output, 0);

    let slot = r#"module spec.cpr.staticout;

func fill(borrow slot: str[1]): void { slot = ["xyz"]; }

func main(): i32 {
    let out: str[1] = ["AA"];
    fill(borrow out);
    return i32(out[0][0]) - 120;
}
"#;
    let output = run_memory("staticout", slot);
    assert_no_asan("staticout", &output);
    assert_exit("staticout", &output, 0);

    let parameter_slot = r#"module spec.cpr.staticoutparam;

func fill(p: str, borrow slot: str[1]): void { slot = [p]; }

func main(): i32 {
    let out: str[1] = ["AA"];
    fill("xyz", borrow out);
    return i32(out[0][0]) - 120;
}
"#;
    compile_rejects("staticoutparam", parameter_slot, "E10033", 1);
}

// ---------------------------------------------------------------------------
// viewcapture: a lambda capture over a view array builds through its
// conversion.
//
// Capturing a `str[1]` copies the array of views into the closure environment.
// The environment field must be spelled with the same C shape the local
// binding uses — an array of view structs — not a bare pointer, so the program
// builds under the strict warning set and the captured element stays readable.
// ---------------------------------------------------------------------------

/// A lambda capturing a `str[1]` builds strictly and reads its element.
///
/// The inline closure `[names] () => i32(names[0][0])` copies the local view
/// array into its environment; the environment field must carry the array's
/// own C shape, so the emitted translation unit passes the strict warning set
/// and the run exits 0 with no memory report.
#[test]
fn should_build_a_lambda_capture_over_a_view_array() {
    let source = r#"module spec.cpr.viewcapture;

func main(): i32 {
    let names: str[1] = ["abc"];
    return ([names] () => i32(names[0][0]))() - 97;
}
"#;
    let output = run_memory("viewcapture", source);
    assert_no_asan("viewcapture", &output);
    assert_exit("viewcapture", &output, 0);
}

// ---------------------------------------------------------------------------
// mergesources: a store whose value names several storages is judged per
// source.
//
// A conditional picks one arm at run time and a view array stores every
// element, so each component is judged on its own against the destination's
// accept set. A global keeps only module-level storage, so a component that
// names a module binding or a literal is fine; a caller slot additionally
// keeps the caller's parameters, and a `null` arm names no storage at all.
// The first component the destination cannot keep supplies the one report.
// ---------------------------------------------------------------------------

/// A mixed-source store whose components are module-level or caller-backed is
/// accepted, builds strictly, and runs clean.
///
/// `g.names = c ? [s1, s2] : [s3, s4]` stores a conditional of two
/// module-backed arrays into a global field; the nested array `grid.m =
/// [[s1, s2], [s3, s4]]` stores four module components, and the mixed element
/// `[c ? s1 : "cd", s2]` pairs a module view with a literal. Every component
/// survives the global's lifetime, so each store is accepted. The `let v =
/// c ? [p, s1] : [q, "cd"]; slot = v;` spelling binds the conditional first,
/// `slot = [p, q];` stores the caller's two parameters, and the scalar twin
/// `out = c ? p : "cd"` pairs a parameter with a literal: every component is
/// the global field accepts module storage. A writable `borrow` slot is
/// caller-retained and cannot keep a view of a lend parameter, so each
/// parameter-backed store reports `E10033`; the `null` arm names no storage and
/// contributes nothing.
#[test]
fn should_judge_a_mixed_source_into_a_global_or_a_writable_slot() {
    let sources = r#"module spec.cpr.mergesources;

let s1: str = "ab";
let s2: str = "cd";
let s3: str = "ef";
let s4: str = "gh";

struct Holder { names: str[2]; }
struct Grid { m: str[2][2]; }

let g: Holder = Holder { names: ["AA", "BB"] };
let grid: Grid = Grid { m: [["AA", "BB"], ["CC", "DD"]] };

func store(c: bool): void {
    g.names = c ? [s1, s2] : [s3, s4];
    grid.m = [[s1, s2], [s3, s4]];
    g.names = [c ? s1 : "cd", s2];
}

func main(): i32 {
    store(true);
    return i32(g.names[1][0]) - 99;
}
"#;
    let output = run_memory("mergesources", sources);
    assert_no_asan("mergesources", &output);
    assert_exit("mergesources", &output, 0);

    let slot = r#"module spec.cpr.mergeparam;

let s1: str = "ab";

func fill(c: bool, p: str, q: str, borrow slot: str[2]): void {
    let v: str[2] = c ? [p, s1] : [q, "cd"];
    slot = v;
    slot = [p, q];
}

func pick(c: bool, p: str, borrow out: str): void { out = c ? p : "cd"; }

func main(): i32 {
    let out: str[2] = ["AA", "BB"];
    fill(true, "xy", "zz", borrow out);
    let whole: str = "aa";
    pick(true, "xy", borrow whole);
    return i32(out[0][0]) - 120;
}
"#;
    compile_rejects("mergeparam", slot, "E10033", 3);

    let staticfn = r#"module spec.cpr.staticfn;

func lit(): static str { return "st"; }

struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func store(c: bool): void { g.names = [c ? lit() : "zz"]; }

func main(): i32 {
    store(true);
    return i32(g.names[0][0]) - 115;
}
"#;
    let output = run_memory("staticfn", staticfn);
    assert_no_asan("staticfn", &output);
    assert_exit("staticfn", &output, 0);

    let nullcond = r#"module spec.cpr.nullcond;

func fill(c: bool, p: str, borrow slot: str?): void { slot = c ? null : p; }

func main(): i32 {
    let out: str? = null;
    fill(false, "xy", borrow out);
    if (out == null) {
        return 9;
    }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("nullcond", nullcond, "E10034", 1);
}

/// A mixed-source store with one frame-only component reports once.
///
/// `slot = c ? "ab" : mk().v;` names a literal and a fresh call result: the
/// chosen arm may read storage that dies at the end of the store statement,
/// so the slot store reports one `E10030`. `names = [c ? s1 : b.v];` stores a
/// module view beside the local `b`'s member: the global outlives the local,
/// so the same store reports one `E10033`. Neither program emits a
/// translation unit.
#[test]
fn should_reject_a_mixed_source_with_a_frame_only_component() {
    let fresh = r#"module spec.cpr.mergefailfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let s1: str = "ab";

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func fill(c: bool, borrow slot: str): void { slot = c ? s1 : mk().v; }

func main(): i32 {
    let out: str = "aa";
    fill(true, borrow out);
    return i32(len(out)) - 2;
}
"#;
    compile_rejects("mergefailfresh", fresh, "E10030", 1);

    let local = r#"module spec.cpr.mergefaillocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let s1: str = "ab";
let g: Holder = Holder { names: ["AA"] };

func store(c: bool): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    g.names = [c ? s1 : b.v];
}

func main(): i32 {
    store(true);
    return 0;
}
"#;
    compile_rejects("mergefaillocal", local, "E10033", 1);
}

// ---------------------------------------------------------------------------
// takebacking: a `take` parameter's storage dies with the callee frame.
//
// A `take` parameter owns its value inside the callee, so a view derived from
// it dies when the callee returns: storing it into a caller slot reports
// `E10030`, exactly like a callee local. The same parameter as a *destination*
// keeps its value in this frame, so a same-frame backing stored into it is
// accepted, and only a backing whose scope ends first reports.
// ---------------------------------------------------------------------------

/// A `take`-backed view stored into the caller's slot reports once, and the
/// same-frame destination twin is accepted and clean.
///
/// `fill(take p: Box, borrow slot: str?) { slot = p.v; }` stores a view of the
/// callee's taken parameter into the caller's slot; the taken storage dies when
/// the callee returns while the caller keeps reading the slot, so the store
/// reports one `E10030`. The row spelling `slot = [p.v];` reports the same.
/// The destination twin `func fill(take t: str) { let b = …; t = b.v; }`
/// stores a same-frame backing into a parameter this frame owns: accepted,
/// builds strictly, and runs clean. When the backing is declared in a nested
/// block instead, the parameter is read after the block ends and the store
/// reports one `E10030`.
#[test]
fn should_judge_a_take_parameter_as_the_callee_frame() {
    let slot = r#"module spec.cpr.takeout;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(take p: Box, borrow slot: str?): void { slot = p.v; }

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    fill(take b, borrow out);
    if (out == null) {
        return 9;
    }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("takeout", slot, "E10030", 1);

    let row = r#"module spec.cpr.takerow;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(take p: Box, borrow slot: str[1]): void { slot = [p.v]; }

func main(): i32 {
    let out: str[1] = ["AA"];
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    fill(take b, borrow out);
    return i32(out[0][0]) - 120;
}
"#;
    compile_rejects("takerow", row, "E10030", 1);

    let frame = r#"module spec.cpr.takeframe;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(take t: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    t = b.v;
    return i32(t[0]);
}

func main(): i32 {
    let out: str = "aa";
    return fill(take out) - 120;
}
"#;
    let output = run_memory("takeframe", frame);
    assert_no_asan("takeframe", &output);
    assert_exit("takeframe", &output, 0);

    let nested = r#"module spec.cpr.takenest;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(take t: str): i32 {
    {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        t = b.v;
    }
    return i32(t[0]);
}

func main(): i32 {
    let out: str = "aa";
    return fill(take out) - 120;
}
"#;
    compile_rejects("takenest", nested, "E10030", 1);
}

// ---------------------------------------------------------------------------
// aliasstore: a `for (borrow e in xs)` store writes the iterable element.
//
// The element alias has no storage of its own, so the store is judged where
// the storage lives: the container's frame decides the escape and the backing's
// loan belongs to the container. A same-frame backing is accepted and a later
// read of the iterable stays clean; a fresh, loop-iteration, or nested-block
// backing dies before the iterable's last read and reports `E10030` once.
// ---------------------------------------------------------------------------

/// A same-frame backing written through the alias is accepted, while a
/// short-lived backing reports once.
///
/// `for (borrow e in xs) { e = b.v; }` writes the local `b`'s member through
/// the alias; `b` and `xs` share the frame, so the store is accepted, builds
/// strictly, and the later read of `xs` is clean. The fresh spelling `e =
/// mk().v;` writes a call result that dies at the statement's end, the loop
/// spelling writes a per-iteration local, and the nested-block spelling
/// writes a local that dies at the block's end; each reports one `E10030` and
/// emits no translation unit.
#[test]
fn should_judge_a_for_alias_store_at_its_iterable_container() {
    let local = r#"module spec.cpr.aliaslocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    for (borrow e in xs) {
        e = b.v;
    }
    return i32(xs[1][0]);
}

func main(): i32 { return run() - 120; }
"#;
    let output = run_memory("aliaslocal", local);
    assert_no_asan("aliaslocal", &output);
    assert_exit("aliaslocal", &output, 0);

    let fresh = r#"module spec.cpr.aliasfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    for (borrow e in xs) {
        e = mk().v;
    }
    return i32(xs[1][0]);
}

func main(): i32 { return run() - 97; }
"#;
    compile_rejects("aliasfresh", fresh, "E10030", 1);

    let loop_store = r#"module spec.cpr.aliasloop;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    let i: i32 = 0;
    while (i < 2) {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        for (borrow e in xs) {
            e = b.v;
        }
        i = i + 1;
    }
    return i32(xs[1][0]);
}

func main(): i32 { return run() - 120; }
"#;
    compile_rejects("aliasloop", loop_store, "E10030", 1);

    let nested = r#"module spec.cpr.aliasnest;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        for (borrow e in xs) {
            e = b.v;
        }
    }
    return i32(xs[1][0]);
}

func main(): i32 { return run() - 120; }
"#;
    compile_rejects("aliasnest", nested, "E10030", 1);
}

// ---------------------------------------------------------------------------
// literalfield: a struct literal's view field shares its destination's frame.
//
// The literal's own storage is materialized in the destination's frame, so a
// frame- or caller-backed view field is accepted exactly as the field
// assignment spelling is, and the backing's loan follows the binding. A
// literal stored into a global keeps reading for the whole program, and a
// returned literal hands the view to the caller, so both keep the escape
// report. A backing declared in a nested block dies before the binding's read
// and reports `E10030`.
// ---------------------------------------------------------------------------

/// A frame-backed view field in a local struct literal is accepted, while the
/// escaping destinations and the nested backing keep their reports.
///
/// `let h: Holder = Holder { names: [b.v] };` and `h = Holder { names: [b.v]
/// };` both store the local `b`'s member into a frame-local carrier: accepted,
/// builds strictly, and the later read is clean. `g = Holder { names: [b.v] };`
/// stores the same literal into a global and `return Holder { names: [b.v] };`
/// hands it to the caller, so each reports one `E10033`. A literal built in an
/// outer scope from a nested-block backing is read after that block ends and
/// reports one `E10030`. Returning the carrier binding itself hands its view
/// fields to the caller, so the binding spelling reports one `E10030` for the
/// same reason the directly returned literal does.
#[test]
fn should_accept_a_frame_backed_view_field_in_a_struct_literal() {
    let local = r#"module spec.cpr.literalpair;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func literal(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    return i32(h.names[0][0]);
}

func assigned(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: ["AA"] };
    h.names = [b.v];
    return i32(h.names[0][0]);
}

func main(): i32 {
    return literal() + assigned() - 240;
}
"#;
    let output = run_memory("literalpair", local);
    assert_no_asan("literalpair", &output);
    assert_exit("literalpair", &output, 0);

    let global = r#"module spec.cpr.literalglobal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func store(): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    g = Holder { names: [b.v] };
}

func main(): i32 {
    store();
    return 0;
}
"#;
    compile_rejects("literalglobal", global, "E10033", 1);

    let returned = r#"module spec.cpr.literalreturn;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func make(): Holder {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    return Holder { names: [b.v] };
}

func main(): i32 {
    return i32(make().names[0][0]) - 120;
}
"#;
    compile_rejects("literalreturn", returned, "E10033", 1);

    let nested = r#"module spec.cpr.literalnested;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func pick(): i32 {
    let h: Holder = Holder { names: ["AA"] };
    {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        h = Holder { names: [b.v] };
    }
    return i32(h.names[0][0]);
}

func main(): i32 { return pick() - 120; }
"#;
    compile_rejects("literalnested", nested, "E10030", 1);

    let returned_binding = r#"module spec.cpr.literalreturnbind;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func make(): Holder {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    return h;
}

func main(): i32 {
    return i32(make().names[0][0]) - 120;
}
"#;
    compile_rejects("literalreturnbind", returned_binding, "E10030", 1);
}

// ---------------------------------------------------------------------------
// carrierreturn: a returned carrier hands its view fields to the caller.
//
// A view field stored into a frame-local carrier reads the backing's storage;
// returning the carrier hands that view to the caller while the backing dies
// with the frame, so the return reports `E10030` exactly as the directly
// returned literal does. A module- or parameter-backed field survives the
// frame, so the same return is accepted and stays readable.
// ---------------------------------------------------------------------------

/// Returning a carrier whose view field reads this frame reports one
/// `E10030`, while the module- and parameter-backed twins run clean.
///
/// `let h = Holder { names: ["AA"] }; h.names = [b.v]; return h;` stores the
/// local `b`'s member into the field and hands it to the caller, so the
/// return reports one `E10030`. The module-backed twin `h.names = [s];` and
/// the parameter-backed twin `h.names = [p];` keep storage that survives the
/// frame: both are accepted, build strictly, and run clean.
#[test]
fn should_reject_returning_a_carrier_whose_view_field_reads_the_frame() {
    let local = r#"module spec.cpr.elementreturn;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func make(): Holder {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: ["AA"] };
    h.names = [b.v];
    return h;
}

func main(): i32 {
    return i32(make().names[0][0]) - 120;
}
"#;
    compile_rejects("elementreturn", local, "E10030", 1);

    let static_backing = r#"module spec.cpr.carrierstatic;

struct Holder { names: str[1]; }

let s: str = "st";

func make(): Holder {
    let h: Holder = Holder { names: ["AA"] };
    h.names = [s];
    return h;
}

func main(): i32 {
    return i32(make().names[0][0]) - 115;
}
"#;
    let output = run_memory("carrierstatic", static_backing);
    assert_no_asan("carrierstatic", &output);
    assert_exit("carrierstatic", &output, 0);

    let parameter = r#"module spec.cpr.carrierparam;

struct Holder { names: str[1]; }

func make(p: str): Holder {
    let h: Holder = Holder { names: ["AA"] };
    h.names = [p];
    return h;
}

func main(): i32 {
    return i32(make("st").names[0][0]) - 115;
}
"#;
    let output = run_memory("carrierparam", parameter);
    assert_no_asan("carrierparam", &output);
    assert_exit("carrierparam", &output, 0);
}

// ---------------------------------------------------------------------------
// paramreturn: a caller-backed view field survives the carrier's return.
//
// A parameter's storage belongs to the caller's frame, which outlives the
// callee's return (EL-2), so a carrier literal whose view field reads a plain
// or `borrow` parameter is a legitimate view return and stays accepted in
// both spellings: the binding form `let h = Holder { names: [p] }; return h;`
// and the directly returned `return Holder { names: [p] };`. The caller's
// read folds the byte into the exit code. A `take` parameter's storage is
// this frame's, so the binding form still reports, and a global keeps only
// module storage, so the parameter-backed literal stored into a global
// reports `E10033` whether it is stored directly or through a binding.
// ---------------------------------------------------------------------------

/// A parameter-backed view field returned in a carrier literal is accepted
/// and memory-clean.
///
/// `let h: Holder = Holder { names: [p] }; return h;` binds the literal in the
/// callee frame and hands the view of `p` to the caller; the same value
/// returned directly and the `borrow`-parameter member spelling (`[b.v]`)
/// behave the same way. Each program builds strictly, runs its caller read
/// without a memory report, and exits 0.
#[test]
fn should_accept_a_parameter_backed_view_field_returned_in_a_carrier_literal() {
    let binding = r#"module spec.cpr.literalparamhold;

struct Holder { names: str[1]; }

func make(p: str): Holder {
    let h: Holder = Holder { names: [p] };
    return h;
}

func main(): i32 {
    return i32(make("st").names[0][0]) - 115;
}
"#;
    let output = run_memory("literalparamhold", binding);
    assert_no_asan("literalparamhold", &output);
    assert_exit("literalparamhold", &output, 0);

    let direct = r#"module spec.cpr.literalparamdirect;

struct Holder { names: str[1]; }

func make(p: str): Holder {
    return Holder { names: [p] };
}

func main(): i32 {
    return i32(make("st").names[0][0]) - 115;
}
"#;
    let output = run_memory("literalparamdirect", direct);
    assert_no_asan("literalparamdirect", &output);
    assert_exit("literalparamdirect", &output, 0);

    let borrow = r#"module spec.cpr.literalparamborrow;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func build(borrow b: Box): Holder {
    let h: Holder = Holder { names: [b.v] };
    return h;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    return i32(build(b).names[0][0]) - 97;
}
"#;
    let output = run_memory("literalparamborrow", borrow);
    assert_no_asan("literalparamborrow", &output);
    assert_exit("literalparamborrow", &output, 0);
}

/// A parameter-backed carrier literal stored where the caller cannot keep it
/// reports once.
///
/// `g = Holder { names: [p] };` stores the literal into a global, which
/// outlives the caller's frame, and reports one `E10033`; the two-step
/// `let h = Holder { names: [p] }; g = h;` reports the same one error. A
/// `take` parameter's literal binding is read by the caller after the taken
/// storage dies, so `let h = Holder { names: [t.v] }; return h;` reports one
/// `E10030`. None of the programs emits a translation unit.
#[test]
fn should_reject_a_parameter_backed_carrier_literal_at_an_escaping_destination() {
    let global = r#"module spec.cpr.literalparamglobal;

struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func store(p: str): void {
    g = Holder { names: [p] };
}

func main(): i32 {
    store("st");
    return 0;
}
"#;
    compile_rejects("literalparamglobal", global, "E10033", 1);

    let bound = r#"module spec.cpr.literalparambound;

struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func store(p: str): void {
    let h: Holder = Holder { names: [p] };
    g = h;
}

func main(): i32 {
    store("st");
    return 0;
}
"#;
    compile_rejects("literalparambound", bound, "E10033", 1);

    let taken = r#"module spec.cpr.literaltakereturn;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func make(take t: Box): Holder {
    let h: Holder = Holder { names: [t.v] };
    return h;
}

func main(): i32 {
    return 0;
}
"#;
    compile_rejects("literaltakereturn", taken, "E10030", 1);
}

// ---------------------------------------------------------------------------
// rejectedwrite: an already-rejected view store keeps one diagnostic.
//
// The `const` report for an immutable global and the read-only-parameter
// report for a lend already reject the write, so the view-store judgment steps
// aside instead of adding a second error at the same site.
// ---------------------------------------------------------------------------

/// An immutable destination keeps its own one report.
///
/// `const names: str = "start"; names = b.v;` writes through an immutable
/// binding, which reports one `E10023` and skips the view judgment. The
/// read-only parameter twin `func fill(p: str) { p = b.v; }` reports one
/// `E10024`. Neither program emits a translation unit.
#[test]
fn should_keep_one_report_at_an_already_rejected_view_store() {
    let global = r#"module spec.cpr.conststore;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

const names: str = "start";

func store(): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    names = b.v;
}

func main(): i32 {
    store();
    return 0;
}
"#;
    compile_rejects("conststore", global, "E10023", 1);

    let parameter = r#"module spec.cpr.readonlystore;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    p = b.v;
}

func main(): i32 {
    return 0;
}
"#;
    compile_rejects("readonlystore", parameter, "E10024", 1);
}

// ---------------------------------------------------------------------------
// elementloanreplace: overwriting an element replaces the loan it carried.
//
// A view element store loans its backing to the container. Overwriting that
// same element with storage that names no owner drops the loan, so the old
// backing stops blocking its owner's writes; a loan another element still
// carries stays live.
// ---------------------------------------------------------------------------

/// A replaced element's loan is dropped, while another element's loan stays.
///
/// `xs[1] = c ? b.v : a; xs[1] = "zz"; b.v = stringOf("qq");` overwrites the
/// element that read `b`, so the write to `b.v` is accepted, builds strictly,
/// and the final read of `xs[1]` is clean. When `xs[0]` still reads `b`, the
/// same overwrite leaves that loan live and the write reports one `E10031`.
#[test]
fn should_replace_a_replaced_element_loan_and_keep_the_others() {
    let replaced = r#"module spec.cpr.elementreplace;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(c: bool, a: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = c ? b.v : a;
    xs[1] = "zz";
    b.v = stringOf("qq");
    return i32(xs[1][0]);
}

func main(): i32 { return pick(true, "ppp") - 122; }
"#;
    let output = run_memory("elementreplace", replaced);
    assert_no_asan("elementreplace", &output);
    assert_exit("elementreplace", &output, 0);

    let kept = r#"module spec.cpr.elementkeep;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(c: bool, a: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = b.v;
    xs[1] = "zz";
    b.v = stringOf("qq");
    return i32(xs[0][0]);
}

func main(): i32 { return pick(true, "ppp") - 120; }
"#;
    compile_rejects("elementkeep", kept, "E10031", 1);
}

// ---------------------------------------------------------------------------
// viewmovestate: a view element store moves nothing.
//
// The element store copies a view; the backing stays where it is and the
// container loans it. A later read of the backing is therefore accepted, and a
// later *move* of it is what the live loan blocks.
// ---------------------------------------------------------------------------

/// Reading the backing after a view element store is accepted, while moving it
/// reports the move-while-borrowed error.
///
/// `xs[1] = c ? b.v : a; let n: usize = len(b.v);` reads `b.v` after the
/// store: a view store moves nothing, so the read is accepted, builds
/// strictly, and runs clean. The move twin `let m: String = b.v;` moves the
/// member while the element's loan is live and reports one `E10022`.
#[test]
fn should_not_mark_a_view_element_store_as_a_move() {
    let read = r#"module spec.cpr.viewread;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(c: bool, a: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = c ? b.v : a;
    let n: usize = len(b.v);
    return i32(xs[1][0]) + i32(n);
}

func main(): i32 { return pick(true, "ppp") - 122; }
"#;
    let output = run_memory("viewread", read);
    assert_no_asan("viewread", &output);
    assert_exit("viewread", &output, 0);

    let moved = r#"module spec.cpr.viewmove;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(c: bool, a: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = c ? b.v : a;
    let m: String = b.v;
    return i32(xs[1][0]) + i32(len(m)) - 122;
}

func main(): i32 { return pick(true, "ppp"); }
"#;
    compile_rejects("viewmove", moved, "E10022", 1);
}

// ---------------------------------------------------------------------------
// loopcondliteral: a conditional literal receiver in a loop condition.
//
// The condition builds a fresh struct literal on its evaluated path, and the
// literal's callable field owns a freshly built string. The loop-condition
// walk must leave that field with exactly one owner — the literal temporary,
// released when the loop leaves — so a second temporary over the same field
// frees its storage twice. The single-shot and re-executing spellings and the
// array-literal element variant are accepted and memory-clean.
// ---------------------------------------------------------------------------

/// A conditional literal receiver in a loop condition releases its field
/// storage once per evaluation.
///
/// `(c ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }).call(5)`
/// builds a fresh literal on the evaluated path; the literal temporary owns
/// its `s` field and releases it at the loop edge, and the callable-field call
/// reads through it. The single-shot spelling runs the condition once, the
/// counted spelling re-executes it per iteration, and the array-literal
/// element spelling reaches the same field through an element. Every call
/// returns 6, so the accumulated counts cancel the trailing constant, and the
/// runs exit 0 with no memory report.
#[test]
fn should_release_a_conditional_literal_in_a_loop_condition() {
    let single = r#"module spec.cpr.loopcondliteral;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func useOnce(c: bool): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < 1 && (c ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func useLoop(c: bool, k: i32): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < k && (c ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return useOnce(true) + useOnce(false) + useLoop(true, 3) + useLoop(false, 3) - 8;
}
"#;
    let output = run_memory("loopcondliteral", single);
    assert_no_asan("loopcondliteral", &output);
    assert_exit("loopcondliteral", &output, 0);

    let element = r#"module spec.cpr.loopcondelem;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func use(c: bool): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < 1 && [c ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }][0].call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return use(true) + use(false) - 2;
}
"#;
    let output = run_memory("loopcondelem", element);
    assert_no_asan("loopcondelem", &output);
    assert_exit("loopcondelem", &output, 0);
}

// ---------------------------------------------------------------------------
// literalpositions: the `?:` and `||` literal-receiver positions.
//
// A conditional selecting between two struct literals is itself a callable
// receiver, and the `||` right side evaluates such a receiver only on the path
// where the guard fails. Each literal's owned field keeps one owner in every
// position — including the loop-condition spelling that re-executes the
// guarded walk — so every path is accepted and memory-clean.
// ---------------------------------------------------------------------------

/// A conditional literal receiver in the `?:` and `||` positions releases its
/// field storage once per evaluation.
///
/// `(g ? H { ... } : H { ... }).call(5)` runs the call of the literal the
/// conditional selects, and `g || (c ? H { ... } : H { ... }).call(5) > 0`
/// evaluates the receiver only where `g` fails; the loop-condition spelling
/// re-executes the guarded walk once per iteration. Every evaluated call
/// returns 6 and every guarded result is 1, so the accumulated values cancel
/// the trailing constants and the runs exit 0 with no memory report.
#[test]
fn should_release_conditional_literals_in_the_receiver_positions() {
    let positions = r#"module spec.cpr.literalpositions;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func pick(g: bool): i32 {
    let r: i32 = (g ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }).call(5);
    return r;
}

func guard(c: bool, g: bool): i32 {
    if (g || (c ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }).call(5) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return pick(true) + pick(false) + guard(true, true) + guard(true, false) + guard(false, false) - 15;
}
"#;
    let output = run_memory("literalpositions", positions);
    assert_no_asan("literalpositions", &output);
    assert_exit("literalpositions", &output, 0);

    let looped = r#"module spec.cpr.literalpositionsloop;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func guard(c: bool, g: bool): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < 1 && (g || (c ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }).call(5) > 0)) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return guard(true, true) + guard(true, false) + guard(false, false) - 3;
}
"#;
    let output = run_memory("literalpositionsloop", looped);
    assert_no_asan("literalpositionsloop", &output);
    assert_exit("literalpositionsloop", &output, 0);
}

// ---------------------------------------------------------------------------
// discardmixed: a discarded struct-literal arm with fresh and place fields.
//
// The discard decides per field: a fresh owning field materializes into a
// statement temporary that releases it when the statement ends, while a field
// reading an existing owned binding stays inline and stays a read. The direct
// `ok ? t : u;` control pins both of its arms as reads; the literal spelling
// must keep its place fields readable the same way.
// ---------------------------------------------------------------------------

/// A discarded mixed-field literal releases its fresh field and reads the
/// place fields.
///
/// `ok ? Pair { a: stringOf("x"), b: t } : Pair { a: stringOf("y"), b: u };`
/// discards the selected literal. Its fresh `a` field must release when the
/// statement ends, and its `b` field reads `t` or `u` without moving them, so
/// both bindings stay readable afterwards — exactly the behavior of the
/// direct `ok ? t : u;` control. Both programs exit 0 with no memory report.
#[test]
fn should_discard_a_mixed_field_literal_as_reads() {
    let literal = r#"module spec.cpr.discardmixed;

import { String, stringOf } from std.text;

struct Pair { a: String; b: String; }

func literal(ok: bool): i32 {
    let t: String = stringOf("tt");
    let u: String = stringOf("uu");
    ok ? Pair { a: stringOf("x"), b: t } : Pair { a: stringOf("y"), b: u };
    return i32(len(t)) + i32(len(u)) - 4;
}

func main(): i32 {
    return literal(true) + literal(false);
}
"#;
    let output = run_memory("discardmixed", literal);
    assert_no_asan("discardmixed", &output);
    assert_exit("discardmixed", &output, 0);

    let direct = r#"module spec.cpr.discardmixeddirect;

import { String, stringOf } from std.text;

func control(ok: bool): i32 {
    let t: String = stringOf("tt");
    let u: String = stringOf("uu");
    ok ? t : u;
    return i32(len(t)) + i32(len(u)) - 4;
}

func main(): i32 {
    return control(true) + control(false);
}
"#;
    let output = run_memory("discardmixeddirect", direct);
    assert_no_asan("discardmixeddirect", &output);
    assert_exit("discardmixeddirect", &output, 0);
}

// ---------------------------------------------------------------------------
// discardprojection: a discarded conditional's fresh projection arm.
//
// The discarded conditional `c ? mk().v : t` has one arm projecting a fresh
// root and one reading a named binding. The statement must bind the fresh root
// so its storage releases after the statement, while the projected member
// stays a read and the place arm stays readable.
// ---------------------------------------------------------------------------

/// A discarded conditional projection releases its fresh root.
///
/// `let t: String = stringOf("tt"); c ? mk().v : t;` evaluates a fresh root on
/// one path; the statement binds that root and releases it once when the
/// statement ends, and the projected member is a read. `t` stays readable on
/// both selections, so the runs exit 0 with no memory report.
#[test]
fn should_release_the_discarded_projection_arm() {
    let source = r#"module spec.cpr.discardprojection;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool): i32 {
    let t: String = stringOf("tt");
    c ? mk().v : t;
    return i32(len(t));
}

func main(): i32 {
    return use(true) + use(false) - 4;
}
"#;
    let output = run_memory("discardprojection", source);
    assert_no_asan("discardprojection", &output);
    assert_exit("discardprojection", &output, 0);
}


// ---------------------------------------------------------------------------
// discardnestedleaf: a discarded mixed literal's nested and fixed-array
// fields keep their fresh leaves released.
//
// The discard decides per field, and an aggregate field recurses into its own
// fields and elements: each fresh owning leaf binds to its own statement
// temporary and releases when the statement ends, while every place leaf stays
// a read. A nested struct-literal field and a fixed-array field with a fresh
// element must therefore both build strictly and run leak-free, with the
// place bindings readable afterwards. The loop spelling re-executes the
// discard once per iteration, so every iteration's fresh leaves release.
// ---------------------------------------------------------------------------

/// A discarded literal with a nested literal field and a fixed-array field
/// releases every fresh leaf, on both selections and in a loop.
///
/// `ok ? Outer { inner: Box { v: stringOf("fr"), w: t } } : …;` drops a
/// literal whose nested field owns a fresh `v` beside the place `w: t`, and
/// `ok ? P { a: [stringOf("x")], b: t } : …;` drops a literal whose
/// fixed-array field owns the fresh element. Both bind every fresh leaf to
/// its own statement temporary and free it when the statement ends, while
/// `t` stays readable for the trailing length read. Each program exits 0 with
/// no memory report, and the loop spellings repeat the discard cleanly.
#[test]
fn should_release_the_fresh_leaves_of_a_discarded_aggregate_field() {
    let nested = r#"module spec.cpr.discardnested;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Outer { inner: Box; }

func nested(ok: bool): i32 {
    let t: String = stringOf("tt");
    ok ? Outer { inner: Box { v: stringOf("fr"), w: t } } : Outer { inner: Box { v: stringOf("yy"), w: t } };
    return i32(len(t)) - 2;
}

func looped(ok: bool, k: i32): i32 {
    let t: String = stringOf("tt");
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < k) {
        ok ? Outer { inner: Box { v: stringOf("fr"), w: t } } : Outer { inner: Box { v: stringOf("yy"), w: t } };
        n = n + i32(len(t));
        i = i + 1;
    }
    return n - k * 2;
}

func main(): i32 {
    return nested(true) + nested(false) + looped(true, 3) + looped(false, 3);
}
"#;
    let output = run_memory("discardnested", nested);
    assert_no_asan("discardnested", &output);
    assert_exit("discardnested", &output, 0);

    let fixed = r#"module spec.cpr.discardfixed;

import { String, stringOf } from std.text;

struct P { a: String[1]; b: String; }

func fixed(ok: bool): i32 {
    let t: String = stringOf("tt");
    ok ? P { a: [stringOf("x")], b: t } : P { a: [stringOf("y")], b: t };
    return i32(len(t)) - 2;
}

func looped(ok: bool, k: i32): i32 {
    let t: String = stringOf("tt");
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < k) {
        ok ? P { a: [stringOf("x")], b: t } : P { a: [stringOf("y")], b: t };
        n = n + i32(len(t));
        i = i + 1;
    }
    return n - k * 2;
}

func main(): i32 {
    return fixed(true) + fixed(false) + looped(true, 3) + looped(false, 3);
}
"#;
    let output = run_memory("discardfixed", fixed);
    assert_no_asan("discardfixed", &output);
    assert_exit("discardfixed", &output, 0);
}

// ---------------------------------------------------------------------------
// discardarray: a discarded array-literal arm is judged like its direct
// statement twin.
//
// The element read rule belongs to a discarded literal's *fields*; an array
// literal that is itself the discarded arm stores its elements exactly as the
// direct `[t, …];` statement does, so the place element is moved and a later
// use reports. The rejection also keeps the arm's brace-elided spelling out of
// the emitted C, which the strict build would refuse.
// ---------------------------------------------------------------------------

/// A discarded array-literal arm reports the moved place element once per
/// arm.
///
/// `ok ? [t, stringOf("x")] : [t, stringOf("y")];` names `t` in both arms, so
/// each arm's element store consumes it and the trailing `len(t)` reports one
/// `E10020` per arm. No translation unit is emitted.
#[test]
fn should_reject_a_discarded_array_literal_arm() {
    let source = r#"module spec.cpr.discardarray;

import { String, stringOf } from std.text;

func use(ok: bool): i32 {
    let t: String = stringOf("tt");
    ok ? [t, stringOf("x")] : [t, stringOf("y")];
    return i32(len(t)) - 2;
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    compile_rejects("discardarray", source, "E10020", 2);
}

// ---------------------------------------------------------------------------
// discardprojfield: a discarded literal field's conditional projection reads.
//
// The field of a discarded literal is never stored, so a projection over a
// conditional keeps its place arms untouched: the selected member is read, its
// root keeps owning it, and a later use stays valid. Only an ownerless root
// binds for release, so the mixed fresh/place spelling frees the fresh arm's
// storage when the statement ends while the place arm stays readable.
// ---------------------------------------------------------------------------

/// A discarded literal field that projects a conditional over place roots is a
/// read, so the roots stay usable afterwards.
///
/// `ok ? Pair { a: (d ? p : q).v, b: t } : …;` drops the selected literal
/// without storing it; the projected member is read from whichever root the
/// inner condition picks, so `p.v`, `q.v`, and `t` keep their storage for the
/// trailing length read. Both selections and the loop spelling run clean.
#[test]
fn should_read_a_discarded_literal_fields_place_projection() {
    let source = r#"module spec.cpr.discardprojfield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Pair { a: String; b: String; }

func pick(ok: bool, d: bool): i32 {
    let p: Box = Box { v: stringOf("pp"), w: stringOf("ww") };
    let q: Box = Box { v: stringOf("qq"), w: stringOf("rr") };
    let t: String = stringOf("tt");
    ok ? Pair { a: (d ? p : q).v, b: t } : Pair { a: (d ? q : p).v, b: t };
    return i32(len(p.v)) + i32(len(q.v)) + i32(len(t)) - 6;
}

func looped(ok: bool, d: bool, k: i32): i32 {
    let p: Box = Box { v: stringOf("pp"), w: stringOf("ww") };
    let q: Box = Box { v: stringOf("qq"), w: stringOf("rr") };
    let t: String = stringOf("tt");
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < k) {
        ok ? Pair { a: (d ? p : q).v, b: t } : Pair { a: (d ? q : p).v, b: t };
        n = n + i32(len(p.v)) + i32(len(q.v));
        i = i + 1;
    }
    return n - k * 4;
}

func main(): i32 {
    return pick(true, true) + pick(true, false) + pick(false, true) + pick(false, false)
        + looped(true, true, 3) + looped(false, false, 3);
}
"#;
    let output = run_memory("discardprojfield", source);
    assert_no_asan("discardprojfield", &output);
    assert_exit("discardprojfield", &output, 0);
}

/// A discarded literal field's mixed fresh/place projection releases the fresh
/// arm's root and keeps the place arm readable.
///
/// `ok ? Pair { a: (d ? p : mkB()).v, b: t } : …;` reads `p.v` on one path and
/// a fresh root's member on the other; the fresh root binds to its own
/// statement temporary and frees both of its fields when the statement ends,
/// while `p.v` and `t` stay usable for the trailing read. Both selections and
/// the loop spelling run leak-free.
#[test]
fn should_release_a_discarded_literal_fields_mixed_projection() {
    let source = r#"module spec.cpr.discardprojmixed;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Pair { a: String; b: String; }

func mkB(): Box { return Box { v: stringOf("cc"), w: stringOf("dd") }; }

func pick(ok: bool, d: bool): i32 {
    let p: Box = Box { v: stringOf("pp"), w: stringOf("ww") };
    let t: String = stringOf("tt");
    ok ? Pair { a: (d ? p : mkB()).v, b: t } : Pair { a: (d ? p : mkB()).v, b: t };
    return i32(len(p.v)) + i32(len(t)) - 4;
}

func looped(ok: bool, d: bool, k: i32): i32 {
    let p: Box = Box { v: stringOf("pp"), w: stringOf("ww") };
    let t: String = stringOf("tt");
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < k) {
        ok ? Pair { a: (d ? p : mkB()).v, b: t } : Pair { a: (d ? p : mkB()).v, b: t };
        n = n + i32(len(p.v)) + i32(len(t));
        i = i + 1;
    }
    return n - k * 4;
}

func main(): i32 {
    return pick(true, true) + pick(true, false) + pick(false, true) + pick(false, false)
        + looped(true, true, 3) + looped(false, false, 3);
}
"#;
    let output = run_memory("discardprojmixed", source);
    assert_no_asan("discardprojmixed", &output);
    assert_exit("discardprojmixed", &output, 0);
}

// ---------------------------------------------------------------------------
// condarmfixed: a fixed-array conditional fills one destination per path.
//
// The fixed-array return ABI passes the result through a hidden out-parameter,
// so a conditional whose arms are fixed-array call results cannot be rendered
// as one C value expression. Each arm must fill the conditional's destination
// on its own path — the call arm passes the hidden destination — and the
// consumer reads that destination. The discarded spelling and the consuming
// store each select both arms once.
// ---------------------------------------------------------------------------

/// A fixed-array conditional of call results builds and runs through the
/// discarded and the consuming-store spellings.
///
/// `c ? a() : b();` discards the selected call result; the calls count their
/// runs, so the true and the false selection each run exactly one arm. The
/// store `let m: u8[2] = c ? a() : b();` keeps the selected result: `a` fills
/// [1, 2] and `b` fills [3, 4], so the two selections sum to 10 and the
/// trailing constant cancels them. Both programs build under the strict flags
/// and run with no memory report.
#[test]
fn should_route_a_fixed_array_conditional_through_its_destination() {
    let discarded = r#"module spec.cpr.condarmfixed;

let left: i32 = 0;
let right: i32 = 0;

func a(): u8[2] {
    left = left + 1;
    return [1, 2];
}

func b(): u8[2] {
    right = right + 1;
    return [3, 4];
}

func use(c: bool): i32 {
    c ? a() : b();
    return 0;
}

func main(): i32 {
    return use(true) + use(false) + left * 10 + right - 11;
}
"#;
    let output = run_memory("condarmfixed", discarded);
    assert_no_asan("condarmfixed", &output);
    assert_exit("condarmfixed", &output, 0);

    let stored = r#"module spec.cpr.condarmfixedstore;

func a(): u8[2] { return [1, 2]; }
func b(): u8[2] { return [3, 4]; }

func use(c: bool): i32 {
    let m: u8[2] = c ? a() : b();
    return i32(m[0]) + i32(m[1]);
}

func main(): i32 {
    return use(true) + use(false) - 10;
}
"#;
    let output = run_memory("condarmfixedstore", stored);
    assert_no_asan("condarmfixedstore", &output);
    assert_exit("condarmfixedstore", &output, 0);
}

// ---------------------------------------------------------------------------
// mixedliteral: a discarded literal's fixed-array call field and fresh String.
//
// A literal mixing a `str[2]` field with a `String` field is discarded. When
// the fixed-array field is filled by a conditional of fixed-array call
// results, each arm must fill that field's destination on its path (the
// hidden out-parameter ABI); a value-expression fallback leaves a bare call in
// the emitted C and fails the strict build. The fresh `String` field releases
// when the statement ends. The literal-arm spelling of the same discard
// releases the selected literal's fresh field as well.
// ---------------------------------------------------------------------------

/// A discarded mixed-field literal fills its fixed-array field per path and
/// releases its fresh string field.
///
/// `Mixed { names: ok ? n1() : n2(), tag: stringOf("x") };` drops the literal;
/// the selected call fills the `str[2]` field (each call counts its runs, so
/// the two selections run one arm each) and the fresh `tag` releases when the
/// statement ends. The literal-arm twin `ok ? Mixed { ... } : Mixed { ... };`
/// drops the selected literal with the same release. Both programs are
/// accepted, build under the strict flags, and exit 0 with no memory report.
#[test]
fn should_discard_a_mixed_fixed_array_and_string_literal() {
    let callfield = r#"module spec.cpr.mixedliteral;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let left: i32 = 0;
let right: i32 = 0;

func n1(): str[2] {
    left = left + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    right = right + 1;
    return ["CC", "DD"];
}

func use(ok: bool): i32 {
    Mixed { names: ok ? n1() : n2(), tag: stringOf("x") };
    return 0;
}

func main(): i32 {
    return use(true) + use(false) + left * 10 + right - 11;
}
"#;
    let output = run_memory("mixedliteral", callfield);
    assert_no_asan("mixedliteral", &output);
    assert_exit("mixedliteral", &output, 0);

    let arms = r#"module spec.cpr.mixedliteralarms;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

func use(ok: bool): i32 {
    ok ? Mixed { names: ["AA", "BB"], tag: stringOf("x") } : Mixed { names: ["CC", "DD"], tag: stringOf("y") };
    return 0;
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    let output = run_memory("mixedliteralarms", arms);
    assert_no_asan("mixedliteralarms", &output);
    assert_exit("mixedliteralarms", &output, 0);
}

// ---------------------------------------------------------------------------
// mixedliteralrest: the same discarded literal with a directly called field
// and a discarded all-fresh fixed-array conditional.
//
// `Mixed { names: n1(), tag: stringOf("x") };` fills its fixed-array field
// with a plain call, which also passes the hidden out-parameter ABI through
// the field's own destination. A discarded conditional whose arms are array
// literals (`ok ? [stringOf("x")] : [stringOf("y")];`) fills one destination
// element-wise on each path and releases the selected element when the
// statement ends. Both programs are accepted, build under the strict flags,
// and exit 0 with no memory report.
// ---------------------------------------------------------------------------

/// A discarded literal with a directly called fixed-array field builds and
/// releases its fresh string field.
///
/// `Mixed { names: n1(), tag: stringOf("x") };` drops the literal: the call
/// fills the `str[2]` field through the field's destination and the fresh
/// `tag` releases when the statement ends. The program is accepted, builds
/// under the strict flags, and exits 0 with no memory report.
#[test]
fn should_discard_a_direct_call_filled_fixed_array_field() {
    let source = r#"module spec.cpr.discardcallfield;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

func n1(): str[2] { return ["AA", "BB"]; }

func use(): i32 {
    Mixed { names: n1(), tag: stringOf("x") };
    return 0;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("discardcallfield", source);
    assert_no_asan("discardcallfield", &output);
    assert_exit("discardcallfield", &output, 0);
}

/// A discarded all-fresh fixed-array conditional builds and releases the
/// selected element.
///
/// `ok ? [stringOf("x")] : [stringOf("y")];` selects one array literal and
/// fills one destination element-wise on that path; the fresh element
/// releases when the statement ends, so both selections run clean. The
/// program is accepted, builds under the strict flags, and exits 0.
#[test]
fn should_discard_a_fresh_fixed_array_conditional() {
    let source = r#"module spec.cpr.discardfresharray;

import { String, stringOf } from std.text;

func use(ok: bool): i32 {
    ok ? [stringOf("x")] : [stringOf("y")];
    return 0;
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    let output = run_memory("discardfresharray", source);
    assert_no_asan("discardfresharray", &output);
    assert_exit("discardfresharray", &output, 0);
}

// ---------------------------------------------------------------------------
// condroot: projecting a fixed-array field of a conditional root.
//
// `(c ? b1 : b2).xs` selects the root first and projects its fixed-array
// field. A conditional is not an lvalue, so the projection cannot take the
// address of a member of the conditional value; each arm must contribute its
// own root's address on its path. The store and the return spelling consume
// the selected field, and the `for` iterable over a call result's field
// follows the same root preparation.
// ---------------------------------------------------------------------------

/// A fixed-array projection of a conditional root builds and runs through the
/// store and the return spelling.
///
/// `let a: u8[4] = (c ? b1 : b2).xs;` copies the selected root's field into
/// `a`; with `b1` = [1, 2, 3, 4] and `b2` = [5, 6, 7, 8] the two selections
/// sum to 10 and 26, so the trailing constant cancels them. The return
/// spelling `return (c ? b1 : b2).xs;` produces the same fields through a
/// callee whose result the caller consumes. Both programs build under the
/// strict flags and exit 0.
#[test]
fn should_project_a_fixed_array_field_of_a_conditional_root() {
    let stored = r#"module spec.cpr.condroot;

struct Box { xs: u8[4]; }

func use(c: bool, b1: Box, b2: Box): i32 {
    let a: u8[4] = (c ? b1 : b2).xs;
    return i32(a[0]) + i32(a[1]) + i32(a[2]) + i32(a[3]);
}

func main(): i32 {
    let b1: Box = Box { xs: [1, 2, 3, 4] };
    let b2: Box = Box { xs: [5, 6, 7, 8] };
    return use(true, b1, b2) + use(false, b1, b2) - 36;
}
"#;
    let output = run_memory("condroot", stored);
    assert_no_asan("condroot", &output);
    assert_exit("condroot", &output, 0);

    let returned = r#"module spec.cpr.condrootreturn;

struct Box { xs: u8[4]; }

func pick(c: bool, b1: Box, b2: Box): u8[4] {
    return (c ? b1 : b2).xs;
}

func sum(a: u8[4]): i32 {
    return i32(a[0]) + i32(a[1]) + i32(a[2]) + i32(a[3]);
}

func main(): i32 {
    let b1: Box = Box { xs: [1, 2, 3, 4] };
    let b2: Box = Box { xs: [5, 6, 7, 8] };
    return sum(pick(true, b1, b2)) + sum(pick(false, b1, b2)) - 36;
}
"#;
    let output = run_memory("condrootreturn", &returned);
    assert_no_asan("condrootreturn", &output);
    assert_exit("condrootreturn", &output, 0);
}

// ---------------------------------------------------------------------------
// foriterable: iterating a fixed-array projection of a call result.
//
// `for (x in mk().xs)` takes the fixed-array field of a call result as the
// loop's iterable. The call's result must bind to a place before the
// projection takes the field's address — the same preparation the store and
// return spellings use — so the loop walks the field's four elements.
// ---------------------------------------------------------------------------

/// A `for` loop over a call result's fixed-array field builds and runs.
///
/// `mk()` returns `Box { xs: [1, 2, 3, 4] }` and the loop sums the field's
/// elements, so the total is 10 and the trailing constant cancels it. The
/// program is accepted, builds under the strict flags, and exits 0.
#[test]
fn should_iterate_a_call_results_fixed_array_field() {
    let source = r#"module spec.cpr.foriterable;

struct Box { xs: u8[4]; }

func mk(): Box { return Box { xs: [1, 2, 3, 4] }; }

func use(): i32 {
    let s: i32 = 0;
    for (x in mk().xs) {
        s = s + i32(x);
    }
    return s;
}

func main(): i32 {
    return use() - 10;
}
"#;
    let output = run_memory("foriterable", source);
    assert_no_asan("foriterable", &output);
    assert_exit("foriterable", &output, 0);
}

// ---------------------------------------------------------------------------
// moveonlyroot: a move-only conditional root's fixed-array projection.
//
// The root struct owns a `String` field, so the conditional's selected root
// cannot be copied into a temporary value. The projection must therefore take
// each arm's own address on its path instead of materializing the conditional
// as a value. Both selections run, and the roots' owned fields release with
// their bindings at the frame's end.
// ---------------------------------------------------------------------------

/// A fixed-array projection of a move-only conditional root builds and runs
/// on both selections.
///
/// `let a: u8[4] = (c ? b1 : b2).xs;` with `b1` and `b2` each owning a
/// `String` field takes no value copy of the selected root: each arm
/// contributes its own field's address, the two selections sum to 10 and 26,
/// and the trailing constant cancels them. The owned `tag` fields release
/// with their bindings, so the run reports no memory error.
#[test]
fn should_project_a_move_only_conditional_roots_fixed_array_field() {
    let source = r#"module spec.cpr.moveonlyroot;

import { String, stringOf } from std.text;

struct Box { xs: u8[4]; tag: String; }

func use(c: bool): i32 {
    let b1: Box = Box { xs: [1, 2, 3, 4], tag: stringOf("pp") };
    let b2: Box = Box { xs: [5, 6, 7, 8], tag: stringOf("qq") };
    let a: u8[4] = (c ? b1 : b2).xs;
    return i32(a[0]) + i32(a[1]) + i32(a[2]) + i32(a[3]);
}

func main(): i32 {
    return use(true) + use(false) - 36;
}
"#;
    let output = run_memory("moveonlyroot", source);
    assert_no_asan("moveonlyroot", &output);
    assert_exit("moveonlyroot", &output, 0);
}

// ---------------------------------------------------------------------------
// fixedcontrols: the direct fixed-array spellings that must not change.
//
// The direct call discard `a();`, the direct call store, the direct
// projection `let a: u8[4] = mk().xs;`, and the owned-field root (a call
// result that carries an owned field) already build and run; the conditional
// spellings extend their destination and root shapes, so these stay accepted
// controls.
// ---------------------------------------------------------------------------

/// The direct fixed-array call and projection spellings stay accepted.
///
/// `a();` and `let m: u8[2] = a();` use the call's hidden destination
/// directly; `let a: u8[4] = mk().xs;` copies the projected field of a call
/// result; and the owned-field root keeps its `v` field released while the
/// projection reads `xs`. Each program exits 0.
#[test]
fn should_keep_the_direct_fixed_array_spellings() {
    let direct = r#"module spec.cpr.fixeddirect;

func a(): u8[2] { return [1, 2]; }

func main(): i32 {
    a();
    let m: u8[2] = a();
    return i32(m[0]) + i32(m[1]) - 3;
}
"#;
    let output = run_memory("fixeddirect", direct);
    assert_no_asan("fixeddirect", &output);
    assert_exit("fixeddirect", &output, 0);

    let projected = r#"module spec.cpr.fixedprojbind;

struct Box { xs: u8[4]; }

func mk(): Box { return Box { xs: [1, 2, 3, 4] }; }

func main(): i32 {
    let a: u8[4] = mk().xs;
    return i32(a[0]) - 1;
}
"#;
    let output = run_memory("fixedprojbind", projected);
    assert_no_asan("fixedprojbind", &output);
    assert_exit("fixedprojbind", &output, 0);

    let owned = r#"module spec.cpr.fixedownedroot;

import { String, stringOf } from std.text;

struct Box { xs: u8[4]; v: String; }

func mk(): Box { return Box { xs: [5, 6, 7, 8], v: stringOf("x") }; }

func main(): i32 {
    let a: u8[4] = mk().xs;
    let b: Box = mk();
    let c: u8[4] = b.xs;
    return i32(a[0]) + i32(c[1]) - 11;
}
"#;
    let output = run_memory("fixedownedroot", owned);
    assert_no_asan("fixedownedroot", &output);
    assert_exit("fixedownedroot", &output, 0);
}

// ---------------------------------------------------------------------------
// fixediterowned: iterating a conditional root's owned fixed-array field.
//
// `for (borrow x in (c ? b1 : b2).xs)` selects the root once and iterates the
// selected field in place: a place arm's storage is only read, never copied or
// moved, so the field stays readable after the loop. The row spelling iterates
// a struct row's elements, the re-executed spelling runs the loop once per
// outer iteration, and the nested spelling reaches a two-step field chain.
// Every program is accepted, builds under the strict flags, exits 0, and is
// free of memory reports; the field read-back after each loop proves the
// iterable did not take the caller's storage.
// ---------------------------------------------------------------------------

/// A conditional root's owned field iterates in place on both selections.
///
/// `for (borrow x in (c ? b1 : b2).xs)` sums the selected field's string
/// lengths and the trailing read-back sums `b1`'s own lengths, so a move of
/// either field would change the result or crash; with `b1` = "AA"/"BB" and
/// `b2` = "CC"/"DD" each`use` returns 0. The row spelling iterates a struct
/// row's `v` fields and reads `b1.rows[0].v` back the same way. Both programs
/// build under the strict flags, exit 0, and report no memory error.
#[test]
fn should_iterate_a_conditional_roots_owned_field_in_place() {
    let owned = r#"module spec.cpr.fixediterowned;

import { String, stringOf } from std.text;

struct Box { xs: String[2]; }

func use(c: bool): i32 {
    let b1: Box = Box { xs: [stringOf("AA"), stringOf("BB")] };
    let b2: Box = Box { xs: [stringOf("CC"), stringOf("DD")] };
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2).xs) {
        s = s + i32(len(x));
    }
    return s + i32(len(b1.xs[0])) + i32(len(b1.xs[1])) - 8;
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    let output = run_memory("fixediterowned", owned);
    assert_no_asan("fixediterowned", &output);
    assert_exit("fixediterowned", &output, 0);

    let rows = r#"module spec.cpr.fixediterrows;

import { String, stringOf } from std.text;

struct Row { v: String; }
struct Box { rows: Row[2]; tag: String; }

func use(c: bool): i32 {
    let b1: Box = Box { rows: [Row { v: stringOf("AA") }, Row { v: stringOf("BB") }], tag: stringOf("p") };
    let b2: Box = Box { rows: [Row { v: stringOf("CC") }, Row { v: stringOf("DD") }], tag: stringOf("q") };
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2).rows) {
        s = s + i32(len(x.v));
    }
    return s + i32(len(b1.rows[0].v)) - 6;
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    let output = run_memory("fixediterrows", rows);
    assert_no_asan("fixediterrows", &output);
    assert_exit("fixediterrows", &output, 0);
}

/// A re-executed loop over a conditional root's field keeps the field.
///
/// The inner loop runs once per outer iteration over `(c ? b1 : b2).xs`; the
/// trailing read-back of `b1.xs[0]` proves every execution read the selected
/// field in place. The nested spelling iterates `(c ? b1 : b2).p.a` — a
/// two-step field chain — and reads `b1.p.a[1]` back. Both programs build
/// under the strict flags, exit 0, and report no memory error.
#[test]
fn should_keep_a_conditional_roots_field_across_a_re_executed_loop() {
    let looped = r#"module spec.cpr.fixediterloop;

import { String, stringOf } from std.text;

struct Box { xs: String[2]; tag: String; }

func use(c: bool, k: i32): i32 {
    let b1: Box = Box { xs: [stringOf("AA"), stringOf("BB")], tag: stringOf("p") };
    let b2: Box = Box { xs: [stringOf("CC"), stringOf("DD")], tag: stringOf("q") };
    let s: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        for (borrow x in (c ? b1 : b2).xs) {
            s = s + i32(len(x));
        }
        i = i + 1;
    }
    return s + i32(len(b1.xs[0])) - 6;
}

func main(): i32 {
    return use(true, 2) + use(false, 2) - 8;
}
"#;
    let output = run_memory("fixediterloop", looped);
    assert_no_asan("fixediterloop", &output);
    assert_exit("fixediterloop", &output, 0);

    let nested = r#"module spec.cpr.fixediternested;

import { String, stringOf } from std.text;

struct Pair { a: String[2]; }
struct Box { p: Pair; tag: String; }

func use(c: bool): i32 {
    let b1: Box = Box { p: Pair { a: [stringOf("AA"), stringOf("BB")] }, tag: stringOf("p") };
    let b2: Box = Box { p: Pair { a: [stringOf("CC"), stringOf("DD")] }, tag: stringOf("q") };
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2).p.a) {
        s = s + i32(len(x));
    }
    return s + i32(len(b1.p.a[1])) - 6;
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    let output = run_memory("fixediternested", nested);
    assert_no_asan("fixediternested", &output);
    assert_exit("fixediternested", &output, 0);
}

// ---------------------------------------------------------------------------
// fixedstorelit: a conditional root's ownerless arm binds before its field is
// copied.
//
// `(c ? Box { ... } : b2).xs` in a consuming position cannot leave the literal
// arm inline: the literal is an lvalue there, but its fresh owned fields must
// release with the statement. Each ownerless arm's root binds on its own path
// — the literal spelling and the call spelling alike — while a place arm keeps
// its storage. The store, the `for` iterable, and the return spellings each
// accept, build under the strict flags, exit 0, and report no memory error.
// ---------------------------------------------------------------------------

/// A conditional root's literal or call arm binds its root in the store.
///
/// `let a: u8[4] = (c ? Box { xs: [...], tag: stringOf("fresh") } : b2).xs;`
/// copies the selected arm's field and releases the literal's fresh `tag` when
/// the statement ends, while `b2.xs` stays readable on the place path. The
/// call spelling `(c ? mk() : b2).xs` binds the call root first, so the copy
/// names `mk()`'s field through a local. Both programs exit 0.
#[test]
fn should_bind_an_ownerless_arm_of_a_conditional_root_store() {
    let literal = r#"module spec.cpr.fixedstorelit;

import { String, stringOf } from std.text;

struct Box { xs: u8[4]; tag: String; }

func use(c: bool): i32 {
    let b2: Box = Box { xs: [5, 6, 7, 8], tag: stringOf("pp") };
    let a: u8[4] = (c ? Box { xs: [1, 2, 3, 4], tag: stringOf("fresh") } : b2).xs;
    return i32(a[0]) + i32(b2.xs[0]) - 6;
}

func main(): i32 {
    return use(true) + use(false) - 4;
}
"#;
    let output = run_memory("fixedstorelit", literal);
    assert_no_asan("fixedstorelit", &output);
    assert_exit("fixedstorelit", &output, 0);

    let call = r#"module spec.cpr.fixedstorecall;

struct Box { xs: u8[4]; }

func mk(): Box { return Box { xs: [1, 2, 3, 4] }; }

func use(c: bool, b2: Box): i32 {
    let a: u8[4] = (c ? mk() : b2).xs;
    return i32(a[0]) + i32(a[3]);
}

func main(): i32 {
    let b2: Box = Box { xs: [5, 6, 7, 8] };
    return use(true, b2) + use(false, b2) - 18;
}
"#;
    let output = run_memory("fixedstorecall", call);
    assert_no_asan("fixedstorecall", &output);
    assert_exit("fixedstorecall", &output, 0);
}

/// A conditional root's ownerless arm binds in the iterable and return too.
///
/// `for (x in (c ? Box { xs: [...], tag: stringOf("fresh") } : b2).xs)` reads
/// the selected arm's field in place and releases the literal's fresh `tag`
/// with the loop's statement, so the summed elements and the release do not
/// collide. The return spelling `return (c ? Box { ... } : b2).xs;` binds the
/// literal arm's root before the field copy, so the fresh `tag` releases with
/// the return's cleanup. Both programs exit 0 with no memory report.
#[test]
fn should_bind_an_ownerless_arm_of_a_conditional_root_iterable_and_return() {
    let iterable = r#"module spec.cpr.fixediterlit;

import { String, stringOf } from std.text;

struct Box { xs: u8[4]; tag: String; }

func use(c: bool): i32 {
    let b2: Box = Box { xs: [5, 6, 7, 8], tag: stringOf("pp") };
    let s: i32 = 0;
    for (x in (c ? Box { xs: [1, 2, 3, 4], tag: stringOf("fresh") } : b2).xs) {
        s = s + i32(x);
    }
    return s;
}

func main(): i32 {
    return use(true) + use(false) - 36;
}
"#;
    let output = run_memory("fixediterlit", iterable);
    assert_no_asan("fixediterlit", &output);
    assert_exit("fixediterlit", &output, 0);

    let returned = r#"module spec.cpr.fixedreturnlit;

import { String, stringOf } from std.text;

struct Box { xs: u8[4]; tag: String; }

func pick(c: bool, b2: Box): u8[4] {
    return (c ? Box { xs: [1, 2, 3, 4], tag: stringOf("fresh") } : b2).xs;
}

func sum(a: u8[4]): i32 {
    return i32(a[0]) + i32(a[3]);
}

func main(): i32 {
    let b2: Box = Box { xs: [5, 6, 7, 8], tag: stringOf("pp") };
    return sum(pick(true, b2)) + sum(pick(false, b2)) - 18;
}
"#;
    let output = run_memory("fixedreturnlit", returned);
    assert_no_asan("fixedreturnlit", &output);
    assert_exit("fixedreturnlit", &output, 0);
}

// ---------------------------------------------------------------------------
// nestedarm: a conditional whose arm projects through a nested conditional.
//
// `(c ? (d ? b1 : b2).ys : b3.ys)` selects the outer root first and then the
// inner one. The projection must distribute into the nested arms, so the
// emission selects between the arms' own fields (`c ? (d ? b1.ys : b2.ys) :
// b3.ys`) — an array of real storage. Leaving the nested arm inline would
// name a member of a struct conditional's C temporary, which cannot be
// addressed and does not outlive the expression. The store spelling and the
// borrow iterable (including the element-index and else-arm spellings) are
// accepted, build under the strict flags, exit 0, and report no memory error.
// ---------------------------------------------------------------------------

/// A nested-arm projection stores the selected root's own field.
///
/// `let a: u8[4] = (c ? (d ? b1 : b2).ys : b3.ys);` copies `b1.ys`, `b2.ys`,
/// or `b3.ys` according to the two conditions; the reads of `a` are 0, 16, and
/// 8 for the three selections, so the trailing constant cancels them. The
/// program is accepted, builds under the strict flags, and exits 0.
#[test]
fn should_store_a_conditional_arm_projecting_through_a_nested_conditional() {
    let source = r#"module spec.cpr.nestedarmstore;

struct Box { ys: u8[4]; }
struct Other { ys: u8[4]; }

func use(c: bool, d: bool, b1: Box, b2: Box, b3: Other): i32 {
    let a: u8[4] = (c ? (d ? b1 : b2).ys : b3.ys);
    return i32(a[0]) + i32(a[3]) - 5;
}

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3, 4] };
    let b2: Box = Box { ys: [5, 6, 7, 8] };
    let b3: Other = Other { ys: [9, 10, 11, 12] };
    return use(true, true, b1, b2, b3) + use(false, true, b1, b2, b3) + use(true, false, b1, b2, b3) - 24;
}
"#;
    let output = run_memory("nestedarmstore", source);
    assert_no_asan("nestedarmstore", &output);
    assert_exit("nestedarmstore", &output, 0);
}

/// A nested-arm projection iterates the selected root's own field in place.
///
/// `for (borrow x in (c ? (d ? b1 : b2).ys : b3.ys))` sums the selected
/// field: 10, 42, and 26 for the three selections of the then-arm spelling;
/// the element-index spelling `(c ? (d ? b1 : b2).grid[0] : b3.grid[0])`
/// selects a row the same way (6, 42, 24), and the else-arm spelling
/// `(c ? b3.ys : (d ? b1 : b2).ys)` selects from the other side (42, 10, 26).
/// Every program is accepted, builds under the strict flags, exits 0, and
/// reports no memory error.
#[test]
fn should_iterate_a_conditional_arm_projecting_through_a_nested_conditional() {
    let arm = r#"module spec.cpr.nestedarmiter;

struct Box { ys: u8[4]; }
struct Other { ys: u8[4]; }

func use(c: bool, d: bool, b1: Box, b2: Box, b3: Other): i32 {
    let s: i32 = 0;
    for (borrow x in (c ? (d ? b1 : b2).ys : b3.ys)) {
        s = s + i32(x);
    }
    return s;
}

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3, 4] };
    let b2: Box = Box { ys: [5, 6, 7, 8] };
    let b3: Other = Other { ys: [9, 10, 11, 12] };
    return use(true, true, b1, b2, b3) + use(false, true, b1, b2, b3) + use(true, false, b1, b2, b3) - 78;
}
"#;
    let output = run_memory("nestedarmiter", arm);
    assert_no_asan("nestedarmiter", &output);
    assert_exit("nestedarmiter", &output, 0);

    let indexed = r#"module spec.cpr.nestedarmiterindex;

struct Box { grid: u8[2][3]; }
struct Other { grid: u8[2][3]; }

func use(c: bool, d: bool, b1: Box, b2: Box, b3: Other): i32 {
    let s: i32 = 0;
    for (borrow x in (c ? (d ? b1 : b2).grid[0] : b3.grid[0])) {
        s = s + i32(x);
    }
    return s;
}

func main(): i32 {
    let b1: Box = Box { grid: [[1, 2, 3], [4, 5, 6]] };
    let b2: Box = Box { grid: [[7, 8, 9], [10, 11, 12]] };
    let b3: Other = Other { grid: [[13, 14, 15], [16, 17, 18]] };
    return use(true, true, b1, b2, b3) + use(false, true, b1, b2, b3) + use(true, false, b1, b2, b3) - 72;
}
"#;
    let output = run_memory("nestedarmiterindex", indexed);
    assert_no_asan("nestedarmiterindex", &output);
    assert_exit("nestedarmiterindex", &output, 0);

    let else_arm = r#"module spec.cpr.nestedarmiterelse;

struct Box { ys: u8[4]; }
struct Other { ys: u8[4]; }

func use(c: bool, d: bool, b1: Box, b2: Box, b3: Other): i32 {
    let s: i32 = 0;
    for (borrow x in (c ? b3.ys : (d ? b1 : b2).ys)) {
        s = s + i32(x);
    }
    return s;
}

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3, 4] };
    let b2: Box = Box { ys: [5, 6, 7, 8] };
    let b3: Other = Other { ys: [9, 10, 11, 12] };
    return use(true, true, b1, b2, b3) + use(false, true, b1, b2, b3) + use(false, false, b1, b2, b3) - 78;
}
"#;
    let output = run_memory("nestedarmiterelse", else_arm);
    assert_no_asan("nestedarmiterelse", &output);
    assert_exit("nestedarmiterelse", &output, 0);
}

// ---------------------------------------------------------------------------
// assigntarget: an assignment target and an updating right-hand side.
//
// `arr[i] = i++;`, `i = i++;`, and `i += i++;` share one place between the
// target's location and the right-hand side's update. The language orders the
// target's location first, then the right-hand side, and completes the update
// before the store: the target index reads the old value, the postfix yields
// the old value, and the store writes after the increment. A plain C expression
// leaves that order unspecified, so the emitted C must pin the target's
// location parts and the update-containing right-hand side. The program is
// accepted, builds under the strict flags, and exits 0.
// ---------------------------------------------------------------------------

/// An assignment target and an updating right-hand side evaluate in order.
///
/// `arr[i] = i++;` keeps `arr[0]` as the stored old value 0 while the old index
/// 0 picks the slot and `i` becomes 1, so `arr[1]` and `arr[2]` keep 6 and 7.
/// `i = i++;` completes the increment and then stores the yielded old value, so
/// `i` stays 1. `i += i++;` reads the current value (1) before the right-hand
/// side, adds the yielded 1, and stores 2. Each intermediate is checked against
/// its own baseline, so the combined result cancels exactly when every order
/// holds; the run exits 0 and is clean.
#[test]
fn should_pin_an_assignment_target_before_an_updating_right_hand_side() {
    let source = r#"module spec.cpr.assigntarget;

func main(): i32 {
    let arr: i32[3] = [5, 6, 7];
    let i: i32 = 0;
    arr[i] = i++;
    let a: i32 = arr[0] + (arr[1] - 6) + (arr[2] - 7) + (i - 1);
    i = i++;
    let b: i32 = i - 1;
    i += i++;
    let c: i32 = i - 2;
    return a + b + c;
}
"#;
    let output = run_memory("assigntarget", source);
    assert_no_asan("assigntarget", &output);
    assert_exit("assigntarget", &output, 0);
}

// ---------------------------------------------------------------------------
// literalorder: an update inside a literal field or element.
//
// `P { a: i++, b: i }` and `[i++, i]` share the value of `i` between an
// update-containing operand and a plain read. The language evaluates struct
// fields and array elements in source order, so the update-containing operand
// yields the old value and increments, and the operand after it reads the
// updated value. The struct literal's emitted initializer must pin the update
// where it is written, because the C initializer order is unspecified. The
// program is accepted, builds under the strict flags, and exits 0.
// ---------------------------------------------------------------------------

/// A literal's update-containing field and element evaluate in source order.
///
/// In `P { a: i++, b: i }` field `a` takes the old value 0 while field `b`
/// reads the updated 1; in `[j++, j]` element 0 takes 0 and element 1 reads 1.
/// Each value is checked against its source-order baseline, so the result
/// cancels only when both literals evaluate left to right; the run exits 0 and
/// is clean.
#[test]
fn should_evaluate_update_containing_literals_in_source_order() {
    let source = r#"module spec.cpr.literalorder;

struct P { a: i32; b: i32; }

func main(): i32 {
    let i: i32 = 0;
    let p: P = P { a: i++, b: i };
    let j: i32 = 0;
    let xs: i32[2] = [j++, j];
    return p.a + (p.b - 1) + xs[0] + (xs[1] - 1);
}
"#;
    let output = run_memory("literalorder", source);
    assert_no_asan("literalorder", &output);
    assert_exit("literalorder", &output, 0);
}

// ---------------------------------------------------------------------------
// dotreceiver: the receiver of a dot-call with an update.
//
// `(i++).get(i)` resolves to the free function `get(i++, i)`: the receiver is
// argument position zero, evaluated before the written arguments. The update
// therefore yields the old value while the argument reads the incremented
// value. A C call leaves its argument order unspecified, so the receiver must
// pin where it is written — in a single-shot call and in a loop body that
// re-executes the call alike. Both programs are accepted, build under the
// strict flags, and exit 0.
// ---------------------------------------------------------------------------

/// A dot receiver with an update evaluates before the call's arguments.
///
/// The single-shot `r = (i++).get(i)` yields `get(0, 1)` = 1 while `i` becomes
/// 1. The loop spelling adds `(i++).get(i)` per iteration: `get(0, 1)` = 1 then
/// `get(1, 2)` = 12 while `i` becomes 2, and the loop condition reads the
/// updated value. Both results check against their left-to-right baselines, so
/// each run exits 0; both are clean.
#[test]
fn should_evaluate_an_updating_dot_receiver_before_its_arguments() {
    let single = r#"module spec.cpr.dotreceiver;

func get(a: i32, b: i32): i32 { return a * 10 + b; }

func main(): i32 {
    let i: i32 = 0;
    let r: i32 = (i++).get(i);
    return r + i - 2;
}
"#;
    let output = run_memory("dotreceiver", single);
    assert_no_asan("dotreceiver", &output);
    assert_exit("dotreceiver", &output, 0);

    let looped = r#"module spec.cpr.dotreceiverloop;

func get(a: i32, b: i32): i32 { return a * 10 + b; }

func main(): i32 {
    let i: i32 = 0;
    let s: i32 = 0;
    while (i < 2) {
        s = s + (i++).get(i);
    }
    return s + i - 15;
}
"#;
    let output = run_memory("dotreceiverloop", looped);
    assert_no_asan("dotreceiverloop", &output);
    assert_exit("dotreceiverloop", &output, 0);
}

// ---------------------------------------------------------------------------
// dotreceivercond: a bare-update dot receiver inside a loop condition.
//
// `while ((i++).get(i) < 20)` evaluates its condition once per test: the
// receiver's update yields the old value, the argument reads the updated one,
// and the call's result decides the test. The condition's operands must pin
// per iteration, because a statement before the loop would run the update once
// and leave the test constant. The program is accepted, builds under the
// strict flags, exits 0, and reports no memory error.
// ---------------------------------------------------------------------------

/// A bare-update dot receiver in a loop condition pins per iteration.
///
/// The test computes `get(i, i + 1)` = `11 * i + 1`, so the tests at `i` = 0
/// and 1 pass (1 and 12) while `i` = 2 fails (23); the body counts 2
/// iterations and the final `i` is 3, so `s + i - 5` cancels. The run exits 0
/// and is clean.
#[test]
fn should_pin_a_loop_conditions_updating_dot_receiver_per_iteration() {
    let source = r#"module spec.cpr.dotreceivercond;

func get(a: i32, b: i32): i32 { return a * 10 + b; }

func main(): i32 {
    let i: i32 = 0;
    let s: i32 = 0;
    while ((i++).get(i) < 20) {
        s = s + 1;
    }
    return s + i - 5;
}
"#;
    let output = run_memory("dotreceivercond", source);
    assert_no_asan("dotreceivercond", &output);
    assert_exit("dotreceivercond", &output, 0);
}

// ---------------------------------------------------------------------------
// loopcondupdate: an update inside a loop condition.
//
// `while (i + id(i++) < 3)` shares `i` between the left operand's read and the
// right operand's update. A loop condition is re-evaluated every iteration, so
// the update runs each time: the left operand reads the value at the test, the
// postfix yields that value and increments, and the test advances through 0, 1,
// then 2 before it stops. The pin must live inside the per-iteration test; a
// statement pin before the loop would run the update once and leave the
// condition constant. The program is accepted, builds under the strict flags,
// and exits 0 with the loop's own count.
// ---------------------------------------------------------------------------

/// A loop condition with an update evaluates the update every iteration.
///
/// The test `i + id(i++) < 3` compares twice the current `i` against 3, so the
/// body runs for `i` = 0 and `i` = 1 while the third test increments `i` to 3
/// before it fails. The body counts 2 iterations and the final `i` is 3, so
/// `n + i - 5` cancels; the run exits 0 and is clean.
#[test]
fn should_pin_an_update_in_a_loop_condition_per_iteration() {
    let source = r#"module spec.cpr.loopcondupdate;

func id(x: i32): i32 { return x; }

func main(): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i + id(i++) < 3) {
        n++;
    }
    return n + i - 5;
}
"#;
    let output = run_memory("loopcondupdate", source);
    assert_no_asan("loopcondupdate", &output);
    assert_exit("loopcondupdate", &output, 0);
}

// ---------------------------------------------------------------------------
// owningarg: an owning argument before a later update.
//
// `f(g(), i++)` passes a fresh owning result as the first argument and an
// update as the second. Evaluation is strictly left to right, so `g()` must run
// before the update — its result encodes what it saw. A naive update pin runs
// before the call and therefore before the owning argument's binding, so the
// binding must move to the argument's own position while the ownership still
// transfers exactly once. The program is accepted, builds under the strict
// flags, and exits 0.
// ---------------------------------------------------------------------------

/// An owning argument binds before a later argument's update.
///
/// `g()` reports whether it saw the update: it records 0 while the global `i`
/// is still 0 and 1 once it changed. With the left-to-right order `g()` runs
/// first, so the result is 0 and `i` becomes 1; the call's value
/// `seen * 10 + n` is 0 and `r + i - 1` cancels. Under the reversed argument
/// order the observation fails and the run exits 10. The owning result's string
/// field transfers into the callee and releases once, so the run is clean.
#[test]
fn should_evaluate_an_owning_argument_before_a_later_update() {
    let source = r#"module spec.cpr.owningarg;

import { String, stringOf } from std.text;

struct Box { v: String; seen: i32; }

let i: i32 = 0;

func g(): Box {
    if (i == 0) {
        return Box { v: stringOf("ax"), seen: 0 };
    }
    return Box { v: stringOf("ax"), seen: 1 };
}

func f(take b: Box, n: i32): i32 {
    return b.seen * 10 + n;
}

func main(): i32 {
    let r: i32 = f(g(), i++);
    return r + i - 1;
}
"#;
    let output = run_memory("owningarg", source);
    assert_no_asan("owningarg", &output);
    assert_exit("owningarg", &output, 0);
}

// ---------------------------------------------------------------------------
// targetupd: a store whose target index and right-hand side both update.
//
// `arr[i++] = i++;` reads the target's location first: the index update
// selects the slot, the side's update yields the stored value, and the store
// follows both. The call-index spelling `arr[mark(i)] = i++;` orders the
// target's call before the side, and the update-only spelling `arr[i++]++;`
// reads and rewrites the slot the index update selected.
// ---------------------------------------------------------------------------

/// A target index and the right-hand side that both update a place.
///
/// `arr[i++] = i++;` stores the side's yielded 1 into `arr[0]` while the index
/// update advances `i` to 2; `arr[mark(i)] = i++;` then marks index 2 and
/// stores the yielded 2 there, so `arr` is `[1, 6, 2]` and the checks cancel.
/// Both programs are accepted, build under the strict flags, and exit 0 with
/// no memory report.
#[test]
fn should_store_a_target_index_that_updates_the_same_place() {
    let source = r#"module spec.cpr.targetupd;

let seen: i32 = 0;

func mark(x: i32): i32 {
    seen = seen * 10 + x;
    return x;
}

func main(): i32 {
    let arr: i32[3] = [5, 6, 7];
    let i: i32 = 0;
    arr[i++] = i++;
    let a: i32 = (arr[0] - 1) + (arr[1] - 6) + (arr[2] - 7) + (i - 2);
    arr[mark(i)] = i++;
    let b: i32 = (seen - 2) + (arr[1] - 6) + (arr[2] - 2) + (i - 3);
    return a + b;
}
"#;
    let output = run_memory("targetupd", source);
    assert_no_asan("targetupd", &output);
    assert_exit("targetupd", &output, 0);
}

/// An increment through an updating element index.
///
/// `arr[i++]++` increments the slot the index update selected — `arr[0]`
/// becomes 6 while `i` becomes 1 — and the checks cancel only when the read
/// and the store share that index. The program is accepted, builds under the
/// strict flags, and exits 0 with no memory report.
#[test]
fn should_update_an_element_through_an_updating_index() {
    let source = r#"module spec.cpr.targetupdindex;

func main(): i32 {
    let arr: i32[3] = [5, 6, 7];
    let i: i32 = 0;
    arr[i++]++;
    return (arr[0] - 6) + (arr[1] - 6) + (arr[2] - 7) + (i - 1);
}
"#;
    let output = run_memory("targetupdindex", source);
    assert_no_asan("targetupdindex", &output);
    assert_exit("targetupdindex", &output, 0);
}

// ---------------------------------------------------------------------------
// receiverorder: a dot receiver with an updating index before its argument.
//
// `arr[i++].takeIt(i)` and `hs[i++].call(i)` read the receiver's location
// first: the index update selects the receiver, the written argument then
// reads the updated value, and the callee receives both in that order. The
// borrow-receiver spelling `bs[i++].bump(i)` writes through the receiver's
// storage after the same order.
// ---------------------------------------------------------------------------

/// An updating receiver index evaluates before the written argument.
///
/// The take receiver `arr[i++].takeIt(i)` selects `arr[0]` (v = 1) and passes
/// the updated `i` = 1, so the callee computes 11; the func-field receiver
/// `hs[i++].call(i)` selects the second-shaped first entry and passes 1, so
/// `addOne` computes 2; the borrow receiver `bs[i++].bump(i)` writes `n` = 2
/// through `bs[0]` and passes the updated 1. Every program is accepted,
/// builds under the strict flags, exits 0, and reports no memory error.
#[test]
fn should_evaluate_an_updating_receiver_index_before_its_argument() {
    let take_receiver = r#"module spec.cpr.receivertake;

struct Box { v: i32; }

func takeIt(take b: Box, n: i32): i32 { return b.v * 10 + n; }

func main(): i32 {
    let arr: Box[2] = [Box { v: 1 }, Box { v: 2 }];
    let i: i32 = 0;
    let r: i32 = arr[i++].takeIt(i);
    return r + i - 12;
}
"#;
    let output = run_memory("receivertake", take_receiver);
    assert_no_asan("receivertake", &output);
    assert_exit("receivertake", &output, 0);

    let field_receiver = r#"module spec.cpr.receiverfield;

func addOne(x: i32): i32 { return x + 1; }

struct H { call: func(i32): i32; }

func main(): i32 {
    let hs: H[2] = [H { call: addOne }, H { call: addOne }];
    let i: i32 = 0;
    let r: i32 = hs[i++].call(i);
    return r + i - 3;
}
"#;
    let output = run_memory("receiverfield", field_receiver);
    assert_no_asan("receiverfield", &output);
    assert_exit("receiverfield", &output, 0);

    let borrow_receiver = r#"module spec.cpr.receiverborrow;

import { String, stringOf } from std.text;

struct Box { v: String; n: i32; }

func bump(borrow b: Box, k: i32): i32 {
    b.n = b.n + k;
    return b.n;
}

func main(): i32 {
    let bs: Box[2] = [Box { v: stringOf("a"), n: 1 }, Box { v: stringOf("b"), n: 2 }];
    let i: i32 = 0;
    let r: i32 = bs[i++].bump(i);
    return r + bs[0].n + i - 5;
}
"#;
    let output = run_memory("receiverborrow", borrow_receiver);
    assert_no_asan("receiverborrow", &output);
    assert_exit("receiverborrow", &output, 0);
}

// ---------------------------------------------------------------------------
// literalfieldorder: an owning update-containing field before a later call.
//
// `P { a: mk(i++), b: g() }` evaluates field `a` first: `mk` runs with the
// yielded 0 and records that it ran; field `b`'s `g()` then observes the
// recorded call. Pinning only the later field would hoist `g()` above `mk`,
// so the owning field must bind at its own position too.
// ---------------------------------------------------------------------------

/// An owning literal field binds before a later field's call.
///
/// `mk(i++)` records `c` = 1 while the update yields 0, and `g()` then reads
/// the recorded 1 into `p.b`, so the run exits 0 only when the owning field's
/// call precedes the later field's. The program is accepted, builds under the
/// strict flags, and reports no memory error.
#[test]
fn should_evaluate_an_owning_literal_field_before_a_later_field() {
    let source = r#"module spec.cpr.literalfieldorder;

import { String, stringOf } from std.text;

let c: i32 = 0;

struct P { a: String; b: i32; }

func mk(x: i32): String {
    c = c + 1;
    return stringOf("ab");
}

func g(): i32 { return c; }

func main(): i32 {
    let i: i32 = 0;
    let p: P = P { a: mk(i++), b: g() };
    return p.b + i - 2;
}
"#;
    let output = run_memory("literalfieldorder", source);
    assert_no_asan("literalfieldorder", &output);
    assert_exit("literalfieldorder", &output, 0);
}

// ---------------------------------------------------------------------------
// updatecontrols: the shared-update spellings that must not change.
//
// `i++ + i`, `f(i++, i)`, and the standalone `i++` already pin their updates
// and the reads the updates can affect into temporaries, so their order is
// defined and their behavior is fixed. They stay accepted, build under the
// strict flags, run in the left-to-right order, and remain memory-clean.
// ---------------------------------------------------------------------------

/// The already-pinned update spellings stay accepted and ordered.
///
/// `j = i++ + i` takes the yielded 0 and the read 1; `k = f(i++, i)` passes 1
/// and 2; the standalone `i++` leaves 3. The checks cancel against those
/// values, so the run exits 0 and is clean.
#[test]
fn should_keep_the_shared_update_spellings_accepted() {
    let source = r#"module spec.cpr.updatecontrols;

func f(a: i32, b: i32): i32 { return a * 10 + b; }

func main(): i32 {
    let i: i32 = 0;
    let j: i32 = i++ + i;
    let k: i32 = f(i++, i);
    i++;
    return j + k + i - 16;
}
"#;
    let output = run_memory("updatecontrols", source);
    assert_no_asan("updatecontrols", &output);
    assert_exit("updatecontrols", &output, 0);
}

// ---------------------------------------------------------------------------
// dupresidual: a duplicable projection over a residual-owning fresh root.
//
// `c ? a.n : mkBox().n` copies the duplicable `n` member, so the carrying
// gates treat the result as a plain value and never bind the fresh `mkBox()`
// root for the release of its un-moved sibling field: each fresh evaluation
// leaks that field. The take and return positions and the element-read twins
// `(c ? mk() : b).ys[i]` and `mk().ys[i]` carry the same rule. The fresh root
// must bind for its residual release while the selected value stays a copy;
// the place-arm selection and the concrete `String`-member twin already bind
// their roots and are the controls.
// ---------------------------------------------------------------------------

/// A duplicable projection's fresh root releases its residual storage.
///
/// `holdNum(c ? a.n : mkBox(7).n)` and the return twin copy the selected
/// `i32`; the fresh arm's root still owns its `String` field, which must
/// release once per evaluation. The read positions `(c ? mk() : b).ys[i]`
/// and `mk().ys[i]` copy an element the same way. Every selection is read
/// back through the exit code (the place arm contributes 3, the fresh arm 7,
/// and the element values 1..4 and 9..12), so a dropped or misread value
/// fails the run. The place-arm selection and the concrete `String`-member
/// twin bind their roots and stay clean.
#[test]
fn should_release_the_residual_root_of_a_duplicable_projection() {
    let place_control = r#"module spec.cpr.dupresidualplace;

import { String, stringOf } from std.text;

struct Box { n: i32; v: String; }

func mkBox(k: i32): Box { return Box { n: k, v: stringOf("ab") }; }
func holdNum(take n: i32): i32 { return n; }

func use(c: bool, take a: Box): i32 {
    return holdNum(c ? a.n : mkBox(7).n);
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let a: Box = Box { n: 3, v: stringOf("xx") };
        n = n + use(true, a);
        i = i + 1;
    }
    return n - 9;
}
"#;
    let output = run_memory("dupresidualplace", place_control);
    assert_no_asan("dupresidualplace", &output);
    assert_exit("dupresidualplace", &output, 0);

    let member_control = r#"module spec.cpr.dupresidualmember;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mkBox(): Box { return Box { v: stringOf("abc"), w: stringOf("cd") }; }
func holdStr(take s: String): i32 { return i32(len(s)); }

func takePos(c: bool, take a: Box): i32 {
    return holdStr(c ? a.v : mkBox().v);
}

func retPos(c: bool, take a: Box): i32 {
    return i32(len(c ? a.v : mkBox().v));
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let a1: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
        n = n + takePos(true, a1);
        let a2: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
        n = n + takePos(false, a2);
        let a3: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
        n = n + retPos(true, a3);
        let a4: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
        n = n + retPos(false, a4);
        i = i + 1;
    }
    return n - 30;
}
"#;
    let output = run_memory("dupresidualmember", member_control);
    assert_no_asan("dupresidualmember", &output);
    assert_exit("dupresidualmember", &output, 0);

    let take_and_return = r#"module spec.cpr.dupresidual;

import { String, stringOf } from std.text;

struct Box { n: i32; v: String; }

func mkBox(k: i32): Box { return Box { n: k, v: stringOf("ab") }; }
func holdNum(take n: i32): i32 { return n; }

func takePos(c: bool, take a: Box): i32 {
    return holdNum(c ? a.n : mkBox(7).n);
}

func retPos(c: bool, take a: Box): i32 {
    return (c ? a.n : mkBox(7).n);
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let a1: Box = Box { n: 3, v: stringOf("xx") };
        n = n + takePos(true, a1);
        let a2: Box = Box { n: 3, v: stringOf("xx") };
        n = n + takePos(false, a2);
        let a3: Box = Box { n: 3, v: stringOf("xx") };
        n = n + retPos(true, a3);
        let a4: Box = Box { n: 3, v: stringOf("xx") };
        n = n + retPos(false, a4);
        i = i + 1;
    }
    return n - 60;
}
"#;
    let output = run_memory("dupresidual", take_and_return);
    assert_no_asan("dupresidual", &output);
    assert_exit("dupresidual", &output, 0);

    let reads = r#"module spec.cpr.dupresidualread;

import { String, stringOf } from std.text;

struct Box { tag: String; ys: u8[4]; }

func mk(): Box { return Box { tag: stringOf("one"), ys: [1, 2, 3, 4] }; }

func readCond(c: bool, b: Box): i32 {
    let s: i32 = 0;
    let i: i32 = 0;
    while (i < 4) {
        s = s + i32((c ? mk() : b).ys[i]);
        i = i + 1;
    }
    return s;
}

func readDirect(): i32 {
    let s: i32 = 0;
    let i: i32 = 0;
    while (i < 4) {
        s = s + i32(mk().ys[i]);
        i = i + 1;
    }
    return s;
}

func main(): i32 {
    let b: Box = Box { tag: stringOf("p"), ys: [9, 10, 11, 12] };
    let r: i32 = readCond(true, b);
    let b2: Box = Box { tag: stringOf("p"), ys: [9, 10, 11, 12] };
    r = r + readCond(false, b2);
    r = r + readDirect();
    return r - 62;
}
"#;
    let output = run_memory("dupresidualread", reads);
    assert_no_asan("dupresidualread", &output);
    assert_exit("dupresidualread", &output, 0);
}

// ---------------------------------------------------------------------------
// runtimestore: a runtime-index element store of a view.
//
// `xs[j] = b.v;` must copy the view and keep the backing's ownership where it
// is, exactly as the constant index does: the owner releases once at its own
// scope and the element loan is recorded, so a later write to the backing
// reports like the constant-index twin while the array still reads `b`'s
// bytes. The re-executed `while { xs[j] = mk().v; }` must reject like the
// constant-index loop store, because the fresh call result dies before the
// caller reads the element.
// ---------------------------------------------------------------------------

/// A runtime-index element store releases its owner and records its loan.
///
/// `xs[k] = b.v;` copies the view of the local `b`'s member into the element
/// slot: `b` keeps owning the storage and releases it once at the frame's
/// end, and the read of `xs[k]` returns the stored byte. A later write to
/// `b.v` reports the shared-borrow error once, exactly as the constant-index
/// spelling `xs[1] = b.v;` does. The re-executed `while { xs[k] = mk().v; }`
/// rejects with one `E10030`, the constant-index loop twin's judgment, and
/// emits no translation unit.
#[test]
fn should_release_and_loan_a_runtime_index_view_element_store() {
    let constant = r#"module spec.cpr.runtimestoreconst;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = b.v;
    return i32(xs[1][0]) - 120;
}

func main(): i32 { return pick(); }
"#;
    let output = run_memory("runtimestoreconst", constant);
    assert_no_asan("runtimestoreconst", &output);
    assert_exit("runtimestoreconst", &output, 0);

    let mutation = r#"module spec.cpr.runtimemut;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(k: i32): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = b.v;
    b.v = stringOf("qq");
    return i32(xs[k][0]) - 120;
}

func main(): i32 { return pick(1); }
"#;
    compile_rejects("runtimemut", mutation, "E10031", 1);

    let constant_mutation = r#"module spec.cpr.runtimemutconst;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = b.v;
    b.v = stringOf("qq");
    return i32(xs[1][0]) - 120;
}

func main(): i32 { return pick(); }
"#;
    compile_rejects("runtimemutconst", constant_mutation, "E10031", 1);

    let store = r#"module spec.cpr.runtimestore;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(k: i32): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = b.v;
    return i32(xs[k][0]) - 120;
}

func main(): i32 { return pick(1); }
"#;
    let output = run_memory("runtimestore", store);
    assert_no_asan("runtimestore", &output);
    assert_exit("runtimestore", &output, 0);

    let loop_store = r#"module spec.cpr.runtimestoreloop;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

func run(k: i32): i32 {
    let xs: str[2] = ["AA", "BB"];
    let i: i32 = 0;
    while (i < 3) {
        xs[k] = mk().v;
        i = i + 1;
    }
    return i32(xs[k][0]);
}

func main(): i32 { return run(1) - 120; }
"#;
    compile_rejects("runtimestoreloop", loop_store, "E10030", 1);
}

// ---------------------------------------------------------------------------
// crossframe: an out-slot pass-through across an intermediate frame.
//
// `outer(borrow slot: str?) { let b = ...; fill(b.v, borrow slot); }` lets the
// callee store a view of `outer`'s frame local into a slot in `outer`'s
// caller's frame. The callee owns the store, so the callee-side judgment
// reports one `E10034` for a foreign lend stored into its whole nullable
// out-slot. The fresh argument, the `str`-valued argument, and the
// array-literal spellings carry the same judgment; the array spelling keeps
// its `E10033` field/element escape beside the store report. The
// parameter-backed chain and the same-frame call also report `E10034` because
// the stored source remains a different non-`take` lend.
// ---------------------------------------------------------------------------

/// A cross-frame out-slot pass-through rejects like its direct twin.
///
/// `outer`'s local `b` dies with its frame while the caller's slot still
/// reads the view, so `fill(b.v, borrow slot)` reports exactly one `E10034`
/// and emits no translation unit; the fresh-argument spelling inside `outer`
/// and the `middle` chain (`fill(borrow out2, mk().v)`) report the same one
/// store error, and the array literal `slot = [p, q]` adds the callee's own
/// `E10033` field/element escape beside the store report. The
/// parameter-backed chain (`outer(p: Box, borrow slot) { fill(p.v, borrow
/// slot); }`) and the same-frame call store a foreign lend too, so they also
/// report `E10034` once.
#[test]
fn should_reject_a_cross_frame_out_slot_pass_through() {
    let parameter = r#"module spec.cpr.crossframeparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void { fill(p.v, borrow slot); }

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("crossframeparam", parameter, "E10034", 1);

    let same_frame = r#"module spec.cpr.crossframesame;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func demo(): i32 {
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    let out: str? = null;
    fill(b.v, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}

func main(): i32 { return demo(); }
"#;
    compile_rejects("crossframesame", same_frame, "E10034", 1);

    let local = r#"module spec.cpr.crossframelocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    fill(b.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("crossframelocal", local, "E10034", 1);

    let fresh = r#"module spec.cpr.crossframetemp;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void { fill(mk().v, borrow slot); }

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("crossframetemp", fresh, "E10034", 1);

    let str_twin = r#"module spec.cpr.crossframestrtwin;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

func fill(borrow slot: str?, p: str): void { slot = p; }

func middle(borrow out2: str?): void { fill(borrow out2, mk().v); }

func main(): i32 {
    let out: str? = null;
    middle(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("crossframestrtwin", str_twin, "E10034", 1);

    let array = r#"module spec.cpr.crossframearray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zw") }; }

func fill(p: str, q: str, borrow slot: str[2]): void { slot = [p, q]; }

func middle(borrow out2: str[2]): void { fill(mk().v, mk().w, borrow out2); }

func run(): i32 {
    let out: str[2] = ["AA", "BB"];
    middle(borrow out);
    return i32(out[0][0]) - 120;
}

func main(): i32 { return run(); }
"#;
    compile_reports("crossframearray", array, &["E10033", "E10030"]);
}

// ---------------------------------------------------------------------------
// modulefresh: a module view binding over a fresh owner.
//
// A module binding initialized from a fresh call holds a view of storage that
// nothing releases: `let g: str = mk().v;` leaks the call result's residual
// field at every program start. The view destination must retain the fresh
// owner for the module's storage, like the direct `stringOf` destination that
// is already clean, so the post-init read-back stays valid and the run is
// memory-clean. The static binding stays accepted as the control.
// ---------------------------------------------------------------------------

/// A module view destination retains the fresh owner of its binding.
///
/// `let g: str = mk().v;` stores a view of the call result's member; the
/// result's remaining field must release at the module's initialization while
/// the member stays owned for the module's lifetime, so `g` keeps reading
/// `xy` after the initializers run. `let g: str = stringOf("xy");` is the
/// direct destination that already retains its fresh owner, and the static
/// binding `let g: str = s;` is the control. Each read-back folds the stored
/// byte into the exit code, and each run is memory-clean.
#[test]
fn should_retain_the_fresh_owner_of_a_module_view_binding() {
    let direct = r#"module spec.cpr.modulefresh;

import { String, stringOf } from std.text;

let g: str = stringOf("xy");

func main(): i32 {
    return i32(g[0]) - 120;
}
"#;
    let output = run_memory("modulefresh", direct);
    assert_no_asan("modulefresh", &output);
    assert_exit("modulefresh", &output, 0);

    let control = r#"module spec.cpr.modulestatic;

let s: str = "st";
let g: str = s;

func main(): i32 {
    return i32(g[0]) + i32(s[1]) - 231;
}
"#;
    let output = run_memory("modulestatic", control);
    assert_no_asan("modulestatic", &output);
    assert_exit("modulestatic", &output, 0);

    let member = r#"module spec.cpr.modulefreshmember;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

let g: str = mk().v;

func main(): i32 {
    return i32(g[0]) - 120;
}
"#;
    let output = run_memory("modulefreshmember", member);
    assert_no_asan("modulefreshmember", &output);
    assert_exit("modulefreshmember", &output, 0);
}

// ---------------------------------------------------------------------------
// containerescape: a view-carrying container escapes with a frame-backed
// element loan.
//
// `xs[1] = b.v; return xs;` hands the caller an array whose element views the
// frame local `b`; the return must consult the binding's element loans and
// report the local-owner escape once, exactly as the single-view twin
// `let s: str = b.v; return s;` reports `E10035`. The alias-written spelling
// (`for (borrow e in xs) { e = b.v; }`) takes the same judgment, and the
// global destination reports its `E10033`. The parameter-backed return and
// the single-view twin stay as the controls: the parameter's storage is the
// caller's frame, so the returned array stays readable.
// ---------------------------------------------------------------------------

/// A container escape with a frame-backed element loan rejects per
/// destination.
///
/// `xs[1] = b.v; return xs;` returns a view of the local `b`, which dies with
/// the frame while the caller reads the element; the return reports exactly
/// one `E10035` and emits no translation unit. The alias spelling
/// `for (borrow e in xs) { e = b.v; } return xs;` reports the same one error,
/// and the global store `xs[1] = b.v; g = xs;` reports exactly one `E10033`.
/// The single-view twin `let s: str = b.v; return s;` keeps its one `E10035`,
/// and `xs[1] = p; return xs;` with a `str` parameter keeps its acceptance
/// and stays memory-clean.
#[test]
fn should_reject_a_container_escape_with_frame_backed_element_loans() {
    let view_twin = r#"module spec.cpr.viewreturntwin;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): str {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let s: str = b.v;
    return s;
}

func main(): i32 { return i32(run()[0]) - 120; }
"#;
    compile_rejects("viewreturntwin", view_twin, "E10035", 1);

    let parameter_twin = r#"module spec.cpr.returnparam;

import { String, stringOf } from std.text;

func run(p: str): str[2] {
    let xs: str[2] = ["AA", "BB"];
    xs[1] = p;
    return xs;
}

func main(): i32 {
    let s: String = stringOf("xy");
    return i32(run(s)[1][0]) - 120;
}
"#;
    let output = run_memory("returnparam", parameter_twin);
    assert_no_asan("returnparam", &output);
    assert_exit("returnparam", &output, 0);

    let element_return = r#"module spec.cpr.returningelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): str[2] {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    xs[1] = b.v;
    return xs;
}

func main(): i32 { return i32(run()[1][0]) - 120; }
"#;
    compile_rejects("returningelem", element_return, "E10035", 1);

    let alias_return = r#"module spec.cpr.returningalias;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): str[2] {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    for (borrow e in xs) {
        e = b.v;
    }
    return xs;
}

func main(): i32 { return i32(run()[0][0]) - 120; }
"#;
    compile_rejects("returningalias", alias_return, "E10035", 1);

    let global_store = r#"module spec.cpr.globalelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let g: str[2] = ["AA", "BB"];

func store(): void {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    xs[1] = b.v;
    g = xs;
}

func main(): i32 { store(); return i32(g[1][0]) - 120; }
"#;
    compile_rejects("globalelem", global_store, "E10033", 1);
}

// ---------------------------------------------------------------------------
// staleloan: an overwritten literal carrier drops its stale loan.
//
// `let h = Holder { names: [b.v] }; h.names = ["AA"];` replaces the stored
// view, so the carrier's loan on `b` has no live view to protect and must
// drop; the write `b.v = ...` then compiles and the run reads the replaced
// element. The whole-carrier reassignment drops the loans the same way. The
// element twin `xs[1] = b.v; xs[1] = "zz";` already replaces its loan
// cleanly, while a still-live view keeps its loan and the write reports one
// `E10031`.
// ---------------------------------------------------------------------------

/// An overwritten literal carrier drops its stale loans, a live view keeps
/// its own.
///
/// `h.names = ["AA"]` and the whole reassignment `h = Holder { names: ["AA"] }`
/// both replace the view that read `b.v`, so the following `b.v = ...` is
/// accepted and the run reads the replaced `AA` element; the same rule
/// already holds for the element twin `xs[1] = b.v; xs[1] = "zz";`. When no
/// overwrite replaces the view, the loan is live: the write reports exactly
/// one `E10031` and no translation unit is emitted.
#[test]
fn should_drop_the_stale_loans_of_an_overwritten_literal_carrier() {
    let element_twin = r#"module spec.cpr.elementtwin;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = b.v;
    xs[1] = "zz";
    b.v = stringOf("qq");
    return i32(xs[1][0]) - 122;
}

func main(): i32 { return pick(); }
"#;
    let output = run_memory("elementtwin", element_twin);
    assert_no_asan("elementtwin", &output);
    assert_exit("elementtwin", &output, 0);

    let live_loan = r#"module spec.cpr.liveloan;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    b.v = stringOf("qq");
    return i32(h.names[0][0]);
}

func main(): i32 { return pick(); }
"#;
    compile_rejects("liveloan", live_loan, "E10031", 1);

    let field_overwrite = r#"module spec.cpr.stalefield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    h.names = ["AA"];
    b.v = stringOf("qq");
    return i32(h.names[0][0]) - 65;
}

func main(): i32 { return pick(); }
"#;
    let output = run_memory("stalefield", field_overwrite);
    assert_no_asan("stalefield", &output);
    assert_exit("stalefield", &output, 0);

    let whole_reassignment = r#"module spec.cpr.stalewhole;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    h = Holder { names: ["AA"] };
    b.v = stringOf("qq");
    return i32(h.names[0][0]) - 65;
}

func main(): i32 { return pick(); }
"#;
    let output = run_memory("stalewhole", whole_reassignment);
    assert_no_asan("stalewhole", &output);
    assert_exit("stalewhole", &output, 0);
}

// ---------------------------------------------------------------------------
// allnull: an all-null composite store into a view destination.
//
// `slot = c ? null : null;` composes two null arms, so the value names no
// storage and cannot escape: the store is accepted and the emitted view stays
// null, exactly like the scalar `slot = null;` case. The global twin
// `g = c ? null : null;` is accepted for the same reason. The null-arm mixes
// keep their judgments: an arm that names a local backing still reports
// `E10030`, while a parameter-backed arm stays accepted.
// ---------------------------------------------------------------------------

/// An all-null composite store is accepted like the scalar null store.
///
/// `slot = c ? null : null;` into a `borrow str?` out-slot names no storage,
/// so nothing escapes and the caller's `null` view is preserved; the global
/// twin takes the same judgment. The controls keep their pinned behavior:
/// `slot = null;` is accepted, the mix `slot = c ? b.v : null;` with a
/// frame-local arm reports exactly one `E10030`, and the mix
/// `slot = c ? p.v : null;` with a parameter-backed arm is accepted and reads
/// the written view.
#[test]
fn should_accept_an_all_null_composite_view_store() {
    let single_null = r#"module spec.cpr.singlenull;

func fill(borrow slot: str?): void {
    slot = null;
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    if (out == null) {
        return 0;
    }
    return 9;
}
"#;
    let output = run_memory("singlenull", single_null);
    assert_no_asan("singlenull", &output);
    assert_exit("singlenull", &output, 0);

    let parameter_mix = r#"module spec.cpr.nullmixparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(c: bool, p: Box, borrow slot: str?): void {
    slot = c ? p.v : null;
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    fill(true, b, borrow out);
    if (out == null) {
        return 9;
    }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("nullmixparam", parameter_mix, "E10034", 1);

    let local_mix = r#"module spec.cpr.nullmixlocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(c: bool, borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    slot = c ? b.v : null;
}

func main(): i32 {
    let out: str? = null;
    fill(false, borrow out);
    if (out == null) {
        return 0;
    }
    return 9;
}
"#;
    compile_rejects("nullmixlocal", local_mix, "E10030", 1);

    let slot = r#"module spec.cpr.allnullslot;

func fill(c: bool, borrow slot: str?): void {
    slot = c ? null : null;
}

func main(): i32 {
    let out: str? = null;
    fill(true, borrow out);
    if (out == null) {
        return 0;
    }
    return 9;
}
"#;
    let output = run_memory("allnullslot", slot);
    assert_no_asan("allnullslot", &output);
    assert_exit("allnullslot", &output, 0);

    let global = r#"module spec.cpr.allnullglobal;

let g: str? = null;

func store(c: bool): void {
    g = c ? null : null;
}

func main(): i32 {
    store(true);
    if (g == null) {
        return 0;
    }
    return 9;
}
"#;
    let output = run_memory("allnullglobal", global);
    assert_no_asan("allnullglobal", &output);
    assert_exit("allnullglobal", &output, 0);
}

// ---------------------------------------------------------------------------
// xfowners: the cross-frame pass-through judged through the view's owners.
//
// The call site resolves what a view argument actually reads: a container's
// element loans, a carrier's recorded positions, a take parameter's frame
// storage, a fresh call result or struct literal, and a conditional or array
// of either. A foreign non-`take` source rejects with one `E10034` at the
// callee's whole out-slot store; the array and receiver spellings keep their
// `E10033`/`E10030` diagnostics.
// ---------------------------------------------------------------------------

/// The owner-resolved cross-frame pass-through rejections.
///
/// Each spelling hands the callee a view that dies with this frame while a
/// further-out slot waits to read it. The foreign-source spellings report one
/// `E10034` at the callee's whole out-slot store; the array and receiver
/// spellings keep their own `E10033`/`E10030` diagnostics. Every one emits no
/// translation unit.
#[test]
fn should_reject_the_owner_resolved_cross_frame_pass_through() {
    let element_constant = r#"module spec.cpr.xfelemconst;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    xs[1] = b.v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
    compile_rejects("xfelemconst", element_constant, "E10034", 1);

    let element_runtime = r#"module spec.cpr.xfelemrt;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(k: i32, borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    xs[1] = b.v;
    fill(xs[k], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(1, borrow out);
    return 0;
}
"#;
    compile_rejects("xfelemrt", element_runtime, "E10034", 1);

    let composite = r#"module spec.cpr.xfcomposite;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", b.v];
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
    compile_rejects("xfcomposite", composite, "E10034", 1);

    let fresh_element = r#"module spec.cpr.xffreshelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    xs[1] = mk().v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
    compile_rejects("xffreshelem", fresh_element, "E10034", 1);

    let literal_root = r#"module spec.cpr.xfliteral;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    fill(Box { v: stringOf("xy"), w: stringOf("zz") }.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
    compile_rejects("xfliteral", literal_root, "E10034", 1);

    let take_parameter = r#"module spec.cpr.xftake;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = "AA"; }

func outer(take inp: Box, borrow slot: str?): void {
    fill(inp.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    outer(b, borrow out);
    return 0;
}
"#;
    compile_rejects("xftake", take_parameter, "E10030", 1);

    let receiver_slot = r#"module spec.cpr.xfreceiver;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Slot { own: String; s: str[1]; }

func put(borrow self: Slot, p: str): void { self.s = [p]; }

func outer(borrow slot: Slot): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    slot.put(b.v);
}

func main(): i32 {
    let sl: Slot = Slot { own: stringOf("oo"), s: ["AA"] };
    outer(borrow sl);
    return 0;
}
"#;
    compile_reports("xfreceiver", receiver_slot, &["E10033", "E10030"]);
}

// ---------------------------------------------------------------------------
// xfaccept: the accepted cross-frame spellings of the owner resolution.
//
// A view through a carrier literal whose position a caller parameter backs,
// and a local view of a parameter, still name a foreign lend at the callee's
// whole out-slot store. Both report `E10034` once; the caller-backed storage
// remains readable but the store rule no longer accepts the source.
// ---------------------------------------------------------------------------

/// The caller-backed cross-frame spellings report the foreign store once.
#[test]
fn should_reject_the_caller_backed_cross_frame_spellings() {
    let carrier = r#"module spec.cpr.xfcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { v: str[1]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void {
    let h: Holder = Holder { v: [p.v] };
    fill(h.v[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("xfcarrier", carrier, "E10034", 1);

    let via_local = r#"module spec.cpr.xfvialocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void {
    let s: str = p.v;
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("xfvialocal", via_local, "E10034", 1);

    let via_alias = r#"module spec.cpr.xfviaalias;

import { String, stringOf } from std.text;

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: str, borrow slot: str?): void {
    let xs: str[2] = ["AA", p];
    for (borrow e in xs) {
        fill(e, borrow slot);
    }
}

func main(): i32 {
    let out: str? = null;
    let s: String = stringOf("xy");
    outer(s, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("xfviaalias", via_alias, "E10034", 1);
}

// ---------------------------------------------------------------------------
// xfreadonly: the conservative boundary of the cross-frame rule.
//
// The judgment reads the callee's signature, not its body: a callee that only
// reads its slot still counts as one that may write it, so a this-frame view
// passed to it with a further-out slot reports exactly one `E10030`. The
// boundary is pinned so a later precision change is a deliberate decision.
// ---------------------------------------------------------------------------

/// A further-out slot rejects even when the callee only reads it.
#[test]
fn should_keep_the_read_only_callee_boundary_of_the_cross_frame_rule() {
    let read_only = r#"module spec.cpr.xfreadonly;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func read(p: str, borrow slot: str?): i32 {
    if (slot == null) {
        return i32(p[0]);
    }
    return i32(slot[0]);
}

func outer(borrow slot: str?): i32 {
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    return read(b.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    return outer(borrow out) - 120;
}
"#;
    compile_rejects("xfreadonly", read_only, "E10030", 1);
}

// ---------------------------------------------------------------------------
// xfcopy: a copied carrier keeps the view positions it hands over.
//
// `let h2 = h1;` hands `h2` the same views `h1` holds, so the source's
// position loans follow the copy: returning the copy reports the same escape
// as returning the source, and a parameter-backed copy stays accepted.
// ---------------------------------------------------------------------------

/// A copied carrier's positions follow the copy.
#[test]
fn should_release_the_root_of_a_copied_carrier_position() {
    let frame_backed = r#"module spec.cpr.xfcopy;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func make(): Holder {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h1: Holder = Holder { names: [b.v] };
    let h2: Holder = h1;
    return h2;
}

func main(): i32 { return i32(make().names[0][0]) - 120; }
"#;
    compile_rejects("xfcopy", frame_backed, "E10030", 1);

    let caller_backed = r#"module spec.cpr.xfcopyparam;

import { String, stringOf } from std.text;

struct Holder { names: str[1]; }

func make(p: str): Holder {
    let h1: Holder = Holder { names: [p] };
    let h2: Holder = h1;
    return h2;
}

func main(): i32 {
    let s: String = stringOf("xy");
    return i32(make(s).names[0][0]) - 120;
}
"#;
    let output = run_memory("xfcopyparam", caller_backed);
    assert_no_asan("xfcopyparam", &output);
    assert_exit("xfcopyparam", &output, 0);
}

// ---------------------------------------------------------------------------
// xfarrtake: a fixed-array member over a fresh root releases the root.
//
// `holdArr(c ? b.ys : mk().ys)` copies the duplicable elements while the
// fresh arm's root still owns its other fields; the arm binds its root, so
// the residual releases at the statement end and the run is memory-clean.
// ---------------------------------------------------------------------------

/// A fixed-array take projection releases its fresh root.
#[test]
fn should_release_the_root_of_a_fixed_array_take_projection() {
    let source = r#"module spec.cpr.xfarrtake;

import { String, stringOf } from std.text;

struct Box { ys: u8[4]; v: String; }

func mk(): Box { return Box { ys: [1, 2, 3, 4], v: stringOf("ab") }; }
func holdArr(take a: u8[4]): i32 { return i32(a[0]); }

func probe(c: bool, b: Box): i32 {
    return holdArr(c ? b.ys : mk().ys);
}

func main(): i32 {
    let b: Box = Box { ys: [9, 10, 11, 12], v: stringOf("xy") };
    return probe(false, b) - 1;
}
"#;
    let output = run_memory("xfarrtake", source);
    assert_no_asan("xfarrtake", &output);
    assert_exit("xfarrtake", &output, 0);
}

// ---------------------------------------------------------------------------
// xfmoddeep: a module view of a deep or array member retains the member.
//
// `let g: str = mkO().i.s;` and `let a: str[2] = [mk().v, mk().w];` read
// members of fresh results: each root binds, the read position empties right
// after the store, and the root's remaining fields release at the
// initializer's end, so both stay readable and memory-clean.
// ---------------------------------------------------------------------------

/// A deep member and an array member of a module view release their roots.
#[test]
fn should_retain_the_members_of_a_deep_or_array_module_view() {
    let deep = r#"module spec.cpr.xfmoddeep;

import { String, stringOf } from std.text;

struct Inner { s: String; n: i32; }
struct Outer { i: Inner; t: String; }

func mkO(): Outer { return Outer { i: Inner { s: stringOf("xy"), n: 1 }, t: stringOf("zz") }; }

let g: str = mkO().i.s;

func main(): i32 {
    return i32(g[0]) - 120;
}
"#;
    let output = run_memory("xfmoddeep", deep);
    assert_no_asan("xfmoddeep", &output);
    assert_exit("xfmoddeep", &output, 0);

    let array = r#"module spec.cpr.xfmodarray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

let a: str[2] = [mk().v, mk().w];

func main(): i32 {
    return i32(a[0][0]) + i32(a[1][0]) - 242;
}
"#;
    let output = run_memory("xfmodarray", array);
    assert_no_asan("xfmodarray", &output);
    assert_exit("xfmodarray", &output, 0);
}

// ---------------------------------------------------------------------------
// xfelemowner: a view stored into an element loans its caller backing too.
//
// A view element store records the storage its view reads, and a caller
// parameter's backing is such storage: a `take` parameter dies with this
// frame, so the return, the mutation, the call, and the global spellings all
// report; a plain parameter's backing dies with the caller, so only a global
// destination, which outlives the caller, reports it.
// ---------------------------------------------------------------------------

/// The element stores of caller-backing views report per destination.
///
/// The `take`-parameter element return reports one `E10035`, its backing
/// mutation one `E10031`, the call spelling one `E10030`, and the global
/// store one `E10033`; the plain-parameter element stored into a global
/// reports one `E10033` as well, and the take-parameter carrier field
/// reports one `E10031` at its conflicting write.
#[test]
fn should_loan_the_caller_backing_of_an_element_view_store() {
    let take_return = r#"module spec.cpr.xfetakereturn;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(k: i32, take b: Box): str[2] {
    let xs: str[2] = ["AA", "BB"];
    xs[k] = b.v;
    return xs;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    return i32(run(1, b)[1][0]) - 120;
}
"#;
    compile_rejects("xfetakereturn", take_return, "E10035", 1);

    let take_mutation = r#"module spec.cpr.xfetakemut;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(k: i32, take b: Box): i32 {
    let xs: str[2] = ["AA", "BB"];
    xs[k] = b.v;
    b.v = stringOf("qq");
    return i32(xs[k][0]) - 120;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    return pick(1, b);
}
"#;
    compile_rejects("xfetakemut", take_mutation, "E10031", 1);

    let take_call = r#"module spec.cpr.xfetakecall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(take b: Box, borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    xs[1] = b.v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("xfetakecall", take_call, "E10034", 1);

    let take_global = r#"module spec.cpr.xfetakeglobal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let g: str[2] = ["AA", "BB"];

func store(take b: Box): void {
    let xs: str[2] = ["AA", "BB"];
    xs[1] = b.v;
    g = xs;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    store(b);
    return i32(g[1][0]) - 120;
}
"#;
    compile_rejects("xfetakeglobal", take_global, "E10033", 1);

    let borrow_global = r#"module spec.cpr.xfeborrowglobal;

import { String, stringOf } from std.text;

let g: str[2] = ["AA", "BB"];

func store(p: str): void {
    let xs: str[2] = ["AA", "BB"];
    xs[1] = p;
    g = xs;
}

func f(): void {
    let s: String = stringOf("xy");
    store(s);
}

func main(): i32 {
    f();
    return i32(g[1][0]) - 120;
}
"#;
    compile_rejects("xfeborrowglobal", borrow_global, "E10033", 1);

    let take_field = r#"module spec.cpr.xfetakefield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func run(take p: Box): i32 {
    let h: Holder = Holder { names: ["AA"] };
    h.names = [p.v];
    p.v = stringOf("qq");
    return i32(h.names[0][0]) - 120;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    return run(b);
}
"#;
    compile_rejects("xfetakefield", take_field, "E10031", 1);
}

// ---------------------------------------------------------------------------
// xfetakefieldret: the take-parameter carrier field return reports once.
//
// `let h = Holder { names: ["AA"] }; h.names = [p.v]; return h;` hands the
// caller a view of the take parameter, which dies with this frame: the
// carrier return reports one `E10030`.
// ---------------------------------------------------------------------------

/// A returned carrier field of a take parameter reports one `E10030`.
#[test]
fn should_reject_a_returned_take_parameter_carrier_field() {
    let source = r#"module spec.cpr.xfefieldret;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func run(take p: Box): Holder {
    let h: Holder = Holder { names: ["AA"] };
    h.names = [p.v];
    return h;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    return i32(run(b).names[0][0]) - 120;
}
"#;
    compile_rejects("xfefieldret", source, "E10030", 1);
}

// ---------------------------------------------------------------------------
// xfemodviewfield: a module view of a fresh root's view-array element.
//
// `let g: str = mkH().names[0];` reads an element of the fresh result's view
// field: the root binds, the read element empties, and the root's remaining
// field releases at the initializer's end, so the view stays readable and
// the run is memory-clean.
// ---------------------------------------------------------------------------

/// A module view of a fresh root's view element releases the root's fields.
///
/// `let g: str = mkH().names[0];` binds the fresh result, empties the read
/// element, and frees the root's remaining field at the initializer's end;
/// `let a: str[2] = mkH().names;` binds the same root for the whole-element
/// fill and frees the same field. Both stay readable and memory-clean.
#[test]
fn should_release_the_root_of_a_module_view_element() {
    let source = r#"module spec.cpr.xfemodview;

import { String, stringOf } from std.text;

struct Holder { names: str[1]; k: String; }

func mkH(): Holder { return Holder { names: ["xy"], k: stringOf("kk") }; }

let g: str = mkH().names[0];

func main(): i32 {
    return i32(g[0]) - 120;
}
"#;
    let output = run_memory("xfemodview", source);
    assert_no_asan("xfemodview", &output);
    assert_exit("xfemodview", &output, 0);

    let array = r#"module spec.cpr.xfemodviewarr;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; k: String; }

func mkH(): Holder { return Holder { names: ["xy", "zz"], k: stringOf("kk") }; }

let g: str[2] = mkH().names;

func main(): i32 {
    return i32(g[0][0]) - 120;
}
"#;
    let output = run_memory("xfemodviewarr", array);
    assert_no_asan("xfemodviewarr", &output);
    assert_exit("xfemodviewarr", &output, 0);
}

// ---------------------------------------------------------------------------
// xfetwolevel: an element copy loans the copied position's backing.
//
// `xs[0] = a.v; ys[1] = xs[0]; fill(ys[1], borrow slot);` copies an element
// that already loans `a`: the copy loans the same storage, so the call
// reports one `E10030` instead of handing the caller a freed view.
// ---------------------------------------------------------------------------

/// A copied element's loan follows the copy into its destination.
#[test]
fn should_reject_a_two_level_element_copy_into_a_further_out_slot() {
    let source = r#"module spec.cpr.xfetwolevel;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = ["CC", "DD"];
    ys[1] = xs[0];
    fill(ys[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("xfetwolevel", source, "E10034", 1);
}

// ---------------------------------------------------------------------------
// xfemixedarg: a mixed array-literal argument scans every element.
//
// `fillArr([p.v, mk().v], borrow slot)` hands the callee two views, one
// caller-backed and one read from a fresh result. The caller-side judgment
// reads the signature, not the callee's body, so it counts the fresh element
// exactly as the read-only boundary counts its slot: the call reports one
// `E10030`. The two-place twin `fillArr([p.v, q.v], borrow slot)` names no
// fresh storage and stays accepted.
// ---------------------------------------------------------------------------

/// A mixed array-literal argument reports one `E10030`, the places twin is
/// accepted.
#[test]
fn should_reject_a_fresh_element_of_a_mixed_array_literal_argument() {
    let fresh = r#"module spec.cpr.xfemixedarg;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fillArr(xs: str[2], borrow slot: str?): void { slot = xs[1]; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

func outer(p: Box, borrow slot: str?): void {
    fillArr([p.v, mk().v], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("xfemixedarg", fresh, "E10034", 1);

    let places = r#"module spec.cpr.xfemixedplaces;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fillArr(xs: str[2], borrow slot: str?): void { slot = xs[0]; }

func outer(p: Box, q: Box, borrow slot: str?): void {
    fillArr([p.v, q.v], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let d: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    outer(b, d, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("xfemixedplaces", places, "E10034", 1);
}

// ---------------------------------------------------------------------------
// xfecondmix: a conditional of a place and a fresh view in a call argument.
//
// `fill(c ? p.v : freshText(), borrow slot)` hands the callee a view that is
// either the caller's parameter or a fresh call result. The caller-side
// judgment reads the signature, not the callee's body, so the fresh arm
// counts in either arm order: each spelling reports one `E10030`. The
// two-place twin `fill(c ? p.v : q.v, borrow slot)` names no fresh storage
// and stays accepted.
// ---------------------------------------------------------------------------

/// A mixed place/fresh conditional argument reports once in either order; the
/// places twin keeps its acceptance.
///
/// The fresh arm's result dies with the statement while the slot it may reach
/// lives in the caller's caller, so both `c ? p.v : freshText()` and
/// `c ? freshText() : p.v` report exactly one `E10030`. The twin
/// `c ? p.v : q.v` reads two caller-backed parameters and accepts, exiting 0
/// with the written view readable and no memory report.
#[test]
fn should_reject_a_mixed_place_and_fresh_conditional_argument() {
    let fresh_then = r#"module spec.cpr.xfecondfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func freshText(): String { return stringOf("z"); }

func outer(c: bool, p: Box, borrow slot: str?): void {
    fill(c ? p.v : freshText(), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("ww") };
    outer(false, b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 122;
}
"#;
    compile_rejects("xfecondfresh", fresh_then, "E10034", 1);

    let fresh_else = r#"module spec.cpr.xfecondfreshelse;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func freshText(): String { return stringOf("z"); }

func outer(c: bool, p: Box, borrow slot: str?): void {
    fill(c ? freshText() : p.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("ww") };
    outer(true, b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 122;
}
"#;
    compile_rejects("xfecondfreshelse", fresh_else, "E10034", 1);

    let places = r#"module spec.cpr.xfecondplaces;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, p: Box, q: Box, borrow slot: str?): void {
    fill(c ? p.v : q.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("ww") };
    let d: Box = Box { v: stringOf("pq"), w: stringOf("rr") };
    outer(true, b, d, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("xfecondplaces", places, "E10034", 1);
}

// ---------------------------------------------------------------------------
// guardcallroot: a guarded call whose view element wraps a fresh root.
//
// `g ? use(Holder { names: [id(mk().v)] }) : 0` must evaluate the call only
// where the guard holds. The element argument reads a member of the fresh
// `mk()` result while that root still owns its sibling field, so the taken
// path must bind the root for the residual's release and the skipped path
// must run nothing. The direct, unguarded element position already binds its
// root per evaluation and is the control.
// ---------------------------------------------------------------------------

/// A guarded call's fresh element root releases on the taken path only.
///
/// `use(Holder { names: [id(mk().v)] })` runs once per taken evaluation: the
/// call-wrapped element's fresh root binds for the release of its remaining
/// field, and the skipped path runs nothing. The `calls` counter and the
/// call's lengths are folded into the exit code (three taken evaluations
/// each contribute a length of 2 and one call; the skipped run adds none),
/// so a call running on the skipped path or a dropped value fails the run.
/// The direct element position stays accepted and memory-clean.
#[test]
fn should_release_a_guarded_call_wrapped_element_root_on_the_taken_path() {
    let guarded = r#"module spec.cpr.guardcallroot;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func id(x: str): str { return x; }

func use(h: Holder): i32 { return i32(len(h.names[0])); }

func run(g: bool, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        n = n + (g ? use(Holder { names: [id(mk().v)] }) : 0);
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    let a: i32 = run(true, 3);
    let taken_calls: i32 = calls;
    let b: i32 = run(false, 3);
    let all_calls: i32 = calls;
    return a + b + (taken_calls - 3) * 10 + (all_calls - 3) * 10 - 6;
}
"#;
    let output = run_memory("guardcallroot", guarded);
    assert_no_asan("guardcallroot", &output);
    assert_exit("guardcallroot", &output, 0);

    let direct = r#"module spec.cpr.guardcalldirect;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func id(x: str): str { return x; }

func use(h: Holder): i32 { return i32(len(h.names[0])); }

func run(k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        n = n + use(Holder { names: [id(mk().v)] });
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    let a: i32 = run(3);
    let c: i32 = calls;
    return a + (c - 3) - 6;
}
"#;
    let output = run_memory("guardcalldirect", direct);
    assert_no_asan("guardcalldirect", &output);
    assert_exit("guardcalldirect", &output, 0);
}

// ---------------------------------------------------------------------------
// carrierliteral: a conditional carrier literal's taken arm.
//
// `let h: Holder = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };`
// carries a view of a fresh `mk()` member on the taken arm. The root still
// owns its sibling field, so the local's frame must bind the root for the
// release while the carried view stays valid; the skipped literal arm names
// no fresh storage. The loop-condition and guarded-literal positions of the
// same fresh element already bind their roots per evaluation and are the
// controls.
// ---------------------------------------------------------------------------

/// The taken arm of a conditional carrier literal binds its fresh element
/// root.
///
/// `g` selects which element the local reads: the taken arm carries a view
/// of `mk()`'s member, whose root's remaining field releases once for the
/// local's frame, while the skipped arm keeps its own literal. Each
/// selection reads its first byte back (`a` on the taken path, `z` on the
/// skipped), so a wrong arm or a dropped value fails the run. The
/// loop-condition and guarded-literal spellings keep their acceptance,
/// release their roots per evaluation, and stay memory-clean.
#[test]
fn should_release_the_taken_arm_root_of_a_conditional_carrier_literal() {
    let carrier = r#"module spec.cpr.carrierliteral;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func run(g: bool, want: i32): i32 {
    let h: Holder = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };
    return i32(h.names[0][0]) - want;
}

func main(): i32 {
    return run(true, 97) + run(false, 122);
}
"#;
    let output = run_memory("carrierliteral", carrier);
    assert_no_asan("carrierliteral", &output);
    assert_exit("carrierliteral", &output, 0);

    let loop_control = r#"module spec.cpr.carrierloopcond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(len(h.names[0])); }

func run(c: bool, k: i32): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < k && (c ? use(Holder { names: [mk().v] }) : 0) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    let a: i32 = run(true, 3);
    let taken_calls: i32 = calls;
    let b: i32 = run(false, 3);
    let all_calls: i32 = calls;
    return a + b + (taken_calls - 3) * 10 + (all_calls - 3) * 10 - 3;
}
"#;
    let output = run_memory("carrierloopcond", loop_control);
    assert_no_asan("carrierloopcond", &output);
    assert_exit("carrierloopcond", &output, 0);

    let guard_control = r#"module spec.cpr.carrierguardlit;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func use(h: Holder): i32 { return i32(len(h.names[0])); }

func run(g: bool): i32 {
    if (g && use(Holder { names: [mk().v] }) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = run(true);
    let taken_calls: i32 = calls;
    let b: i32 = run(false);
    let all_calls: i32 = calls;
    return a + b + (taken_calls - 1) * 10 + (all_calls - 1) * 10 - 1;
}
"#;
    let output = run_memory("carrierguardlit", guard_control);
    assert_no_asan("carrierguardlit", &output);
    assert_exit("carrierguardlit", &output, 0);
}

// ---------------------------------------------------------------------------
// discardresidual: a discarded non-owning arm over a residual-owning root.
//
// `c ? i32(mk().v.len()) : t;` selects an `i32` that owns nothing, so the
// discard gate never binds the fresh `mk()` root even though the root still
// owns its sibling field: each taken evaluation leaks that field. The arm's
// release need is judged by the root's residual ownership, not by the
// selected type. A sole-field root needs no release and stays clean, and the
// discarded conditional with an owned selected type already releases its
// root, so both are the controls.
// ---------------------------------------------------------------------------

/// A discarded arm releases the residual of a non-owning root.
///
/// `c ? i32(mk().v.len()) : t;` reads the fresh member's length and discards
/// the value; the fresh root's remaining `w` field releases when the
/// statement ends, while the place arm keeps `t` readable (the read-back
/// subtracts 5). The sole-field twin needs no release, and the discarded
/// conditional selecting an owned `Box` binds its root and frees it once;
/// both controls stay clean and readable.
#[test]
fn should_release_the_residual_of_a_discarded_non_owning_arm() {
    let residual = r#"module spec.cpr.discardresidual;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool, t: i32): i32 {
    c ? i32(mk().v.len()) : t;
    return t - 5;
}

func main(): i32 {
    return use(true, 5) + use(false, 5);
}
"#;
    let output = run_memory("discardresidual", residual);
    assert_no_asan("discardresidual", &output);
    assert_exit("discardresidual", &output, 0);

    let sole_control = r#"module spec.cpr.discardsole;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("ab") }; }

func use(c: bool, t: i32): i32 {
    c ? i32(mk().v.len()) : t;
    return t - 5;
}

func main(): i32 {
    return use(true, 5) + use(false, 5);
}
"#;
    let output = run_memory("discardsole", sole_control);
    assert_no_asan("discardsole", &output);
    assert_exit("discardsole", &output, 0);

    let owned_control = r#"module spec.cpr.discardowned;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool, b: Box): i32 {
    c ? mk() : b;
    return i32(len(b.v)) - 2;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    return use(true, b) + use(false, b);
}
"#;
    let output = run_memory("discardowned", owned_control);
    assert_no_asan("discardowned", &output);
    assert_exit("discardowned", &output, 0);
}

// ---------------------------------------------------------------------------
// barediscard: a bare discarded all-fresh literal statement.
//
// `[mk().v];` as a bare statement materializes nothing, so the fresh
// element's storage and the root's residual never release and the emitted
// discard is not valid C. The discarded-literal materialization must reach
// the bare statement spelling: each fresh owning leaf binds to its own
// released statement temporary on every execution, while a place leaf stays
// a read. The conditional discard spelling with its place fields already
// does this and is the control.
// ---------------------------------------------------------------------------

/// A bare discarded all-fresh literal releases its fresh leaves.
///
/// `[mk().v];` and the looped `[mk().w];` drop a literal whose element reads
/// a fresh root's member; each fresh leaf binds to its own statement
/// temporary and releases when the statement ends, once per execution, so
/// the run exits 0 with no memory report. The conditional discard spelling
/// keeps its per-field materialization: the fresh fields release while the
/// place fields `t` and `u` stay readable and stay reads.
#[test]
fn should_materialize_a_bare_discarded_literal_statement() {
    let bare = r#"module spec.cpr.barediscard;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("hello"), w: stringOf("world") }; }

func use(): i32 {
    [mk().v];
    return 0;
}

func looped(k: i32): i32 {
    let i: i32 = 0;
    while (i < k) {
        [mk().w];
        i = i + 1;
    }
    return i - k;
}

func main(): i32 {
    return use() + looped(3);
}
"#;
    let output = run_memory("barediscard", bare);
    assert_no_asan("barediscard", &output);
    assert_exit("barediscard", &output, 0);

    let conditional_control = r#"module spec.cpr.barediscardcond;

import { String, stringOf } from std.text;

struct Pair { a: String; b: String; }

func literal(ok: bool): i32 {
    let t: String = stringOf("tt");
    let u: String = stringOf("uu");
    ok ? Pair { a: stringOf("x"), b: t } : Pair { a: stringOf("y"), b: u };
    return i32(len(t)) + i32(len(u)) - 4;
}

func main(): i32 {
    return literal(true) + literal(false);
}
"#;
    let output = run_memory("barediscardcond", conditional_control);
    assert_no_asan("barediscardcond", &output);
    assert_exit("barediscardcond", &output, 0);
}

// ---------------------------------------------------------------------------
// discardcalls: a bare discarded literal runs every leaf's effect.
//
// A literal statement drops its value but must still evaluate what it spells:
// `Pair { a: counted(), b: counted() };` counts both fields, an array-literal
// field counts its elements, a scalar array literal counts its elements, and
// an owning field releases the storage it created while a place leaf stays a
// read.
// ---------------------------------------------------------------------------

/// A bare discarded literal runs every leaf's effect exactly once.
#[test]
fn should_run_every_leaf_of_a_bare_discarded_literal() {
    let struct_calls = r#"module spec.cpr.discardcalls;

struct Pair { a: i32; b: i32; }

let calls: i32 = 0;

func counted(): i32 {
    calls = calls + 1;
    return 1;
}

func use(): i32 {
    Pair { a: counted(), b: counted() };
    return calls - 2;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("discardcalls", struct_calls);
    assert_no_asan("discardcalls", &output);
    assert_exit("discardcalls", &output, 0);

    let array_field = r#"module spec.cpr.discardcallsfield;

struct Holder { row: i32[2]; }

let calls: i32 = 0;

func counted(): i32 {
    calls = calls + 1;
    return 1;
}

func use(): i32 {
    Holder { row: [counted(), counted()] };
    return calls - 2;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("discardcallsfield", array_field);
    assert_no_asan("discardcallsfield", &output);
    assert_exit("discardcallsfield", &output, 0);

    let scalar_array = r#"module spec.cpr.discardcallsarr;

let calls: i32 = 0;

func counted(): i32 {
    calls = calls + 1;
    return 1;
}

func use(): i32 {
    [counted(), counted()];
    return calls - 2;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("discardcallsarr", scalar_array);
    assert_no_asan("discardcallsarr", &output);
    assert_exit("discardcallsarr", &output, 0);

    let owning = r#"module spec.cpr.discardcallsowning;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let calls: i32 = 0;

func countedText(): String {
    calls = calls + 1;
    return stringOf("a");
}

func use(): i32 {
    Box { v: countedText(), w: countedText() };
    return calls - 2;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("discardcallsowning", owning);
    assert_no_asan("discardcallsowning", &output);
    assert_exit("discardcallsowning", &output, 0);
}

// ---------------------------------------------------------------------------
// keptcarrier: a conditional carrier keeps a view of a fresh result.
//
// `let h: Holder = g ? Holder { names: [id(mk().v)] } : Holder { names: ["zz"] };`
// hands the local a view of a fresh result through a view-returning call. The
// taken arm's root binds to the local's frame, so the byte stays readable
// after the statement and the root's remaining field releases at the frame's
// end; the skipped arm keeps its own literal. Both selections read back.
// ---------------------------------------------------------------------------

/// A kept conditional carrier's view-returning call reads live storage.
#[test]
fn should_keep_a_conditional_carriers_view_returning_call_readable() {
    let source = r#"module spec.cpr.keptcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func id(x: str): str { return x; }

func run(g: bool, want: i32): i32 {
    let h: Holder = g ? Holder { names: [id(mk().v)] } : Holder { names: ["zz"] };
    return i32(h.names[0][0]) - want;
}

func main(): i32 {
    return run(true, 97) + run(false, 122);
}
"#;
    let output = run_memory("keptcarrier", source);
    assert_no_asan("keptcarrier", &output);
    assert_exit("keptcarrier", &output, 0);
}

// ---------------------------------------------------------------------------
// globcarrier: a module carrier cannot keep a fresh view.
//
// `glob = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };` stores a
// view of a fresh result into a module binding, which outlives the frame that
// view dies with: the store reports one `E10033`. A module store whose arms
// name static storage keeps its acceptance and reads back.
// ---------------------------------------------------------------------------

/// A module carrier store of a fresh view reports once; the static twin is
/// accepted.
#[test]
fn should_reject_a_module_carrier_store_of_a_fresh_view() {
    let fresh = r#"module spec.cpr.globcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

let glob: Holder = Holder { names: ["aa"] };

func run(g: bool): i32 {
    glob = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };
    return 0;
}

func main(): i32 { run(true); return i32(glob.names[0][0]) - 97; }
"#;
    compile_rejects("globcarrier", fresh, "E10033", 1);

    let static_twin = r#"module spec.cpr.globcarrierstatic;

struct Holder { names: str[1]; }

let glob: Holder = Holder { names: ["aa"] };

func run(g: bool): i32 {
    glob = g ? Holder { names: ["bb"] } : Holder { names: ["cc"] };
    return 0;
}

func main(): i32 {
    run(true);
    run(false);
    return i32(glob.names[0][0]) - 99;
}
"#;
    let output = run_memory("globcarrierstatic", static_twin);
    assert_no_asan("globcarrierstatic", &output);
    assert_exit("globcarrierstatic", &output, 0);
}

// ---------------------------------------------------------------------------
// mixeddiscard: a mixed view/place bare discard releases the fresh root.
//
// `[u, mk().v];` drops a view array whose second element reads a fresh
// result's member: the root's remaining field releases when the statement
// ends, once per execution, while the place element stays a read.
// ---------------------------------------------------------------------------

/// A mixed view/place bare discard releases its fresh root per execution.
#[test]
fn should_release_the_root_of_a_mixed_bare_discard() {
    let source = r#"module spec.cpr.mixeddiscard;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(u: str, k: i32): i32 {
    let i: i32 = 0;
    while (i < k) {
        [u, mk().v];
        i = i + 1;
    }
    return i - k + i32(len(u)) - 2;
}

func main(): i32 { return use("uu", 3); }
"#;
    let output = run_memory("mixeddiscard", source);
    assert_no_asan("mixeddiscard", &output);
    assert_exit("mixeddiscard", &output, 0);
}

// ---------------------------------------------------------------------------
// condcarrier: a conditional view element inside a carrier literal.
//
// `Holder { names: [c ? id(mk().v) : id(mk().w)] }` stores a view that reads
// a member of a fresh result on either selection. The element-conditional
// inside a conditional carrier, its one-arm spelling, the re-executed loop
// spelling, the nested conditional, and the deferred fill
// `[; c ? id(mk().v) : "zz"]` each bind the taken arm's root for the local's
// frame, so the stored byte stays readable while the roots' remaining fields
// release. Every selection folds its byte into the exit code.
// ---------------------------------------------------------------------------

/// A conditional element inside a carrier literal keeps its byte readable.
#[test]
fn should_keep_the_conditional_element_of_a_carrier_literal_readable() {
    let direct = r#"module spec.cpr.condcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func id(x: str): str { return x; }

func run(c: bool): i32 {
    let h: Holder = Holder { names: [c ? id(mk().v) : id(mk().w)] };
    return i32(h.names[0][0]) - (c ? 97 : 99);
}

func main(): i32 {
    return run(true) + run(false);
}
"#;
    let output = run_memory("condcarrier", direct);
    assert_no_asan("condcarrier", &output);
    assert_exit("condcarrier", &output, 0);

    let one_arm = r#"module spec.cpr.condcarrierone;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func id(x: str): str { return x; }

func run(c: bool): i32 {
    let h: Holder = Holder { names: [c ? id(mk().v) : "zz"] };
    return i32(h.names[0][0]) - (c ? 97 : 122);
}

func main(): i32 {
    return run(true) + run(false);
}
"#;
    let output = run_memory("condcarrierone", one_arm);
    assert_no_asan("condcarrierone", &output);
    assert_exit("condcarrierone", &output, 0);

    let looped = r#"module spec.cpr.condcarrierloop;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func id(x: str): str { return x; }

func run(g: bool, c: bool, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        let h: Holder = g ? Holder { names: [c ? id(mk().v) : id(mk().w)] } : Holder { names: ["zz"] };
        n = n + i32(h.names[0][0]) - (g ? (c ? 97 : 99) : 122);
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return run(true, true, 3) + run(true, false, 3) + run(false, false, 3);
}
"#;
    let output = run_memory("condcarrierloop", looped);
    assert_no_asan("condcarrierloop", &output);
    assert_exit("condcarrierloop", &output, 0);

    let nested = r#"module spec.cpr.condcarriernested;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func id(x: str): str { return x; }

func run(g: bool, c: bool, want: i32): i32 {
    let h: Holder = g ? Holder { names: [c ? id(mk().v) : id(mk().w)] } : Holder { names: ["zz"] };
    return i32(h.names[0][0]) - want;
}

func main(): i32 {
    return run(true, true, 97) + run(true, false, 99) + run(false, false, 122);
}
"#;
    let output = run_memory("condcarriernested", nested);
    assert_no_asan("condcarriernested", &output);
    assert_exit("condcarriernested", &output, 0);

    let fill = r#"module spec.cpr.condcarrierfill;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[2]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func id(x: str): str { return x; }

func run(c: bool): i32 {
    let h: Holder = Holder { names: [; c ? id(mk().v) : "zz"] };
    return i32(h.names[0][0]) - (c ? 97 : 122);
}

func main(): i32 {
    return run(true) + run(false);
}
"#;
    let output = run_memory("condcarrierfill", fill);
    assert_no_asan("condcarrierfill", &output);
    assert_exit("condcarrierfill", &output, 0);
}

// ---------------------------------------------------------------------------
// condfield: a conditional value in a fixed-array literal field.
//
// A struct literal whose field is a fixed array cannot name a conditional
// value in a C initializer, because neither the selected field nor a call
// has an initializer form. The selected value must fill the literal's own
// field per arm, on that arm's path: the plain spelling stores the literal
// into a local, and the guarded spelling builds the same literal inside a
// `&&` operand, so only the taken path evaluates it. The direct field
// spellings (a place field and a call-filled field) stay accepted controls.
// ---------------------------------------------------------------------------

/// A literal's fixed-array field fills per arm from a conditional value.
///
/// `Holder { row: c ? a.row : mkBox().row }` copies the selected `Box`
/// field's rows into the holder: `a`'s rows on the true selection and the
/// fresh `mkBox()`'s rows on the false one, so each program reads four
/// bytes and exits 0. The guarded spelling builds the same literal behind
/// `g && ...` and evaluates it on the taken path only. The direct field
/// spellings keep their acceptance and read back.
#[test]
fn should_fill_a_conditional_fixed_array_field_per_arm() {
    let plain = r#"module spec.cpr.condfieldplain;

import { String, stringOf } from std.text;

struct Box { row: String[2]; }
struct Holder { row: String[2]; tag: i32; }

func mkBox(): Box { return Box { row: [stringOf("ab"), stringOf("cd")] }; }

func use(c: bool, take a: Box): i32 {
    let h: Holder = Holder { row: c ? a.row : mkBox().row, tag: 2 };
    return h.tag - 2 + i32(len(h.row[0])) + i32(len(h.row[1])) - 4;
}

func main(): i32 {
    let a1: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let n: i32 = use(true, a1);
    let a2: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    return n + use(false, a2);
}
"#;
    let output = run_memory("condfieldplain", plain);
    assert_no_asan("condfieldplain", &output);
    assert_exit("condfieldplain", &output, 0);

    let guarded = r#"module spec.cpr.condfieldguard;

import { String, stringOf } from std.text;

struct Box { row: String[2]; }
struct Holder { row: String[2]; tag: i32; }

func mkBox(): Box { return Box { row: [stringOf("ab"), stringOf("cd")] }; }

func use(c: bool, take a: Box, g: bool): i32 {
    if (g && i32(len(Holder { row: c ? a.row : mkBox().row, tag: 1 }.row[0])) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a1: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let n: i32 = use(true, a1, true);
    let a2: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let m: i32 = use(false, a2, true);
    let a3: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let s: i32 = use(true, a3, false);
    return n + m + s - 2;
}
"#;
    let output = run_memory("condfieldguard", guarded);
    assert_no_asan("condfieldguard", &output);
    assert_exit("condfieldguard", &output, 0);

    let direct = r#"module spec.cpr.condfielddirect;

import { String, stringOf } from std.text;

struct Box { row: String[2]; }
struct Holder { row: String[2]; tag: i32; }

func mkBox(): Box { return Box { row: [stringOf("ab"), stringOf("cd")] }; }

func use(take a: Box): i32 {
    let h: Holder = Holder { row: a.row, tag: 2 };
    let k: Holder = Holder { row: mkBox().row, tag: 3 };
    return h.tag - 2 + k.tag - 3 + i32(len(h.row[0])) + i32(len(k.row[1])) - 4;
}

func main(): i32 {
    let a: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    return use(a);
}
"#;
    let output = run_memory("condfielddirect", direct);
    assert_no_asan("condfielddirect", &output);
    assert_exit("condfielddirect", &output, 0);
}

// ---------------------------------------------------------------------------
// condrootfield: a conditional root projected into a fixed-array literal
// field.
//
// `Holder { row: (c ? a : b).row }` selects the root first and projects its
// fixed-array field into the literal's field. A conditional is not an
// lvalue, so the initializer cannot address a member of the conditional
// value: each arm must contribute its own root's field on its path, the
// same per-arm copy the store and return spellings already use. The store
// spelling of the same projection is the accepted control.
// ---------------------------------------------------------------------------

/// A conditional root's field copies into a literal field per arm.
///
/// `Holder { row: (c ? a : b).row, tag: 4 }` copies `a`'s rows on the true
/// selection and `b`'s on the false one, so both selections read four bytes
/// and exit 0; the tag proves the whole literal was built. The store
/// spelling `let r: String[2] = (c ? a : b).row;` keeps its acceptance.
#[test]
fn should_copy_a_conditional_root_into_a_literal_fixed_array_field() {
    let field = r#"module spec.cpr.condrootfield;

import { String, stringOf } from std.text;

struct Box { row: String[2]; }
struct Holder { row: String[2]; tag: i32; }

func use(c: bool, take a: Box, take b: Box): i32 {
    let h: Holder = Holder { row: (c ? a : b).row, tag: 4 };
    return h.tag - 4 + i32(len(h.row[0])) + i32(len(h.row[1])) - 4;
}

func main(): i32 {
    let a1: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let b1: Box = Box { row: [stringOf("pp"), stringOf("qq")] };
    let n: i32 = use(true, a1, b1);
    let a2: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let b2: Box = Box { row: [stringOf("pp"), stringOf("qq")] };
    return n + use(false, a2, b2);
}
"#;
    let output = run_memory("condrootfield", field);
    assert_no_asan("condrootfield", &output);
    assert_exit("condrootfield", &output, 0);

    let stored = r#"module spec.cpr.condrootfieldstore;

import { String, stringOf } from std.text;

struct Box { row: String[2]; }

func use(c: bool, take a: Box, take b: Box): i32 {
    let r: String[2] = (c ? a : b).row;
    return i32(len(r[0])) + i32(len(r[1])) - 4;
}

func main(): i32 {
    let a1: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let b1: Box = Box { row: [stringOf("pp"), stringOf("qq")] };
    let n: i32 = use(true, a1, b1);
    let a2: Box = Box { row: [stringOf("xx"), stringOf("yy")] };
    let b2: Box = Box { row: [stringOf("pp"), stringOf("qq")] };
    return n + use(false, a2, b2);
}
"#;
    let output = run_memory("condrootfieldstore", stored);
    assert_no_asan("condrootfieldstore", &output);
    assert_exit("condrootfieldstore", &output, 0);
}

// ---------------------------------------------------------------------------
// guardviewfield: a literal view-array field inside a guarded operand.
//
// `g && len(Holder { names: [mk().v] }.names[0]) > 0` builds a literal whose
// view-array field holds a view of a fresh result. The field must be
// initialized completely (every element, including the array's full
// extent), because a partially initialized view array is rejected by the
// strict build, and the fresh root must release once per taken evaluation.
// The unguarded spelling of the same literal is the accepted control.
// ---------------------------------------------------------------------------

/// A guarded literal's view-array field initializes completely.
///
/// `Holder { names: [mk().v] }` fills its one-element view array with a view
/// of the fresh `mk()` result; the guarded spelling evaluates the literal
/// only when `g` holds, so the taken path returns 1 and the skipped path 0.
/// The unguarded spelling builds the local and reads the view's length back.
#[test]
fn should_fill_a_guarded_literal_view_array_field_completely() {
    let guarded = r#"module spec.cpr.guardviewfield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(g: bool): i32 {
    if (g && i32(len(Holder { names: [mk().v] }.names[0])) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return use(true) - 1 + use(false);
}
"#;
    let output = run_memory("guardviewfield", guarded);
    assert_no_asan("guardviewfield", &output);
    assert_exit("guardviewfield", &output, 0);

    let plain = r#"module spec.cpr.guardviewfieldplain;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(): i32 {
    let h: Holder = Holder { names: [mk().v] };
    return i32(len(h.names[0])) - 2;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("guardviewfieldplain", plain);
    assert_no_asan("guardviewfieldplain", &output);
    assert_exit("guardviewfieldplain", &output, 0);
}

// ---------------------------------------------------------------------------
// condcallliteral: a conditional of call-filled fixed-array literals.
//
// `ok ? Mixed { names: n1(), tag: stringOf("x") } : Mixed { names: n2(), ... }`
// selects one literal whose fixed-array field is filled by a call. The call
// writes through the field's hidden destination, so each arm's literal must
// build with its own field store and the conditional's destination must
// fill on the selected path. The single-literal spelling of the same
// call-filled field is the accepted control.
// ---------------------------------------------------------------------------

/// A conditional of call-filled literals runs each arm's call once on its
/// path.
///
/// The selected `Mixed` literal fills `names` with the selected arm's call
/// (`n1` on true, `n2` on false); the counters prove each call ran exactly
/// once across the two selections and the fields were built. The discarded
/// single-literal spelling fills its call-filled field through the same
/// destination ABI and stays accepted.
#[test]
fn should_run_each_arm_of_a_conditional_of_call_filled_literals() {
    let conditional = r#"module spec.cpr.condcallliteral;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let left: i32 = 0;
let right: i32 = 0;

func n1(): str[2] {
    left = left + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    right = right + 1;
    return ["CC", "DD"];
}

func use(ok: bool): i32 {
    ok ? Mixed { names: n1(), tag: stringOf("x") } : Mixed { names: n2(), tag: stringOf("y") };
    return 0;
}

func main(): i32 {
    let n: i32 = use(true) + use(false);
    return n + left * 10 + right - 11;
}
"#;
    let output = run_memory("condcallliteral", conditional);
    assert_no_asan("condcallliteral", &output);
    assert_exit("condcallliteral", &output, 0);

    let single = r#"module spec.cpr.condcallliteralsingle;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let calls: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func use(): i32 {
    Mixed { names: n1(), tag: stringOf("x") };
    return calls - 1;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("condcallliteralsingle", single);
    assert_no_asan("condcallliteralsingle", &output);
    assert_exit("condcallliteralsingle", &output, 0);
}

// ---------------------------------------------------------------------------
// condrowreturn: a `String[2]` conditional-root return by value.
//
// `return (c ? b1 : b2).xs;` selects one root and returns its fixed-array
// field by value through the hidden destination parameter. A `?:` cannot
// pass the hidden out-parameter a fixed-array field needs, so each arm must
// copy into the destination on its own path. The caller reads the returned
// rows' bytes, so a dropped or wrong copy is visible as a wrong value or a
// crash. The store spelling and the direct conditional-free return are the
// accepted controls.
// ---------------------------------------------------------------------------

/// A `String[2]` conditional-root return copies per arm into the caller.
///
/// `pick(true)` returns `b1`'s rows and `pick(false)` returns `b2`'s; the
/// caller reads the first byte of each returned row (`a`/`b`, `c`/`d`) and
/// cancels them with the byte sum, so the run exits 0 only when both
/// selections copied the right bytes. The store spelling moves the same
/// rows into a local and the direct return copies a single root's field;
/// both controls exit 0 and run clean.
#[test]
fn should_copy_a_string_array_conditional_root_return_per_arm() {
    let returned = r#"module spec.cpr.condrowreturn;

import { String, stringOf, bytes } from std.text;

struct Box { xs: String[2]; }

func pick(c: bool): String[2] {
    let b1: Box = Box { xs: [stringOf("aa"), stringOf("bb")] };
    let b2: Box = Box { xs: [stringOf("cc"), stringOf("dd")] };
    return (c ? b1 : b2).xs;
}

func main(): i32 {
    let a: String[2] = pick(true);
    let b: String[2] = pick(false);
    let s: i32 = i32(bytes(a[0])[0]) + i32(bytes(a[1])[1]);
    s = s + i32(bytes(b[0])[0]) + i32(bytes(b[1])[1]);
    return s - (97 + 98 + 99 + 100);
}
"#;
    let output = run_memory("condrowreturn", returned);
    assert_no_asan("condrowreturn", &output);
    assert_exit("condrowreturn", &output, 0);

    let stored = r#"module spec.cpr.condrowreturnstore;

import { String, stringOf, bytes } from std.text;

struct Box { xs: String[2]; }

func use(c: bool): i32 {
    let b1: Box = Box { xs: [stringOf("aa"), stringOf("bb")] };
    let b2: Box = Box { xs: [stringOf("cc"), stringOf("dd")] };
    let a: String[2] = (c ? b1 : b2).xs;
    let s: i32 = i32(bytes(a[0])[0]) + i32(bytes(a[1])[1]);
    return s - (c ? (97 + 98) : (99 + 100));
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    let output = run_memory("condrowreturnstore", stored);
    assert_no_asan("condrowreturnstore", &output);
    assert_exit("condrowreturnstore", &output, 0);

    let direct = r#"module spec.cpr.condrowreturndirect;

import { String, stringOf, bytes } from std.text;

struct Box { xs: String[2]; }

func pick(c: bool): String[2] {
    let b: Box = Box { xs: [stringOf("aa"), stringOf("bb")] };
    if (c) {
        return b.xs;
    }
    let d: Box = Box { xs: [stringOf("cc"), stringOf("dd")] };
    return d.xs;
}

func main(): i32 {
    let a: String[2] = pick(true);
    let b: String[2] = pick(false);
    let s: i32 = i32(bytes(a[0])[0]) + i32(bytes(a[1])[1]);
    s = s + i32(bytes(b[0])[0]) + i32(bytes(b[1])[1]);
    return s - (97 + 98 + 99 + 100);
}
"#;
    let output = run_memory("condrowreturndirect", direct);
    assert_no_asan("condrowreturndirect", &output);
    assert_exit("condrowreturndirect", &output, 0);
}

// ---------------------------------------------------------------------------
// guardfallback: a fixed-array constructor behind a short-circuit operand.
//
// `g && consume((c ? b1 : b2).names)` and `g && consume(ok ? mk1() : mk2())`
// pass a fixed-array conditional to a `take` parameter. The argument has no
// C value expression — the projection is not an addressable member of a
// conditional value, and a call arm cannot pass its hidden out-parameter
// through `?:` — so the operand must route through the per-arm destination
// store, exactly like the statement path, while `g` keeps the operand lazy:
// a skipped path runs no call and consumes nothing. The unguarded call
// spellings of the same arguments are the accepted controls.
// ---------------------------------------------------------------------------

/// A guarded fixed-array consume fills its destination per arm and stays
/// lazy.
///
/// The projection spelling consumes the selected root's rows (the bytes
/// prove the value arrived), and the call spelling consumes the selected
/// arm's fresh rows; both selections read back and exit 0. The call
/// counters prove each call ran once on its path and the skipped evaluation
/// ran nothing. The unguarded spellings keep their acceptance.
#[test]
fn should_route_a_guarded_fixed_array_consume_through_its_destination() {
    let projection = r#"module spec.cpr.guardfallbackproj;

import { String, stringOf, bytes } from std.text;

struct Box { names: String[2]; }

let calls: i32 = 0;

func consume(take names: String[2]): i32 {
    calls = calls + 1;
    return i32(bytes(names[0])[0]) + i32(bytes(names[1])[1]);
}

func use(g: bool, c: bool, want: i32): i32 {
    let b1: Box = Box { names: [stringOf("xa"), stringOf("yb")] };
    let b2: Box = Box { names: [stringOf("pc"), stringOf("qd")] };
    if (g && consume((c ? b1 : b2).names) == want) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let n: i32 = use(true, true, 120 + 98) - 1;
    n = n + use(true, false, 112 + 100) - 1;
    n = n + use(false, true, 120 + 98);
    return n + (calls - 2) * 10;
}
"#;
    let output = run_memory("guardfallbackproj", projection);
    assert_no_asan("guardfallbackproj", &output);
    assert_exit("guardfallbackproj", &output, 0);

    let calls = r#"module spec.cpr.guardfallbackcond;

import { String, stringOf, bytes } from std.text;

let calls: i32 = 0;

func mk1(): String[2] {
    calls = calls + 1;
    return [stringOf("AA"), stringOf("BB")];
}

func mk2(): String[2] {
    calls = calls + 1;
    return [stringOf("CC"), stringOf("DD")];
}

func consume(take names: String[2]): i32 {
    return i32(bytes(names[0])[0]) + i32(bytes(names[1])[1]);
}

func use(g: bool, ok: bool, want: i32): i32 {
    if (g && consume(ok ? mk1() : mk2()) == want) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let n: i32 = use(true, true, 65 + 66) - 1;
    n = n + use(true, false, 67 + 68) - 1;
    n = n + use(false, true, 65 + 66);
    return n + (calls - 2) * 10;
}
"#;
    let output = run_memory("guardfallbackcond", calls);
    assert_no_asan("guardfallbackcond", &output);
    assert_exit("guardfallbackcond", &output, 0);

    let control = r#"module spec.cpr.guardfallbackctrlproj;

import { String, stringOf, bytes } from std.text;

struct Box { names: String[2]; }

let calls: i32 = 0;

func consume(take names: String[2]): i32 {
    calls = calls + 1;
    return i32(bytes(names[0])[0]) + i32(bytes(names[1])[1]);
}

func use(c: bool, want: i32): i32 {
    let b1: Box = Box { names: [stringOf("xa"), stringOf("yb")] };
    let b2: Box = Box { names: [stringOf("pc"), stringOf("qd")] };
    return consume((c ? b1 : b2).names) - want;
}

func main(): i32 {
    let n: i32 = use(true, 120 + 98) + use(false, 112 + 100);
    return n + (calls - 2) * 10;
}
"#;
    let output = run_memory("guardfallbackctrlproj", control);
    assert_no_asan("guardfallbackctrlproj", &output);
    assert_exit("guardfallbackctrlproj", &output, 0);

    let control_calls = r#"module spec.cpr.guardfallbackctrlcond;

import { String, stringOf, bytes } from std.text;

let calls: i32 = 0;

func mk1(): String[2] {
    calls = calls + 1;
    return [stringOf("AA"), stringOf("BB")];
}

func mk2(): String[2] {
    calls = calls + 1;
    return [stringOf("CC"), stringOf("DD")];
}

func consume(take names: String[2]): i32 {
    return i32(bytes(names[0])[0]) + i32(bytes(names[1])[1]);
}

func use(ok: bool, want: i32): i32 {
    return consume(ok ? mk1() : mk2()) - want;
}

func main(): i32 {
    let n: i32 = use(true, 65 + 66) + use(false, 67 + 68);
    return n + (calls - 2) * 10;
}
"#;
    let output = run_memory("guardfallbackctrlcond", control_calls);
    assert_no_asan("guardfallbackctrlcond", &output);
    assert_exit("guardfallbackctrlcond", &output, 0);
}

// ---------------------------------------------------------------------------
// arrayelement: a call-filled fixed-array field inside an array literal
// element.
//
// `Outer { items: [Inner { row: n1() }, ...] }` nests a struct literal in an
// array literal. An element whose struct carries a call-filled fixed-array
// field can only be built through the field's hidden destination, so the
// array literal must descend into its elements when it decides between the
// initializer and the per-field store form; otherwise the emitted element
// renders the call as a value and drops its destination. The discarded
// direct-element spelling and the direct struct field are the controls.
// ---------------------------------------------------------------------------

/// An array element's call-filled field fills through its destination and
/// reads back.
///
/// The kept spelling selects one element conditional and reads the selected
/// element's first byte back (`A` or `C`), so both selections exit 0 and the
/// element's field provably holds the call's rows; the counters prove each
/// element's call ran once. The discarded direct-element spelling and the
/// direct `Inner { row: n1() }` field keep their acceptance.
#[test]
fn should_read_back_a_call_filled_field_of_an_array_element() {
    let kept = r#"module spec.cpr.arrayelement;

struct Inner { row: str[2]; }
struct Outer { items: Inner[2]; }

let calls: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    calls = calls + 1;
    return ["CC", "DD"];
}

func use(ok: bool): i32 {
    let o: Outer = Outer { items: [ok ? Inner { row: n1() } : Inner { row: n2() }, Inner { row: ["zz", "ww"] }] };
    return i32(o.items[0].row[0][0]) - (ok ? 65 : 67);
}

func main(): i32 {
    return use(true) + use(false) + (calls - 2) * 10;
}
"#;
    let output = run_memory("arrayelement", kept);
    assert_no_asan("arrayelement", &output);
    assert_exit("arrayelement", &output, 0);

    let discarded = r#"module spec.cpr.arrayelementdiscard;

struct Inner { row: str[2]; }
struct Outer { items: Inner[2]; }

let calls: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    calls = calls + 1;
    return ["CC", "DD"];
}

func use(): i32 {
    Outer { items: [Inner { row: n1() }, Inner { row: n2() }] };
    return calls - 2;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("arrayelementdiscard", discarded);
    assert_no_asan("arrayelementdiscard", &output);
    assert_exit("arrayelementdiscard", &output, 0);

    let field = r#"module spec.cpr.arrayelementfield;

struct Inner { row: str[2]; }

let calls: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    calls = calls + 1;
    return ["CC", "DD"];
}

func use(): i32 {
    let i: Inner = Inner { row: n1() };
    let k: Inner = Inner { row: n2() };
    return i32(i.row[0][0]) + i32(k.row[1][1]) - (65 + 68) + calls - 2;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("arrayelementfield", field);
    assert_no_asan("arrayelementfield", &output);
    assert_exit("arrayelementfield", &output, 0);
}

// ---------------------------------------------------------------------------
// nestedread: the element-read base of a nested conditional arm.
//
// `(c ? (d ? b1 : b2).ys : b3.ys)[i]` selects a fixed-array field through a
// nested conditional and reads one element. The outer conditional's arm is
// itself a projection of a conditional, which is not an lvalue, so the read
// base must distribute through the nested arms the same way the store,
// return, and `for`-iterable gates already do: each leaf's own field is
// addressed on its path. The store and iterable distributions are the
// accepted controls.
// ---------------------------------------------------------------------------

/// A nested conditional arm reads its selected element on every path.
///
/// The read walks `(c ? (d ? b1 : b2).ys : b3.ys)[i]` over four elements,
/// twice per call with the selection flipped, so all three leaves (`b1`,
/// `b2`, `b3`) are read and the trailing constants cancel the sums. The
/// store and `for`-iterable distributions of the same base keep their
/// acceptance.
#[test]
fn should_read_a_nested_conditional_arms_fixed_array_element() {
    let element = r#"module spec.cpr.nestedread;

struct Box { ys: u8[4]; }

func read(c0: bool, d: bool): i32 {
    let b1: Box = Box { ys: [1, 2, 3, 4] };
    let b2: Box = Box { ys: [5, 6, 7, 8] };
    let b3: Box = Box { ys: [9, 10, 11, 12] };
    let c: bool = c0;
    let s: i32 = 0;
    let k: i32 = 0;
    while (k < 2) {
        let i: i32 = 0;
        while (i < 4) {
            s = s + i32((c ? (d ? b1 : b2).ys : b3.ys)[i]);
            i = i + 1;
        }
        c = !c;
        k = k + 1;
    }
    return s;
}

func main(): i32 {
    let r: i32 = read(true, true) - 52;
    r = r + read(true, false) - 68;
    r = r + read(false, true) - 52;
    return r;
}
"#;
    let output = run_memory("nestedread", element);
    assert_no_asan("nestedread", &output);
    assert_exit("nestedread", &output, 0);

    let stored = r#"module spec.cpr.nestedreadstore;

struct Box { ys: u8[4]; }

func use(c: bool, d: bool): i32 {
    let b1: Box = Box { ys: [1, 2, 3, 4] };
    let b2: Box = Box { ys: [5, 6, 7, 8] };
    let b3: Box = Box { ys: [9, 10, 11, 12] };
    let a: u8[4] = (c ? (d ? b1 : b2).ys : b3.ys);
    return i32(a[0]) + i32(a[1]) + i32(a[2]) + i32(a[3]);
}

func main(): i32 {
    let r: i32 = use(true, true) - 10;
    r = r + use(true, false) - 26;
    r = r + use(false, true) - 42;
    r = r + use(false, false) - 42;
    return r;
}
"#;
    let output = run_memory("nestedreadstore", stored);
    assert_no_asan("nestedreadstore", &output);
    assert_exit("nestedreadstore", &output, 0);

    let iterated = r#"module spec.cpr.nestedreaditerate;

struct Box { ys: u8[4]; }

func use(c: bool, d: bool): i32 {
    let b1: Box = Box { ys: [1, 2, 3, 4] };
    let b2: Box = Box { ys: [5, 6, 7, 8] };
    let b3: Box = Box { ys: [9, 10, 11, 12] };
    let s: i32 = 0;
    for (y in (c ? (d ? b1 : b2).ys : b3.ys)) {
        s = s + i32(y);
    }
    return s;
}

func main(): i32 {
    let r: i32 = use(true, true) - 10;
    r = r + use(true, false) - 26;
    r = r + use(false, true) - 42;
    r = r + use(false, false) - 42;
    return r;
}
"#;
    let output = run_memory("nestedreaditerate", iterated);
    assert_no_asan("nestedreaditerate", &output);
    assert_exit("nestedreaditerate", &output, 0);
}

// ---------------------------------------------------------------------------
// keptviewtag: a kept binding whose element is a fresh call view.
//
// `Holder { names: [mk().v], tag: stringOf("t") }` stores a view of a fresh
// result beside an owning sibling field. The destination keeps the view past
// the statement that fills it, so the fresh root lives in the binding's own
// scope: the binding reads the bytes in later statements, and the scope's
// cleanup releases the root after that last read. The owning sibling's own
// release follows the same scope, and both selections run.
// ---------------------------------------------------------------------------

/// A kept view binding releases its fresh root after the binding's last use.
#[test]
fn should_keep_a_fresh_view_root_until_the_bindings_scope_ends() {
    let kept = r#"module spec.cpr.keptviewtag;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; tag: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func run(g: bool, want: i32): i32 {
    let h: Holder = g ? Holder { names: [mk().v], tag: stringOf("t") } : Holder { names: ["zz"], tag: stringOf("q") };
    let first: i32 = i32(h.names[0][0]);
    let tag: i32 = i32(bytes(h.tag)[0]);
    return first + tag - want;
}

func main(): i32 {
    return run(true, 97 + 116) + run(false, 122 + 113);
}
"#;
    let output = run_memory("keptviewtag", kept);
    assert_no_asan("keptviewtag", &output);
    assert_exit("keptviewtag", &output, 0);
}

// ---------------------------------------------------------------------------
// guardelemcond: a guarded element conditional of store-needing literals.
//
// `Outer { items: [ok ? Inner { row: a.row } : Inner { row: b.row }] }` inside
// a `&&` operand stores the chosen element per arm: that arm moves into the
// element and is emptied, so its own release finds nothing to free and the
// element the guard reads is the single owner of the moved rows. The second
// program keeps a following element, so the store order stays visible.
// ---------------------------------------------------------------------------

/// A guarded element conditional moves the chosen arm into the element.
#[test]
fn should_move_the_guarded_element_conditional_arm_into_its_destination() {
    let moved = r#"module spec.cpr.guardelemcond;

import { String, stringOf } from std.text;

struct Inner { row: String[2]; }
struct Outer { items: Inner[1]; }

func use(ok: bool, g: bool, want: i32): i32 {
    let a: Inner = Inner { row: [stringOf("ab"), stringOf("cd")] };
    let b: Inner = Inner { row: [stringOf("ef"), stringOf("gh")] };
    if (g && i32(len(Outer { items: [ok ? Inner { row: a.row } : Inner { row: b.row }] }.items[0].row[0])) > 0) {
        return 1;
    }
    return want;
}

func main(): i32 {
    let r: i32 = use(true, true, 0) - 1;
    let s: i32 = use(false, true, 0) - 1;
    let t: i32 = use(true, false, 2) - 2;
    return r + s + t;
}
"#;
    let output = run_memory("guardelemcond", moved);
    assert_no_asan("guardelemcond", &output);
    assert_exit("guardelemcond", &output, 0);

    let with_second = r#"module spec.cpr.guardelemcondsecond;

import { String, stringOf } from std.text;

struct Box { row: String[2]; }
struct Inner { row: String[2]; }
struct Outer { items: Inner[2]; }

func mkBox(x: u8): Box {
    if (x == 0) { return Box { row: [stringOf("ab"), stringOf("cd")] }; }
    return Box { row: [stringOf("ef"), stringOf("gh")] };
}

func use(ok: bool, g: bool, want: i32): i32 {
    let a: Box = mkBox(0);
    let b: Box = mkBox(1);
    if (g && i32(len(Outer { items: [ok ? Inner { row: a.row } : Inner { row: b.row }, Inner { row: [stringOf("zz"), stringOf("ww")] }] }.items[0].row[0])) > 0) {
        return 1;
    }
    return want;
}

func main(): i32 {
    let r: i32 = use(true, true, 0) - 1;
    let s: i32 = use(false, true, 0) - 1;
    let t: i32 = use(true, false, 0);
    return r + s + t;
}
"#;
    let output = run_memory("guardelemcondsecond", with_second);
    assert_no_asan("guardelemcondsecond", &output);
    assert_exit("guardelemcondsecond", &output, 0);
}

// ---------------------------------------------------------------------------
// loopreceiver: the mixed-arm receiver of a loop condition.
//
// `while (i < k && (c ? mkH() : a).call(5) > 0)` reads the callable field
// from a conditional receiver and calls it. The receiver's fresh arm builds a
// value that owns a string, so the receiver position must settle the arm's
// parts before the field is read: the current emission nests the fresh arm's
// release and fill in a comma expression that the pinned container toolchain
// accepts and the host GCC 15 rejects with `-Werror=sequence-point`. The case
// pins the run values and the strict build in the suite's own environment.
// The direct receiver and the plain loop condition are the controls.
// ---------------------------------------------------------------------------

/// The mixed-arm receiver of a loop condition settles per iteration and runs
/// the selected arm's callable.
///
/// `mkH()` carries `addTen`, so the fresh arm's `call(5)` is `15` and the
/// loop runs its `k` iterations; the place arm carries `neg`, so its
/// `call(5)` is `-5` and the loop runs zero times. The two runs prove both
/// selections and their values. The direct receiver and the plain loop
/// condition keep their acceptance and values.
#[test]
fn should_settle_a_mixed_arm_receiver_in_a_loop_condition() {
    let source = r#"module spec.cpr.loopreceiver;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addTen(x: i32): i32 { return x + 10; }
func neg(x: i32): i32 { return 0 - x; }

func mkH(): H { return H { call: addTen, s: stringOf("mk") }; }

func use(c: bool, take a: H, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && (c ? mkH() : a).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func usePlain(k: i32): i32 {
    let a: H = H { call: addTen, s: stringOf("pa") };
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && a.call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func useDirect(): i32 {
    let a: H = H { call: addTen, s: stringOf("pa") };
    return a.call(5) - 15;
}

func main(): i32 {
    let a1: H = H { call: neg, s: stringOf("aa") };
    let n: i32 = use(true, a1, 1) - 1;
    let a2: H = H { call: neg, s: stringOf("bb") };
    n = n + use(false, a2, 1);
    return n + usePlain(2) - 2 + useDirect();
}
"#;
    let output = run_memory("loopreceiver", source);
    assert_no_asan("loopreceiver", &output);
    assert_exit("loopreceiver", &output, 0);
}

// ---------------------------------------------------------------------------
// loopmember: the loop-condition member projection.
//
// `while (i < k && i32(len(mkH().s)) > 0)` projects a member of a fresh
// result inside the loop condition. The projection's root must bind once per
// evaluation and release exactly once, so the single-shot and the re-executed
// loop run without freeing the same storage twice and the run is
// memory-clean. The store and argument twins of the same projection are the
// controls.
// ---------------------------------------------------------------------------

/// A loop-condition member projection releases its fresh root once per
/// evaluation.
///
/// The single-shot (`k = 1`) and re-executed (`k = 3`) loops each evaluate
/// `mkH().s` once per iteration, count their taken iterations, and exit 0
/// with no memory report; the store twin `let s: String = mkH().s;` and the
/// argument twin `consume(mkH().s)` keep their acceptance and values.
#[test]
fn should_release_a_loop_condition_member_projection_once() {
    let source = r#"module spec.cpr.loopmember;

import { String, stringOf } from std.text;

struct H { s: String; tag: String; }

func mkH(): H { return H { s: stringOf("ab"), tag: stringOf("cd") }; }

func consume(take s: String): i32 { return i32(len(s)); }

func use(k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && i32(len(mkH().s)) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func storeTwin(): i32 {
    let s: String = mkH().s;
    return i32(len(s)) - 2;
}

func takeTwin(): i32 {
    return consume(mkH().s) - 2;
}

func main(): i32 {
    let a: i32 = use(1) - 1;
    let b: i32 = use(3) - 3;
    let c: i32 = storeTwin();
    let d: i32 = takeTwin();
    return a + b + c + d;
}
"#;
    let output = run_memory("loopmember", source);
    assert_no_asan("loopmember", &output);
    assert_exit("loopmember", &output, 0);
}

// ---------------------------------------------------------------------------
// guardupdate: the updating parts of a guarded operand.
//
// `g && f(i++, i) == 34`, `g || f(i++, i) == 34`, and
// `c ? f(i++, i) : 0` keep the deferred spelling, which reads the update and
// the use in one unsequenced C expression and fails the strict build
// (`-Werror=sequence-point` on the pinned container and the host alike). The
// guarded operand's updating parts must settle inside the guard path, the way
// the loop-condition call arguments already settle, so the update completes
// at its source position on the evaluated path only. The plain loop condition
// `while (f(i++, i) < 30)` is the landed control.
//
// Each path reads its pass value from the guard's own outcome: the evaluated
// path passes when `ok` is true, the skipped path passes when the guard
// short-circuited and `ok` is false. The exit stays 0 while every path is
// correct, and a wrong call-argument order makes `ok` false on the evaluated
// path, so the exit check fails.
// ---------------------------------------------------------------------------

/// A guarded or conditional operand with an update settles its parts on the
/// evaluated path only.
///
/// Each spelling leaves `i` at `4` on the evaluated path (the call sees the
/// old `i` left-to-right and the increment lands once) and at `3` on the
/// skipped path, and the call counter proves the skipped path ran nothing.
/// The recorded call arguments (`3` and `4`) and the ternary's value (`34`)
/// prove the left-to-right order. Each path's pass value comes from its own
/// guard outcome: the evaluated path needs `ok` true, the skipped path needs
/// `ok` false, so a wrong argument order leaves the exit non-zero. The plain
/// loop condition counts its three taken iterations and its four calls.
#[test]
fn should_settle_the_updating_parts_of_a_guarded_operand() {
    let source = r#"module spec.cpr.guardupdate;

let calls: i32 = 0;
let last_a: i32 = 0;
let last_b: i32 = 0;

func f(a: i32, b: i32): i32 {
    calls = calls + 1;
    last_a = a;
    last_b = b;
    return a * 10 + b;
}

func guardAnd(g: bool): i32 {
    let i: i32 = 3;
    let ok: bool = g && f(i++, i) == 34;
    if (g) {
        return (ok ? 0 : 1) + (i - 4);
    }
    return (ok ? 1 : 0) + (i - 3);
}

func guardOr(g: bool): i32 {
    let i: i32 = 3;
    let ok: bool = g || f(i++, i) == 34;
    if (g) {
        return (ok ? 0 : 1) + (i - 3);
    }
    return (ok ? 0 : 1) + (i - 4);
}

func ternary(c: bool): i32 {
    let i: i32 = 3;
    let r: i32 = c ? f(i++, i) : 0;
    return r - (c ? 34 : 0) + i - (c ? 4 : 3);
}

func plain(): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (f(i++, i) < 30) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let n: i32 = guardAnd(true) + guardAnd(false) + guardOr(true) + guardOr(false);
    n = n + ternary(true) + ternary(false);
    n = n + plain() - 3;
    n = n + (calls - 7) * 10;
    return n + (last_a - 3) + (last_b - 4);
}
"#;
    let output = run_memory("guardupdate", source);
    assert_no_asan("guardupdate", &output);
    assert_exit("guardupdate", &output, 0);
}

// ---------------------------------------------------------------------------
// arrayfill: the array fill form's single evaluation.
//
// `let xs: i32[3] = [; i++];` evaluates the fill expression once and copies
// the value to every element (Ch 04 §3). The observing counter and the
// element values prove the single evaluation: a per-element evaluation would
// count three increments and leave three distinct values. The fill-free
// literal and the element-list literal are the controls.
// ---------------------------------------------------------------------------

/// An array fill expression evaluates once and copies to every element.
///
/// `[; i++]` with `i = 7` gives every element `7` and leaves `i = 8`; the sum
/// and the counter fold into the exit code, so a per-element fill fails the
/// read-back. The fill-free `[1, 2, 3]` and the element-list `[i++, i++, i++]`
/// controls keep their element-wise semantics.
#[test]
fn should_evaluate_an_array_fill_expression_once() {
    let source = r#"module spec.cpr.arrayfill;

func fillOnce(): i32 {
    let i: i32 = 7;
    let xs: i32[3] = [; i++];
    let n: i32 = xs[0] + xs[1] + xs[2];
    return n - 21 + (i - 8);
}

func fillFree(): i32 {
    let a: i32[3] = [1, 2, 3];
    return a[0] + a[1] + a[2] - 6;
}

func elementList(): i32 {
    let i: i32 = 0;
    let b: i32[3] = [i++, i++, i++];
    let n: i32 = b[0] + b[1] + b[2];
    return n - 3 + (i - 3);
}

func main(): i32 {
    return fillOnce() + fillFree() + elementList();
}
"#;
    let output = run_memory("arrayfill", source);
    assert_no_asan("arrayfill", &output);
    assert_exit("arrayfill", &output, 0);
}

// ---------------------------------------------------------------------------
// rawderefstore: the bare raw-deref store.
//
// `unsafe { *q = 9; }` and `unsafe { *q = i++; }` name a dereferenced raw
// pointer as the assignment target, which Ch 04 §7 accepts. The store must
// lower to a write through the address (the indexed path's store shape) and
// an updating right-hand side must settle before the store; the field write
// through a raw and the bare deref read are the working controls.
// ---------------------------------------------------------------------------

/// A bare raw-deref store writes through the address and reads back.
///
/// `unsafe { *q = 9; }` leaves `a = 9`, and the deref read returns it; the
/// update twin `unsafe { *r = i++; }` stores the pre-increment value
/// (`b = 0`) and lands the increment once (`i = 1`). The field write through
/// a raw (`p.v = 7`) keeps working.
#[test]
fn should_lower_a_bare_raw_deref_store() {
    let source = r#"module spec.cpr.rawderefstore;

struct S { v: i32; w: i32; }

func main(): i32 {
    let a: i32 = 0;
    let q: raw i32 = unsafe &a;
    unsafe { *q = 9; }
    let v: i32 = unsafe *q;
    let i: i32 = 0;
    let b: i32 = 0;
    let r: raw i32 = unsafe &b;
    unsafe { *r = i++; }
    let s: S = S { v: 0, w: 0 };
    let p: raw S = unsafe &s;
    unsafe { p.v = 7; }
    return (a - 9) + (v - 9) + b + (i - 1) + (s.v - 7);
}
"#;
    let output = run_memory("rawderefstore", source);
    assert_no_asan("rawderefstore", &output);
    assert_exit("rawderefstore", &output, 0);
}

// ---------------------------------------------------------------------------
// looprecvplace: a mixed-arm receiver whose place arm is selected again.
//
// A receiver conditional that borrows a place must keep the place a read: the
// call only reads the callable field, so the place stays usable after the
// call and across a re-executed loop condition. The first program selects the
// place arm twice (`k = 2`) and reads its bytes after the loop, the second
// selects the fresh arm, the third spells the eager read beside a later read
// of the same place, and the fourth replaces an owning pointee through a raw
// store, which must release the old value exactly once.
// ---------------------------------------------------------------------------

/// A borrowed receiver place stays readable after the call.
#[test]
fn should_keep_a_borrowed_receiver_place_readable_after_the_call() {
    let looped = r#"module spec.cpr.looprecvplace;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addTen(x: i32): i32 { return x + 10; }
func mkH(): H { return H { call: addTen, s: stringOf("mk") }; }

func use(c: bool, take a: H, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && (c ? mkH() : a).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n + i32(len(a.s)) - (k + 2);
}

func main(): i32 {
    let a1: H = H { call: addTen, s: stringOf("aa") };
    let n: i32 = use(false, a1, 2);
    let a2: H = H { call: addTen, s: stringOf("bb") };
    let m: i32 = use(true, a2, 1);
    return n + m;
}
"#;
    let output = run_memory("looprecvplace", looped);
    assert_no_asan("looprecvplace", &output);
    assert_exit("looprecvplace", &output, 0);

    let eager = r#"module spec.cpr.eagerborrowrecv;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addTen(x: i32): i32 { return x + 10; }
func mkH(): H { return H { call: addTen, s: stringOf("mk") }; }

func use(c: bool, take a: H): i32 {
    return (c ? mkH() : a).call(5) - 15;
}

func main(): i32 {
    let a1: H = H { call: addTen, s: stringOf("aa") };
    let n: i32 = use(false, a1);
    let a2: H = H { call: addTen, s: stringOf("bb") };
    let m: i32 = use(true, a2);
    return n + m;
}
"#;
    let output = run_memory("eagerborrowrecv", eager);
    assert_no_asan("eagerborrowrecv", &output);
    assert_exit("eagerborrowrecv", &output, 0);

    let later = r#"module spec.cpr.laterreadrecv;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addTen(x: i32): i32 { return x + 10; }
func mkH(): H { return H { call: addTen, s: stringOf("mk") }; }

func use(c: bool, take a: H): i32 {
    let r: i32 = (c ? mkH() : a).call(5) - 15;
    return r + i32(len(a.s)) - 2;
}

func main(): i32 {
    let a1: H = H { call: addTen, s: stringOf("aa") };
    let n: i32 = use(false, a1);
    let a2: H = H { call: addTen, s: stringOf("bb") };
    let m: i32 = use(true, a2);
    return n + m;
}
"#;
    let output = run_memory("laterreadrecv", later);
    assert_no_asan("laterreadrecv", &output);
    assert_exit("laterreadrecv", &output, 0);
}

// ---------------------------------------------------------------------------
// rawreplace: an owning pointee replaced through a raw store.
//
// `unsafe { *p = stringOf("cd"); }` replaces the pointee's value: the new
// value lands, the read-back proves it, and the replaced value is released
// exactly once, so the run is memory-clean.
// ---------------------------------------------------------------------------

/// A raw store releases the replaced owning pointee exactly once.
#[test]
fn should_release_the_replaced_owning_pointee_of_a_raw_store() {
    let replaced = r#"module spec.cpr.rawreplace;

import { String, stringOf, bytes } from std.text;

func use(): i32 {
    let s: String = stringOf("ab");
    let p: raw String = unsafe &s;
    unsafe { *p = stringOf("cd"); }
    return i32(bytes(s)[0]) - 99 + (i32(len(s)) - 2);
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("rawreplace", replaced);
    assert_no_asan("rawreplace", &output);
    assert_exit("rawreplace", &output, 0);
}

// ---------------------------------------------------------------------------
// rawcompoundonce: a compound raw-deref store evaluates its pointer once.
//
// `unsafe { *mkq(&a) += 7; }` reads and writes the same address, so the
// pointer expression runs a single time (the counter proves it) and the stored
// value is correct. The update spelling `*(r + j++) += 7` runs the increment
// once and reads and writes through that one address.
// ---------------------------------------------------------------------------

/// A compound raw store evaluates its pointer once and shares the address.
#[test]
fn should_evaluate_a_compound_raw_store_pointer_once() {
    let impure = r#"module spec.cpr.rawcompoundonce;

let pcalls: i32 = 0;

func mkq(p: raw i32): raw i32 {
    pcalls = pcalls + 1;
    return p;
}

func main(): i32 {
    let a: i32 = 5;
    unsafe {
        *mkq(&a) += 7;
    }
    return (a - 12) + (pcalls - 1) * 10;
}
"#;
    let output = run_memory("rawcompoundonce", impure);
    assert_no_asan("rawcompoundonce", &output);
    assert_exit("rawcompoundonce", &output, 0);

    let update = r#"module spec.cpr.rawarithstore;

func main(): i32 {
    let a: i32 = 4;
    let q: raw i32 = unsafe &a;
    let i: i32 = 0;
    unsafe {
        *(q + i++) = 9;
    }
    let n: i32 = (a - 9) + (i - 1);
    let b: i32 = 3;
    let r: raw i32 = unsafe &b;
    let j: i32 = 0;
    unsafe {
        *(r + j++) += 7;
    }
    return n + (b - 10) + (j - 1);
}
"#;
    let output = run_memory("rawarithstore", update);
    assert_no_asan("rawarithstore", &output);
    assert_exit("rawarithstore", &output, 0);
}

// ---------------------------------------------------------------------------
// rawnullablestore: the nullable raw store reports the non-null requirement.
//
// A dereferenced nullable raw pointer reports `E10010` whether it is read or
// written, so the store spelling and its read twin each carry exactly one
// report; a non-nullable pointer stores unchanged.
// ---------------------------------------------------------------------------

/// A nullable raw store reports one `E10010`, matching its read twin.
#[test]
fn should_reject_a_nullable_raw_store_like_its_read_twin() {
    let stored = r#"module spec.cpr.rawnullablestore;

func main(): i32 {
    let p: raw i32? = null;
    unsafe {
        *p = 9;
    }
    return 0;
}
"#;
    compile_rejects("rawnullablestore", stored, "E10010", 1);

    let read = r#"module spec.cpr.rawnullableread;

func main(): i32 {
    let p: raw i32? = null;
    let v: i32 = unsafe *p;
    return v;
}
"#;
    compile_rejects("rawnullableread", read, "E10010", 1);

    let nonnull = r#"module spec.cpr.rawnonnullstore;

func main(): i32 {
    let a: i32 = 0;
    let p: raw i32 = unsafe &a;
    unsafe {
        *p = 9;
    }
    return a - 9;
}
"#;
    let output = run_memory("rawnonnullstore", nonnull);
    assert_no_asan("rawnonnullstore", &output);
    assert_exit("rawnonnullstore", &output, 0);
}

// ---------------------------------------------------------------------------
// rawselfstore: the raw self-store follows the ordinary self-assignment.
//
// `unsafe { *q = *q; }` releases the replaced pointee and clears its storage
// before the value is read, exactly as `a = a` does for a named destination:
// the read then sees the emptied storage, so the replaced value is released
// exactly once and the run is memory-clean. The move-in spelling stores a
// named value and empties its source, like an ordinary owned assignment; a
// later read of the source reports the moved-value error for the raw target
// exactly as it does for the named one.
// ---------------------------------------------------------------------------

/// A raw self-store releases the replaced value once, like `a = a`.
#[test]
fn should_release_a_raw_self_store_once_like_a_named_self_assignment() {
    let self_store = r#"module spec.cpr.rawselfstore;

import { String, stringOf } from std.text;

func use(): i32 {
    let a: String = stringOf("aa");
    let q: raw String = unsafe &a;
    unsafe {
        *q = *q;
    }
    return 0;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("rawselfstore", self_store);
    assert_no_asan("rawselfstore", &output);
    assert_exit("rawselfstore", &output, 0);

    let move_in = r#"module spec.cpr.rawmovestore;

import { String, stringOf } from std.text;

func use(): i32 {
    let a: String = stringOf("aa");
    let y: String = stringOf("cd");
    let q: raw String = unsafe &a;
    unsafe {
        *q = y;
    }
    return i32(len(a)) - 2;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("rawmovestore", move_in);
    assert_no_asan("rawmovestore", &output);
    assert_exit("rawmovestore", &output, 0);

    let raw_use = r#"module spec.cpr.rawmovestoreuse;

import { String, stringOf } from std.text;

func use(): i32 {
    let a: String = stringOf("aa");
    let y: String = stringOf("cd");
    let q: raw String = unsafe &a;
    unsafe {
        *q = y;
    }
    return i32(len(y));
}

func main(): i32 { return use(); }
"#;
    compile_rejects("rawmovestoreuse", raw_use, "E10020", 1);

    let place_use = r#"module spec.cpr.placemovestoreuse;

import { String, stringOf } from std.text;

func use(): i32 {
    let a: String = stringOf("aa");
    let y: String = stringOf("cd");
    a = y;
    return i32(len(y));
}

func main(): i32 { return use(); }
"#;
    compile_rejects("placemovestoreuse", place_use, "E10020", 1);
}

// ---------------------------------------------------------------------------
// rawcalltarget: an impure raw store pointer evaluates exactly once.
//
// `unsafe { *alt(p, q) = v; }` names a call as the store target, so the
// pointer expression must run once and every use — the release of the
// replaced value, the clear, and the store — must read that one result. The
// counter proves the single evaluation, the first result's pointee receives
// the value, and the replaced value is released exactly once. The simple
// place spelling and the compound spelling with an impure pointer run under
// the same rule.
// ---------------------------------------------------------------------------

/// An impure raw store pointer runs once and every use reads its result.
#[test]
fn should_evaluate_an_impure_raw_store_pointer_once() {
    let plain = r#"module spec.cpr.rawcalltarget;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt(s: raw String, t: raw String): raw String {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func use(): i32 {
    let s: String = stringOf("aa");
    let t: String = stringOf("bb");
    let v: String = stringOf("zzzz");
    let p: raw String = unsafe &s;
    let q: raw String = unsafe &t;
    unsafe {
        *alt(p, q) = v;
    }
    return i32(len(s)) - 4 + i32(len(t)) - 2 + (calls - 1) * 10;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("rawcalltarget", plain);
    assert_no_asan("rawcalltarget", &output);
    assert_exit("rawcalltarget", &output, 0);

    let simple = r#"module spec.cpr.rawplainstore;

func use(): i32 {
    let a: i32 = 0;
    let p: raw i32 = unsafe &a;
    unsafe {
        *p = 9;
    }
    return a - 9;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("rawplainstore", simple);
    assert_no_asan("rawplainstore", &output);
    assert_exit("rawplainstore", &output, 0);

    let compound = r#"module spec.cpr.rawcallcompound;

let calls: i32 = 0;

func alt(s: raw i32, t: raw i32): raw i32 {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func use(): i32 {
    let a: i32 = 5;
    let b: i32 = 7;
    let p: raw i32 = unsafe &a;
    let q: raw i32 = unsafe &b;
    unsafe {
        *alt(p, q) += 3;
    }
    return (a - 8) + (b - 7) + (calls - 1) * 10;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("rawcallcompound", compound);
    assert_no_asan("rawcallcompound", &output);
    assert_exit("rawcallcompound", &output, 0);
}

// ---------------------------------------------------------------------------
// rawdistinct: an owned raw-deref read moves from its own pointee.
//
// `unsafe { *q = *r; }` with two distinct owning pointees moves the value: the
// target's replaced value is released once, the source pointee is emptied, and
// only the target owns the buffer, so the run is memory-clean and the value
// lands once. A named source through a raw (`unsafe { *q = b; }`) consumes the
// source exactly like the named twin, so a later use reports one moved-value
// error for both spellings.
// ---------------------------------------------------------------------------

/// An owned raw-deref read used as a store value moves from its pointee.
#[test]
fn should_move_an_owned_pointee_read_used_as_a_store_value() {
    let moved = r#"module spec.cpr.rawdistinct;

import { String, stringOf } from std.text;

func use(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let q: raw String = unsafe &a;
    let r: raw String = unsafe &b;
    unsafe {
        *q = *r;
    }
    return i32(len(a)) - 2;
}

func main(): i32 { return use(); }
"#;
    let output = run_memory("rawdistinct", moved);
    assert_no_asan("rawdistinct", &output);
    assert_exit("rawdistinct", &output, 0);

    let raw_use = r#"module spec.cpr.rawdistinctuse;

import { String, stringOf } from std.text;

func use(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let q: raw String = unsafe &a;
    unsafe {
        *q = b;
    }
    return i32(len(b));
}

func main(): i32 { return use(); }
"#;
    compile_rejects("rawdistinctuse", raw_use, "E10020", 1);

    let named_use = r#"module spec.cpr.nameddistinctuse;

import { String, stringOf } from std.text;

func use(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    a = b;
    return i32(len(b));
}

func main(): i32 { return use(); }
"#;
    compile_rejects("nameddistinctuse", named_use, "E10020", 1);
}

// ---------------------------------------------------------------------------
// onereportnested: one report for a nested mixed view return.
//
// A return whose conditional mixes a local arm with a nested fresh-owner
// conditional makes the per-backing walk report the generated-owner escape
// twice at one span. Dependent errors are suppressed (Ch 14 section 1), so
// the site carries one report. The cross-site twin — a rejected callee view
// return whose result the caller stores into a field — keeps only the
// callee's own `E10036`; the store's consequence report is withheld. The
// local-only and local-plus-parameter returns keep their single `E10035`.
// ---------------------------------------------------------------------------

/// A nested mixed view return carries one report, and a rejected callee's
/// consequence report is withheld.
///
/// `return (c ? a : (d ? mkBox() : mkBox2())).v;` mixes a local arm with two
/// fresh arms, so two generated owners back one expression; the site reports
/// exactly one `E10030`. The `str[2]` return that derives its views from
/// module bindings rather than one view-like parameter reports exactly one
/// `E10036` at the callee; the caller's store of that result adds no `E10033`
/// consequence. A `Pair` array literal that stores views of two locals
/// reports exactly one `E10030` at the store. The local-only and
/// local-plus-parameter returns keep their single `E10035` at the return.
#[test]
fn should_report_a_nested_mixed_view_return_once() {
    let nested = r#"module spec.cpr.onereportnested;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }
func mkBox2(): Box { return Box { v: stringOf("ef") }; }

func f(c: bool, d: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    return (c ? a : (d ? mkBox() : mkBox2())).v;
}

func main(): i32 { return 0; }
"#;
    compile_rejects_once_at(
        "onereportnested",
        nested,
        "E10030",
        12,
        5,
        &["a compiler-generated owner", "does not live long enough"],
        &[],
    );

    let gfieldcall = r#"module spec.cpr.onereportgfield;

let s: str = "ab";
let t: str = "cd";

struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func row(): str[2] { return [s, t]; }

func store(): void { g.names = row(); }

func main(): i32 { store(); return i32(len(g.names[1])) - 2; }
"#;
    compile_rejects_once_at(
        "onereportgfield",
        gfieldcall,
        "E10036",
        10,
        6,
        &["view return does not derive from exactly one parameter"],
        &["E10033"],
    );

    let pair = r#"module spec.cpr.onereportpair;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Pair { names: str[2]; }

func pick(): i32 {
    let h: Pair = Pair { names: ["AA", "BB"] };
    {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        let c: Box = Box { v: stringOf("pq"), w: stringOf("rs") };
        h = Pair { names: [b.v, c.v] };
    }
    return i32(h.names[0][0]);
}

func main(): i32 { return pick() - 120; }
"#;
    compile_rejects_once_at(
        "onereportpair",
        pair,
        "E10030",
        13,
        9,
        &["does not live long enough"],
        &[],
    );

    let localonly = r#"module spec.cpr.onereportlocalonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    let b: Box = Box { v: stringOf("ef") };
    return (c ? a : b).v;
}

func main(): i32 { return 0; }
"#;
    compile_rejects_once_at(
        "onereportlocalonly",
        localonly,
        "E10035",
        10,
        5,
        &["cannot return a view of local"],
        &[],
    );

    let localparam = r#"module spec.cpr.onereportlocalparam;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool, a: str): str {
    let b: Box = Box { v: stringOf("cd") };
    return (c ? b.v : a);
}

func main(): i32 { return 0; }
"#;
    compile_rejects_once_at(
        "onereportlocalparam",
        localparam,
        "E10035",
        9,
        5,
        &["cannot return a view of local"],
        &[],
    );
}

// ---------------------------------------------------------------------------
// deferphrase: the deferred generated owner renders its phrase.
//
// A `defer` block returning a view of a mixed conditional reports the
// generated-owner escape as `borrow of '__cl_owned1' ...` — the internal
// binding name leaks into the message. The escape must render the phrase the
// direct spelling already uses (`a compiler-generated owner`), and the
// internal name must never be user-visible; the direct mixed return is the
// control.
// ---------------------------------------------------------------------------

/// A deferred mixed view return renders the generated-owner phrase, never the
/// internal binding name.
///
/// The deferred `return (c ? l : mkBox()).v;` reports exactly one `E10030` at
/// the deferred return, whose message names `a compiler-generated owner` and
/// contains no `__cl_` spelling; the direct mixed return reports the same
/// phrase at its own return.
#[test]
fn should_render_the_deferred_generated_owner_phrase() {
    let deferred = r#"module spec.cpr.deferphrase;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(a: str, c: bool): str {
    defer {
        let l: Box = Box { v: stringOf("cd") };
        return (c ? l : mkBox()).v;
    }
    return a;
}

func main(): i32 { return 0; }
"#;
    compile_rejects_once_at(
        "deferphrase",
        deferred,
        "E10030",
        12,
        9,
        &[
            "borrow of a compiler-generated owner",
            "does not live long enough",
        ],
        &["__cl_"],
    );

    let direct = r#"module spec.cpr.deferphrasedirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(c: bool): str {
    let l: Box = Box { v: stringOf("cd") };
    return (c ? l : mkBox()).v;
}

func main(): i32 { return 0; }
"#;
    compile_rejects_once_at(
        "deferphrasedirect",
        direct,
        "E10030",
        11,
        5,
        &[
            "borrow of a compiler-generated owner",
            "does not live long enough",
        ],
        &["__cl_"],
    );
}

// ---------------------------------------------------------------------------
// iterableark: a whole-array conditional iterable leaves its arms usable.
//
// `for (borrow x in (c ? b1 : b2))` binds an exclusive element view per
// iteration (Ch 05 section 9): the loop reads the selected arm in place and
// never moves it, so a later use of either array stays accepted. The owned
// `String[2]` spelling pins the read-back of both arms; the `u8[4]` twin is
// the duplicable control.
// ---------------------------------------------------------------------------

/// A conditional iterable's arrays stay usable after the loop.
///
/// `for (borrow x in (c ? b1 : b2))` over `String[2]` arrays borrows each
/// element of the selected arm. The trailing reads of `b1[0]` and `b2[1]`
/// prove both arrays were read in place, and the exit code folds the lengths
/// so an emptied array would fail the read-back. The `u8[4]` twin keeps the
/// same shape with duplicable elements. Both programs build under the strict
/// flags, exit 0, and report no memory error.
#[test]
fn should_keep_a_whole_array_conditional_iterable_usable() {
    let owned = r#"module spec.cpr.iterableark;

import { String, stringOf } from std.text;

func use(c: bool): i32 {
    let b1: String[2] = [stringOf("AA"), stringOf("BBB")];
    let b2: String[2] = [stringOf("CCCC"), stringOf("DD")];
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2)) {
        s = s + i32(len(x));
    }
    return s + i32(len(b1[0])) + i32(len(b2[1]));
}

func main(): i32 {
    return (use(true) - 9) + (use(false) - 10);
}
"#;
    let output = run_memory("iterableark", owned);
    assert_no_asan("iterableark", &output);
    assert_exit("iterableark", &output, 0);

    let duplicable = r#"module spec.cpr.iterablearku8;

func use(c: bool): i32 {
    let b1: u8[4] = [1, 2, 3, 4];
    let b2: u8[4] = [5, 6, 7, 8];
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2)) {
        s = s + i32(x);
    }
    return s + i32(b1[0]) + i32(b2[1]);
}

func main(): i32 {
    return (use(true) - 17) + (use(false) - 33);
}
"#;
    let output = run_memory("iterablearku8", duplicable);
    assert_no_asan("iterablearku8", &output);
    assert_exit("iterablearku8", &output, 0);
}

// ---------------------------------------------------------------------------
// localfitarm: a frame-backed element of a returned array literal reports at
// its own return.
//
// An array literal stores every element, so its storage provenance keeps a
// local component beside the others and the declaration's count judgment is
// not reached for it. A return that carries a frame-backed element hands the
// caller storage that dies with the frame, so the site reports the
// local-owner judgment (`E10035`) whether a sibling return fits every caller
// (a literal), the store is an out-slot, the declaration counts one lend
// candidate, or the fitting arm is deferred. The all-fitting callee spellings
// — the deferred literal arm and the static return beside a lend candidate —
// stay accepted and memory-clean.
// ---------------------------------------------------------------------------

/// A returned array literal's frame-backed element reports its own `E10035`.
///
/// `return [l.v, "zw"]` stores a view of the local `l`, so the return reports
/// exactly one `E10035` however the caller stores the result: behind a
/// literal arm, into an out-slot, beside the single lend candidate `a`, and
/// behind a deferred literal arm. The out-slot spelling lends a duplicable
/// array, so the compiler also warns `W10001` there; its assertion pins the
/// single `E10035` by count. The two all-fitting spellings stay accepted,
/// build under the strict flags, and exit 0.
#[test]
fn should_reject_a_frame_backed_element_of_a_returned_array_literal() {
    let literal_arm = r#"module spec.cpr.localfitarm;

import { String, stringOf } from std.text;

struct Box { v: String; }

let g: str[2] = ["AA", "BB"];

func row(c: bool): str[2] {
    if (c) {
        return ["xy", "zw"];
    }
    let l: Box = Box { v: stringOf("ll") };
    return [l.v, "zw"];
}

func store(c: bool): void { g = row(c); }

func main(): i32 {
    store(false);
    return i32(g[0][0]) - 108;
}
"#;
    compile_rejects_once_at(
        "localfitarm",
        literal_arm,
        "E10035",
        14,
        5,
        &["cannot return a view of local 'l'"],
        &[],
    );

    let out_slot = r#"module spec.cpr.localslot;

import { String, stringOf } from std.text;

struct Box { v: String; }

func row(c: bool): str[2] {
    if (c) {
        return ["xy", "zw"];
    }
    let l: Box = Box { v: stringOf("ll") };
    return [l.v, "zw"];
}

func outer(borrow slot: str[2], c: bool): void { slot = row(c); }

func main(): i32 {
    let out: str[2] = ["AA", "BB"];
    outer(borrow out, false);
    return i32(out[0][0]) - 108;
}
"#;
    compile_rejects("localslot", out_slot, "E10035", 1);

    let single_candidate = r#"module spec.cpr.localcandidate;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func row(a: str): str[2] {
    let l: Box = Box { v: stringOf("ll") };
    return [l.v, a];
}

func store(): void { g.names = row("xy"); }

func main(): i32 { store(); return i32(len(g.names[1])) - 2; }
"#;
    compile_rejects_once_at(
        "localcandidate",
        single_candidate,
        "E10035",
        12,
        5,
        &["cannot return a view of local 'l'"],
        &[],
    );

    let deferred = r#"module spec.cpr.localdefer;

import { String, stringOf } from std.text;

struct Box { v: String; }

let g: str[2] = ["AA", "BB"];

func row(c: bool): str[2] {
    defer {
        if (c) {
            return ["xy", "zw"];
        }
    }
    let l: Box = Box { v: stringOf("ll") };
    return [l.v, "zw"];
}

func store(c: bool): void { g = row(c); }

func main(): i32 {
    store(false);
    return i32(g[0][0]) - 108;
}
"#;
    compile_rejects_once_at(
        "localdefer",
        deferred,
        "E10035",
        16,
        5,
        &["cannot return a view of local 'l'"],
        &[],
    );

    let deferred_fit = r#"module spec.cpr.localfitcontrol;

let s: str = "ab";
let t: str = "cd";

struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func row(): str[2] {
    defer { return [s, t]; }
    return ["xy", "zw"];
}

func store(): void { g.names = row(); }

func main(): i32 { store(); return i32(len(g.names[1])) - 2; }
"#;
    let output = run_memory("localfitcontrol", deferred_fit);
    assert_no_asan("localfitcontrol", &output);
    assert_exit("localfitcontrol", &output, 0);

    let static_fit = r#"module spec.cpr.localstaticcontrol;

let s: str = "ab";
let t: str = "cd";

struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func row(a: str): str[2] { return [s, t]; }

func store(): void { g.names = row("xy"); }

func main(): i32 { store(); return i32(len(g.names[1])) - 2; }
"#;
    let output = run_memory("localstaticcontrol", static_fit);
    assert_no_asan("localstaticcontrol", &output);
    assert_exit("localstaticcontrol", &output, 0);
}

// ---------------------------------------------------------------------------
// funcval: an escaping store through a callable value keeps its own report.
//
// A call through a `func`-typed binding, parameter, field, or capture is not
// the module function its name may share, so the declaration's elision
// judgment is never reached through it and the destination keeps the
// escaping-store report (`E10033`) when the value views a frame local. The
// parameter-backed callable spelling is the run control: its result views the
// caller's parameter, so it stays accepted, builds under the strict flags,
// and exits 0.
// ---------------------------------------------------------------------------

/// An escaping store through a callable value reports exactly one `E10033`.
///
/// The local binding, the parameter, the callable field, and the capture all
/// name a `func`-typed value, so the store of the local-backed result into
/// the module binding `g` reports one `E10033` at its own site. The
/// parameter-backed callable call is accepted and exits 0.
#[test]
fn should_keep_the_escaping_store_report_through_a_callable_value() {
    let local_value = r#"module spec.cpr.funclocal;

import { String, stringOf } from std.text;

let g: str = "gg";

func give(): str { return "mod"; }

func target(l: str): str { return l; }

func store(): void {
    let s: String = stringOf("ll");
    let l: str = s;
    let give: func(str): str = target;
    g = give(l);
}

func main(): i32 {
    store();
    return i32(g[0]) - 108;
}
"#;
    compile_rejects_once_at(
        "funclocal",
        local_value,
        "E10033",
        15,
        5,
        &["borrow of 'g' escapes its scope"],
        &[],
    );

    let parameter = r#"module spec.cpr.funcparam;

import { String, stringOf } from std.text;

struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func give(): str { return "mod"; }

func target(l: str): str { return l; }

func store(give: func(str): str): void {
    let s: String = stringOf("ll");
    let l: str = s;
    g.names[0] = give(l);
}

func main(): i32 {
    store(target);
    return i32(g.names[0][0]) - 108;
}
"#;
    compile_rejects_once_at(
        "funcparam",
        parameter,
        "E10033",
        16,
        5,
        &["borrow of 'g' escapes its scope"],
        &[],
    );

    let field = r#"module spec.cpr.funcfield;

import { String, stringOf } from std.text;

struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func give(): str { return "mod"; }

func target(l: str): str { return l; }

struct FnHolder { give: func(str): str; }

func store(): void {
    let s: String = stringOf("ll");
    let l: str = s;
    let h: FnHolder = FnHolder { give: target };
    g.names[0] = h.give(l);
}

func main(): i32 { return 0; }
"#;
    compile_rejects_once_at(
        "funcfield",
        field,
        "E10033",
        19,
        5,
        &["borrow of 'g' escapes its scope"],
        &[],
    );

    let capture = r#"module spec.cpr.funccap;

import { String, stringOf } from std.text;

let g: str = "gg";

func give(): str { return "mod"; }

func target(l: str): str { return l; }

func store(): void {
    let s: String = stringOf("ll");
    let l: str = s;
    let give: func(str): str = target;
    ([give, l] () => {
        g = give(l);
    })();
}

func main(): i32 {
    store();
    return i32(g[0]) - 108;
}
"#;
    compile_rejects_once_at(
        "funccap",
        capture,
        "E10033",
        16,
        9,
        &["borrow of 'g' escapes its scope"],
        &[],
    );

    let run_control = r#"module spec.cpr.funcrun;

import { String, stringOf } from std.text;

func target(l: str): str { return l; }

func use(p: str): i32 {
    let f: func(str): str = target;
    let s: str = f(p);
    return i32(len(s));
}

func main(): i32 {
    let s: String = stringOf("abcd");
    return use(s) - 4;
}
"#;
    let output = run_memory("funcrun", run_control);
    assert_no_asan("funcrun", &output);
    assert_exit("funcrun", &output, 0);
}

// ---------------------------------------------------------------------------
// rowlit: a static callee's view return stores into a global field.
//
// `rowlit()` returns an array literal, so its storage fits every caller and
// the declaration adds no return judgment; the store takes the value without
// a consequence report. The direct twin stores the same literal at the
// destination. Both are accepted, build under the strict flags, and exit 0.
// ---------------------------------------------------------------------------

/// A static callee's view-returning call stores into a global field and runs.
#[test]
fn should_accept_the_static_callee_store_without_a_consequence_report() {
    let callee = r#"module spec.cpr.rowlitaccept;

struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func rowlit(): str[2] { return ["AB", "CD"]; }

func store(): void { g.names = rowlit(); }

func main(): i32 { return i32(len(g.names[1])) - 2; }
"#;
    let output = run_memory("rowlitaccept", callee);
    assert_no_asan("rowlitaccept", &output);
    assert_exit("rowlitaccept", &output, 0);

    let direct = r#"module spec.cpr.rowlitdirect;

struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func store(): void { g.names = ["AB", "CD"]; }

func main(): i32 { return i32(len(g.names[1])) - 2; }
"#;
    let output = run_memory("rowlitdirect", direct);
    assert_no_asan("rowlitdirect", &output);
    assert_exit("rowlitdirect", &output, 0);
}

// ---------------------------------------------------------------------------
// usergenname: a user binding named like a generated owner keeps its name.
//
// The compiler's generated owners carry internal spellings (`__cl_ownedN`)
// that a source program may also use. A view of such a user binding escapes
// its scope at the binding's last use, and the report must name the binding
// the reader wrote (`'__cl_owned1'`) rather than the generated-owner phrase.
// ---------------------------------------------------------------------------

/// A user binding named like a generated owner renders its own written name.
#[test]
fn should_keep_the_written_name_of_a_binding_named_like_a_generated_owner() {
    let source = r#"module spec.cpr.usergenname;

import { String, stringOf } from std.text;

func use(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let out: str = "zz";
    {
        let __cl_owned1: String = stringOf("ab");
        let v: str = __cl_owned1;
        out = v;
    }
    return use(out) - 2;
}
"#;
    compile_rejects_once_at(
        "usergenname",
        source,
        "E10030",
        11,
        9,
        &["borrow of '__cl_owned1'", "does not live long enough"],
        &["a compiler-generated owner"],
    );
}

// ---------------------------------------------------------------------------
// capturepair: a borrowed element moved from a lifted lambda keeps the
// concrete twin's rejection.
//
// `let s: String = ([v] () => v.name)();` inside `each<T>` moves the
// borrowed element's member out of the capture, which the frozen capture
// rule rejects once; the concrete twin `each(xs: S[2])` reports the same
// single `E10401`, so the pair pins the substituted capture-move judgment
// and the concrete parity. (green control at the phase base: the
// substitution reaches the capture-move duty, so the invalid C is
// unreachable; a regression turns this case red.)
// ---------------------------------------------------------------------------

/// A lifted lambda's borrowed-element capture reports one `E10401` in both
/// spellings.
#[test]
fn should_reject_a_captured_borrowed_element_move_in_both_spellings() {
    let generic = r#"module spec.cc.capturepair;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        let s: String = ([v] () => v.name)();
        total = total + i32(len(bytes(s)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cde"), y: 2 }];
    return each<S[2]>(a);
}
"#;
    compile_rejects("capturepair", generic, "E10401", 1);

    let concrete = r#"module spec.cc.capturepairconcrete;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func each(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        let s: String = ([v] () => v.name)();
        total = total + i32(len(bytes(s)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cde"), y: 2 }];
    return each(a);
}
"#;
    compile_rejects("capturepairconcrete", concrete, "E10401", 1);
}

// ---------------------------------------------------------------------------
// viewcopyloans: a view-like array copy carries its element loans.
//
// `let ys: str[2] = xs;` (and the overwrite `ys = xs;`) copies an array whose
// element views loan a frame local. The copy must hand each element's loan to
// the copy, so the later `fill(ys[0], borrow slot)` stores a view whose owner
// dies with the frame into the caller's out-slot and reports the frozen escape
// `E10030`, exactly as the direct element argument and the single-view member
// twin already report it. A caller-backed source keeps its acceptance (the
// caller's box outlives the out-slot's read), the return variant keeps its
// `E10035`, the single-view twins keep their `E10030`/`E10031` reports, and the
// read-back copy stays accepted and memory-clean.
// ---------------------------------------------------------------------------

/// A whole-array copy of a view-like array carries its element loans.
///
/// The copy spellings — the `let` form over a frame local, the overwrite form,
/// the loop fill over the copied elements, and the `take`-parameter source —
/// each report exactly one `E10030` naming the frame owner whose buffer the
/// copied elements view and emit no translation unit. The plain-parameter
/// source and the read-back copy stay accepted and memory-clean; the return
/// variant keeps its one `E10035`; the single-view element twin keeps its one
/// backing-write `E10031`, and the direct member/element twins keep their one
/// `E10030`.
#[test]
fn should_loan_the_element_views_of_a_whole_array_copy() {
    let local = r#"module spec.cpr.viewcopyloans;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("viewcopyloans", local, "E10034", 1);

    let overwrite = r#"module spec.cpr.viewcopyloansassign;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = ["CC", "DD"];
    ys = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("viewcopyloansassign", overwrite, "E10034", 1);

    let looped = r#"module spec.cpr.viewcopyloansloop;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[2] = xs;
    for (borrow e in ys) {
        fill(e, borrow slot);
    }
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("viewcopyloansloop", looped, "E10034", 1);

    let take_source = r#"module spec.cpr.viewcopyloanstake;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(take p: Box, borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    xs[0] = p.v;
    let ys: str[2] = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("viewcopyloanstake", take_source, "E10034", 1);

    let parameter = r#"module spec.cpr.viewcopyloansparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    xs[0] = p.v;
    let ys: str[2] = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("viewcopyloansparam", parameter, "E10034", 1);

    let returned = r#"module spec.cpr.viewcopyloansreturn;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): str[2] {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = xs;
    return ys;
}

func main(): i32 {
    return i32(run()[0][0]) - 97;
}
"#;
    compile_rejects("viewcopyloansreturn", returned, "E10035", 1);

    let backing = r#"module spec.cpr.viewcopyloansbacking;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    a.v = stringOf("qq");
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects_codes(
        "viewcopyloansbacking",
        backing,
        &[("E10034", 7, 46), ("E10031", 14, 5)],
    );

    let member_twin = r#"module spec.cpr.viewcopyloansmember;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let s: str = a.v;
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("viewcopyloansmember", member_twin, "E10034", 1);

    let element_twin = r#"module spec.cpr.viewcopyloanselem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(xs[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects("viewcopyloanselem", element_twin, "E10034", 1);

    let read_back = r#"module spec.cpr.viewcopyloansread;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = xs;
    return i32(ys[0][0]) - 97 + i32(ys[1][0]) - 66;
}

func main(): i32 { return outer(); }
"#;
    let output = run_memory("viewcopyloansread", read_back);
    assert_no_asan("viewcopyloansread", &output);
    assert_exit("viewcopyloansread", &output, 0);
}

// ---------------------------------------------------------------------------
// rawreadowner: an owned raw deref read moves its pointee out.
//
// `let v: String = unsafe *q;` is an owned read of the raw pointee, so it
// takes the ordinary owned read's judgment: the value moves into `v` and the
// pointee is emptied through its own address, leaving exactly one owner (the
// run reads the value back and is memory-clean). The discarded read creates no
// owner, the ordinary named read is the model, and a duplicable pointee
// copies, so the source stays readable.
// ---------------------------------------------------------------------------

/// An owned raw deref read moves out of its pointee like the named read.
///
/// `let v: String = unsafe *q;` binds the pointee's buffer to `v` and empties
/// the pointee, so the run reads the moved value back (exit 2) with no memory
/// report. The discarded read (`unsafe { *q; }`) and the duplicable-pointee
/// read stay accepted and clean, and the named ordinary read keeps its own
/// move (exit 2).
#[test]
fn should_move_the_owned_raw_deref_read_out_of_its_pointee() {
    let owned = r#"module spec.cpr.rawreadowner;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String = stringOf("aa");
    let q: raw String = unsafe &a;
    let v: String = unsafe *q;
    return i32(len(v));
}
"#;
    let output = run_memory("rawreadowner", owned);
    assert_no_asan("rawreadowner", &output);
    assert_exit("rawreadowner", &output, 2);

    let discarded = r#"module spec.cpr.rawreadownerdiscard;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String = stringOf("aa");
    let q: raw String = unsafe &a;
    unsafe { *q; }
    return i32(len(a));
}
"#;
    let output = run_memory("rawreadownerdiscard", discarded);
    assert_no_asan("rawreadownerdiscard", &output);
    assert_exit("rawreadownerdiscard", &output, 2);

    let duplicable = r#"module spec.cpr.rawreadownerduplicable;

func main(): i32 {
    let a: i32 = 5;
    let q: raw i32 = unsafe &a;
    let v: i32 = unsafe *q;
    return (v - 5) + (a - 5);
}
"#;
    let output = run_memory("rawreadownerduplicable", duplicable);
    assert_no_asan("rawreadownerduplicable", &output);
    assert_exit("rawreadownerduplicable", &output, 0);

    let named = r#"module spec.cpr.rawreadownernamed;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String = stringOf("aa");
    let v: String = a;
    return i32(len(v));
}
"#;
    let output = run_memory("rawreadownernamed", named);
    assert_no_asan("rawreadownernamed", &output);
    assert_exit("rawreadownernamed", &output, 2);
}

// ---------------------------------------------------------------------------
// owningtake: a conditional take of an opaque member moves out per instance.
//
// `eat(take (c ? a.name : b.name))` inside a generic `pick<T>` takes the
// chosen arm's owned member out of its root. The `T`-typed member is a type
// parameter, so the move-out duty must be recorded per instantiation: the
// chosen root empties the member and the un-taken arm keeps its owner, so
// every buffer releases exactly once (the concrete twin's behavior). The
// direct (non-conditional) local take and the duplicable member stay accepted
// as controls.
// ---------------------------------------------------------------------------

/// A conditional take of an opaque member empties the chosen arm's root.
///
/// The parameter-arm conditional, the false selection, and the conditional
/// over local roots each run with the selected member moved out and every
/// owner released exactly once (exit 0, no memory report). The concrete
/// conditional twins and the direct local take stay accepted and clean, and
/// the duplicable-member instance keeps its copy.
#[test]
fn should_move_the_taken_opaque_conditional_member_out_at_the_instance() {
    let conditional = r#"module spec.cpr.owningtake;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func pick<T>(take a: T, take b: T, c: bool): i32 {
    return eat(take (c ? a.name : b.name));
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pick<S>(take a, take b, true) - 1;
}
"#;
    let output = run_memory("owningtake", conditional);
    assert_no_asan("owningtake", &output);
    assert_exit("owningtake", &output, 0);

    let false_selection = r#"module spec.cpr.owningtakefalse;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func pick<T>(take a: T, take b: T, c: bool): i32 {
    return eat(c ? a.name : b.name);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pick<S>(take a, take b, false) - 1;
}
"#;
    let output = run_memory("owningtakefalse", false_selection);
    assert_no_asan("owningtakefalse", &output);
    assert_exit("owningtakefalse", &output, 0);

    let local_conditional = r#"module spec.cpr.owningtakelocal;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func pick<T>(take a: T, take b: T, c: bool): i32 {
    let x: T = a;
    let y: T = b;
    return eat(c ? x.name : y.name);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pick<S>(take a, take b, true) - 1;
}
"#;
    let output = run_memory("owningtakelocal", local_conditional);
    assert_no_asan("owningtakelocal", &output);
    assert_exit("owningtakelocal", &output, 0);

    let concrete = r#"module spec.cpr.owningtakeconcrete;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eatS(take x: String): i32 { return 1; }

func pickS(take a: S, take b: S, c: bool): i32 {
    return eatS(c ? a.name : b.name);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pickS(take a, take b, true) - 1;
}
"#;
    let output = run_memory("owningtakeconcrete", concrete);
    assert_no_asan("owningtakeconcrete", &output);
    assert_exit("owningtakeconcrete", &output, 0);

    let local_concrete = r#"module spec.cpr.owningtakelocalconcrete;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eatS(take x: String): i32 { return 1; }

func pickS(take a: S, take b: S, c: bool): i32 {
    let x: S = a;
    let y: S = b;
    return eatS(c ? x.name : y.name);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pickS(take a, take b, true) - 1;
}
"#;
    let output = run_memory("owningtakelocalconcrete", local_concrete);
    assert_no_asan("owningtakelocalconcrete", &output);
    assert_exit("owningtakelocalconcrete", &output, 0);

    let direct = r#"module spec.cpr.owningtakelocaldirect;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func pick<T>(take a: T): i32 {
    let b: T = a;
    return eat(b.name);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    return pick<S>(take a) - 1;
}
"#;
    let output = run_memory("owningtakelocaldirect", direct);
    assert_no_asan("owningtakelocaldirect", &output);
    assert_exit("owningtakelocaldirect", &output, 0);

    let duplicable = r#"module spec.cpr.owningtakeduplicable;

struct S { name: i32; y: i32; }

let hits: i32 = 0;

func eat<T>(take x: T): i32 {
    hits = hits + 1;
    return 1;
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + eat(v.y);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: 1, y: 1 }, S { name: 2, y: 2 }];
    return (each<S[2]>(a) - 2) + (hits - 2);
}
"#;
    let output = run_memory("owningtakeduplicable", duplicable);
    assert_no_asan("owningtakeduplicable", &output);
    assert_exit("owningtakeduplicable", &output, 0);
}

// ---------------------------------------------------------------------------
// borrowelem: an owned member of a borrowed element keeps the partial-move
// judgment per instance.
//
// `v.row = w.row;` moves an owned fixed-array member out of a borrowed
// element, and `eat(v.name)` takes an owned member of a borrowed element. A
// borrowed element is neither an owned local nor a `take` parameter, so the
// move is the v1 partial-move limit `E10026` in the concrete spelling; the
// generic instance must report the same one error instead of emitting invalid
// C or accepting the moved-out member. The `let` spelling and the concrete
// twins already report it and stay as controls.
// ---------------------------------------------------------------------------

/// A borrowed element's owned member move reports the partial-move limit once
/// per instance.
///
/// Both generic row-copy spellings (one array, two arrays) and the generic
/// member take report exactly one `E10026` and emit no translation unit, the
/// same error their concrete twins report. The generic and concrete member
/// `let` spellings keep their one report.
#[test]
fn should_judge_a_borrowed_element_member_move_per_instance() {
    let rowcopy = r#"module spec.cpr.borrowelemstore;

import { String, stringOf, bytes } from std.text;

struct R { row: String[2]; }

func eachCopy<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        for (borrow w in xs) {
            v.row = w.row;
        }
        total = total + i32(len(bytes(v.row[0]))) + i32(len(bytes(v.row[1])));
    }
    return total;
}

func main(): i32 {
    let a: R[2] = [R { row: [stringOf("a"), stringOf("b")] },
                   R { row: [stringOf("cc"), stringOf("dd")] }];
    return eachCopy<R[2]>(a);
}
"#;
    compile_rejects("borrowelemstore", rowcopy, "E10026", 1);

    let rowcopy_two = r#"module spec.cpr.borrowelemstoretwo;

import { String, stringOf, bytes } from std.text;

struct R { row: String[2]; }

func eachCopy2<T>(xs: T, ys: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        for (borrow w in ys) {
            v.row = w.row;
        }
        total = total + i32(len(bytes(v.row[0]))) + i32(len(bytes(v.row[1])));
    }
    return total;
}

func main(): i32 {
    let a: R[2] = [R { row: [stringOf("a"), stringOf("b")] },
                   R { row: [stringOf("cc"), stringOf("dd")] }];
    let b: R[2] = [R { row: [stringOf("e"), stringOf("f")] },
                   R { row: [stringOf("gg"), stringOf("hh")] }];
    return eachCopy2<R[2]>(a, b);
}
"#;
    compile_rejects("borrowelemstoretwo", rowcopy_two, "E10026", 1);

    let rowcopy_concrete = r#"module spec.cpr.borrowelemstoreconcrete;

import { String, stringOf, bytes } from std.text;

struct R { row: String[2]; }

func eachCopy(xs: R[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        for (borrow w in xs) {
            v.row = w.row;
        }
        total = total + i32(len(bytes(v.row[0]))) + i32(len(bytes(v.row[1])));
    }
    return total;
}

func main(): i32 {
    let a: R[2] = [R { row: [stringOf("a"), stringOf("b")] },
                   R { row: [stringOf("cc"), stringOf("dd")] }];
    return eachCopy(a);
}
"#;
    compile_rejects("borrowelemstoreconcrete", rowcopy_concrete, "E10026", 1);

    let take = r#"module spec.cpr.borrowelemtake;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func each<T>(xs: T): void {
    for (borrow v in xs) {
        eat(v.name);
        v.name = stringOf("zzzz");
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    each<S[1]>(a);
    return 0;
}
"#;
    compile_rejects("borrowelemtake", take, "E10026", 1);

    let take_concrete = r#"module spec.cpr.borrowelemtakeconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func eachS(xs: S[1]): void {
    for (borrow v in xs) {
        eat(v.name);
        v.name = stringOf("zzzz");
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    eachS(a);
    return 0;
}
"#;
    compile_rejects("borrowelemtakeconcrete", take_concrete, "E10026", 1);

    let member_let = r#"module spec.cpr.borrowelemlet;

import { String, stringOf } from std.text;

struct S { name: String; }

func each<T>(xs: T): void {
    for (borrow v in xs) {
        let s: String = v.name;
        v.name = stringOf("zzzz");
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    each<S[1]>(a);
    return 0;
}
"#;
    compile_rejects("borrowelemlet", member_let, "E10026", 1);

    let member_let_concrete = r#"module spec.cpr.borrowelemletconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func eachS(xs: S[1]): void {
    for (borrow v in xs) {
        let s: String = v.name;
        v.name = stringOf("zzzz");
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    eachS(a);
    return 0;
}
"#;
    compile_rejects("borrowelemletconcrete", member_let_concrete, "E10026", 1);
}

// ---------------------------------------------------------------------------
// The borrowed element's move judgment reaches every owning destination. A
// move of an element member into a `let`, a return, an assignment target, or
// a conditional arm is the v1 element-move limit once the instance's member
// type owns, exactly as each concrete twin reports it; a duplicable instance
// keeps its copy.
// ---------------------------------------------------------------------------

/// The member move into a `let` destination.
const ELEMENT_DEST_LET_SOURCE: &str = r#"module spec.cpr.elemdests;

import { String, stringOf } from std.text;

struct S { name: String; }

func each<T, E>(xs: T): void {
    for (borrow v in xs) {
        let t: E = v.name;
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    each<S[1], String>(a);
    return 0;
}
"#;

/// The concrete `let` twin of the element move.
const ELEMENT_DEST_LET_CONCRETE_SOURCE: &str = r#"module spec.cpr.elemdestsconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func eachS(xs: S[1]): void {
    for (borrow v in xs) {
        let t: String = v.name;
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    eachS(a);
    return 0;
}
"#;

/// The member move into a return.
const ELEMENT_DEST_RET_SOURCE: &str = r#"module spec.cpr.elemdestret;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func pick<C, E>(xs: C, take fallback: E): E {
    for (borrow v in xs) {
        return v.name;
    }
    return fallback;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab") }];
    let out: String = pick<S[1], String>(a, stringOf("z"));
    return i32(len(bytes(out))) - 2;
}
"#;

/// The concrete return twin of the element move.
const ELEMENT_DEST_RET_CONCRETE_SOURCE: &str = r#"module spec.cpr.elemdestretconcrete;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func pickS(xs: S[1], take fallback: String): String {
    for (borrow v in xs) {
        return v.name;
    }
    return fallback;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab") }];
    let out: String = pickS(a, stringOf("z"));
    return i32(len(bytes(out))) - 2;
}
"#;

/// The member move into an assignment target.
const ELEMENT_DEST_ASSIGN_SOURCE: &str = r#"module spec.cpr.elemdestassign;

import { String, stringOf } from std.text;

struct S { name: String; }

func each<T, E>(xs: T, take seed: E): void {
    let slot: E = seed;
    for (borrow v in xs) {
        slot = v.name;
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    each<S[1], String>(a, stringOf("z"));
    return 0;
}
"#;

/// The concrete assignment twin of the element move.
const ELEMENT_DEST_ASSIGN_CONCRETE_SOURCE: &str = r#"module spec.cpr.elemdestassignconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func eachS(xs: S[1], take seed: String): void {
    let slot: String = seed;
    for (borrow v in xs) {
        slot = v.name;
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    eachS(a, stringOf("z"));
    return 0;
}
"#;

/// The member move of each arm of a conditional take.
const ELEMENT_DEST_COND_SOURCE: &str = r#"module spec.cpr.elemdestcond;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat<T>(take x: T): i32 { return 1; }

func each<T>(xs: T, ys: T, c: bool): i32 {
    for (borrow v in xs) {
        for (borrow w in ys) {
            return eat(take (c ? v.name : w.name));
        }
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    let b: S[1] = [S { name: stringOf("b") }];
    return each<S[1]>(a, b, true) - 1;
}
"#;

/// The concrete conditional twin of the element move.
const ELEMENT_DEST_COND_CONCRETE_SOURCE: &str = r#"module spec.cpr.elemdestcondconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func eatS(take x: String): i32 { return 1; }

func eachS(xs: S[1], ys: S[1], c: bool): i32 {
    for (borrow v in xs) {
        for (borrow w in ys) {
            return eatS(c ? v.name : w.name);
        }
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    let b: S[1] = [S { name: stringOf("b") }];
    return eachS(a, b, true) - 1;
}
"#;

/// The duplicable member twin of the element move.
const ELEMENT_DEST_DUP_SOURCE: &str = r#"module spec.cpr.elemdestsdup;

struct D { name: i32; }

func eat(take x: i32): i32 { return x; }

func pick<T, E>(xs: T, take seed: E, c: bool): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        let t: E = v.name;
        let slot: E = seed;
        slot = v.name;
        total = total + i32(t) + i32(slot);
    }
    return total + eat(c ? 3 : 4);
}

func main(): i32 {
    let a: D[1] = [D { name: 2 }];
    return pick<D[1], i32>(a, 5, true) - 7;
}
"#;

/// The duplicable conditional twin at borrowed-element roots.
const ELEMENT_DEST_COND_DUP_SOURCE: &str = r#"module spec.cpr.elemdestconddup;

struct D { name: i32; }

func eat<T>(take x: T): i32 { return 1; }

func each<T>(xs: T, ys: T, c: bool): i32 {
    for (borrow v in xs) {
        for (borrow w in ys) {
            return eat(take (c ? v.name : w.name));
        }
    }
    return 0;
}

func main(): i32 {
    let a: D[1] = [D { name: 1 }];
    let b: D[1] = [D { name: 2 }];
    return each<D[1]>(a, b, true) - 1;
}
"#;

/// The `let`, the return, the assignment, and the conditional take each report
/// the v1 element-move limit once per moved member at their own site, exactly
/// as their concrete twins do; the conditional take names both arms, so it
/// reports once per arm.
#[test]
fn should_judge_a_borrowed_element_move_at_every_owning_destination() {
    compile_rejects("elemdests", ELEMENT_DEST_LET_SOURCE, "E10026", 1);
    compile_rejects(
        "elemdestsconcrete",
        ELEMENT_DEST_LET_CONCRETE_SOURCE,
        "E10026",
        1,
    );
    compile_rejects_once_at(
        "elemdestret",
        ELEMENT_DEST_RET_SOURCE,
        "E10026",
        9,
        9,
        &["partial moves are limited to top-level fields in v1"],
        &[],
    );
    compile_rejects_once_at(
        "elemdestretconcrete",
        ELEMENT_DEST_RET_CONCRETE_SOURCE,
        "E10026",
        9,
        9,
        &["partial moves are limited to top-level fields in v1"],
        &[],
    );
    compile_rejects_once_at(
        "elemdestassign",
        ELEMENT_DEST_ASSIGN_SOURCE,
        "E10026",
        10,
        9,
        &["partial moves are limited to top-level fields in v1"],
        &[],
    );
    compile_rejects_once_at(
        "elemdestassignconcrete",
        ELEMENT_DEST_ASSIGN_CONCRETE_SOURCE,
        "E10026",
        10,
        9,
        &["partial moves are limited to top-level fields in v1"],
        &[],
    );
    compile_rejects("elemdestcond", ELEMENT_DEST_COND_SOURCE, "E10026", 2);
    compile_rejects(
        "elemdestcondconcrete",
        ELEMENT_DEST_COND_CONCRETE_SOURCE,
        "E10026",
        2,
    );
}

/// A duplicable member copies into a `let`, an assignment, and the guard-free
/// take, so the same destinations stay accepted and the run reads the values
/// back (exit 0) with the leak checker clean; the conditional take at
/// borrowed-element roots is the same copy.
#[test]
fn should_copy_a_borrowed_element_move_into_a_duplicable_destination() {
    let output = run_memory("elemdestsdup", ELEMENT_DEST_DUP_SOURCE);
    assert_no_asan("elemdestsdup", &output);
    assert_exit("elemdestsdup", &output, 0);

    let output = run_memory("elemdestconddup", ELEMENT_DEST_COND_DUP_SOURCE);
    assert_no_asan("elemdestconddup", &output);
    assert_exit("elemdestconddup", &output, 0);
}

/// The whole-binding use after a conditional member take.
const ELEMENT_DEST_PARTIAL_SOURCE: &str = r#"module spec.cpr.elemdestpartial;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func peek<T>(x: T): i32 { return 1; }

func pick<T>(take a: T, take b: T, c: bool): i32 {
    let x: T = a;
    let y: T = b;
    let r: i32 = eat(c ? x.name : y.name);
    return r + peek(y);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pick<S>(take a, take b, true) - 2;
}
"#;

/// The concrete twin of the whole-binding use.
const ELEMENT_DEST_PARTIAL_CONCRETE_SOURCE: &str = r#"module spec.cpr.elemdestpartialconcrete;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eatS(take x: String): i32 { return 1; }

func peekS(x: S): i32 { return 1; }

func pickS(take a: S, take b: S, c: bool): i32 {
    let x: S = a;
    let y: S = b;
    let r: i32 = eatS(c ? x.name : y.name);
    return r + peekS(y);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pickS(take a, take b, true) - 2;
}
"#;

/// A whole-binding use after a member move reports the concrete spelling's
/// partial-move code once at the use: the conditional take moved only `y.name`,
/// so reading the whole `y` afterwards is the partial-move judgment, exactly as
/// the concrete twin reports it.
#[test]
fn should_report_the_partial_move_code_for_a_whole_use_after_a_member_move() {
    compile_rejects_once_at(
        "elemdestpartial",
        ELEMENT_DEST_PARTIAL_SOURCE,
        "E10025",
        15,
        21,
        &["is partially moved and cannot be used as a whole"],
        &[],
    );
    compile_rejects_once_at(
        "elemdestpartialconcrete",
        ELEMENT_DEST_PARTIAL_CONCRETE_SOURCE,
        "E10025",
        15,
        22,
        &["is partially moved and cannot be used as a whole"],
        &[],
    );
}

// ---------------------------------------------------------------------------
// A whole-array copy from a projected source. `let ys: str[2] = rows[0];` and
// the assignment spelling hand the source row's element loans to the
// destination, so the store through the copy reports the frozen conflict once;
// the element store, the single-view spelling, and a copy with no later use
// keep their own verdicts.
// ---------------------------------------------------------------------------

/// A whole-`str[2]` copy whose source is a row of a `str[2][2]` container.
const VIEW_COPY_ROW_LET_SOURCE: &str = r#"module spec.cpr.viewcopyrowlet;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["AA", "AA"], ["BB", "BB"]];
    rows[0] = xs;
    let ys: str[2] = rows[0];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The overwriting-assignment spelling of the projected-source copy.
const VIEW_COPY_ROW_ASSIGN_SOURCE: &str = r#"module spec.cpr.viewcopyrowassign;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["AA", "AA"], ["BB", "BB"]];
    rows[0] = xs;
    let ys: str[2] = ["CC", "DD"];
    ys = rows[0];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The element-store route into the same row, which already transfers the loan.
const VIEW_COPY_ROW_ELEMENT_SOURCE: &str = r#"module spec.cpr.viewcopyrowelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["AA", "AA"], ["BB", "BB"]];
    rows[0] = xs;
    fill(rows[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The single-view spelling of the same row element.
const VIEW_COPY_ROW_VIEW_SOURCE: &str = r#"module spec.cpr.viewcopyrowview;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    a.v = stringOf("qq");
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The projected-source copy whose elements are never read afterwards.
const VIEW_COPY_ROW_UNUSED_SOURCE: &str = r#"module spec.cpr.viewcopyrowunused;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func outer(): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["AA", "AA"], ["BB", "BB"]];
    rows[0] = xs;
    let ys: str[2] = rows[0];
    a.v = stringOf("qq");
}

func main(): i32 {
    outer();
    return 0;
}
"#;

/// The projected-source copy hands the row's element loan to the destination,
/// so the store through the copy reports the frozen conflict once at its own
/// site; the element-store route reports the same, while the single-view twin
/// keeps its one backing-write report and a copy with no later use stays
/// accepted and clean.
#[test]
fn should_loan_a_whole_array_copy_from_a_projected_source() {
        compile_rejects_once_at(
        "viewcopyrowlet",
        VIEW_COPY_ROW_LET_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "viewcopyrowassign",
        VIEW_COPY_ROW_ASSIGN_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "viewcopyrowelem",
        VIEW_COPY_ROW_ELEMENT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    compile_rejects_codes(
        "viewcopyrowview",
        VIEW_COPY_ROW_VIEW_SOURCE,
        &[("E10034", 7, 46), ("E10031", 14, 5)],
    );
    let output = run_memory("viewcopyrowunused", VIEW_COPY_ROW_UNUSED_SOURCE);
    assert_no_asan("viewcopyrowunused", &output);
    assert_exit("viewcopyrowunused", &output, 0);
}

// ---------------------------------------------------------------------------
// The guarded conditional take. `d && eat(take (c ? a.name : b.name))` moves
// the selected member out of its root on the evaluated path only: each
// selection runs clean, the values release once, and a skipped guard leaves
// both roots owned.
// ---------------------------------------------------------------------------

/// The guarded conditional take with every condition selection.
const GUARDED_CONDITIONAL_TAKE_SOURCE: &str = r#"module spec.cpr.guardcondboth;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func pick<T>(take a: T, take b: T, c: bool, d: bool): i32 {
    if (d && (eat(take (c ? a.name : b.name)) == 1)) { return 1; }
    return 0;
}

func main(): i32 {
    let a1: S = S { name: stringOf("ab"), y: 1 };
    let b1: S = S { name: stringOf("cd"), y: 2 };
    let a2: S = S { name: stringOf("ef"), y: 3 };
    let b2: S = S { name: stringOf("gh"), y: 4 };
    let a3: S = S { name: stringOf("ij"), y: 5 };
    let b3: S = S { name: stringOf("kl"), y: 6 };
    return (pick<S>(take a1, take b1, true, true) - 1)
         + (pick<S>(take a2, take b2, false, true) - 1)
         + (pick<S>(take a3, take b3, true, false) - 0);
}
"#;

/// The concrete guarded conditional twin.
const GUARDED_CONDITIONAL_TAKE_CONCRETE_SOURCE: &str = r#"module spec.cpr.guardcondconcrete;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eatS(take x: String): i32 { return 1; }

func pickS(take a: S, take b: S, c: bool, d: bool): i32 {
    if (d && (eatS(c ? a.name : b.name) == 1)) { return 1; }
    return 0;
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pickS(take a, take b, true, true) - 1;
}
"#;

/// The duplicable guarded conditional twin.
const GUARDED_CONDITIONAL_TAKE_DUP_SOURCE: &str = r#"module spec.cpr.guardconddup;

struct D { name: i32; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func pick<T>(take a: T, take b: T, c: bool): i32 {
    return eat(take (c ? a.name : b.name));
}

func main(): i32 {
    let a: D = D { name: 1, y: 1 };
    let b: D = D { name: 2, y: 2 };
    return pick<D>(take a, take b, true) - 1;
}
"#;

/// The guarded conditional take moves the chosen member on the evaluated path
/// only: both selections and the skipped guard run clean (exit 0) with the
/// leak checker clean, and the concrete and duplicable twins stay clean too.
#[test]
fn should_move_a_guarded_conditional_take_member_per_arm() {
    let output = run_memory("guardcondboth", GUARDED_CONDITIONAL_TAKE_SOURCE);
    assert_no_asan("guardcondboth", &output);
    assert_exit("guardcondboth", &output, 0);

    let output = run_memory(
        "guardcondconcrete",
        GUARDED_CONDITIONAL_TAKE_CONCRETE_SOURCE,
    );
    assert_no_asan("guardcondconcrete", &output);
    assert_exit("guardcondconcrete", &output, 0);

    let output = run_memory("guardconddup", GUARDED_CONDITIONAL_TAKE_DUP_SOURCE);
    assert_no_asan("guardconddup", &output);
    assert_exit("guardconddup", &output, 0);
}

// ---------------------------------------------------------------------------
// The owned raw-deref read at every owned position. The return, the `take`
// argument, and the struct field each empty the pointee once the value has
// moved, so a fresh write through the pointer lands without releasing the
// moved value; the array-element spelling keeps its own clean shape.
// ---------------------------------------------------------------------------

/// The deref read returned from a callee.
const DEREF_READ_RETURN_SOURCE: &str = r#"module spec.cpr.derefretread;

import { String, stringOf, bytes } from std.text;

func takeOut(q: raw String): String {
    return unsafe *q;
}

func main(): i32 {
    let a: String = stringOf("ab");
    let q: raw String = unsafe &a;
    let v: String = takeOut(q);
    unsafe { *q = stringOf("zz"); }
    return i32(len(bytes(v))) - 2 + i32(len(bytes(a))) - 2;
}
"#;

/// The deref read passed to a `take` parameter.
const DEREF_READ_TAKE_SOURCE: &str = r#"module spec.cpr.dereftakeread;

import { String, stringOf, bytes } from std.text;

func eat(take s: String): i32 { return i32(len(bytes(s))); }

func main(): i32 {
    let a: String = stringOf("ab");
    let q: raw String = unsafe &a;
    let n: i32 = eat(unsafe *q);
    unsafe { *q = stringOf("zz"); }
    return n - 2 + i32(len(bytes(a))) - 2;
}
"#;

/// The deref read stored into a struct field.
const DEREF_READ_FIELD_SOURCE: &str = r#"module spec.cpr.dereffieldread;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func main(): i32 {
    let a: String = stringOf("ab");
    let q: raw String = unsafe &a;
    let s: S = S { name: unsafe *q };
    unsafe { *q = stringOf("zz"); }
    return i32(len(bytes(s.name))) - 2 + i32(len(bytes(a))) - 2;
}
"#;

/// The deref read stored into an array element.
const DEREF_READ_ELEMENT_SOURCE: &str = r#"module spec.cpr.derefelemread;

import { String, stringOf, bytes } from std.text;

func main(): i32 {
    let a: String = stringOf("ab");
    let q: raw String = unsafe &a;
    let v: String[1] = [unsafe *q];
    unsafe { *q = stringOf("zz"); }
    return i32(len(bytes(v[0]))) - 2 + i32(len(bytes(a))) - 2;
}
"#;

/// Every owned position that consumes a raw-deref read empties its pointee, so
/// the moved value keeps one owner: a fresh write through the pointer lands
/// and both values read back (exit 0) with the leak checker clean. The
/// array-element spelling is the unchanged control.
#[test]
fn should_empty_a_deref_read_at_every_owned_position() {
    let output = run_memory("derefretread", DEREF_READ_RETURN_SOURCE);
    assert_no_asan("derefretread", &output);
    assert_exit("derefretread", &output, 0);

    let output = run_memory("dereftakeread", DEREF_READ_TAKE_SOURCE);
    assert_no_asan("dereftakeread", &output);
    assert_exit("dereftakeread", &output, 0);

    let output = run_memory("dereffieldread", DEREF_READ_FIELD_SOURCE);
    assert_no_asan("dereffieldread", &output);
    assert_exit("dereffieldread", &output, 0);

    let output = run_memory("derefelemread", DEREF_READ_ELEMENT_SOURCE);
    assert_no_asan("derefelemread", &output);
    assert_exit("derefelemread", &output, 0);
}

// ---------------------------------------------------------------------------
// condbasestore: the element store through a conditional base.
//
// A conditional is not a place, so `(c ? b1 : b2).ys[i] = ...` cannot be the
// target of a store: the frozen place rule (a local, a parameter, a struct
// field, an array element, or a dereferenced raw pointer) reports one `E10221`
// and leaves no translation unit, exactly as it rejects the scalar member twin
// `(c ? b1 : b2).x = ...`. Because a rejected program never builds, the store
// is never silently dropped and its side-effecting right-hand side never runs.
// The read spelling needs no place and stays accepted; a real place (a local
// element and a field chain) keeps storing, with its right-hand side running
// exactly once per store.
// ---------------------------------------------------------------------------

/// An element store through a conditional base is rejected on both selections,
/// while its read spelling and the place-store forms stay accepted.
///
/// `(c ? b1 : b2).ys[i] = bump();` targets a conditional, which the frozen
/// place rule does not list, so the assignment reports exactly one `E10221`
/// at the target and emits no translation unit; the side-effecting right-hand
/// side never runs. The scalar member twin `(c ? b1 : b2).x = 7;` is rejected
/// the same way. The read spelling reads the selected element on each
/// selection; the accepted local and projected place stores write their values
/// back with `bump` running once per store, so every accepted control exits 0.
#[test]
fn should_reject_an_element_store_through_a_conditional_base() {
    let taken = r#"module spec.cpr.condbasestore;

struct Box { ys: i32[3]; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let b2: Box = Box { ys: [4, 5, 6] };
    let c: bool = true;
    let i: i32 = 1;
    (c ? b1 : b2).ys[i] = bump();
    return (b1.ys[1] - 7) + (b2.ys[1] - 5) + calls;
}
"#;
    compile_rejects_once_at(
        "condbasestore",
        taken,
        "E10221",
        17,
        5,
        &[
            "left side of an assignment must be a place",
            "assign to a variable, field, or element",
        ],
        &[],
    );

    let other = r#"module spec.cpr.condbasestoreother;

struct Box { ys: i32[3]; }

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let b2: Box = Box { ys: [4, 5, 6] };
    let c: bool = false;
    let i: i32 = 1;
    (c ? b1 : b2).ys[i] = 7;
    return (b2.ys[1] - 7) + (b1.ys[1] - 2);
}
"#;
    compile_rejects_once_at(
        "condbasestoreother",
        other,
        "E10221",
        10,
        5,
        &["left side of an assignment must be a place"],
        &[],
    );

    let read = r#"module spec.cpr.condbasestoreread;

struct Box { ys: i32[3]; }

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let b2: Box = Box { ys: [4, 5, 6] };
    let c: bool = true;
    let d: bool = false;
    let i: i32 = 1;
    let v1: i32 = (c ? b1 : b2).ys[i];
    let v2: i32 = (d ? b1 : b2).ys[i];
    return (v1 - 2) + (v2 - 5) + (b1.ys[1] - 2) + (b2.ys[1] - 5);
}
"#;
    let output = run_memory("condbasestoreread", read);
    assert_no_asan("condbasestoreread", &output);
    assert_exit("condbasestoreread", &output, 0);

    let place = r#"module spec.cpr.condbaseplace;

struct Box { ys: i32[3]; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let i: i32 = 1;
    b1.ys[i] = bump();
    b1.ys[0] = bump() + 1;
    return (b1.ys[1] - 7) + (b1.ys[0] - 8) + calls - 2;
}
"#;
    let output = run_memory("condbaseplace", place);
    assert_no_asan("condbaseplace", &output);
    assert_exit("condbaseplace", &output, 0);

    let projected = r#"module spec.cpr.condbaseproj;

struct Box { ys: i32[3]; }
struct Holder { b: Box; }

func main(): i32 {
    let h: Holder = Holder { b: Box { ys: [1, 2, 3] } };
    let i: i32 = 1;
    h.b.ys[i] = 7;
    return h.b.ys[1] - 7;
}
"#;
    let output = run_memory("condbaseproj", projected);
    assert_no_asan("condbaseproj", &output);
    assert_exit("condbaseproj", &output, 0);

    let scalar = r#"module spec.cpr.condbasescalar;

struct Box { x: i32; ys: i32[3]; }

func main(): i32 {
    let b1: Box = Box { x: 1, ys: [1, 2, 3] };
    let b2: Box = Box { x: 4, ys: [4, 5, 6] };
    let c: bool = true;
    (c ? b1 : b2).x = 7;
    return (b1.x - 7) + (b2.x - 4);
}
"#;
    compile_rejects_once_at(
        "condbasescalar",
        scalar,
        "E10221",
        9,
        5,
        &["left side of an assignment must be a place"],
        &[],
    );
}

// ---------------------------------------------------------------------------
// guardviewliteral: the guarded conditional of call-filled view-array literals.
//
// `g && consume(ok ? Mixed { names: n1(), ... } : Mixed { names: n2(), ... })`
// builds the selected literal's view-array field from the selected call. The
// field is filled through the call's hidden destination, so the fill must sit
// on the path that evaluates it: a skipped guard runs no call, and a taken path
// runs exactly the selected arm's call. Both the `take` and the plain parameter
// spelling build under the strict flags; the direct-literal take and the scalar
// and owned field twins stay accepted controls.
// ---------------------------------------------------------------------------

/// A guarded conditional of call-filled view-array literals runs only the
/// taken arm's call, on both parameter spellings.
///
/// The two taken evaluations run `n1` once and `n2` once (the selected arm's
/// field byte reads back as `A` and `C`) and the skipped guard runs no call at
/// all, so the counter and byte read-backs cancel to exit 0. The direct
/// literal take and the scalar and owned field twins run their own counted
/// shapes unchanged.
#[test]
fn should_run_only_the_taken_arm_of_a_guarded_view_array_literal() {
    let take = r#"module spec.cpr.guardviewliteral;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    calls = calls + 1;
    return ["CC", "DD"];
}

func consume(take m: Mixed): i32 {
    seen = i32(m.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Mixed { names: n1(), tag: stringOf("x") } : Mixed { names: n2(), tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c + 1) + calls - 2;
}
"#;
    let output = run_memory("guardviewliteral", take);
    assert_no_asan("guardviewliteral", &output);
    assert_exit("guardviewliteral", &output, 0);

    let plain = r#"module spec.cpr.guardviewliteralplain;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    calls = calls + 1;
    return ["CC", "DD"];
}

func consume(m: Mixed): i32 {
    seen = i32(m.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Mixed { names: n1(), tag: stringOf("x") } : Mixed { names: n2(), tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c + 1) + calls - 2;
}
"#;
    let output = run_memory("guardviewliteralplain", plain);
    assert_no_asan("guardviewliteralplain", &output);
    assert_exit("guardviewliteralplain", &output, 0);

    let direct = r#"module spec.cpr.guardviewliteraldirect;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func consume(take m: Mixed): i32 {
    seen = i32(m.names[0][0]);
    return 1;
}

func use(g: bool): i32 {
    if (g && consume(Mixed { names: n1(), tag: stringOf("x") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true);
    let b: i32 = use(false);
    return (a - 65) + (b + 1) + calls - 1;
}
"#;
    let output = run_memory("guardviewliteraldirect", direct);
    assert_no_asan("guardviewliteraldirect", &output);
    assert_exit("guardviewliteraldirect", &output, 0);

    let scalar = r#"module spec.cpr.guardviewliteralscalar;

import { String, stringOf } from std.text;

struct Sc { n: i32; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func s1(): i32 {
    calls = calls + 1;
    return 1;
}

func s2(): i32 {
    calls = calls + 1;
    return 2;
}

func consumeS(take m: Sc): i32 {
    seen = m.n;
    return 1;
}

func useS(g: bool, ok: bool): i32 {
    if (g && consumeS(ok ? Sc { n: s1(), tag: stringOf("x") } : Sc { n: s2(), tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = useS(true, true);
    let b: i32 = useS(true, false);
    let c: i32 = useS(false, true);
    return (a - 1) + (b - 2) + (c + 1) + calls - 2;
}
"#;
    let output = run_memory("guardviewliteralscalar", scalar);
    assert_no_asan("guardviewliteralscalar", &output);
    assert_exit("guardviewliteralscalar", &output, 0);

    let owned = r#"module spec.cpr.guardviewliteralowned;

import { String, stringOf, bytes } from std.text;

struct Ow { s: String; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func m1(): String {
    calls = calls + 1;
    return stringOf("ab");
}

func m2(): String {
    calls = calls + 1;
    return stringOf("cdef");
}

func consumeO(take m: Ow): i32 {
    seen = i32(len(bytes(m.s)));
    return 1;
}

func useO(g: bool, ok: bool): i32 {
    if (g && consumeO(ok ? Ow { s: m1(), tag: stringOf("x") } : Ow { s: m2(), tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = useO(true, true);
    let b: i32 = useO(true, false);
    let c: i32 = useO(false, true);
    return (a - 2) + (b - 4) + (c + 1) + calls - 2;
}
"#;
    let output = run_memory("guardviewliteralowned", owned);
    assert_no_asan("guardviewliteralowned", &output);
    assert_exit("guardviewliteralowned", &output, 0);
}

/// An unsafe-wrapped array base keeps its place at an element store.
///
/// `(unsafe b)[i] = 7` and `(unsafe b)[i]++` store into the element exactly as
/// the unparenthesized spelling does — the unsafe wrapper is place-transparent
/// — so each run reads the stored value back (exit 0) and the leak checker is
/// clean. The unsafe member spelling and the plain increment are the sibling
/// controls, beside the conditional-base rejections above.
#[test]
fn should_keep_the_unsafe_array_base_place_at_an_element_store() {
    let store = r#"module spec.cpr.unsafestore;

func main(): i32 {
    let b: i32[3] = [1, 2, 3];
    let i: i32 = 1;
    (unsafe b)[i] = 7;
    return b[1] - 7;
}
"#;
    let output = run_memory("unsafestore", store);
    assert_no_asan("unsafestore", &output);
    assert_exit("unsafestore", &output, 0);

    let increment = r#"module spec.cpr.unsafeinc;

func main(): i32 {
    let b: i32[3] = [1, 2, 3];
    let i: i32 = 1;
    (unsafe b)[i]++;
    return b[1] - 3;
}
"#;
    let output = run_memory("unsafeinc", &increment);
    assert_no_asan("unsafeinc", &output);
    assert_exit("unsafeinc", &output, 0);

    let control = r#"module spec.cpr.unsafecontrol;

struct Box { ys: i32[3]; }

func main(): i32 {
    let b: Box = Box { ys: [1, 2, 3] };
    let i: i32 = 1;
    (unsafe b).ys[i] = 7;
    (unsafe b).ys[i]++;
    b.ys[i]++;
    return b.ys[1] - 9;
}
"#;
    let output = run_memory("unsafecontrol", control);
    assert_no_asan("unsafecontrol", &output);
    assert_exit("unsafecontrol", &output, 0);
}

/// A nested literal inside a taken arm hands its fresh root to the arm.
///
/// `consume(take ok ? Outer { inner: Inner { names: [mk().s, mk().s], ... } } : ...)`
/// materializes the inner literal, and the arm that copies it must empty that
/// source in the same expression: the destination is the only owner, so the
/// take's release frees the tag once (exit 0, the leak checker clean, and the
/// element views read their bytes back). The skipped guard runs neither call.
#[test]
fn should_hand_a_nested_arm_literal_root_to_the_taken_arm() {
    let nested = r#"module spec.cpr.guardnest;

import { String, stringOf } from std.text;

struct Held { s: String; }
struct Inner { names: str[2]; tag: String; }
struct Outer { inner: Inner; }

let mkcalls: i32 = 0;
let seen: i32 = 0;

func mk(): Held {
    mkcalls = mkcalls + 1;
    return Held { s: stringOf("abcd") };
}

func consume(take m: Outer): i32 {
    seen = i32(m.inner.names[0][0]);
    return 1;
}

func useT(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Outer { inner: Inner { names: [mk().s, mk().s], tag: stringOf("i1") } }
                        : Outer { inner: Inner { names: [mk().s, mk().s], tag: stringOf("i2") } }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = useT(true, true);
    let b: i32 = useT(false, true);
    return (a - 97) + (b + 1) + mkcalls - 2;
}
"#;
    let output = run_memory("guardnest", nested);
    assert_no_asan("guardnest", &output);
    assert_exit("guardnest", &output, 0);
}

/// A two-field arm literal keeps every stored part inside its own arm.
///
/// `consumeO(take ok ? Outer { inner: ..., tag: ... } : Outer { ... })` stores
/// each arm field by field, and the field-wise text must stay inside the
/// conditional: a tail that escapes the else arm would run on the taken path,
/// overwrite the then arm's fresh field (a leak), and evaluate the skipped
/// arm's call. Both selections fold their read-backs and call counts (exit 0,
/// the leak checker clean), and the skipped arm's `t2` never runs.
#[test]
fn should_keep_a_two_field_arm_literal_inside_its_own_arm() {
    let nested = r#"module spec.cpr.guardtwice;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; tag: String; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func consumeO(take m: Outer): i32 {
    seen = i32(m.inner.names[0][0]);
    return 1;
}

func useT(g: bool, ok: bool): i32 {
    if (g && consumeO(ok ? Outer { inner: Inner { names: n1(), tag: stringOf("iiii1") }, tag: stringOf("ooooo1") }
                         : Outer { inner: Inner { names: n1(), tag: stringOf("iiii2") }, tag: stringOf("ooooo2") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = useT(true, true);
    let b: i32 = useT(true, false);
    let c: i32 = useT(false, true);
    return (a - 65) + (b - 65) + (c + 1) + calls - 2;
}
"#;
    let output = run_memory("guardtwice", nested);
    assert_no_asan("guardtwice", &output);
    assert_exit("guardtwice", &output, 0);

    let skipped = r#"module spec.cpr.guardskip;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; tag: String; }
struct Outer { inner: Inner; tag: String; }

let tcalls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    return ["AA", "BB"];
}

func t1(): String {
    tcalls = tcalls + 1;
    return stringOf("oooooo1");
}

func t2(): String {
    tcalls = tcalls + 1;
    return stringOf("oooooo2");
}

func consumeO(take m: Outer): i32 {
    seen = i32(m.inner.names[0][0]);
    return 1;
}

func useT(g: bool, ok: bool): i32 {
    if (g && consumeO(ok ? Outer { inner: Inner { names: n1(), tag: stringOf("iiiii") }, tag: t1() }
                         : Outer { inner: Inner { names: n1(), tag: stringOf("jjjjj") }, tag: t2() }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = useT(true, true);
    let b: i32 = useT(false, true);
    return (a - 65) + (b + 1) + tcalls - 1;
}
"#;
    let output = run_memory("guardskip", skipped);
    assert_no_asan("guardskip", &output);
    assert_exit("guardskip", &output, 0);
}

// ---------------------------------------------------------------------------
// Indexed element fields: a field reached through a run-time index.
//
// The frozen place list includes a struct field and an array element, so a
// field reached through a run-time index is a place on every supported root:
// the local fixed array, a borrow of a fixed array, a generic borrow, and a
// borrow slice. The store and its right side must emit once, the read-back
// must see the stored value, the overwritten owner must release once, and the
// bounds guard must stay in front of the write.
// ---------------------------------------------------------------------------

/// Returns the emitted C body of the definition whose signature ends with
/// `suffix`, up to its closing brace at the start of a line.
///
/// The emitter prints a forward declaration for every function, so the search
/// takes the last match, which is the definition. An uncalled function has no
/// later call site that could match the same suffix.
fn emitted_function_body<'a>(emitted: &'a str, suffix: &str) -> &'a str {
    let start = emitted
        .rfind(suffix)
        .unwrap_or_else(|| panic!("the emitted C has no function ending in {suffix}"));
    let rest = &emitted[start..];
    let end = rest.find("\n}").map(|offset| offset + 2).unwrap_or(rest.len());
    &rest[..end]
}

/// Asserts a run failed and its first stderr line is exactly `expected`.
///
/// The runtime trap contract is `error CODE: message` on the first line
/// followed by an abort, so a successful exit is always a failure, and a
/// different first line (a sanitizer banner, a stack-smashing message, or an
/// unchecked write that survives) is a failure too.
fn assert_trap_line(tag: &str, build: &str, output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "{tag} ({build}): expected a runtime trap (nonzero exit); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = String::from_utf8_lossy(&output.stderr)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(
        first, expected,
        "{tag} ({build}): unexpected first stderr line"
    );
}

/// Asserts a program traps with the exact first stderr line `expected` on both
/// a plain strict host build and an AddressSanitizer build.
///
/// A bounds trap is a memory-safety contract: the sanitizer build must reach
/// the same trap and must not report a memory error instead, so both runs must
/// agree on the first line and the sanitizer run must print no report.
fn assert_traps(tag: &str, source: &str, expected: &str) {
    let plain = run_host(tag, source);
    assert_trap_line(tag, "plain", &plain, expected);

    let sanitized = run_asan(tag, source);
    assert_trap_line(tag, "asan", &sanitized, expected);
    assert_no_asan(tag, &sanitized);
}

/// A member store through a run-time index writes the element and evaluates
/// its right side exactly once.
///
/// The local fixed array, the borrow parameter, the generic borrow, and the
/// constant-index twin each store `bump()` into one element and read it back,
/// so the four calls cancel the counter term and the run exits 0. The compound
/// store, the update, the deep member chain, the nested generic member, the
/// multi-index target, and the target-order case each read back their stored
/// value. The generic owning member and whole-element stores release the
/// overwritten owner once, so the memory run is leak-free. An uncalled borrow
/// slice still emits the element store's call and its bounds guard, and an
/// out-of-range generic-borrow index traps before the write.
#[test]
fn should_store_a_field_of_a_runtime_indexed_element() {
    let basic = r#"module spec.cpr.indexedfieldstore;

struct Elem { y: i32; }
struct Holder { items: Elem[3]; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func local(i: i32): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[i].y = bump();
    return a[i].y - 7;
}

func viaslice(borrow ps: Elem[], i: i32): i32 {
    ps[i].y = bump();
    return ps[i].y - 7;
}

func viaborrow(borrow w: Holder, i: i32): i32 {
    w.items[i].y = bump();
    return w.items[i].y - 7;
}

func viageneric<T>(borrow w: T, i: i32): i32 {
    w.items[i].y = bump();
    return w.items[i].y - 7;
}

func constant(): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[1].y = bump();
    return a[1].y - 7;
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }] };
    let r: i32 = 0;
    r = r + local(1);
    r = r + viaborrow(borrow h, 1);
    r = r + viageneric<Holder>(borrow h, 1);
    r = r + constant();
    return r + (calls - 4) * 100;
}
"#;
    let output = run_host("indexedfieldstore", basic);
    assert_exit("indexedfieldstore", &output, 0);

    let compound = r#"module spec.cpr.indexedcompound;

struct Elem { y: i32; }

func add(i: i32): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[i].y += 3;
    return a[i].y - 3;
}

func main(): i32 {
    return add(1);
}
"#;
    let output = run_host("indexedcompound", compound);
    assert_exit("indexedcompound", &output, 0);

    let update = r#"module spec.cpr.indexedupdate;

struct Elem { y: i32; }

func run(i: i32): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[i].y++;
    return a[i].y - 1;
}

func main(): i32 {
    return run(1);
}
"#;
    let output = run_host("indexedupdate", update);
    assert_exit("indexedupdate", &output, 0);

    let chain = r#"module spec.cpr.indexedchain;

struct Inner { y: i32; }
struct Elem { b: Inner; }

func run(i: i32): i32 {
    let a: Elem[3] = [Elem { b: Inner { y: 0 } }, Elem { b: Inner { y: 0 } }, Elem { b: Inner { y: 0 } }];
    a[i].b.y = 7;
    return a[i].b.y - 7;
}

func main(): i32 {
    return run(1);
}
"#;
    let output = run_host("indexedchain", chain);
    assert_exit("indexedchain", &output, 0);

    let nested = r#"module spec.cpr.indexednested;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct Mid { inner: Inner; }
struct Holder { mids: Mid[2]; }

func setg<T>(borrow w: T, i: i32): i32 {
    w.mids[i].inner.name = stringOf("zz");
    return i32(len(w.mids[i].inner.name));
}

func main(): i32 {
    let h: Holder = Holder { mids: [Mid { inner: Inner { name: stringOf("a") } }, Mid { inner: Inner { name: stringOf("b") } }] };
    let n: i32 = setg<Holder>(borrow h, 1);
    return n - 2;
}
"#;
    let output = run_host("indexednested", nested);
    assert_exit("indexednested", &output, 0);

    let multi = r#"module spec.cpr.indexedmulti;

import { String, stringOf } from std.text;

struct Cell { name: String; }
struct Grid { grid: Cell[2][2]; }

func setg<T>(borrow w: T, i: i32, j: i32): i32 {
    w.grid[i][j].name = stringOf("zz");
    return i32(len(w.grid[i][j].name));
}

func main(): i32 {
    let g: Grid = Grid { grid: [[Cell { name: stringOf("a") }, Cell { name: stringOf("b") }], [Cell { name: stringOf("c") }, Cell { name: stringOf("d") }]] };
    let n: i32 = setg<Grid>(borrow g, 1, 0);
    return n - 2;
}
"#;
    let output = run_host("indexedmulti", multi);
    assert_exit("indexedmulti", &output, 0);

    let order = r#"module spec.cpr.indexedorder;

struct Elem { y: i32; }

let idx: i32 = 1;
let calls: i32 = 0;

func set_index(): i32 {
    idx = 2;
    calls = calls + 1;
    return 7;
}

func run(): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[idx].y = set_index();
    let stored: i32 = a[1].y;
    return (stored - 7) + (a[2].y * 100) + (idx - 2);
}

func main(): i32 {
    return run() + calls - 1;
}
"#;
    let output = run_host("indexedorder", order);
    assert_exit("indexedorder", &output, 0);

    let member = r#"module spec.cpr.indexedgenmember;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func setg<T>(borrow w: T, i: i32): i32 {
    w.items[i].name = stringOf("zz");
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("a") }, Elem { name: stringOf("b") }] };
    let n: i32 = setg<Holder>(borrow h, 1);
    return n - 2;
}
"#;
    let output = run_memory("indexedgenmember", member);
    assert_no_asan("indexedgenmember", &output);
    assert_exit("indexedgenmember", &output, 0);

    let element = r#"module spec.cpr.indexedgenelement;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func setg<T>(borrow w: T, i: i32): i32 {
    w.items[i] = Elem { name: stringOf("zz") };
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("a") }, Elem { name: stringOf("b") }] };
    let n: i32 = setg<Holder>(borrow h, 1);
    return n - 2;
}
"#;
    let output = run_memory("indexedgenelement", element);
    assert_no_asan("indexedgenelement", &output);
    assert_exit("indexedgenelement", &output, 0);

    let whole = r#"module spec.cpr.indexedgenwhole;

import { String, stringOf } from std.text;

struct Cell { name: String; }
struct Grid { grid: Cell[2][2]; }

func setg<T>(borrow w: T, i: i32, j: i32): i32 {
    w.grid[i][j] = Cell { name: stringOf("zz") };
    return i32(len(w.grid[i][j].name));
}

func main(): i32 {
    let g: Grid = Grid { grid: [[Cell { name: stringOf("a") }, Cell { name: stringOf("b") }], [Cell { name: stringOf("c") }, Cell { name: stringOf("d") }]] };
    let n: i32 = setg<Grid>(borrow g, 1, 0);
    return n - 2;
}
"#;
    let output = run_memory("indexedgenwhole", whole);
    assert_no_asan("indexedgenwhole", &output);
    assert_exit("indexedgenwhole", &output, 0);

    let slice = r#"module spec.cpr.indexedslice;

struct Elem { y: i32; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func viaslice(borrow ps: Elem[], i: i32): i32 {
    ps[i].y = bump();
    return ps[i].y - 7;
}

func main(): i32 {
    return 0;
}
"#;
    let (_slice_scratch, emitted) = emit_host("indexedslice", slice);
    let body = emitted_function_body(&emitted, "_viaslice(");
    assert!(
        body.contains("bump("),
        "indexedslice: the element store's right side is missing from the slice function; body:\n{body}"
    );
    assert!(
        body.lines().any(|line| {
            let line = line.trim();
            !line.starts_with("return") && (line.contains(".y =") || line.contains("->y ="))
        }),
        "indexedslice: the slice function has no element store; body:\n{body}"
    );
    assert!(
        body.contains("cl_trap(\"E11211\", \"index out of range\")"),
        "indexedslice: the store has no bounds guard; body:\n{body}"
    );

    let bounds = r#"module spec.cpr.indexedbounds;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func setg<T>(borrow w: T, i: i32): void {
    w.items[i].name = stringOf("zz");
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("a") }, Elem { name: stringOf("b") }] };
    setg<Holder>(borrow h, 5);
    return 0;
}
"#;
    assert_traps("indexedbounds", bounds, "error E11211: index out of range");
}

// ---------------------------------------------------------------------------
// conditionalbasestore: a conditional selection is not a place.
//
// The frozen place list is closed, so a member or element store whose root is
// a conditional selection reports `E10221` instead of being accepted and
// dropped. The member walk must recurse to its root, and the `unsafe` wrapper
// is transparent to the place it wraps: a raw-prefix store stays accepted and
// must read back the written value, while a store rooted at a conditional
// selection stays rejected. Reads need no place, and the block spellings on a
// named or raw base stay accepted.
// ---------------------------------------------------------------------------

/// A store whose root is a conditional selection is rejected once, while the
/// raw-prefix and block spellings stay accepted and read back.
///
/// The five member/element shapes (`(c ? a : b).f.g` and its index, array,
/// index-base, and deep-chain siblings), the two `unsafe`-wrapped selections,
/// and the prefix conditional selection each report exactly one `E10221` and
/// emit no translation unit. The prefix raw stores `unsafe p.f` and
/// `unsafe p.f.x` are accepted and read back 7, and the read plus block
/// controls (`s.f.g`, `s.arr[i]`, `p.f.g` inside `unsafe { }`) run 0.
#[test]
fn should_reject_a_conditional_base_member_store() {
    let member = r#"module spec.cpr.condmemberstore;

struct Inner { g: i32; }
struct Box { f: Inner; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func use(c: bool, a: Box, b: Box): i32 {
    (c ? a : b).f.g = bump();
    return calls;
}

func main(): i32 {
    let a: Box = Box { f: Inner { g: 0 } };
    let b: Box = Box { f: Inner { g: 0 } };
    return use(true, a, b) - 1;
}
"#;
    compile_rejects("condmemberstore", member, "E10221", 1);

    let index = r#"module spec.cpr.condindexstore;

struct Cell { x: i32; }
struct Inner { rows: Cell[2]; }
struct Box { f: Inner; }

func use(c: bool, i: i32, a: Box, b: Box): i32 {
    (c ? a : b).f.rows[i].x = 7;
    return (c ? a : b).f.rows[i].x;
}

func main(): i32 {
    let a: Box = Box { f: Inner { rows: [Cell { x: 0 }, Cell { x: 0 }] } };
    let b: Box = Box { f: Inner { rows: [Cell { x: 0 }, Cell { x: 0 }] } };
    return use(true, 1, a, b) - 7;
}
"#;
    compile_rejects("condindexstore", index, "E10221", 1);

    let array = r#"module spec.cpr.condarraystore;

struct Bag { ys: i32[2]; }
struct Box { b: Bag; }

func use(c: bool, i: i32, a: Box, b: Box): i32 {
    (c ? a : b).b.ys[i] = 7;
    return (c ? a : b).b.ys[i];
}

func main(): i32 {
    let a: Box = Box { b: Bag { ys: [0, 0] } };
    let b: Box = Box { b: Bag { ys: [0, 0] } };
    return use(true, 1, a, b) - 7;
}
"#;
    compile_rejects("condarraystore", array, "E10221", 1);

    let index_base = r#"module spec.cpr.condindexbasestore;

struct Cell { x: i32; }

func use(c: bool, i: i32, a: Cell[2], b: Cell[2]): i32 {
    (c ? a : b)[i].x = 7;
    return (c ? a : b)[i].x;
}

func main(): i32 {
    let a: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    let b: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    return use(true, 1, a, b) - 7;
}
"#;
    compile_rejects("condindexbasestore", index_base, "E10221", 1);

    let deep = r#"module spec.cpr.conddeepstore;

struct Bag { ys: i32[2]; }
struct Holder { b: Bag; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func use(c: bool, i: i32, h1: Holder, h2: Holder): i32 {
    (c ? h1 : h2).b.ys[i] = bump();
    return calls;
}

func main(): i32 {
    let h1: Holder = Holder { b: Bag { ys: [0, 0] } };
    let h2: Holder = Holder { b: Bag { ys: [0, 0] } };
    return use(true, 1, h1, h2) - 1;
}
"#;
    compile_rejects("conddeepstore", deep, "E10221", 1);

    let unsafe_member = r#"module spec.cpr.condunsafemember;

struct Inner { x: i32; }
struct Box { v: Inner; }

func run(c: bool, a: Box, b: Box): i32 {
    (unsafe (c ? a : b)).v.x = 7;
    return 0;
}

func main(): i32 {
    let a: Box = Box { v: Inner { x: 0 } };
    let b: Box = Box { v: Inner { x: 0 } };
    return run(true, a, b);
}
"#;
    compile_rejects("condunsafemember", unsafe_member, "E10221", 1);

    let unsafe_index = r#"module spec.cpr.condunsafeindex;

struct Cell { x: i32; }

func run(c: bool, i: i32, a: Cell[2], b: Cell[2]): i32 {
    (unsafe (c ? a : b))[i].x = 7;
    return 0;
}

func main(): i32 {
    let a: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    let b: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    return run(true, 1, a, b);
}
"#;
    compile_rejects("condunsafeindex", unsafe_index, "E10221", 1);

    let prefix = r#"module spec.cpr.condprefixstore;

struct Inner { x: i32; }
struct Box { v: Inner; }

func run(c: bool, a: Box, b: Box): i32 {
    unsafe (c ? a : b).v = Inner { x: 7 };
    return 0;
}

func main(): i32 {
    let a: Box = Box { v: Inner { x: 0 } };
    let b: Box = Box { v: Inner { x: 0 } };
    return run(true, a, b);
}
"#;
    compile_rejects_once_at(
        "condprefixstore",
        prefix,
        "E10221",
        7,
        5,
        &[
            "left side of an assignment must be a place",
            "assign to a variable, field, or element",
        ],
        &[],
    );

    let raw_field = r#"module spec.cpr.condrawprefixfield;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        unsafe p.f = Inner { x: 7 };
    }
    return s.f.x - 7;
}
"#;
    let output = run_host("condrawprefixfield", raw_field);
    assert_exit("condrawprefixfield", &output, 0);

    let raw_chain = r#"module spec.cpr.condrawprefixchain;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        unsafe p.f.x = 7;
    }
    return s.f.x - 7;
}
"#;
    let output = run_host("condrawprefixchain", raw_chain);
    assert_exit("condrawprefixchain", &output, 0);

    let read = r#"module spec.cpr.condread;

struct Inner { g: i32; }
struct Box { f: Inner; }

func use(c: bool, a: Box, b: Box): i32 {
    return (c ? a : b).f.g;
}

func main(): i32 {
    let a: Box = Box { f: Inner { g: 7 } };
    let b: Box = Box { f: Inner { g: 9 } };
    return use(true, a, b) - 7;
}
"#;
    let output = run_host("condread", read);
    assert_exit("condread", &output, 0);

    let blocks = r#"module spec.cpr.condblocks;

struct Inner { x: i32; }
struct S { f: Inner; arr: i32[2]; }

func use(borrow s: S): i32 {
    s.f.x = 7;
    s.arr[1] = 7;
    return s.f.x + s.arr[1] - 14;
}

func main(): i32 {
    let s: S = S { f: Inner { x: 0 }, arr: [0, 0] };
    return use(borrow s);
}
"#;
    let output = run_host("condblocks", blocks);
    assert_exit("condblocks", &output, 0);

    let raw_block = r#"module spec.cpr.condrawblock;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        p.f.x = 7;
        let q: raw S = &s;
        q.f = Inner { x: 8 };
    }
    return s.f.x - 8;
}
"#;
    let output = run_host("condrawblock", raw_block);
    assert_exit("condrawblock", &output, 0);
}

// ---------------------------------------------------------------------------
// overwrittenview: a view or loop alias that reads the element being replaced.
//
// The overwritten owner must stay alive until the right-hand side has read it.
// A view local and a loop alias reach the same storage through a different
// base, so the store saves the old value instead of releasing it first.
// The constant-index and whole-row spellings share the rule.
// ---------------------------------------------------------------------------

/// A view of the overwritten element keeps its backing alive through the store.
///
/// Each program stores a fresh value over an element that a view local or a
/// loop alias still reads. The old owner is saved until the right-hand side has
/// run, so every run exits 0 and the memory run reports no error.
#[test]
fn should_store_a_value_a_view_derived_from_the_overwritten_element_reads() {
    let borrow_view = r#"module spec.cpr.overwrittenborrow;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func set(borrow w: Holder, i: i32): i32 {
    let v: str = w.items[1].name;
    w.items[i].name = stringOf(v);
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return set(borrow h, 1) - 2;
}
"#;
    let output = run_memory("overwrittenborrow", borrow_view);
    assert_no_asan("overwrittenborrow", &output);
    assert_exit("overwrittenborrow", &output, 0);

    let generic_view = r#"module spec.cpr.overwrittengeneric;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func set<T>(borrow w: T, i: i32): i32 {
    let v: str = w.items[1].name;
    w.items[i].name = stringOf(v);
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return set<Holder>(borrow h, 1) - 2;
}
"#;
    let output = run_memory("overwrittengeneric", generic_view);
    assert_no_asan("overwrittengeneric", &output);
    assert_exit("overwrittengeneric", &output, 0);

    let local_view = r#"module spec.cpr.overwrittenlocal;

import { String, stringOf } from std.text;

struct Elem { name: String; }

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    let i: i32 = 1;
    let v: str = a[1].name;
    a[i].name = stringOf(v);
    return i32(len(a[i].name)) - 2;
}
"#;
    let output = run_memory("overwrittenlocal", local_view);
    assert_no_asan("overwrittenlocal", &output);
    assert_exit("overwrittenlocal", &output, 0);

    let fixed_member = r#"module spec.cpr.overwrittenfixed;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let i: i32 = 1;
    let v: str = a[1].names[0];
    a[i].names = [stringOf(v), stringOf("xx")];
    return i32(len(a[i].names[0])) - 2;
}
"#;
    let output = run_memory("overwrittenfixed", fixed_member);
    assert_no_asan("overwrittenfixed", &output);
    assert_exit("overwrittenfixed", &output, 0);

    let struct_member = r#"module spec.cpr.overwrittenstruct;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct Elem { inner: Inner; }

func main(): i32 {
    let a: Elem[2] = [Elem { inner: Inner { name: stringOf("aa") } }, Elem { inner: Inner { name: stringOf("bb") } }];
    let i: i32 = 1;
    let v: str = a[1].inner.name;
    a[i].inner = Inner { name: stringOf(v) };
    return i32(len(a[i].inner.name)) - 2;
}
"#;
    let output = run_memory("overwrittenstruct", struct_member);
    assert_no_asan("overwrittenstruct", &output);
    assert_exit("overwrittenstruct", &output, 0);

    let loop_write = r#"module spec.cpr.overwrittenloop;

import { String, stringOf } from std.text;

struct Elem { name: String; }

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    let j: i32 = 1;
    for (borrow e in a) {
        a[j].name = stringOf(e.name);
    }
    return i32(len(a[1].name)) - 2;
}
"#;
    let output = run_memory("overwrittenloop", loop_write);
    assert_no_asan("overwrittenloop", &output);
    assert_exit("overwrittenloop", &output, 0);

    let loop_take = r#"module spec.cpr.overwrittenlooptake;

import { String, clone, stringOf } from std.text;

struct Elem { name: String; }

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    let j: i32 = 1;
    for (borrow e in a) {
        a[j].name = clone(e.name);
    }
    return i32(len(a[1].name)) - 2;
}
"#;
    let output = run_memory("overwrittenlooptake", loop_take);
    assert_no_asan("overwrittenlooptake", &output);
    assert_exit("overwrittenlooptake", &output, 0);

    let nested_alias = r#"module spec.cpr.overwrittennested;

import { String, stringOf } from std.text;

struct Cell { name: String; }

func main(): i32 {
    let g: Cell[2][2] = [[Cell { name: stringOf("aa") }, Cell { name: stringOf("ab") }], [Cell { name: stringOf("ba") }, Cell { name: stringOf("bb") }]];
    for (borrow row in g) {
        let j: i32 = 0;
        for (borrow e in row) {
            row[j].name = stringOf(e.name);
            j = j + 1;
        }
    }
    return i32(len(g[0][0].name)) - 2;
}
"#;
    let output = run_memory("overwrittennested", nested_alias);
    assert_no_asan("overwrittennested", &output);
    assert_exit("overwrittennested", &output, 0);

    let constant_view = r#"module spec.cpr.overwrittenconstant;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func set(borrow w: Holder): i32 {
    let v: str = w.items[1].name;
    w.items[1].name = stringOf(v);
    return i32(len(w.items[1].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return set(borrow h) - 2;
}
"#;
    let output = run_memory("overwrittenconstant", constant_view);
    assert_no_asan("overwrittenconstant", &output);
    assert_exit("overwrittenconstant", &output, 0);

    let constant_loop = r#"module spec.cpr.overwrittenconstantloop;

import { String, stringOf } from std.text;

struct Elem { name: String; }

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    for (borrow e in a) {
        a[1].name = stringOf(e.name);
    }
    return i32(len(a[1].name)) - 2;
}
"#;
    let output = run_memory("overwrittenconstantloop", constant_loop);
    assert_no_asan("overwrittenconstantloop", &output);
    assert_exit("overwrittenconstantloop", &output, 0);

    let fixed_generic = r#"module spec.cpr.overwrittenfixedgeneric;

import { String, stringOf } from std.text;

struct Holder<T> { rows: T[2]; }

func setrow<T>(borrow w: Holder<T>, i: i32): i32 {
    let v: str = w.rows[1][0];
    w.rows[i] = [stringOf(v), stringOf("xx")];
    return i32(len(w.rows[i][0]));
}

func main(): i32 {
    let h: Holder<String[2]> = Holder<String[2]> { rows: [[stringOf("aa"), stringOf("ab")], [stringOf("bb"), stringOf("bc")]] };
    return setrow<String[2]>(borrow h, 1) - 2;
}
"#;
    let output = run_memory("overwrittenfixedgeneric", fixed_generic);
    assert_no_asan("overwrittenfixedgeneric", &output);
    assert_exit("overwrittenfixedgeneric", &output, 0);
}

// ---------------------------------------------------------------------------
// rhsindex: an owned place store whose source element has a run-time index.
//
// The source keeps its index and its bounds guard, so the move empties the slot
// the read selected instead of element zero.
// ---------------------------------------------------------------------------

/// A runtime-index source place reads the slot the index selects.
///
/// The duplicable copies read the slot the index selects, the side-effecting
/// index runs once, and an out-of-range source traps before the store. A
/// distinct-index owned member/element move out of a `borrow` parameter reports
/// `E10034` exactly as the concrete twin does; the identical self-copy stays
/// accepted.
#[test]
fn should_copy_a_runtime_index_place_on_the_stored_value() {
    let copy = r#"module spec.cpr.rhsindex;

struct Elem { y: i32; }
struct Holder { items: Elem[2]; }

let cursor: i32 = 0;
let calls: i32 = 0;

func next_index(): i32 {
    calls = calls + 1;
    cursor = 1;
    return cursor;
}

func copy<T>(borrow w: T, i: i32, j: i32): i32 {
    w.items[i].y = w.items[j].y;
    return w.items[i].y;
}

func side<T>(borrow w: T, i: i32): i32 {
    w.items[i].y = w.items[next_index()].y;
    return w.items[i].y;
}

func concrete(i: i32, j: i32): i32 {
    let h: Holder = Holder { items: [Elem { y: 11 }, Elem { y: 22 }] };
    h.items[i].y = h.items[j].y;
    return h.items[i].y;
}

func main(): i32 {
    let h1: Holder = Holder { items: [Elem { y: 11 }, Elem { y: 22 }] };
    let h2: Holder = Holder { items: [Elem { y: 33 }, Elem { y: 44 }] };
    let a: i32 = copy<Holder>(borrow h1, 0, 1) - 22;
    let b: i32 = side<Holder>(borrow h2, 0) - 44;
    let c: i32 = concrete(0, 1) - 22;
    return a + b + c + (calls - 1) * 100 + (cursor - 1) * 10;
}
"#;
    let output = run_memory("rhsindex", copy);
    assert_no_asan("rhsindex", &output);
    assert_exit("rhsindex", &output, 0);

    let bounds = r#"module spec.cpr.rhsindexbounds;

struct Elem { y: i32; }
struct Holder { items: Elem[2]; }

func oob<T>(borrow w: T, j: i32): void {
    w.items[0].y = w.items[j].y;
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { y: 11 }, Elem { y: 22 }] };
    oob<Holder>(borrow h, 5);
    return 0;
}
"#;
    assert_traps("rhsindexbounds", bounds, "error E11211: index out of range");

    let self_member = r#"module spec.cpr.rhsindexselfmember;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func copy<T>(borrow w: T, i: i32, j: i32): i32 {
    w.items[i].name = w.items[j].name;
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return copy<Holder>(borrow h, 1, 1) - 2;
}
"#;
    compile_rejects("rhsindexselfmember", self_member, "E10034", 1);

    let self_same = r#"module spec.cpr.rhsindexselfsame;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func copy<T>(borrow w: T, i: i32): i32 {
    w.items[i].name = w.items[i].name;
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return copy<Holder>(borrow h, 1) - 2;
}
"#;
    let output = run_memory("rhsindexselfsame", self_same);
    assert_no_asan("rhsindexselfsame", &output);
    assert_exit("rhsindexselfsame", &output, 0);

    let self_whole = r#"module spec.cpr.rhsindexselfwhole;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func copy<T>(borrow w: T, i: i32, j: i32): i32 {
    w.items[i] = w.items[j];
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return copy<Holder>(borrow h, 1, 1) - 2;
}
"#;
    compile_rejects("rhsindexselfwhole", self_whole, "E10034", 1);

    let same_whole = r#"module spec.cpr.rhsindexselfsamewhole;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func copy<T>(borrow w: T, i: i32): i32 {
    w.items[i] = w.items[i];
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return copy<Holder>(borrow h, 1) - 2;
}
"#;
    let output = run_memory("rhsindexselfsamewhole", same_whole);
    assert_no_asan("rhsindexselfsamewhole", &output);
    assert_exit("rhsindexselfsamewhole", &output, 0);

    let take_member = r#"module spec.cpr.rhsindextakemember;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func copy<T>(take w: T, i: i32, j: i32): i32 {
    w.items[i].name = w.items[j].name;
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return copy<Holder>(take h, 1, 1) - 2;
}
"#;
    compile_rejects("rhsindextakemember", take_member, "E10026", 1);

    let take_duplicable = r#"module spec.cpr.rhsindextakedup;

struct Elem { y: i32; }
struct Holder { items: Elem[2]; }

func copy<T>(take w: T, i: i32, j: i32): i32 {
    w.items[i].y = w.items[j].y;
    return w.items[i].y;
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { y: 11 }, Elem { y: 22 }] };
    return copy<Holder>(take h, 0, 1) - 22;
}
"#;
    let output = run_memory("rhsindextakedup", take_duplicable);
    assert_no_asan("rhsindextakedup", &output);
    assert_exit("rhsindextakedup", &output, 0);
}

// ---------------------------------------------------------------------------
// conditional self-arm: a branch that names the destination through another
// index spelling.
//
// A literal index proves the same element at the store and the arm, so the
// chosen arm restores the saved value instead of copying it onto itself and
// emptying it. Every other index can change and is an element move.
// ---------------------------------------------------------------------------

/// A conditional self-arm is judged by index stability.
///
/// The member, whole-element, and fixed-array member literals store the
/// selected value back and read it; a local or computed index reports E10026.
#[test]
fn should_judge_a_conditional_self_arm_by_index_stability() {
    let member = r#"module spec.cpr.selfmember;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct Elem { inner: Inner; }

func main(): i32 {
    let a: Elem[2] = [Elem { inner: Inner { name: stringOf("aa") } }, Elem { inner: Inner { name: stringOf("bb") } }];
    let c: bool = true;
    a[1].inner = c ? a[1].inner : Inner { name: stringOf("zz") };
    return i32(len(a[1].inner.name)) - 2;
}
"#;
    let output = run_memory("selfmember", member);
    assert_no_asan("selfmember", &output);
    assert_exit("selfmember", &output, 0);

    let element = r#"module spec.cpr.selfelement;

import { String, stringOf } from std.text;

struct Elem { name: String; }

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    let c: bool = true;
    a[1] = c ? a[1] : Elem { name: stringOf("zz") };
    return i32(len(a[1].name)) - 2;
}
"#;
    let output = run_memory("selfelement", element);
    assert_no_asan("selfelement", &output);
    assert_exit("selfelement", &output, 0);

    let fixed = r#"module spec.cpr.selffixed;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let c: bool = true;
    a[1].names = c ? a[1].names : [stringOf("zz"), stringOf("ww")];
    return i32(len(a[1].names[0])) - 2;
}
"#;
    let output = run_memory("selffixed", fixed);
    assert_no_asan("selffixed", &output);
    assert_exit("selffixed", &output, 0);

    let local_self = r#"module spec.cpr.selflocal;

import { String, stringOf } from std.text;

struct Elem { name: String; }

func main(): i32 {
    let a: Elem[2] = [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }];
    let i: i32 = 1;
    let c: bool = true;
    a[i] = c ? a[i] : Elem { name: stringOf("zz") };
    return i32(len(a[i].name)) - 2;
}
"#;
    // A local index can be changed by the condition; the self-named arm is an
    // element move under the frozen v1 rule.
    assert_rejected_once("selflocal", local_self, "E10026");

    let equal_indices = r#"module spec.cpr.selfequal;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct Elem { inner: Inner; }

func main(): i32 {
    let a: Elem[2] = [Elem { inner: Inner { name: stringOf("aa") } }, Elem { inner: Inner { name: stringOf("bb") } }];
    let i: i32 = 1;
    let j: i32 = 1;
    let c: bool = true;
    a[i].inner = c ? a[j].inner : Inner { name: stringOf("zz") };
    return i32(len(a[i].inner.name)) - 2;
}
"#;
    // The same run-time index through a different expression cannot be
    // proved a self-copy; it is an element move and reports E10026.
    assert_rejected_once("selfequal", equal_indices, "E10026");

    let distinct_fixed = r#"module spec.cpr.selfdistinctfixed;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let i: i32 = 1;
    let j: i32 = 0;
    let c: bool = true;
    a[i].names = c ? a[j].names : [stringOf("zz"), stringOf("ww")];
    return i32(len(a[i].names[0])) - 2;
}
"#;
    assert_rejected_once("selfdistinctfixed", distinct_fixed, "E10026");

    let distinct_whole = r#"module spec.cpr.selfdistinctwhole;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2][2] = [[stringOf("aa"), stringOf("ab")], [stringOf("bb"), stringOf("bc")]];
    let i: i32 = 1;
    let j: i32 = 0;
    let c: bool = true;
    a[i] = c ? a[j] : [stringOf("zz"), stringOf("ww")];
    return i32(len(a[i][0])) - 2;
}
"#;
    assert_rejected_once("selfdistinctwhole", distinct_whole, "E10026");
}

// ---------------------------------------------------------------------------
// genericwrap: a compound store or update in a generic body.
//
// The arithmetic type resolves from the destination place, so the generic
// spelling uses the same signed wrap casts as the concrete twin.
// ---------------------------------------------------------------------------

/// A generic compound store and update wrap at the signed boundary.
///
/// `INT_MAX + 1` wraps to `INT_MIN` for the member, the whole element, and the
/// update, with the concrete twin unchanged.
#[test]
fn should_wrap_a_generic_compound_and_update() {
    let generic = r#"module spec.cpr.genericwrap;

struct Box { value: i32; }
struct Vec { values: i32[2]; }

func inc<T>(borrow w: T): i32 {
    w.value = 2147483647;
    w.value++;
    return w.value;
}

func add<T>(borrow w: T): i32 {
    w.value = 2147483647;
    w.value += 1;
    return w.value;
}

func addel<T>(borrow w: T, i: i32): i32 {
    w.values[i] = 2147483647;
    w.values[i] += 1;
    return w.values[i];
}

func main(): i32 {
    let b: Box = Box { value: 0 };
    let v: Vec = Vec { values: [0, 0] };
    let a: i32 = inc<Box>(borrow b);
    let c: i32 = add<Box>(borrow b);
    let d: i32 = addel<Vec>(borrow v, 1);
    if (a != -2147483648) { return 1; }
    if (c != -2147483648) { return 2; }
    if (d != -2147483648) { return 3; }
    return 0;
}
"#;
    let output = run_memory("genericwrap", generic);
    assert_no_asan("genericwrap", &output);
    assert_exit("genericwrap", &output, 0);

    let concrete = r#"module spec.cpr.concretewrap;

struct Box { value: i32; }
struct Vec { values: i32[2]; }

func main(): i32 {
    let b: Box = Box { value: 2147483647 };
    b.value++;
    let c: Vec = Vec { values: [0, 2147483647] };
    c.values[1] += 1;
    if (b.value != -2147483648) { return 1; }
    if (c.values[1] != -2147483648) { return 2; }
    return 0;
}
"#;
    let output = run_host("concretewrap", concrete);
    assert_exit("concretewrap", &output, 0);
}

// ---------------------------------------------------------------------------
// genericrow: a bare type parameter's row store.
//
// The store type resolves per instance, so a row-typed value copies by bytes
// instead of assigning a C array.
// ---------------------------------------------------------------------------

/// A bare generic row store copies a literal, a call, and an owned row.
#[test]
fn should_store_a_bare_generic_row() {
    let rows = r#"module spec.cpr.genericrow;

import { String, stringOf } from std.text;

struct Grid { rows: i32[2][2]; tag: String; }
struct TextGrid { rows: String[2][2]; tag: String; }

func setrow<T>(borrow g: T, i: i32, take v: i32[2]): void {
    g.rows[i] = v;
}

func settext<T>(borrow g: T, i: i32, take v: String[2]): void {
    g.rows[i] = v;
}

func mkrow(): i32[2] {
    return [7, 8];
}

func main(): i32 {
    let g: Grid = Grid { rows: [[1, 2], [3, 4]], tag: stringOf("g") };
    setrow<Grid>(borrow g, 1, [5, 6]);
    setrow<Grid>(borrow g, 0, mkrow());
    let t: TextGrid = TextGrid { rows: [[stringOf("aa"), stringOf("ab")], [stringOf("bb"), stringOf("bc")]], tag: stringOf("t") };
    settext<TextGrid>(borrow t, 0, [stringOf("xx"), stringOf("yy")]);
    return g.rows[1][0] + g.rows[0][0] + i32(len(t.rows[0][0])) - 14;
}
"#;
    let output = run_memory("genericrow", rows);
    assert_no_asan("genericrow", &output);
    assert_exit("genericrow", &output, 0);
}

// ---------------------------------------------------------------------------
// rawoutslot: a raw-deref member passed to a nullable out-slot.
//
// The argument is a writable place, so the call passes its address and the
// callee writes through it.
// ---------------------------------------------------------------------------

/// A raw-deref member out-slot argument passes the member's address.
#[test]
fn should_store_through_a_raw_deref_member_out_slot() {
    let source = r#"module spec.cpr.rawoutslot;

import { String, stringOf } from std.text;

struct Inner { name: String?; }

func set(borrow slot: String?, pick: bool): void {
    let other: String? = stringOf("bbbb");
    slot = pick ? slot : other;
}

func main(): i32 {
    let inner: Inner = Inner { name: stringOf("aaaa") };
    unsafe {
        let p: raw Inner = &inner;
        set(unsafe (*p).name, true);
    }
    if (inner.name == null) { return 9; }
    return i32(len(inner.name)) - 4;
}
"#;
    let output = run_memory("rawoutslot", source);
    assert_no_asan("rawoutslot", &output);
    assert_exit("rawoutslot", &output, 0);
}

// ---------------------------------------------------------------------------
// conditionalderef: the dereferenced conditional pointer is an accepted place.
//
// `*(c ? p : q)` names the selected pointer's pointee, so the store writes and
// the read-back observes the value on each selection. A member or element
// selected directly from the conditional value stays rejected.
// ---------------------------------------------------------------------------

/// A dereferenced conditional pointer stores and reads back on both selections.
#[test]
fn should_store_through_a_dereferenced_conditional_pointer() {
    let source = r#"module spec.cpr.condderefstore;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let a: S = S { f: Inner { x: 0 } };
    let b: S = S { f: Inner { x: 0 } };
    let c: bool = true;
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        *(c ? p : q) = S { f: Inner { x: 7 } };
        *(c ? q : p) = S { f: Inner { x: 9 } };
    }
    return (a.f.x - 7) + (b.f.x - 9);
}
"#;
    let output = run_host("condderefstore", source);
    assert_exit("condderefstore", &output, 0);
}

// ---------------------------------------------------------------------------
// storeloans: a view copied out of a loaned element keeps the element's loan.
//
// `xs[0] = a.v` stores a view of the local `a` into a view array element. A
// later copy of that element into a `str` local or a `str[N]` value must carry
// the loan with the value, so passing the copy into a caller's out-slot is the
// same escape the direct element and member arguments report: one `E10030` at
// the call site. A copy read inside the owner's scope names no escape and
// stays accepted.
// ---------------------------------------------------------------------------

/// Asserts the front end rejected `source` with exactly one report carrying
/// `code` at each of the given `line:column` sites, that every report contains
/// every phrase in `fragments`, and that it emitted no translation unit.
///
/// Two equal entries pin two reports at one span, which is the borrow-element
/// conditional take's shape; distinct entries pin one report at each site. The
/// site set is compared as a whole, so a report moved to another expression
/// fails the case even when the count and the code still match.
fn compile_rejects_at_sites(
    tag: &str,
    source: &str,
    code: &str,
    sites: &[(usize, usize)],
    fragments: &[&str],
) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|report| report.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        sites.len(),
        "{tag}: expected {} error(s), saw {}; stderr:\n{stderr}",
        sites.len(),
        errors.len()
    );
    for line in &errors {
        assert!(
            line.contains(code),
            "{tag}: expected the {code} report; saw `{line}`; stderr:\n{stderr}"
        );
    }
    let mut actual_sites: Vec<String> = stderr
        .lines()
        .filter(|report| report.trim_start().starts_with("--> "))
        .map(|report| report.trim().to_owned())
        .collect();
    actual_sites.sort();
    let mut expected_sites: Vec<String> = sites
        .iter()
        .map(|(line, column)| format!(":{line}:{column}"))
        .collect();
    expected_sites.sort();
    assert_eq!(
        actual_sites.len(),
        expected_sites.len(),
        "{tag}: expected {} report site(s), saw {}; stderr:\n{stderr}",
        expected_sites.len(),
        actual_sites.len()
    );
    for (actual, expected) in actual_sites.iter().zip(&expected_sites) {
        assert!(
            actual.ends_with(expected.as_str()),
            "{tag}: expected a report at {expected}; saw `{actual}`; stderr:\n{stderr}"
        );
    }
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

/// A view copied from a loaned element into a local carries the element's
/// loan, so the copy's out-slot store reports one `E10030` at the call.
///
/// `let s: str = xs[0];` copies the view stored in `xs[0]`, whose backing is
/// the local `a`. The copy reads freed storage once `a` is released, so
/// `fill(s, borrow slot)` reports one `E10030` at the call exactly as the
/// direct element argument and the member argument do. A read of the copy
/// while `a` is alive runs and stays memory-clean.
#[test]
fn should_report_an_escaping_element_read_store() {
    let store = r#"module spec.cpr.elemreadstore;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "elemreadstore",
        store,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let direct = r#"module spec.cpr.elemreaddirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(xs[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "elemreaddirect",
        direct,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let member = r#"module spec.cpr.elemreadmember;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    fill(a.w, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "elemreadmember",
        member,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let in_scope = r#"module spec.cpr.elemreadinscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    return i32(s[0]) - 97;
}

func main(): i32 { return outer(); }
"#;
    let output = run_memory("elemreadinscope", in_scope);
    assert_no_asan("elemreadinscope", &output);
    assert_exit("elemreadinscope", &output, 0);
}

/// A view array, or a single view, copied from conditional or literal sources
/// carries every source element's loan, so each out-slot store of a loaned
/// element reports one `E10030` at its call.
///
/// The conditional array copy, the array-literal copy (plain and fill form),
/// the single-view conditional, the conditional with an array-literal arm, the
/// conditional element inside an array literal, the different-arm-count
/// conditional, and the fill-form literal each report once at the affected
/// fill. The mixed-position conditional loans `ys[0]` from one arm and
/// `ys[1]` from the other, so both fills report: the destination record is the
/// union of both arms. The direct conditional argument keeps its one report,
/// and the copies read inside the owners' scope run and stay memory-clean.
#[test]
fn should_report_an_escaping_copy_from_a_conditional_or_literal_source() {
    let conditional = r#"module spec.cpr.copycond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[0] = a.w;
    let ys: str[2] = (c ? xs : zs);
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "copycond",
        conditional,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let literal = r#"module spec.cpr.copyliteral;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[2] = [xs[0], xs[1]];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "copyliteral",
        literal,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let literal_fill = r#"module spec.cpr.copyliteralfill;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = [xs[0]; "EE"];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "copyliteralfill",
        literal_fill,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let single = r#"module spec.cpr.copysingle;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let b: Box = Box { v: stringOf("cc") };
    let xs: str[1] = ["AA"];
    let zs: str[1] = ["CC"];
    xs[0] = a.v;
    zs[0] = b.v;
    let s: str = c ? xs[0] : zs[0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "copysingle",
        single,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let conditional_arm = r#"module spec.cpr.copycondarm;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let zs: str[2] = ["CC", "DD"];
    let ys: str[2] = c ? [xs[0], xs[1]] : zs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "copycondarm",
        conditional_arm,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let conditional_element = r#"module spec.cpr.copymix;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow s0: str?, borrow s1: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[0] = b.v;
    zs[1] = b.w;
    let ys: str[2] = [c ? xs[0] : zs[0], xs[1]];
    fill(ys[0], borrow s0);
    fill(ys[1], borrow s1);
}

func main(): i32 {
    let out0: str? = null;
    let out1: str? = null;
    outer(true, borrow out0, borrow out1);
    if (out0 == null) { return 9; }
    if (out1 == null) { return 10; }
    return i32(out0[0]) + i32(out1[0]) - (97 + 66);
}
"#;
        compile_rejects_once_at(
        "copymix",
        conditional_element,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let different_arms = r#"module spec.cpr.copydiff;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow s0: str?, borrow s1: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    xs[1] = a.w;
    zs[0] = a.v;
    let ys: str[2] = c ? xs : zs;
    fill(ys[0], borrow s0);
    fill(ys[1], borrow s1);
}

func main(): i32 {
    let out0: str? = null;
    let out1: str? = null;
    outer(true, borrow out0, borrow out1);
    if (out0 == null) { return 9; }
    if (out1 == null) { return 10; }
    return i32(out0[0]) + i32(out1[0]) - (97 + 98);
}
"#;
    compile_rejects_at_sites(
        "copydiff",
        different_arms,
        "E10034",
        &[(7, 46)],
        &["cannot release or store borrowed parameter 'p'"],
    );

    let fill_form = r#"module spec.cpr.copyfill;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow s0: str?, borrow s1: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let b: Box = Box { v: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[0] = b.v;
    let ys: str[2] = [c ? xs[0] : zs[0]; "EE"];
    fill(ys[0], borrow s0);
    fill(ys[1], borrow s1);
}

func main(): i32 {
    let out0: str? = null;
    let out1: str? = null;
    outer(true, borrow out0, borrow out1);
    if (out0 == null) { return 9; }
    if (out1 == null) { return 10; }
    return i32(out0[0]) + i32(out1[0]) - (97 + 69);
}
"#;
        compile_rejects_once_at(
        "copyfill",
        fill_form,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let mixed_positions = r#"module spec.cpr.copyunion;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow s0: str?, borrow s1: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = b.w;
    let ys: str[2] = (c ? xs : zs);
    fill(ys[0], borrow s0);
    fill(ys[1], borrow s1);
}

func main(): i32 {
    let out0: str? = null;
    let out1: str? = null;
    outer(true, borrow out0, borrow out1);
    if (out0 == null) { return 9; }
    if (out1 == null) { return 10; }
    return i32(out0[0]) + i32(out1[0]) - (97 + 100);
}
"#;
    compile_rejects_at_sites(
        "copyunion",
        mixed_positions,
        "E10034",
        &[(7, 46)],
        &["cannot release or store borrowed parameter 'p'"],
    );

    let direct = r#"module spec.cpr.copydirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let b: Box = Box { v: stringOf("cc") };
    fill(c ? a.v : b.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "copydirect",
        direct,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let in_scope = r#"module spec.cpr.copyinscope;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func outer(c: bool): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    xs[1] = a.w;
    zs[0] = b.v;
    zs[1] = b.w;
    let ys: str[2] = (c ? xs : zs);
    let lit: str[2] = [xs[0]; "EE"];
    let arr: str[2] = [c ? xs[0] : zs[0], xs[1]];
    let one: str = c ? xs[0] : zs[0];
    let arm: str[2] = c ? [xs[0], xs[1]] : zs;
    return i32(ys[0][0]) + i32(lit[0][0]) + i32(arr[0][0]) + i32(one[0])
         + i32(arm[1][0]) - (97 + 97 + 97 + 97 + 98);
}

func main(): i32 { return outer(true); }
"#;
    let output = run_memory("copyinscope", in_scope);
    assert_no_asan("copyinscope", &output);
    assert_exit("copyinscope", &output, 0);
}

// ---------------------------------------------------------------------------
// borrowelemcond: the borrow-element conditional take stays the two-report
// control.
//
// A `take` of a member selected from two borrowed elements moves the member
// out of each arm, so the v1 partial-move limit reports once per arm at the
// `take` argument. The generic and the concrete spelling must agree in the
// count and the site.
// ---------------------------------------------------------------------------

/// The borrow-element conditional take reports the partial-move limit twice at
/// one span in both the generic and the concrete spelling.
#[test]
fn should_report_the_borrow_element_conditional_take_twice() {
    let generic = r#"module spec.cpr.borrowelemcond;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat<T>(take x: T): i32 { return 1; }

func each<T>(xs: T, ys: T, c: bool): i32 {
    for (borrow v in xs) {
        for (borrow w in ys) {
            return eat(take (c ? v.name : w.name));
        }
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    let b: S[1] = [S { name: stringOf("b") }];
    return each<S[1]>(a, b, true) - 1;
}
"#;
    compile_rejects_at_sites(
        "borrowelemcond",
        generic,
        "E10026",
        &[(12, 24), (12, 24)],
        &["partial moves are limited to top-level fields in v1"],
    );

    let concrete = r#"module spec.cpr.borrowelemcondconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat(take x: String): i32 { return 1; }

func each(xs: S[1], ys: S[1], c: bool): i32 {
    for (borrow v in xs) {
        for (borrow w in ys) {
            return eat(take (c ? v.name : w.name));
        }
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    let b: S[1] = [S { name: stringOf("b") }];
    return each(a, b, true) - 1;
}
"#;
    compile_rejects_at_sites(
        "borrowelemcondconcrete",
        concrete,
        "E10026",
        &[(12, 24), (12, 24)],
        &["partial moves are limited to top-level fields in v1"],
    );
}

// ---------------------------------------------------------------------------
// Fill-position loans. A fill over a known fixed extent loans each element it
// actually stores: an unused fill records nothing, an overwritten element
// drops only its own loan, and a copy of a filled element keeps the loan. The
// write-conflict reading and its global form are pinned beside the direct
// twins, and a run-time element read carries the loan its constant twin does.
// ---------------------------------------------------------------------------

/// Asserts the front end rejected `source` with one report per `(code, line,
/// column)` entry, in any order, and emitted no translation unit.
///
/// One program may carry two different judgments — a write conflict and a
/// global escape — so the pair is compared as a whole: a fix that drops or
/// duplicates one of them fails the case even when the other still matches.
fn compile_rejects_codes(tag: &str, source: &str, reports: &[(&str, usize, usize)]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let mut actual: Vec<(String, String)> = Vec::new();
    let mut lines = stderr.lines();
    while let Some(current) = lines.next() {
        if !current.starts_with("error ") {
            continue;
        }
        let code = current
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .trim_end_matches(':')
            .to_owned();
        let Some(location) = lines.next() else { continue };
        actual.push((code, location.trim().to_owned()));
    }
    assert_eq!(
        actual.len(),
        reports.len(),
        "{tag}: expected {} error(s), saw {}; stderr:\n{stderr}",
        reports.len(),
        actual.len()
    );
    let mut remaining = actual;
    for (code, line, column) in reports {
        let suffix = format!(":{line}:{column}");
        let position = remaining
            .iter()
            .position(|(actual_code, site)| actual_code == code && site.ends_with(&suffix))
            .unwrap_or_else(|| {
                panic!("{tag}: expected {code} at {suffix}; stderr:\n{stderr}")
            });
        remaining.remove(position);
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// An array literal whose fill stores no remaining element, so its loan is
/// never part of the destination.
const FILL_UNUSED_SOURCE: &str = r#"module spec.cpr.fillunused;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = ["AA", "BB"; xs[0]];
    fill(ys[0], borrow slot);
    return i32(ys[0][0]) + i32(xs[0][0]) - (65 + 97);
}

func main(): i32 {
    let out: str? = null;
    let rc: i32 = outer(borrow out);
    if (out == null) { return 9; }
    return rc + i32(out[0]) - 65;
}
"#;

/// The single-element spelling of the unused fill.
const FILL_SINGLE_SOURCE: &str = r#"module spec.cpr.fillsingle;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[1] = ["AA"; xs[0]];
    fill(ys[0], borrow slot);
    return i32(ys[0][0]) + i32(xs[0][0]) - (65 + 97);
}

func main(): i32 {
    let out: str? = null;
    let rc: i32 = outer(borrow out);
    if (out == null) { return 9; }
    return rc + i32(out[0]) - 65;
}
"#;

/// A fill that covers a known extent, one of whose elements is overwritten
/// before it is read.
const FILL_OVERWRITE_SOURCE: &str = r#"module spec.cpr.filloverwrite;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let ys: str[4] = [; a.v];
    ys[2] = "ZZ";
    fill(ys[2], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 90;
}
"#;

/// A copy of a fill-covered element keeps the element's loan.
const FILL_COVERED_COPY_SOURCE: &str = r#"module spec.cpr.fillcovered;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[4] = [xs[0]; xs[1]];
    let s: str = ys[2];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The direct read of the same fill-covered element reports the same escape.
const FILL_COVERED_DIRECT_SOURCE: &str = r#"module spec.cpr.fillcovereddirect;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[4] = [xs[0]; xs[1]];
    fill(ys[2], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The fill-only spelling of the covered element.
const FILL_ONLY_COPY_SOURCE: &str = r#"module spec.cpr.fillonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[4] = [; xs[0]];
    let s: str = ys[2];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The fill-only direct twin.
const FILL_ONLY_DIRECT_SOURCE: &str = r#"module spec.cpr.fillonlydirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[4] = [; xs[0]];
    fill(ys[2], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A run-time element store followed by a run-time element copy.
const RUNTIME_COPY_SOURCE: &str = r#"module spec.cpr.runtimecopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(k: u32, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    let s: str = xs[k];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(0, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A constant element store followed by a run-time element copy, which must
/// union every recorded position because the selected one is unknown.
const RUNTIME_COPY_CONST_SOURCE: &str = r#"module spec.cpr.runtimecopyconst;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(k: u32, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[k];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(0, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A direct element read after a conflicting write reports only the write's
/// conflict: the old element position is dead and the write report stands.
const DIRECT_WRITE_SOURCE: &str = r#"module spec.cpr.directwrite;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    a.v = stringOf("qq");
    fill(xs[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The global-destination form of the same write: the global escape reports
/// beside the write's conflict.
const GLOBAL_WRITE_SOURCE: &str = r#"module spec.cpr.globalwrite;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let g: str = "AA";

func outer(): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    a.v = stringOf("qq");
    g = xs[0];
}

func main(): i32 {
    outer();
    return i32(g[0]) - 97;
}
"#;

/// The unused and overwritten fill copies stay accepted and memory-clean: a
/// fill that stores no element contributes no loan, and an overwritten
/// element drops only its own.
#[test]
fn should_keep_the_unused_and_overwritten_fill_copies_accepted() {
    for (tag, source) in [
        ("fillunused", FILL_UNUSED_SOURCE),
        ("fillsingle", FILL_SINGLE_SOURCE),
        ("filloverwrite", FILL_OVERWRITE_SOURCE),
    ] {
        compile_rejects(tag, source, "E10034", 1);
    }
}

/// A copy of a fill-covered element reports one `E10030` at its fill, exactly
/// as the direct read of the same element does.
#[test]
fn should_report_a_fill_covered_element_copy() {
        compile_rejects_once_at(
        "fillcovered",
        FILL_COVERED_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "fillcovereddirect",
        FILL_COVERED_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "fillonly",
        FILL_ONLY_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "fillonlydirect",
        FILL_ONLY_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// A run-time element copy carries the loan of the element it may select.
#[test]
fn should_report_a_runtime_index_element_copy() {
        compile_rejects_once_at(
        "runtimecopy",
        RUNTIME_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "runtimecopyconst",
        RUNTIME_COPY_CONST_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// A direct element read after a conflicting write keeps one `E10031`; the
/// fill's foreign store reports `E10034` beside it.
#[test]
fn should_report_one_conflict_when_a_direct_read_follows_a_write() {
    compile_rejects_codes(
        "directwrite",
        DIRECT_WRITE_SOURCE,
        &[("E10034", 7, 46), ("E10031", 13, 5)],
    );
}

/// The global form of the write conflict reports the write and the escape,
/// one report each.
#[test]
fn should_report_the_global_escape_after_a_write() {
    compile_rejects_codes(
        "globalwrite",
        GLOBAL_WRITE_SOURCE,
        &[("E10031", 13, 5), ("E10033", 14, 5)],
    );
}

// ---------------------------------------------------------------------------
// Nested fixed-extent fills. A nested literal expands its fill at its own
// dimension, so an unused or overwritten nested fill contributes nothing and
// a copy of a nested fill-covered element keeps the loan its direct read has.
// ---------------------------------------------------------------------------

/// A fill-only row inside a nested array literal, copied at a covered element.
const NESTED_FILLONLY_SOURCE: &str = r#"module spec.cpr.nestedfillonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The direct read of the same nested fill-covered element.
const NESTED_FILLONLY_DIRECT_SOURCE: &str = r#"module spec.cpr.nestedfillonlydirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    fill(g[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The nested fill-covered element copied through a whole-array copy.
const NESTED_COPY2_SOURCE: &str = r#"module spec.cpr.nestedcopy2;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    let h: str[2][2] = g;
    let s: str = h[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The direct read through the whole-array copy.
const NESTED_COPY2_DIRECT_SOURCE: &str = r#"module spec.cpr.nestedcopy2direct;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    let h: str[2][2] = g;
    fill(h[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The conditional spelling of the nested fill-covered element.
const NESTED_COND_COPY_SOURCE: &str = r#"module spec.cpr.nestedcondcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let zs: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    let g: str[2][2] = c ? [["AA", "BB"], [; xs[0]]] : zs;
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The direct read of the conditional spelling.
const NESTED_COND_COPY_DIRECT_SOURCE: &str = r#"module spec.cpr.nestedcondcopydirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let zs: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    let g: str[2][2] = c ? [["AA", "BB"], [; xs[0]]] : zs;
    fill(g[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The inner fill stores no element because the row's elements cover its
/// extent.
const NESTED_UNUSED_SOURCE: &str = r#"module spec.cpr.nestedunused;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"; xs[0]]];
    fill(g[1][0], borrow slot);
    return i32(g[1][0][0]) + i32(xs[0][0]) - (67 + 97);
}

func main(): i32 {
    let out: str? = null;
    let rc: i32 = outer(borrow out);
    if (out == null) { return 9; }
    return rc + i32(out[0]) - 67;
}
"#;

/// The nested fill element overwritten before its read.
const NESTED_OVERWRITE_SOURCE: &str = r#"module spec.cpr.nestedoverwrite;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    g[1][0] = "ZZ";
    fill(g[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 90;
}
"#;

/// A fill-only row whose every element is overwritten before its read.
const NESTED_FILLONLY_OVERWRITTEN_SOURCE: &str = r#"module spec.cpr.nestedfillonlyoverwritten;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    g[1][0] = "ZZ";
    g[1][1] = "YY";
    fill(g[1][1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 89;
}
"#;

/// The same inner fill read through a static sibling element.
const NESTED_WHOLE_UNUSED_SOURCE: &str = r#"module spec.cpr.nestedwholeunused;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"; xs[0]]];
    fill(g[0][0], borrow slot);
    return i32(g[1][0][0]) + i32(xs[0][0]) - (67 + 97);
}

func main(): i32 {
    let out: str? = null;
    let rc: i32 = outer(borrow out);
    if (out == null) { return 9; }
    return rc + i32(out[0]) - 65;
}
"#;

/// The nested unused and overwritten fill copies stay accepted and clean.
#[test]
fn should_keep_the_nested_unused_and_overwritten_fill_copies_accepted() {
    for (tag, source) in [
        ("nestedunused", NESTED_UNUSED_SOURCE),
        ("nestedoverwrite", NESTED_OVERWRITE_SOURCE),
        ("nestedfillonlyoverwritten", NESTED_FILLONLY_OVERWRITTEN_SOURCE),
        ("nestedwholeunused", NESTED_WHOLE_UNUSED_SOURCE),
    ] {
        compile_rejects(tag, source, "E10034", 1);
    }
}

/// A copy of a nested fill-covered element reports one `E10030` at its fill,
/// exactly as the direct read of the same element does.
#[test]
fn should_report_a_nested_fill_covered_element_copy() {
        compile_rejects_once_at(
        "nestedfillonly",
        NESTED_FILLONLY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "nestedfillonlydirect",
        NESTED_FILLONLY_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "nestedcopy2",
        NESTED_COPY2_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "nestedcopy2direct",
        NESTED_COPY2_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "nestedcondcopy",
        NESTED_COND_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
        compile_rejects_once_at(
        "nestedcondcopydirect",
        NESTED_COND_COPY_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// oncederef: an owning raw-deref read evaluates its pointer once.
//
// `unsafe *alt(p, q)` reads the pointee of a pointer expression that advances a
// counter on every call. Wherever the read's value is consumed — a raw store,
// a return, a struct-literal field, an array-literal element, and a
// conditional arm — the pointer must be evaluated once and the value must move
// out of that one pointee, so exactly one owner releases it. A second
// evaluation selects the other pointee: the read binds one buffer while the
// emptying releases the other, and the first buffer is released twice. The
// discarded read and the by-value argument keep the read-only shape, and a
// duplicable pointee is the concrete control that copies instead of moving.
// ---------------------------------------------------------------------------

/// An owning raw-deref read evaluates its impure pointer once in every
/// consuming position.
///
/// `alt` returns its first argument on the first call and advances a counter,
/// so a second call would select the other pointee. The store `v = *alt(p, q)`
/// inside a generic `overwrite<T>`, the return `return *alt(p, q)`, the
/// struct-literal field `Box { s: *alt(p, q) }`, the array-literal element
/// `[*alt(p, q)]`, and the conditional arm `flag ? *alt(p, q) : x` each read
/// the first call's pointee (two bytes) and must record exactly one call, so
/// every read-back exits 12. The discarded read and the non-taking argument
/// move nothing: each leaves the owner readable and exits 0 with one call. The
/// duplicable `T = i32` control copies its pointee, records one call, and exits
/// 0; the generic instance may emit one extra temporary versus its concrete
/// twin, and that emission delta is not part of this expectation. Every program
/// is accepted, builds under the strict flags, and must be free of memory
/// errors.
#[test]
fn should_evaluate_an_impure_deref_operand_once() {
    let store = r#"module spec.cpr.oncederefstore;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func overwrite<T>(p: raw T, q: raw T, take v: T): T {
    unsafe {
        v = *alt(p, q);
    }
    return v;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let v: String = stringOf("vv");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let r: String = overwrite<String>(p, q, v);
        return i32(len(r)) + calls * 10;
    }
}
"#;
    let output = run_memory("oncederefstore", store);
    assert_no_asan("oncederefstore", &output);
    assert_exit("oncederefstore", &output, 12);

    let returned = r#"module spec.cpr.oncederefreturn;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func read_it<T>(p: raw T, q: raw T): T {
    return unsafe *alt(p, q);
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let r: String = read_it<String>(p, q);
        return i32(len(r)) + calls * 10;
    }
}
"#;
    let output = run_memory("oncederefreturn", returned);
    assert_no_asan("oncederefreturn", &output);
    assert_exit("oncederefreturn", &output, 12);

    let field = r#"module spec.cpr.oncedereffield;

import { String, stringOf } from std.text;

let calls: i32 = 0;

struct Box { s: String; }

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func make<T>(p: raw T, q: raw T): Box {
    return Box { s: unsafe *alt(p, q) };
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let bx: Box = make<String>(p, q);
        return i32(len(bx.s)) + calls * 10;
    }
}
"#;
    let output = run_memory("oncedereffield", field);
    assert_no_asan("oncedereffield", &output);
    assert_exit("oncedereffield", &output, 12);

    let element = r#"module spec.cpr.oncederefarray;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let xs: String[1] = [*alt(p, q)];
        return i32(len(xs[0])) + calls * 10;
    }
}
"#;
    let output = run_memory("oncederefarray", element);
    assert_no_asan("oncederefarray", &output);
    assert_exit("oncederefarray", &output, 12);

    let arm = r#"module spec.cpr.oncederefcond;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let fallback: String = stringOf("cc");
    let flag: bool = true;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let r: String = flag ? *alt(p, q) : fallback;
        return i32(len(r)) + calls * 10;
    }
}
"#;
    let output = run_memory("oncederefcond", arm);
    assert_no_asan("oncederefcond", &output);
    assert_exit("oncederefcond", &output, 12);

    let discarded = r#"module spec.cpr.oncederefdiscard;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        *alt(p, q);
    }
    return i32(len(a)) - 2 + (calls - 1) * 10;
}
"#;
    let output = run_memory("oncederefdiscard", discarded);
    assert_no_asan("oncederefdiscard", &output);
    assert_exit("oncederefdiscard", &output, 0);

    let argument = r#"module spec.cpr.oncederefarg;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func observe(v: String): i32 { return i32(len(v)); }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let n: i32 = observe(*alt(p, q));
        return n + i32(len(a)) - 4 + (calls - 1) * 10;
    }
}
"#;
    let output = run_memory("oncederefarg", argument);
    assert_no_asan("oncederefarg", &output);
    assert_exit("oncederefarg", &output, 0);

    let duplicable = r#"module spec.cpr.oncederefdup;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func read_it<T>(p: raw T, q: raw T): T {
    return unsafe *alt(p, q);
}

func main(): i32 {
    let a: i32 = 5;
    let b: i32 = 6;
    unsafe {
        let p: raw i32 = &a;
        let q: raw i32 = &b;
        let r: i32 = read_it<i32>(p, q);
        return r + calls * 10 - 15;
    }
}
"#;
    let output = run_memory("oncederefdup", duplicable);
    assert_no_asan("oncederefdup", &output);
    assert_exit("oncederefdup", &output, 0);
}

// ---------------------------------------------------------------------------
// guardeddereftake: a guarded take carries its consumed deref source.
//
// `g && (eat(take (unsafe *q)) == 1)` reaches the callee only when `g` is
// true. On that path the callee owns the pointee and the caller's binding is
// emptied, so the caller must not release the moved value again. On the
// skipped path nothing is consumed and the owner keeps its value. When the
// pointer is an impure expression, the skipped path must not evaluate it at
// all: the call counter is the evidence.
// ---------------------------------------------------------------------------

/// A guarded take of a raw-deref source empties the pointee on the evaluated
/// path and leaves the pointer uncalled on the skipped path.
///
/// With `g` true, `eat(take (unsafe *q))` transfers the pointee to the callee
/// and the caller's `a` is emptied, so the run exits 0. With `g` false, the
/// guarded take never runs and `a` keeps its two bytes, so the read-back exits
/// 0. The impure spellings replace `q` with `alt(p, q)`: the taken path exits
/// 0, and the skipped path must not call `alt` at all — its counter folds into
/// the exit code, so a statement-entry call exits non-zero. Every program is
/// accepted, builds under the strict flags, and must be free of memory errors.
#[test]
fn should_carry_a_guarded_deref_take_source() {
    let taken = r#"module spec.cpr.guardeddereftake;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let g: bool = true;
    unsafe {
        let q: raw String = &a;
        let ok: bool = g && (eat(take (unsafe *q)) == 1);
        if (ok) { return 0; }
    }
    return 1;
}
"#;
    let output = run_memory("guardeddereftake", taken);
    assert_no_asan("guardeddereftake", &output);
    assert_exit("guardeddereftake", &output, 0);

    let skipped = r#"module spec.cpr.guardedderefskip;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let g: bool = false;
    unsafe {
        let q: raw String = &a;
        let ok: bool = g && (eat(take (unsafe *q)) == 1);
        if (ok) { return 1; }
    }
    return i32(len(a)) - 2;
}
"#;
    let output = run_memory("guardedderefskip", skipped);
    assert_no_asan("guardedderefskip", &output);
    assert_exit("guardedderefskip", &output, 0);

    let impure_taken = r#"module spec.cpr.guardedderefimpure;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let g: bool = true;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let ok: bool = g && (eat(take (unsafe *alt(p, q))) == 1);
        if (ok) { return 0; }
    }
    return 1;
}
"#;
    let output = run_memory("guardedderefimpure", impure_taken);
    assert_no_asan("guardedderefimpure", &output);
    assert_exit("guardedderefimpure", &output, 0);

    let impure_skipped = r#"module spec.cpr.guardedderefimpureskip;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let g: bool = false;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let ok: bool = g && (eat(take (unsafe *alt(p, q))) == 1);
        if (ok) { return 2; }
    }
    return calls * 10 + i32(len(a)) - 2;
}
"#;
    let output = run_memory("guardedderefimpureskip", impure_skipped);
    assert_no_asan("guardedderefimpureskip", &output);
    assert_exit("guardedderefimpureskip", &output, 0);
}

// ---------------------------------------------------------------------------
// literalsiblingorder: a literal's earlier sibling reads before a later
// sibling's owning-deref pointer fill.
//
// Fields and elements evaluate in source order. A later field or element that
// moves a pointee through an impure pointer must therefore fill its pointer
// only when its own store runs, after the earlier sibling's read. `counter`
// starts at 2 and `bump` adds 10, so an order-preserving run exits 2 while a
// fill hoisted above the literal exits 12.
// ---------------------------------------------------------------------------

/// A later field's and a later element's owning-deref fill runs after the
/// earlier sibling's read.
///
/// Both the two-field struct literal and the two-element array literal start
/// with `a: counter` (2). The second sibling's `b` reads the pointee of
/// `bump`, which adds 10 to `counter`. The run exits 2 when the earlier read
/// happens first; a fill hoisted above the literal would make the first read
/// observe 12. Each program is accepted, builds under the strict flags, and is
/// free of memory errors.
#[test]
fn should_evaluate_a_literal_sibling_before_a_later_deref_fill() {
    let field = r#"module spec.cpr.literalfieldorder;

import { String, stringOf } from std.text;

let counter: i32 = 2;

func bump<T>(s: raw T, t: raw T): raw T {
    counter = counter + 10;
    return s;
}

struct P { a: i32; b: String; }

func main(): i32 {
    let p: String = stringOf("pp");
    let q: String = stringOf("qq");
    unsafe {
        let rp: raw String = &p;
        let rq: raw String = &q;
        let v: P = P { a: counter, b: *bump(rp, rq) };
        return v.a;
    }
}
"#;
    let output = run_memory("literalfieldorder", field);
    assert_no_asan("literalfieldorder", &output);
    assert_exit("literalfieldorder", &output, 2);

    let element = r#"module spec.cpr.literalarrayorder;

import { String, stringOf } from std.text;

let counter: i32 = 2;

func bump<T>(s: raw T, t: raw T): raw T {
    counter = counter + 10;
    return s;
}

struct P { a: i32; b: String; }

func main(): i32 {
    let p: String = stringOf("pp");
    let q: String = stringOf("qq");
    let fill: String = stringOf("ff");
    unsafe {
        let rp: raw String = &p;
        let rq: raw String = &q;
        let xs: P[2] = [P { a: counter, b: fill }, P { a: 0, b: *bump(rp, rq) }];
        return xs[0].a;
    }
}
"#;
    let output = run_memory("literalarrayorder", element);
    assert_no_asan("literalarrayorder", &output);
    assert_exit("literalarrayorder", &output, 2);
}

// ---------------------------------------------------------------------------
// literalviewdestinations: a call-filled view-array field at every literal
// position.
//
// A function that returns a fixed view array writes through a hidden
// destination parameter, so a literal that names such a call as a field must
// bind that destination wherever the literal is evaluated: as the base of a
// projection, in an unguarded row conditional, in a multi-extent grid row, and
// on the guarded path of a row conditional. A concrete brace-list field needs
// no destination and stays the control.
// ---------------------------------------------------------------------------

/// A call-filled view-array field materializes its hidden destination at every
/// literal position.
///
/// `Inner { row: n1() }` is read through the literal, `Inner { row: ok ?
/// n1() : n2() }` selects one of two call rows without a guard, and the grid
/// literal fills two `str[2]` rows. Each call runs once, so the counters
/// cancel the trailing constants and every program exits 0. The guarded row
/// conditional evaluates its row only when the guard holds: the condition and
/// the taken return each run one call, while the skipped guard runs none, so
/// the fixture records four calls and exits 0. The concrete brace-list literal
/// needs no hidden destination and stays accepted. Every program must build
/// under the strict flags and be free of memory errors.
///
/// The grid program is the multi-extent `str[2][2]` row spelling, which a
/// separate multi-extent view-array emission gap also covers; the close-out
/// measures whether this case closes that gap incidentally.
#[test]
fn should_materialize_a_hidden_view_destination_at_every_literal_position() {
    let read = r#"module spec.cpr.viewdestread;

struct Inner { row: str[2]; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func use(): i32 {
    return i32(Inner { row: n1() }.row[0][0]) - 65 + (calls - 1) * 10;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("viewdestread", read);
    assert_no_asan("viewdestread", &output);
    assert_exit("viewdestread", &output, 0);

    let unguarded = r#"module spec.cpr.viewdestrowcond;

struct Inner { row: str[2]; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func use(ok: bool): i32 {
    return i32(Inner { row: ok ? n1() : n2() }.row[0][0]);
}

func main(): i32 {
    let a: i32 = use(true);
    let b: i32 = use(false);
    return (a - 65) + (b - 67) + (calls - 2) * 10;
}
"#;
    let output = run_memory("viewdestrowcond", unguarded);
    assert_no_asan("viewdestrowcond", &output);
    assert_exit("viewdestrowcond", &output, 0);

    let grid = r#"module spec.cpr.viewdestgrid;

struct Grid { rows: str[2][2]; }

let calls: i32 = 0;

func r1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func r2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func use(g: bool, o: bool): i32 {
    if (g && i32(Grid { rows: [o ? r1() : r2(), r1()] }.rows[0][0][0]) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(false, true);
    return (a - 1) + (b - 0) + calls - 2;
}
"#;
    let output = run_memory("viewdestgrid", grid);
    assert_no_asan("viewdestgrid", &output);
    assert_exit("viewdestgrid", &output, 0);

    let guarded = r#"module spec.cpr.viewdestguardrow;

struct Inner { row: str[2]; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func use(g: bool, ok: bool): i32 {
    if (g && i32(Inner { row: ok ? n1() : n2() }.row[0][0]) > 0) {
        return i32(Inner { row: ok ? n1() : n2() }.row[0][0]);
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c + 1) + calls - 4;
}
"#;
    let output = run_memory("viewdestguardrow", guarded);
    assert_no_asan("viewdestguardrow", &output);
    assert_exit("viewdestguardrow", &output, 0);

    let control = r#"module spec.cpr.viewdestcontrol;

struct Inner { row: str[2]; }

func readDirect(): i32 {
    return i32(Inner { row: ["AA", "BB"] }.row[0][0]);
}

func readBound(): i32 {
    let inner: Inner = Inner { row: ["CC", "DD"] };
    return i32(inner.row[1][0]);
}

func main(): i32 {
    return readDirect() + readBound() - 133;
}
"#;
    let output = run_memory("viewdestcontrol", control);
    assert_no_asan("viewdestcontrol", &output);
    assert_exit("viewdestcontrol", &output, 0);
}

// ---------------------------------------------------------------------------
// guardnestcarrier: an owning nested conditional under a short-circuit guard.
//
// The right side of `&&` evaluates only when the left does not decide, so an
// owning conditional argument must fill its carrier on the evaluated path
// only. A skipped guard runs no call, moves no source, and releases nothing.
// A loop re-executes the guard, so the carrier's previous value releases
// exactly once. The eager (unguarded) spelling keeps its current order, and a
// guarded literal or by-value take keeps the caller's pointee intact when the
// guard skips.
// ---------------------------------------------------------------------------

/// An owning nested conditional under a short-circuit guard fills its carrier
/// only on the evaluated path.
///
/// `g && consume(ok ? Outer { ... n1() ... } : Outer { ... n2() ... })` runs
/// the chosen arm's call only when `g` is true, so the single-pass fixture
/// records two calls (one per selected arm) and the loop fixture records four
/// (two guard evaluations per selected arm); both exit 0. The eager control
/// has no guard and keeps its order, and the direct literal under a guard
/// already fills only on its evaluated path, so it stays the second control.
/// A guarded struct-literal field, a guarded array-literal element, and a
/// guarded by-value take whose source is the caller's pointee leave that
/// pointee intact when the guard skips: the skipped call exits 0 after reading
/// the value, while the evaluated call moves it and exits with the expected
/// value. Every run must be free of memory errors, so the loop's re-fill and
/// both skipped paths are leak- and double-free-clean.
#[test]
fn should_keep_an_owning_nested_conditional_inside_its_guard() {
    let single = r#"module spec.cpr.guardnest;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 {
    seen = i32(o.inner.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                        : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c + 1) + calls - 2;
}
"#;
    let output = run_memory("guardnest", single);
    assert_no_asan("guardnest", &output);
    assert_exit("guardnest", &output, 0);

    let looped = r#"module spec.cpr.guardnestloop;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 {
    seen = i32(o.inner.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 2) {
        if (g && consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                            : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) == 1) {
            total = total + seen;
        }
        i = i + 1;
    }
    return total;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 130) + (b - 134) + (c - 0) + calls - 4;
}
"#;
    let output = run_memory("guardnestloop", looped);
    assert_no_asan("guardnestloop", &output);
    assert_exit("guardnestloop", &output, 0);

    let eager = r#"module spec.cpr.guardnesteager;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 {
    seen = i32(o.inner.names[0][0]);
    return 1;
}

func use(ok: bool): i32 {
    if (consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                   : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true);
    let b: i32 = use(false);
    return (a - 65) + (b - 67) + calls - 2;
}
"#;
    let output = run_memory("guardnesteager", eager);
    assert_no_asan("guardnesteager", &output);
    assert_exit("guardnesteager", &output, 0);

    let direct = r#"module spec.cpr.guardnestdirect;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func consume(o: Outer): i32 {
    seen = i32(o.inner.names[0][0]);
    return 1;
}

func use(g: bool): i32 {
    if (g && consume(Outer { inner: Inner { names: n1() }, tag: stringOf("x") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true);
    let b: i32 = use(false);
    return (a - 65) + (b + 1) + calls - 1;
}
"#;
    let output = run_memory("guardnestdirect", direct);
    assert_no_asan("guardnestdirect", &output);
    assert_exit("guardnestdirect", &output, 0);

    let field = r#"module spec.cpr.guardskipfield;

import { String, stringOf } from std.text;

struct Box { s: String; }

func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(take s: String, g: bool): i32 {
    unsafe {
        let q: raw String = &s;
        if (g && consume(Box { s: *q }) == 0) {
            return 1;
        }
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: i32 = use(stringOf("xx"), false);
    let b: i32 = use(stringOf("yy"), true);
    return a + b - 1;
}
"#;
    let output = run_memory("guardskipfield", field);
    assert_no_asan("guardskipfield", &output);
    assert_exit("guardskipfield", &output, 0);

    let element = r#"module spec.cpr.guardskipelement;

import { String, stringOf } from std.text;

func consume(xs: String[1]): i32 { return i32(len(xs[0])) - 2; }

func use(take s: String, g: bool): i32 {
    unsafe {
        let q: raw String = &s;
        if (g && consume([*q]) == 0) {
            return 1;
        }
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: i32 = use(stringOf("xx"), false);
    let b: i32 = use(stringOf("yy"), true);
    return a + b - 1;
}
"#;
    let output = run_memory("guardskipelement", element);
    assert_no_asan("guardskipelement", &output);
    assert_exit("guardskipelement", &output, 0);

    let argument = r#"module spec.cpr.guardskiparg;

import { String, stringOf } from std.text;

func consume(take x: String): i32 { return i32(len(x)) - 2; }

func use(take s: String, g: bool): i32 {
    unsafe {
        let q: raw String = &s;
        if (g && consume(*q) == 0) {
            return 1;
        }
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: i32 = use(stringOf("xx"), false);
    let b: i32 = use(stringOf("yy"), true);
    return a + b - 1;
}
"#;
    let output = run_memory("guardskiparg", argument);
    assert_no_asan("guardskiparg", &output);
    assert_exit("guardskiparg", &output, 0);
}

// ---------------------------------------------------------------------------
// owned literal field sources and the ternary carrier: a guarded literal that
// moves a place, a raw-deref read, a member, a nested field, or a fixed-array
// element must empty the source on the evaluated path only, and a ternary arm
// carrier must pass the hidden destination for every call-filled field.
// ---------------------------------------------------------------------------

/// An owned place field in a conditional arm under a guard.
const M7_OWNED_PLACE_SOURCE: &str = r#"module spec.cpr.m7ownedplace;

import { String, stringOf } from std.text;

struct Box { s: String; }
func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(g: bool, ok: bool, take s: String, take t: String): i32 {
    if (g && consume(ok ? Box { s: s } : Box { s: t }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    return a + b + c - 2;
}
"#;

/// A raw-deref field in a conditional arm under a guard.
const M7_OWNED_DEREF_SOURCE: &str = r#"module spec.cpr.m7ownedderef;

import { String, stringOf } from std.text;

struct Box { s: String; }
func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(g: bool, ok: bool, take s: String, take t: String): i32 {
    unsafe {
        let q: raw String = &s;
        let r: raw String = &t;
        if (g && consume(ok ? Box { s: *q } : Box { s: *r }) == 0) {
            return 1;
        }
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    return a + b + c - 2;
}
"#;

/// A member field in a conditional arm under a guard.
const M7_OWNED_MEMBER_SOURCE: &str = r#"module spec.cpr.m7ownedmember;

import { String, stringOf } from std.text;

struct H { s: String; }
struct Box { s: String; }
func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(g: bool, ok: bool, take h: H, take k: H): i32 {
    if (g && consume(ok ? Box { s: h.s } : Box { s: k.s }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, H { s: stringOf("xx") }, H { s: stringOf("yy") });
    let b: i32 = use(true, false, H { s: stringOf("xx") }, H { s: stringOf("yy") });
    let c: i32 = use(false, true, H { s: stringOf("xx") }, H { s: stringOf("yy") });
    return a + b + c - 2;
}
"#;

/// A nested field in a conditional arm under a guard.
const M7_OWNED_NESTED_SOURCE: &str = r#"module spec.cpr.m7ownednested;

import { String, stringOf } from std.text;

struct Inner { s: String; }
struct Outer { inner: Inner; }
func consume(o: Outer): i32 { return i32(len(o.inner.s)) - 2; }

func use(g: bool, ok: bool, take s: String, take t: String): i32 {
    if (g && consume(ok ? Outer { inner: Inner { s: s } } : Outer { inner: Inner { s: t } }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    return a + b + c - 2;
}
"#;

/// A fixed-array place field in a conditional arm under a guard.
const M7_OWNED_FIXED_SOURCE: &str = r#"module spec.cpr.m7ownedfixed;

import { String, stringOf } from std.text;

struct A { ss: String[2]; }
func consume(a: A): i32 { return i32(len(a.ss[0])) - 2; }

func use(g: bool, ok: bool, take a: String, take b: String, take c: String, take d: String): i32 {
    if (g && consume(ok ? A { ss: [a, b] } : A { ss: [c, d] }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("aa"), stringOf("bb"), stringOf("cc"), stringOf("dd"));
    let b: i32 = use(true, false, stringOf("aa"), stringOf("bb"), stringOf("cc"), stringOf("dd"));
    let c: i32 = use(false, true, stringOf("aa"), stringOf("bb"), stringOf("cc"), stringOf("dd"));
    return a + b + c - 2;
}
"#;

/// A fixed-array field of member reads in a conditional arm.
const M7_OWNED_MEMBER_FIXED_SOURCE: &str = r#"module spec.cpr.m7ownedmemberfixed;

import { String, stringOf } from std.text;

struct H { s: String; }
struct A { ss: String[2]; }
func consume(a: A): i32 { return i32(len(a.ss[0])) - 2; }

func use(g: bool, ok: bool, take h: H, take k: H, take m: H, take n: H): i32 {
    if (g && consume(ok ? A { ss: [h.s, k.s] } : A { ss: [m.s, n.s] }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, H { s: stringOf("aa") }, H { s: stringOf("bb") }, H { s: stringOf("cc") }, H { s: stringOf("dd") });
    let b: i32 = use(true, false, H { s: stringOf("aa") }, H { s: stringOf("bb") }, H { s: stringOf("cc") }, H { s: stringOf("dd") });
    let c: i32 = use(false, true, H { s: stringOf("aa") }, H { s: stringOf("bb") }, H { s: stringOf("cc") }, H { s: stringOf("dd") });
    return a + b + c - 2;
}
"#;

/// A direct literal argument whose field is an owned place.
const M7_OWNED_DIRECT_SOURCE: &str = r#"module spec.cpr.m7owneddirect;

import { String, stringOf } from std.text;

struct Box { s: String; }
func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(g: bool, take s: String): i32 {
    if (g && consume(Box { s: s }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, stringOf("xx"));
    let b: i32 = use(false, stringOf("yy"));
    return a + b - 1;
}
"#;

/// A direct literal argument whose fixed-array field is owned places.
const M7_OWNED_DIRECT_FIXED_SOURCE: &str = r#"module spec.cpr.m7owneddirectfixed;

import { String, stringOf } from std.text;

struct A { ss: String[2]; }
func consume(a: A): i32 { return i32(len(a.ss[0])) - 2; }

func use(g: bool, take a: String, take b: String): i32 {
    if (g && consume(A { ss: [a, b] }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, stringOf("aa"), stringOf("bb"));
    let b: i32 = use(false, stringOf("cc"), stringOf("dd"));
    return a + b - 1;
}
"#;

/// An owned place field under `||`.
const M7_OWNED_OR_SOURCE: &str = r#"module spec.cpr.m7ownedor;

import { String, stringOf } from std.text;

struct Box { s: String; }
func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(g: bool, ok: bool, take s: String, take t: String): i32 {
    if (g || consume(ok ? Box { s: s } : Box { s: t }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, false, stringOf("xx"), stringOf("yy"));
    return a + b + c - 3;
}
"#;

/// An owned place field re-created and moved on each loop iteration.
const M7_OWNED_LOOP_SOURCE: &str = r#"module spec.cpr.m7ownedloop;

import { String, stringOf } from std.text;

struct Box { s: String; }
func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(g: bool): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 2) {
        let s: String = stringOf("xx");
        if (g && consume(Box { s: s }) == 0) {
            total = total + 1;
        }
        i = i + 1;
    }
    return total;
}

func main(): i32 {
    let a: i32 = use(true);
    let b: i32 = use(false);
    return a + b - 2;
}
"#;

/// An owned place field in a ternary arm.
const M7_OWNED_TERNARY_SOURCE: &str = r#"module spec.cpr.m7ownedternary;

import { String, stringOf } from std.text;

struct Box { s: String; }
func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(g: bool, ok: bool, take s: String, take t: String): i32 {
    return g ? consume(ok ? Box { s: s } : Box { s: t }) : 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    return a + b + c;
}
"#;

/// Every storage position of an owned literal field moves the source on the
/// evaluated path, and the skipped selection leaves it owned.
#[test]
fn should_move_an_owned_literal_field_source_on_every_guarded_path() {
    for (tag, source) in [
        ("m7ownedplace", M7_OWNED_PLACE_SOURCE),
        ("m7ownedderef", M7_OWNED_DEREF_SOURCE),
        ("m7ownedmember", M7_OWNED_MEMBER_SOURCE),
        ("m7ownednested", M7_OWNED_NESTED_SOURCE),
        ("m7ownedfixed", M7_OWNED_FIXED_SOURCE),
        ("m7ownedmemberfixed", M7_OWNED_MEMBER_FIXED_SOURCE),
        ("m7owneddirect", M7_OWNED_DIRECT_SOURCE),
        ("m7owneddirectfixed", M7_OWNED_DIRECT_FIXED_SOURCE),
        ("m7ownedor", M7_OWNED_OR_SOURCE),
        ("m7ownedloop", M7_OWNED_LOOP_SOURCE),
        ("m7ownedternary", M7_OWNED_TERNARY_SOURCE),
    ] {
        let output = run_memory(tag, source);
        assert_no_asan(tag, &output);
        assert_exit(tag, &output, 0);
    }
}

/// A ternary-arm carrier whose conditional argument has call-filled fields.
const M7_TERNARY_CARRIER_SOURCE: &str = r#"module spec.cpr.m7ternarycarrier;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 { return i32(o.inner.names[0][0]); }

func use(g: bool, ok: bool): i32 {
    return g ? consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                          : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) : 0;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c - 0) + calls - 2;
}
"#;

/// The ternary-arm carrier builds strictly, selects both arms, and calls each
/// chosen arm once; the skipped selection runs neither branch.
#[test]
fn should_fill_a_ternary_arm_carrier_on_its_evaluated_path() {
    let output = run_memory("m7ternarycarrier", M7_TERNARY_CARRIER_SOURCE);
    assert_no_asan("m7ternarycarrier", &output);
    assert_exit("m7ternarycarrier", &output, 0);
}

/// An owning literal row projection bound to a whole row.
const M7_OWNING_ROW_SOURCE: &str = r#"module spec.cpr.m7owningrow;

import { String, stringOf } from std.text;

struct Mixed { row: str[2]; tag: String; }

let calls: i32 = 0;
func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func use(): i32 {
    let r: str[2] = Mixed { row: n1(), tag: stringOf("xy") }.row;
    return i32(r[0][0]) - 65 + calls - 1;
}

func main(): i32 { return use(); }
"#;

/// An owning literal row projection bound to one element.
const M7_OWNING_ELEMENT_SOURCE: &str = r#"module spec.cpr.m7owningelem;

import { String, stringOf } from std.text;

struct Mixed { row: str[2]; tag: String; }

let calls: i32 = 0;
func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func use(): i32 {
    let e: str = Mixed { row: n1(), tag: stringOf("xy") }.row[0];
    return i32(e[0]) - 65 + calls - 1;
}

func main(): i32 { return use(); }
"#;

/// An owning literal whose row or element is bound to a view outlives the
/// statement that produced it.
#[test]
fn should_outlive_the_statement_for_an_owning_literal_projection() {
    for (tag, source) in [
        ("m7owningrow", M7_OWNING_ROW_SOURCE),
        ("m7owningelem", M7_OWNING_ELEMENT_SOURCE),
    ] {
        let output = run_memory(tag, source);
        assert_no_asan(tag, &output);
        assert_exit(tag, &output, 0);
    }
}

// ---------------------------------------------------------------------------
// guarded literal conditional fields: an owned conditional field moves the
// chosen arm into the field and empties that arm's source, including a
// raw-deref arm whose pointer runs once.
// ---------------------------------------------------------------------------

/// Two place arms under `&&`.
const M7_L4_PLACE_AND_SOURCE: &str = r#"module spec.cpr.m7l4placeand;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }

func use(g: bool, flag: bool, take x: String, take y: String): i32 {
    if (g && consume(Holder { s: flag ? x : y, tag: stringOf("x") }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    return a + b + c - 2;
}
"#;

/// Two place arms under `||`.
const M7_L4_PLACE_OR_SOURCE: &str = r#"module spec.cpr.m7l4placeor;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }

func use(g: bool, flag: bool, take x: String, take y: String): i32 {
    if (g || consume(Holder { s: flag ? x : y, tag: stringOf("x") }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, false, stringOf("xx"), stringOf("yy"));
    return a + b + c - 3;
}
"#;

/// Two place arms in a ternary arm.
const M7_L4_PLACE_TERNARY_SOURCE: &str = r#"module spec.cpr.m7l4placetenary;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }

func use(g: bool, flag: bool, take x: String, take y: String): i32 {
    return g ? consume(Holder { s: flag ? x : y, tag: stringOf("x") }) : 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"));
    return a + b + c;
}
"#;

/// A raw-deref impure arm and a place arm under `&&`.
const M7_L4_DEREF_AND_SOURCE: &str = r#"module spec.cpr.m7l4derefand;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }
func pickPtr(p: raw String, q: raw String, flag: bool): raw String {
    if (flag) { return p; }
    return q;
}

func use(g: bool, flag: bool, take x: String, take y: String, take f: String): i32 {
    unsafe {
        let p: raw String = &x;
        if (g && consume(Holder { s: flag ? *pickPtr(p, p, flag) : f, tag: stringOf("x") }) == 0) {
            return 1;
        }
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    return a + b + c - 2;
}
"#;

/// A raw-deref impure arm and a place arm under `||`.
const M7_L4_DEREF_OR_SOURCE: &str = r#"module spec.cpr.m7l4derefor;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }
func pickPtr(p: raw String, q: raw String, flag: bool): raw String {
    if (flag) { return p; }
    return q;
}

func use(g: bool, flag: bool, take x: String, take y: String, take f: String): i32 {
    unsafe {
        let p: raw String = &x;
        if (g || consume(Holder { s: flag ? *pickPtr(p, p, flag) : f, tag: stringOf("x") }) == 0) {
            return 1;
        }
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    let b: i32 = use(false, true, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    let c: i32 = use(false, false, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    return a + b + c - 3;
}
"#;

/// A raw-deref impure arm and a place arm in a ternary arm.
const M7_L4_DEREF_TERNARY_SOURCE: &str = r#"module spec.cpr.m7l4derefternary;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }
func pickPtr(p: raw String, q: raw String, flag: bool): raw String {
    if (flag) { return p; }
    return q;
}

func use(g: bool, flag: bool, take x: String, take y: String, take f: String): i32 {
    unsafe {
        let p: raw String = &x;
        return g ? consume(Holder { s: flag ? *pickPtr(p, p, flag) : f, tag: stringOf("x") }) : 0;
    }
}

func main(): i32 {
    let a: i32 = use(true, true, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    let b: i32 = use(true, false, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    let c: i32 = use(false, true, stringOf("xx"), stringOf("yy"), stringOf("ff"));
    return a + b + c;
}
"#;

/// An owned conditional field moves its chosen arm and empties that arm's
/// source under every guarded position; the raw-deref arm's pointer runs once.
#[test]
fn should_move_a_guarded_literal_conditional_field_arm() {
    for (tag, source) in [
        ("m7l4placeand", M7_L4_PLACE_AND_SOURCE),
        ("m7l4placeor", M7_L4_PLACE_OR_SOURCE),
        ("m7l4placetenary", M7_L4_PLACE_TERNARY_SOURCE),
        ("m7l4derefand", M7_L4_DEREF_AND_SOURCE),
        ("m7l4derefor", M7_L4_DEREF_OR_SOURCE),
        ("m7l4derefternary", M7_L4_DEREF_TERNARY_SOURCE),
    ] {
        let output = run_memory(tag, source);
        assert_no_asan(tag, &output);
        assert_exit(tag, &output, 0);
    }
}

// ---------------------------------------------------------------------------
// call-filled element moves: an owning element read from the body-scope read
// base moves the selected slot and empties it, so the base's cleanup cannot
// free the value the destination now holds.
// ---------------------------------------------------------------------------

/// A `let` binding from element zero of a call-filled field.
const M7_R1_ZERO_SOURCE: &str = r#"module spec.cpr.m7r1zero;

import { String, stringOf } from std.text;

struct Holder { names: String[2]; }
let calls: i32 = 0;
func n1(): String[2] { calls = calls + 1; return [stringOf("aa"), stringOf("bb")]; }

func use(): i32 {
    let s: String = Holder { names: n1() }.names[0];
    return i32(len(s)) - 2 + calls - 1;
}

func main(): i32 { return use(); }
"#;

/// A `let` binding from element one of a call-filled field.
const M7_R1_ONE_SOURCE: &str = r#"module spec.cpr.m7r1one;

import { String, stringOf } from std.text;

struct Holder { names: String[2]; }
let calls: i32 = 0;
func n1(): String[2] { calls = calls + 1; return [stringOf("aa"), stringOf("bb")]; }

func use(): i32 {
    let s: String = Holder { names: n1() }.names[1];
    return i32(len(s)) - 2 + calls - 1;
}

func main(): i32 { return use(); }
"#;

/// The assignment spelling from element one of a call-filled field.
const M7_R1_ASSIGN_SOURCE: &str = r#"module spec.cpr.m7r1assign;

import { String, stringOf } from std.text;

struct Holder { names: String[2]; }
let calls: i32 = 0;
func n1(): String[2] { calls = calls + 1; return [stringOf("aa"), stringOf("bb")]; }

func use(): i32 {
    let s: String = stringOf("zz");
    s = Holder { names: n1() }.names[1];
    return i32(len(s)) - 2 + calls - 1;
}

func main(): i32 { return use(); }
"#;

/// The whole-row projection control.
const M7_R1_WHOLE_SOURCE: &str = r#"module spec.cpr.m7r1whole;

import { String, stringOf } from std.text;

struct Mixed { row: str[2]; tag: String; }
let calls: i32 = 0;
func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func use(): i32 {
    let r: str[2] = Mixed { row: n1(), tag: stringOf("xy") }.row;
    return i32(r[0][0]) - 65 + calls - 1;
}

func main(): i32 { return use(); }
"#;

/// The member projection control.
const M7_R1_MEMBER_SOURCE: &str = r#"module spec.cpr.m7r1member;

import { String, stringOf } from std.text;

struct Mixed { row: str[2]; tag: String; }
let calls: i32 = 0;
func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func use(): i32 {
    let s: String = Mixed { row: n1(), tag: stringOf("xy") }.tag;
    return i32(len(s)) - 2 + calls - 1;
}

func main(): i32 { return use(); }
"#;

/// The argument projection control.
const M7_R1_ARGUMENT_SOURCE: &str = r#"module spec.cpr.m7r1arg;

import { String, stringOf } from std.text;

struct Mixed { row: str[2]; tag: String; }
let calls: i32 = 0;
func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func consume(s: String): i32 { return i32(len(s)) - 2; }

func use(): i32 {
    if (consume(Mixed { row: n1(), tag: stringOf("xy") }.tag) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 { return use() - 1; }
"#;

/// The return projection control.
const M7_R1_RETURN_SOURCE: &str = r#"module spec.cpr.m7r1ret;

import { String, stringOf } from std.text;

struct Mixed { row: str[2]; tag: String; }
let calls: i32 = 0;
func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func get(): String { return Mixed { row: n1(), tag: stringOf("xy") }.tag; }

func main(): i32 { return i32(len(get())) - 2 + calls - 1; }
"#;

/// The element move empties the selected slot for both indices and the
/// assignment spelling; the whole-row, member, argument, and return controls
/// stay clean.
#[test]
fn should_move_a_call_filled_element_into_an_owning_binding() {
    for (tag, source) in [
        ("m7r1zero", M7_R1_ZERO_SOURCE),
        ("m7r1one", M7_R1_ONE_SOURCE),
        ("m7r1assign", M7_R1_ASSIGN_SOURCE),
        ("m7r1whole", M7_R1_WHOLE_SOURCE),
        ("m7r1member", M7_R1_MEMBER_SOURCE),
        ("m7r1arg", M7_R1_ARGUMENT_SOURCE),
        ("m7r1ret", M7_R1_RETURN_SOURCE),
    ] {
        let output = run_memory(tag, source);
        assert_no_asan(tag, &output);
        assert_exit(tag, &output, 0);
    }
}

// ---------------------------------------------------------------------------
// selfassignloans: an exact self-assignment keeps the loans of a copied view.
//
// A `str` local or a `str[N]` local copied from a loaned element carries that
// element's loan. Assigning the binding to itself copies the view onto itself,
// so the assignment is a no-op and the loan record must survive it; a later
// out-slot store is then the same escape the direct element argument reports.
// A read of the copy while its owner is alive names no escape and stays
// accepted. The direct element argument and the write-conflict cases elsewhere
// in this suite are the controls.
// ---------------------------------------------------------------------------

/// An exact self-assignment keeps the copied element's loan, so the later
/// out-slot store reports one `E10030` as the direct element argument does.
///
/// The single-view and the array forms are accepted at the head and read the
/// released storage. `selfassignsingle` and `selfassignarray` expect the
/// rejection at their `fill`; `selfassigninscope` reads both copies while the
/// owner is alive and must run 0 with no memory report; `selfassigndirect`
/// keeps the direct element argument's report.
#[test]
fn should_keep_the_loans_of_an_exact_self_assignment() {
    let control = r#"module spec.cpr.selfassigndirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(xs[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "selfassigndirect",
        control,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let in_scope = r#"module spec.cpr.selfassigninscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    s = s;
    let ys: str[2] = xs;
    ys = ys;
    return i32(s[0]) + i32(ys[0][0]) - (97 + 97);
}

func main(): i32 { return outer(); }
"#;
    let output = run_memory("selfassigninscope", in_scope);
    assert_no_asan("selfassigninscope", &output);
    assert_exit("selfassigninscope", &output, 0);

    let single = r#"module spec.cpr.selfassignsingle;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    s = s;
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "selfassignsingle",
        single,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let array = r#"module spec.cpr.selfassignarray;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(borrow slot: str?, ys: str[2]): void { slot = ys[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = xs;
    ys = ys;
    fill(borrow slot, ys);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "selfassignarray",
        array,
        "E10034",
        7,
        50,
        &["cannot release or store borrowed parameter 'ys'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// compositeelementstore: a composite copy into an element, a projected
// element, or a runtime element keeps every source position.
//
// `grid[0] = c ? xs : zs`, `out2[0] = c ? xs[0] : zs[0]`, and
// `g[0] = [xs[0], xs[1]]` copy views into a destination element. The
// destination's loan record must be the union of the source positions, so a
// later read of a loaned element reports one `E10030` at its out-slot store,
// whether the read is direct or through a local copy. A runtime destination
// `grid[k]` cannot name its element, so it records a wildcard position that
// matches a constant read under the same path. A constant store into a sibling
// element loans only that element; a read of the sibling stays accepted and
// clean, and the copies stay accepted.
// ---------------------------------------------------------------------------

/// A composite copy into an element, a projected element, or a runtime element
/// reports one `E10030` at the later out-slot store, while the constant
/// sibling control stays accepted and clean.
#[test]
fn should_keep_the_loans_of_a_composite_element_store() {
    let sibling = r#"module spec.cpr.compsibling;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let ys: str[2] = ["AA", "BB"];
    ys[0] = a.v;
    let s: str = ys[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 66;
}
"#;
    compile_rejects("compsibling", sibling, "E10034", 1);

    let in_scope = r#"module spec.cpr.compinscope;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func outer(c: bool, k: i32): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = a.w;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    let s1: str = grid[0][0];
    let out2: str[2] = ["EE", "FF"];
    out2[0] = c ? xs[0] : zs[1];
    let s2: str = out2[0];
    let g: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    g[0] = [xs[0], xs[1]];
    let s3: str = g[0][0];
    let grid2: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid2[k] = c ? xs : zs;
    let s4: str = grid2[1][0];
    return i32(s1[0]) + i32(s2[0]) + i32(s3[0]) + i32(s4[0]) - (97 + 97 + 97 + 97);
}

func main(): i32 { return outer(true, 1); }
"#;
    let output = run_memory("compinscope", in_scope);
    assert_no_asan("compinscope", &output);
    assert_exit("compinscope", &output, 0);

    let conditional_row = r#"module spec.cpr.compcondrow;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[0] = a.w;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    fill(grid[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "compcondrow",
        conditional_row,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let projected = r#"module spec.cpr.compprojected;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[0] = a.w;
    let out2: str[2] = ["EE", "FF"];
    out2[0] = c ? xs[0] : zs[0];
    fill(out2[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "compprojected",
        projected,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let literal = r#"module spec.cpr.complitarray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let g: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    g[0] = [xs[0], xs[1]];
    fill(g[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "complitarray",
        literal,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let runtime_direct = r#"module spec.cpr.compruntimedirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    fill(grid[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "compruntimedirect",
        runtime_direct,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let runtime_copy = r#"module spec.cpr.compruntimecopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    let s: str = grid[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "compruntimecopy",
        runtime_copy,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// deeperprojectioncopy: a read one projection below a copied row keeps the
// row's loan.
//
// `rows[0] = xs` copies the loaned elements of `xs` into the row `rows[0]`.
// A later read of `rows[0][0]` is one projection below the copied row, so the
// copy transfer must keep the recorded position that covers the read. The
// direct read of the same element reports the escape already; the copy must
// report it at the same point.
// ---------------------------------------------------------------------------

/// A copy read one projection deeper than the copied row keeps the loan, so
/// its out-slot store reports one `E10030` as the direct read does.
#[test]
fn should_keep_the_loans_of_a_deeper_projection_copy() {
    let direct = r#"module spec.cpr.deeperprojdirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    rows[0] = xs;
    fill(rows[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "deeperprojdirect",
        direct,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let copy = r#"module spec.cpr.deeperprojcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    rows[0] = xs;
    let s: str = rows[0][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "deeperprojcopy",
        copy,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// elementstorenestedfill: an element store of a fill literal keeps the
// literal value's loan.
//
// `g[1] = [; a.v]` stores a view of the local `a` in every element of the
// constant destination element, and `g[k] = [; a.v]` does the same through a
// runtime index. A later constant read of `g[1][0]`, direct or through a local
// copy, reads the loaned view, so the out-slot store reports one `E10030` at
// the fill. The constant destination is the direct-read control; the runtime
// destination records a wildcard position that reaches the constant read.
// ---------------------------------------------------------------------------

/// An element store whose value is a fill literal keeps the value's loan, so a
/// later constant read reports one `E10030` whether the destination index is
/// constant or runtime.
#[test]
fn should_keep_the_loans_of_an_element_store_fill() {
    let direct = r#"module spec.cpr.elemstoreconstdirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
    g[1] = [; a.v];
    fill(g[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "elemstoreconstdirect",
        direct,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let constant_copy = r#"module spec.cpr.elemstoreconstcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
    g[1] = [; a.v];
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "elemstoreconstcopy",
        constant_copy,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let runtime_copy = r#"module spec.cpr.elemstoreruntimecopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
    g[k] = [; a.v];
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "elemstoreruntimecopy",
        runtime_copy,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// runtimeoverwrite: a clean store after a run-time loan, and a clean
// run-time store before a safe read.
//
// A run-time element store cannot name the slot it wrote, so its loan is
// recorded per candidate element of the destination's fixed extent. A later
// constant store replaces exactly its own candidate slot, and a clean
// run-time store adds no loan, so both safe reads stay accepted. A read under
// a genuinely different prefix names no candidate and stays accepted too;
// an open destination extent keeps one conservative wildcard record.
// ---------------------------------------------------------------------------

/// A clean constant overwrite clears the candidate slot its run-time loan may
/// have written, so the later read of that slot stays accepted and clean.
///
/// `runtimeoverwrite` writes a conditional row through a run-time index and
/// then overwrites the first row with literals before the fill.
/// `cleanruntimestore` loans the first row, writes literals through a
/// run-time index, and reads the second row, which the loan never named.
#[test]
fn should_accept_a_clean_store_after_a_runtime_loan() {
    let overwrite = r#"module spec.cpr.runtimeoverwrite;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    grid[0] = ["II", "JJ"];
    fill(grid[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 73;
}
"#;
    compile_rejects("runtimeoverwrite", overwrite, "E10034", 1);

    let clean_store = r#"module spec.cpr.runtimeclean;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    grid[k] = ["II", "JJ"];
    fill(grid[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 73;
}
"#;
    compile_rejects("runtimeclean", clean_store, "E10034", 1);

    let overwrite_copy = r#"module spec.cpr.runtimeoverwritecopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    grid[0] = ["II", "JJ"];
    let s: str = grid[0][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 73;
}
"#;
    compile_rejects("runtimeoverwritecopy", overwrite_copy, "E10034", 1);

    let clean_copy = r#"module spec.cpr.runtimecleancopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    grid[k] = ["II", "JJ"];
    let s: str = grid[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 73;
}
"#;
    compile_rejects("runtimecleancopy", clean_copy, "E10034", 1);
}

/// A read under another prefix names no candidate slot the run-time store may
/// have written, so it stays accepted and clean.
///
/// `otherprefix` writes one element below the constant first row and reads the
/// second row; `otherbinding` writes a full row through a run-time index and
/// reads a different array entirely.
#[test]
fn should_accept_a_read_under_another_prefix() {
    let other_prefix = r#"module spec.cpr.otherprefix;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: static str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0][k] = xs[0];
    fill("GG", borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 71;
}
"#;
    let output = run_memory("otherprefix", other_prefix);
    assert_exit("otherprefix", &output, 0);
    assert_no_asan("otherprefix", &output);

    let other_binding = r#"module spec.cpr.otherbinding;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: static str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    let rows2: str[2][2] = [["II", "JJ"], ["KK", "LL"]];
    rows[k] = xs;
    fill("LL", borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 76;
}
"#;
    let output = run_memory("otherbinding", other_binding);
    assert_exit("otherbinding", &output, 0);
    assert_no_asan("otherbinding", &output);
}

/// A slice destination has no fixed extent, so its run-time store keeps one
/// conservative wildcard record and the later constant read reports.
#[test]
fn should_keep_the_wildcard_for_an_open_extent() {
    let source = r#"module spec.cpr.openextent;

import { String, stringOf } from std.text;
import { slice } from std.mem;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let base: str[2] = ["AA", "BB"];
    let xs: str[] = slice(base, 0, 2);
    xs[k] = a.v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "openextent",
        source,
        "E10034",
        8,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// runtimestorecopy: a constant read of an element a run-time store may have
// written keeps that store's loan.
//
// A run-time index cannot name the element it writes, so the store records the
// loan over every candidate slot of the destination's fixed extent. A later
// constant read of such a slot copies the loaned view into a local, and the
// out-slot store reports one `E10030` at the fill. A constant-index store names
// its own element, so a read of the sibling element stays accepted and clean,
// while a direct read of the written element keeps its report. A direct read
// after a run-time store reports for the same reason as the copy: the read
// selects storage the store may have loaned.
// ---------------------------------------------------------------------------

/// A run-time store's loan reaches a constant-read copy, and the direct read
/// and constant-index controls keep their reports while the sibling read stays
/// accepted.
///
/// `runtimestorecopyone` and `runtimestorecopyzero` write the loaned view
/// through a run-time index and copy the constant element `xs[1]`; both report
/// the conservative rejection, since the compiler cannot know the index. The
/// constant store and the two direct reads report at their own fills. The
/// sibling read after a constant store names no loan and runs 0.
#[test]
fn should_keep_the_loans_of_a_runtime_store_copy() {
    let k_one = r#"module spec.cpr.runtimestorecopyone;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    let s: str = xs[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "runtimestorecopyone",
        k_one,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let k_zero = r#"module spec.cpr.runtimestorecopyzero;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    let s: str = xs[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 0);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "runtimestorecopyzero",
        k_zero,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let constant = r#"module spec.cpr.runtimestoreconstant;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = a.v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "runtimestoreconstant",
        constant,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let direct_one = r#"module spec.cpr.runtimestorereadone;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "runtimestorereadone",
        direct_one,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let direct_runtime = r#"module spec.cpr.runtimestorereadruntime;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    fill(xs[k], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "runtimestorereadruntime",
        direct_runtime,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let sibling = r#"module spec.cpr.runtimestoresibling;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let ys: str[2] = ["AA", "BB"];
    ys[0] = a.v;
    let s: str = ys[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 66;
}
"#;
    compile_rejects("runtimestoresibling", sibling, "E10034", 1);
}

// ---------------------------------------------------------------------------
// temporaryelementcopy: a view of a statement temporary stored in a
// fixed-array literal keeps the temporary's loan.
//
// `[mk().v; "EE"]` fills the array with views of the call result's member, and
// the call result is a statement temporary. The element copy must record that
// temporary as the loan's owner, so a later out-slot store of the element
// reports one `E10030` at the fill. The direct argument reports the same
// escape, and a length read while the array is alive stays accepted and clean.
// ---------------------------------------------------------------------------

/// A view of a statement temporary stored in a fixed-array literal reports
/// one `E10030` at the later fill, as the direct argument does.
///
/// `temporaryelementcopy` copies the temporary-derived view into the array and
/// fills from the element; `temporaryelementdirect` passes the same view
/// straight to the callee. Both report one `E10030`; the in-scope length read
/// names no escape and runs 0.
#[test]
fn should_keep_the_loans_of_a_temporary_element_copy() {
    let copy = r#"module spec.cpr.temporaryelementcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let ys: str[2] = [mk().v; "EE"];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "temporaryelementcopy",
        copy,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let direct = r#"module spec.cpr.temporaryelementdirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    fill(mk().v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
        compile_rejects_once_at(
        "temporaryelementdirect",
        direct,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let in_scope = r#"module spec.cpr.temporaryelementinscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func main(): i32 {
    let ys: str[2] = [mk().v; "EE"];
    return i32(len(ys[0])) - 2;
}
"#;
    let output = run_memory("temporaryelementinscope", in_scope);
    assert_no_asan("temporaryelementinscope", &output);
    assert_exit("temporaryelementinscope", &output, 0);
}

// ---------------------------------------------------------------------------
// callviewresult: a view returned by a call keeps the loan of the storage it
// reads.
//
// `pick(xs, 0)` returns the element view `xs[0]`, whose storage belongs to the
// loaned local `a`. The call result's binding loans name that storage, so both
// `let s = pick(xs, 0); fill(s, ...)` and `fill(pick(xs, 0), ...)` report one
// `E10030` at the fill. The direct element fill keeps its report. A view
// returned from a caller parameter is backed by the caller's storage, which
// outlives the frame, and stays accepted; a length read while the owner is
// alive stays accepted and clean.
// ---------------------------------------------------------------------------

/// A call-mediated element view reports one `E10030` at each fill, while the
/// direct control keeps its report and a caller-backed view stays accepted.
///
/// `callviewcopy` copies the call result before the fill and `callviewdirect`
/// passes the call straight to the callee; both report the escaping loan.
/// `callviewcontrol` reads the element directly and keeps its report.
/// `callviewparam` returns a view of a caller parameter and runs 0, and
/// `callviewinscope` reads the result length while the owner is alive.
#[test]
fn should_keep_the_loans_of_a_call_view_result() {
    let copy = r#"module spec.cpr.callviewcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = pick(xs, 0);
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "callviewcopy",
        copy,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let direct = r#"module spec.cpr.callviewdirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(pick(xs, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "callviewdirect",
        direct,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let control = r#"module spec.cpr.callviewcontrol;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(xs[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "callviewcontrol",
        control,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let caller_parameter = r#"module spec.cpr.callviewparam;

func pass(p: str): str { return p; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: str, borrow slot: str?): void {
    let s: str = pass(p);
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    compile_rejects("callviewparam", caller_parameter, "E10034", 1);

    let in_scope = r#"module spec.cpr.callviewinscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = pick(xs, 0);
    return i32(len(s)) - 2;
}

func main(): i32 { return outer(); }
"#;
    let output = run_memory("callviewinscope", in_scope);
    assert_no_asan("callviewinscope", &output);
    assert_exit("callviewinscope", &output, 0);
}

// ---------------------------------------------------------------------------
// containerunion: a call-mediated view over a mixed-backing container carries
// the container's loan union.
//
// The modular call model gives a call result no positional precision: when a
// view is returned from a container, the result carries the loans recorded on
// the container, which are the union over every element. A later fill of any
// element therefore reports the loaned sibling's owner, even when the read
// selects an element that is clean. The soundness stance allows this
// conservative report (Ch 16 section 9: when the checker cannot prove a loan
// safe, it rejects), and a container with no loan keeps its acceptance.
// ---------------------------------------------------------------------------

/// A call result over a container with a loaned sibling reports that sibling's
/// owner at the fill, while a container with no loan stays accepted and clean.
///
/// `containerunionpick` reads the clean element `xs[1]` after `xs[0]` holds a
/// view of `a`, and `containerunionarray` copies the whole container through a
/// call and reads its second element. Both report one `E10030` naming `a`, not
/// the container `xs`, because the call result carries the container's union.
/// `containerunionclean` is the acceptance control: it has no loaned sibling
/// and runs 0.
#[test]
fn should_report_the_conservative_container_union() {
    let pick = r#"module spec.cpr.containerunionpick;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(pick(xs, 1), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 66;
}
"#;
        compile_rejects_once_at(
        "containerunionpick",
        pick,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let array = r#"module spec.cpr.containerunionarray;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pickArr(xs: str[2]): str[2] { return xs; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = pickArr(xs);
    fill(ys[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 66;
}
"#;
        compile_rejects_once_at(
        "containerunionarray",
        array,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let clean = r#"module spec.cpr.containerunionclean;

func pick(xs: str[2], i: i32): str { return xs[i]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    fill(pick(xs, 1), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 66;
}
"#;
    compile_rejects("containerunionclean", clean, "E10034", 1);
}

// ---------------------------------------------------------------------------
// takesites: a taken temporary element reports at both stores.
//
// A temporary-derived view handed to a `take` parameter escapes twice: the
// callee stores the take parameter's view into the caller's out-slot, and the
// caller's statement temporary backs the argument. The array spelling
// `take ys[0]` reaches both sites exactly as the direct `take mk().v` twin
// does, so the two-report parity is the contract for both spellings.
// ---------------------------------------------------------------------------

/// A taken temporary element and its direct twin each report twice: once at
/// the callee's take-parameter store and once at the caller's call.
///
/// `takesiteselement` passes `take ys[0]` from an array filled with views of
/// `mk()`; `takesitesdirect` is the control that passes `take mk().v`. Both
/// carry one report at the callee store and one at the caller call, naming the
/// take parameter and the statement temporary respectively.
#[test]
fn should_report_both_sites_of_a_taken_temporary_element() {
    let element = r#"module spec.cpr.takesiteselement;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func consume(take p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let ys: str[2] = [mk().v; "EE"];
    consume(take ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects_at_sites(
        "takesiteselement",
        element,
        "E10030",
        &[(9, 54), (13, 5)],
        &[
            "borrow of 'p' does not live long enough",
            "borrow of a statement temporary does not live long enough",
        ],
    );

    let direct = r#"module spec.cpr.takesitesdirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func consume(take p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    consume(take mk().v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    compile_rejects_at_sites(
        "takesitesdirect",
        direct,
        "E10030",
        &[(9, 54), (12, 5)],
        &[
            "borrow of 'p' does not live long enough",
            "borrow of a statement temporary does not live long enough",
        ],
    );
}

// ---------------------------------------------------------------------------
// ownerphrase: a generated owner borrowed out of an element store renders as
// a statement temporary.
//
// An element store of `mk().v` records the generated owner under the element
// path. A later fill of that element is a recorded-position escape, and its
// message must use the same phrase as the direct `fill(mk().v)` value route:
// `a statement temporary`. The internal `__cl_` spelling is never shown.
// ---------------------------------------------------------------------------

/// The recorded-position and value-route escapes name the same phrase.
///
/// `ownerphraserecorded` stores a temporary-derived view into `ys[0]` and
/// fills from that element; `ownerphrasevalue` passes `mk().v` straight to the
/// callee. The recorded-position case is the discriminator and the value-route
/// case is the control. Both report one `E10030` naming `a statement
/// temporary`, and neither mentions `a compiler-generated owner` or an
/// internal name.
#[test]
fn should_render_the_statement_temporary_phrase_for_recorded_and_value_routes() {
    let recorded = r#"module spec.cpr.ownerphraserecorded;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let ys: str[2] = ["AA", "BB"];
    ys[0] = mk().v;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "ownerphraserecorded",
        recorded,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let value = r#"module spec.cpr.ownerphrasevalue;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    fill(mk().v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
        compile_rejects_once_at(
        "ownerphrasevalue",
        value,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// assignedcopy: an assignment and its fill and nested spellings keep the
// lowered owner.
//
// A `let` declaration is not the only store that copies a temporary-derived
// element view: `ys = [mk().v; "EE"]` replaces the array through the assignment
// path, and the array-literal fill spelling `["EE"; mk().v]` stores the same
// view. A nested call `pass(pick(xs, 0))` forwards a call result's binding
// loans through another call. Every spelling keeps the owner, so a later fill
// reports the escape; an in-scope read of the assigned element stays accepted
// and clean.
// ---------------------------------------------------------------------------

/// The assignment, fill-value, and nested call spellings keep their loans.
///
/// `assignedcopy` assigns a temporary-derived array to an existing binding
/// and fills from its first element; `assignedfill` uses the fill value of an
/// array literal; both report one `E10030` naming the statement temporary.
/// `assignednested` forwards a call result through `pass` and reports one
/// `E10030` naming the loaned binding `a`, not the container `xs`.
/// `assignedinscope` reads the assigned element while the owner is alive and
/// runs 0.
#[test]
fn should_keep_the_loans_of_an_assigned_and_nested_temporary_view() {
    let assigned = r#"module spec.cpr.assignedcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let ys: str[2] = ["AA", "BB"];
    ys = [mk().v; "EE"];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "assignedcopy",
        assigned,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let fill = r#"module spec.cpr.assignedfill;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let ys: str[2] = ["EE"; mk().v];
    fill(ys[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "assignedfill",
        fill,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let nested = r#"module spec.cpr.assignednested;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func pass(p: str): str { return p; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(pass(pick(xs, 0)), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
        compile_rejects_once_at(
        "assignednested",
        nested,
        "E10034",
        11,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let in_scope = r#"module spec.cpr.assignedinscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func main(): i32 {
    let ys: str[2] = ["AA", "BB"];
    ys = [mk().v; "EE"];
    return i32(len(ys[0])) - 2;
}
"#;
    let output = run_memory("assignedinscope", in_scope);
    assert_no_asan("assignedinscope", &output);
    assert_exit("assignedinscope", &output, 0);
}

// ---------------------------------------------------------------------------
// rawderefprojection: an owning member of a raw pointee moves through the
// projection, not only through a top-level dereference.
//
// `(*p).s` names a member of the caller's pointee. Consuming that member must
// pin the pointer once and empty the selected member, whether the value flows
// into a binding, a return, a struct-literal field, or a `take` argument. A
// copy left in place lets both the destination and the pointee release the
// same storage. The conditional arm is the control: the batch-M arm store
// already moves the chosen member, so it stays green.
// ---------------------------------------------------------------------------

/// The store, return, field, and take forms each move the pointee once.
///
/// Every program reads `a.s` through a raw pointer and consumes the member.
/// The member must be emptied at the consuming position, so the pointee's own
/// scope release finds nothing; a copy left in place frees the same storage a
/// second time. The run exits 0 and reports no memory error. `p437arm` is the
/// conditional-arm control and keeps its batch-M behavior.
#[test]
fn should_move_a_member_projection_of_a_raw_pointee() {
    let store = r#"module spec.cpr.p437store;

import { String, stringOf } from std.text;

struct S { s: String; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let v: String = (*p).s;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p437store", store);
    assert_no_asan("p437store", &output);
    assert_exit("p437store", &output, 0);

    let returned = r#"module spec.cpr.p437return;

import { String, stringOf } from std.text;

struct S { s: String; }

func takeS(p: raw S): String {
    unsafe { return (*p).s; }
}

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let v: String = takeS(p);
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p437return", returned);
    assert_no_asan("p437return", &output);
    assert_exit("p437return", &output, 0);

    let field = r#"module spec.cpr.p437field;

import { String, stringOf } from std.text;

struct S { s: String; }
struct Holder { t: String; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let h: Holder = Holder { t: (*p).s };
        return i32(len(h.t)) - 2;
    }
}
"#;
    let output = run_memory("p437field", field);
    assert_no_asan("p437field", &output);
    assert_exit("p437field", &output, 0);

    let taken = r#"module spec.cpr.p437take;

import { String, stringOf } from std.text;

struct S { s: String; }

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let n: i32 = eat(take (*p).s);
        return n - 1;
    }
}
"#;
    let output = run_memory("p437take", taken);
    assert_no_asan("p437take", &output);
    assert_exit("p437take", &output, 0);

    let arm = r#"module spec.cpr.p437arm;

import { String, stringOf } from std.text;

struct S { s: String; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    let g: bool = true;
    unsafe {
        let p: raw S = &a;
        let v: String = g ? (*p).s : stringOf("zz");
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p437arm", arm);
    assert_no_asan("p437arm", &output);
    assert_exit("p437arm", &output, 0);
}

// ---------------------------------------------------------------------------
// guardedtargetalias: a guarded take through a raw pointer may consume the
// value a conditional store saved.
//
// The conditional store saves the old destination because its condition reads
// the destination through a `take`. When the pointer aliases the destination,
// the callee already freed the saved value, so the saved handle must be zeroed
// instead of released again. When the pointer does not alias, the save stays a
// real owner and is released normally. The false guard selection skips the take
// entirely and stores the else arm.
// ---------------------------------------------------------------------------

/// The aliasing take zeroes the consumed save; the non-aliasing take drops it.
///
/// `p439alias` takes `v` through `p = &v` and stores `x`; the taken value and
/// the save are one allocation, so the save must not be freed again.
/// `p439aliasfalse` skips the take and stores `y = "yyyy"`, so its run reads
/// the stored value (exit 2, not 0). `p439nonalias` takes `w` through `p = &w`
/// and still releases the old `v` through the save. `p439falsecallee` runs the
/// take but its call returns false, so the else arm stores `y` while the take
/// already freed the saved value. `p439orskip` nests the guarded take under a
/// disjunction whose left selects the then arm without running the take, so
/// the save must still release. `p439crosstype` takes an unrelated struct
/// through the pointer; the save is a different type and is released normally.
/// `p439membertarget` stores into the member the same take consumes.
/// `p439nestedarm` consumes the destination in a nested conditional arm.
/// `p439condname` uses a source binding named like the generated condition
/// local, and `p439castalias` reaches the destination through a raw pointer
/// cast of a different nominal type. Every run exits with its stated code and
/// reports no memory error.
#[test]
fn should_keep_the_consumed_save_of_a_guarded_aliasing_take() {
    let alias = r#"module spec.cpr.p439alias;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = (g && (eat(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439alias", alias);
    assert_no_asan("p439alias", &output);
    assert_exit("p439alias", &output, 0);

    let alias_false = r#"module spec.cpr.p439aliasfalse;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = false;
    unsafe {
        let p: raw String = &v;
        v = (g && (eat(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439aliasfalse", alias_false);
    assert_no_asan("p439aliasfalse", &output);
    assert_exit("p439aliasfalse", &output, 2);

    let nonalias = r#"module spec.cpr.p439nonalias;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let w: String = stringOf("ww");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &w;
        v = (g && (eat(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439nonalias", nonalias);
    assert_no_asan("p439nonalias", &output);
    assert_exit("p439nonalias", &output, 0);

    let false_callee = r#"module spec.cpr.p439falsecallee;

import { String, stringOf } from std.text;

func eat0(take x: String): i32 { return 0; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = (g && (eat0(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439falsecallee", false_callee);
    assert_no_asan("p439falsecallee", &output);
    assert_exit("p439falsecallee", &output, 2);

    let or_skip = r#"module spec.cpr.p439orskip;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g1: bool = false;
    let g2: bool = true;
    unsafe {
        let p: raw String = &v;
        v = ((g1 && (eat(take (unsafe *p)) == 1)) || g2) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439orskip", or_skip);
    assert_no_asan("p439orskip", &output);
    assert_exit("p439orskip", &output, 0);

    let cross_type = r#"module spec.cpr.p439crosstype;

import { String, stringOf } from std.text;

struct S { s: String; other: String; }

func eatS(take v: S): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let a: S = S { s: stringOf("aa"), other: stringOf("oo") };
    let g: bool = true;
    unsafe {
        let p: raw S = &a;
        v = (g && (eatS(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439crosstype", cross_type);
    assert_no_asan("p439crosstype", &output);
    assert_exit("p439crosstype", &output, 0);

    let member_target = r#"module spec.cpr.p439membertarget;

import { String, stringOf } from std.text;

struct S { name: String; other: String; }

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: S = S { name: stringOf("aa"), other: stringOf("oo") };
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw S = &a;
        a.name = (g && (eat(take (*p).name) == 1)) ? x : y;
        return i32(len(a.name)) - 2;
    }
}
"#;
    let output = run_memory("p439membertarget", member_target);
    assert_no_asan("p439membertarget", &output);
    assert_exit("p439membertarget", &output, 0);

    let nested_arm = r#"module spec.cpr.p439nestedarm;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let z: String = stringOf("zzzzzz");
    let g: bool = true;
    let d: bool = true;
    unsafe {
        let p: raw String = &v;
        v = g ? ((d && (eat(take (unsafe *p)) == 1)) ? x : y) : z;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439nestedarm", nested_arm);
    assert_no_asan("p439nestedarm", &output);
    assert_exit("p439nestedarm", &output, 0);

    let cond_name = r#"module spec.cpr.p439condname;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let __cl_cond: bool = false;
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = (g && (eat(take (unsafe *p)) == 1)) ? (__cl_cond ? x : y) : y;
        if (i32(len(v)) != 4) { return 9; }
        return 0;
    }
}
"#;
    let output = run_memory("p439condname", cond_name);
    assert_no_asan("p439condname", &output);
    assert_exit("p439condname", &output, 0);

    let cast_alias = r#"module spec.cpr.p439castalias;

import { String, stringOf } from std.text;

struct S { s: String; }

func eatS(take x: S): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw S = raw S(&v);
        v = (g && (eatS(take *p) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#;
    let output = run_memory("p439castalias", cast_alias);
    assert_no_asan("p439castalias", &output);
    assert_exit("p439castalias", &output, 0);
}

// ---------------------------------------------------------------------------
// guardedintrinsicderef: a borrowing intrinsic or borrow argument reads a
// raw-deref projection and binds nothing.
//
// `len(*alt(p, q))` and `read((*alt(p, q)).s)` only read the pointee. Binding
// the read into a released temporary hands the caller's storage to the
// statement cleanup, so the pointee's own release frees it a second time. The
// intrinsic loop must run twice, the argument call must read and return, and
// the impure `alt` must run once per evaluation.
// ---------------------------------------------------------------------------

/// The intrinsic and borrow-argument forms read the pointee without binding it.
///
/// `p440loop` and `p440plain` run a two-iteration loop whose condition reads
/// the pointee through `len`; `p440borrow` passes a projected read to a plain
/// (borrowing) parameter. The loop spelling of the same call stays a control.
/// Every run exits 0 and reports no memory error; the loop run also pins that
/// `alt` runs once per evaluated condition (twice, because the third test
/// short-circuits before it).
#[test]
fn should_leave_a_raw_deref_read_unbound_for_a_borrowing_intrinsic_or_argument() {
    let loop_cast = r#"module spec.cpr.p440loop;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let i: i32 = 0;
    let total: i32 = 0;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        while (i < 2 && (i32(len(*alt(p, q))) == 2)) {
            i = i + 1;
            total = total + 1;
        }
        return calls * 10 + i * 10 + total - 42;
    }
}
"#;
    let output = run_memory("p440loop", loop_cast);
    assert_no_asan("p440loop", &output);
    assert_exit("p440loop", &output, 0);

    let loop_plain = r#"module spec.cpr.p440plain;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let i: i32 = 0;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        while (i < 2 && len(*alt(p, q)) == 2) {
            i = i + 1;
        }
        return calls * 10 + i * 10 - 40;
    }
}
"#;
    let output = run_memory("p440plain", loop_plain);
    assert_no_asan("p440plain", &output);
    assert_exit("p440plain", &output, 0);

    let borrow = r#"module spec.cpr.p440borrow;

import { String, stringOf } from std.text;

struct S { s: String; }

let calls: i32 = 0;

func alt(s: raw S, t: raw S): raw S {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func read(p: String): i32 { return i32(len(p)); }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    let b: S = S { s: stringOf("bb") };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let n: i32 = read((*alt(p, q)).s);
        return n - 2;
    }
}
"#;
    let output = run_memory("p440borrow", borrow);
    assert_no_asan("p440borrow", &output);
    assert_exit("p440borrow", &output, 0);

    let loop_borrow = r#"module spec.cpr.p440loopborrow;

import { String, stringOf } from std.text;

struct S { s: String; }

let calls: i32 = 0;

func alt(s: raw S, t: raw S): raw S {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func read(p: String): i32 { return i32(len(p)); }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    let b: S = S { s: stringOf("bb") };
    let i: i32 = 0;
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        while (i < 2 && read((*alt(p, q)).s) == 2) {
            i = i + 1;
        }
        return calls * 10 + i * 10 - 40;
    }
}
"#;
    let output = run_memory("p440loopborrow", loop_borrow);
    assert_no_asan("p440loopborrow", &output);
    assert_exit("p440loopborrow", &output, 0);
}

// ---------------------------------------------------------------------------
// guardedmembertake: a guarded take of a member over a raw pointer moves only
// the selected member.
//
// `eat(take (*alt(p, q)).name)` hands the selected member to the callee, so the
// caller must empty that member and leave the sibling owned. Recording the
// whole pointee instead would empty the sibling too and lose its value; not
// recording the member at all double-frees it. The pointer expression runs once
// on the evaluated path, and the false guard selection never calls it.
// ---------------------------------------------------------------------------

/// The taken member empties, the sibling stays owned, and the false path skips
/// the call.
///
/// `p441member` takes `a.name` through an impure `alt`, then reads the sibling
/// `a.other` and the untouched `b.name`; the run exits 0 only when the take ran
/// once, the member was emptied, and the sibling survived. `p441false` skips
/// the guarded call and returns the call count (0). Both runs report no memory
/// error.
#[test]
fn should_move_only_the_taken_member_of_a_guarded_raw_pointee() {
    let member = r#"module spec.cpr.p441member;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

struct S { name: String; other: String; }

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: S = S { name: stringOf("aa"), other: stringOf("oo") };
    let b: S = S { name: stringOf("bb"), other: stringOf("pp") };
    let g: bool = true;
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let ok: bool = g && (eat(take (*alt(p, q)).name) == 1);
        if (!ok) { return 99; }
        if (calls != 1) { return 98; }
        if (i32(len(a.other)) != 2) { return 97; }
        return i32(len(b.name)) - 2;
    }
}
"#;
    let output = run_memory("p441member", member);
    assert_no_asan("p441member", &output);
    assert_exit("p441member", &output, 0);

    let guarded_false = r#"module spec.cpr.p441false;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

struct S { name: String; other: String; }

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: S = S { name: stringOf("aa"), other: stringOf("oo") };
    let b: S = S { name: stringOf("bb"), other: stringOf("pp") };
    let g: bool = false;
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let ok: bool = g && (eat(take (*alt(p, q)).name) == 1);
        if (ok) { return 99; }
        return calls;
    }
}
"#;
    let output = run_memory("p441false", guarded_false);
    assert_no_asan("p441false", &output);
    assert_exit("p441false", &output, 0);
}

/// Asserts a store trap prints `code` first on the plain and sanitizer builds.
///
/// The out-of-range store is the bounds contract: both builds must reach the
/// same trap line and the sanitizer build must not report a memory error
/// instead.
fn assert_store_traps(tag: &str, source: &str, code: &str) {
    for (build, output) in [
        ("plain", run_host(tag, source)),
        ("asan", run_asan(tag, source)),
    ] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr.lines().next().unwrap_or_default();
        assert!(
            first.contains(code),
            "{tag} ({build}): expected the {code} trap; stderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("AddressSanitizer"),
            "{tag} ({build}): the trap must not be a sanitizer report; stderr:\n{stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// derefrootedstore: a member or element store through a raw pointer reaches
// the pointee's selected storage.
//
// `(*p).f`, `(*p).f.x`, and `(*p).arr[i].x` are places (Ch 04 §4), so the store
// must be emitted, not dropped. The indexed form pins its index before the
// right-hand side: a call that changes the index still stores at the index the
// target named first and runs once. The conditional base and the structural
// `.len` target stay rejected.
// ---------------------------------------------------------------------------

/// The member, nested-index, and unsafe spellings store through the pointer.
///
/// Every program writes a value through a raw pointer and reads the stored
/// value back from the pointee, so a dropped store leaves the old value and
/// fails the run. `p421bump` additionally changes the index inside the
/// right-hand side and checks that the store used the pre-call index.
/// `p421condbase` and `p421len` keep their `E10221` rejection.
#[test]
fn should_store_a_deref_rooted_member_target() {
    let member = r#"module spec.cpr.p421member;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        (*p).f = Inner { x: 7 };
        (*p).f.x = 7;
    }
    return s.f.x - 7;
}
"#;
    let output = run_memory("p421member", member);
    assert_no_asan("p421member", &output);
    assert_exit("p421member", &output, 0);

    let nested = r#"module spec.cpr.p421nested;

struct Inner { x: i32; }
struct S { arr: Inner[2]; }

func main(): i32 {
    let s: S = S { arr: [Inner { x: 1 }, Inner { x: 2 }] };
    let i: i32 = 1;
    unsafe {
        let p: raw S = &s;
        (*p).arr[i].x = 7;
    }
    return s.arr[1].x - 7;
}
"#;
    let output = run_memory("p421nested", nested);
    assert_no_asan("p421nested", &output);
    assert_exit("p421nested", &output, 0);

    let unsafe_form = r#"module spec.cpr.p421unsafe;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        unsafe (*p).f = Inner { x: 7 };
        unsafe (*p).f.x = 7;
    }
    return s.f.x - 7;
}
"#;
    let output = run_memory("p421unsafe", unsafe_form);
    assert_no_asan("p421unsafe", &output);
    assert_exit("p421unsafe", &output, 0);

    let bumped = r#"module spec.cpr.p421bump;

struct Elem { y: i32; }
struct S { arr: Elem[2]; }

let idx: i32 = 1;
let calls: i32 = 0;

func bump(): i32 {
    idx = 5;
    calls = calls + 1;
    return 9;
}

func main(): i32 {
    let s: S = S { arr: [Elem { y: 1 }, Elem { y: 2 }] };
    unsafe {
        let p: raw S = &s;
        (*p).arr[idx].y = bump();
    }
    if (s.arr[1].y != 9) { return 9; }
    if (s.arr[0].y != 1) { return 8; }
    if (calls != 1) { return 7; }
    return 0;
}
"#;
    let output = run_memory("p421bump", bumped);
    assert_no_asan("p421bump", &output);
    assert_exit("p421bump", &output, 0);

    let conditional_base = r#"module spec.cpr.p421condbase;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let a: S = S { f: Inner { x: 1 } };
    let b: S = S { f: Inner { x: 2 } };
    let c: bool = true;
    (c ? a : b).f.x = 9;
    return 0;
}
"#;
    compile_rejects("p421condbase", conditional_base, "E10221", 1);

    let structural_len = r#"module spec.cpr.p421len;

struct S { arr: i32[2]; }

func main(): i32 {
    let s: S = S { arr: [1, 2] };
    unsafe {
        let p: raw S = &s;
        (*p).arr.len = 3;
    }
    return 0;
}
"#;
    compile_rejects("p421len", structural_len, "E10221", 1);
}

/// A whole-element store pins its base and index before the right-hand side.
///
/// `p423row` changes the target index inside the row call; the store must land
/// in the element the target named first. `p423cond`'s conditional element
/// source is a v1 element move, so the frozen rule rejects it with `E10026`;
/// `p423trap` keeps the out-of-range fault, and `p423member` is the pinned
/// member-route control.
#[test]
fn should_pin_the_whole_element_target_before_the_value() {
    let row = r#"module spec.cpr.p423row;

let idx: i32 = 1;

func mkrow(): i32[2] {
    idx = 5;
    return [7, 8];
}

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [3, 4]];
    a[idx] = mkrow();
    if (a[1][0] != 7 || a[1][1] != 8) { return 9; }
    if (a[0][0] != 1 || a[0][1] != 2) { return 8; }
    if (idx != 5) { return 7; }
    return 0;
}
"#;
    let output = run_memory("p423row", row);
    assert_no_asan("p423row", &output);
    assert_exit("p423row", &output, 0);

    let conditional = r#"module spec.cpr.p423cond;

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
    if (i32(len(a[0].name)) != 2) { return 9; }
    return 0;
}
"#;
    // The conditional element source is a v1 element move; the frozen rule
    // rejects it with E10026. The destination pin is covered by p423row and
    // p423member above.
    assert_rejected_once("p423cond", conditional, "E10026");

    let trap = r#"module spec.cpr.p423trap;

func main(): i32 {
    let a: i32[2] = [1, 2];
    let idx: i32 = 5;
    a[idx] = 9;
    return 0;
}
"#;
    assert_store_traps("p423trap", trap, "E11211");

    let member = r#"module spec.cpr.p423member;

struct Elem { y: i32; }

let idx: i32 = 0;

func set_idx(): i32 {
    idx = 1;
    return 7;
}

func main(): i32 {
    let a: Elem[2] = [Elem { y: 0 }, Elem { y: 0 }];
    a[idx].y = set_idx();
    if (a[0].y != 7) { return 9; }
    if (a[1].y != 0) { return 8; }
    return 0;
}
"#;
    let output = run_memory("p423member", member);
    assert_no_asan("p423member", &output);
    assert_exit("p423member", &output, 0);
}

// ---------------------------------------------------------------------------
// fixedarrayselfstore: a fixed-array literal reads the destination before the
// clear, and a self-copy or raw self-alias is a no-op.
//
// The literal's element expressions run before the destination is cleared, so
// a self-read sees the old values. `x = *(&x)` and `a[i].inner =
// *(&a[i].inner)` copy a place onto itself and must leave it intact, while a
// distinct source still moves. The generic constant-index self-copy is
// skipped; the concrete spelling keeps its `E10026` rejection.
// ---------------------------------------------------------------------------

/// The self-read, self-alias, and self-copy forms keep their values.
///
/// `p424selfread` and `p424two` read the destination while building its
/// replacement; `p424raw` self-assigns a whole member and a local through
/// `*(&...)` and then stores a distinct value; `p424distinct` and
/// `p424distinctplace` move a different place; `p424generic` hits the generic
/// constant-index self-copy. Every run exits 0 and is memory-clean.
#[test]
fn should_read_the_fixed_array_destination_before_clearing_it() {
    let self_read = r#"module spec.cpr.p424selfread;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let i: i32 = 1;
    a[i].names = [stringOf(a[i].names[0]), stringOf("xx")];
    if (i32(len(a[1].names[0])) != 2) { return 9; }
    if (i32(len(a[1].names[1])) != 2) { return 8; }
    return 0;
}
"#;
    let output = run_memory("p424selfread", self_read);
    assert_no_asan("p424selfread", &output);
    assert_exit("p424selfread", &output, 0);

    let two = r#"module spec.cpr.p424two;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let i: i32 = 1;
    a[i].names = [stringOf(a[i].names[0]), stringOf(a[i].names[1])];
    return (i32(len(a[1].names[0])) - 2) + (i32(len(a[1].names[1])) - 2);
}
"#;
    let output = run_memory("p424two", two);
    assert_no_asan("p424two", &output);
    assert_exit("p424two", &output, 0);

    let raw_member = r#"module spec.cpr.p424raw;

import { String, stringOf } from std.text;

struct Inner { a: String; b: String; }
struct Elem { inner: Inner; }

func main(): i32 {
    let a: Elem[2] = [
        Elem { inner: Inner { a: stringOf("aa"), b: stringOf("ab") } },
        Elem { inner: Inner { a: stringOf("bb"), b: stringOf("bc") } }
    ];
    let i: i32 = 1;
    unsafe {
        a[i].inner = *(&a[i].inner);
    }
    if (i32(len(a[1].inner.a)) != 2) { return 9; }
    if (i32(len(a[1].inner.b)) != 2) { return 8; }
    a[1].inner.a = stringOf("zz");
    if (i32(len(a[1].inner.a)) != 2) { return 7; }
    return 0;
}
"#;
    let output = run_memory("p424raw", raw_member);
    assert_no_asan("p424raw", &output);
    assert_exit("p424raw", &output, 0);

    let raw_local = r#"module spec.cpr.p424rawlocal;

import { String, stringOf } from std.text;

func main(): i32 {
    let x: String = stringOf("aa");
    unsafe {
        x = *(&x);
    }
    if (i32(len(x)) != 2) { return 9; }
    x = stringOf("zz");
    if (i32(len(x)) != 2) { return 8; }
    return 0;
}
"#;
    let output = run_memory("p424rawlocal", raw_local);
    assert_no_asan("p424rawlocal", &output);
    assert_exit("p424rawlocal", &output, 0);

    let distinct = r#"module spec.cpr.p424distinct;

import { String, stringOf } from std.text;

func main(): i32 {
    let x: String = stringOf("xx");
    let y: String = stringOf("yy");
    unsafe {
        x = *(&y);
    }
    return i32(len(x)) - 2;
}
"#;
    let output = run_memory("p424distinct", distinct);
    assert_no_asan("p424distinct", &output);
    assert_exit("p424distinct", &output, 0);

    let distinct_place = r#"module spec.cpr.p424distinctplace;

import { String, stringOf } from std.text;

struct Inner { a: String; b: String; }
struct Elem { inner: Inner; }

func main(): i32 {
    let a: Elem[2] = [
        Elem { inner: Inner { a: stringOf("aa"), b: stringOf("ab") } },
        Elem { inner: Inner { a: stringOf("bb"), b: stringOf("bc") } }
    ];
    unsafe {
        a[0].inner = *(&a[1].inner);
    }
    return i32(len(a[0].inner.a)) - 2;
}
"#;
    let output = run_memory("p424distinctplace", distinct_place);
    assert_no_asan("p424distinctplace", &output);
    assert_exit("p424distinctplace", &output, 0);

    let generic = r#"module spec.cpr.p424generic;

import { String, stringOf } from std.text;

struct W { items: String[2]; }

func same<T>(borrow w: T): void {
    w.items[1] = w.items[1];
}

func main(): i32 {
    let w: W = W { items: [stringOf("aa"), stringOf("bb")] };
    same<W>(borrow w);
    return i32(len(w.items[1])) - 2;
}
"#;
    let output = run_memory("p424generic", generic);
    assert_no_asan("p424generic", &output);
    assert_exit("p424generic", &output, 0);

    let concrete = r#"module spec.cpr.p424concrete;

import { String, stringOf } from std.text;

struct W { items: String[2]; }

func main(): i32 {
    let w: W = W { items: [stringOf("aa"), stringOf("bb")] };
    w.items[1] = w.items[1];
    return 0;
}
"#;
    compile_rejects("p424concrete", concrete, "E10026", 1);
}

// ---------------------------------------------------------------------------
// storematerialize: an owning raw-deref right-hand side is pinned once in the
// element and raw-target store paths.
//
// The element store and the raw-target store lower their right-hand side
// directly, so an impure `*alt(p, q)` was read once for the value and again
// for the empty. The shared materializer pins the pointer once; the raw-target
// form has one pointer expression on each side, so each runs once.
// ---------------------------------------------------------------------------

/// The element and raw-target stores evaluate each pointer expression once.
///
/// `p436element` stores an impure raw-deref read into a fixed-array element;
/// `p436raw` stores one through a second impure raw pointer. The named store
/// and a duplicable element are the controls. Every run exits 0 and is
/// memory-clean.
#[test]
fn should_materialize_the_owning_deref_store_value() {
    let element = r#"module spec.cpr.p436element;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let xs: String[2] = [stringOf("AA"), stringOf("BB")];
    let i: i32 = 1;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        xs[i] = *alt(p, q);
    }
    if (i32(len(xs[1])) != 2) { return 9; }
    if (calls != 1) { return 8; }
    return 0;
}
"#;
    let output = run_memory("p436element", element);
    assert_no_asan("p436element", &output);
    assert_exit("p436element", &output, 0);

    let raw_target = r#"module spec.cpr.p436raw;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let v: String = stringOf("vv");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let rp: raw String = &v;
        let rq: raw String = &v;
        unsafe *alt(rp, rq) = *alt(p, q);
    }
    if (i32(len(v)) != 2) { return 9; }
    if (calls != 2) { return 8; }
    return 0;
}
"#;
    let output = run_memory("p436raw", raw_target);
    assert_no_asan("p436raw", &output);
    assert_exit("p436raw", &output, 0);

    let named = r#"module spec.cpr.p436named;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let v: String = *alt(p, q);
        if (i32(len(v)) != 2) { return 9; }
    }
    if (calls != 1) { return 8; }
    return 0;
}
"#;
    let output = run_memory("p436named", named);
    assert_no_asan("p436named", &output);
    assert_exit("p436named", &output, 0);

    let duplicable = r#"module spec.cpr.p436dup;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: i32 = 3;
    let b: i32 = 4;
    unsafe {
        let p: raw i32 = &a;
        let q: raw i32 = &b;
        let xs: i32[2] = [1, 2];
        xs[0] = *alt(p, q);
        if (xs[0] != 3) { return 9; }
    }
    if (calls != 1) { return 8; }
    return 0;
}
"#;
    let output = run_memory("p436dup", duplicable);
    assert_no_asan("p436dup", &output);
    assert_exit("p436dup", &output, 0);
}

/// A raw field chain, the prefix-only unsafe form, and a view element store.
///
/// `p421rawfield` stores through a member reached through a raw-pointer field,
/// so the emitted C must render the intermediate `->`. `p421prefix` uses the
/// `unsafe` prefix on the target without an enclosing block. `p421view`
/// stores a view through a raw target and keeps the owner readable.
#[test]
fn should_store_through_a_raw_field_and_keep_a_view_owner_readable() {
    let raw_field = r#"module spec.cpr.p421rawfield;

struct Node { x: i32; next: raw Node; }

func main(): i32 {
    let b: Node = Node { x: 2, next: raw Node(0) };
    let a: Node = Node { x: 1, next: raw Node(0) };
    unsafe {
        a.next = &b;
        let p: raw Node = &a;
        (*p).next.x = 7;
    }
    return b.x - 7;
}
"#;
    let output = run_memory("p421rawfield", raw_field);
    assert_no_asan("p421rawfield", &output);
    assert_exit("p421rawfield", &output, 0);

    let prefix = r#"module spec.cpr.p421prefix;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        unsafe (*p).f.x = 7;
    }
    return s.f.x - 7;
}
"#;
    let output = run_memory("p421prefix", prefix);
    assert_no_asan("p421prefix", &output);
    assert_exit("p421prefix", &output, 0);

    let view = r#"module spec.cpr.p421view;

import { String, stringOf } from std.text;

struct S { vs: str[2]; n: i32; }

func main(): i32 {
    let t: String = stringOf("aa");
    let s: S = S { vs: ["", ""], n: 0 };
    unsafe {
        let p: raw S = &s;
        (*p).vs[1] = t;
    }
    if (i32(len(s.vs[1])) != 2) { return 9; }
    return i32(len(t)) - 2;
}
"#;
    let output = run_memory("p421view", view);
    assert_no_asan("p421view", &output);
    assert_exit("p421view", &output, 0);
}

/// Raw self-copies, pointer aliases, and runtime self-copies keep their checks.
///
/// `p424rawself` and `p424rawaddrof` copy a raw-deref target onto itself and
/// leave it intact; `p424rawalias` does the same through two pointers to the
/// same storage, while `p424rawdistinct` moves through a distinct pointer.
/// `p424runtimedistinct` moves between different runtime indices. The generic
/// self-copy still runs the bounds guard (`p424genbounds` traps), and the
/// read-only, borrow, and updating-index spellings keep their diagnostics.
#[test]
fn should_keep_the_raw_store_self_copies_and_checks_intact() {
    let raw_self = r#"module spec.cpr.p424rawself;

import { String, stringOf } from std.text;

struct S { name: String; }

func main(): i32 {
    let s: S = S { name: stringOf("aa") };
    unsafe {
        let p: raw S = &s;
        (*p).name = (*p).name;
    }
    return i32(len(s.name)) - 2;
}
"#;
    let output = run_memory("p424rawself", raw_self);
    assert_no_asan("p424rawself", &output);
    assert_exit("p424rawself", &output, 0);

    let raw_address_of = r#"module spec.cpr.p424rawaddrof;

import { String, stringOf } from std.text;

struct S { name: String; }

func main(): i32 {
    let s: S = S { name: stringOf("aa") };
    unsafe {
        let p: raw S = &s;
        (*p).name = *(&(*p).name);
    }
    return i32(len(s.name)) - 2;
}
"#;
    let output = run_memory("p424rawaddrof", raw_address_of);
    assert_no_asan("p424rawaddrof", &output);
    assert_exit("p424rawaddrof", &output, 0);

    let raw_alias = r#"module spec.cpr.p424rawalias;

import { String, stringOf } from std.text;

struct S { name: String; }

func main(): i32 {
    let s: S = S { name: stringOf("aa") };
    unsafe {
        let p: raw S = &s;
        let q: raw S = &s;
        (*p).name = (*q).name;
    }
    return i32(len(s.name)) - 2;
}
"#;
    let output = run_memory("p424rawalias", raw_alias);
    assert_no_asan("p424rawalias", &output);
    assert_exit("p424rawalias", &output, 0);

    let raw_distinct = r#"module spec.cpr.p424rawdistinct;

import { String, stringOf } from std.text;

struct S { name: String; }

func main(): i32 {
    let a: S = S { name: stringOf("aa") };
    let b: S = S { name: stringOf("bbbb") };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        (*p).name = (*q).name;
    }
    return i32(len(a.name)) - 4;
}
"#;
    let output = run_memory("p424rawdistinct", raw_distinct);
    assert_no_asan("p424rawdistinct", &output);
    assert_exit("p424rawdistinct", &output, 0);

    let runtime_distinct = r#"module spec.cpr.p424runtimedistinct;

import { String, stringOf } from std.text;

func main(): i32 {
    let xs: String[2] = [stringOf("AAAA"), stringOf("BB")];
    let i: i32 = 1;
    let j: i32 = 0;
    unsafe {
        xs[i] = *(&xs[j]);
    }
    return i32(len(xs[1])) - 4;
}
"#;
    let output = run_memory("p424runtimedistinct", runtime_distinct);
    assert_no_asan("p424runtimedistinct", &output);
    assert_exit("p424runtimedistinct", &output, 0);

    let fixed_alias = r#"module spec.cpr.p424fixedalias;

import { String, stringOf } from std.text;

struct S { items: String[2]; other: i32; }

func main(): i32 {
    let a: S = S { items: [stringOf("aa"), stringOf("ab")], other: 1 };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &a;
        (*p).items = (*q).items;
    }
    if (a.items[0] == null) { return 9; }
    return i32(len(a.items[0])) - 2;
}
"#;
    let output = run_memory("p424fixedalias", fixed_alias);
    assert_no_asan("p424fixedalias", &output);
    assert_exit("p424fixedalias", &output, 0);

    let call_alias = r#"module spec.cpr.p424callalias;

import { String, stringOf } from std.text;

struct S { name: String; }

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i % 2 == 0) { return s; }
    return t;
}

func main(): i32 {
    let s: S = S { name: stringOf("aa") };
    unsafe {
        let p: raw S = &s;
        (*alt(p, p)).name = (*alt(p, p)).name;
    }
    if (calls != 2) { return 8; }
    return i32(len(s.name)) - 2;
}
"#;
    let output = run_memory("p424callalias", call_alias);
    assert_no_asan("p424callalias", &output);
    assert_exit("p424callalias", &output, 0);

    let array_address_of = r#"module spec.cpr.p424arrayaddrof;

import { String, stringOf } from std.text;

struct S { items: String[2]; }

func main(): i32 {
    let a: S = S { items: [stringOf("aa"), stringOf("bb")] };
    unsafe {
        let p: raw S = &a;
        (*p).items = *(&a.items);
    }
    return (i32(len(a.items[0])) - 2) + (i32(len(a.items[1])) - 2);
}
"#;
    let output = run_memory("p424arrayaddrof", array_address_of);
    assert_no_asan("p424arrayaddrof", &output);
    assert_exit("p424arrayaddrof", &output, 0);

    let generic_runtime = r#"module spec.cpr.p424genruntime;

import { String, stringOf } from std.text;

struct W { items: String[2]; }

func same<T>(borrow w: T, i: i32): i32 {
    w.items[i] = w.items[i];
    return i32(len(w.items[i]));
}

func main(): i32 {
    let w: W = W { items: [stringOf("aa"), stringOf("bb")] };
    return same<W>(borrow w, 1) - 2;
}
"#;
    let output = run_memory("p424genruntime", generic_runtime);
    assert_no_asan("p424genruntime", &output);
    assert_exit("p424genruntime", &output, 0);

    let generic_bounds = r#"module spec.cpr.p424genbounds;

import { String, stringOf } from std.text;

struct W { items: String[2]; }

func same<T>(borrow w: T, i: i32): void {
    w.items[i] = w.items[i];
}

func main(): i32 {
    let w: W = W { items: [stringOf("aa"), stringOf("bb")] };
    same<W>(borrow w, 5);
    return 0;
}
"#;
    assert_store_traps("p424genbounds", generic_bounds, "E11211");

    let read_only = r#"module spec.cpr.p424readonly;

import { String, stringOf } from std.text;

struct W { items: String[2]; }

func same<T>(w: T): void {
    w.items[1] = w.items[1];
}

func main(): i32 {
    let w: W = W { items: [stringOf("aa"), stringOf("bb")] };
    same<W>(w);
    return 0;
}
"#;
    compile_rejects("p424readonly", read_only, "E10024", 1);

    let borrow = r#"module spec.cpr.p424borrow;

import { String, stringOf } from std.text;

func wipe(borrow p: String): void {
    unsafe {
        p = *(&p);
    }
}

func main(): i32 {
    let s: String = stringOf("aa");
    wipe(borrow s);
    return 0;
}
"#;
    compile_rejects("p424borrow", borrow, "E10034", 1);

    let updating = r#"module spec.cpr.p424update;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let i: i32 = 0;
    a[i++].names = a[i++].names;
    return i;
}
"#;
    compile_rejects("p424update", updating, "E10026", 1);
}

// ---------------------------------------------------------------------------
// genericdivision: a compound division or remainder inside a generic body
// keeps the same zero and minimum guards as its concrete twin.
//
// The operand's recorded type is a projection of the type parameter, so the
// guard must resolve it through the instance before deciding whether the
// operation is integer division. Both the generic and concrete twins then
// trap `E11210` for zero and for `MIN / -1` / `MIN % -1`.
// ---------------------------------------------------------------------------

/// The generic and concrete compound divisions trap with the same guard.
///
/// `p426generic` and `p426concrete` divide by two and return the same value.
/// Their zero, `MIN / -1`, and `MIN % -1` twins each trap `E11210` on the
/// plain and sanitizer builds.
#[test]
fn should_guard_a_generic_compound_division_and_remainder() {
    let generic = r#"module spec.cpr.p426generic;

import { String, stringOf } from std.text;

struct H { n: i32; items: i32[2]; name: String; }

func go<T>(borrow h: T, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
    h.items[1] /= d;
}

func main(): i32 {
    let h: H = H { n: 10, items: [11, 12], name: stringOf("h") };
    go<H>(borrow h, 2);
    return h.n + h.items[0] + h.items[1] - 12;
}
"#;
    let output = run_memory("p426generic", generic);
    assert_no_asan("p426generic", &output);
    assert_exit("p426generic", &output, 0);

    let concrete = r#"module spec.cpr.p426concrete;

import { String, stringOf } from std.text;

struct H { n: i32; items: i32[2]; name: String; }

func go(borrow h: H, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
    h.items[1] /= d;
}

func main(): i32 {
    let h: H = H { n: 10, items: [11, 12], name: stringOf("h") };
    go(borrow h, 2);
    return h.n + h.items[0] + h.items[1] - 12;
}
"#;
    let output = run_memory("p426concrete", concrete);
    assert_no_asan("p426concrete", &output);
    assert_exit("p426concrete", &output, 0);

    let generic_zero = r#"module spec.cpr.p426genzero;

struct H { n: i32; items: i32[2]; }

func go<T>(borrow h: T, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
}

func main(): i32 {
    let h: H = H { n: 10, items: [11, 12] };
    go<H>(borrow h, 0);
    return 0;
}
"#;
    assert_store_traps("p426genzero", generic_zero, "E11210");

    let concrete_zero = r#"module spec.cpr.p426conczero;

struct H { n: i32; items: i32[2]; }

func go(borrow h: H, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
}

func main(): i32 {
    let h: H = H { n: 10, items: [11, 12] };
    go(borrow h, 0);
    return 0;
}
"#;
    assert_store_traps("p426conczero", concrete_zero, "E11210");

    let generic_min = r#"module spec.cpr.p426genmin;

struct H { n: i32; items: i32[2]; }

func go<T>(borrow h: T, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
}

func main(): i32 {
    let min: i32 = i32(u32(0x80000000));
    let h: H = H { n: min, items: [11, 12] };
    go<H>(borrow h, -1);
    return 0;
}
"#;
    assert_store_traps("p426genmin", generic_min, "E11210");

    let generic_min_rem = r#"module spec.cpr.p426genminrem;

struct H { n: i32; items: i32[2]; }

func go<T>(borrow h: T, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
}

func main(): i32 {
    let min: i32 = i32(u32(0x80000000));
    let h: H = H { n: 10, items: [min, 12] };
    go<H>(borrow h, -1);
    return 0;
}
"#;
    assert_store_traps("p426genminrem", generic_min_rem, "E11210");

    let concrete_min = r#"module spec.cpr.p426concmin;

struct H { n: i32; items: i32[2]; }

func go(borrow h: H, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
}

func main(): i32 {
    let min: i32 = i32(u32(0x80000000));
    let h: H = H { n: min, items: [11, 12] };
    go(borrow h, -1);
    return 0;
}
"#;
    assert_store_traps("p426concmin", concrete_min, "E11210");

    let concrete_min_rem = r#"module spec.cpr.p426concminrem;

struct H { n: i32; items: i32[2]; }

func go(borrow h: H, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
}

func main(): i32 {
    let min: i32 = i32(u32(0x80000000));
    let h: H = H { n: 10, items: [min, 12] };
    go(borrow h, -1);
    return 0;
}
"#;
    assert_store_traps("p426concminrem", concrete_min_rem, "E11210");
}

// ---------------------------------------------------------------------------
// literaldestinations: a conditional literal read base, a take conditional
// literal in an eager ternary arm, and a take of a conditional array literal
// bind their hidden fixed-array destinations.
//
// A `str[2]` field is returned through a hidden destination, so a literal
// reachable through a conditional must select the per-arm store form. The
// conditional array of view places is stored element by element into a bound
// destination before the take call.
// ---------------------------------------------------------------------------

/// The conditional literal read bases store each arm through its destination.
///
/// `p445both` and `p445nested` build both arms; `p445mixed` mixes a place arm
/// with a literal arm. Each run returns the chosen arm's first byte and calls
/// the chosen arm's function once.
#[test]
fn should_bind_the_conditional_literal_read_base() {
    let both = r#"module spec.cpr.p445both;

struct Inner { row: str[2]; }

func n1(): str[2] { return ["AA", "BB"]; }
func n2(): str[2] { return ["CC", "DD"]; }

func use(ok: bool): i32 {
    return i32((ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]);
}

func main(): i32 {
    return (use(true) - 65) + (use(false) - 67);
}
"#;
    let output = run_memory("p445both", both);
    assert_no_asan("p445both", &output);
    assert_exit("p445both", &output, 0);

    let nested = r#"module spec.cpr.p445nested;

struct Inner { row: str[2]; }
struct Outer { inner: Inner; }

func n1(): str[2] { return ["AA", "BB"]; }
func n2(): str[2] { return ["CC", "DD"]; }

func use(ok: bool): i32 {
    let o: Outer = Outer { inner: ok ? Inner { row: n1() } : Inner { row: n2() } };
    return i32(o.inner.row[0][0]);
}

func main(): i32 {
    return (use(true) - 65) + (use(false) - 67);
}
"#;
    let output = run_memory("p445nested", nested);
    assert_no_asan("p445nested", &output);
    assert_exit("p445nested", &output, 0);

    let mixed = r#"module spec.cpr.p445mixed;

struct Inner { row: str[2]; }

func n2(): str[2] { return ["CC", "DD"]; }

func use(ok: bool, inner: Inner): i32 {
    return i32((ok ? inner : Inner { row: n2() }).row[0][0]);
}

func main(): i32 {
    let inner: Inner = Inner { row: ["AA", "BB"] };
    return (use(true, inner) - 65) + (use(false, inner) - 67);
}
"#;
    let output = run_memory("p445mixed", mixed);
    assert_no_asan("p445mixed", &output);
    assert_exit("p445mixed", &output, 0);

    let control = r#"module spec.cpr.p445control;

struct Inner { row: str[2]; }

func n1(): str[2] { return ["AA", "BB"]; }

func use(ok: bool): i32 {
    let inner: Inner = Inner { row: n1() };
    return i32(inner.row[0][0]);
}

func main(): i32 {
    return (use(true) - 65) + (use(false) - 65);
}
"#;
    let output = run_memory("p445control", control);
    assert_no_asan("p445control", &output);
    assert_exit("p445control", &output, 0);
}

/// A take conditional literal in an eager ternary arm stores per arm.
///
/// `p446tern` selects `consumeO` with a conditional literal argument; the
/// chosen arm's call runs once and the unchosen arm never runs.
#[test]
fn should_store_a_take_conditional_literal_in_an_eager_ternary_arm() {
    let ternary = r#"module spec.cpr.p446tern;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consumeO(take o: Outer): i32 { return i32(o.inner.names[0][0]); }

func use(g: bool, ok: bool): i32 {
    let x: i32 = g ? consumeO(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                                : Outer { inner: Inner { names: n2() }, tag: stringOf("y") })
                   : 0;
    return x;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(false, true);
    let c: i32 = use(true, false);
    return (a - 65) + (b - 0) + (c - 67) + calls - 2;
}
"#;
    let output = run_memory("p446tern", ternary);
    assert_no_asan("p446tern", &output);
    assert_exit("p446tern", &output, 0);

    let control = r#"module spec.cpr.p446control;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

func n1(): str[2] { return ["AA", "BB"]; }

func consumeO(take o: Outer): i32 { return i32(o.inner.names[0][0]); }

func main(): i32 {
    return consumeO(Outer { inner: Inner { names: n1() }, tag: stringOf("x") }) - 65;
}
"#;
    let output = run_memory("p446control", control);
    assert_no_asan("p446control", &output);
    assert_exit("p446control", &output, 0);
}

/// A take of a conditional array literal of places stores element by element.
///
/// `p448cond` passes `ok ? [a, b] : [c, d]` to a take parameter; each choice
/// reads its own first byte. The unconditional and owned-element controls
/// stay green.
#[test]
fn should_store_a_take_conditional_view_array_literal() {
    let conditional = r#"module spec.cpr.p448cond;

func consume(take xs: str[2]): i32 { return i32(xs[0][0]) - 65; }

func use(ok: bool): i32 {
    let a: str = "AA";
    let b: str = "BB";
    let c: str = "CC";
    let d: str = "DD";
    return consume(take (ok ? [a, b] : [c, d]));
}

func main(): i32 {
    return (use(true) - 0) + (use(false) - 2);
}
"#;
    let output = run_memory("p448cond", conditional);
    assert_no_asan("p448cond", &output);
    assert_exit("p448cond", &output, 0);

    let plain = r#"module spec.cpr.p448plain;

func consume(take xs: str[2]): i32 { return i32(xs[0][0]) - 65; }

func main(): i32 {
    let a: str = "AA";
    let b: str = "BB";
    return consume(take [a, b]);
}
"#;
    let output = run_memory("p448plain", plain);
    assert_no_asan("p448plain", &output);
    assert_exit("p448plain", &output, 0);

    let owned = r#"module spec.cpr.p448owned;

import { String, stringOf } from std.text;

func consume(take xs: String[2]): i32 { return i32(len(xs[0])) - 2; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let c: String = stringOf("cc");
    let d: String = stringOf("dd");
    return consume(take (true ? [a, b] : [c, d]));
}
"#;
    let output = run_memory("p448owned", owned);
    assert_no_asan("p448owned", &output);
    assert_exit("p448owned", &output, 0);
}

/// An owned conditional field and a guarded conditional take keep their order.
///
/// `p445ownedfield` selects each arm of an owned field with a fixed-array row
/// and a trailing tag: the place arm moves into the field and the literal arm
/// stores its row through the hidden destination. `p446order` writes `inner`
/// before `tag` even though `tag` is declared first. `p448guard` fills the
/// conditional array only on the evaluated outer arm.
#[test]
fn should_keep_the_owned_literal_field_and_guarded_take_safe() {
    let owned_field = r#"module spec.cpr.p445ownedfield;

import { String, stringOf } from std.text;

struct Inner { row: str[2]; tag: String; }
struct Outer { inner: Inner; }

func use(ok: bool): i32 {
    let existing: Inner = Inner { row: ["CC", "DD"], tag: stringOf("yy") };
    let o: Outer = Outer { inner: ok ? Inner { row: ["AA", "BB"], tag: stringOf("xx") } : existing };
    return i32(len(o.inner.tag)) - 2;
}

func main(): i32 {
    return use(true) + use(false);
}
"#;
    let output = run_memory("p445ownedfield", owned_field);
    assert_no_asan("p445ownedfield", &output);
    assert_exit("p445ownedfield", &output, 0);

    let order = r#"module spec.cpr.p446order;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { tag: String; inner: Inner; }

let order: i32 = 0;
let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; order = order * 10 + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; order = order * 10 + 2; return ["CC", "DD"]; }
func tagOf(): String { order = order * 10 + 2; return stringOf("x"); }

func consumeO(take o: Outer): i32 { return i32(o.inner.names[0][0]); }

func use(g: bool, ok: bool): i32 {
    let x: i32 = g ? consumeO(ok ? Outer { inner: Inner { names: n1() }, tag: tagOf() }
                                : Outer { inner: Inner { names: n2() }, tag: tagOf() })
                   : 0;
    return x;
}

func main(): i32 {
    let a: i32 = use(true, true);
    return (a - 65) + (order - 12) * 10 + (calls - 1) * 100;
}
"#;
    let output = run_memory("p446order", order);
    assert_no_asan("p446order", &output);
    assert_exit("p446order", &output, 0);

    let owned_read = r#"module spec.cpr.p445ownedread;

import { String, stringOf } from std.text;

struct Inner { row: str[2]; name: String; }

func n1(): str[2] { return ["AA", "BB"]; }
func n2(): str[2] { return ["CC", "DD"]; }

func use(ok: bool): i32 {
    return i32((ok ? Inner { row: n1(), name: stringOf("x") } : Inner { row: n2(), name: stringOf("y") }).row[0][0]) - (ok ? 65 : 67);
}

func guard(g: bool, ok: bool): i32 {
    return g ? use(ok) : 0;
}

func main(): i32 {
    return use(true) + use(false) + guard(true, true) + guard(false, true);
}
"#;
    let output = run_memory("p445ownedread", owned_read);
    assert_no_asan("p445ownedread", &output);
    assert_exit("p445ownedread", &output, 0);

    let guarded = r#"module spec.cpr.p448guard;

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(take xs: str[2]): i32 { return i32(xs[0][0]) - 65; }

func use(g: bool, ok: bool): i32 {
    let x: i32 = g ? consume(take (ok ? n1() : n2())) : 0;
    return x;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(false, true);
    let c: i32 = use(true, false);
    return (a - 0) + b + (c - 2) + calls - 2;
}
"#;
    let output = run_memory("p448guard", guarded);
    assert_no_asan("p448guard", &output);
    assert_exit("p448guard", &output, 0);
}

// ---------------------------------------------------------------------------
// inlineownedliteral: a read/projection of an inline owning literal keeps the
// literal's storage alive for the read and releases it once.
// ---------------------------------------------------------------------------

/// The inline owning literal and its variants run clean.
///
/// `p447rows` reads a nested `str[2][2]` field; `p447nested` nests the literal
/// in a field; `p447string` reads an owned `String`; `p447grid` projects a
/// `String[2][2]` row; and `p447direct` binds the projected `String` first.
#[test]
fn should_keep_the_inline_read_base_literal_clean() {
    let rows = r#"module spec.cpr.p447rows;

import { String, stringOf } from std.text;

struct G { rows: str[2][2]; tag: String; }

func n1(): str[2] { return ["AA", "BB"]; }

func use(): i32 {
    return i32(G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0][0]);
}

func main(): i32 {
    return use() - 65;
}
"#;
    let output = run_memory("p447rows", rows);
    assert_no_asan("p447rows", &output);
    assert_exit("p447rows", &output, 0);

    let nested = r#"module spec.cpr.p447nested;

import { String, stringOf } from std.text;

struct Inner { rows: str[2][2]; }
struct Outer { inner: Inner; tag: String; }

func n1(): str[2] { return ["AA", "BB"]; }

func use(): i32 {
    return i32(Outer { inner: Inner { rows: [n1(), n1()] }, tag: stringOf("x") }.inner.rows[0][0][0]);
}

func main(): i32 {
    return use() - 65;
}
"#;
    let output = run_memory("p447nested", nested);
    assert_no_asan("p447nested", &output);
    assert_exit("p447nested", &output, 0);

    let string = r#"module spec.cpr.p447string;

import { String, stringOf } from std.text;

struct G { s: String; tag: String; }

func use(): i32 {
    return i32(len(G { s: stringOf("aa"), tag: stringOf("x") }.s));
}

func main(): i32 {
    return use() - 2;
}
"#;
    let output = run_memory("p447string", string);
    assert_no_asan("p447string", &output);
    assert_exit("p447string", &output, 0);

    let grid = r#"module spec.cpr.p447grid;

import { String, stringOf } from std.text;

struct G { rows: String[2][2]; tag: String; }

func n1(): String[2] { return [stringOf("AA"), stringOf("BB")]; }

func use(): i32 {
    return i32(len(G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0]));
}

func main(): i32 {
    return use() - 2;
}
"#;
    let output = run_memory("p447grid", grid);
    assert_no_asan("p447grid", &output);
    assert_exit("p447grid", &output, 0);

    let direct = r#"module spec.cpr.p447direct;

import { String, stringOf } from std.text;

struct G { rows: String[2][2]; tag: String; }

func n1(): String[2] { return [stringOf("AA"), stringOf("BB")]; }

func use(): i32 {
    let s: String = G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0];
    return i32(len(s)) - 2;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("p447direct", direct);
    assert_no_asan("p447direct", &output);
    assert_exit("p447direct", &output, 0);
}

// ---------------------------------------------------------------------------
// puretakepointer: a pure pointer place used by a take is captured before the
// call, so a callee that mutates its index cannot redirect the move-out.
// ---------------------------------------------------------------------------

/// A pointer-index take whose callee mutates the index moves the captured slot.
///
/// `pureptrindex` takes `*ps[idx]`; the callee sets `idx = 1`, so the move-out
/// and alias test must keep the element selected before the call. The program
/// runs clean and the conditional selects `x`.
#[test]
fn should_capture_a_pure_pointer_take_before_the_call() {
    let source = r#"module spec.cpr.pureptrindex;

import { String, stringOf } from std.text;

let idx: i32 = 0;

func eat(take s: String): i32 { idx = 1; return 1; }

func main(): i32 {
    let arr: String[2] = [stringOf("aa"), stringOf("bb")];
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let v: String = stringOf("vv");
    let g: bool = true;
    let ps: raw String[2] = [unsafe &arr[0], unsafe &arr[1]];
    v = (g && (eat(take (unsafe *ps[idx])) == 1)) ? x : y;
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("pureptrindex", source);
    assert_no_asan("pureptrindex", &output);
    assert_exit("pureptrindex", &output, 0);
}

// ---------------------------------------------------------------------------
// generatedownerread: a generated owner is a compiler binding, not a `__cl_tmp`
// source temporary, so a same-scope read of its view stays accepted.
// ---------------------------------------------------------------------------

/// A next-statement read of a generated owner's view runs clean.
///
/// Ch 03 D5 governs source temporaries (`__cl_tmp<N>`). The generated owner
/// (`__cl_owned<N>`) is the binding that holds the value captured by `ys`, so
/// it lives in the binding's frame; the next-statement length read is a use of
/// that binding, not of a dropped temporary.
#[test]
fn should_accept_a_next_statement_read_of_a_generated_owner() {
    let source = r#"module spec.cpr.genownerread;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func use(): i32 {
    let ys: str[2] = [mk().v, "EE"];
    return i32(len(ys[0])) - 2;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("genownerread", source);
    assert_no_asan("genownerread", &output);
    assert_exit("genownerread", &output, 0);
}

/// A conditional, member, or global pointer is captured before a take call.
///
/// The callee flips the condition, mutates the member, or reassigns the global;
/// the capture keeps the pointer the call read, so the move-out empties that
/// pointee only.
#[test]
fn should_capture_conditional_member_and_global_pointer_takes() {
    let conditional = r#"module spec.cpr.ptrcond;

import { String, stringOf } from std.text;

let gp: raw String = null;
let gq: raw String = null;
let which: bool = true;

func eat(take s: String): i32 { which = false; return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let v: String = stringOf("vv");
    let g: bool = true;
    gp = unsafe &a;
    gq = unsafe &b;
    v = (g && (eat(take (unsafe *(which ? gp : gq))) == 1)) ? x : y;
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("ptrcond", conditional);
    assert_no_asan("ptrcond", &output);
    assert_exit("ptrcond", &output, 0);

    let member = r#"module spec.cpr.ptrmember;

import { String, stringOf } from std.text;

struct Holder { ptr: raw String; }

let gq: raw String = null;
let gb: Holder = Holder { ptr: null };

func eat(take s: String): i32 { gb.ptr = gq; return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let v: String = stringOf("vv");
    let g: bool = true;
    gq = unsafe &b;
    gb.ptr = unsafe &a;
    v = (g && (eat(take (unsafe *gb.ptr)) == 1)) ? x : y;
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("ptrmember", member);
    assert_no_asan("ptrmember", &output);
    assert_exit("ptrmember", &output, 0);

    let global = r#"module spec.cpr.ptrglobal;

import { String, stringOf } from std.text;

let gp: raw String = null;
let gq: raw String = null;

func eat(take s: String): i32 { gp = gq; return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let v: String = stringOf("vv");
    let g: bool = true;
    gp = unsafe &a;
    gq = unsafe &b;
    v = (g && (eat(take (unsafe *gp)) == 1)) ? x : y;
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("ptrglobal", global);
    assert_no_asan("ptrglobal", &output);
    assert_exit("ptrglobal", &output, 0);
}

/// A computed address take is captured before the callee mutates its index.
#[test]
fn should_capture_a_computed_address_take() {
    let source = r#"module spec.cpr.ptraddress;

import { String, stringOf } from std.text;

let idx: i32 = 0;

func eat(take s: String): i32 { idx = 1; return 1; }

func main(): i32 {
    let arr: String[2] = [stringOf("aa"), stringOf("bb")];
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let v: String = stringOf("vv");
    let g: bool = true;
    v = (g && (eat(take (unsafe *(&arr[idx]))) == 1)) ? x : y;
    return i32(len(v)) - 2;
}
"#;
    let output = run_memory("ptraddress", source);
    assert_no_asan("ptraddress", &output);
    assert_exit("ptraddress", &output, 0);
}

/// A generic element call and a multi-extent take move are judged per instance.
///
/// A `take` argument that reads an owned element of a `borrow` parameter is
/// the same lend move as the assignment spelling: `E10034` at the argument
/// read. The `take` parameter twin reports `E10026`, while a duplicable element
/// copies. A deeper multi-extent projection out of a `take` parameter also
/// reports `E10026` instead of leaking the selected handle through a literal
/// field.
#[test]
fn should_judge_a_generic_element_call_and_a_multi_extent_take_move() {
    let borrow_call = r#"module spec.cpr.genericcallborrow;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func sink(take e: Elem): i32 { return i32(len(e.name)); }

func relay<T>(borrow w: T, i: i32): i32 {
    return sink(w.items[i]);
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return relay<Holder>(borrow h, 0) - 2;
}
"#;
    compile_rejects("genericcallborrow", borrow_call, "E10034", 1);

    let take_call = r#"module spec.cpr.genericcalltake;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func sink(take e: Elem): i32 { return i32(len(e.name)); }

func relay<T>(take w: T, i: i32): i32 {
    return sink(w.items[i]);
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return relay<Holder>(take h, 0) - 2;
}
"#;
    compile_rejects("genericcalltake", take_call, "E10026", 1);

    let duplicable = r#"module spec.cpr.genericcalldup;

struct Elem { y: i32; }
struct Holder { items: Elem[2]; }

func sink(e: Elem): i32 { return e.y; }

func relay<T>(borrow w: T, i: i32): i32 {
    return sink(w.items[i]);
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { y: 11 }, Elem { y: 22 }] };
    return relay<Holder>(borrow h, 0) - 11;
}
"#;
    let output = run_memory("genericcalldup", duplicable);
    assert_no_asan("genericcalldup", &output);
    assert_exit("genericcalldup", &output, 0);

    let multi_extent = r#"module spec.cpr.multiextenttake;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { rows: Elem[2][2]; }

func makeIt<T>(take w: T): Elem {
    return Elem { name: w.rows[0][0].name };
}

func main(): i32 {
    let h: Holder = Holder { rows: [[Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }],
                                   [Elem { name: stringOf("cc") }, Elem { name: stringOf("dd") }]] };
    let e: Elem = makeIt<Holder>(take h);
    return i32(len(e.name)) - 2;
}
"#;
    compile_rejects("multiextenttake", multi_extent, "E10026", 1);
}

/// A conditional of borrowed places is judged by the instance's destination.
///
/// A duplicable container root is copied at the call, so the store fills the
/// callee's copy and is accepted. A nonduplicable caller-retained container
/// cannot keep a view of another lend parameter: the lend is call-scoped, so
/// the field or element store reports one `E10033`. An owning instance keeps
/// the concrete arm judgment and reports `E10034` for each non-`take` lend arm.
/// A local-backed conditional into a caller slot is still the frame escape
/// (`E10030`); a frame-local nonduplicable container keeps its accepted store.
#[test]
fn should_store_a_conditional_of_borrowed_places_at_a_view_instance() {
    let view_store = r#"module spec.cpr.condviewstore;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, borrow a: String, borrow b: String, i: i32): i32 {
    w.items[i] = run ? a : b;
    return i32(w.items[i][0]);
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["xx", "yy"] };
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    return store<str>(borrow w, true, borrow a, borrow b, 0) - 97;
}
"#;
    let output = run_memory("condviewstore", view_store);
    assert_no_asan("condviewstore", &output);
    assert_exit("condviewstore", &output, 0);

    let nonduplicable_view = r#"module spec.cpr.condviewstorewrap2;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap2<T>, run: bool, borrow a: String, borrow b: String, i: i32): i32 {
    w.items[i] = run ? a : b;
    return 0;
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let r: i32 = store<str>(borrow w, true, borrow a, borrow b, 0);
    return r + i32(len(w.items[0])) - 2;
}
"#;
    compile_rejects("condviewstorewrap2", nonduplicable_view, "E10033", 1);

    let plain_nonduplicable = r#"module spec.cpr.condplainwrap2;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap2<T>, borrow a: String): void {
    w.items[0] = a;
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let a: String = stringOf("aa");
    store<str>(borrow w, borrow a);
    return i32(w.items[0][0]) - 120;
}
"#;
    compile_rejects("condplainwrap2", plain_nonduplicable, "E10033", 1);

    let concrete_nonduplicable = r#"module spec.cpr.condconcretewrap2;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func store(borrow w: Wrap2<str>, borrow a: String): void {
    w.items[0] = a;
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let a: String = stringOf("aa");
    store(borrow w, borrow a);
    return i32(w.items[0][0]) - 120;
}
"#;
    compile_rejects("condconcretewrap2", concrete_nonduplicable, "E10033", 1);

    let view_source_nonduplicable = r#"module spec.cpr.condviewsourcewrap2;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Wrap2<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap2<T>, borrow a: Box): void {
    w.items[0] = a.v;
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let a: Box = Box { v: stringOf("ab") };
    store<str>(borrow w, borrow a);
    return i32(w.items[0][0]) - 97;
}
"#;
    compile_rejects("condviewsourcewrap2", view_source_nonduplicable, "E10033", 1);

    let local_view_of_param = r#"module spec.cpr.condlocalsourcewrap2;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Wrap2<T> { items: T[2]; tag: String; }

func store(borrow w: Wrap2<str>, borrow p: Box, run: bool): void {
    let s: str = p.v;
    w.items[0] = run ? s : "AA";
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let p: Box = Box { v: stringOf("ab") };
    store(borrow w, borrow p, true);
    return i32(w.items[0][0]) - 97;
}
"#;
    compile_rejects("condlocalsourcewrap2", local_view_of_param, "E10033", 1);

    let frame_local = r#"module spec.cpr.condframelocalwrap2;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func store(run: bool, borrow a: String, borrow b: String): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    w.items[0] = run ? a : b;
    return i32(w.items[0][0]);
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let r: i32 = store(true, borrow a, borrow b);
    return r - 97;
}
"#;
    let output = run_memory("condframelocalwrap2", frame_local);
    assert_no_asan("condframelocalwrap2", &output);
    assert_exit("condframelocalwrap2", &output, 0);

    let concrete_view = r#"module spec.cpr.condviewstoreconcrete;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func store(borrow w: Wrap<str>, run: bool, borrow a: String, borrow b: String, i: i32): i32 {
    w.items[i] = run ? a : b;
    return i32(w.items[i][0]);
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["xx", "yy"] };
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    return store(borrow w, true, borrow a, borrow b, 0) - 97;
}
"#;
    let output = run_memory("condviewstoreconcrete", concrete_view);
    assert_no_asan("condviewstoreconcrete", &output);
    assert_exit("condviewstoreconcrete", &output, 0);

    let own_store = r#"module spec.cpr.condownstore;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, borrow a: String, borrow b: String, i: i32): i32 {
    w.items[i] = run ? a : b;
    return 0;
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("xx"), stringOf("yy")] };
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    return store<String>(borrow w, true, borrow a, borrow b, 0);
}
"#;
    compile_rejects("condownstore", own_store, "E10034", 2);

    let local_view = r#"module spec.cpr.condlocalview;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, i: i32): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    w.items[i] = run ? a : b;
    return 0;
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["xx", "yy"] };
    return store<str>(borrow w, true, 0);
}
"#;
    compile_rejects("condlocalview", local_view, "E10030", 1);
}

/// Compiles `source` and asserts the exact set of diagnostic codes.
fn compile_reports(tag: &str, source: &str, codes: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let actual: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .map(|line| line.split_whitespace().nth(1).unwrap_or("").trim_end_matches(':'))
        .collect();
    assert_eq!(
        actual.len(),
        codes.len(),
        "{tag}: expected {codes:?}, saw {actual:?}; stderr:\n{stderr}"
    );
    for code in codes {
        assert!(
            actual.contains(code),
            "{tag}: expected {code}; saw {actual:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// A conditional opaque store is judged per instance and per source path.
///
/// A fresh arm keeps a statement-temporary frame beside a named arm. At an
/// owning instance the arm reports are distinct: a lend arm is `E10034`, a
/// later local read is `E10020`, and a deeper projected arm is `E10026`. A
/// whole-struct read after a top-level field move is `E10025`. A conditional
/// re-initialization that covers only one path does not hide the other path's
/// use; an all-path or unconditional re-initialization stays accepted.
#[test]
fn should_judge_a_conditional_opaque_store_by_instance_and_path() {
    let fresh_local = r#"module spec.cpr.condfreshlocal;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(take w: Wrap<T>, run: bool, i: i32): i32 {
    let a: String = stringOf("aa");
    w.items[i] = run ? a : stringOf("bb");
    return i32(len(w.items[i]));
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["xx", "yy"] };
    return store<str>(w, false, 0);
}
"#;
    compile_rejects("condfreshlocal", fresh_local, "E10030", 1);

    let mixed = r#"module spec.cpr.condmixedreports;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, borrow a: String, i: i32): i32 {
    let b: String = stringOf("bb");
    w.items[i] = run ? a : b;
    return i32(len(b));
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("xx"), stringOf("yy")] };
    let a: String = stringOf("aa");
    return store<String>(borrow w, true, borrow a, 0);
}
"#;
    compile_reports("condmixedreports", mixed, &["E10034", "E10020"]);

    let deep = r#"module spec.cpr.conddeepreports;

import { String, stringOf } from std.text;

struct Inner { s: String; }
struct Holder { inner: Inner; }
struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, borrow a: String, i: i32): i32 {
    let h: Holder = Holder { inner: Inner { s: stringOf("hh") } };
    w.items[i] = run ? a : h.inner.s;
    return 0;
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("xx"), stringOf("yy")] };
    let a: String = stringOf("aa");
    return store<String>(borrow w, true, borrow a, 0);
}
"#;
    compile_reports("conddeepreports", deep, &["E10034", "E10026"]);

    let partial_whole = r#"module spec.cpr.condpartialwhole;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: i32; }
struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, i: i32): i32 {
    let h: Holder = Holder { s: stringOf("aaaa"), tag: 7 };
    let b: String = stringOf("bb");
    w.items[i] = run ? h.s : b;
    let whole: Holder = h;
    return whole.tag;
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("xx"), stringOf("yy")] };
    return store<String>(borrow w, true, 0);
}
"#;
    compile_rejects("condpartialwhole", partial_whole, "E10025", 1);

    let conditional_reinit = r#"module spec.cpr.condreinit;

import { String, stringOf, bytes } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, later: bool, i: i32): i32 {
    let a: String = stringOf("aa");
    w.items[i] = run ? a : stringOf("bb");
    if (later) {
        a = stringOf("cc");
    }
    let bs: u8[] = bytes(a);
    return i32(bs[0]);
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("xx"), stringOf("yy")] };
    return store<String>(borrow w, true, false, 0);
}
"#;
    compile_rejects("condreinit", conditional_reinit, "E10020", 1);

    let both_reinit = r#"module spec.cpr.condreinitboth;

import { String, stringOf, bytes } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, later: bool, i: i32): i32 {
    let a: String = stringOf("aa");
    w.items[i] = run ? a : stringOf("bb");
    if (later) {
        a = stringOf("cc");
    } else {
        a = stringOf("dd");
    }
    let bs: u8[] = bytes(a);
    return i32(bs[0]);
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("xx"), stringOf("yy")] };
    let r: i32 = store<String>(borrow w, true, true, 0);
    return r - 99;
}
"#;
    let output = run_memory("condreinitboth", both_reinit);
    assert_no_asan("condreinitboth", &output);
    assert_exit("condreinitboth", &output, 0);

    let fresh_no_read = r#"module spec.cpr.condfreshnoread;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(take w: Wrap<T>, run: bool, i: i32): i32 {
    let a: String = stringOf("aa");
    w.items[i] = run ? a : stringOf("bb");
    return 0;
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["xx", "yy"] };
    return store<str>(w, false, 0);
}
"#;
    // A fresh owned arm has no frame-scoped owner; the store reports the
    // escape even when the carrier is never read again.
    compile_rejects("condfreshnoread", fresh_no_read, "E10030", 1);

    let unconditional = r#"module spec.cpr.condreinituncond;

import { String, stringOf, bytes } from std.text;

struct Wrap<T> { items: T[2]; }

func store<T>(borrow w: Wrap<T>, run: bool, i: i32): i32 {
    let a: String = stringOf("aa");
    w.items[i] = run ? a : stringOf("bb");
    a = stringOf("cc");
    let bs: u8[] = bytes(a);
    return i32(bs[0]);
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("xx"), stringOf("yy")] };
    let r: i32 = store<String>(borrow w, true, 0);
    return r - 99;
}
"#;
    let output = run_memory("condreinituncond", unconditional);
    assert_no_asan("condreinituncond", &output);
    assert_exit("condreinituncond", &output, 0);
}

/// A generic or array view store cannot escape into a caller-retained field.
///
/// The declaration may see a type-parameter value or a concrete view field
/// under a generic root; the instance decides whether the destination is a
/// view. A fixed-array or slice root is duplicable in the type table but its C
/// parameter still reaches the caller's storage, so it is not a copy. Each
/// spelling reports one `E10033` at the store. The duplicable-root, same-carrier,
/// static, and owning-instance controls keep their behavior.
#[test]
fn should_reject_a_generic_or_array_view_store_into_a_caller_retained_container() {
    let generic_value = r#"module spec.cpr.genericvaluewrap2;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Wrap2<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap2<T>, borrow a: T): void {
    w.items[0] = a;
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let b: Box = Box { v: stringOf("aa") };
    let s: str = b.v;
    store<str>(borrow w, borrow s);
    b.v = stringOf("QQ");
    return i32(w.items[0][0]) - 97;
}
"#;
    compile_rejects("genericvaluewrap2", generic_value, "E10033", 1);

    let generic_value_owning = r#"module spec.cpr.genericvalueowning;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap2<T>, borrow a: T): void {
    w.items[0] = a;
}

func main(): i32 {
    let w: Wrap2<String> = Wrap2<String> { items: [stringOf("xx"), stringOf("yy")], tag: stringOf("t") };
    let a: String = stringOf("aa");
    store<String>(borrow w, borrow a);
    return 0;
}
"#;
    compile_rejects("genericvalueowning", generic_value_owning, "E10034", 1);

    let generic_root = r#"module spec.cpr.genericrootconcrete;

import { String, stringOf } from std.text;

struct Outer<T> { payload: T; items: str[2]; tag: String; }

func store<T>(borrow o: Outer<T>, borrow a: String): void {
    o.items[0] = a;
}

func main(): i32 {
    let o: Outer<i32> = Outer<i32> { payload: 1, items: ["xx", "yy"], tag: stringOf("t") };
    let a: String = stringOf("aa");
    store<i32>(borrow o, borrow a);
    a = stringOf("cc");
    return i32(o.items[0][0]) - 97;
}
"#;
    compile_rejects("genericrootconcrete", generic_root, "E10033", 1);

    let generic_root_copy = r#"module spec.cpr.genericrootcopy;

import { String, stringOf } from std.text;

struct Outer<T> { payload: T; items: str[2]; }

func store<T>(borrow o: Outer<T>, borrow a: String): i32 {
    o.items[0] = a;
    return i32(o.items[0][0]);
}

func main(): i32 {
    let o: Outer<i32> = Outer<i32> { payload: 1, items: ["xx", "yy"] };
    let a: String = stringOf("aa");
    return store<i32>(borrow o, borrow a) - 97;
}
"#;
    let output = run_memory("genericrootcopy", generic_root_copy);
    assert_no_asan("genericrootcopy", &output);
    assert_exit("genericrootcopy", &output, 0);

    let fixed_array_root = r#"module spec.cpr.fixedarrayroot;

import { String, stringOf } from std.text;

func store(borrow w: str[2], borrow a: String): void {
    w[0] = a;
}

func main(): i32 {
    let w: str[2] = ["xx", "yy"];
    let a: String = stringOf("aa");
    store(borrow w, borrow a);
    a = stringOf("cc");
    return i32(w[0][0]) - 97;
}
"#;
    compile_rejects("fixedarrayroot", fixed_array_root, "E10033", 1);

    let slice_root = r#"module spec.cpr.sliceroot;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func store(borrow w: str[], borrow a: String): void {
    w[0] = a;
}

func main(): i32 {
    let backing: str[2] = ["xx", "yy"];
    let w: str[] = slice(backing, 0, 2);
    let a: String = stringOf("aa");
    store(borrow w, borrow a);
    a = stringOf("cc");
    return i32(w[0][0]) - 97;
}
"#;
    compile_rejects("sliceroot", slice_root, "E10033", 1);

    let call_result = r#"module spec.cpr.callresultwrap2;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func id<T>(n: T): T { return n; }

func store<T>(borrow w: Wrap2<T>, borrow a: T): void {
    w.items[0] = id<T>(a);
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let b: String = stringOf("aa");
    let s: str = b;
    store<str>(borrow w, borrow s);
    b = stringOf("QQ");
    return i32(w.items[0][0]) - 97;
}
"#;
    compile_rejects("callresultwrap2", call_result, "E10033", 1);

    let fixed_array_same = r#"module spec.cpr.fixedarraysame;

func store(borrow w: str[2]): i32 {
    w[1] = w[0];
    return i32(w[1][0]);
}

func main(): i32 {
    let w: str[2] = ["xx", "yy"];
    return store(borrow w) - 120;
}
"#;
    let output = run_memory("fixedarraysame", fixed_array_same);
    assert_no_asan("fixedarraysame", &output);
    assert_exit("fixedarraysame", &output, 0);

    let slice_same = r#"module spec.cpr.slicesame;

import { slice } from std.mem;

func store(borrow w: str[]): i32 {
    w[1] = w[0];
    return i32(w[1][0]);
}

func main(): i32 {
    let backing: str[2] = ["xx", "yy"];
    let w: str[] = slice(backing, 0, 2);
    return store(borrow w) - 120;
}
"#;
    let output = run_memory("slicesame", slice_same);
    assert_no_asan("slicesame", &output);
    assert_exit("slicesame", &output, 0);

    let static_scalar_call = r#"module spec.cpr.staticscalarcall;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func ignoreN<T>(n: i32): T { return "AA"; }

func store<T>(borrow w: Wrap2<T>, n: i32): i32 {
    w.items[0] = ignoreN<T>(n);
    return i32(w.items[0][0]);
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    return store<str>(borrow w, 7) - 65;
}
"#;
    let output = run_memory("staticscalarcall", static_scalar_call);
    assert_no_asan("staticscalarcall", &output);
    assert_exit("staticscalarcall", &output, 0);

    let static_unused_arg = r#"module spec.cpr.staticunusedarg;

import { String, stringOf } from std.text;

struct Wrap2<T> { items: T[2]; tag: String; }

func ignore<T>(borrow n: String): T { return "AA"; }

func store<T>(borrow w: Wrap2<T>, borrow n: String): i32 {
    w.items[0] = ignore<T>(borrow n);
    return i32(w.items[0][0]);
}

func main(): i32 {
    let w: Wrap2<str> = Wrap2<str> { items: ["xx", "yy"], tag: stringOf("t") };
    let n: String = stringOf("zz");
    return store<str>(borrow w, borrow n) - 65;
}
"#;
    let output = run_memory("staticunusedarg", static_unused_arg);
    assert_no_asan("staticunusedarg", &output);
    assert_exit("staticunusedarg", &output, 0);

    let fixed_array_static = r#"module spec.cpr.fixedarraystatic;

func store(borrow w: str[2]): i32 {
    w[0] = "AA";
    return i32(w[0][0]);
}

func main(): i32 {
    let w: str[2] = ["xx", "yy"];
    return store(borrow w) - 65;
}
"#;
    let output = run_memory("fixedarraystatic", fixed_array_static);
    assert_no_asan("fixedarraystatic", &output);
    assert_exit("fixedarraystatic", &output, 0);
}

/// A nested captured whole out-slot keeps the ownership-slot exemption.
///
/// A relayed capture is a parameter in the inner lifted body; the original
/// whole `borrow T?` kind must travel through the relay. A captured
/// non-nullable writable view slot is not an ownership slot and keeps the
/// parameter-backed field escape.
#[test]
fn should_accept_a_nested_captured_whole_out_slot() {
    let nested = r#"module spec.cpr.nestedcaptureoutslot;

func outer(borrow slot: str?): void {
    let s: str = "AA";
    ([s, borrow slot] () => {
        ([s, borrow slot] () => {
            slot = s;
        })();
    })();
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    compile_rejects("nestedcaptureoutslot", nested, "E10034", 1);

    let writable = r#"module spec.cpr.capturedwritable;

func outer(p: str, borrow slot: str): void {
    ([p, borrow slot] () => {
        slot = p;
    })();
}

func main(): i32 {
    let out: str = "AA";
    outer("xy", borrow out);
    return i32(out[0]) - 120;
}
"#;
    compile_rejects("capturedwritable", writable, "E10033", 1);
}

/// A generic conditional element store is judged per instance: an owning
/// substitution reports one `E10026`, while a duplicable substitution copies.
#[test]
fn should_judge_a_generic_conditional_element_move_per_instance() {
    let owning = r#"module spec.cpr.genericcondmove;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }

func pick<T>(take w: Wrap<T>, i: i32, j: i32, c: bool): i32 {
    w.items[i] = c ? w.items[j] : w.items[j];
    return 0;
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> { items: [stringOf("aa"), stringOf("bb")] };
    return pick<String>(take w, 0, 1, true);
}
"#;
    compile_rejects("genericcondmove", owning, "E10026", 1);

    let duplicable = r#"module spec.cpr.genericconddup;

struct Wrap<T> { items: T[2]; }

func pick<T>(take w: Wrap<T>, i: i32, j: i32, c: bool): i32 {
    w.items[i] = c ? w.items[j] : w.items[j];
    return 0;
}

func main(): i32 {
    let w: Wrap<i32> = Wrap<i32> { items: [1, 2] };
    return pick<i32>(take w, 0, 1, true);
}
"#;
    let output = run_memory("genericconddup", duplicable);
    assert_no_asan("genericconddup", &output);
    assert_exit("genericconddup", &output, 0);
}
