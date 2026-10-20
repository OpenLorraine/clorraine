//! Runnable specification tests for callable release integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. The memory helper
//! runs a case as an ordinary strict host build and adds leak and
//! use-after-free detection when `RDC_ASAN` is set.
//!
//! The identity group pins the three generated-identity cases: a source local
//! spelled like a generated owner stays an ordinary local, two `func`
//! signatures of one generic struct are two instances, and an inline
//! capturing lambda builds with one environment definition.
//!
//! The callable effect-mode group pins that each callee kind applies its own
//! signature's parameter modes: a `take` or borrow call through a func-typed
//! field, a module-level func-typed global, a callable-bound type parameter,
//! or a lambda parameter reports exactly what the module spelling reports,
//! and an owned temporary argument is released exactly once — including the
//! temporary passed to a lend-bodied lambda. The lambda cases also pin that an
//! expression body is judged like a `return`, so it reports the same single
//! rejection its block-bodied and module spellings report.

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
            "clc-cri-{}-{tag}-{sequence}",
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

/// Asserts one compiler run rejected the program with exactly one `code`
/// diagnostic, that the message contains every fragment and none of the
/// absent markers, and that no translation unit is written.
fn assert_rejected_output(
    tag: &str,
    output: &Output,
    c_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = format!("error {code}:");
    assert_eq!(
        stderr.matches(&marker).count(),
        1,
        "{tag}: expected exactly one {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    for absent_marker in absent {
        assert!(
            !stderr.contains(absent_marker),
            "{tag}: the report {absent_marker:?} must be absent; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the rejection contract on the output.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str], absent: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
}

/// Compiles a two-file module program and returns the compiler output.
///
/// The holder file declares the module the program imports; both paths reach
/// the compiler, so an imported or qualified name resolves and the ordinary
/// checks run on it.
fn compile_pair(tag: &str, holder: &str, main: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// Compiles a three-file module program and returns the compiler output.
///
/// Two holder files declare the modules the program names; every path reaches
/// the compiler, so two same-named bindings in different modules both resolve
/// and the ordinary checks run on them.
fn compile_three(
    tag: &str,
    first: &str,
    second: &str,
    main: &str,
) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let first_path = scratch.path().join("first.clo");
    let second_path = scratch.path().join("second.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&first_path, first)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the first holder: {err}"));
    std::fs::write(&second_path, second)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the second holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        first_path.display().to_string(),
        second_path.display().to_string(),
    ]);
    (scratch, output, c_path)
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
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
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

/// Builds a multi-file program's emission with the sanitizer.
///
/// The single-file `run_memory` path selects the sanitizer when `RDC_ASAN` is
/// set; a module program needs the same build, so this helper mirrors the
/// flags that leg uses.
fn build_host_c_asan(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build_asan.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog_asan");
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
    program
}

/// Compiles and runs a two-file module program under the active memory leg.
///
/// The program must be accepted and its emission must build; the run happens
/// with the same build `run_memory` selects, so the sanitizer leg checks the
/// same shapes the single-file cases check.
fn run_pair_memory(tag: &str, holder: &str, main: &str) -> Output {
    let (scratch, output, c_path) = compile_pair(tag, holder, main);
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
    let program = if std::env::var_os("RDC_ASAN").is_some() {
        build_host_c_asan(tag, &scratch, &emitted)
    } else {
        build_host_c(tag, &scratch, &emitted)
    };
    run_with_timeout(tag, &program, &[])
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

/// A source local spelled like a generated owner stays an ordinary local.
///
/// The ownership layer binds the borrowed argument's owned temporary in a
/// generated slot; the user's local of the same spelling must keep its own
/// identity, so the program runs one release per value (red at the pre-fix
/// head: the collision double-frees under the sanitizer).
#[test]
fn should_keep_a_generated_owner_distinct_from_a_source_local() {
    let source = r#"module spec.cri.identity;

import { String, stringOf } from std.text;

func use(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let __cl_owned0: String = stringOf("q");
    let n: i32 = use(stringOf("w"));
    return n + i32(len(__cl_owned0)) - 2;
}
"#;
    let output = run_memory("identity", source);
    assert_no_asan("identity", &output);
    assert_exit("identity", &output, 0);
}

/// Two `func` signatures of one generic struct are two instances.
///
/// `Holder<func(i32): i32>` and `Holder<func(u32): u32>` have distinct C
/// representations, so the translation unit defines two structs and builds
/// (red at the pre-fix head: both instances share the `fn` key and the strict
/// build fails on the redefinition).
#[test]
fn should_build_two_func_signature_instances_of_one_generic_struct() {
    let source = r#"module spec.cri.funckey;

struct Holder<T> {
    call: T;
}

func addOne(v: i32): i32 {
    return v + 1;
}

func addTwo(v: u32): u32 {
    return v + 2;
}

func main(): i32 {
    let a: Holder<func(i32): i32> = Holder<func(i32): i32> { call: addOne };
    let b: Holder<func(u32): u32> = Holder<func(u32): u32> { call: addTwo };
    return a.call(2) + i32(b.call(2)) - 7;
}
"#;
    let output = run_host("funckey", source);
    assert_exit("funckey", &output, 0);
}

/// Two mode-differing func signatures of one generic struct are two instances.
///
/// A `borrow` parameter reaches the callee as a pointer, so `func(String):
/// i32` and `func(borrow String): i32` are different types with different C
/// representations: each instance builds and resolves its own callable.
#[test]
fn should_build_two_mode_differing_func_instances_of_one_generic_struct() {
    let source = r#"module spec.cri.modekey;

import { String, stringOf } from std.text;

struct Holder<T> {
    call: T;
}

func lenOf(s: String): i32 {
    return i32(len(s));
}

func lendOf(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let a: Holder<func(String): i32> = Holder<func(String): i32> { call: lenOf };
    let b: Holder<func(borrow String): i32> = Holder<func(borrow String): i32> { call: lendOf };
    return a.call(stringOf("ab")) + b.call(stringOf("cde")) - 5;
}
"#;
    let output = run_host("modekey", source);
    assert_exit("modekey", &output, 0);
}

/// A capture-less lambda inside a `defer` block builds and runs.
///
/// The defer registration lowering emits its own IR, so its lifted record —
/// the one the emitted call site references — must survive; only the
/// discarded speculative loop pass's records are dropped (red at the pre-fix
/// head: the emitted defer body calls an undefined lifted function and the
/// strict build fails).
#[test]
fn should_build_a_capture_less_lambda_inside_a_defer() {
    let source = r#"module spec.cri.deferlambda;

func main(): i32 {
    let seen: i32 = 0;
    defer {
        seen = (() => i32(7))();
        assert(seen == 7, "the deferred lambda ran");
    }
    return 0;
}
"#;
    let output = run_host("deferlambda", source);
    assert_exit("deferlambda", &output, 0);
}

/// A capturing lambda in a loop releases a temporary its body borrows.
///
/// The speculative loop pass suppresses owned-temporary materialization, so
/// its lifted record must never be the emitted one: the reporting pass's
/// record binds the argument's owner and releases it once per call (red at
/// the pre-fix head: the emitted speculative body leaks under the sanitizer).
#[test]
fn should_release_a_temporary_inside_a_loop_lambda() {
    let source = r#"module spec.cri.loopowned;

import { String, stringOf } from std.text;

func use(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: String = stringOf("ab");
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        total = total + ([s] () => i32(use(stringOf("w"))))();
        i = i + 1;
    }
    return total - 2;
}
"#;
    let output = run_memory("loopowned", source);
    assert_no_asan("loopowned", &output);
    assert_exit("loopowned", &output, 0);
}

/// A capturing lambda in a loop body builds with one definition set.
///
/// The ownership layer's speculative loop pass must not emit a second lifted
/// function for the same site: the translation unit defines the environment
/// struct, the closure typedef, and the lifted function once (red at the
/// pre-fix head: the strict build fails on the redefinitions).
#[test]
fn should_build_a_capturing_lambda_in_a_loop_once() {
    let source = r#"module spec.cri.envdedupe;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("ab");
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        total = total + ([s] () => i32(len(s)))();
        i = i + 1;
    }
    return total - 4;
}
"#;
    let output = run_host("envdedupe", source);
    assert_exit("envdedupe", &output, 0);
}

/// A `take` call through a func-typed field moves the argument.
///
/// A func field's signature carries its parameter modes, so passing an owned
/// local to a `take` field call transfers ownership exactly as the module
/// function does: the later reuse of the local reports one `E10020`. The
/// module spelling is the control (red at the pre-fix head: the field call
/// records no move, so the field program is accepted and its reuse
/// double-frees at run time).
#[test]
fn should_report_a_take_field_calls_move_like_its_module_twin() {
    let twin = r#"module spec.cri.takefieldtwin;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: String = stringOf("x");
    let n: i32 = sink(s);
    return n + i32(len(s));
}
"#;
    assert_rejected_once(
        "takefield-twin",
        twin,
        "E10020",
        &["'s' was moved and cannot be used"],
        &[],
    );

    let field = r#"module spec.cri.fieldtake;

import { String, stringOf } from std.text;

struct Holder {
    call: func(take String): i32;
}

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let h: Holder = Holder { call: sink };
    let s: String = stringOf("x");
    let n: i32 = h.call(s);
    return n + i32(len(s));
}
"#;
    assert_rejected_once(
        "takefield",
        field,
        "E10020",
        &["'s' was moved and cannot be used"],
        &[],
    );
}

/// Passing the same owned local twice to a `borrow` field call conflicts.
///
/// Each `borrow` parameter of the field's signature takes a loan of the
/// argument, so the overlapping loans report one `E10032`, exactly as the
/// module function's twin does. The module spelling is the control (red at
/// the pre-fix head: the field call records no lend and the field program is
/// accepted).
#[test]
fn should_report_a_borrow_field_calls_conflict_like_its_module_twin() {
    let twin = r#"module spec.cri.borrowfieldtwin;

import { String, stringOf } from std.text;

func two(borrow a: String, borrow b: String): i32 {
    return i32(len(a)) + i32(len(b));
}

func main(): i32 {
    let s: String = stringOf("x");
    return two(s, s) - 2;
}
"#;
    assert_rejected_once(
        "borrowfield-twin",
        twin,
        "E10032",
        &["'s' is already borrowed"],
        &[],
    );

    let field = r#"module spec.cri.fieldconflict;

import { String, stringOf } from std.text;

struct Holder {
    call: func(borrow String, borrow String): i32;
}

func two(borrow a: String, borrow b: String): i32 {
    return i32(len(a)) + i32(len(b));
}

func main(): i32 {
    let h: Holder = Holder { call: two };
    let s: String = stringOf("x");
    return h.call(s, s) - 2;
}
"#;
    assert_rejected_once(
        "borrowfield",
        field,
        "E10032",
        &["'s' is already borrowed"],
        &[],
    );
}

/// A func-typed field call releases an owned temporary argument once.
///
/// The temporary `String` passed to a plain field call is owned by no local,
/// so the call must bind and release it at the statement end, exactly as the
/// module twin's signature decides (red at the pre-fix head: the field call
/// leaks the temporary under the sanitizer).
#[test]
fn should_release_an_owned_temporary_passed_to_a_func_field_call() {
    let source = r#"module spec.cri.fieldborrow;

import { String, stringOf } from std.text;

struct Holder {
    call: func(String): i32;
}

func lenOf(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let h: Holder = Holder { call: lenOf };
    return h.call(stringOf("x")) - 1;
}
"#;
    let output = run_memory("fieldborrow", source);
    assert_no_asan("fieldborrow", &output);
    assert_exit("fieldborrow", &output, 0);
}

/// A `take` call through a module-level func global moves the argument.
///
/// The global's declared signature carries the modes, so the call transfers
/// ownership exactly as the module function does and the later reuse reports
/// one `E10020`; the module spelling is the control (red at the pre-fix head:
/// the global call records no move, so the program is accepted and its reuse
/// double-frees at run time).
#[test]
fn should_report_a_take_global_calls_move_like_its_module_twin() {
    let twin = r#"module spec.cri.takeglobaltwin;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: String = stringOf("x");
    let n: i32 = sink(s);
    return n + i32(len(s));
}
"#;
    assert_rejected_once(
        "takeglobal-twin",
        twin,
        "E10020",
        &["'s' was moved and cannot be used"],
        &[],
    );

    let global = r#"module spec.cri.globaltake;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return i32(len(s));
}

let g: func(take String): i32 = sink;

func main(): i32 {
    let s: String = stringOf("x");
    let n: i32 = g(s);
    return n + i32(len(s));
}
"#;
    assert_rejected_once(
        "takeglobal",
        global,
        "E10020",
        &["'s' was moved and cannot be used"],
        &[],
    );
}

/// A module-level func global call releases an owned temporary argument once.
///
/// The temporary `String` passed to the global's plain signature must be
/// released at the statement end, exactly as the module twin's signature
/// decides (red at the pre-fix head: the global call leaks the temporary
/// under the sanitizer).
#[test]
fn should_release_an_owned_temporary_passed_to_a_func_global_call() {
    let source = r#"module spec.cri.globalborrow;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

let g: func(String): i32 = lenOf;

func main(): i32 {
    return g(stringOf("x")) - 1;
}
"#;
    let output = run_memory("globalborrow", source);
    assert_no_asan("globalborrow", &output);
    assert_exit("globalborrow", &output, 0);
}

/// A call through a callable-bound type parameter releases an owned temporary.
///
/// The bound's signature governs the call, so the temporary `String` passed
/// to the callable is bound and released at the statement end (red at the
/// pre-fix head under the sanitizer: the call leaks the temporary).
#[test]
fn should_release_an_owned_temporary_passed_to_a_callable_bound_parameter() {
    let source = r#"module spec.cri.paramleak;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

func wrap<F: func(String): i32>(f: F): i32 {
    return f(stringOf("x"));
}

func main(): i32 {
    return wrap<func(String): i32>(lenOf) - 1;
}
"#;
    let output = run_memory("paramleak", source);
    assert_no_asan("paramleak", &output);
    assert_exit("paramleak", &output, 0);
}

/// A parameter shadowing a same-named `take` function keeps its own modes.
///
/// The callee is the parameter, whose callable bound takes the argument by a
/// plain read-only lend, so the owned local stays usable afterwards and the
/// program runs to exit 0. The direct module call is the control and still
/// reports its one `E10020` (red at the pre-fix head: resolution falls back
/// to the same-named module function's take places and reports a false
/// `E10020` at the later use).
#[test]
fn should_keep_a_parameter_that_shadows_a_take_function_distinct_from_it() {
    let twin = r#"module spec.cri.paramshadowtwin;

import { String, stringOf } from std.text;

func sink(take v: String): i32 {
    return i32(len(v));
}

func main(): i32 {
    let s: String = stringOf("x");
    let r: i32 = sink(s);
    return r + i32(len(s));
}
"#;
    assert_rejected_once(
        "paramshadow-twin",
        twin,
        "E10020",
        &["'s' was moved and cannot be used"],
        &[],
    );

    let shadow = r#"module spec.cri.paramshadow;

import { String, stringOf } from std.text;

func sink(take v: String): i32 {
    return i32(len(v));
}

func relay<sink: func(String): i32>(sink: sink): i32 {
    let s: String = stringOf("x");
    let r: i32 = sink(s);
    return r + i32(len(s));
}

func lenOf(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    return relay<func(String): i32>(lenOf) - 2;
}
"#;
    let output = run_host("paramshadow", shadow);
    assert_exit("paramshadow", &output, 0);
}

/// A lambda's expression body stores a borrowed parameter like its other
/// spellings.
///
/// A lambda parameter is unmarked, so it is a read-only lend exactly as an
/// unmarked module parameter is, and storing a lend reports one `E10034`; the
/// block-bodied spelling and the module twin already report it. An expression
/// body is the lifted function's return, so it owes the same judgement (red
/// at the pre-fix head: the expression-bodied shape is accepted and the
/// owning instance double-frees the caller's string).
#[test]
fn should_report_a_lambda_expression_body_store() {
    let source = r#"module spec.cri.lamparam;

import { String, stringOf } from std.text;

struct Holder<T> {
    v: T;
}

func wrap<T>(x: T): Holder<T> {
    return ((p: T) => Holder<T> { v: p })(x);
}

func main(): i32 {
    let h = wrap(stringOf("ab"));
    return i32(len(h.v)) - 2;
}
"#;
    assert_rejected_once(
        "lamparam",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &[],
    );
}

/// A block-bodied lambda's borrowed store reports once per instance.
///
/// The block-bodied spelling runs the return-move judgement, so storing the
/// borrowed parameter reports exactly one `E10034`; this control pins the
/// report the expression-bodied spelling must match.
#[test]
fn should_report_a_block_bodied_lambdas_borrowed_store() {
    let source = r#"module spec.cri.lamblock;

import { String, stringOf } from std.text;

struct Holder<T> {
    v: T;
}

func wrap<T>(x: T): Holder<T> {
    return ((p: T) => { return Holder<T> { v: p }; })(x);
}

func main(): i32 {
    let h = wrap(stringOf("ab"));
    return i32(len(h.v)) - 2;
}
"#;
    assert_rejected_once(
        "lamblock",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &[],
    );
}

/// A module function's borrowed store reports once, like the lambda spellings.
///
/// An unmarked module parameter is a read-only lend, so storing it in a holder
/// reports exactly one `E10034`; this module twin is the other control the
/// lambda spellings must match.
#[test]
fn should_report_a_module_functions_borrowed_store() {
    let source = r#"module spec.cri.lamstore;

import { String, stringOf } from std.text;

struct Holder<T> {
    v: T;
}

func make(p: String): Holder<String> {
    return Holder<String> { v: p };
}

func main(): i32 {
    let h: Holder<String> = make(stringOf("ab"));
    return i32(len(h.v)) - 2;
}
"#;
    assert_rejected_once(
        "lamstore",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &[],
    );
}

/// A lend-bodied lambda call releases an owned temporary argument once.
///
/// A lambda's parameter is unmarked, so the closure takes a read-only lend and
/// the caller must materialize and release the owned temporary it passes,
/// exactly as a module call does; the run exits 0 (red at the pre-fix head:
/// the call path never materializes the lend argument, so the temporary leaks
/// under the sanitizer).
#[test]
fn should_release_an_owned_temporary_passed_to_a_lend_bodied_lambda() {
    let source = r#"module spec.cri.lamcall;

import { String, stringOf } from std.text;

func main(): i32 {
    return ((p: String) => i32(len(p)))(stringOf("x")) - 1;
}
"#;
    let output = run_memory("lamcall", source);
    assert_no_asan("lamcall", &output);
    assert_exit("lamcall", &output, 0);
}

/// A mixed conditional lend argument keeps its place arm readable.
///
/// The conditional mixes a fresh call result with a named place; the argument
/// is a lend, so the place must stay an inline read and the later use must see
/// the live value. Binding the conditional as a whole would adopt the chosen
/// arm and empty the place (red at the pre-fix head: the accepted program
/// dereferences the emptied storage and segfaults).
#[test]
fn should_keep_a_mixed_conditional_lend_arguments_place_arm_readable() {
    let source = r#"module spec.cri.condplace;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let b: String = stringOf("bb");
    let pick: bool = false;
    let n: i32 = lenOf(pick ? stringOf("aa") : b);
    let m: i32 = i32(len(b));
    return n + m - 4;
}
"#;
    let output = run_memory("condplace", source);
    assert_no_asan("condplace", &output);
    assert_exit("condplace", &output, 0);
}

/// A mixed conditional lend argument releases its fresh arm.
///
/// The fresh arm owns its storage and the callee only borrows, so the caller
/// releases it on the path that evaluates it. Every callee kind is covered
/// with the fresh arm taken: the module call, the func-typed field, the
/// callable-bound parameter, the closure call, and the intrinsic argument
/// (red at the pre-fix head: the unbound fresh arms leak under the sanitizer,
/// and the fully bound spellings adopt the place arm instead).
#[test]
fn should_release_a_mixed_conditional_lend_arguments_fresh_arm() {
    let source = r#"module spec.cri.condfresh;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

struct Holder { call: func(String): i32; }

func wrap<F: func(String): i32>(f: F, pick: bool, b: String): i32 {
    return f(pick ? stringOf("aa") : b);
}

func main(): i32 {
    let b: String = stringOf("bb");
    let h: Holder = Holder { call: lenOf };
    let pick: bool = true;
    let n: i32 = lenOf(pick ? stringOf("aa") : b);
    let o: i32 = h.call(pick ? stringOf("aa") : b);
    let r: i32 = wrap<func(String): i32>(lenOf, pick, b);
    let s: i32 = ((p: String) => i32(len(p)))(pick ? stringOf("aa") : b);
    let t: i32 = i32(len(pick ? stringOf("aa") : b));
    return n + o + r + s + t - 10;
}
"#;
    let output = run_memory("condfresh", source);
    assert_no_asan("condfresh", &output);
    assert_exit("condfresh", &output, 0);
}

/// A `take` call-result callee marks its move like a value twin.
///
/// A call whose callee is itself a call reaches the returned function value,
/// so its signature's `take` argument moves and a later use reports `E10020`
/// exactly as a func-typed binding's call does (red at the pre-fix head: the
/// move was lowered without the check, and the accepted program read the
/// emptied storage).
#[test]
fn should_report_a_take_call_result_callees_move_like_its_value_twin() {
    let source = r#"module spec.cri.callresulttake;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return i32(len(s));
}

func pick(): func(take String): i32 {
    return sink;
}

func main(): i32 {
    let s: String = stringOf("x");
    let n: i32 = pick()(s);
    return n + i32(len(s));
}
"#;
    assert_rejected_once(
        "callresulttake",
        source,
        "E10020",
        &["'s' was moved"],
        &[],
    );
}

/// A borrow call-result callee records its argument conflict.
///
/// The returned function value's signature marks both parameters `borrow`, so
/// lending the same place twice in one call reports `E10032` like the module
/// twin (red at the pre-fix head: accepted).
#[test]
fn should_report_a_borrow_call_result_callees_conflict_like_its_value_twin() {
    let source = r#"module spec.cri.callresultborrow;

import { String, stringOf } from std.text;

func two(borrow a: String, borrow b: String): i32 {
    return i32(len(a)) + i32(len(b));
}

func pick(): func(borrow String, borrow String): i32 {
    return two;
}

func main(): i32 {
    let s: String = stringOf("x");
    return pick()(s, s) - 2;
}
"#;
    assert_rejected_once(
        "callresultborrow",
        source,
        "E10032",
        &["'s' is already borrowed"],
        &[],
    );
}

/// A call through a returned `take` value accepts a fresh operand.
///
/// The fresh temporary transfers to the callee, so no owner is bound and the
/// program runs and exits correctly; the companion rejection cases pin the
/// checker's move marking, and this case guards the accepted spelling (the
/// base compiler emits the same clean program).
#[test]
fn should_run_a_call_result_callees_fresh_take_operand() {
    let source = r#"module spec.cri.callresultfresh;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return i32(len(s));
}

func pick(): func(take String): i32 {
    return sink;
}

func main(): i32 {
    return pick()(stringOf("x")) - 1;
}
"#;
    let output = run_memory("callresultfresh", source);
    assert_no_asan("callresultfresh", &output);
    assert_exit("callresultfresh", &output, 0);
}

/// A module global's call-site marker must match its declaration.
///
/// The declared signature is authoritative, so a written `take` on a plain
/// parameter reports `E10039` like the module-function twin (red at the
/// pre-fix head: accepted).
#[test]
fn should_report_a_marker_mismatch_at_a_func_global_call() {
    let source = r#"module spec.cri.globalmarker;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

let g: func(String): i32 = lenOf;

func main(): i32 {
    let s: String = stringOf("ab");
    return g(take s) - 2;
}
"#;
    assert_rejected_once(
        "globalmarker",
        source,
        "E10039",
        &["marker does not match the declaration of 'g'"],
        &[],
    );
}

/// An imported or qualified callable global is checked like the local spelling.
///
/// Both spellings reach the same global value, so the signature's `take`
/// argument moves and the later use reports `E10020` in either spelling (red
/// at the pre-fix head: accepted, with the move applied but never marked).
#[test]
fn should_report_a_take_imported_globals_move_like_the_local_spelling() {
    let holder = r#"module spec.cri.gpair;

import { String } from std.text;

func sink(take s: String): i32 {
    return i32(len(s));
}

export let g: func(take String): i32 = sink;
"#;
    let imported = r#"module spec.cri.gpairmain;

import { String, stringOf } from std.text;
import { g } from spec.cri.gpair;

func main(): i32 {
    let s: String = stringOf("x");
    let n: i32 = g(s);
    return n + i32(len(s));
}
"#;
    let (_scratch, output, c_path) = compile_pair("gpair-import", holder, imported);
    assert_rejected_output(
        "gpair-import",
        &output,
        &c_path,
        "E10020",
        &["'s' was moved"],
        &[],
    );

    let qualified = r#"module spec.cri.gpairqual;

import { String, stringOf } from std.text;
import { g } from spec.cri.gpair;

func main(): i32 {
    let s: String = stringOf("x");
    let n: i32 = spec.cri.gpair.g(s);
    return n + i32(len(s));
}
"#;
    let (_scratch, output, c_path) = compile_pair("gpair-qual", holder, qualified);
    assert_rejected_output(
        "gpair-qual",
        &output,
        &c_path,
        "E10020",
        &["'s' was moved"],
        &[],
    );
}

/// A marker mismatch on a call-result callee reports like its twins.
///
/// The returned function value's signature declares the parameter modes, so a
/// written marker that contradicts it reports `E10039`, exactly as the
/// binding and module twins do (red at the fix base: the branch ran the
/// argument checks but not the marker check).
#[test]
fn should_report_a_marker_mismatch_at_a_call_result_callee() {
    let source = r#"module spec.cri.callresultmarker;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

func pick(): func(String): i32 {
    return lenOf;
}

func main(): i32 {
    let s: String = stringOf("ab");
    return pick()(take s) - 2;
}
"#;
    assert_rejected_once(
        "callresultmarker",
        source,
        "E10039",
        &["marker does not match the declaration of 'pick'"],
        &[],
    );
}

/// A mixed conditional lend argument over an aggregate keeps its place arm.
///
/// The conditional mixes a fresh struct literal with a named place; the
/// argument crosses as the address of the selected storage, so each arm takes
/// its own address and the place arm is never emptied. Both arms run clean
/// (red at the fix base: the address of the whole select was not spellable
/// and the strict build failed on the accepted program).
#[test]
fn should_keep_a_mixed_conditional_aggregate_lend_buildable() {
    let source = r#"module spec.cri.condagg;

import { String, stringOf } from std.text;

struct Box { s: String; }

func weigh(borrow x: Box): i32 {
    return i32(len(x.s));
}

func main(): i32 {
    let box: Box = Box { s: stringOf("bb") };
    let pick: bool = false;
    let n: i32 = weigh(pick ? Box { s: stringOf("aa") } : box) - 2;
    let pick2: bool = true;
    let m: i32 = weigh(pick2 ? Box { s: stringOf("aa") } : box) - 2;
    return n + m;
}
"#;
    let output = run_memory("condagg", source);
    assert_no_asan("condagg", &output);
    assert_exit("condagg", &output, 0);
}

/// A mixed conditional lend argument with an array literal arm builds.
///
/// The literal arm fills a destination temporary on its own branch, so the
/// argument select yields array places that decay to the parameter's pointer;
/// both arms run the expected answer (red at the fix base: the literal
/// rendered as an initializer list inside the select and the strict build
/// failed).
#[test]
fn should_release_a_mixed_conditional_array_literal_arm() {
    let source = r#"module spec.cri.condarr;

func sum(borrow xs: i32[3]): i32 {
    return xs[0] + xs[1] + xs[2];
}

func main(): i32 {
    let arr: i32[3] = [1, 2, 3];
    let pick: bool = false;
    let n: i32 = sum(pick ? arr : [4, 5, 6]) - 15;
    let pick2: bool = true;
    let m: i32 = sum(pick2 ? arr : [4, 5, 6]) - 6;
    return n + m;
}
"#;
    let output = run_memory("condarr", source);
    assert_no_asan("condarr", &output);
    assert_exit("condarr", &output, 0);
}

/// A mixed conditional lend argument keeps the left-to-right order.
///
/// The condition runs once, the fresh arm fills under its own branch at the
/// argument's position, and the later argument's evaluation follows it; the
/// side-effect ledger proves the order 1-2 (red at the fix base: the later
/// argument's binding overtook the fresh arm, giving 2-1).
#[test]
fn should_keep_the_left_to_right_argument_order_with_a_mixed_conditional_argument() {
    let source = r#"module spec.cri.condorder;

import { String, stringOf } from std.text;

let order: i32 = 0;

func fresh(): String {
    order = order * 10 + 1;
    return stringOf("aa");
}

func mark(tag: i32): String {
    order = order * 10 + tag;
    return stringOf("zz");
}

func combine(a: String, b: String): i32 {
    return i32(len(a)) + i32(len(b));
}

func main(): i32 {
    let b: String = stringOf("bbbb");
    let pick: bool = true;
    let n: i32 = combine(pick ? fresh() : b, mark(2));
    if (n != 4) {
        return 99;
    }
    return order - 12;
}
"#;
    let output = run_memory("condorder", source);
    assert_no_asan("condorder", &output);
    assert_exit("condorder", &output, 0);
}

/// A field call through a temporary receiver releases the receiver's siblings.
///
/// The receiver aggregate is a fresh struct literal or a call result, so its
/// other owning fields have no owner unless the call path binds the receiver
/// root for cleanup; each call releases them exactly once (red at the phase
/// head: 20 bytes leak across the two calls).
#[test]
fn should_release_a_temporary_func_field_receivers_siblings() {
    let source = r#"module spec.cri.temprec;

import { String, stringOf } from std.text;

func addOne(x: i32): i32 {
    return x + 1;
}

struct Holder { call: func(i32): i32; s: String; }

func makeH(): Holder {
    return Holder { call: addOne, s: stringOf("x") };
}

func main(): i32 {
    let a: i32 = Holder { call: addOne, s: stringOf("x") }.call(5);
    let b: i32 = makeH().call(5);
    return a + b - 12;
}
"#;
    let output = run_memory("temprec", source);
    assert_no_asan("temprec", &output);
    assert_exit("temprec", &output, 0);
}

/// An owned projection backing inside a `defer` body releases per execution.
///
/// The defer body's view binding takes its backing from a call temporary, so
/// a block-scope owner must be created and released once per execution of the
/// body (red at the phase head: 11 bytes leak for the single execution).
#[test]
fn should_release_an_owned_projection_backing_inside_a_defer() {
    let source = r#"module spec.cri.deferview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("ab") };
}

let total: i32 = 0;

func main(): i32 {
    defer {
        let v: str = mk().v;
        total = total + i32(len(v));
    }
    return 0;
}
"#;
    let output = run_memory("deferview", source);
    assert_no_asan("deferview", &output);
    assert_exit("deferview", &output, 0);
}

/// An owned temporary at a func-value call inside a `defer` body releases.
///
/// The deferred call passes a fresh owned temporary to a lend parameter, so
/// the caller binds and releases it; the block's cleanup runs the release
/// once per execution (red at the phase head: 10 bytes leak).
#[test]
fn should_release_an_owned_temporary_at_a_func_call_inside_a_defer() {
    let source = r#"module spec.cri.defercall;

import { String, stringOf } from std.text;

let total: i32 = 0;

func use(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let f: func(String): i32 = use;
    defer {
        total = total + f(stringOf("x"));
    }
    return 0;
}
"#;
    let output = run_memory("defercall", source);
    assert_no_asan("defercall", &output);
    assert_exit("defercall", &output, 0);
}

/// A mixed conditional view's taken arm is owned on its own path.
///
/// The first loop takes the call arm, which must be owned by the view
/// binding's scope; the second takes the place arm, which stays the caller's
/// borrow (red at the phase head: 33 bytes leak across the first loop's three
/// iterations).
#[test]
fn should_release_a_mixed_conditional_views_taken_call_arm() {
    let source = r#"module spec.cri.condview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func makeBox(): Box {
    return Box { v: stringOf("ab") };
}

func main(): i32 {
    let b: Box = Box { v: stringOf("cd") };
    let c: bool = false;
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let v: str = (c ? b : makeBox()).v;
        n = n + i32(len(v));
        i = i + 1;
    }
    let d: bool = true;
    let m: i32 = 0;
    let j: i32 = 0;
    while (j < 3) {
        let v: str = (d ? b : makeBox()).v;
        m = m + i32(len(v));
        j = j + 1;
    }
    return n + m - 12;
}
"#;
    let output = run_memory("condview", source);
    assert_no_asan("condview", &output);
    assert_exit("condview", &output, 0);
}

/// Both owning conditional view arms release their siblings.
///
/// Both arms project a fresh struct literal, so the taken arm's backing and
/// every sibling field must release on that path (red at the phase head:
/// 33 bytes leak across three iterations).
#[test]
fn should_release_both_owning_conditional_view_arms_siblings() {
    let source = r#"module spec.cri.condviewstruct;

import { String, stringOf } from std.text;

struct P { a: String; b: String; }
struct Q { a: String; b: String; }

func main(): i32 {
    let c: bool = true;
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let v: str = c ? (P { a: stringOf("ab"), b: stringOf("cd") }).a : (Q { a: stringOf("ef"), b: stringOf("gh") }).a;
        n = n + i32(len(v));
        i = i + 1;
    }
    return n - 6;
}
"#;
    let output = run_memory("condviewstruct", source);
    assert_no_asan("condviewstruct", &output);
    assert_exit("condviewstruct", &output, 0);
}

/// A view-array return reports like the single-view spelling of its derivation.
///
/// A function returning `str` from a temporary projection reports `E10036`
/// ("derive from exactly one parameter"); the fixed array of the same
/// derivation reports the same code once per site (red at the phase head: the
/// array spelling is accepted and its backing leaks when called). An array
/// derived from exactly one view lend parameter stays accepted by both
/// spellings.
#[test]
fn should_report_a_view_array_return_like_the_single_view_spelling() {
    let single = r#"module spec.cri.viewretsingle;

import { String, stringOf } from std.text;

struct Box { v: String; }

func make(): str {
    return (Box { v: stringOf("ab") }).v;
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "viewretsingle",
        single,
        "E10036",
        &["view return does not derive from exactly one parameter"],
        &[],
    );

    let array = r#"module spec.cri.viewretarray;

import { String, stringOf } from std.text;

struct Box { v: String; }

func make(): str[1] {
    return [(Box { v: stringOf("ab") }).v];
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "viewretarray",
        array,
        "E10036",
        &["view return does not derive from exactly one parameter"],
        &[],
    );

    let control = r#"module spec.cri.viewretcontrol;

func wrap(v: str): str[1] {
    return [v];
}

func main(): i32 {
    let s: str = "ab";
    let a: str[1] = wrap(s);
    return i32(len(a[0])) - 2;
}
"#;
    let output = run_memory("viewretcontrol", control);
    assert_no_asan("viewretcontrol", &output);
    assert_exit("viewretcontrol", &output, 0);
}

/// A discarded owned call result releases at the statement's end.
///
/// The bare expression statement's result has no binding, so a statement
/// temporary must own and release it once per execution (red at the phase
/// head: 33 bytes leak across three iterations).
#[test]
fn should_release_a_discarded_owned_call_result() {
    let source = r#"module spec.cri.discard;

import { String, stringOf } from std.text;

func make(): String {
    return stringOf("ab");
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 3) {
        make();
        i = i + 1;
    }
    return 0;
}
"#;
    let output = run_memory("discard", source);
    assert_no_asan("discard", &output);
    assert_exit("discard", &output, 0);
}

/// A deferred return of a temporary-derived view is rejected.
///
/// The defer body's view binding takes its backing from a call temporary, so
/// the view dies before the caller reads it; the escape reports once like a
/// view of a named local (red at the phase head: the accepted program read
/// freed storage).
#[test]
fn should_reject_a_deferred_return_of_a_temporary_derived_view() {
    let source = r#"module spec.cri.deferret;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("ab") };
}

func f(a: str): str {
    defer {
        return mk().v;
    }
    return a;
}

func main(): i32 {
    let r: str = f("zz");
    let copy: String = stringOf(r);
    return i32(len(copy)) - 2;
}
"#;
    assert_rejected_once(
        "deferret",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// A direct return of a temporary-derived view is rejected.
///
/// The returned view reads through the receiver's generated owner, which the
/// return cleanup releases before the caller's use; the escape reports once
/// (red at the phase head: the accepted program use-after-freed).
#[test]
fn should_reject_a_return_of_a_temporary_derived_view() {
    let source = r#"module spec.cri.tempviewret;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("ab") };
}

func f(a: str): str {
    return mk().v;
}

func main(): i32 {
    let r: str = f("zz");
    let copy: String = stringOf(r);
    return i32(len(copy)) - 2;
}
"#;
    assert_rejected_once(
        "tempviewret",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// A static-derived view array is accepted like the single-view spelling.
///
/// A `str[1]` literal of static strings carries no input borrow, so the
/// static return needs no elision candidate (EL-4) and the program runs
/// (parity control: the single-view spelling of the same derivation is
/// accepted).
#[test]
fn should_accept_a_static_view_array_return_like_the_single_spelling() {
    let source = r#"module spec.cri.staticarr;

func litArr(): str[1] {
    return ["ab"];
}

func main(): i32 {
    return i32(len(litArr()[0])) - 2;
}
"#;
    let output = run_memory("staticarr", source);
    assert_no_asan("staticarr", &output);
    assert_exit("staticarr", &output, 0);
}

/// A local-derived view array reports like the single-view spelling.
///
/// The array's element names a local binding, so the return reports `E10035`
/// exactly as returning the single view does (red at the phase head:
/// accepted).
#[test]
fn should_report_a_local_view_array_return_like_the_single_spelling() {
    let source = r#"module spec.cri.localarr;

import { String, stringOf } from std.text;

func locArr(): str[1] {
    let k: String = stringOf("ab");
    return [k];
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "localarr",
        source,
        "E10035",
        &["cannot return a view of local 'k'"],
        &[],
    );
}

/// A view array whose element derives from a temporary is rejected.
///
/// The element reads a call temporary, so the returned element view dies
/// before the caller reads it; the array reports the single view's escape
/// (red at the fix base: accepted, leaking the backing).
#[test]
fn should_reject_a_view_array_return_of_a_temporary_derived_element() {
    let source = r#"module spec.cri.arrtempescape;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("ab") };
}

func f(a: str): str[1] {
    return [mk().v];
}

func main(): i32 {
    let r: str[1] = f("zz");
    let copy: String = stringOf(r[0]);
    return i32(len(copy)) - 2;
}
"#;
    assert_rejected_once(
        "arrtempescape",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// A mixed conditional view return is rejected when the fresh arm escapes.
///
/// The returned view reads the chosen arm; the fresh arm's storage dies
/// before the caller reads the view, so the escape reports once (red at the
/// fix base: accepted, reading freed storage).
#[test]
fn should_reject_a_mixed_conditional_view_return() {
    let source = r#"module spec.cri.condviewret;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box {
    return Box { v: stringOf("ab") };
}

func f(c: bool, p: str): str {
    let a: Box = Box { v: stringOf("cd") };
    return (c ? a : mkBox()).v;
}

func main(): i32 {
    let r: str = f(true, "zz");
    let copy: String = stringOf(r);
    return i32(len(copy)) - 2;
}
"#;
    assert_rejected_once(
        "condviewret",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// A projection of a place-borrowing conditional passes through unadopted.
///
/// The conditional stays an inline read, so the projected member is read from
/// the chosen arm rather than adopted: adopting the fresh arm's copy while
/// the place arm keeps its own storage would release the same buffer twice.
/// The case runs the plain build because the fresh arm's backing stays filed
/// as a leak (DEF-176); red at the fix base: the place arm double-freed.
#[test]
fn should_not_adopt_a_projection_of_a_place_borrowing_conditional() {
    let source = r#"module spec.cri.condprojarg;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box {
    return Box { v: stringOf("ab") };
}

func use(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let a: Box = Box { v: stringOf("cd") };
    let c: bool = true;
    let n: i32 = use((c ? a : mkBox()).v) - 2;
    let d: bool = false;
    let m: i32 = use((d ? a : mkBox()).v) - 2;
    return n + m;
}
"#;
    let output = run_host("condprojarg", source);
    assert_exit("condprojarg", &output, 0);
}

/// A conditional projection lend argument binds its root for release.
///
/// The element read names a member of a fresh call result, so the call path
/// binds the result's root at the argument's branch and the statement's
/// release frees the remaining fields after the callee's read (red at the fix
/// base: the selected member alone was filled, leaving the sibling leaked).
#[test]
fn should_bind_the_root_of_a_conditional_projection_lend_argument() {
    let source = r#"module spec.cri.condprojlend;

import { String, stringOf } from std.text;

struct Box { name: String; extra: String; }

func mk(): Box {
    return Box { name: stringOf("ab"), extra: stringOf("cdef") };
}

func use(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: String = stringOf("zz");
    let c: bool = true;
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        n = n + use(c ? mk().name : s);
        i = i + 1;
    }
    return n - 6;
}
"#;
    let output = run_memory("condprojlend", source);
    assert_no_asan("condprojlend", &output);
    assert_exit("condprojlend", &output, 0);
}

/// A possibly-null `String` widened into `len` reports `E10010`.
///
/// The implicit `String`-to-`str` widening reads the handle, so the operand
/// must be narrowed first; the report is the frozen narrowing diagnostic at
/// the use (red at the phase head: accepted, then a null dereference at run).
#[test]
fn should_reject_a_nullable_string_widened_into_len() {
    let source = r#"module spec.cri.nullwidening;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String? = null;
    let n: i32 = i32(len(s));
    return n;
}
"#;
    assert_rejected_once(
        "nullwidening",
        source,
        "E10010",
        &[
            "value 's' may be null here",
            "check `!= null` before the use",
        ],
        &[],
    );
}

/// The dot spelling of the widened `len` reports identically.
#[test]
fn should_reject_a_nullable_string_widened_into_dot_len() {
    let source = r#"module spec.cri.nulldotlen;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String? = null;
    return i32(s.len());
}
"#;
    assert_rejected_once(
        "nulldotlen",
        source,
        "E10010",
        &["value 's' may be null here"],
        &[],
    );
}

/// Both widened `len` spellings report once each.
#[test]
fn should_report_once_per_widening_site() {
    let source = r#"module spec.cri.nulltwice;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String? = null;
    let b: String? = null;
    let n: i32 = i32(len(a));
    let m: i32 = i32(b.len());
    return n + m;
}
"#;
    let (_scratch, output, _c_path) = compile_only("nulltwice", source);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "nulltwice: expected a rejection; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("error E10010:").count(),
        2,
        "nulltwice: one report per site; stderr:\n{stderr}"
    );
}

/// A narrowed nullable `String` widens and runs.
#[test]
fn should_accept_a_narrowed_nullable_string_widened_into_len() {
    let source = r#"module spec.cri.narrownull;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String? = stringOf("ab");
    if (s != null) {
        return i32(len(s)) - 2;
    }
    return 0;
}
"#;
    let output = run_memory("narrownull", source);
    assert_no_asan("narrownull", &output);
    assert_exit("narrownull", &output, 0);
}

/// A non-nullable `String` widens unchanged.
#[test]
fn should_accept_a_non_nullable_string_widened_into_len() {
    let source = r#"module spec.cri.nonnullwiden;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("ab");
    return i32(len(s)) - 2;
}
"#;
    let output = run_memory("nonnullwiden", source);
    assert_no_asan("nonnullwiden", &output);
    assert_exit("nonnullwiden", &output, 0);
}

/// A nullable widening reports inside `unsafe` like any other use.
///
/// The frozen narrowing rule lists no exemption for the view widening, so the
/// report matches the conversion and member-read sites.
#[test]
fn should_reject_a_nullable_string_widened_into_len_inside_unsafe() {
    let source = r#"module spec.cri.unsafewiden;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String? = null;
    unsafe {
        return i32(len(s));
    }
}
"#;
    assert_rejected_once(
        "unsafewiden",
        source,
        "E10010",
        &["value 's' may be null here"],
        &[],
    );
}

/// A nullable view builder source reports at the `slice` call.
///
/// `slice` is a declared view builder, so its source argument widens like any
/// argument whose expected type is a view (red at the fix base: accepted, the
/// slice reading a null view).
#[test]
fn should_reject_a_nullable_view_widened_into_slice() {
    let source = r#"module spec.cri.slicenull;

import { slice } from std.mem;

func main(): i32 {
    let v: u8[]? = null;
    let w: u8[] = slice(v, 0, 0);
    return i32(len(w));
}
"#;
    assert_rejected_once(
        "slicenull",
        source,
        "E10010",
        &["value 'v' may be null here"],
        &[],
    );
}

/// A nullable field read through a dot call reports like the free spelling.
///
/// The receiver path's type decides: `h.f` is nullable even though `h` is
/// not, and the parenthesized spelling reports the same use (red at the fix
/// base: accepted, reading a null handle).
#[test]
fn should_reject_a_nullable_field_reached_through_a_dot_call() {
    let source = r#"module spec.cri.fielddotnull;

import { String, stringOf } from std.text;

struct Holder { f: String?; }

func main(): i32 {
    let h: Holder = Holder { f: null };
    return i32(h.f.len());
}
"#;
    assert_rejected_once(
        "fielddotnull",
        source,
        "E10010",
        &["value 'h' may be null here"],
        &[],
    );
}

/// A narrowed view receiver stays accepted and the guarded read executes.
///
/// The gate tests the receiver path's type, so narrowing the head clears the
/// nullable use; the source is non-null at run time, so the guarded read
/// returns the real length.
#[test]
fn should_accept_a_non_nullable_receiver_under_a_narrowed_head() {
    let source = r#"module spec.cri.receivernarrow;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[4] = [0, 0, 0, 0];
    let s: u8[]? = slice(a, 0, 4);
    if (s != null) {
        return i32(s.len()) - 4;
    }
    return 0;
}
"#;
    let output = run_memory("receivernarrow", source);
    assert_no_asan("receivernarrow", &output);
    assert_exit("receivernarrow", &output, 0);
}

/// A current-module func-global call emits the module's own symbol.
///
/// The global's identity names the declaring module, so the callee resolves
/// by (module, name) and the emitted pointer carries the mangled spelling;
/// the run reads the call's value back (green control at the phase base).
#[test]
fn should_call_a_current_module_func_global() {
    let source = r#"module spec.cri.globalcalleectl;

func triple(v: i32): i32 {
    return v * 3;
}

let local: func(i32): i32 = triple;

func main(): i32 {
    return local(2) - 6;
}
"#;
    let output = run_host("globalcallee-ctl", source);
    assert_exit("globalcallee-ctl", &output, 0);
}

/// An imported, qualified, or field func-global call emits the target's
/// symbol.
///
/// The import binding and the qualified path carry the target module, so the
/// call resolves the global by (module, name) and renders the mangled symbol;
/// the struct global's field call resolves through the same identity. The run
/// reads every call result back, so a silently wrong callee cannot exit zero
/// (red at the phase base: the strict build failed with `implicit declaration
/// of function 'g'` and `'h' undeclared`; both foreign spellings emitted the
/// bare written name).
#[test]
fn should_call_an_imported_or_qualified_func_global() {
    let holder = r#"module spec.cri.globalcallee;

func addOne(v: i32): i32 {
    return v + 1;
}

func addTwo(v: i32): i32 {
    return v + 2;
}

struct Holder {
    call: func(i32): i32;
}

export let g: func(i32): i32 = addOne;
export let h: Holder = Holder { call: addTwo };
"#;
    let main = r#"module spec.cri.globalcalleemain;

import { g, h } from spec.cri.globalcallee;

func main(): i32 {
    let a: i32 = g(5);
    let b: i32 = h.call(6);
    let c: i32 = spec.cri.globalcallee.g(5);
    return (a - 6) + (b - 8) + (c - 6);
}
"#;
    let (scratch, output, c_path) = compile_pair("globalcallee", holder, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "globalcallee: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "globalcallee: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("globalcallee", &scratch, &emitted);
    let output = run_with_timeout("globalcallee", &program, &[]);
    assert_exit("globalcallee", &output, 0);
}

/// Two same-named func globals in different modules stay apart.
///
/// The front end resolves each call to its own module-level binding, so a
/// qualified spelling reaches the module it names, an imported spelling
/// reaches the module it imports from, and a bare call inside a module keeps
/// that module's own binding even while another module exports the same name.
/// The run reads every call back, so resolving one name to the wrong module's
/// binding cannot exit zero (red at the phase base: the import and the bare
/// same-module call both ran the first module's function, so two of the three
/// calls read the wrong value).
#[test]
fn should_keep_two_same_named_func_globals_apart() {
    let first = r#"module spec.cri.globalsplitfirst;

func plusOne(v: i32): i32 {
    return v + 1;
}

export let pick: func(i32): i32 = plusOne;
"#;
    let second = r#"module spec.cri.globalsplitsecond;

func plusHundred(v: i32): i32 {
    return v + 100;
}

export let pick: func(i32): i32 = plusHundred;

export func ownCall(): i32 {
    return pick(5) - 105;
}
"#;
    let main = r#"module spec.cri.globalsplitmain;

import { pick, ownCall } from spec.cri.globalsplitsecond;

func main(): i32 {
    let imported: i32 = pick(5) - 105;
    let qualified: i32 = spec.cri.globalsplitfirst.pick(5) - 6;
    let own: i32 = ownCall();
    return imported + qualified + own;
}
"#;
    let (scratch, output, c_path) = compile_three("globalsplit", first, second, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "globalsplit: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "globalsplit: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("globalsplit", &scratch, &emitted);
    let output = run_with_timeout("globalsplit", &program, &[]);
    assert_exit("globalsplit", &output, 0);
}

/// A same-module function is not shadowed by another module's same-named
/// global.
///
/// The call in the caller's own module names the function that module
/// declares, so its symbol is emitted even though a loaded module exports a
/// func-typed global of the same name. The run reads both calls back, so a
/// call through the foreign global cannot exit zero (red at the phase base:
/// the emitted call named the foreign module's global and exited 99).
#[test]
fn should_prefer_the_module_function_over_a_same_named_func_global() {
    let holder = r#"module spec.cri.globalmod;

export func g(x: i32): i32 {
    return x + 100;
}

export let f: func(i32): i32 = g;
"#;
    let main = r#"module spec.cri.globalmodmain;

import { g } from spec.cri.globalmod;

func f(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    return (f(5) - 6) + (g(5) - 105);
}
"#;
    let (scratch, output, c_path) = compile_pair("globalmod", holder, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "globalmod: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "globalmod: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("globalmod", &scratch, &emitted);
    let output = run_with_timeout("globalmod", &program, &[]);
    assert_exit("globalmod", &output, 0);
}

/// An imported or qualified func-global keeps its binding when a same-named
/// function exists elsewhere.
///
/// The import binds its head to the holder's func-typed global and the
/// qualified spelling names it directly, so a function of the same simple
/// name in another module — exported or not, imported or not — is not in
/// scope and never becomes the callee. Both spellings read their values back
/// in each fixture, so a call through the foreign function cannot exit zero
/// (red at the phase base: the imported, qualified, and unexported-function
/// programs all ran the foreign function).
#[test]
fn should_keep_the_func_global_when_a_same_named_function_exists_elsewhere() {
    let holder = r#"module spec.cri.globalfnholder;

func hg(x: i32): i32 {
    return x + 200;
}

export let f: func(i32): i32 = hg;
"#;
    let other = r#"module spec.cri.globalfnother;

export func f(x: i32): i32 {
    return x + 3;
}
"#;
    let main = r#"module spec.cri.globalfnmain;

import { f } from spec.cri.globalfnholder;

func main(): i32 {
    return (f(5) - 205) + (spec.cri.globalfnholder.f(5) - 205);
}
"#;
    let (scratch, output, c_path) = compile_three("globalfn", holder, other, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "globalfn: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "globalfn: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("globalfn", &scratch, &emitted);
    let output = run_with_timeout("globalfn", &program, &[]);
    assert_exit("globalfn", &output, 0);

    let hidden = r#"module spec.cri.globalfnhidden;

func f(x: i32): i32 {
    return x + 230;
}
"#;
    let private_main = r#"module spec.cri.globalfnprivmain;

import { f } from spec.cri.globalfnholder;

func main(): i32 {
    return (f(5) - 205) + (spec.cri.globalfnholder.f(5) - 205);
}
"#;
    let (scratch, output, c_path) = compile_three("globalfnpriv", holder, hidden, private_main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "globalfnpriv: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "globalfnpriv: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("globalfnpriv", &scratch, &emitted);
    let output = run_with_timeout("globalfnpriv", &program, &[]);
    assert_exit("globalfnpriv", &output, 0);
}

// ===== Module/global identity and the scope-aware checker typing =====

/// An aliased import and a qualified value head reach the holder's binding.
///
/// The alias binds the source name's binding, so the call spells the holder's
/// function symbol rather than the alias, which names no declaration; a
/// struct global's field reached through its declaring module
/// (`m.holder.g(5)`) resolves the leading segments as that module and spells
/// the holder's symbol with the field. Every call reads its value back, so an
/// unresolved or redirected callee cannot exit zero (red at the phase base:
/// the alias emitted `cl_..._bg`, undeclared at the strict build, and the
/// qualified value head was rejected `'...holder.g' is not declared`).
#[test]
fn should_call_an_aliased_import_and_a_qualified_value_head() {
    let holder = r#"module spec.cri.funcglobalaliasmod;

export func g(x: i32): i32 {
    return x + 4;
}
"#;
    let alias_main = r#"module spec.cri.funcglobalalias;

import { g as bg } from spec.cri.funcglobalaliasmod;

func main(): i32 {
    return (bg(5) - 9) + (bg(7) - 11);
}
"#;
    let (scratch, output, c_path) = compile_pair("funcglobalalias", holder, alias_main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "funcglobalalias: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "funcglobalalias: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("funcglobalalias", &scratch, &emitted);
    let output = run_with_timeout("funcglobalalias", &program, &[]);
    assert_exit("funcglobalalias", &output, 0);

    let field_holder = r#"module spec.cri.funcglobalfieldmod;

func qf(x: i32): i32 {
    return x + 30;
}

struct Holder {
    g: func(i32): i32;
}

export let holder: Holder = Holder { g: qf };
"#;
    let qualified_main = r#"module spec.cri.funcglobalvaluehead;

func main(): i32 {
    return (spec.cri.funcglobalfieldmod.holder.g(5) - 35)
        + (spec.cri.funcglobalfieldmod.holder.g(7) - 37);
}
"#;
    let (scratch, output, c_path) =
        compile_pair("funcglobalvaluehead", field_holder, qualified_main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "funcglobalvaluehead: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "funcglobalvaluehead: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("funcglobalvaluehead", &scratch, &emitted);
    let output = run_with_timeout("funcglobalvaluehead", &program, &[]);
    assert_exit("funcglobalvaluehead", &output, 0);
}

/// A value-headed field call and a plain field import stay on their field.
///
/// The value `b` heads the field call, so the module's own func-typed global
/// `g` of the same simple name must not displace the field; the plain import
/// names its struct global directly, and its field call stays on the
/// module's binding. Both spellings read their values back, so a redirected
/// callee cannot exit zero (green controls at the phase base; the module
/// global would exit 98 in the value-headed case).
#[test]
fn should_keep_a_value_headed_field_call_and_a_plain_field_import_green() {
    let value_headed = r#"module spec.cri.funcglobalselfq;

func field(x: i32): i32 {
    return x;
}

func wrong(x: i32): i32 {
    return x + 98;
}

struct Holder {
    g: func(i32): i32;
}

struct Outer {
    holder: Holder;
}

let b: Outer = Outer { holder: Holder { g: field } };

let g: func(i32): i32 = wrong;

func main(): i32 {
    return b.holder.g(5) - 5;
}
"#;
    let output = run_memory("funcglobalselfq", value_headed);
    assert_no_asan("funcglobalselfq", &output);
    assert_exit("funcglobalselfq", &output, 0);

    let holder = r#"module spec.cri.funcglobalplainmod;

func pf(x: i32): i32 {
    return x + 20;
}

struct Holder {
    g: func(i32): i32;
}

export let holder: Holder = Holder { g: pf };
"#;
    let plain_main = r#"module spec.cri.funcglobalplain;

import { holder } from spec.cri.funcglobalplainmod;

func main(): i32 {
    return (holder.g(5) - 25) + (holder.g(8) - 28);
}
"#;
    let (scratch, output, c_path) = compile_pair("funcglobalplain", holder, plain_main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "funcglobalplain: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "funcglobalplain: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("funcglobalplain", &scratch, &emitted);
    let output = run_with_timeout("funcglobalplain", &program, &[]);
    assert_exit("funcglobalplain", &output, 0);
}

/// A bare func-global call is typed through the scope's binding.
///
/// The caller's import binds `f` to the holder's `func(i32): i32` global, so
/// the argument `"x"` reports the frozen conversion once instead of emitting
/// a translation unit the strict build rejects; a same-named `func(str)`
/// function in another loaded module is not in scope and never provides the
/// callee's signature (red at the phase base: accepted, then the strict
/// build failed with `incompatible type for argument 1 of '..._f'`).
#[test]
fn should_type_a_bare_func_global_call_through_the_scope_binding() {
    let holder = r#"module spec.cri.sigscopeholder;

func hg(x: i32): i32 {
    return x + 200;
}

export let f: func(i32): i32 = hg;
"#;
    let other = r#"module spec.cri.sigscopeother;

export func f(s: str): i32 {
    return 0;
}
"#;
    let mismatched = r#"module spec.cri.sigscopemain;

import { f } from spec.cri.sigscopeholder;

func main(): i32 {
    return f("x") - 0;
}
"#;
    let (_scratch, output, c_path) = compile_three("sigscope", holder, other, mismatched);
    assert_rejected_output(
        "sigscope",
        &output,
        &c_path,
        "E10200",
        &["cannot convert 'static str' to 'i32' implicitly"],
        &[],
    );
}

/// The matching and qualified func-global calls keep their values.
///
/// The imported `f(5)` and the qualified `holder.f(5)` both name the
/// holder's func-typed global, and the same-named `func(str)` in the other
/// loaded module stays out of scope for both; each call reads its value
/// back, so a redirected callee cannot exit zero (green controls at the
/// phase base).
#[test]
fn should_keep_the_matching_and_qualified_func_global_calls_green() {
    let holder = r#"module spec.cri.sigscopeholder;

func hg(x: i32): i32 {
    return x + 200;
}

export let f: func(i32): i32 = hg;
"#;
    let other = r#"module spec.cri.sigscopeother;

export func f(s: str): i32 {
    return 0;
}
"#;
    let control = r#"module spec.cri.sigscopegreen;

import { f } from spec.cri.sigscopeholder;

func main(): i32 {
    return (f(5) - 205) + (spec.cri.sigscopeholder.f(5) - 205);
}
"#;
    let (scratch, output, c_path) = compile_three("sigscopegreen", holder, other, control);
    assert_eq!(
        output.status.code(),
        Some(0),
        "sigscopegreen: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "sigscopegreen: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("sigscopegreen", &scratch, &emitted);
    let output = run_with_timeout("sigscopegreen", &program, &[]);
    assert_exit("sigscopegreen", &output, 0);
}

/// A module path reads its binding even when an import collides with it.
///
/// The import binds the head's name to another module's value, but the
/// qualified reading of a loaded module stays available (Ch 10): `dep.thing`
/// reads module `dep`'s binding, exactly as the resolver and the emitter
/// resolve it, and the run reads the module's value back.
#[test]
fn should_read_a_module_path_binding_despite_an_import_collision() {
    let dep = r#"module collisiondep;

export let thing: i32 = 3;
"#;
    let other = r#"module collisionother;

export let collisiondep: i32 = 4;
"#;
    let app = r#"module collisionapp;

import { collisiondep } from collisionother;

func main(): i32 {
    return collisiondep.thing - 3;
}
"#;
    let (scratch, output, c_path) = compile_three("collisionapp", dep, other, app);
    assert_eq!(
        output.status.code(),
        Some(0),
        "collisionapp: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "collisionapp: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("collisionapp", &scratch, &emitted);
    let output = run_with_timeout("collisionapp", &program, &[]);
    assert_exit("collisionapp", &output, 0);
}

/// An aliased import of a `take` function moves its argument out.
///
/// The alias binds the imported declaration, so the caller's move-out is
/// emitted exactly as the plain import's is: the callee owns the value and the
/// caller's slot empties, so no double release is possible (the run is
/// sanitizer-clean in the memory leg).
#[test]
fn should_move_out_an_aliased_take_arguments_place() {
    let holder = r#"module spec.cri.aliastakeholder;

export func sink(take s: String): i32 {
    return i32(len(s));
}

export func lend(borrow s: String): i32 {
    return i32(len(s));
}
"#;
    let main = r#"module spec.cri.aliastakemain;

import { stringOf } from std.text;
import { sink as drop, lend as use } from spec.cri.aliastakeholder;

func main(): i32 {
    let s: String = stringOf("abcd");
    let n: i32 = use(s);
    return drop(s) + n - 8;
}
"#;
    let output = run_pair_memory("aliastake", holder, main);
    assert_no_asan("aliastake", &output);
    assert_exit("aliastake", &output, 0);
}

/// A two-field module-path callee carries its receiver.
///
/// The module path reaches the binding `outer`, the field `holder`, and the
/// callable field `g`; the emitted call goes through the receiver the module
/// declares, so the strict build accepts it and the run reads its value back.
#[test]
fn should_call_a_two_field_module_path_callee() {
    let holder = r#"module spec.cri.deepheadmod;

func qf(x: i32): i32 {
    return x + 30;
}

struct Holder {
    g: func(i32): i32;
}

struct Outer {
    holder: Holder;
}

export let outer: Outer = Outer { holder: Holder { g: qf } };
"#;
    let main = r#"module spec.cri.deepheadmain;

func main(): i32 {
    return spec.cri.deepheadmod.outer.holder.g(5) - 35;
}
"#;
    let (scratch, output, c_path) = compile_pair("deephead", holder, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "deephead: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "deephead: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("deephead", &scratch, &emitted);
    let output = run_with_timeout("deephead", &program, &[]);
    assert_exit("deephead", &output, 0);
}

/// An aliased take callee moves its argument exactly like the plain twin.
///
/// The alias binds the imported declaration, so the own stage's argument
/// check sees the `take` mode: the move is marked and the later read reports
/// the code and site the plain spelling reports.
#[test]
fn should_move_out_an_aliased_take_argument_at_the_use_site() {
    let holder = r#"module spec.cri.aliastakeuseholder;

export func sink(take s: String): i32 {
    return i32(len(s));
}
"#;
    let aliased = r#"module spec.cri.aliastakeusemain;

import { stringOf } from std.text;
import { sink as drop } from spec.cri.aliastakeuseholder;

func main(): i32 {
    let s: String = stringOf("abcd");
    let n: i32 = drop(s);
    return n + i32(len(s)) - 4;
}
"#;
    let (_scratch, output, c_path) = compile_pair("aliastakeuse", holder, aliased);
    assert_rejected_output(
        "aliastakeuse",
        &output,
        &c_path,
        "E10020",
        &["'s' was moved and cannot be used", "main.clo:9:24"],
        &[],
    );

    let plain = r#"module spec.cri.aliastakeuseplain;

import { stringOf } from std.text;
import { sink } from spec.cri.aliastakeuseholder;

func main(): i32 {
    let s: String = stringOf("abcd");
    let n: i32 = sink(s);
    return n + i32(len(s)) - 4;
}
"#;
    let (_scratch, output, c_path) = compile_pair("aliastakeuseplain", holder, plain);
    assert_rejected_output(
        "aliastakeuseplain",
        &output,
        &c_path,
        "E10020",
        &["'s' was moved and cannot be used", "main.clo:9:24"],
        &[],
    );
}

/// A value-headed call resolves its receiver like the checker does.
///
/// The receiver path spells a loaded module whose binding chain differs from
/// the local binding of the same name, so the module path wins (Ch 10
/// MOD-13): the call goes through the module's binding and the run reads its
/// value back.
#[test]
fn should_call_a_module_path_receiver_beside_a_colliding_binding() {
    let holder = r#"module collcall.a;

func qf(x: i32): i32 {
    return x + 30;
}

struct Holder {
    g: func(i32): i32;
}

export let holder: Holder = Holder { g: qf };
"#;
    let main = r#"module collmain;

struct H2 {
    g: func(i32): i32;
}

struct Outer {
    holder: H2;
}

func wrong(x: i32): i32 {
    return x + 7;
}

let collcall: Outer = Outer { holder: H2 { g: wrong } };

func main(): i32 {
    return collcall.a.holder.g(5) - 35;
}
"#;
    let (scratch, output, c_path) = compile_pair("collcall", holder, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "collcall: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "collcall: the emitted C is empty, so the emitter is not implemented yet"
    );
    let program = build_host_c("collcall", &scratch, &emitted);
    let output = run_with_timeout("collcall", &program, &[]);
    assert_exit("collcall", &output, 0);
}

/// A borrow parameter reads a raw-deref argument without emptying it.
///
/// The callee only borrows the pointee, so the owner stays readable after
/// the call: both the aliased and the plain spelling run clean in the memory
/// leg and read the owner's value back.
#[test]
fn should_lend_a_raw_deref_argument_without_emptying_its_pointee() {
    let holder = r#"module spec.cri.ptrlendmod;

export func lend(borrow s: String): i32 {
    return i32(len(s));
}
"#;
    let aliased = r#"module spec.cri.ptrlendalias;

import { stringOf } from std.text;
import { lend as use } from spec.cri.ptrlendmod;

func main(): i32 {
    let a: String = stringOf("abcd");
    let q: raw String = unsafe &a;
    let n: i32 = use(unsafe *q);
    return n + i32(len(a)) - 8;
}
"#;
    let output = run_pair_memory("ptrlendalias", holder, aliased);
    assert_no_asan("ptrlendalias", &output);
    assert_exit("ptrlendalias", &output, 0);

    let plain = r#"module spec.cri.ptrlendplain;

import { stringOf } from std.text;
import { lend } from spec.cri.ptrlendmod;

func main(): i32 {
    let a: String = stringOf("abcd");
    let q: raw String = unsafe &a;
    let n: i32 = lend(unsafe *q);
    return n + i32(len(a)) - 8;
}
"#;
    let output = run_pair_memory("ptrlendplain", holder, plain);
    assert_no_asan("ptrlendplain", &output);
    assert_exit("ptrlendplain", &output, 0);
}

// ===== Qualified call identity =====

/// A qualified call reaches the module's function, not a same-named caller
/// function.
///
/// A qualified path names the loaded module and its function, so the module's
/// symbol, parameter modes, and return type govern the call. The value call
/// returns the module's value, the signature-divergent shape passes an `i32`
/// to the module's `f(n: i32)`, and the checker's mismatch keeps its one
/// `E10200` at the written argument. The qualified func-typed global and the
/// local-shadowed module head stay on their own callees, and a plain
/// same-module call is the unchanged baseline.
///
/// Before the fix the call was typed with the module's signature but emitted
/// the caller's same-named function, so the value call returned the wrong
/// value and the signature shape failed the strict C build on the caller's
/// symbol.
#[test]
fn should_emit_the_qualified_function_identity() {
    let lib = r#"module spec.cc.quallib;

export func f(): i32 {
    return 1;
}
"#;
    let main = r#"module spec.cc.qualmain;

func f(): i32 {
    return 2;
}

func main(): i32 {
    return spec.cc.quallib.f() - 1;
}
"#;
    let output = run_pair_memory("qualifiedcall", lib, main);
    assert_no_asan("qualifiedcall", &output);
    assert_exit("qualifiedcall", &output, 0);

    let siglib = r#"module spec.cc.qualsig;

export func f(n: i32): i32 {
    return n + 1;
}
"#;
    let sigmain = r#"module spec.cc.qualsigmain;

func f(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    return spec.cc.qualsig.f(5) - 6;
}
"#;
    let output = run_pair_memory("qualifiedsig", siglib, sigmain);
    assert_no_asan("qualifiedsig", &output);
    assert_exit("qualifiedsig", &output, 0);

    let mismatch = r#"module spec.cc.qualmismatch;

func f(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    return spec.cc.qualsig.f("x") - 6;
}
"#;
    let (_scratch, output, c_path) = compile_pair("qualifiedmismatch", siglib, mismatch);
    assert_rejected_output(
        "qualifiedmismatch",
        &output,
        &c_path,
        "E10200",
        &[
            "cannot convert 'static str' to 'i32' implicitly",
            "main.clo:8:30",
        ],
        &[],
    );

    let hooklib = r#"module spec.cc.qualhooklib;

export let hook: func(i32): i32 = (n: i32) => n + 1;
"#;
    let hookmain = r#"module spec.cc.qualhookmain;

func hook(n: i32): i32 { return n + 9; }

func main(): i32 {
    return spec.cc.qualhooklib.hook(5) - 6;
}
"#;
    let output = run_pair_memory("qualifiedhook", hooklib, hookmain);
    assert_no_asan("qualifiedhook", &output);
    assert_exit("qualifiedhook", &output, 0);

    let shadowlib = r#"module m390shadowlib;

export func f(): i32 { return 1; }
"#;
    let shadowmain = r#"module spec.cc.qualshadowmain;

struct Lib { f: func(): i32; }

func run(): i32 {
    let m390shadowlib: Lib = Lib { f: () => 2 };
    return m390shadowlib.f();
}

func main(): i32 {
    return run() - 2;
}
"#;
    let output = run_pair_memory("qualifiedshadow", shadowlib, shadowmain);
    assert_no_asan("qualifiedshadow", &output);
    assert_exit("qualifiedshadow", &output, 0);

    let control = r#"module spec.cc.qualcontrol;

func f(): i32 {
    return 2;
}

func main(): i32 {
    return f() - 2;
}
"#;
    let output = run_memory("qualifiedcontrol", control);
    assert_no_asan("qualifiedcontrol", &output);
    assert_exit("qualifiedcontrol", &output, 0);
}

/// A run-time-index view store keeps an owned source place alive.
///
/// The generic body sees an opaque element and defers the move. At a `str`
/// instance the store is a view: it widens the source place and leaves the
/// caller's owner readable. The constant-index twin stays accepted. At the
/// phase base the run-time lowering consumed the source into a transfer
/// temporary, emptied the caller's field, and left the view over the
/// temporary.
#[test]
fn should_keep_an_owned_source_place_alive_at_a_runtime_view_store() {
    let runtime = r#"module spec.cri.runtimeviewstore;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Wrap<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap<T>, borrow a: Box, i: i32): void {
    w.items[i] = a.v;
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["", ""], tag: stringOf("t") };
    let a: Box = Box { v: stringOf("ab") };
    store(w, a, 1);
    return i32(len(w.items[1])) - 2 + i32(len(a.v)) - 2;
}
"#;
    let output = run_memory("runtimeviewstore", runtime);
    assert_no_asan("runtimeviewstore", &output);
    assert_exit("runtimeviewstore", &output, 0);

    let constant = r#"module spec.cri.constantviewstore;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Wrap<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap<T>, borrow a: Box): void {
    w.items[0] = a.v;
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["", ""], tag: stringOf("t") };
    let a: Box = Box { v: stringOf("ab") };
    store(w, a);
    return i32(len(w.items[0])) - 2 + i32(len(a.v)) - 2;
}
"#;
    let output = run_memory("constantviewstore", constant);
    assert_no_asan("constantviewstore", &output);
    assert_exit("constantviewstore", &output, 0);
}
