//! Implementation tests for deferred-copy emission.
//!
//! The specification suite proves *what* a deferred block computes; these cases
//! prove *how* the emitter spells it. A scope's cleanup emits one copy of each
//! registered deferred block per cleanup activation (so the fall-through, the
//! continue/break edges, and the return rung each run it), every copy takes its
//! activation's label suffix while plain `__cl_b<N>` labels stay with the
//! main-body walk, and two emissions of one program must be byte-identical.

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

/// A deferred loop edge prints as the enclosing loop's C statement, and the
/// copy stays inside its own blocks.
///
/// The do-while body's deferred `continue` must reach the loop's condition
/// edge as a plain `continue;` — the copy is bounded to its own block range,
/// so no label machinery (and no outer block) leaks into it. The loop leaves
/// with `i == 3`.
#[test]
fn should_emit_the_loop_edge_inside_a_deferred_copy() {
    let source = r#"module spec.defer_integrity.impl8;

func main(): i32 {
    let i: i32 = 0;
    do {
        defer {
            if (i == 1) {
                continue;
            }
        }
        i = i + 1;
    } while (i < 3);
    return i;
}
"#;
    let emitted = emit_host("defer-impl8", source);
    assert_contains("defer-impl8", &emitted, "continue;");
    assert!(
        !emitted.contains("goto __cl_b"),
        "defer-impl8: the bounded copy must not need a label"
    );
    let output = run_host("defer-impl8-run", source);
    assert_exit("defer-impl8-run", &output, 3);
}

/// A guard-excluded outer scope's cleanup runs after an inner rung's cleanup.
///
/// The switch arm registers an outer defer with a `break` (no rung — it runs
/// inline) and an inner block whose defer keeps a rung. The inner cleanup must
/// run first (innermost first), so the inner block's `return` edge must not be
/// preempted by the outer inline `break`. The program answers 111: the inner
/// defer adds one, the outer adds ten and leaves the switch, and the function
/// falls out of the switch.
#[test]
fn should_keep_the_cleanup_order_when_an_inner_scope_keeps_a_rung() {
    let source = r#"module spec.defer_integrity.impl9;

let order: i32 = 0;

func f(k: i32): i32 {
    switch (k) {
    case 1: {
        defer {
            order = order + 10;
            break;
        }
        {
            defer {
                order = order + 1;
            }
            return 0;
        }
    }
    default: { }
    }
    return 0;
}

func main(): i32 {
    let r: i32 = f(1);
    return (r + 100) + order;
}
"#;
    let output = run_host("defer-impl9-run", source);
    assert_exit("defer-impl9-run", &output, 111);
}

/// A deferred return reads its own local before the drop of that local.
///
/// The deferred block's scope owns the string; the return's value reads it, so
/// the store must come first and the drop after it. The program answers one.
#[test]
fn should_store_a_deferred_returns_value_before_its_drop() {
    let source = r#"module spec.defer_integrity.impl10;

import { String, stringOf } from std.text;

func f(): i32 {
    defer {
        let s: String = stringOf("x");
        return i32(len(s));
    }
    return 0;
}

func main(): i32 {
    return f();
}
"#;
    let emitted = emit_host("defer-impl10", source);
    let store = emitted
        .find("__cl_ret = (i32)((cl_string_view(s)).len);")
        .expect("defer-impl10: the value store is missing");
    let drop = emitted
        .find("    if (s) { FreeString(s); }\n    s = NULL;\n    return __cl_ret;")
        .expect("defer-impl10: the drop-before-return tail is missing");
    assert!(
        store < drop,
        "defer-impl10: the deferred return must store its value before the drop"
    );
    let output = run_host("defer-impl10-run", source);
    assert_exit("defer-impl10-run", &output, 1);
}

/// The remaining cleanup of a scope runs after a returning defer's value store.
///
/// The newer defer returns 7; the older defer still runs, so the counter is
/// one and the program answers 71.
#[test]
fn should_run_the_remaining_defer_after_a_deferred_return() {
    let source = r#"module spec.defer_integrity.impl11;

let runs: i32 = 0;

func f(): i32 {
    defer {
        runs = runs + 1;
    }
    defer {
        return 7;
    }
    return 0;
}

func main(): i32 {
    let r: i32 = f();
    return r * 10 + runs;
}
"#;
    let emitted = emit_host("defer-impl11", source);
    let store = emitted
        .find("    __cl_ret = 7;")
        .expect("defer-impl11: the deferred return's store is missing");
    let remaining = emitted
        .find("cl_spec_defer_integrity_impl11_runs = (i32)((u32)cl_spec_defer_integrity_impl11_runs + (u32)1);")
        .expect("defer-impl11: the remaining defer's effect is missing");
    assert!(
        store < remaining,
        "defer-impl11: the returning defer must run before the remaining defer"
    );
    let output = run_host("defer-impl11-run", source);
    assert_exit("defer-impl11-run", &output, 71);
}

/// A deferred return from a nested scope jumps to the outer cleanup rung.
///
/// The inner block's deferred return leaves the enclosing function scope, so
/// the emitted copy jumps to that scope's rung and the outer drop runs there.
/// The program answers three and is sanitizer-clean.
#[test]
fn should_jump_to_the_outer_rung_when_a_deferred_return_leaves_a_nested_scope() {
    let source = r#"module spec.defer_integrity.impl12;

import { String, stringOf } from std.text;

func f(): i32 {
    let a: String = stringOf("abc");
    {
        defer {
            return i32(len(a));
        }
    }
    return 0;
}

func main(): i32 {
    return f();
}
"#;
    let emitted = emit_host("defer-impl12", source);
    assert_contains("defer-impl12", &emitted, "goto __cl_c1;");
    let output = run_host("defer-impl12-run", source);
    assert_exit("defer-impl12-run", &output, 3);
}

/// A deferred return releases the pending owned result before its store.
///
/// The body stores one owned string in the result; the deferred return
/// supersedes it, so the emitted copy releases the pending value first (the
/// release is null-safe, so the first store in a copy is a no-op) and then
/// stores its own. The program answers one.
#[test]
fn should_release_the_superseded_result_before_a_deferred_returns_store() {
    let source = r#"module spec.defer_integrity.impl13;

import { String, stringOf } from std.text;

func f(): String {
    defer {
        return stringOf("x");
    }
    return stringOf("ABCDEFGH");
}

func main(): i32 {
    return i32(len(f()));
}
"#;
    let emitted = emit_host("defer-impl13", source);
    let release = "    if (__cl_ret) { FreeString(__cl_ret); }\n    __cl_ret = NULL;\n    __cl_ret = cl_std_text_stringOf(cl_str(\"x\"));";
    assert_contains("defer-impl13", &emitted, release);
    let output = run_host("defer-impl13-run", source);
    assert_exit("defer-impl13-run", &output, 1);
}

/// An ordinary return releases a superseded deferred result before its store.
///
/// A deferred return stores an owned value; an older deferred `break` preempts
/// the pending return and control reaches the ordinary return. The ordinary
/// store releases the abandoned value first, so the emitted release precedes
/// the ordinary store. The program answers one.
#[test]
fn should_release_a_superseded_deferred_result_before_an_ordinary_return() {
    let source = r#"module spec.defer_integrity.impl14;

import { String, stringOf } from std.text;

func f(): String {
    switch (1) {
    case 1: {
        defer {
            break;
        }
        defer {
            return stringOf("inner-value");
        }
    }
    default: { }
    }
    return stringOf("f");
}

func main(): i32 {
    return i32(len(f()));
}
"#;
    let emitted = emit_host("defer-impl14", source);
    let release = "    if (__cl_ret) { FreeString(__cl_ret); }\n    __cl_ret = NULL;\n    __cl_ret = cl_std_text_stringOf(cl_str(\"f\"));";
    assert_contains("defer-impl14", &emitted, release);
    let output = run_host("defer-impl14-run", source);
    assert_exit("defer-impl14-run", &output, 1);
}

/// An older defer reading an owned local stays accepted before a newer return.
///
/// The newer defer returns; the older defer still runs first in cleanup order
/// and reads the owned local, whose release comes after it. The program is
/// accepted and answers seven.
#[test]
fn should_accept_an_older_defer_reading_an_owned_local_before_a_newer_return() {
    let source = r#"module spec.defer_integrity.impl15;

import { String, stringOf } from std.text;

func sink(v: str): void {}

func f(): i32 {
    let a: String = stringOf("abc");
    defer {
        sink(a);
    }
    defer {
        return 7;
    }
    return 0;
}

func main(): i32 {
    return f();
}
"#;
    let output = run_host("defer-impl15-run", source);
    assert_exit("defer-impl15-run", &output, 7);
}

/// A deferred read is reported once across every exit edge of its scope.
///
/// The loop body's deferred block reads a conditionally initialized local, so
/// the checker rejects the program. The block is checked on the continue, the
/// break, and the fall-through edges, but the diagnostic describes one source
/// occurrence and must be reported exactly once.
#[test]
fn should_report_a_deferred_read_once_across_exit_edges() {
    let source = r#"module spec.defer_integrity.impl16;

import { String, stringOf } from std.text;

func use(v: str): void {}

func f(c: bool): i32 {
    let i: i32 = 0;
    while (i < 4) {
        let a: String;
        if (c) {
            a = stringOf("abc");
        }
        defer {
            use(a);
        }
        i = i + 1;
        if (i == 1) {
            continue;
        }
        if (i == 3) {
            break;
        }
    }
    return 0;
}

func main(): i32 {
    return f(true);
}
"#;
    let scratch = ScratchDir::new("defer-impl16");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("defer-impl16: cannot write the program: {err}"));
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
        "defer-impl16: expected rejection; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("error E10021").count(),
        1,
        "defer-impl16: the deferred read must be reported once; stderr:\n{stderr}"
    );
}

/// The ladder's planning pass keeps the base local-name numbering.
///
/// The planning pass walks every rung so a growable-array cleanup's temporary
/// names match the numbering the emitted output has always used; a regression
/// would shift the suffixes down. The program is defer-free and answers zero.
#[test]
fn should_keep_the_base_local_name_numbering_for_a_ladder_drop() {
    let source = r#"module spec.defer_integrity.impl17;

import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;

func f(c: bool): void {
    let xs: Array<String> = arrayNew<String>();
    xs.push(stringOf("ab"));
    if (c) {
        return;
    }
    xs.push(stringOf("cd"));
}

func main(): i32 {
    f(true);
    return 0;
}
"#;
    let emitted = emit_host("defer-impl17", source);
    assert_contains("defer-impl17", &emitted, "__cl_elems_3");
    assert!(
        !emitted.contains("__cl_elems_2"),
        "defer-impl17: the planning pass must keep the base name numbering"
    );
    let output = run_host("defer-impl17-run", source);
    assert_exit("defer-impl17-run", &output, 0);
}
