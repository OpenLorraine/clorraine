//! Host specification tests for template placeholder residuals.
//!
//! The cases pin the null-placeholder rejection (E10500), the conditional
//! nullability rule (E10010), and the null-arm settlement at a handle sibling
//! (also E10010). The oracle is the frozen specification: Ch 04 §3 (source
//! order and conditional evaluation), Ch 02 TY-6 (nullable use), Ch 07 STR-7
//! (zero value), and Ch 03 O7 (zero = empty).
//!
//! Each program is emitted by the test `clc`, built with the host C compiler
//! under `-Wall -Wextra -Werror`, and run. A sanitizer leg adds ASan/UBSan
//! flags; the leak helper builds with ASan in the plain run as well.

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
            "clc-template-residual-{}-{tag}-{sequence}",
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
fn build_host(tag: &str, scratch: &ScratchDir, emitted: &str, address_sanitizer: bool) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut command = Command::new("cc");
    command.args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"]);
    if address_sanitizer || std::env::var("RDC_ASAN").is_ok() {
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

fn run_program(program: &Path) -> Output {
    let mut command = Command::new(program);
    command.env("ASAN_OPTIONS", "detect_leaks=1:symbolize=0");
    if std::env::var("RDC_UBSAN").is_ok() {
        command.env("UBSAN_OPTIONS", "halt_on_error=1");
    }
    command.output().expect("run the program")
}

/// Emits, builds, and runs one program.
fn run(tag: &str, source: &str) -> Output {
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted, false);
    run_program(&program)
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

// -- Null literal placeholder ----------------------------------------------

/// ST-1: a bare null placeholder reports E10500 and writes no C.
#[test]
fn st1_null_placeholder_is_rejected() {
    let source = r#"module spec.tpr.nullplaceholder;
import { println } from std.io;
func main(): i32 {
    println($"{null}");
    return 0;
}
"#;
    assert_rejected("null", source, "E10500", &["type 'null' is not formattable"]);
}

/// ST-2: the module-initializer path reports the same E10500.
#[test]
fn st2_module_null_placeholder_is_rejected() {
    let source = r#"module spec.tpr.modulenull;
import { String } from std.text;
let s: str = $"{null}";
func main(): i32 {
    return 0;
}
"#;
    assert_rejected("modulenull", source, "E10500", &["type 'null' is not formattable"]);
}

/// ST-3: a null view is an empty view and prints empty.
#[test]
fn st3_null_view_prints_empty() {
    let source = r#"module spec.tpr.nullview;
import { println } from std.io;
func main(): i32 {
    let v: str = null;
    println($"[{v}]");
    return 0;
}
"#;
    assert_output("nullview", source, "[]\n");
}

/// ST-4: a null arm in a conditional with a view sibling stays an empty view.
#[test]
fn st4_null_view_conditional_prints_empty() {
    let source = r#"module spec.tpr.nullviewcond;
import { println } from std.io;
func main(): i32 {
    let flag: bool = true;
    println($"{flag ? null : "x"}");
    return 0;
}
"#;
    assert_output("nullviewcond", source, "\n");
}

/// ST-25: an uninstantiated generic body with a null placeholder is rejected.
#[test]
fn st25_generic_null_body_is_rejected_at_the_declaration() {
    let source = r#"module spec.tpr.genericnull;
import { String } from std.text;
func show<T>(x: T): String {
    return $"{null}";
}
func main(): i32 {
    return 0;
}
"#;
    assert_rejected("genericnull", source, "E10500", &["type 'null' is not formattable"]);
}

/// ST-26: a null arm with a handle sibling is a nullable String, so the
/// placeholder reports the possible-null use E10010 at the null arm.
#[test]
fn st26_null_arm_with_a_handle_sibling_is_rejected() {
    let source = r#"module spec.tpr.nullhandle;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = true;
    println($"{flag ? null : stringOf("x")}");
    return 0;
}
"#;
    assert_rejected("nullhandle", source, "E10010", &["'value'"]);
}

// -- Mixed-nullability conditional -----------------------------------------

/// ST-5: an unnarrowed nullable arm in a placeholder reports E10010.
#[test]
fn st5_mixed_conditional_placeholder_is_rejected() {
    let source = r#"module spec.tpr.mixed;
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
    assert_rejected("mixed", source, "E10010", &["'b'"]);
}

/// ST-6: the same mixed conditional in a plain store reports E10010.
#[test]
fn st6_mixed_conditional_store_is_rejected() {
    let source = r#"module spec.tpr.mixedstore;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = false;
    let a: String = stringOf("alpha");
    let b: String? = null;
    let s: String = flag ? a : b;
    println(s);
    return 0;
}
"#;
    assert_rejected("mixedstore", source, "E10010", &["'b'"]);
}

/// ST-7: a narrowed nullable arm is accepted and the then arm formats.
#[test]
fn st7_mixed_conditional_narrowed_then() {
    let source = r#"module spec.tpr.mixedthen;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = true;
    let a: String = stringOf("alpha");
    let b: String? = stringOf("beta");
    if (b != null) {
        println($"{flag ? a : b}");
    }
    return 0;
}
"#;
    assert_output("mixedthen", source, "alpha\n");
}

/// ST-8: the narrowed else arm formats.
#[test]
fn st8_mixed_conditional_narrowed_else() {
    let source = r#"module spec.tpr.mixedelse;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = false;
    let a: String = stringOf("alpha");
    let b: String? = stringOf("beta");
    if (b != null) {
        println($"{flag ? a : b}");
    }
    return 0;
}
"#;
    assert_output("mixedelse", source, "beta\n");
}

/// ST-9: a `str`/`str?` mixed conditional reports E10010 at the view arm.
#[test]
fn st9_mixed_view_conditional_is_rejected() {
    let source = r#"module spec.tpr.mixedview;
import { println } from std.io;
func main(): i32 {
    let flag: bool = true;
    let x: str = null;
    let y: str? = "v";
    println($"{flag ? x : y}");
    return 0;
}
"#;
    assert_rejected("mixedview", source, "E10010", &["'y'"]);
}

/// ST-10: the narrowed `str?` arm is accepted.
#[test]
fn st10_mixed_view_conditional_narrowed() {
    let source = r#"module spec.tpr.mixedviewnarrow;
import { println } from std.io;
func main(): i32 {
    let flag: bool = false;
    let x: str = null;
    let y: str? = "v";
    if (y != null) {
        println($"{flag ? x : y}");
    }
    return 0;
}
"#;
    assert_output("mixedviewnarrow", source, "v\n");
}

/// ST-24: the #26 generic narrowed conditional stays accepted.
#[test]
fn st24_generic_narrowed_conditional_formats() {
    let source = r#"module spec.tpr.genericcond;
import { println } from std.io;
import { String, stringOf } from std.text;
func pick<T>(flag: bool, x: T, y: T): String {
    if (x != null && y != null) {
        return $"{flag ? x : y}";
    }
    return stringOf("null");
}
func main(): i32 {
    println(pick<String?>(true, stringOf("alpha"), stringOf("beta")));
    return 0;
}
"#;
    assert_output("genericcond", source, "alpha\n");
}

/// ST-28: the concrete twin of #26 ST-37 still reports at the null arm.
#[test]
fn st28_null_arm_in_a_narrowed_conditional_is_rejected() {
    let source = r#"module spec.tpr.concretenullarm;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = true;
    let x: String? = stringOf("alpha");
    if (x != null) {
        println($"{flag ? x : null}");
    }
    return 0;
}
"#;
    assert_rejected("concretenullarm", source, "E10010", &["value 'value'"]);
}

/// ST-29: a flow-narrowed conditional arm stays accepted as an index and a
/// `len` operand (the direct operand checks judge the arms, not the whole).
#[test]
fn st29_narrowed_conditional_index_and_len_are_accepted() {
    let source = r#"module spec.tpr.mixedindex;
import { println } from std.io;
func main(): i32 {
    let c: bool = true;
    let a: str = "a";
    let y: str? = "y";
    if (y != null) {
        println($"{i32((c ? a : y)[0])}");
        println($"{len(c ? a : y)}");
    }
    return 0;
}
"#;
    assert_output("mixedindex", source, "97\n1\n");
}

/// ST-30: a bare cstring arm is empty-capable and must be judged.
#[test]
fn st30_cstring_conditional_arm_is_rejected() {
    let source = r#"module spec.tpr.cstringcond;
import { toCString } from std.text;
func f(c: bool, p: cstring?, q: cstring): i32 {
    if (p != null) {
        return i32(len(c ? p : q));
    }
    return 0;
}
func main(): i32 {
    return f(true, toCString("x"), toCString("y"));
}
"#;
    assert_rejected("cstringcond", source, "E10010", &["'q'"]);
}
