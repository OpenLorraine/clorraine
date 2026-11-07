//! Corpus-wide UndefinedBehaviorSanitizer specification test.
//!
//! The test is a no-op unless `RDC_UBSAN` is set, so the ordinary workspace
//! run stays fast. The UBSan runner sets it and this test builds the three
//! portable corpus programs with `-fsanitize=undefined
//! -fno-sanitize-recover=all`, runs each in a private working directory, and
//! asserts that no program reports undefined behavior.

// The discovery marker the UBSan runner greps for.
const _UBSAN_SUITE: &str = "RDC_UBSAN";

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-ubsan-corpus-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The compiler under test; `RDC_CLC` lets a recorded binary run the same test.
fn clc_path() -> String {
    std::env::var("RDC_CLC").unwrap_or_else(|_| env!("CARGO_BIN_EXE_clc").to_owned())
}

fn run_clc(args: &[String]) -> Output {
    Command::new(clc_path())
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// The repository root, one level above the compiler crate.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the compiler crate has a repository root")
        .to_path_buf()
}

/// Builds one corpus program with UBSan and runs it in a private directory.
fn run_corpus_program(name: &str) {
    let source = repository_root()
        .join("spec/tests/corpus")
        .join(name)
        .join("program.clo");
    let scratch = ScratchDir::new(name);
    let c_path = scratch.path().join("program.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "ubsan-corpus/{name}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let program = scratch.path().join("program");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsanitize=undefined",
            "-fno-sanitize-recover=all",
            "-no-pie",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "ubsan-corpus/{name}: the C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let run = Command::new(&program)
        .current_dir(scratch.path())
        .env("UBSAN_OPTIONS", "print_stacktrace=0")
        .output()
        .expect("failed to run the corpus program");
    assert!(
        run.status.success(),
        "ubsan-corpus/{name}: the program exited {}; stderr:\n{}",
        run.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&run.stderr)
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        !stderr.contains("runtime error:"),
        "ubsan-corpus/{name}: UBSan reported undefined behavior; stderr:\n{stderr}"
    );
}

#[test]
fn should_run_the_portable_corpus_under_ubsan() {
    if std::env::var_os("RDC_UBSAN").is_none() {
        return;
    }
    for name in ["buffer-container", "file-copier", "string-parser"] {
        run_corpus_program(name);
    }
}
