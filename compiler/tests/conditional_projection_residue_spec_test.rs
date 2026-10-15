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
