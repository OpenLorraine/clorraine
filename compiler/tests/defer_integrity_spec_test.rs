//! Runnable specification tests for `defer` cleanup on every exit edge of its
//! scope.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process result. The chosen answer travels
//! in `main`'s return code, so a case that prints nothing states its result as
//! the exit code.
//!
//! The suite pins four shapes:
//!
//! * a `defer` inside a `while` body whose guarded `return` only fires on the
//!   final pass, so the answer is produced by the deferred return on the
//!   fall-through edge (the filed miss);
//! * two sibling `defer`s where the earlier one returns unconditionally and
//!   the later one guards a `return` that never fires, so the earlier cleanup
//!   answer must survive the later cleanup's evaluation;
//! * a `defer` that increments a module counter on both a `return` edge and
//!   the fall-through edge of the same scope, so both calls run it exactly
//!   once;
//! * a single scope reached through `continue`, fall-through, and `break`,
//!   where each of the three cleanup sites must run the `defer` exactly once.
//!
//! The memory helper runs the same program as an ordinary strict host build
//! and adds leak and use-after-free detection when `RDC_ASAN` is set.

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
            "clc-defer-{}-{tag}-{sequence}",
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

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit. A rejection is a failure of the case,
/// not a result to assert, because the run cases are about runtime behavior.
fn emit_host(tag: &str, source: &str) -> String {
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
///
/// Panics with the compiler's stderr when the build fails. Returns the built
/// program path.
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
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
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
///
/// A leaked allocation fails the run and a double free aborts, so a clean exit
/// pins release-once for the case.
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
///
/// The plain build still aborts in the C library on a double release, so the
/// exit code stays observable; the sanitizer run adds leak and use-after-free
/// detection for the cleanup probes.
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
///
/// The check is meaningful under `RDC_ASAN`; a plain strict build has no
/// sanitizer output to find. A leak or double free that aborts the run fails
/// the exit-code assertion first.
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

/// A `defer` in a loop body runs on the fall-through edge of its scope.
///
/// The `while` body loops twice and `continue`s on the first pass. On the
/// second pass `i` is two when the scope ends, so the deferred `return 9`
/// fires and `main` (through `f`) answers nine. The guarded return is only
/// reachable from the fall-through cleanup, so an implementation that runs
/// `defer` on the `continue` edge but misses the fall-through edge answers
/// zero instead. The run is host-ASan clean.
#[test]
fn should_run_the_defer_on_the_fall_through_edge_of_a_loop_body() {
    let source = r#"module spec.defer_integrity.st1;

func f(): i32 {
    let i: i32 = 0;
    while (i < 2) {
        defer {
            if (i == 2) {
                return 9;
            }
        }
        i = i + 1;
        if (i == 1) {
            continue;
        }
    }
    return 0;
}

func main(): i32 {
    return f();
}
"#;
    let output = run_memory("defer-st1", source);
    assert_exit("defer-st1", &output, 9);
    assert_no_asan("defer-st1", &output);
}

/// An earlier unconditional defer return is preserved while a later defer's
/// guarded return does not fire.
///
/// The scope holds two `defer`s: the first returns seven unconditionally and
/// the second only returns zero when `i` is ninety-nine. Running the cleanup
/// block must let the first defer's pending return stand, so `f` answers seven
/// and `main` answers seven. If the later defer's guard is evaluated as if it
/// returned (or the first defer is skipped), the answer differs. The run is
/// host-ASan clean.
#[test]
fn should_keep_an_earlier_defer_return_past_a_later_guarded_defer() {
    let source = r#"module spec.defer_integrity.st2;

func f(): i32 {
    let i: i32 = 1;
    {
        defer {
            return 7;
        }
        defer {
            if (i == 99) {
                return 0;
            }
        }
    }
    return 0;
}

func main(): i32 {
    return f();
}
"#;
    let output = run_memory("defer-st2", source);
    assert_exit("defer-st2", &output, 7);
    assert_no_asan("defer-st2", &output);
}

/// A `defer` runs on the return edge as well as the fall-through edge.
///
/// `f(true)` and `f(false)` each enter a scope whose `defer` increments the
/// module counter. The `true` call takes the `return 1` edge; the `false` call
/// falls through to `return 0`. Both must run the `defer` exactly once, so
/// `count` is two and the returned sum is one; `main` folds them into `count *
/// 10 + f(true) + f(false)` and answers twenty-one. The run is host-ASan clean.
#[test]
fn should_run_a_defer_on_both_the_return_and_fall_through_edges() {
    let source = r#"module spec.defer_integrity.st3;

let count: i32 = 0;

func f(c: bool): i32 {
    {
        defer {
            count = count + 1;
        }
        if (c) {
            return 1;
        }
    }
    return 0;
}

func main(): i32 {
    return count * 10 + f(true) + f(false);
}
"#;
    let output = run_memory("defer-st3", source);
    assert_exit("defer-st3", &output, 21);
    assert_no_asan("defer-st3", &output);
}

/// Three cleanup sites in one loop scope each run the defer exactly once.
///
/// The `while` body `continue`s on the first pass, falls through on the second,
/// and `break`s on the third, so the body's `defer` must run once per pass.
/// With three runs and `i` left at three, `runs * 10 + i` is thirty-three. A
/// miss on any one edge changes the count. The strict host build must accept
/// the emitted C.
#[test]
fn should_run_the_defer_once_per_cleanup_site_reached_through_continue_fallthrough_and_break() {
    let source = r#"module spec.defer_integrity.st4;

let runs: i32 = 0;

func f(): i32 {
    let i: i32 = 0;
    while (i < 4) {
        defer {
            runs = runs + 1;
        }
        i = i + 1;
        if (i == 1) {
            continue;
        }
        if (i == 3) {
            break;
        }
    }
    return runs * 10 + i;
}

func main(): i32 {
    return f();
}
"#;
    let output = run_host("defer-st4", source);
    assert_exit("defer-st4", &output, 33);
}
