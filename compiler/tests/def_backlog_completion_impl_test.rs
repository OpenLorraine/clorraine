//! Implementation tests for the generic-lend and move-out defect fixes.
//!
//! The specification suite states the accepted and rejected programs; this
//! suite pins the shapes the fixes rely on: the lend duty reports once per
//! move site, the run-time element index is bound once and shared by the read
//! and the move-out, a guarded index keeps short-circuit evaluation, the store
//! and return paths empty the selected slot before the temporary release, and
//! re-emission is byte-identical.
//!
//! A memory case runs under AddressSanitizer when `RDC_ASAN` is set and
//! otherwise as an ordinary strict host build.

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
            "clc-dbcimpl-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the scratch directory, the compiler output,
/// and the C path.
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
        "{tag}: the emitted C is empty"
    );
    emitted
}

/// Emits the same program twice into one directory and returns both units.
///
/// Both emissions share the source path, so their `#line` directives match and
/// a difference in the two strings is a difference in the translation itself.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
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
        emitted.push(std::fs::read_to_string(&c_path).unwrap_or_default());
    }
    (emitted[0].clone(), emitted[1].clone())
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

/// Runs a memory-safety case under AddressSanitizer when `RDC_ASAN` is set and
/// as an ordinary strict host build otherwise.
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

/// Asserts the text contains a fragment.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: the emitted C does not contain {needle:?}"
    );
}

/// Counts the occurrences of a fragment.
fn count_occurrences(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

/// A program whose move site is reached by two owning instantiations.
const TWO_OWNING_INSTANCES: &str = r#"module impl.dbc.one;
import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;

func id<T>(x: T): T {
    return x;
}

func main(): i32 {
    let a: String = id<String>(stringOf("ab"));
    let b: Array<String> = id<Array<String>>(arrayNew<String>());
    return i32(len(a)) + i32(b.count);
}
"#;

/// The same move site must report once even when more than one owning
/// instantiation reaches it.
#[test]
fn should_report_once_when_two_owning_instances_move_the_same_lend() {
    let (_scratch, output, c_path) = compile_only("owning-once", TWO_OWNING_INSTANCES);
    assert_ne!(
        output.status.code(),
        Some(0),
        "two owning instances must reject the lend move"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10034:").count(),
        1,
        "the same move site must report once; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "a rejected program must not produce a translation unit"
    );
}

/// A run-time element take binds its index once and uses the same temporary in
/// the call and in the move-out.
#[test]
fn should_bind_the_index_once_when_a_runtime_index_is_taken() {
    let source = r#"module impl.dbc.takei;
import { String, stringOf } from std.text;

func sink<T>(take s: T): i32 {
    return 0;
}

func at<T>(mk: func(): T[2], i: i32): i32 {
    return sink<T>(mk()[i]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return at<String>(mkString, 1);
}
"#;
    let emitted = emit_host("take-index", source);
    assert_eq!(
        count_occurrences(&emitted, "__cl_tmp1 = i;"),
        1,
        "take-index: the index must be evaluated once"
    );
    assert_contains("take-index", &emitted, "_sink__String(__cl_tmp0[__cl_tmp1])");
    assert_contains("take-index", &emitted, "__cl_tmp0[__cl_tmp1] = NULL;");
    let output = run_memory("take-index-run", source);
    assert_exit("take-index-run", &output, 0);
}

/// A guarded take keeps short-circuit evaluation: the call and its move-out
/// run only when the guard is true, and the skipped path stays clean.
#[test]
fn should_keep_the_short_circuit_when_a_guarded_index_is_taken() {
    let source = r#"module impl.dbc.guardi;
import { String, stringOf } from std.text;

func sink<T>(take s: T): i32 {
    return 0;
}

func guarded<T>(mk: func(): T[2], flag: bool, i: i32): i32 {
    if (flag && sink<T>(mk()[i]) > 0) {
        return 1;
    }
    return 0;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return guarded<String>(mkString, false, 1);
}
"#;
    let emitted = emit_host("guard-index", source);
    assert_contains("guard-index", &emitted, "__cl_tmp0[__cl_tmp1] = NULL");
    assert_eq!(
        count_occurrences(&emitted, "__cl_tmp1 = i"),
        1,
        "guard-index: the index must be evaluated once"
    );
    let guard_line = emitted
        .lines()
        .find(|line| line.contains("if ((flag"))
        .expect("guard-index: the guarded condition must be emitted");
    assert!(
        guard_line.contains("__cl_tmp1 = i"),
        "guard-index: the index assignment must stay inside the guarded condition"
    );
    let output = run_memory("guard-index-run", source);
    assert_exit("guard-index-run", &output, 0);
}

/// A runtime-indexed element stored into a binding moves out of the filled
/// destination temporary before the release.
#[test]
fn should_empty_the_stored_slot_when_a_runtime_index_is_stored() {
    let source = r#"module impl.dbc.storei;
import { String, stringOf } from std.text;

func storeAt<T>(mk: func(): T[2], i: i32): T {
    let s: T = mk()[i];
    return s;
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(storeAt<String>(mkString, 1)));
}
"#;
    let emitted = emit_host("store-index", source);
    assert_contains("store-index", &emitted, "mk(");
    assert_contains("store-index", &emitted, "s = __cl_tmp0[__cl_tmp1];");
    assert_contains("store-index", &emitted, "__cl_tmp0[__cl_tmp1] = NULL;");
    let output = run_memory("store-index-run", source);
    assert_exit("store-index-run", &output, 3);
}

/// A constant-index element returned from a temporary is emptied before the
/// temporary's release frees its slots.
#[test]
fn should_empty_before_release_when_a_constant_index_is_returned() {
    let source = r#"module impl.dbc.returnc;
import { String, stringOf } from std.text;

func first<T>(mk: func(): T[2]): T {
    return mk()[0];
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(first<String>(mkString)));
}
"#;
    let emitted = emit_host("return-const", source);
    assert_contains("return-const", &emitted, "mk(");
    let zeroed = emitted
        .find("__cl_tmp0[0] = NULL;")
        .expect("return-const: the returned slot must be emptied");
    let released = emitted
        .rfind("FreeString(__cl_tmp0[0])")
        .expect("return-const: the temporary slot must be released");
    assert!(
        released > zeroed,
        "return-const: the slot must be emptied before the release"
    );
    let output = run_memory("return-const-run", source);
    assert_exit("return-const-run", &output, 2);
}

/// A run-time index on a returned element is read once and shared by the
/// returned copy and the source move.
#[test]
fn should_read_the_return_index_once_when_a_runtime_index_is_returned() {
    let source = r#"module impl.dbc.returni;
import { String, stringOf } from std.text;

func first<T>(mk: func(): T[2], i: i32): T {
    return mk()[i];
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return i32(len(first<String>(mkString, 1)));
}
"#;
    let emitted = emit_host("return-index", source);
    assert_eq!(
        count_occurrences(&emitted, "__cl_tmp1 = i;"),
        1,
        "return-index: the index must be evaluated once"
    );
    assert_contains(
        "return-index",
        &emitted,
        "__cl_tmp0[__cl_tmp1] = NULL;",
    );
    let output = run_memory("return-index-run", source);
    assert_exit("return-index-run", &output, 3);
}

/// Every level of a nested run-time index is bound and the exact slot is
/// emptied.
#[test]
fn should_empty_every_level_when_a_multi_level_index_is_taken() {
    let source = r#"module impl.dbc.mli;
import { String, stringOf } from std.text;

func sink<T>(take s: T): i32 {
    return 0;
}

func at<T>(mk: func(): T[2][2], i: i32, j: i32): i32 {
    return sink<T>(mk()[i][j]);
}

func mkGrid(): String[2][2] {
    return [[stringOf("a"), stringOf("bb")], [stringOf("ccc"), stringOf("dddd")]];
}

func main(): i32 {
    return at<String>(mkGrid, 1, 1);
}
"#;
    let emitted = emit_host("multi-index", source);
    assert_contains("multi-index", &emitted, "__cl_tmp1 = i;");
    assert_contains("multi-index", &emitted, "__cl_tmp2 = j;");
    assert_contains(
        "multi-index",
        &emitted,
        "_sink__String(__cl_tmp0[__cl_tmp1][__cl_tmp2])",
    );
    assert_contains(
        "multi-index",
        &emitted,
        "__cl_tmp0[__cl_tmp1][__cl_tmp2] = NULL;",
    );
    let output = run_memory("multi-index-run", source);
    assert_exit("multi-index-run", &output, 0);
}

/// Re-emitting a program with a run-time element index is deterministic.
#[test]
fn should_emit_identical_translations_when_a_runtime_index_is_taken() {
    let source = r#"module impl.dbc.det;
import { String, stringOf } from std.text;

func sink<T>(take s: T): i32 {
    return 0;
}

func at<T>(mk: func(): T[2], i: i32): i32 {
    return sink<T>(mk()[i]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return at<String>(mkString, 1);
}
"#;
    let (first, second) = emit_twice("determinism", source);
    assert_eq!(
        first, second,
        "determinism: the same program emitted two different translations"
    );
}
