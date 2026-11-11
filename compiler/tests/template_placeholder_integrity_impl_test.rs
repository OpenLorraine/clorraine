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

// -- Ordered evaluation emission --------------------------------------------

fn emitted(tag: &str, source: &str) -> String {
    let (stderr, emitted) = compile(tag, source);
    assert!(
        !stderr.contains("error "),
        "{tag}: unexpected rejection:\n{stderr}"
    );
    emitted.unwrap_or_else(|| panic!("{tag}: no C emitted"))
}

fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: emitted C does not contain {needle:?}:\n{text}"
    );
}

/// IT-1: a String placeholder is copied once and the part adopts it.
#[test]
fn it1_string_placeholder_adopts_one_copy() {
    let source = r#"module spec.tpii.stringcopy;
import { println } from std.io;
import { String, stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("abc");
    println($"[{s}]");
    return 0;
}
"#;
    let c = emitted("stringcopy", source);
    assert_eq!(c.matches("cl_string_copy(s)").count(), 1, "{c}");
    assert_contains("stringcopy", &c, "cl_str_part_owned(__cl_tmp");
}

/// IT-2: a `str`-returning call is emitted once.
#[test]
fn it2_str_call_is_emitted_once() {
    let source = r#"module spec.tpii.strcall;
import { println } from std.io;
let calls: i32 = 0;
func mk(): str { calls = calls + 1; return "x"; }
func main(): i32 {
    println($"{mk()}");
    return 0;
}
"#;
    let c = emitted("strcalli", source);
    assert_eq!(c.matches("mk()").count(), 1, "{c}");
}

/// IT-3: the copy precedes the take call in the host C.
#[test]
fn it3_copy_precedes_take() {
    let source = r#"module spec.tpii.takeorder;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let s: String = stringOf("alpha");
    println($"{s}{consume(s)}");
    return 0;
}
"#;
    let c = emitted("takeorder", source);
    let copy = c.find("cl_string_copy(s)").expect("copy");
    let call = c.find("consume(s)").expect("take call");
    assert!(copy < call, "the copy must precede the take call:\n{c}");
}

/// IT-4: a pure scalar-only template keeps its direct parts and adds no temp.
#[test]
fn it4_pure_scalar_template_has_no_temp() {
    let source = r#"module spec.tpii.scalar;
import { println } from std.io;
func main(): i32 {
    let i: u32 = 3;
    println($"token-{i}-{i * u32(3)}");
    return 0;
}
"#;
    let c = emitted("scalarparts", source);
    let line = c
        .lines()
        .find(|line| line.contains("cl_string_concat((ClStrPart[])"))
        .expect("template line");
    assert!(!line.contains("__cl_tmp"), "{line}");
    assert_contains("scalarparts", line, "cl_str_part_u32(");
}

/// IT-5: the internal copy sentinel never reaches the C.
#[test]
fn it5_internal_sentinel_is_not_emitted() {
    let source = r#"module spec.tpii.sentinel;
import { println } from std.io;
import { stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("abc");
    println($"{s}");
    return 0;
}
"#;
    let c = emitted("sentinel", source);
    assert!(!c.contains("$text"), "the sentinel leaked into the C:\n{c}");
}

/// IT-6: a generic String instance copies before the take call.
#[test]
fn it6_generic_copy_precedes_take() {
    let source = r#"module spec.tpii.generictake;
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
    let c = emitted("generictakei", source);
    let copy = c.find("cl_string_copy(x)").expect("generic copy");
    let call = c.find("consume(y)").expect("generic take");
    assert!(copy < call, "the generic copy must precede the take:\n{c}");
}

/// IT-7: the module source is freed and the adopted copy is not.
#[test]
fn it7_module_source_release_and_copy_retention() {
    let source = r#"module spec.tpii.modulecopy;
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
    let c = emitted("modulecopy", source);
    assert_eq!(c.matches("mkString()").count(), 1, "{c}");
    let init_start = c.find("static void cl_init_spec_tpii_modulecopy(void)").expect("init");
    let init_end = c[init_start..].find("\n}").map(|end| init_start + end).expect("init end");
    let init = &c[init_start..init_end];
    let part = init.find("cl_str_part_owned(__cl_").expect("adopted part");
    let part_name_start = part + "cl_str_part_owned(".len();
    let part_name_end = init[part_name_start..].find(')').unwrap() + part_name_start;
    let part_name = &init[part_name_start..part_name_end];
    let freed = init
        .match_indices("FreeString(")
        .map(|(index, _)| {
            let start = index + "FreeString(".len();
            let end = init[start..].find(')').unwrap() + start;
            &init[start..end]
        })
        .collect::<Vec<_>>();
    assert!(!freed.is_empty(), "the source was not released:\n{init}");
    assert!(
        !freed.iter().any(|name| *name == part_name),
        "the adopted copy {part_name} was freed twice:\n{init}"
    );
}

/// IT-8: host and Amiga emit the same copy-before-take order.
#[test]
fn it8_amiga_order_matches_host() {
    let source = r#"module spec.tpii.amigaorder;
import { println } from std.io;
import { stringOf } from std.text;
func consume(take s: String): i32 { return i32(len(s)); }
func main(): i32 {
    let s: String = stringOf("alpha");
    println($"{s}{consume(s)}");
    return 0;
}
"#;
    for profile in ["host", "hosted-amiga"] {
        let (stderr, emitted) = {
            let scratch = ScratchDir::new("amigaorder");
            let source_path = scratch.path().join("main.clo");
            std::fs::write(&source_path, source).expect("write main.clo");
            let output_path = scratch.path().join("out.c");
            let output = run_clc(&[
                "--profile".to_owned(),
                profile.to_owned(),
                "-o".to_owned(),
                output_path.display().to_string(),
                source_path.display().to_string(),
            ]);
            (
                String::from_utf8_lossy(&output.stderr).into_owned(),
                std::fs::read_to_string(&output_path).expect("read the emitted C"),
            )
        };
        assert!(!stderr.contains("error "), "{profile}: {stderr}");
        let copy = emitted.find("cl_string_copy(s)").expect("copy");
        let call = emitted.find("consume(s)").expect("take call");
        assert!(copy < call, "{profile}: copy must precede the take:\n{emitted}");
    }
}

/// IT-9: the short-circuit copy is an adopting fill with no second free.
#[test]
fn it9_short_circuit_copy_is_adopting() {
    let source = r#"module spec.tpii.shortadopt;
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
    let c = emitted("shortadopt", source);
    let part = c.find("cl_str_part_owned(__cl_tmp").expect("adopted part");
    let name_start = part + "cl_str_part_owned(".len();
    let name_end = c[name_start..].find(')').unwrap() + name_start;
    let name = &c[name_start..name_end];
    assert!(
        !c.contains(&format!("FreeString({name})")),
        "the admitted copy {name} was freed:\n{c}"
    );
}

/// IT-10: the template part count and kind codes are unchanged.
#[test]
fn it10_part_count_and_kind_codes() {
    let source = r#"module spec.tpii.parts;
import { println } from std.io;
func main(): i32 {
    let n: i32 = 1;
    println($"n={n}");
    return 0;
}
"#;
    let c = emitted("partcount", source);
    assert_contains("partcount", &c, "#define CL_STR_KIND_I32 2u");
    assert_contains("partcount", &c, "}, 2u);");
}

/// IT-13: a pinned adopted call keeps its argument's bounds trap.
#[test]
fn it13_adopted_copy_keeps_guard_and_release() {
    let source = r#"module spec.tpii.adoptguard;
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
    let c = emitted("adoptguard", source);
    let lines: Vec<&str> = c.lines().collect();
    let call = lines
        .iter()
        .position(|line| line.contains("= cl_spec_tpii_adoptguard_mk("))
        .expect("pinned call");
    assert!(call > 0, "the pinned call has no preceding line");
    assert!(
        lines[call - 1].contains("cl_trap(\"E11211\", \"index out of range\")"),
        "the guard must be the line before the pinned call:\n{}",
        lines[call - 1]
    );
    let name = lines[call].split('=').next().unwrap_or_default().trim();
    assert!(
        !c.contains(&format!("FreeString({name})")),
        "the adopted call {name} was released:\n{c}"
    );
}
