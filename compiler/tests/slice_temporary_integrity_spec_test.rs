//! Runnable specification tests for row-typed slice views over a temporary
//! source.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. The chosen answer
//! travels in `main`'s return code, so a case that prints nothing states its
//! result as the exit code.
//!
//! The suite covers five shapes:
//!
//! * a row-typed fixed grid passed to a slice parameter and summed over its
//!   second column (a behavior pin);
//! * a generic row move out of a view over a call-result temporary, which must
//!   move the row and release the source once;
//! * a concrete row move out of a view over a named local owner, which keeps
//!   the element-move limit `E10026` (a regression pin);
//! * a direct nested element read through a row-typed view over a named local,
//!   where the strict host build must accept the emitted C;
//! * an explicit slice type argument that disagrees with the source's row,
//!   which is rejected with `E10200`.
//!
//! Additional cases pin the view-element conversion rule and constant extent
//! resolution: an inferred row view bound to a scalar view is rejected, and a
//! `const`-sized or arithmetic row extent resolves to its constant in both the
//! declared view and the explicit type argument.
//!
//! The first and third cases already behave as required, so they are behavior
//! and regression pins. The generic row move, the nested read, and the
//! mismatched explicit argument are the expectations that fail until the fix
//! lands.
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
            "clc-stispec-{}-{tag}-{sequence}",
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
/// detection for the row-move probes.
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

/// Compiles `source` and returns the scratch directory, output, and C path.
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

/// Asserts the front end rejects the program with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
    assert_rejected_with(tag, source, code, &[]);
}

/// Asserts the front end rejects the program with `code`, that the message
/// contains every fragment, and that no translation unit is written.
fn assert_rejected_with(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
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

/// A fixed grid passed to a row-typed slice parameter sums the second column.
///
/// This is a behavior pin: `sink` walks the `i32[][3]` view of the `i32[2][3]`
/// grid and adds each row's second element (`2 + 5`), so `main` answers seven.
/// The strict host build must accept the emitted C.
#[test]
fn should_sum_the_second_column_when_a_fixed_grid_is_viewed_by_rows() {
    let source = r#"module spec.sti.st1;

import { slice } from std.mem;

func sink(xs: i32[][3]): i32 {
    let sum: i32 = 0;
    let i: usize = 0;
    while (i < len(xs)) {
        sum = sum + xs[i][1];
        i = i + 1;
    }
    return sum;
}

func main(): i32 {
    let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return sink(slice<i32[3]>(grid, 0, 2));
}
"#;
    let output = run_host("sti-st1", source);
    assert_exit("sti-st1", &output, 7);
}

/// A generic row move out of a view over a call-result temporary stays valid.
///
/// `storeRows<String>` binds `slice<T[2]>(make(), 0, 2)[i]` and returns the
/// selected row. The row must move out of the temporary before the temporary
/// releases the grid, so the returned row still owns its two strings and
/// `main` answers two (the row length). The generic spelling compiles today and
/// corrupts: the view source is released twice, which AddressSanitizer reports
/// as a double free. This case is red until the row move empties the source
/// slot.
#[test]
fn should_move_the_row_when_a_generic_view_over_a_call_temporary_is_bound() {
    let source = r#"module spec.sti.st2;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func storeRows<T>(make: func(): T[2][2], i: usize): T[2] {
    let row: T[2] = slice<T[2]>(make(), 0, 2)[i];
    return row;
}

func makeGrid(): String[2][2] {
    return [[stringOf("a"), stringOf("bb")], [stringOf("ccc"), stringOf("dddd")]];
}

func main(): i32 {
    let row: String[2] = storeRows<String>(makeGrid, usize(1));
    return i32(len(row));
}
"#;
    let output = run_memory("sti-st2", source);
    assert_exit("sti-st2", &output, 2);
    assert_no_asan("sti-st2", &output);
}

/// A concrete row move out of a view over a named local owner stays rejected.
///
/// This is a regression pin: the element-move limit reports `E10026` for a
/// concrete `String[2]` row moved out of a view over a named local `grid`. The
/// limit is unchanged by the fix, and a rejected program writes no translation
/// unit.
#[test]
fn should_reject_a_concrete_row_move_when_the_view_covers_a_named_local() {
    let source = r#"module spec.sti.st3;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let grid: String[2][2] = [[stringOf("a"), stringOf("bb")], [stringOf("ccc"), stringOf("dddd")]];
    let i: usize = 1;
    let row: String[2] = slice<String[2]>(grid, 0, 2)[i];
    return i32(len(row));
}
"#;
    assert_rejected("sti-st3", source, "E10026");
}

/// A direct nested element read through a row-typed view selects the row cell.
///
/// `slice<i32[2]>(xs, 0, 2)[1][0]` views the `i32[2][2]` local by rows, takes
/// the second row, and reads its first cell (`3`), so `main` answers three. The
/// strict host build must accept the emitted C: before the fix the nested
/// access is mis-typed and the host compiler rejects the unit.
#[test]
fn should_read_the_nested_cell_when_a_row_typed_view_is_indexed_twice() {
    let source = r#"module spec.sti.st4;

import { slice } from std.mem;

func main(): i32 {
    let xs: i32[2][2] = [[1, 2], [3, 4]];
    return slice<i32[2]>(xs, 0, 2)[1][0];
}
"#;
    let output = run_host("sti-st4", source);
    assert_exit("sti-st4", &output, 3);
}

/// An explicit slice type argument that disagrees with the source row is
/// rejected.
///
/// `slice<i32>(grid, 0, 2)` fixes the view's element to `i32`, but `grid` is an
/// `i32[2][3]` whose row elements are `i32[3]`, so the conversion is not
/// allowed and the front end reports `E10200` (`cannot convert ...`). Before
/// the fix the program is accepted. A rejected program writes no translation
/// unit.
#[test]
fn should_reject_when_the_explicit_slice_type_disagrees_with_the_source_row() {
    let source = r#"module spec.sti.st5;

import { slice } from std.mem;

func sink(xs: i32[]): i32 {
    return i32(len(xs));
}

func main(): i32 {
    let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return sink(slice<i32>(grid, 0, 2));
}
"#;
    assert_rejected_with("sti-st5", source, "E10200", &["cannot convert"]);
}

/// An inferred row view bound to a scalar view is rejected.
///
/// `slice(g, 0, 2)` over an `i32[2][3]` grid is a view of `i32[3]` rows, so
/// binding it to a declared `i32[]`, whose elements are `i32`, is an implicit
/// conversion the language does not allow: the front end reports `E10200` and
/// writes no translation unit. A context that accepted the untyped result
/// would emit C whose view type disagrees with the declared one.
#[test]
fn should_reject_when_an_inferred_row_view_is_bound_to_a_scalar_view() {
    let source = r#"module spec.sti.st6;

import { slice } from std.mem;

func main(): i32 {
    let g: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let v: i32[] = slice(g, 0, 2);
    return i32(len(v));
}
"#;
    assert_rejected("sti-st6", source, "E10200");
}

/// A `const`-sized row extent resolves to its constant everywhere.
///
/// The view parameter `i32[][M]` and the explicit argument `slice<i32[M]>`
/// both name `i32[3]` rows, exactly as the type stage resolves `M`, so the
/// view the call builds matches the declared typedef and `sink` reads the
/// second row cell (`3`). An extent resolved as a slice would disagree with
/// the call site.
#[test]
fn should_run_when_a_const_sized_row_view_and_explicit_argument_agree() {
    let source = r#"module spec.sti.st7;

import { slice } from std.mem;

const M: usize = 3;

func sink(xs: i32[][M]): i32 {
    return xs[0][2];
}

func main(): i32 {
    let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return sink(slice<i32[M]>(grid, 0, 2));
}
"#;
    let output = run_host("sti-st7", source);
    assert_exit("sti-st7", &output, 3);
}

/// An arithmetic extent folds with the same constant environment.
///
/// `slice<i32[2 + 1]>` names the same `i32[3]` row element as the source
/// grid, so the call is accepted and `sink` reads `3`.
#[test]
fn should_run_when_an_arithmetic_extent_names_the_source_row() {
    let source = r#"module spec.sti.st8;

import { slice } from std.mem;

func sink(xs: i32[][3]): i32 {
    return xs[0][2];
}

func main(): i32 {
    let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return sink(slice<i32[2 + 1]>(grid, 0, 2));
}
"#;
    let output = run_host("sti-st8", source);
    assert_exit("sti-st8", &output, 3);
}
