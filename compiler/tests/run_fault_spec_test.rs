//! Host runtime fault specification tests.
//!
//! For each program under `spec/tests/run/`, the case compiles it with the
//! built `clc` binary for the host profile, compiles the emitted C with the
//! host C compiler under the fixed warning flags, runs it, and asserts that it
//! aborts on the exact first stderr line. That line is the fault contract:
//! `error CODE: message`. The checks abort without running the cleanup ladder,
//! so a successful exit is always a failure. Until the emitter lands the
//! compiler writes an empty translation unit; the case checks for non-empty C
//! so the failure names the missing emitter rather than a later link error.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

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
            "clc-run-{}-{tag}-{sequence}",
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

/// Root of the frozen specification, resolved from the crate location.
fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec")
}

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Compiles one run-suite program, builds the emitted C on the host, runs it,
/// and asserts the exact first stderr line and a nonzero exit.
fn assert_run_fault(file_name: &str, expected_first_line: &str) {
    let scratch = ScratchDir::new(file_name.trim_end_matches(".clo"));
    let source = spec_root().join("tests/run").join(file_name);
    let c_path = scratch.path().join("out.c");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source.display().to_string(),
    ]);
    let front_end = String::from_utf8_lossy(&compile.stderr);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{file_name}: the front end rejected a valid program; stderr:\n{front_end}"
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{file_name}: the emitted C is empty, so the emitter is not implemented yet"
    );

    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99".to_owned(),
            "-O0".to_owned(),
            "-g".to_owned(),
            "-Wall".to_owned(),
            "-Wextra".to_owned(),
            "-Werror".to_owned(),
            "-o".to_owned(),
            program.display().to_string(),
            c_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{file_name}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let run = Command::new(&program)
        .output()
        .expect("failed to run the fault program");
    assert!(
        !run.status.success(),
        "{file_name}: a runtime fault must exit nonzero"
    );
    let first = String::from_utf8_lossy(&run.stderr)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(
        first, expected_first_line,
        "{file_name}: unexpected first stderr line"
    );
}

/// Integer division by a zero divisor aborts with E11210.
#[test]
fn division_by_zero_traps_at_runtime() {
    assert_run_fault(
        "E11210_division_by_zero.clo",
        "error E11210: integer division by zero",
    );
}

/// The minimum signed value divided by -1 aborts with E11210.
#[test]
fn minimum_divided_by_negative_one_traps_at_runtime() {
    assert_run_fault(
        "E11210_min_neg_one.clo",
        "error E11210: integer division by zero",
    );
}

/// An index past a fixed array extent aborts with E11211.
#[test]
fn index_out_of_range_traps_at_runtime() {
    assert_run_fault(
        "E11211_index_out_of_range.clo",
        "error E11211: index out of range",
    );
}

/// An out-of-range float-to-integer conversion aborts with E11212.
#[test]
fn out_of_range_float_conversion_traps_at_runtime() {
    assert_run_fault(
        "E11212_float_to_int_range.clo",
        "error E11212: float-to-integer conversion out of range",
    );
}

/// A NaN float-to-integer conversion aborts with the same E11212 guard.
#[test]
fn nan_float_conversion_traps_at_runtime() {
    assert_run_fault(
        "E11212_nan_float_to_int.clo",
        "error E11212: float-to-integer conversion out of range",
    );
}

/// A false assertion aborts with E11213.
#[test]
fn failed_assertion_traps_at_runtime() {
    assert_run_fault(
        "E11213_assertion_failed.clo",
        "error E11213: assertion failed",
    );
}
