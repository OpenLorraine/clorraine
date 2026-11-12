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

// -- Lowering and runtime pins ---------------------------------------------

/// Emits one translation unit for a profile.
fn emitted_profile(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
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
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// IT-7: a for-step owning assignment uses the flat loop and pre-drops.
#[test]
fn it7_for_step_owning_assignment_is_flat_and_pre_drops() {
    let source = r#"module spec.tprii.it7;
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
    let c = emitted_profile("forstep", "host", source);
    assert!(
        c.contains("goto __cl_b1;"),
        "the owning step must use the flat loop:\n{c}"
    );
    // The step block is the label the loop header jumps to after the body.
    let step = c.find("__cl_b3:").expect("the step block label");
    let drop = c[step..]
        .find("if (s) { FreeString(s); }")
        .map(|offset| step + offset)
        .expect("the step has no pre-drop");
    let store = c[step..]
        .find("s = cl_string_concat(")
        .map(|offset| step + offset)
        .expect("the step has no store");
    assert!(drop < store, "the pre-drop must precede the store:\n{c}");
}

/// IT-8: a non-owning for step keeps the compact C `for` header.
#[test]
fn it8_non_owning_for_step_keeps_the_header() {
    let source = r#"module spec.tprii.it8;
import { println } from std.io;
func main(): i32 {
    let n: i32 = 0;
    for (let i: i32 = 0; i < 3; n = n + 1, i = i + 1) { }
    println($"{n}");
    return 0;
}
"#;
    let c = emitted_profile("plainstep", "host", source);
    assert!(
        c.contains("for (; (i < 3); n = (i32)((u32)n + (u32)1), i = (i32)((u32)i + (u32)1))"),
        "the non-owning step lost its compact header:\n{c}"
    );
}

/// IT-9: a for-init template stores and releases before the loop header.
#[test]
fn it9_for_init_template_emits_before_the_header() {
    let source = r#"module spec.tprii.it9;
import { println } from std.io;
import { String, stringOf } from std.text;
let seq: i32 = 0;
func tick(): String { seq = seq + 1; return stringOf("t"); }
func main(): i32 {
    let i: i32 = 0;
    for ($"{tick()}", i = 0; i < 1; i = i + 1) {
        println($"{seq}");
    }
    return 0;
}
"#;
    let c = emitted_profile("forinit", "host", source);
    let store = c
        .find("cl_string_concat(")
        .expect("the init template was not emitted");
    let release = c[store..]
        .find("FreeString(")
        .map(|offset| store + offset)
        .expect("the init template was not released");
    let header = c.find("for (; (i < 1);").expect("the loop header");
    assert!(store < header, "the init store must precede the header:\n{c}");
    assert!(release < header, "the init release must precede the header:\n{c}");
}

/// IT-10: the same-order aggregate keeps the initializer; a differing order
/// stores field by field.
#[test]
fn it10_aggregate_order_shapes() {
    let same = r#"module spec.tprii.it10a;
import { println } from std.io;
let seq: i32 = 0;
func mark(n: i32): i32 { seq = seq * 10 + n; return n; }
struct P { a: i32; b: i32; }
func main(): i32 {
    let p: P = P { a: mark(1), b: mark(2) };
    println($"{seq}");
    return 0;
}
"#;
    let c = emitted_profile("aggsame", "host", same);
    assert!(
        c.contains("p = (ClP){ cl_spec_tprii_it10a_mark(1), cl_spec_tprii_it10a_mark(2) }"),
        "the same-order aggregate lost its initializer:\n{c}"
    );

    let reversed = r#"module spec.tprii.it10b;
import { println } from std.io;
let seq: i32 = 0;
func mark(n: i32): i32 { seq = seq * 10 + n; return n; }
struct P { a: i32; b: i32; }
func main(): i32 {
    let p: P = P { b: mark(2), a: mark(1) };
    println($"{seq}");
    return 0;
}
"#;
    let c = emitted_profile("aggrev", "host", reversed);
    let second = c
        .find("p.b = cl_spec_tprii_it10b_mark(2);")
        .expect("the reversed aggregate did not store b first");
    let first = c
        .find("p.a = cl_spec_tprii_it10b_mark(1);")
        .expect("the reversed aggregate did not store a second");
    assert!(second < first, "the fieldwise store order is wrong:\n{c}");
}

/// IT-11: both shims guard the zero-length panic write and keep their stream.
#[test]
fn it11_panic_guards_in_both_shims() {
    let source = r#"module spec.tprii.it11;
func main(): i32 {
    let v: str = null;
    panic(v);
    return 0;
}
"#;
    let host = emitted_profile("panicguard", "host", source);
    let host_body = host
        .split("static inline void cl_panic")
        .nth(1)
        .expect("host cl_panic");
    assert!(host_body.contains("if (len > 0)"), "{host_body}");
    assert!(
        host_body.contains("fwrite(message, 1, len, stderr);"),
        "{host_body}"
    );

    let amiga = emitted_profile("panicguard", "hosted-amiga", source);
    let amiga_body = amiga
        .split("static inline void cl_panic")
        .nth(1)
        .expect("Amiga cl_panic");
    assert!(amiga_body.contains("if (len > 0)"), "{amiga_body}");
    assert!(
        amiga_body.contains("fwrite(message, 1, (size_t)len, stdout);"),
        "{amiga_body}"
    );
}

/// IT-12: the #26/#27 ordered String copy shape is unchanged.
#[test]
fn it12_string_placeholder_shape_is_unchanged() {
    let source = r#"module spec.tprii.it12;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("abc");
    println($"{s}");
    return 0;
}
"#;
    let c = emitted_profile("stringshape", "host", source);
    assert_eq!(c.matches("cl_string_copy(s)").count(), 1, "{c}");
    assert!(c.contains("cl_str_part_owned(__cl_tmp"), "{c}");
    assert!(!c.contains("$text"), "{c}");
}
