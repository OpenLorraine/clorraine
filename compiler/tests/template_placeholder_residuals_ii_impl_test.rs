//! Implementation tests for the second template-placeholder residual set.
//!
//! The behavior oracle is `template_placeholder_residuals_ii_spec_test.rs`.
//! These cases pin the diagnostic surface (codes, spans, single-report
//! behaviour) and the emitted one-argument `text` shape. The loop, aggregate,
//! and panic pins are appended in the lowering phase.

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
            "clc-template-residual-ii-impl-{}-{tag}-{sequence}",
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

/// IT-1: `text(null)` reports E10500 with the frozen surface and writes no C.
#[test]
fn it1_text_null_diagnostic_surface() {
    let source = r#"module spec.tprii.it1;
import { String } from std.text;
import { text } from std.fmt;
func main(): i32 {
    let t: String = text(null);
    return i32(len(t));
}
"#;
    let (stderr, emitted) = compile("textnull", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10500"), 1, "stderr:\n{stderr}");
    assert!(
        stderr.contains("type 'null' is not formattable"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("convert with `text`, `toHex`, or `toDecimal`"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("main.clo:5:26"), "stderr:\n{stderr}");
}

/// IT-2: a generic non-formattable `text` instance reports E10500 once.
#[test]
fn it2_generic_text_instance_reports_once() {
    let source = r#"module spec.tprii.it2;
import { String } from std.text;
import { text } from std.fmt;
struct Box { n: i32; }
func show<T>(x: T): String {
    return text(x);
}
func main(): i32 {
    let t: String = show<Box>(Box { n: 1 });
    return i32(len(t));
}
"#;
    let (stderr, emitted) = compile("generictext", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10500"), 1, "stderr:\n{stderr}");
    assert!(
        stderr.contains("type 'Box' is not formattable"),
        "stderr:\n{stderr}"
    );
}

/// IT-3: the valid one-argument shape is unchanged.
#[test]
fn it3_one_argument_text_shape_is_unchanged() {
    let source = r#"module spec.tprii.it3;
import { println } from std.io;
import { text } from std.fmt;
func main(): i32 {
    println(text(7));
    return 0;
}
"#;
    let (stderr, emitted) = compile("textshape", source);
    assert!(stderr.is_empty() || stderr.contains("warning"), "stderr:\n{stderr}");
    let c = emitted.expect("the valid program emitted C");
    assert!(c.contains("cl_text_i32((i32)(7))"), "{c}");
    assert!(!c.contains("cl_text_i32()"), "{c}");
}

/// IT-4: `null + 1` reports E10012 at the null and writes no C.
#[test]
fn it4_null_plus_one_span() {
    let source = r#"module spec.tprii.it4;
import { println } from std.io;
func main(): i32 {
    println($"{null + 1}");
    return 0;
}
"#;
    let (stderr, emitted) = compile("nullplus", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10012"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:4:16"), "stderr:\n{stderr}");
}

/// IT-5: the generic `String` null arm reports E10010 only.
#[test]
fn it5_generic_string_null_arm_is_e10010_only() {
    let source = r#"module spec.tprii.it5;
import { println } from std.io;
import { String, stringOf } from std.text;
func show<T>(f: bool, x: T): String {
    return $"{f ? null : x}";
}
func main(): i32 {
    println(show<String>(true, stringOf("x")));
    return 0;
}
"#;
    let (stderr, emitted) = compile("genericstring", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10010"), 1, "stderr:\n{stderr}");
    assert_eq!(count_reports(&stderr, "E10012"), 0, "stderr:\n{stderr}");
}

/// IT-6: an inferred scalar null arm reports E10012 at the null.
#[test]
fn it6_scalar_null_arm_span() {
    let source = r#"module spec.tprii.it6;
func main(): i32 {
    let flag: bool = true;
    let x = flag ? null : 5;
    return x;
}
"#;
    let (stderr, emitted) = compile("scalarnull", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10012"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:4:20"), "stderr:\n{stderr}");
}

/// IT-14: a conditional null arm in a binary expression reports once.
#[test]
fn it14_conditional_binary_null_reports_once() {
    let source = r#"module spec.tprii.it14;
func main(): i32 {
    let c: bool = true;
    let z: i32 = (c ? 1 : null) + 1;
    return z;
}
"#;
    let (stderr, emitted) = compile("condbinary", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10012"), 1, "stderr:\n{stderr}");
}

/// IT-15: `null + null` reports E10012 once at the left null.
#[test]
fn it15_both_null_reports_once() {
    let source = r#"module spec.tprii.it15;
func main(): i32 {
    let c = null + null;
    return 0;
}
"#;
    let (stderr, emitted) = compile("bothnull", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10012"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:3:13"), "stderr:\n{stderr}");
}
