//! Host specification tests for the second template-placeholder residual set.
//!
//! The cases pin the explicit `text` formattability rules, the bare-null
//! binary operand rule, and the conditional null-arm settlement (concrete and
//! generic). The oracle is the frozen specification: Ch 04 §3 (conditional
//! evaluation and operators), Ch 02 TY-6 (nullable use), Ch 04 §12
//! (formattable types, E10500), and the E10218 condition rule. The loop,
//! aggregate-order, and panic cases are appended to this suite in the
//! lowering phase.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-template-residual-ii-{}-{tag}-{sequence}",
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

/// Compiles the emitted C into a host executable and runs it.
fn run(tag: &str, source: &str) -> Output {
    let (scratch, emitted) = emit(tag, source);
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
    let mut command = Command::new(&program);
    command.env("ASAN_OPTIONS", "detect_leaks=1:symbolize=0");
    if std::env::var("RDC_UBSAN").is_ok() {
        command.env("UBSAN_OPTIONS", "halt_on_error=1");
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

/// Runs the front end and asserts the program is rejected with `code` and
/// without any code in `absent`.
fn assert_rejected_with(
    tag: &str,
    source: &str,
    code: &str,
    absent: &[&str],
    names: &[&str],
) {
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
    assert_eq!(
        stderr.matches(&format!("error {code}:")).count(),
        1,
        "{tag}: expected one {code}; stderr:\n{stderr}"
    );
    for absent in absent {
        assert!(
            !stderr.contains(&format!("error {absent}:")),
            "{tag}: unexpected {absent}; stderr:\n{stderr}"
        );
    }
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

fn assert_rejected(tag: &str, source: &str, code: &str, names: &[&str]) {
    assert_rejected_with(tag, source, code, &[], names);
}

fn assert_no_sanitizer_report(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer") && !stderr.contains("runtime error:"),
        "{tag}: the sanitizer reported an error; stderr:\n{stderr}"
    );
}

// -- Explicit text formattability ------------------------------------------

/// ST-1: `text(null)` is E10500 and writes no C.
#[test]
fn st1_text_null_is_rejected() {
    let source = r#"module spec.tprii.textnull;
import { String } from std.text;
import { text } from std.fmt;
func main(): i32 {
    let t: String = text(null);
    return i32(len(t));
}
"#;
    assert_rejected("textnull", source, "E10500", &["type 'null' is not formattable"]);
}

/// ST-2: a concrete non-formattable argument is E10500.
#[test]
fn st2_text_struct_is_rejected() {
    let source = r#"module spec.tprii.textstruct;
import { String } from std.text;
import { text } from std.fmt;
struct Box { n: i32; }
func main(): i32 {
    let t: String = text(Box { n: 1 });
    return i32(len(t));
}
"#;
    assert_rejected("textstruct", source, "E10500", &["type 'Box' is not formattable"]);
}

/// ST-3: a generic text argument reports E10500 at the instance.
#[test]
fn st3_generic_text_struct_is_rejected() {
    let source = r#"module spec.tprii.generictext;
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
    assert_rejected("generictext", source, "E10500", &["type 'Box' is not formattable"]);
}

/// ST-4: a formattable generic text instance still runs.
#[test]
fn st4_generic_text_scalar_runs() {
    let source = r#"module spec.tprii.generictextok;
import { println } from std.io;
import { String } from std.text;
import { text } from std.fmt;
func show<T>(x: T): String {
    return text(x);
}
func main(): i32 {
    println(show<i32>(7));
    return 0;
}
"#;
    assert_output("generictextok", source, "7\n");
}

/// ST-5: an unnarrowed nullable text argument is E10010.
#[test]
fn st5_nullable_text_is_rejected() {
    let source = r#"module spec.tprii.textnullable;
import { println } from std.io;
import { text } from std.fmt;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String? = stringOf("x");
    println(text(s));
    return 0;
}
"#;
    assert_rejected("textnullable", source, "E10010", &["'s'"]);
}

/// ST-6: a narrowed nullable text argument is accepted.
#[test]
fn st6_narrowed_nullable_text_runs() {
    let source = r#"module spec.tprii.textnarrowed;
import { println } from std.io;
import { text } from std.fmt;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String? = stringOf("x");
    if (s != null) {
        println(text(s));
    }
    return 0;
}
"#;
    assert_output("textnarrowed", source, "x\n");
}

/// ST-7: an opaque projection over a parameter is E10500.
#[test]
fn st7_opaque_text_projection_is_rejected() {
    let source = r#"module spec.tprii.textopaque;
import { String } from std.text;
import { text } from std.fmt;
struct P { n: i32; }
func show<T>(x: T): String {
    return text(x.n);
}
func main(): i32 {
    let t: String = show<P>(P { n: 1 });
    return i32(len(t));
}
"#;
    assert_rejected("textopaque", source, "E10500", &["not formattable"]);
}

// -- Bare null binary operand ----------------------------------------------

/// ST-9: a bare null arithmetic operand is E10012.
#[test]
fn st9_null_plus_one_is_rejected() {
    let source = r#"module spec.tprii.nullplus;
import { println } from std.io;
func main(): i32 {
    println($"{null + 1}");
    return 0;
}
"#;
    assert_rejected("nullplus", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-10: `v == null` stays legal and narrows.
#[test]
fn st10_null_comparison_runs() {
    let source = r#"module spec.tprii.nullcmp;
import { println } from std.io;
func main(): i32 {
    let v: str = null;
    if (v == null) {
        println("null");
    }
    return 0;
}
"#;
    assert_output("nullcmp", source, "null\n");
}

/// ST-30: `null && flag` keeps the frozen E10218 condition report.
#[test]
fn st30_logical_null_stays_e10218() {
    let source = r#"module spec.tprii.nulllogical;
func main(): i32 {
    let flag: bool = true;
    let c = null && flag;
    if (c) { return 1; }
    return 0;
}
"#;
    assert_rejected("nulllogical", source, "E10218", &[]);
}

/// ST-33: both-null arithmetic is E10012.
#[test]
fn st33_null_plus_null_is_rejected() {
    let source = r#"module spec.tprii.nullboth;
func main(): i32 {
    let c = null + null;
    return 0;
}
"#;
    assert_rejected("nullboth", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-34: `null == null` stays a legal comparison.
#[test]
fn st34_null_equals_null_runs() {
    let source = r#"module spec.tprii.nulleqnull;
import { println } from std.io;
func main(): i32 {
    let c: bool = null == null;
    println($"{c}");
    return 0;
}
"#;
    assert_output("nulleqnull", source, "true\n");
}

// -- Conditional null-arm settlement ---------------------------------------

/// ST-8: a null arm at a view sibling stays the empty view.
#[test]
fn st8_null_view_conditional_prints_empty() {
    let source = r#"module spec.tprii.nullview;
import { println } from std.io;
func main(): i32 {
    let flag: bool = true;
    println($"{flag ? null : "x"}");
    return 0;
}
"#;
    assert_output("nullview", source, "\n");
}

/// ST-11: a generic null arm at `T = str` is accepted and prints the value.
#[test]
fn st11_generic_null_arm_at_str_runs() {
    let source = r#"module spec.tprii.genericnullstr;
import { println } from std.io;
import { String } from std.text;
func show<T>(f: bool, x: T): String {
    return $"{f ? null : x}";
}
func main(): i32 {
    println(show<str>(false, "x"));
    return 0;
}
"#;
    assert_output("genericnullstr", source, "x\n");
}

/// ST-12: a generic null arm at `T = String` reports E10010 only.
#[test]
fn st12_generic_null_arm_at_string_is_e10010_only() {
    let source = r#"module spec.tprii.genericnullstring;
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
    assert_rejected_with(
        "genericnullstring",
        source,
        "E10010",
        &["E10012"],
        &["'value'"],
    );
}

/// ST-13: a generic null arm at a scalar reports E10012 only.
#[test]
fn st13_generic_null_arm_scalar_is_e10012_only() {
    let source = r#"module spec.tprii.genericnullscalar;
import { String } from std.text;
func show<T>(f: bool, x: T): String {
    return $"{f ? null : x}";
}
func main(): i32 {
    let t: String = show<i32>(true, 1);
    return i32(len(t));
}
"#;
    assert_rejected_with(
        "genericnullscalar",
        source,
        "E10012",
        &["E10010"],
        &["'null' is not allowed here"],
    );
}

/// ST-14: an inferred scalar null arm is E10012.
#[test]
fn st14_inferred_scalar_null_arm_is_rejected() {
    let source = r#"module spec.tprii.scalarnull;
func main(): i32 {
    let flag: bool = true;
    let x = flag ? null : 5;
    return x;
}
"#;
    assert_rejected("scalarnull", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-15: an inferred struct null arm is E10012.
#[test]
fn st15_inferred_struct_null_arm_is_rejected() {
    let source = r#"module spec.tprii.structnull;
struct Box { n: i32; }
func main(): i32 {
    let flag: bool = true;
    let x = flag ? null : Box { n: 1 };
    return x.n;
}
"#;
    assert_rejected("structnull", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-16: an inferred handle null arm is `String?` and narrows.
#[test]
fn st16_inferred_handle_null_arm_runs() {
    let source = r#"module spec.tprii.handlenull;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = true;
    let y = flag ? stringOf("x") : null;
    if (y != null) {
        println(y);
    }
    return 0;
}
"#;
    assert_output("handlenull", source, "x\n");
}

/// ST-17: a written non-nullable destination keeps E10012.
#[test]
fn st17_written_scalar_destination_keeps_e10012() {
    let source = r#"module spec.tprii.writtenscalar;
func main(): i32 {
    let flag: bool = true;
    let y: i32 = flag ? 1 : null;
    return y;
}
"#;
    assert_rejected("writtenscalar", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-26: the concrete twin of #26 ST-37 reports E10010 at the null arm.
#[test]
fn st26_concrete_null_arm_at_nullable_handle_is_e10010() {
    let source = r#"module spec.tprii.concretenullarm;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let flag: bool = true;
    let x: String? = stringOf("a");
    if (x != null) {
        println($"{flag ? x : null}");
    }
    return 0;
}
"#;
    assert_rejected("concretenullarm", source, "E10010", &["'value'"]);
}

/// ST-27: a narrowed nullable text argument runs.
#[test]
fn st27_narrowed_text_argument_runs() {
    let source = r#"module spec.tprii.textargnarrowed;
import { println } from std.io;
import { text } from std.fmt;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String? = stringOf("x");
    if (s != null) {
        println(text(s));
    }
    return 0;
}
"#;
    assert_output("textargnarrowed", source, "x\n");
}

/// ST-28: a generic nullable text instance is E10010.
#[test]
fn st28_generic_nullable_text_is_rejected() {
    let source = r#"module spec.tprii.generictextnullable;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func show<T>(x: T): String {
    return text(x);
}
func main(): i32 {
    let t: String = show<String?>(stringOf("x"));
    return i32(len(t));
}
"#;
    assert_rejected("generictextnullable", source, "E10010", &["'x'"]);
}

/// ST-29: a narrowed generic nullable text instance runs.
#[test]
fn st29_narrowed_generic_text_runs() {
    let source = r#"module spec.tprii.generictextnarrowed;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func show<T>(x: T): String {
    if (x == null) {
        return stringOf("n");
    }
    return text(x);
}
func main(): i32 {
    println(show<String?>(stringOf("x")));
    return 0;
}
"#;
    assert_output("generictextnarrowed", source, "x\n");
}

/// ST-31: a written view destination keeps its pre-existing E10012 rule.
#[test]
fn st31_written_view_destination_keeps_e10012() {
    let source = r#"module spec.tprii.writtenview;
func main(): i32 {
    let c: bool = true;
    let v: str = "v";
    let q: str = c ? null : v;
    return i32(len(q));
}
"#;
    assert_rejected("writtenview", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-35: a placeholder over a non-formattable cstring arm is E10500.
#[test]
fn st35_non_formattable_placeholder_arm_is_e10500() {
    let source = r#"module spec.tprii.cstringarm;
import { println } from std.io;
import { toCString } from std.text;
func main(): i32 {
    let flag: bool = true;
    println($"{flag ? null : toCString("x")}");
    return 0;
}
"#;
    assert_rejected("cstringarm", source, "E10500", &["not formattable"]);
}

/// ST-36: a conditional nested in a value operand of a condition is still a
/// value use, so its null arm reports E10012.
#[test]
fn st36_nested_value_operand_condition_reports_null() {
    let source = r#"module spec.tprii.nestedcond;
func main(): i32 {
    let flag: bool = true;
    if (-(flag ? null : 5) > 0) { return 1; }
    return 0;
}
"#;
    assert_rejected("nestedcond", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-37: a module-level func value named `text` is not the intrinsic, even
/// with a nullable argument.
#[test]
fn st37_module_text_value_shadow_runs() {
    let source = r#"module spec.tprii.textshadow;
import { println } from std.io;
import { String, stringOf } from std.text;
func impl(s: String?): String {
    return stringOf("m");
}
let text: func(String?): String = impl;
func main(): i32 {
    let s: String? = stringOf("x");
    println(text(s));
    return 0;
}
"#;
    assert_output("textshadow", source, "m
");
}

/// ST-38: a binding inferred from `null` against a still-open parameter is a
/// concrete null use and reports E10012 (the base rule).
#[test]
fn st38_null_binding_against_param_is_rejected() {
    let source = r#"module spec.tprii.nullparam;
func f<T>(x: T): bool {
    let z = null;
    return z == x;
}
func main(): i32 {
    return f<i32>(1) ? 1 : 0;
}
"#;
    assert_rejected("nullparam", source, "E10012", &["'null' is not allowed here"]);
}

// -- Lowering and runtime --------------------------------------------------

/// Runs a spawned command with a bounded wait, so a wedged child cannot hang
/// the test process.
fn run_bounded(tag: &str, mut command: Command) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{tag}: spawn: {error}"));
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().expect("collect the child output"),
            Ok(None) => {}
            Err(error) => panic!("{tag}: wait: {error}"),
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{tag}: timed out after 30s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Builds with ASan and asserts a clean exit with no leak report.
fn assert_leak_free(tag: &str, source: &str, expected: &str) {
    let (scratch, emitted) = emit(tag, source);
    let c_path = scratch.path().join("asan.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("asan-prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsanitize=address",
            "-fno-omit-frame-pointer",
            "-no-pie",
        ])
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
    let mut command = Command::new(&program);
    command.env(
        "ASAN_OPTIONS",
        "detect_leaks=1:symbolize=0:handle_segv=0",
    );
    let output = run_bounded(tag, command);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected exit 0 under ASan; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "{tag}: unexpected stdout"
    );
    assert_no_sanitizer_report(tag, &output);
}

/// Builds with UBSan, runs a panic program, and asserts a clean report.
fn assert_panic_clean(tag: &str, source: &str) {
    let (scratch, emitted) = emit(tag, source);
    let c_path = scratch.path().join("ubsan.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("ubsan-prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsanitize=undefined",
            "-fno-sanitize-recover=all",
            "-no-pie",
        ])
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
    let mut command = Command::new(&program);
    command.env("UBSAN_OPTIONS", "halt_on_error=1");
    let output = run_bounded(tag, command);
    assert!(
        !output.status.success(),
        "{tag}: a panic program exited successfully"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("panic: "), "{tag}: missing panic output:\n{stderr}");
    assert!(
        !stderr.contains("runtime error:") && !stderr.contains("AddressSanitizer"),
        "{tag}: the sanitizer reported an error; stderr:\n{stderr}"
    );
}

/// ST-18: a for-step owning assignment releases the old value each iteration.
#[test]
fn st18_for_step_owning_assignment_is_leak_free() {
    let source = r#"module spec.tprii.forstep;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("");
    for (let i: i32 = 0; i < 3; s = $"{i}", i = i + 1) {
        println($"{i}");
    }
    return 0;
}
"#;
    assert_leak_free("forstep", source, "0\n1\n2\n");
}

/// ST-19: the step cleanup also runs on a `continue` edge.
#[test]
fn st19_for_step_continue_is_leak_free() {
    let source = r#"module spec.tprii.forcontinue;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("");
    for (let i: i32 = 0; i < 3; s = $"{i}", i = i + 1) {
        if (i == 1) { continue; }
        println($"{i}");
    }
    return 0;
}
"#;
    assert_leak_free("forcontinue", source, "0\n2\n");
}

/// ST-20: a template in the for-init runs once and releases its result.
#[test]
fn st20_for_init_template_runs_once() {
    let source = r#"module spec.tprii.forinittpl;
import { println } from std.io;
import { String, stringOf } from std.text;
let seq: i32 = 0;
func tick(): String {
    seq = seq + 1;
    return stringOf("t");
}
func main(): i32 {
    let i: i32 = 0;
    for ($"{tick()}", i = 0; i < 1; i = i + 1) {
        println($"{seq}");
    }
    return 0;
}
"#;
    assert_leak_free("forinittpl", source, "1\n");
}

/// ST-21: an owning call in the for-init runs once and is released.
#[test]
fn st21_for_init_owning_call_runs_once() {
    let source = r#"module spec.tprii.forinitcall;
import { println } from std.io;
import { String, stringOf } from std.text;
let seq: i32 = 0;
func mkString(): String {
    seq = seq + 1;
    return stringOf("x");
}
func main(): i32 {
    let i: i32 = 0;
    for (mkString(), i = 0; i < 1; i = i + 1) {
        println($"{seq}");
    }
    return 0;
}
"#;
    assert_leak_free("forinitcall", source, "1\n");
}

/// ST-32: an increment in the for-init now runs.
#[test]
fn st32_for_init_increment_runs() {
    let source = r#"module spec.tprii.forinitinc;
import { println } from std.io;
func main(): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    for (i++, n = 0; n < 1; n = n + 1) {
        println($"{i}");
    }
    return 0;
}
"#;
    assert_output("forinitinc", source, "1\n");
}

/// ST-22: a same-order aggregate evaluates its fields in source order.
#[test]
fn st22_same_order_aggregate_evaluates_in_order() {
    let source = r#"module spec.tprii.aggorder;
import { println } from std.io;
let seq: i32 = 0;
func mark(n: i32): i32 {
    seq = seq * 10 + n;
    return n;
}
struct P { a: i32; b: i32; }
func main(): i32 {
    let p: P = P { a: mark(1), b: mark(2) };
    println($"{seq}");
    return 0;
}
"#;
    assert_output("aggorder", source, "12\n");
}

/// ST-23: a differing order uses the fieldwise store and runs in source order.
#[test]
fn st23_differing_order_aggregate_uses_stores() {
    let source = r#"module spec.tprii.aggorderrev;
import { println } from std.io;
let seq: i32 = 0;
func mark(n: i32): i32 {
    seq = seq * 10 + n;
    return n;
}
struct P { a: i32; b: i32; }
func main(): i32 {
    let p: P = P { b: mark(2), a: mark(1) };
    println($"{seq}");
    return 0;
}
"#;
    assert_output("aggorderrev", source, "21\n");
}

/// ST-24: a null-view panic is clean under UBSan.
#[test]
fn st24_null_view_panic_is_clean() {
    let source = r#"module spec.tprii.panicnull;
func main(): i32 {
    let v: str = null;
    panic(v);
    return 0;
}
"#;
    assert_panic_clean("panicnull", source);
}

/// ST-25: a zero-state String panic is clean under UBSan.
#[test]
fn st25_zero_state_string_panic_is_clean() {
    let source = r#"module spec.tprii.paniczero;
import { String } from std.text;
struct H { name: String; }
func main(): i32 {
    let h: H = H {};
    let v: str = h.name;
    panic(v);
    return 0;
}
"#;
    assert_panic_clean("paniczero", source);
}

/// ST-39: an aggregate literal assigned in a for step keeps source order.
#[test]
fn st39_for_step_aggregate_order() {
    let source = r#"module spec.tprii.foragg;
import { println } from std.io;
let seq: i32 = 0;
func mark(n: i32): i32 {
    seq = seq * 10 + n;
    return n;
}
struct P { a: i32; b: i32; }
func main(): i32 {
    let p: P = P { a: 0, b: 0 };
    for (let i: i32 = 0; i < 1; p = P { b: mark(2), a: mark(1) }, i = i + 1) { }
    println($"{seq}");
    return 0;
}
"#;
    assert_output("foragg", source, "21\n");
}
