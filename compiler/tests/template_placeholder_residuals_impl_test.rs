//! Implementation tests for template placeholder residuals.
//!
//! The cases pin the diagnostic surface: the null-literal E10500 with its
//! frozen message/help, the mixed-conditional E10010 at the nullable arm, and
//! the E10010 at a null arm that settles to a nullable handle. The behavior
//! oracle is the specification suite.

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
            "clc-template-residual-impl-{}-{tag}-{sequence}",
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

/// IT-1: the null placeholder reports E10500 with the frozen surface.
#[test]
fn it1_null_placeholder_reports_e10500() {
    let source = r#"module spec.tpri.nullplaceholder;
import { println } from std.io;
func main(): i32 {
    println($"{null}");
    return 0;
}
"#;
    let (stderr, emitted) = compile("null", source);
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
    assert!(stderr.contains("main.clo:4:16"), "stderr:\n{stderr}");
}

/// IT-2: the mixed conditional reports E10010 at the nullable arm, and the
/// null arm with a handle sibling settles to `String?` and also reports E10010.
#[test]
fn it2_conditional_null_diagnostics() {
    let mixed = r#"module spec.tpri.mixed;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = false;
    let a: String = stringOf("alpha");
    let b: String? = null;
    println($"{flag ? a : b}");
    return 0;
}
"#;
    let (stderr, emitted) = compile("mixed", mixed);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10010"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("'b'"), "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:8:27"), "stderr:\n{stderr}");

    let null_arm = r#"module spec.tpri.nullhandle;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = true;
    println($"{flag ? null : stringOf("x")}");
    return 0;
}
"#;
    let (stderr, emitted) = compile("nullhandle", null_arm);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10010"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("'value'"), "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:6:23"), "stderr:\n{stderr}");
}
