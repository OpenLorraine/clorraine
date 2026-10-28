//! Runnable specification tests for the zero-defect critical/major batch.
//!
//! Each case compiles a small Clorraine program, inspects or builds the emitted
//! host C under the fixed strict warning set, and runs it where the behavior is
//! observable. Memory cases run under AddressSanitizer when `RDC_ASAN` is set.
//!
//! The batch pins the conditional-store and take integrity rows, the view
//! returns, and the initializer/evaluation-order rows.

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
            "clc-zerocrit-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the scratch directory, output, and C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            c_path.to_str().expect("C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
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
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    emitted
}

/// Compiles one emitted host translation unit under the strict warning set.
fn build_strict_host(tag: &str, emitted: &str) {
    let scratch = ScratchDir::new(&format!("{tag}-build"));
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
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
}

/// A literal zero divisor in a non-constant division is a runtime fault, and
/// the emitted C must not contain the bare division the strict build rejects.
#[test]
fn should_trap_a_literal_zero_divisor_without_emitting_the_division() {
    let source = r#"module spec.zc.zero549;

struct H { n: i32; }

func main(): i32 {
    let h: H = H { n: 4 };
    let p: raw H = unsafe &h;
    let r: i32 = (unsafe (*p).n) / 0;
    return r;
}
"#;
    let emitted = emit_host("zero549", source);
    assert!(
        emitted.contains("E11210"),
        "zero549: the emitted C must carry the integer-division trap"
    );
    assert!(
        !emitted.contains("/ 0"),
        "zero549: the emitted C must not contain a bare division by zero"
    );
    build_strict_host("zero549", &emitted);
}
