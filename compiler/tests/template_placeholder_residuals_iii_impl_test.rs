//! Implementation tests for the third template-placeholder residual set.
//!
//! The behavior oracle is `template_placeholder_residuals_iii_spec_test.rs`.
//! These cases pin the diagnostic surface (codes, spans, single-report
//! behavior), the `$text` sentinel emission for every intrinsic spelling, the
//! shim guard, and the fixed-array step shape. The #24/#28 emission pins stay
//! in their own suites.

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
            "clc-template-residual-iii-impl-{}-{tag}-{sequence}",
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

/// Emits a program and its declaring module as one build.
fn emitted_pair(tag: &str, holder: &str, main: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|error| panic!("{tag}: write holder.clo: {error}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the pair; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

fn count_reports(stderr: &str, code: &str) -> usize {
    stderr.matches(&format!("error {code}:")).count()
}

/// Builds one emitted translation unit under the strict host flags.
fn build_strict(tag: &str, c: &str) {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("strict.c");
    std::fs::write(&c_path, c).expect("write the emitted C");
    let program = scratch.path().join("strict-prog");
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
}

/// Compiles one emitted translation unit to an object under strict flags.
fn build_object_strict(tag: &str, c: &str) {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("strict.c");
    std::fs::write(&c_path, c).expect("write the emitted C");
    let object = scratch.path().join("strict.o");
    let built = Command::new("cc")
        .args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-c"])
        .args([
            "-o",
            object.to_str().expect("object path"),
            c_path.to_str().expect("C path"),
        ])
        .output()
        .expect("spawn cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// IT-1: the alias emits the `cl_text_*` selector, not a `t(...)` call.
#[test]
fn it1_alias_emits_the_text_selector() {
    let source = r#"module spec.tpriii.it1;
import { println } from std.io;
import { text as t } from std.fmt;
func main(): i32 {
    println(t(7));
    return 0;
}
"#;
    let (stderr, emitted) = compile("alias", source);
    assert!(stderr.is_empty() || stderr.contains("warning"), "stderr:\n{stderr}");
    let c = emitted.expect("the valid program emitted C");
    assert_eq!(c.matches("cl_text_i32((i32)(7))").count(), 1, "{c}");
    assert!(!c.contains("t(7)"), "the alias survived into C:\n{c}");
    assert!(!c.contains("$text"), "the sentinel survived into C:\n{c}");
}

/// IT-2: the qualified spelling emits the same selector.
#[test]
fn it2_qualified_emits_the_text_selector() {
    let source = r#"module spec.tpriii.it2;
import { println } from std.io;
func main(): i32 {
    println(std.fmt.text(7));
    return 0;
}
"#;
    let (stderr, emitted) = compile("qualified", source);
    assert!(stderr.is_empty() || stderr.contains("warning"), "stderr:\n{stderr}");
    let c = emitted.expect("the valid program emitted C");
    assert_eq!(c.matches("cl_text_i32((i32)(7))").count(), 1, "{c}");
    assert!(!c.contains("std.fmt.text"), "the path survived into C:\n{c}");
}

/// IT-3: the alias null argument reports E10500 with the frozen help.
#[test]
fn it3_alias_null_diagnostic_surface() {
    let source = r#"module spec.tpriii.it3;
import { String } from std.text;
import { text as t } from std.fmt;
func main(): i32 {
    let s: String = t(null);
    return i32(len(s));
}
"#;
    let (stderr, emitted) = compile("aliasnull", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10500"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("type 'null' is not formattable"), "stderr:\n{stderr}");
    assert!(
        stderr.contains("convert with `text`, `toHex`, or `toDecimal`"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("main.clo:5:23"), "stderr:\n{stderr}");
}

/// IT-4: the generic condition reports E10218 at the conditional and no
/// E10012.
#[test]
fn it4_generic_condition_diagnostic_surface() {
    let source = r#"module spec.tpriii.it4;
func show<T>(c: bool, x: T): i32 {
    if (c ? null : x) { return 1; }
    return 0;
}
func main(): i32 {
    return show<i32>(false, 1);
}
"#;
    let (stderr, emitted) = compile("genericcond", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10218"), 1, "stderr:\n{stderr}");
    assert_eq!(count_reports(&stderr, "E10012"), 0, "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:3:9"), "stderr:\n{stderr}");
}

/// IT-5: the inferred both-null conditional emits valid C with no deferred
/// diagnostic.
#[test]
fn it5_inferred_both_null_emits_valid_c() {
    let source = r#"module spec.tpriii.it5;
import { println } from std.io;
func main(): i32 {
    let c: bool = true;
    let z = c ? null : null;
    if (z == null) { println("n"); }
    return 0;
}
"#;
    let (stderr, emitted) = compile("bothnull", source);
    assert!(!stderr.contains("error "), "stderr:\n{stderr}");
    let c = emitted.expect("the valid program emitted C");
    assert!(c.contains("(c ? NULL : NULL)"), "{c}");
    build_strict("bothnull", &c);
}

/// IT-6: the deferred null comparison reports at the literal null.
#[test]
fn it6_null_comparison_diagnostic_surface() {
    let source = r#"module spec.tpriii.it6;
func f<T>(x: T): bool { return null == x; }
func main(): i32 { return f<i32>(1) ? 1 : 0; }
"#;
    let (stderr, emitted) = compile("nullcomparison", source);
    assert!(emitted.is_none(), "a rejected program wrote output C");
    assert_eq!(count_reports(&stderr, "E10012"), 1, "stderr:\n{stderr}");
    assert!(stderr.contains("'null' is not allowed here"), "stderr:\n{stderr}");
    assert!(stderr.contains("main.clo:2:32"), "stderr:\n{stderr}");
}

/// IT-7: all three shims null-test the cstring length.
#[test]
fn it7_cstring_length_guards() {
    let source = r#"module spec.tpriii.it7;
func main(): i32 { return 0; }
"#;
    let host = emitted_profile("shimhost", "host", source);
    assert!(
        host.contains("return text == NULL ? 0 : strlen(text);"),
        "{host}"
    );
    let amiga = emitted_profile("shimamiga", "hosted-amiga", source);
    assert!(
        amiga.contains("return text == 0 ? 0 : (unsigned long)strlen(text);"),
        "{amiga}"
    );
    let free = emitted_profile("shimfree", "freestanding", source);
    assert!(
        free.contains("return text == 0 ? 0 : strlen(text);"),
        "{free}"
    );
}

/// IT-8: the fixed-array step emits the flat loop and the hidden store.
#[test]
fn it8_fixed_array_step_is_flat() {
    let source = r#"module spec.tpriii.it8;
func mk3(): i32[3] { return [1, 2, 3]; }
func main(): i32 {
    let a: i32[3] = [0, 0, 0];
    for (let i: i32 = 0; i < 1; a = mk3(), i = i + 1) { }
    return a[0];
}
"#;
    let c = emitted_profile("steparray", "host", source);
    assert!(c.contains("goto __cl_b1;"), "the step kept a compact header:\n{c}");
    assert!(
        c.contains("cl_spec_tpriii_it8_mk3(__cl_call"),
        "the hidden destination call is missing:\n{c}"
    );
    assert!(
        c.contains("memcpy(&(a), &(__cl_call"),
        "the element copy is missing:\n{c}"
    );
}

/// IT-9: a non-owning step keeps the compact C `for` header.
#[test]
fn it9_non_owning_step_keeps_the_header() {
    let source = r#"module spec.tpriii.it9;
func main(): i32 {
    let n: i32 = 0;
    for (let i: i32 = 0; i < 3; n = n + 1, i = i + 1) { }
    return n;
}
"#;
    let c = emitted_profile("plainstep", "host", source);
    assert!(
        c.contains(
            "for (; (i < 3); n = (i32)((u32)n + (u32)1), i = (i32)((u32)i + (u32)1))"
        ),
        "the non-owning step lost its compact header:\n{c}"
    );
}

/// IT-10: every intrinsic spelling emits the same `cl_text_*` shape.
#[test]
fn it10_intrinsic_spellings_share_the_shape() {
    let bare = r#"module spec.tpriii.it10a;
import { println } from std.io;
import { text } from std.fmt;
func main(): i32 { println(text(7)); return 0; }
"#;
    let alias = r#"module spec.tpriii.it10b;
import { println } from std.io;
import { text as t } from std.fmt;
func main(): i32 { println(t(7)); return 0; }
"#;
    let qualified = r#"module spec.tpriii.it10c;
import { println } from std.io;
func main(): i32 { println(std.fmt.text(7)); return 0; }
"#;
    for (tag, source) in [("bare", bare), ("alias", alias), ("qualified", qualified)] {
        let c = emitted_profile(tag, "host", source);
        assert_eq!(
            c.matches("cl_text_i32((i32)(7))").count(),
            1,
            "{tag}: unexpected selector shape:\n{c}"
        );
    }
}

/// IT-11: a user `std.fmt.text` declaration keeps its own symbol.
#[test]
fn it11_user_std_fmt_text_keeps_its_symbol() {
    let holder = r#"module std.fmt;
import { String, stringOf } from std.text;
export func text(n: i32): String {
    return stringOf("user");
}
"#;
    let main = r#"module spec.tpriii.it11;
import { println } from std.io;
func main(): i32 {
    println(std.fmt.text(7));
    return 0;
}
"#;
    let c = emitted_pair("userfmt", holder, main);
    assert!(c.contains("cl_std_fmt_text(7)"), "{c}");
    assert!(!c.contains("cl_text_i32((i32)(7))"), "{c}");

    let ffi_holder = r#"module std.fmt;
ffi stdfmtuser {
    func text(n: i32): String;
}
"#;
    let ffi_main = r#"module spec.tpriii.it11b;
import { println } from std.io;
import { text } from std.fmt;
func main(): i32 {
    println(text(7));
    return 0;
}
"#;
    let c = emitted_pair("userffi", ffi_holder, ffi_main);
    assert!(c.contains("text(7)"), "{c}");
    assert!(!c.contains("cl_text_i32((i32)(7))"), "{c}");
    build_object_strict("userffi", &c);
}

/// IT-12: the template String copy shape is unchanged.
#[test]
fn it12_template_string_shape_is_unchanged() {
    let source = r#"module spec.tpriii.it12;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("abc");
    println($"{s}");
    return 0;
}
"#;
    let c = emitted_profile("templateshape", "host", source);
    assert_eq!(c.matches("cl_string_copy(s)").count(), 1, "{c}");
    assert!(c.contains("cl_str_part_owned(__cl_tmp"), "{c}");
    assert!(!c.contains("$text"), "{c}");
}

/// IT-13: a module-level value head does not hide the `std.fmt.text` module
/// path (MOD-13); only a lexical binding shadows it (MOD-16).
#[test]
fn it13_module_level_value_head_uses_the_module_path() {
    let source = r#"module spec.tpriii.it13;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
func main(): i32 {
    println(std.fmt.text(7));
    return 0;
}
"#;
    let c = emitted_profile("valueheadmodule", "host", source);
    assert!(c.contains("cl_text_i32((i32)(7))"), "{c}");
    assert!(!c.contains("std.fmt.text(7)"), "{c}");
}
