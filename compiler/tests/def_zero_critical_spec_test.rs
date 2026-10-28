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

/// A later argument whose literal has no C value expression is filled at its
/// own position, after the effects of every earlier argument.
#[test]
fn should_evaluate_earlier_arguments_before_a_later_literal_fill() {
    let source = r#"module spec.zc.order552;

let order: i32 = 0;

struct Outer { xs: i32[2]; n: i32; }

func a(): i32 { order = order * 10 + 1; return 1; }
func mk(): i32 { order = order * 10 + 2; return 7; }
func useTwo(x: i32, o: Outer): i32 { order = order * 10 + 3; return x + o.n; }

func main(): i32 {
    let r: i32 = useTwo(a(), Outer { xs: [mk(), 8], n: 2 });
    return order - 123 + r - 3;
}
"#;
    let emitted = emit_host("order552", source);
    let output = run_strict_host("order552", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "order552: the arguments must run in source order (got order {}); stderr:\n{}",
        0,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A fill expression evaluates once, not once per covered element, including
/// a nested-row fill that evaluates once per row.
#[test]
fn should_evaluate_an_array_fill_once() {
    let statement = r#"module spec.zc.fill551;

let calls: i32 = 0;

func mk(): i32 {
    calls = calls + 1;
    return 1;
}

func main(): i32 {
    let xs: i32[3] = [; mk()];
    return xs[0] + calls * 10;
}
"#;
    let emitted = emit_host("fill551", statement);
    let output = run_strict_host("fill551", &emitted);
    assert_eq!(
        output.status.code(),
        Some(11),
        "fill551: the statement fill must run once; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let nested = r#"module spec.zc.fill551b;

let calls: i32 = 0;

func mk(): i32 {
    calls = calls + 1;
    return 1;
}

func main(): i32 {
    let m: i32[2][3] = [; [; mk()]];
    return m[0][0] + calls * 10;
}
"#;
    let emitted = emit_host("fill551b", nested);
    let output = run_strict_host("fill551b", &emitted);
    assert_eq!(
        output.status.code(),
        Some(21),
        "fill551b: the nested-row fill must run once per row; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts the front end rejected `source` with the given code and count.
fn assert_rejected(tag: &str, source: &str, code: &str, expected: usize, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        expected,
        "{tag}: expected {expected} error(s); stderr:\n{stderr}"
    );
    assert!(
        errors[0].contains(code),
        "{tag}: expected the {code} report; saw `{}`; stderr:\n{stderr}",
        errors[0]
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

/// A plain (borrow) parameter selected by a conditional may not be moved into
/// an owned raw-deref destination; the store reports the lend rule once.
#[test]
fn should_reject_a_borrowed_parameter_moved_into_a_raw_destination() {
    let source = r#"module spec.zc.store544;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(c: bool, p: String, q: String): i32 {
    let b: Box = Box { v: stringOf("zz") };
    let h: raw Box = unsafe &b;
    unsafe { (*h).v = c ? p : q; }
    return i32(len(b.v)) - 2;
}

func main(): i32 {
    return pick(true, stringOf("ab"), stringOf("cd"));
}
"#;
    assert_rejected("store544", source, "E10034", 1, &["p"]);
}

/// Builds and runs one emitted host translation unit under the strict set.
fn run_strict_host(tag: &str, emitted: &str) -> Output {
    let scratch = ScratchDir::new(&format!("{tag}-run"));
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
    Command::new(&program)
        .output()
        .expect("failed to run the host program")
}

/// A runtime-indexed store of a struct literal with a fixed-array call field
/// builds the literal in a temporary so the call receives its hidden
/// destination, then stores and empties the temporary.
#[test]
fn should_store_a_runtime_indexed_literal_with_a_fixed_array_call_field() {
    let source = r#"module spec.zc.store548;

struct S { xs: i32[2]; }

func mkArr(): i32[2] { return [7, 8]; }

func main(): i32 {
    let i: usize = 1;
    let arr: S[2] = [S { xs: [1, 2] }, S { xs: [3, 4] }];
    arr[i] = S { xs: mkArr() };
    return arr[1].xs[0] - 7;
}
"#;
    let emitted = emit_host("store548", source);
    let output = run_strict_host("store548", &emitted);
    assert_eq!(
        output.status.code(),
        Some(0),
        "store548: the stored literal must hold the call result; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
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
