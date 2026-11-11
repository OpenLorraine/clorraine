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

/// Runs a program and asserts it traps with `code`.
fn assert_trap(tag: &str, source: &str, code: &str) {
    let output = run(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a runtime trap; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(code),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
}

fn assert_no_sanitizer_report(tag: &str, output: &Output) {    let stderr = String::from_utf8_lossy(&output.stderr);
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

// -- Ordered placeholder evaluation -----------------------------------------

/// ST-6: a `str`-returning call placeholder is evaluated once.
#[test]
fn st6_str_call_placeholder_evaluates_once() {
    let source = r#"module spec.tpi.strcall;
import { println } from std.io;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "x"; }
func main(): i32 {
    println($"{mk()} calls={calls}");
    return 0;
}
"#;
    assert_output("strcall", source, "x calls=1\n");
}

/// ST-7: a String placeholder copies before a later take sibling.
#[test]
fn st7_string_placeholder_copied_before_take_sibling() {
    let source = r#"module spec.tpi.takesibling;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let s: String = stringOf("alpha");
    println($"{s}{consume(s)}");
    return 0;
}
"#;
    assert_output("takesibling", source, "alpha5\n");
}

/// ST-8: a `str` view is read before a later take sibling releases its owner.
#[test]
fn st8_view_placeholder_copied_before_move() {
    let source = r#"module spec.tpi.viewmove;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let s: String = stringOf("alpha");
    let v: str = s;
    println($"{v}{consume(s)}");
    return 0;
}
"#;
    assert_output("viewmove", source, "alpha5\n");
}

/// ST-9: the reverse order stays a move-while-borrowed rejection.
#[test]
fn st9_view_after_move_is_rejected() {
    let source = r#"module spec.tpi.viewafter;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let s: String = stringOf("alpha");
    let v: str = s;
    println($"{consume(s)}{v}");
    return 0;
}
"#;
    assert_rejected("viewafter", source, "E10022", &["'s'"]);
}

/// ST-10: a scalar before a later effect reads its pre-effect value.
#[test]
fn st10_scalar_before_effect_reads_first() {
    let source = r#"module spec.tpi.scalarfirst;
import { println } from std.io;
import { stringOf } from std.text;
let x: i32 = 1;
func bump(take s: String): i32 { x = 2; return i32(len(s)); }
func main(): i32 {
    let s: String = stringOf("alpha");
    println($"{x}{bump(s)}");
    return 0;
}
"#;
    assert_output("scalarfirst", source, "15\n");
}

/// ST-11: a scalar after an effect reads its post-effect value.
#[test]
fn st11_scalar_after_effect_reads_second() {
    let source = r#"module spec.tpi.scalarsecond;
import { println } from std.io;
import { stringOf } from std.text;
let x: i32 = 1;
func bump(take s: String): i32 { x = 2; return i32(len(s)); }
func main(): i32 {
    let s: String = stringOf("alpha");
    println($"{bump(s)}{x}");
    return 0;
}
"#;
    assert_output("scalarsecond", source, "52\n");
}

/// ST-12: two String placeholders both read the same source.
#[test]
fn st12_two_string_placeholders_format() {
    let source = r#"module spec.tpi.twostrings;
import { println } from std.io;
import { stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("alpha");
    println($"{s}{s}");
    return 0;
}
"#;
    assert_output("twostrings", source, "alphaalpha\n");
}

/// ST-13: a generic String placeholder copies before a take sibling.
#[test]
fn st13_generic_string_copied_before_take() {
    let source = r#"module spec.tpi.generictake;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func show<T>(x: T, take y: String): String { return $"{x}{consume(y)}"; }
func main(): i32 {
    let s: String = stringOf("alpha");
    println(show<String>(s, s));
    return 0;
}
"#;
    assert_output("generictake", source, "alpha5\n");
}

/// ST-14: a generic scalar placeholder still formats.
#[test]
fn st14_generic_scalar_formats() {
    let source = r#"module spec.tpi.genericscalar;
import { println } from std.io;
import { String } from std.text;
func show<T>(x: T): String { return $"{x}"; }
func main(): i32 {
    println(show<i32>(7));
    return 0;
}
"#;
    assert_output("genericscalar", source, "7\n");
}

/// ST-15: a view before a take sibling in an eager `?:` arm.
#[test]
fn st15_view_and_take_in_conditional_arm() {
    let source = r#"module spec.tpi.condarm;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let c: bool = true;
    let s: String = stringOf("alpha");
    let v: str = s;
    let t: String = c ? $"{v}{consume(s)}" : stringOf("none");
    println(t);
    return 0;
}
"#;
    assert_output("condarm", source, "alpha5\n");
}

/// ST-16: a `str` call in a short-circuit operand is evaluated once.
#[test]
fn st16_short_circuit_str_call_once() {
    let source = r#"module spec.tpi.shortcall;
import { println } from std.io;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "x"; }
func main(): i32 {
    let c: bool = true;
    let ok: bool = c && len($"v={mk()}") > 0;
    println($"{ok}");
    println($"{calls}");
    return 0;
}
"#;
    assert_output("shortcall", source, "true\n1\n");
}

/// ST-17: a view and take in a short-circuit operand is sanitizer-clean.
#[test]
fn st17_short_circuit_view_take() {
    let source = r#"module spec.tpi.shorttake;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let c: bool = true;
    let s: String = stringOf("alpha");
    let v: str = s;
    let n: i32 = c && len($"{v}{consume(s)}") > 0 ? 1 : 0;
    println($"{n}");
    return 0;
}
"#;
    assert_output("shorttake", source, "1\n");
}

/// ST-18: a `str` call in a loop condition is evaluated once per test.
#[test]
fn st18_loop_condition_str_call_once() {
    let source = r#"module spec.tpi.loopcall;
import { println } from std.io;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "x"; }
func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && len($"v={mk()}") > 0) { i = i + 1; }
    println($"{calls}");
    return 0;
}
"#;
    assert_output("loopcall", source, "2\n");
}

/// ST-19: a module-initializer `str` call is evaluated once.
#[test]
fn st19_module_str_call_once() {
    let source = r#"module spec.tpi.modulecall;
import { println } from std.io;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "x"; }
let s: str = $"v={mk()}";
func main(): i32 {
    println($"{calls}");
    return 0;
}
"#;
    assert_output("modulecall", source, "1\n");
}

/// ST-20: two module-initializer placeholders each evaluate once.
#[test]
fn st20_module_two_placeholders_once() {
    let source = r#"module spec.tpi.moduletwo;
import { println } from std.io;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "x"; }
let s: str = $"a={mk()} b={mk()}";
func main(): i32 {
    println($"{calls}");
    return 0;
}
"#;
    assert_output("moduletwo", source, "2\n");
}

/// ST-21: a placeholder-free template stays a plain str.
#[test]
fn st21_literal_only_template() {
    let source = r#"module spec.tpi.literal;
import { println } from std.io;
func main(): i32 {
    println($"literal {{ braces }}");
    return 0;
}
"#;
    assert_output("literal", source, "literal { braces }\n");
}

/// ST-25: a skipped `?:` arm evaluates no placeholder.
#[test]
fn st25_skipped_arm_has_no_effect() {
    let source = r#"module spec.tpi.skipped;
import { println } from std.io;
import { stringOf } from std.text;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "x"; }
func main(): i32 {
    let c: bool = false;
    let s: String = c ? $"v={mk()}" : stringOf("none");
    println(s);
    println($"{calls}");
    return 0;
}
"#;
    assert_output("skipped", source, "none\n0\n");
}

/// ST-28: a String-returning call in a short-circuit operand is once.
#[test]
fn st28_short_circuit_string_call_once() {
    let source = r#"module spec.tpi.shortstring;
import { println } from std.io;
import { String, stringOf } from std.text;
let calls: i32 = 0;
func mkString(): String { calls = calls + 1; return stringOf("z"); }
func main(): i32 {
    let c: bool = true;
    let ok: bool = c && len($"{mkString()}") > 0;
    println($"{ok}");
    println($"{calls}");
    return 0;
}
"#;
    assert_output("shortstring", source, "true\n1\n");
}

/// ST-29: a module-initializer String source is released and evaluated once.
#[test]
fn st29_module_string_source_once() {
    let source = r#"module spec.tpi.modulestring;
import { println } from std.io;
import { String, stringOf } from std.text;
let calls: i32 = 0;
func mkString(): String { calls = calls + 1; return stringOf("z"); }
let s: str = $"v={mkString()}";
func main(): i32 {
    println($"{calls}");
    return 0;
}
"#;
    assert_output("modulestring", source, "1\n");
}

/// ST-30: a String member projection over a call is evaluated once.
#[test]
fn st30_string_member_projection_once() {
    let source = r#"module spec.tpi.membercall;
import { println } from std.io;
import { String, stringOf } from std.text;
let calls: i32 = 0;
struct H { name: String; }
func mkH(): H { calls = calls + 1; return H { name: stringOf("alpha") }; }
func main(): i32 {
    println($"{mkH().name}");
    println($"{calls}");
    return 0;
}
"#;
    assert_output("membercall", source, "alpha\n1\n");
}

/// ST-31: a view conditional is evaluated once.
#[test]
fn st31_view_conditional_once() {
    let source = r#"module spec.tpi.viewcond;
import { println } from std.io;
import { stringOf } from std.text;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "z"; }
func main(): i32 {
    let c: bool = false;
    let s: String = stringOf("alpha");
    let v: str = s;
    println($"x={(c ? v : mk())}");
    println($"{calls}");
    return 0;
}
"#;
    assert_output("viewcond", source, "x=z\n1\n");
}

/// ST-35: a static-str member over a call is evaluated once.
#[test]
fn st35_static_member_projection_once() {
    let source = r#"module spec.tpi.staticmember;
import { println } from std.io;
let calls: i32 = 0;
struct H { name: static str; }
func get(): H { calls = calls + 1; return H { name: "alpha" }; }
func main(): i32 {
    println($"{get().name}");
    println($"{calls}");
    return 0;
}
"#;
    assert_output("staticmember", source, "alpha\n1\n");
}

/// ST-38: an out-of-range index inside a string placeholder keeps its bounds trap.
#[test]
fn st38_index_placeholder_traps() {
    let source = r#"module spec.tpi.indextrap;
import { println } from std.io;
import { Array, arrayNew, push, get } from std.mem;
func main(): i32 {
    let xs: Array<str> = arrayNew<str>();
    push(xs, "ab");
    println($"{get(xs, 0)[5]}");
    return 0;
}
"#;
    assert_trap("indextrap", source, "E11211");
}

/// ST-40: a pinned owning literal field is adopted across loop re-entry.
#[test]
fn st40_adopted_literal_field_in_a_loop() {
    let source = r#"module spec.tpi.adoptloop;
import { println } from std.io;
import { String, stringOf } from std.text;
struct P { a: String; b: String; }
func mk(n: i32): String { return stringOf($"{n}"); }
func main(): i32 {
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let p: P = P { a: mk(i), b: $"{mk(i + 10)}" };
        total = total + i32(len(p.a)) + i32(len(p.b));
        i = i + 1;
    }
    println($"{total}");
    return 0;
}
"#;
    assert_output("adoptloop", source, "9\n");
}

/// ST-39: an earlier literal field evaluates before a later template copy.
#[test]
fn st39_literal_field_order_is_preserved() {
    let source = r#"module spec.tpi.literalorder;
import { println } from std.io;
import { String, stringOf } from std.text;
let seq: i32 = 0;
struct Pair { a: i32; b: String; }
func mark(n: i32): i32 { seq = seq * 10 + n; return n; }
func txt(n: i32): String { return stringOf($"{n}"); }
func main(): i32 {
    let p: Pair = Pair { a: mark(1), b: $"{txt(mark(2))}" };
    println($"{seq}");
    return 0;
}
"#;
    assert_output("literalorder", source, "12\n");
}

/// ST-41: a pinned adopted call keeps its argument's bounds trap.
#[test]
fn st41_pinned_call_keeps_argument_guard() {
    let source = r#"module spec.tpi.pinguard;
import { println } from std.io;
import { Array, arrayNew, push, get } from std.mem;
import { String, stringOf } from std.text;
struct P { a: String; b: String; }
func mk(n: str): String { return stringOf("x"); }
func main(): i32 {
    let xs: Array<str> = arrayNew<str>();
    push(xs, "ab");
    let i: i32 = 1;
    let p: P = P { a: mk(get(xs, 0)[5]), b: $"{i}" };
    println(p.a);
    return 0;
}
"#;
    assert_trap("pinguard", source, "E11211");
}
