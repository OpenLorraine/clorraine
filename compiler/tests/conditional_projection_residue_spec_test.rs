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
/// `slot = p.v;` keep the caller's frame alive for as long as the slot is
/// read: in the non-nullable spelling the callee reads the copy it stored, and
/// in the nullable by-address spelling the caller reads the written view. Both
/// are accepted, exit 0, and are memory-clean.
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
    let output = run_memory("outslotpartwin", twin);
    assert_no_asan("outslotpartwin", &output);
    assert_exit("outslotpartwin", &output, 0);

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
    let output = run_memory("outslotpartwide", wide);
    assert_no_asan("outslotpartwide", &output);
    assert_exit("outslotpartwide", &output, 0);

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

func fill(p: str, borrow slot: str[1]): void { slot = [p]; }

func main(): i32 {
    let out: str[1] = ["AA"];
    fill("xyz", borrow out);
    return i32(out[0][0]) - 120;
}
"#;
    let output = run_memory("staticout", slot);
    assert_no_asan("staticout", &output);
    assert_exit("staticout", &output, 0);
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
/// the caller's or module storage, so each slot store is accepted. The
/// static-str call `lit()` names static storage and is accepted into the same
/// global field. The `null` arm of `slot = c ? null : p` names no storage, so
/// the caller-backed spelling is accepted.
#[test]
fn should_accept_a_mixed_source_whose_components_module_or_caller_storage_covers() {
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
    let output = run_memory("mergeparam", slot);
    assert_no_asan("mergeparam", &output);
    assert_exit("mergeparam", &output, 0);

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
    let output = run_memory("nullcond", nullcond);
    assert_no_asan("nullcond", &output);
    assert_exit("nullcond", &output, 0);
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
