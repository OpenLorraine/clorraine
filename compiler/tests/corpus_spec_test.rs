//! Corpus end-to-end specification tests.
//!
//! The validation corpus is the MVP's acceptance target. Each program is
//! compiled with the built `clc`, the emitted C is built with the host C
//! compiler under the fixed warning flags, and the program runs in a private
//! working directory. The three portable programs must compile with zero
//! diagnostics and print their recorded output byte for byte. The
//! interrupt-counter needs the AmigaOS scheduler and interrupt dispatcher, the
//! graphics program needs the AmigaOS screen and graphics libraries, and the
//! volatile-mmio program needs the per-chip MMIO wrapper; the host profile
//! provides none of them, so under `--profile host` each must be rejected with
//! the profile-surface diagnostic `E11200` instead of being compiled.

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
            "clc-corpus-{}-{tag}-{sequence}",
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

/// The repository's corpus directory.
fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/tests/corpus")
}

/// The path of one corpus program.
fn program_path(name: &str) -> PathBuf {
    corpus_dir().join(name).join("program.clo")
}

/// The recorded stdout of one corpus program.
fn expected_output(name: &str) -> String {
    let path = corpus_dir().join(name).join("expected-output.txt");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

/// Runs the built compiler on one corpus program for the host profile.
fn compile_host(name: &str, c_path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            &c_path.display().to_string(),
            &program_path(name).display().to_string(),
        ])
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Compiles one portable corpus program, builds the emitted C, runs it, and
/// asserts the recorded output and a clean exit.
///
/// The program runs inside a private scratch directory, so any file it
/// creates (the file-copier round-trip) never touches the repository tree.
fn run_host_program(name: &str) {
    let scratch = ScratchDir::new(name);
    let c_path = scratch.path().join("out.c");
    let compile = compile_host(name, &c_path);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{name}: the host compile must succeed with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(
        compile.stderr.is_empty(),
        "{name}: the corpus must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
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
            &program.display().to_string(),
            &c_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{name}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let output = Command::new(&program)
        .current_dir(scratch.path())
        .output()
        .unwrap_or_else(|err| panic!("{name}: cannot run the program: {err}"));
    assert_eq!(
        output.status.code(),
        Some(0),
        "{name}: expected a clean exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected_output(name),
        "{name}: the program output differs from its recorded output"
    );
}

/// `file-copier` round-trips a payload through the file and buffer wrappers.
#[test]
fn file_copier_matches_its_recorded_output() {
    run_host_program("file-copier");
}

/// `string-parser` classifies and sums the tokens of a fixed input line.
#[test]
fn string_parser_matches_its_recorded_output() {
    run_host_program("string-parser");
}

/// `buffer-container` builds and walks an `Array<i32>` and fills a buffer.
#[test]
fn buffer_container_matches_its_recorded_output() {
    run_host_program("buffer-container");
}

/// `interrupt-counter` is hosted-amiga only, so the host profile rejects it.
///
/// The program imports the AmigaOS scheduler and interrupt dispatcher, which
/// the host profile does not provide; the compiler must report `E11200` as its
/// first diagnostic and exit 1 rather than emit C that cannot link.
#[test]
fn interrupt_counter_is_rejected_on_the_host_profile() {
    let scratch = ScratchDir::new("interrupt-counter");
    let c_path = scratch.path().join("out.c");
    let compile = compile_host("interrupt-counter", &c_path);
    assert_eq!(
        compile.status.code(),
        Some(1),
        "interrupt-counter: the host profile must reject the program with exit 1; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let stderr = String::from_utf8_lossy(&compile.stderr);
    let first_line = stderr.lines().next().unwrap_or_default();
    assert!(
        first_line.starts_with("error E11200:"),
        "interrupt-counter: expected an E11200 first line, got:\n{stderr}"
    );
    assert!(
        first_line.contains("is not available on profile 'host'"),
        "interrupt-counter: the diagnostic must name the host profile, got:\n{stderr}"
    );
}

/// `graphics` is hosted-amiga only, so the host profile rejects it.
///
/// The program imports the AmigaOS screen and graphics libraries, which the
/// host profile does not provide; the compiler must report `E11200` as its first
/// diagnostic and exit 1 rather than emit C that cannot link.
#[test]
fn graphics_is_rejected_on_the_host_profile() {
    let scratch = ScratchDir::new("graphics");
    let c_path = scratch.path().join("out.c");
    let compile = compile_host("graphics", &c_path);
    assert_eq!(
        compile.status.code(),
        Some(1),
        "graphics: the host profile must reject the program with exit 1; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let stderr = String::from_utf8_lossy(&compile.stderr);
    let first_line = stderr.lines().next().unwrap_or_default();
    assert!(
        first_line.starts_with("error E11200:"),
        "graphics: expected an E11200 first line, got:\n{stderr}"
    );
    assert!(
        first_line.contains("is not available on profile 'host'"),
        "graphics: the diagnostic must name the host profile, got:\n{stderr}"
    );
}

/// `volatile-mmio` is hosted-amiga only, so the host profile rejects it.
///
/// The program imports the per-chip MMIO wrapper `std.amiga.custom`, which the
/// host profile does not provide; the compiler must report `E11200` as its first
/// diagnostic and exit 1 rather than emit C that cannot link.
#[test]
fn volatile_mmio_is_rejected_on_the_host_profile() {
    let scratch = ScratchDir::new("volatile-mmio");
    let c_path = scratch.path().join("out.c");
    let compile = compile_host("volatile-mmio", &c_path);
    assert_eq!(
        compile.status.code(),
        Some(1),
        "volatile-mmio: the host profile must reject the program with exit 1; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let stderr = String::from_utf8_lossy(&compile.stderr);
    let first_line = stderr.lines().next().unwrap_or_default();
    assert!(
        first_line.starts_with("error E11200:"),
        "volatile-mmio: expected an E11200 first line, got:\n{stderr}"
    );
    assert!(
        first_line.contains("is not available on profile 'host'"),
        "volatile-mmio: the diagnostic must name the host profile, got:\n{stderr}"
    );
}
