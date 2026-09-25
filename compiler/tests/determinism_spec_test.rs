//! Determinism specification test for the emitted translation unit.
//!
//! Compiling the same source with the same profile and flags twice must
//! produce byte-identical C. The case copies the string-parser corpus program
//! into one scratch directory and compiles it through the same relative path
//! both times, so the `#line` paths and the symbol, temporary, and drop
//! numbering are the only content under comparison. The output is checked for
//! non-empty text first, because two empty files would otherwise compare equal
//! while the emitter is absent.

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
            "clc-determinism-{}-{tag}-{sequence}",
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

/// Compiles `relative_path` with the scratch directory as the working
/// directory and returns the emitted C text.
///
/// Using the working directory keeps the command-line path relative, so both
/// runs name the same logical source and the emitted `#line` directives stay
/// comparable.
fn compile_relative(scratch: &ScratchDir, relative_path: &str, output_name: &str) -> String {
    let output_path = scratch.path().join(output_name);
    let run: Output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .current_dir(scratch.path())
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            relative_path.to_owned(),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(
        run.status.code(),
        Some(0),
        "the corpus program must compile; stderr:\n{stderr}"
    );
    std::fs::read_to_string(&output_path).unwrap_or_default()
}

/// The string-parser corpus program compiles to the same C bytes every time.
#[test]
fn compiling_the_same_source_twice_is_byte_identical() {
    let scratch = ScratchDir::new("string-parser");
    let program =
        std::fs::read_to_string(spec_root().join("tests/corpus/string-parser/program.clo"))
            .expect("read the corpus program");
    std::fs::write(scratch.path().join("program.clo"), program)
        .expect("write the corpus copy");

    let first = compile_relative(&scratch, "program.clo", "first.c");
    let second = compile_relative(&scratch, "program.clo", "second.c");

    assert!(
        !first.trim().is_empty(),
        "the emitted C is empty, so the emitter is not implemented yet"
    );
    assert!(
        first.contains("int main(void)"),
        "the generated entry stub is present:\n{first}"
    );
    assert!(
        first.contains("#line"),
        "the source mapping is present:\n{first}"
    );
    assert_eq!(
        first, second,
        "the same source, profile, and flags must emit byte-identical C"
    );
}

/// A sample that exercises generics, lambdas, and volatile together compiles
/// to the same C bytes every time.
#[test]
fn a_mixed_language_sample_rebuilds_byte_identical() {
    let scratch = ScratchDir::new("mixed");
    let source = r#"module tests.determinism.mixed;

struct Pair<T> {
    first: T;
    second: T;
}

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func demo(): i32 {
    let p = Pair<i32> { first: 1, second: 2 };
    let base: i32 = 10;
    let r = apply(1, [base] (x: i32) => x + base);
    let value: u8 = 0;
    unsafe {
        let q: raw u8 = &value;
        writeVolatile8(q, 7);
        return r + p.first + i32(readVolatile8(q));
    }
}

func main(): i32 {
    return demo();
}
"#;
    std::fs::write(scratch.path().join("mixed.clo"), source)
        .expect("write the mixed sample");

    let first = compile_relative(&scratch, "mixed.clo", "first.c");
    let second = compile_relative(&scratch, "mixed.clo", "second.c");

    assert!(
        first.contains("ClPair__i32"),
        "the sample must exercise a generic struct:\n{first}"
    );
    assert!(
        first.contains("cl_lambda_0"),
        "the sample must exercise a lambda:\n{first}"
    );
    assert!(
        first.contains("*(volatile u8 *)"),
        "the sample must exercise a volatile access:\n{first}"
    );
    assert_eq!(
        first, second,
        "the same source, profile, and flags must emit byte-identical C"
    );
}
