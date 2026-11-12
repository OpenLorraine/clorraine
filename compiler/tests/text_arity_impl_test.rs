//! Implementation tests for the one-argument `text` intrinsic.
//!
//! The behavior oracle is `text_arity_spec_test.rs`; these cases pin the
//! diagnostic surface and the emitted one-argument shape so a future change
//! cannot silently restore the zero-argument fallback or drop the arity rule.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-text-arity-impl-{}-{tag}-{sequence}",
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

/// The compiler under test; a relative `RDC_CLC` resolves against the
/// workspace root because Cargo runs integration tests from `compiler/`.
fn clc_path() -> String {
    match std::env::var("RDC_CLC") {
        Ok(value) => {
            let path = PathBuf::from(&value);
            if path.is_absolute() {
                value
            } else {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("the workspace root")
                    .join(path)
                    .display()
                    .to_string()
            }
        }
        Err(_) => env!("CARGO_BIN_EXE_clc").to_owned(),
    }
}

fn run_clc(args: &[String]) -> Output {
    Command::new(clc_path())
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Compiles one source and returns the diagnostics and the optional C text.
fn compile(tag: &str, source: &str) -> (String, Option<String>) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let emitted = std::fs::read_to_string(&output_path).ok();
    (stderr, emitted)
}

fn count_reports(stderr: &str, code: &str) -> usize {
    stderr.matches(&format!("error {code}:")).count()
}

/// TIT-1: `text()` reports E10308 once with the frozen message/help.
#[test]
fn tit1_zero_argument_diagnostic_surface() {
    let source = r#"module spec.tai.zero;
import { String } from std.text;
import { text } from std.fmt;
func main(): i32 {
    let s: String = text();
    return i32(len(s));
}
"#;
    let (stderr, emitted) = compile("zero", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10308"), 1, "stderr:\n{stderr}");
    assert!(
        stderr.contains("wrong number of arguments for 'text': expected 1, found 0"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("pass one argument per parameter"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("main.clo:5:21"), "stderr:\n{stderr}");
}

/// TIT-2: a two-argument call reports arity once and nothing else.
#[test]
fn tit2_two_argument_is_the_only_report() {
    let source = r#"module spec.tai.two;
import { println } from std.io;
import { text } from std.fmt;
func side(): i32 {
    println("side");
    return 1;
}
func main(): i32 {
    println(text(1, side()));
    return 0;
}
"#;
    let (stderr, emitted) = compile("two", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10308"), 1, "stderr:\n{stderr}");
    assert_eq!(
        stderr.matches("error E").count(),
        1,
        "the arity rule is the only error; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("wrong number of arguments for 'text': expected 1, found 2"),
        "stderr:\n{stderr}"
    );
}

/// TIT-3: the valid one-argument shape is unchanged and has no zero-arg call.
#[test]
fn tit3_one_argument_emission_is_unchanged() {
    let source = r#"module spec.tai.one;
import { println } from std.io;
import { text } from std.fmt;
func main(): i32 {
    println(text(7));
    return 0;
}
"#;
    let (stderr, emitted) = compile("one", source);
    assert!(stderr.contains("warning") || stderr.is_empty(), "stderr:\n{stderr}");
    let c = emitted.expect("the valid program emitted C");
    assert!(c.contains("cl_text_i32((i32)(7))"), "{c}");
    assert!(
        !c.contains("cl_text_i32()"),
        "the zero-argument fallback survived:\n{c}"
    );
}
