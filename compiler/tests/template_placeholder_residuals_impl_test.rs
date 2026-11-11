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

// -- Phase 2 emission pins --------------------------------------------------

/// Emits one translation unit for a profile and asserts success.
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

/// IT-3: the zero state uses the normal copy path, with no special helper.
#[test]
fn it3_zero_state_uses_the_normal_copy_path() {
    let source = r#"module spec.tpri.zerostate;
import { println } from std.io;
struct H { name: String; }
func main(): i32 {
    let h: H = H {};
    println($"{h.name}");
    return 0;
}
"#;
    let c = emitted_profile("zerostate", "host", source);
    assert!(
        c.contains("= cl_string_copy(h.name)"),
        "the zero state did not take the normal copy path:\n{c}"
    );
    assert!(
        c.contains("cl_str_part_owned(__cl_tmp"),
        "the part did not adopt the copy:\n{c}"
    );
    assert!(!c.contains("$text"), "the sentinel leaked:\n{c}");
}

/// IT-4: both shims carry the NULL branch in the three String accessors.
#[test]
fn it4_zero_state_accessors_are_null_safe_in_both_shims() {
    let source = r#"module spec.tpri.zerosafety;
import { println } from std.io;
struct H { name: String; }
func main(): i32 {
    let h: H = H {};
    println($"{h.name}");
    return 0;
}
"#;
    for profile in ["host", "hosted-amiga"] {
        let c = emitted_profile("zerosafety", profile, source);
        for header in [
            "void *cl_string_ptr(void *handle)",
            "cl_string_len(void *handle)",
            "void *cl_string_copy(void *handle)",
        ] {
            let start = c
                .find(header)
                .unwrap_or_else(|| panic!("{profile}: missing {header}"));
            let end = c[start..]
                .find("\n}")
                .map(|offset| start + offset)
                .unwrap_or_else(|| panic!("{profile}: unterminated {header}"));
            let body = &c[start..end];
            assert!(
                body.contains("== NULL"),
                "{profile}: {header} has no NULL branch:\n{body}"
            );
        }
    }
}

/// IT-5: a discarded template result is stored and released.
#[test]
fn it5_discarded_template_is_stored_and_released() {
    let source = r#"module spec.tpr.discard;
import { println } from std.io;
func main(): i32 {
    $"{1}";
    println("done");
    return 0;
}
"#;
    let c = emitted_profile("discard", "host", source);
    let store = c
        .find("= cl_string_concat(")
        .expect("the discarded template was not stored");
    let release = c[store..]
        .find("FreeString(__cl_tmp")
        .map(|offset| store + offset)
        .expect("the discarded template was not released");
    assert!(store < release, "the release must follow the store:\n{c}");
}

/// IT-6: a for-step template emits its release inside the step block.
#[test]
fn it6_for_step_template_releases_in_the_step() {
    let source = r#"module spec.tpr.forstep;
import { println } from std.io;
func main(): i32 {
    for (let i: i32 = 0; i < 3; $"{i}", i = i + 1) {
        println($"{i}");
    }
    return 0;
}
"#;
    let c = emitted_profile("forstep", "host", source);
    let head = c.find("goto __cl_b1;").expect("the loop head");
    let step = c[head..]
        .find("= cl_string_concat(")
        .map(|offset| head + offset)
        .expect("the step template was not stored");
    let release = c[step..]
        .find("FreeString(__cl_tmp")
        .map(|offset| step + offset)
        .expect("the step template was not released");
    assert!(step < release, "the step release must follow the store:\n{c}");
}

/// IT-7: the adopted aggregate is zeroed and stored before the later copy.
#[test]
fn it7_adopted_aggregate_is_stored_before_the_copy() {
    let string_source = r#"module spec.tpri.aggstring;
import { println } from std.io;
import { String, stringOf } from std.text;
let seq: i32 = 0;
struct Inner { a: String; }
struct P { inner: Inner; b: String; }
func mark(n: i32): i32 { seq = seq * 10 + n; return n; }
func mkString(): String { return stringOf($"{mark(1)}"); }
func main(): i32 {
    let p: P = P { inner: Inner { a: mkString() }, b: $"{mark(2)}" };
    println($"{seq}");
    println(p.b);
    return 0;
}
"#;
    let c = emitted_profile("aggstring", "host", string_source);
    let inner = c
        .find("__cl_tmp0 = (ClInner){ cl_spec_tpri_aggstring_mkString() }")
        .expect("the aggregate was not adopted before the copy");
    let outer = c
        .find("p = (ClP){ __cl_tmp0,")
        .expect("the outer literal is missing");
    assert!(inner < outer, "the aggregate must precede the copy:\n{c}");
    assert!(
        c.contains("p = (ClP){ __cl_tmp0,"),
        "the outer literal did not read the adopted aggregate:\n{c}"
    );

    let fixed_source = r#"module spec.tpri.aggarr;
import { println } from std.io;
import { String, stringOf } from std.text;
let seq: i32 = 0;
struct InnerA { a: String[2]; }
struct PA { inner: InnerA; b: String; }
func mark(n: i32): i32 { seq = seq * 10 + n; return n; }
func mkPair(): String[2] { return [stringOf($"{mark(1)}"), stringOf("x")]; }
func main(): i32 {
    let p: PA = PA { inner: InnerA { a: mkPair() }, b: $"{mark(2)}" };
    println($"{seq}");
    println(p.b);
    return 0;
}
"#;
    let c = emitted_profile("aggarr", "host", fixed_source);
    assert!(
        c.contains("cl_zero(&__cl_tmp"),
        "the fixed-array aggregate temporary was not zeroed:\n{c}"
    );
    assert!(
        c.contains("p = (ClPA){ __cl_tmp"),
        "the outer literal did not read the adopted fixed-array aggregate:\n{c}"
    );
}

/// IT-8: the aggregate shape is the same on host and hosted-amiga.
#[test]
fn it8_aggregate_shape_matches_on_both_profiles() {
    let source = r#"module spec.tpri.aggprofiles;
import { println } from std.io;
import { String, stringOf } from std.text;
let seq: i32 = 0;
struct InnerA { a: String[2]; }
struct PA { inner: InnerA; b: String; }
func mark(n: i32): i32 { seq = seq * 10 + n; return n; }
func mkPair(): String[2] { return [stringOf($"{mark(1)}"), stringOf("x")]; }
func main(): i32 {
    let p: PA = PA { inner: InnerA { a: mkPair() }, b: $"{mark(2)}" };
    println($"{seq}");
    println(p.b);
    return 0;
}
"#;
    for profile in ["host", "hosted-amiga"] {
        let c = emitted_profile("aggprofiles", profile, source);
        assert!(
            c.contains("cl_zero(&__cl_tmp") && c.contains("p = (ClPA){ __cl_tmp"),
            "{profile}: the aggregate adopt shape is missing:\n{c}"
        );
    }
}

/// IT-9: the #26 ordered String copy shape is unchanged.
#[test]
fn it9_string_placeholder_shape_is_unchanged() {
    let source = r#"module spec.tpri.stringshape;
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

/// IT-10: the literal-only and scalar template shapes are unchanged.
#[test]
fn it10_literal_and_scalar_shapes_are_unchanged() {
    let literal = r#"module spec.tpri.literalshape;
import { println } from std.io;
func main(): i32 {
    println($"x");
    return 0;
}
"#;
    let c = emitted_profile("literalshape", "host", literal);
    assert!(c.contains("cl_str(\"x\")"), "{c}");
    assert_eq!(
        c.matches("cl_string_concat((ClStrPart[]").count(),
        0,
        "a literal-only template built a concat:\n{c}"
    );

    let scalar = r#"module spec.tpri.scalarshape;
import { println } from std.io;
func main(): i32 {
    let n: i32 = 1;
    println($"n={n}");
    return 0;
}
"#;
    let c = emitted_profile("scalarshape", "host", scalar);
    let line = c
        .lines()
        .find(|line| line.contains("cl_string_concat((ClStrPart[])"))
        .expect("template line");
    assert!(!line.contains("__cl_tmp"), "{line}");
    assert!(line.contains("cl_str_part_i32("), "{line}");
}
