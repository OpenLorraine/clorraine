//! Implementation tests for deferred-copy emission.
//!
//! The specification suite proves *what* a deferred block computes; these cases
//! prove *how* the emitter spells it. A scope's cleanup emits one copy of each
//! registered deferred block per cleanup activation (so the fall-through, the
//! continue/break edges, and the return rung each run it), the first copy keeps
//! the plain `__cl_b<N>` label spelling while later copies suffix their labels,
//! and two emissions of one program must be byte-identical.

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
            "clc-deferimpl-{}-{tag}-{sequence}",
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
/// Returns the emitted translation unit. These cases are about what the
/// emitter produces for accepted programs, so a rejection is a failure.
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

/// Emits the scratch directory's `main.clo` with a relative source path.
///
/// Both runs of a determinism case then name the same logical source, so the
/// emitted `#line` directives stay comparable.
fn emit_relative(scratch: &ScratchDir, tag: &str, output_name: &str) -> String {
    let output_path = scratch.path().join(output_name);
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .current_dir(scratch.path())
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            "main.clo".to_owned(),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&output_path).unwrap_or_default()
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
///
/// The emitted C is built with the same strict flag set the specification
/// suite uses, so a spelling that only builds without `-Werror` still fails
/// here.
fn run_host(tag: &str, source: &str) -> Output {
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
    run_with_timeout(tag, &program, &[])
}

/// Asserts the text contains a fragment.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: the emitted C does not contain {needle:?}"
    );
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

/// The counter program used by the copy-count and determinism cases.
///
/// One loop scope registers a deferred increment and reaches three cleanup
/// sites per run: the continue edge, the fall-through end, and the break edge.
const THREE_SITE_SOURCE: &str = r#"module spec.defer_integrity.impl1;

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

/// A scope's cleanup emits one copy of its deferred block per cleanup site.
///
/// The loop body registers one deferred increment and reaches three sites over
/// the run, so the emitter must spell the increment three times: the fall-through
/// end, the continue edge, and the break edge. The program still answers the
/// once-per-site value (three runs, `i` left at three).
#[test]
fn should_emit_one_deferred_copy_per_cleanup_site() {
    let emitted = emit_host("defer-impl1", THREE_SITE_SOURCE);
    let increment =
        "cl_spec_defer_integrity_impl1_runs = (i32)((u32)cl_spec_defer_integrity_impl1_runs + (u32)1);";
    assert_eq!(
        emitted.matches(increment).count(),
        3,
        "one deferred copy per cleanup site (fall-through, continue, break)"
    );
    let output = run_host("defer-impl1-run", THREE_SITE_SOURCE);
    assert_exit("defer-impl1-run", &output, 33);
}

/// A deferred body that needs block labels takes a per-copy suffixed
/// namespace.
///
/// The `for` step with a call forces label-and-`goto` blocks inside the
/// deferred copy; the same block is copied at the continue edge and at the
/// fall-through end, so the two copies spell their labels `__1` and `__2`.
/// Plain labels stay reserved for the main-body walk, so a copy can never
/// collide with a block the main flow also prints. The program answers two.
#[test]
fn should_suffix_labels_per_deferred_copy() {
    let source = r#"module spec.defer_integrity.impl2;

func mk(): i32[2] {
    return [1, 2];
}

func idx(): usize {
    return 1;
}

func f(): i32 {
    let i: i32 = 0;
    while (i < 2) {
        defer {
            let j: i32 = 0;
            for (; j < 3; j = j + mk()[idx()]) {
            }
        }
        i = i + 1;
        if (i == 1) {
            continue;
        }
    }
    return i;
}

func main(): i32 {
    return f();
}
"#;
    let emitted = emit_host("defer-impl2", source);
    assert_contains("defer-impl2", &emitted, "goto __cl_b5__1;");
    assert_contains("defer-impl2", &emitted, "__cl_b5__1:");
    assert_contains("defer-impl2", &emitted, "goto __cl_b5__2;");
    assert_contains("defer-impl2", &emitted, "__cl_b5__2:");
    let output = run_host("defer-impl2-run", source);
    assert_exit("defer-impl2-run", &output, 2);
}

/// Two emissions of one deferred-copy program must be byte-identical.
#[test]
fn should_emit_identically_when_a_defer_program_is_emitted_twice() {
    let scratch = ScratchDir::new("defer-impl3");
    std::fs::write(scratch.path().join("main.clo"), THREE_SITE_SOURCE)
        .expect("write the program");
    let first = emit_relative(&scratch, "defer-impl3", "first.c");
    let second = emit_relative(&scratch, "defer-impl3", "second.c");
    assert_eq!(
        first, second,
        "defer-impl3: two emissions of one program must be byte-identical"
    );
}

/// A deferred `break` builds and leaves the loop on the iteration it fires.
///
/// The break target block also lies on the main flow (the loop exit), so the
/// deferred copy's labels must not collide with the main walk's labels: every
/// copy takes a suffixed namespace while plain labels stay reserved for the
/// main body. The loop leaves when the deferred guard sees `i == 2`, so the
/// program answers two.
#[test]
fn should_build_and_run_when_a_deferred_break_exits_the_loop() {
    let source = r#"module spec.defer_integrity.impl4;

func f(): i32 {
    let i: i32 = 0;
    while (i < 5) {
        defer {
            if (i == 2) {
                break;
            }
        }
        i = i + 1;
    }
    return i;
}

func main(): i32 {
    return f();
}
"#;
    let output = run_host("defer-impl4-run", source);
    assert_exit("defer-impl4-run", &output, 2);
}

/// A nested defer inside a continuing deferred block stays accepted.
///
/// The outer defer's `continue` runs the inner deferred block exactly once,
/// and the enclosing scope's cleanup must not be re-checked on that same edge:
/// a second check reads the released string and reports a spurious
/// use-before-initialization. The program answers the three-character length.
#[test]
fn should_accept_when_a_nested_defer_precedes_a_continue() {
    let source = r#"module spec.defer_integrity.impl5;

import { String, stringOf } from std.text;

func f(): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 1) {
        let s: String = stringOf("abc");
        defer {
            defer {
                total = total + i32(len(s));
            }
            continue;
        }
        i = i + 1;
    }
    return total;
}

func main(): i32 {
    return f();
}
"#;
    let output = run_host("defer-impl5-run", source);
    assert_exit("defer-impl5-run", &output, 3);
}

/// A use of a value an outer scope's defer moved stays rejected.
///
/// The outer defer moves the owned string; the inner deferred block's presence
/// must not stop the outer scope's cleanup from checking that move, so the
/// later use (the next iteration's deferred call) is a use-after-move. The
/// compiler must report `E10020` and exit 1 rather than accepting the program.
#[test]
fn should_reject_using_a_value_an_outer_defer_moved() {
    let source = r#"module spec.defer_integrity.impl6;

import { String, stringOf } from std.text;

let runs: i32 = 0;

func sink(take s: String): void {
    runs = runs + 1;
}

func f(): i32 {
    let x: String = stringOf("x");
    let i: i32 = 0;
    while (i < 2) {
        defer {
            sink(x);
        }
        {
            defer {
                runs = runs + 10;
            }
            i = i + 1;
            continue;
        }
    }
    return runs;
}

func main(): i32 {
    return f();
}
"#;
    let scratch = ScratchDir::new("defer-impl6");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("defer-impl6: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "defer-impl6: expected rejection; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("E10020"),
        "defer-impl6: expected a use-after-move diagnostic; stderr:\n{stderr}"
    );
}

/// A deferred `break` inside a guard-stepped `for` builds and leaves the loop.
///
/// A `for` whose step needs a guard is emitted as a label-and-`goto` form with
/// no surrounding `break` target, so the deferred copy's edge leaves its own
/// block range and must keep the main walk's plain label spelling; a namespaced
/// label there would never be defined. One body pass runs, so the program
/// answers ten (`n == 1`, `j == 0`).
#[test]
fn should_build_when_a_deferred_break_leaves_a_guard_stepped_for() {
    let source = r#"module spec.defer_integrity.impl7;

func mk(): i32[2] {
    return [1, 2];
}

func f(): i32 {
    let j: i32 = 0;
    let n: i32 = 0;
    for (; j < 3; j = j + mk()[1]) {
        defer {
            break;
        }
        n = n + 1;
    }
    return n * 10 + j;
}

func main(): i32 {
    return f();
}
"#;
    let output = run_host("defer-impl7-run", source);
    assert_exit("defer-impl7-run", &output, 10);
}
