//! Host specification tests for the one-argument `text` intrinsic.
//!
//! The standard-library appendix specifies `text(v)`: exactly one argument.
//! The checker previously accepted any count and codegen read only the first
//! argument, so `text(1, side())` silently dropped `side()` and `text()`
//! emitted a call with no argument. The oracle is the frozen appendix
//! (`spec/appendix-stdlib.md` §`text(v)`) and the E10308 arity rule
//! (`spec/14-diagnostics.md`).
//!
//! A rejected program must exit 1, report E10308 with the frozen message and
//! help, and write no C. A valid one-argument call must build under
//! `-Wall -Wextra -Werror` and print its value.

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
            "clc-text-arity-{}-{tag}-{sequence}",
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

/// Emits one host translation unit and returns the scratch directory and C.
fn emit(tag: &str, source: &str) -> (ScratchDir, String) {
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
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&output_path).expect("read the emitted C");
    (scratch, emitted)
}

fn assert_output(tag: &str, source: &str, expected: &str) {
    let (scratch, emitted) = emit(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"])
        .args([
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("C path"),
        ])
        .output()
        .expect("spawn cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let output = Command::new(&program).output().expect("run the program");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "{tag}: unexpected stdout"
    );
}

/// Runs the front end and asserts the program is rejected with `code`.
fn assert_rejected(tag: &str, source: &str, code: &str, names: &[&str]) {
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{tag}: expected the front end to reject the program; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    for name in names {
        assert!(
            stderr.contains(name),
            "{tag}: expected the diagnostic to mention {name}; stderr:\n{stderr}"
        );
    }
    assert!(
        !output_path.exists(),
        "{tag}: a rejected program wrote output C"
    );
}

/// AT-1: a zero-argument `text()` is E10308 and writes no C.
#[test]
fn at1_zero_arguments_are_rejected() {
    let source = r#"module spec.ta.zero;
import { String } from std.text;
import { text } from std.fmt;
func main(): i32 {
    let s: String = text();
    return i32(len(s));
}
"#;
    assert_rejected(
        "zero",
        source,
        "E10308",
        &[
            "wrong number of arguments for 'text': expected 1, found 0",
            "pass one argument per parameter",
        ],
    );
}

/// AT-2: a second argument is rejected; its side effect is never run.
#[test]
fn at2_two_arguments_are_rejected_not_dropped() {
    let source = r#"module spec.ta.two;
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
    assert_rejected(
        "two",
        source,
        "E10308",
        &[
            "wrong number of arguments for 'text': expected 1, found 2",
            "pass one argument per parameter",
        ],
    );
}

/// AT-3: the one-argument integer form still runs and prints.
#[test]
fn at3_one_argument_still_formats() {
    let source = r#"module spec.ta.one;
import { println } from std.io;
import { text } from std.fmt;
func main(): i32 {
    println(text(7));
    return 0;
}
"#;
    assert_output("one", source, "7\n");
}

/// AT-4: the one-argument view form still runs and prints.
#[test]
fn at4_one_argument_view_still_formats() {
    let source = r#"module spec.ta.view;
import { println } from std.io;
import { text } from std.fmt;
func main(): i32 {
    println(text("ok"));
    return 0;
}
"#;
    assert_output("view", source, "ok\n");
}

/// AT-5: a wrong-arity call in a module initializer is rejected too.
#[test]
fn at5_module_initializer_arity_is_rejected() {
    let source = r#"module spec.ta.modinit;
import { text } from std.fmt;
let n: i32 = i32(len(text()));
func main(): i32 {
    return n;
}
"#;
    assert_rejected(
        "module",
        source,
        "E10308",
        &["wrong number of arguments for 'text': expected 1, found 0"],
    );
}

/// AT-6: a module-level func-typed value named `text` is a real value, so a
/// call on it keeps its ordinary arity and runs.
#[test]
fn at6_module_value_named_text_is_not_the_intrinsic() {
    let source = r#"module spec.ta.shadow;
import { println } from std.io;
import { String, stringOf } from std.text;
func impl2(a: i32, b: i32): String {
    return stringOf("two");
}
let text: func(i32, i32): String = impl2;
func main(): i32 {
    println(text(1, 2));
    return 0;
}
"#;
    assert_output("shadow", source, "two\n");
}
