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
