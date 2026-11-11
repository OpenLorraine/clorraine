//! Implementation tests for template placeholder integrity.
//!
//! The cases pin the diagnostic surface and the emitted shape: a nullable
//! placeholder reports once at its own span with no output C, and a generic
//! placeholder's instance duty reports E10010/E10500 while a valid narrowed
//! instance emits the copy. The behavior oracle is the specification suite.

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
            "clc-template-integrity-impl-{}-{tag}-{sequence}",
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

/// A nullable concrete placeholder reports once at its own span and writes no C.
#[test]
fn it11_nullable_placeholder_reports_once_and_writes_no_c() {
    let source = r#"module spec.tpii.nullstr;
import { println } from std.io;
func main(): i32 {
    let s: String? = null;
    println($"{s}");
    return 0;
}
"#;
    let (stderr, emitted) = compile("nullstr", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10010"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("'s'"), "stderr:\n{stderr}");
    // The report points at the placeholder line, not the whole statement.
    assert!(stderr.contains("main.clo:5:16"), "stderr:\n{stderr}");
}

/// A nullable generic instance reports at the placeholder span.
#[test]
fn it12_nullable_generic_instance_reports_at_the_placeholder() {
    let source = r#"module spec.tpii.genericnull;
import { println } from std.io;
import { String } from std.text;
func show<T>(x: T): String { return $"{x}"; }
func main(): i32 {
    let s: String? = null;
    println(show<String?>(s));
    return 0;
}
"#;
    let (stderr, emitted) = compile("genericnull", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10010"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("'x'"), "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:4:40"), "stderr:\n{stderr}");
}

/// A non-formattable generic instance reports E10500.
#[test]
fn it12_non_formattable_generic_instance_reports_e10500() {
    let source = r#"module spec.tpii.genericstruct;
import { println } from std.io;
import { String } from std.text;
struct H { x: i32; }
func show<T>(x: T): String { return $"{x}"; }
func main(): i32 {
    let h: H = H { x: 1 };
    println(show<H>(h));
    return 0;
}
"#;
    let (stderr, emitted) = compile("genericstruct", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10500"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("'H'"), "stderr:\n{stderr}");
}

/// A narrowed generic instance emits the copy with no diagnostic.
#[test]
fn it12_narrowed_generic_instance_emits_the_copy() {
    let source = r#"module spec.tpii.genericnarrow;
import { println } from std.io;
import { String, stringOf } from std.text;
func show<T>(x: T): String {
    if (x != null) { return $"{x}"; }
    return stringOf("null");
}
func main(): i32 {
    println(show<String?>(stringOf("alpha")));
    return 0;
}
"#;
    let (stderr, emitted) = compile("genericnarrow", source);
    assert!(
        !stderr.contains("error "),
        "a valid narrowed instance was rejected:\n{stderr}"
    );
    let emitted = emitted.expect("the valid instance wrote C");
    assert!(
        emitted.contains("cl_string_copy"),
        "the narrowed String placeholder did not emit a copy"
    );
}
