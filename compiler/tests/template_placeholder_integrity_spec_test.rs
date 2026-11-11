//! Host specification tests for template placeholder integrity.
//!
//! The cases pin the nullable/formattability rules and the once-only,
//! source-order evaluation contract for template placeholders. The oracle is
//! the frozen specification: Ch 04 §3 (left-to-right), Ch 02 TY-6 (nullable
//! use), Ch 16 §1–2 (views/loans), and the E10500 formattability rule.
//!
//! Each program is emitted by the test `clc`, built with the host C compiler
//! under `-Wall -Wextra -Werror`, and run. A sanitizer leg adds ASan/UBSan
//! flags; the allocation wrapper is not used here.

const _ASAN_SUITE: &str = "RDC_ASAN";
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
            "clc-template-integrity-{}-{tag}-{sequence}",
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

/// Compiles the emitted C into a host executable.
fn build_host(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut command = Command::new("cc");
    command.args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"]);
    if std::env::var("RDC_ASAN").is_ok() {
        command.args(["-fsanitize=address", "-fno-omit-frame-pointer", "-no-pie"]);
    }
    if std::env::var("RDC_UBSAN").is_ok() {
        command.args(["-fsanitize=undefined", "-fno-sanitize-recover=all", "-no-pie"]);
    }
    command.args([
        "-o",
        program.to_str().expect("program path"),
        c_path.to_str().expect("C path"),
    ]);
    let built = command.output().expect("spawn cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Emits, builds, and runs one program.
fn run(tag: &str, source: &str) -> Output {
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted);
    let mut command = Command::new(&program);
    if std::env::var("RDC_ASAN").is_ok() {
        command.env("ASAN_OPTIONS", "detect_leaks=1:symbolize=0");
    }
    command.output().expect("run the program")
}

fn assert_output(tag: &str, source: &str, expected: &str) {
    let output = run(tag, source);
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
    assert_no_sanitizer_report(tag, &output);
}

/// Runs the front end and asserts the program is rejected with `code`.
///
/// The diagnostic must mention every string in `names`, the exit code must be
/// 1, and no output C file may be written.
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

fn assert_no_sanitizer_report(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer") && !stderr.contains("runtime error:"),
        "{tag}: the sanitizer reported an error; stderr:\n{stderr}"
    );
}

// -- Nullable placeholders --------------------------------------------------

/// An unnarrowed nullable `String?` placeholder reports E10010.
#[test]
fn st1_nullable_string_placeholder_is_rejected() {
    let source = r#"module spec.tpi.nullstr;
import { println } from std.io;
func main(): i32 {
    let s: String? = null;
    println($"{s}");
    return 0;
}
"#;
    assert_rejected("nullstr", source, "E10010", &["'s'"]);
}

/// An unnarrowed nullable `str?` placeholder reports E10010 too.
#[test]
fn st2_nullable_str_placeholder_is_rejected() {
    let source = r#"module spec.tpi.nullview;
import { println } from std.io;
func main(): i32 {
    let v: str? = null;
    println($"{v}");
    return 0;
}
"#;
    assert_rejected("nullview", source, "E10010", &["'v'"]);
}

/// A narrowed `String?` placeholder formats its value.
#[test]
fn st3_narrowed_nullable_string_formats() {
    let source = r#"module spec.tpi.narrowstr;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String? = stringOf("alpha");
    if (s != null) { println($"{s}"); }
    return 0;
}
"#;
    assert_output("narrowstr", source, "alpha\n");
}

/// A call returning `String?` reports E10010 and names the call.
#[test]
fn st4_nullable_call_placeholder_is_rejected() {
    let source = r#"module spec.tpi.nullcall;
import { println } from std.io;
import { String, stringOf } from std.text;
func get(): String? { return stringOf("ab"); }
func main(): i32 {
    println($"{get()}");
    return 0;
}
"#;
    assert_rejected("nullcall", source, "E10010", &["get()"]);
}

/// The module-initializer nullable template keeps its existing E10010.
#[test]
fn st5_module_nullable_template_still_rejected() {
    let source = r#"module spec.tpi.moduleinit;
import { String, stringOf } from std.text;
func get(): String? { return stringOf("ab"); }
let s: str = $"x{get()}";
func main(): i32 {
    return 0;
}
"#;
    assert_rejected("modulenull", source, "E10010", &["get()"]);}

/// A narrowed nullable `str?` formats.
#[test]
fn st22_narrowed_nullable_view_formats() {
    let source = r#"module spec.tpi.narrowview;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("alpha");
    let v: str? = s;
    if (v != null) { println($"{v}"); }
    return 0;
}
"#;
    assert_output("narrowview", source, "alpha\n");
}

/// An unnarrowed nullable field names the root binding.
#[test]
fn st23_nullable_field_placeholder_is_rejected() {
    let source = r#"module spec.tpi.nullfield;
import { println } from std.io;
struct H { name: String?; }
func main(): i32 {
    let h: H = H { name: null };
    println($"{h.name}");
    return 0;
}
"#;
    assert_rejected("nullfield", source, "E10010", &["'h'"]);
}

/// A narrowed nullable field formats.
#[test]
fn st24_narrowed_field_formats() {
    let source = r#"module spec.tpi.narrowfield;
import { println } from std.io;
import { stringOf } from std.text;
struct H { name: String?; }
func main(): i32 {
    let h: H = H { name: stringOf("alpha") };
    if (h.name != null) { println($"{h.name}"); }
    return 0;
}
"#;
    assert_output("narrowfield", source, "alpha\n");
}

// -- Generic duties ---------------------------------------------------------

/// A nullable generic instantiation reports E10010 at the placeholder.
#[test]
fn st26_nullable_generic_instance_is_rejected() {
    let source = r#"module spec.tpi.genericnull;
import { println } from std.io;
import { String } from std.text;
func show<T>(x: T): String { return $"{x}"; }
func main(): i32 {
    let s: String? = null;
    println(show<String?>(s));
    return 0;
}
"#;
    assert_rejected("genericnull", source, "E10010", &["'x'"]);
}

/// A nullable generic instance whose body narrowed the value is accepted.
#[test]
fn st27_narrowed_generic_instance_formats() {
    let source = r#"module spec.tpi.genericnarrow;
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
    assert_output("genericnarrow", source, "alpha\n");
}

/// A non-formattable generic instantiation reports E10500.
#[test]
fn st32_non_formattable_generic_instance_is_rejected() {
    let source = r#"module spec.tpi.genericstruct;
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
    assert_rejected("genericstruct", source, "E10500", &["H"]);
}

/// An opaque generic member placeholder is rejected at the declaration.
#[test]
fn st33_opaque_generic_member_is_rejected() {
    let source = r#"module spec.tpi.opaque;
import { String } from std.text;
func name<T>(x: T): String { return $"{x.name}"; }
func main(): i32 {
    return 0;
}
"#;
    assert_rejected("opaque", source, "E10500", &["not formattable"]);
}

/// A generic conditional with both nullable arms narrowed is accepted.
#[test]
fn st34_narrowed_generic_conditional_arms_format() {
    let source = r#"module spec.tpi.genericcond;
import { println } from std.io;
import { String, stringOf } from std.text;
func pick<T>(flag: bool, x: T, y: T): String {
    if (x != null && y != null) { return $"{flag ? x : y}"; }
    return stringOf("null");
}
func main(): i32 {
    println(pick<String?>(true, stringOf("alpha"), stringOf("beta")));
    return 0;
}
"#;
    assert_output("genericcond", source, "alpha\n");
}

/// A generic conditional with a null arm is rejected at the arm.
#[test]
fn st37_generic_null_arm_is_rejected() {
    let source = r#"module spec.tpi.genericnullarm;
import { println } from std.io;
import { String, stringOf } from std.text;
func show<T>(flag: bool, x: T): String {
    if (x != null) { return $"v={flag ? x : null}"; }
    return stringOf("null");
}
func main(): i32 {
    println(show<String?>(true, stringOf("alpha")));
    return 0;
}
"#;
    assert_rejected("genericnullarm", source, "E10010", &["'value'"]);
}

/// An element parameter resolves per instance and formats.
#[test]
fn st36_generic_element_parameter_formats() {
    let source = r#"module spec.tpi.genericelem;
import { println } from std.io;
import { String } from std.text;
func first<T>(x: T): String { return $"{x[0]}"; }
func main(): i32 {
    println(first<str>("a"));
    return 0;
}
"#;
    assert_output("genericelem", source, "97\n");
}
