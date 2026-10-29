//! Emission pins for the zero-defect null and type-rule batch.
//!
//! The specification tests in `def_zero_null_spec_test.rs` state what the
//! diagnostics and runs must do. These cases pin the emitted shapes that carry
//! those rules: the runtime cstring length call, the inferred nullable generic
//! instance, and the narrowed short-circuit and loop-head C conditions.

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
            "clc-zeronullimpl-{}-{tag}-{sequence}",
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

/// Emits one accepted host program and returns the translation unit.
fn emit_host(tag: &str, source: &str) -> String {
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
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Emits, strictly builds, and runs one accepted host program.
fn run_strict_host(tag: &str, source: &str) -> Output {
    let emitted = emit_host(tag, source);
    let scratch = ScratchDir::new(&format!("{tag}-run"));
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
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
        .expect("failed to spawn the built program")
}

/// A cstring length reads the runtime terminator, and an inferred null
/// argument keys the nullable generic instance.
#[test]
fn should_emit_the_runtime_cstring_length_and_the_nullable_instance() {
    let cstring = r#"module impl.zn.cstringlen;

import { toCString } from std.text;

func main(): i32 {
    let c: cstring = toCString("abcd");
    if (c == null) {
        return 1;
    }
    return i32(len(c)) - 4;
}
"#;
    let emitted = emit_host("cstringlen", cstring);
    assert!(
        emitted.contains("cl_cstring_len(c)"),
        "cstringlen: the length must call the runtime helper; emitted C:\n{emitted}"
    );

    let pair = r#"module impl.zn.nullablepair;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let s: str? = "ab";
    let a: i32 = pair(s, null);
    let b: i32 = pair<str?>(s, null);
    return a + b;
}
"#;
    let emitted = emit_host("nullablepair", pair);
    assert!(
        emitted.matches("cl_pair__strQ(").count() >= 3,
        "nullablepair: the declaration and calls must use the nullable instance; emitted C:\n{emitted}"
    );
    assert!(
        emitted.contains("cl_pair__strQ(s, cl_zero_ClStr())"),
        "nullablepair: the null argument must reach the nullable instance; emitted C:\n{emitted}"
    );
    assert!(
        emitted.contains("cl_zero_ClStr()"),
        "nullablepair: the null argument must lower to the view's empty value; emitted C:\n{emitted}"
    );
}

/// The short-circuit right operand and the loop head carry the narrowed C
/// condition, and the accepted program runs.
#[test]
fn should_emit_the_narrowed_short_circuit_and_loop_head() {
    let source = r#"module impl.zn.narrowchain;

func count(s: str?): i32 {
    let n: i32 = 0;
    if (s != null && len(s) > 0) {
        n = n + 1;
    }
    if (s == null || len(s) > 0) {
        n = n + 1;
    }
    while (s != null && n < 2) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let t: str = "ab";
    return count(t) - 2;
}
"#;
    let emitted = emit_host("narrowchain", source);
    assert!(
        emitted.contains("(s).ptr != NULL"),
        "narrowchain: the loop head must test the view pointer; emitted C:\n{emitted}"
    );
    let output = run_strict_host("narrowchain", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "narrowchain: the narrowed chain must run; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
