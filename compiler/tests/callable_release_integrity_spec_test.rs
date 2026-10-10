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
