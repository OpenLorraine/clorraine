//! Host specification tests for the third template-placeholder residual set.
//!
//! The cases pin the aliased and qualified `text` spellings, the generic
//! condition context, the both-null decision, and the literal-null comparison
//! against an open parameter. The oracle is the frozen specification: Ch 04 §3
//! (operators and source order), §6 (null context type), §12 (formattable
//! types), the E10218 condition rule, and the E10500/E10010/E10012 registry
//! text. The cstring-panic and array-step cases follow in the same suite.
//!
//! A rejected program must exit 1, report the named diagnostic, and write no
//! C. A run case builds under `-Wall -Wextra -Werror`, runs, and compares
//! stdout. Two holder-module helpers cover the alias/qualified shadow cases.

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
            "clc-template-residual-iii-{}-{tag}-{sequence}",
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
    build_and_run(tag, &scratch, emitted)
}

/// Builds the emitted C into a host executable and runs it.
fn build_and_run(tag: &str, scratch: &ScratchDir, emitted: String) -> Output {
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

/// Emits a program and its declaring module as one build.
fn emit_pair(tag: &str, holder: &str, main: &str) -> (ScratchDir, String) {
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
    let emitted = std::fs::read_to_string(&output_path).expect("read the emitted C");
    (scratch, emitted)
}

fn run_pair(tag: &str, holder: &str, main: &str) -> Output {
    let (scratch, emitted) = emit_pair(tag, holder, main);
    build_and_run(tag, &scratch, emitted)
}

fn assert_pair_output(tag: &str, holder: &str, main: &str, expected: &str) {
    let output = run_pair(tag, holder, main);
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
    assert_panic_clean_message(tag, source, None);
}

fn assert_panic_clean_message(tag: &str, source: &str, message: Option<&str>) {
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
    if let Some(message) = message {
        assert!(
            stderr.contains(message),
            "{tag}: the panic message is missing; stderr:\n{stderr}"
        );
    }
    assert!(
        !stderr.contains("runtime error:") && !stderr.contains("AddressSanitizer"),
        "{tag}: the sanitizer reported an error; stderr:\n{stderr}"
    );
}


/// Runs the front end and returns stderr, requiring a rejection and no C.
fn reject(tag: &str, source: &str) -> String {
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
    assert_eq!(
        output.status.code(),
        Some(1),
        "{tag}: expected the front end to reject the program; stderr:\n{stderr}"
    );
    assert!(
        !output_path.exists(),
        "{tag}: a rejected program wrote output C"
    );
    stderr
}


// -- Text spellings --------------------------------------------------------

/// ST-1: an aliased `text` call runs through the intrinsic converter.
#[test]
fn st1_aliased_text_scalar_runs() {
    let source = r#"module spec.tpriii.alias;
import { println } from std.io;
import { text as t } from std.fmt;
func main(): i32 {
    println(t(7));
    return 0;
}
"#;
    assert_output("aliasscalar", source, "7\n");
}

/// ST-2: the alias keeps the formattability rule for a null argument.
#[test]
fn st2_aliased_text_null_is_rejected() {
    let source = r#"module spec.tpriii.aliasnull;
import { String } from std.text;
import { text as t } from std.fmt;
func main(): i32 {
    let s: String = t(null);
    return i32(len(s));
}
"#;
    assert_rejected("aliasnull", source, "E10500", &["type 'null' is not formattable"]);
}

/// ST-3: the alias keeps the formattability rule for a struct argument.
#[test]
fn st3_aliased_text_struct_is_rejected() {
    let source = r#"module spec.tpriii.aliasstruct;
import { String } from std.text;
import { text as t } from std.fmt;
struct Box { n: i32; }
func main(): i32 {
    let s: String = t(Box { n: 1 });
    return i32(len(s));
}
"#;
    assert_rejected("aliasstruct", source, "E10500", &["type 'Box' is not formattable"]);
}

/// ST-4: an unnarrowed nullable alias argument is E10010.
#[test]
fn st4_aliased_text_nullable_is_rejected() {
    let source = r#"module spec.tpriii.aliasnullable;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text as t } from std.fmt;
func main(): i32 {
    let s: String? = stringOf("x");
    println(t(s));
    return 0;
}
"#;
    assert_rejected("aliasnullable", source, "E10010", &["'s'"]);
}

/// ST-5: a narrowed nullable alias argument runs.
#[test]
fn st5_aliased_text_narrowed_runs() {
    let source = r#"module spec.tpriii.aliasnarrowed;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text as t } from std.fmt;
func main(): i32 {
    let s: String? = stringOf("x");
    if (s != null) {
        println(t(s));
    }
    return 0;
}
"#;
    assert_output("aliasnarrowed", source, "x\n");
}

/// ST-6: the module-qualified spelling runs.
#[test]
fn st6_qualified_text_scalar_runs() {
    let source = r#"module spec.tpriii.qualified;
import { println } from std.io;
func main(): i32 {
    println(std.fmt.text(7));
    return 0;
}
"#;
    assert_output("qualifiedscalar", source, "7\n");
}

/// ST-7: the qualified spelling rejects a null argument.
#[test]
fn st7_qualified_text_null_is_rejected() {
    let source = r#"module spec.tpriii.qualifiednull;
import { String } from std.text;
func main(): i32 {
    let s: String = std.fmt.text(null);
    return i32(len(s));
}
"#;
    assert_rejected("qualifiednull", source, "E10500", &["type 'null' is not formattable"]);
}

/// ST-8: the qualified spelling rejects a struct argument.
#[test]
fn st8_qualified_text_struct_is_rejected() {
    let source = r#"module spec.tpriii.qualifiedstruct;
import { String } from std.text;
struct Box { n: i32; }
func main(): i32 {
    let s: String = std.fmt.text(Box { n: 1 });
    return i32(len(s));
}
"#;
    assert_rejected("qualifiedstruct", source, "E10500", &["type 'Box' is not formattable"]);
}

/// ST-9: the alias keeps the one-argument arity rule.
#[test]
fn st9_aliased_text_arity_is_rejected() {
    let zero = r#"module spec.tpriii.aliaszero;
import { String } from std.text;
import { text as t } from std.fmt;
func main(): i32 {
    let s: String = t();
    return i32(len(s));
}
"#;
    assert_rejected(
        "aliaszero",
        zero,
        "E10308",
        &["wrong number of arguments for 'text': expected 1, found 0", "pass one argument per parameter"],
    );
    let two = r#"module spec.tpriii.aliastwo;
import { println } from std.io;
import { text as t } from std.fmt;
func main(): i32 {
    println(t(1, 2));
    return 0;
}
"#;
    assert_rejected(
        "aliastwo",
        two,
        "E10308",
        &["wrong number of arguments for 'text': expected 1, found 2"],
    );
}

/// ST-10: an unrelated import alias is not the intrinsic.
#[test]
fn st10_unrelated_alias_runs() {
    let holder = r#"module my.other;
import { String, stringOf } from std.text;
export func helper(a: i32, b: i32): String {
    return stringOf("two");
}
"#;
    let main = r#"module spec.tpriii.unrelatedalias;
import { println } from std.io;
import { helper as t } from my.other;
func main(): i32 {
    println(t(1, 2));
    return 0;
}
"#;
    assert_pair_output("unrelatedalias", holder, main, "two\n");
}

/// ST-11: a lexical value named `t` shadows the alias.
#[test]
fn st11_lexical_alias_shadow_runs() {
    let source = r#"module spec.tpriii.lexicalshadow;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text as t } from std.fmt;
func userText(n: i32): String { return stringOf("local"); }
func main(): i32 {
    let t: func(i32): String = userText;
    println(t(7));
    return 0;
}
"#;
    assert_output("lexicalshadow", source, "local\n");
}

/// ST-12: the alias defers a generic argument's formattability.
#[test]
fn st12_aliased_generic_struct_is_rejected() {
    let source = r#"module spec.tpriii.aliasgeneric;
import { String } from std.text;
import { text as t } from std.fmt;
struct Box { n: i32; }
func show<T>(x: T): String {
    return t(x);
}
func main(): i32 {
    let s: String = show<Box>(Box { n: 1 });
    return i32(len(s));
}
"#;
    assert_rejected("aliasgeneric", source, "E10500", &["type 'Box' is not formattable"]);
}

/// ST-13: the alias accepts a formattable generic instance.
#[test]
fn st13_aliased_generic_scalar_runs() {
    let source = r#"module spec.tpriii.aliasgenericok;
import { println } from std.io;
import { String } from std.text;
import { text as t } from std.fmt;
func show<T>(x: T): String {
    return t(x);
}
func main(): i32 {
    println(show<i32>(7));
    return 0;
}
"#;
    assert_output("aliasgenericok", source, "7\n");
}

/// ST-14: a user `std.fmt.text` function wins over the intrinsic.
#[test]
fn st14_user_std_fmt_text_wins() {
    let holder = r#"module std.fmt;
import { String, stringOf } from std.text;
export func text(n: i32): String {
    return stringOf("user");
}
"#;
    let main = r#"module spec.tpriii.userfmt;
import { println } from std.io;
func main(): i32 {
    println(std.fmt.text(7));
    return 0;
}
"#;
    assert_pair_output("userfmt", holder, main, "user\n");
}

/// ST-15: a value head makes `std.fmt.text` a member path.
#[test]
fn st15_value_head_is_a_member_path() {
    let source = r#"module spec.tpriii.valuehead;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
func main(): i32 {
    let std: Std = Std { fmt: Fmt { text: userText } };
    println(std.fmt.text(7));
    return 0;
}
"#;
    assert_output("valuehead", source, "u\n");
    // A module-level or imported value head does not shadow the module path
    // (MOD-13); only a lexical binding does (MOD-16). IT-13 pins the emission.
}

// -- Generic condition context ---------------------------------------------

/// ST-16: a generic scalar condition reports E10218, not E10012.
#[test]
fn st16_generic_condition_scalar_is_e10218() {
    let source = r#"module spec.tpriii.gencondscalar;
func show<T>(c: bool, x: T): i32 {
    if (c ? null : x) { return 1; }
    return 0;
}
func main(): i32 {
    return show<i32>(false, 1);
}
"#;
    assert_rejected_with("gencondscalar", source, "E10218", &["E10012"], &[]);
}

/// ST-17: a generic String condition reports E10218, not E10012.
#[test]
fn st17_generic_condition_string_is_e10218() {
    let source = r#"module spec.tpriii.gencondstring;
import { String, stringOf } from std.text;
func show<T>(c: bool, x: T): i32 {
    if (c ? null : x) { return 1; }
    return 0;
}
func main(): i32 {
    return show<String>(false, stringOf("x"));
}
"#;
    assert_rejected_with("gencondstring", source, "E10218", &["E10012"], &[]);
}

/// ST-18: a bool settlement keeps the null-arm E10012 rule.
#[test]
fn st18_generic_condition_bool_keeps_e10012() {
    let source = r#"module spec.tpriii.gencondbool;
func show<T>(c: bool, x: T): i32 {
    if (c ? x : null) { return 1; }
    return 0;
}
func main(): i32 {
    return show<bool>(true, false);
}
"#;
    assert_rejected_with("gencondbool", source, "E10012", &["E10218"], &["'null' is not allowed here"]);
}

/// ST-19: a generic logical operand matches the concrete E10218 + E10012
/// pair.
#[test]
fn st19_generic_logical_condition_reports_both() {
    let source = r#"module spec.tpriii.gencondlogical;
func show<T>(flag: bool, c: bool, x: T): bool {
    return flag && (c ? null : x);
}
func main(): i32 {
    return show<i32>(true, false, 1) ? 1 : 0;
}
"#;
    let stderr = reject("gencondlogical", source);
    assert_eq!(stderr.matches("error E10218:").count(), 1, "stderr:\n{stderr}");
    assert_eq!(stderr.matches("error E10012:").count(), 1, "stderr:\n{stderr}");
}

/// ST-20: a generic while condition reports E10218.
#[test]
fn st20_generic_while_condition_is_e10218() {
    let source = r#"module spec.tpriii.gencondwhile;
func show<T>(c: bool, x: T): i32 {
    while (c ? null : x) { return 1; }
    return 0;
}
func main(): i32 {
    return show<i32>(false, 1);
}
"#;
    assert_rejected_with("gencondwhile", source, "E10218", &["E10012"], &[]);
}

/// ST-21: a generic assert condition reports E10218.
#[test]
fn st21_generic_assert_condition_is_e10218() {
    let source = r#"module spec.tpriii.gencondassert;
func show<T>(c: bool, x: T): i32 {
    assert(c ? null : x);
    return 0;
}
func main(): i32 {
    return show<i32>(false, 1);
}
"#;
    assert_rejected_with("gencondassert", source, "E10218", &["E10012"], &[]);
}

/// ST-22: a generic conditional nested in a value operand still reports
/// E10012; the condition flag is one-shot.
#[test]
fn st22_generic_nested_value_operand_is_e10012() {
    let source = r#"module spec.tpriii.gencondnested;
func show<T>(c: bool, x: T): i32 {
    if (-(c ? null : x) > 0) { return 1; }
    return 0;
}
func main(): i32 {
    return show<i32>(false, 1);
}
"#;
    assert_rejected_with("gencondnested", source, "E10012", &["E10218"], &["'null' is not allowed here"]);
}

/// ST-23: the concrete twin reports E10218 only.
#[test]
fn st23_concrete_condition_is_e10218() {
    let source = r#"module spec.tpriii.concretecond;
func main(): i32 {
    let c: bool = true;
    if (c ? null : 5) { return 1; }
    return 0;
}
"#;
    assert_rejected_with("concretecond", source, "E10218", &["E10012"], &[]);
}

// -- Both-null conditional -------------------------------------------------

/// ST-24: an inferred both-null conditional is the null value.
#[test]
fn st24_inferred_both_null_runs() {
    let source = r#"module spec.tpriii.bothnull;
import { println } from std.io;
func main(): i32 {
    let c: bool = true;
    let z = c ? null : null;
    if (z == null) { println("n"); }
    return 0;
}
"#;
    assert_output("bothnull", source, "n\n");
}

/// ST-25: a concrete both-null destination reports each arm.
#[test]
fn st25_typed_both_null_reports_each_arm() {
    let source = r#"module spec.tpriii.bothnulltyped;
func main(): i32 {
    let c: bool = true;
    let z: i32 = c ? null : null;
    return z;
}
"#;
    let stderr = reject("bothnulltyped", source);
    assert_eq!(stderr.matches("error E10012:").count(), 2, "stderr:\n{stderr}");
}

/// ST-26: a nullable String destination accepts the both-null value.
#[test]
fn st26_nullable_both_null_runs() {
    let source = r#"module spec.tpriii.bothnullnullable;
import { String } from std.text;
func main(): i32 {
    let c: bool = true;
    let s: String? = c ? null : null;
    if (s == null) { return 0; }
    return 1;
}
"#;
    assert_output("bothnullnullable", source, "");
}

/// ST-27: a concrete use of the inferred both-null value reports.
#[test]
fn st27_inferred_both_null_concrete_use_is_rejected() {
    let source = r#"module spec.tpriii.bothnulluse;
func main(): i32 {
    let c: bool = true;
    let z = c ? null : null;
    let y: i32 = z;
    return y;
}
"#;
    assert_rejected("bothnulluse", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-28: a placeholder over both null arms is E10500.
#[test]
fn st28_both_null_placeholder_is_rejected() {
    let source = r#"module spec.tpriii.bothnullplaceholder;
import { println } from std.io;
func main(): i32 {
    let c: bool = true;
    println($"{c ? null : null}");
    return 0;
}
"#;
    assert_rejected("bothnullplaceholder", source, "E10500", &["not formattable"]);
}

/// ST-29: a both-null condition reports E10218.
#[test]
fn st29_both_null_condition_is_e10218() {
    let source = r#"module spec.tpriii.bothnullcondition;
func main(): i32 {
    let c: bool = true;
    if (c ? null : null) { return 1; }
    return 0;
}
"#;
    assert_rejected("bothnullcondition", source, "E10218", &[]);
}

/// ST-30: a nullable cstring destination accepts the both-null value.
#[test]
fn st30_nullable_cstring_both_null_runs() {
    let source = r#"module spec.tpriii.bothnullcstring;
func main(): i32 {
    let c: bool = true;
    let p: cstring? = c ? null : null;
    if (p == null) { return 0; }
    return 1;
}
"#;
    assert_output("bothnullcstring", source, "");
}

/// ST-31: a concrete call argument reports each both-null arm.
#[test]
fn st31_both_null_concrete_argument_reports_each_arm() {
    let source = r#"module spec.tpriii.bothnullarg;
func f(n: i32): i32 { return n; }
func main(): i32 {
    let c: bool = true;
    return f(c ? null : null);
}
"#;
    let stderr = reject("bothnullarg", source);
    assert_eq!(stderr.matches("error E10012:").count(), 2, "stderr:\n{stderr}");
}

/// ST-32: a generic call argument reports each both-null arm.
#[test]
fn st32_both_null_generic_argument_reports_each_arm() {
    let source = r#"module spec.tpriii.bothnullgeneric;
func id<T>(x: T): T { return x; }
func main(): i32 {
    let c: bool = true;
    let y: i32 = id<i32>(c ? null : null);
    return y;
}
"#;
    let stderr = reject("bothnullgeneric", source);
    assert_eq!(stderr.matches("error E10012:").count(), 2, "stderr:\n{stderr}");
}

// -- Null compared with an open parameter ----------------------------------

/// ST-33: `null == x` at a scalar instance is E10012.
#[test]
fn st33_null_equals_param_scalar_is_rejected() {
    let source = r#"module spec.tpriii.nullequal;
func f<T>(x: T): bool { return null == x; }
func main(): i32 { return f<i32>(1) ? 1 : 0; }
"#;
    assert_rejected("nullequal", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-34: `null != x` at a scalar instance is E10012 (the left-literal
/// spelling is the issue's reproduction).
#[test]
fn st34_null_not_equals_param_scalar_is_rejected() {
    let source = r#"module spec.tpriii.nullnotequal;
func f<T>(x: T): bool { return null != x; }
func main(): i32 { return f<i32>(1) ? 1 : 0; }
"#;
    assert_rejected("nullnotequal", source, "E10012", &["'null' is not allowed here"]);
}

/// ST-35: the right-literal `x == null` narrowing form stays accepted for a
/// nullable instance.
#[test]
fn st35_param_equals_null_nullable_runs() {
    let source = r#"module spec.tpriii.paramnullablenull;
import { println } from std.io;
import { String } from std.text;
func f<T>(x: T): bool { return x == null; }
func main(): i32 {
    println($"{f<String?>(null)}");
    return 0;
}
"#;
    assert_output("paramnullablenull", source, "true\n");
}

/// ST-36: a non-null String instance accepts `== null`.
#[test]
fn st36_param_equals_null_string_runs() {
    let source = r#"module spec.tpriii.paramstring;
import { println } from std.io;
import { String, stringOf } from std.text;
func f<T>(x: T): bool { return x == null; }
func main(): i32 {
    println($"{f<String>(stringOf("x"))}");
    return 0;
}
"#;
    assert_output("paramstring", source, "false\n");
}

/// ST-37: a non-empty str instance accepts `!= null`.
#[test]
fn st37_param_not_null_str_runs() {
    let source = r#"module spec.tpriii.paramstr;
import { println } from std.io;
func f<T>(x: T): bool { return x != null; }
func main(): i32 {
    println($"{f<str>("x")}");
    return 0;
}
"#;
    assert_output("paramstr", source, "true\n");
}

/// ST-38: the narrowing idiom stays legal for a nullable String instance.
#[test]
fn st38_generic_narrowing_runs() {
    let source = r#"module spec.tpriii.paramnarrow;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func show<T>(x: T): String {
    if (x == null) { return stringOf("n"); }
    return text(x);
}
func main(): i32 {
    println(show<String?>(stringOf("x")));
    return 0;
}
"#;
    assert_output("paramnarrow", source, "x\n");
}

/// ST-39: a nullable Box instance accepts the null comparison.
#[test]
fn st39_param_nullable_box_runs() {
    let source = r#"module spec.tpriii.parambox;
import { println } from std.io;
func f<T>(x: T): bool { return x == null; }
func main(): i32 {
    let b: Box<i32>? = null;
    println($"{f<Box<i32>?>(b)}");
    return 0;
}
"#;
    assert_output("parambox", source, "true\n");
}

/// ST-40: the binding form of a null comparison keeps its E10012 rule.
#[test]
fn st40_null_binding_against_param_is_rejected() {
    let source = r#"module spec.tpriii.nullbinding;
func f<T>(x: T): bool {
    let z = null;
    return z == x;
}
func main(): i32 { return f<i32>(1) ? 1 : 0; }
"#;
    assert_rejected("nullbinding", source, "E10012", &["'null' is not allowed here"]);
}

// -- Runtime and lowering --------------------------------------------------

/// ST-41: a null cstring panics cleanly.
#[test]
fn st41_null_cstring_panic_is_clean() {
    let source = r#"module spec.tpriii.paniccstring;
func main(): i32 {
    let p: cstring = null;
    panic(p);
    return 0;
}
"#;
    assert_panic_clean("paniccstring", source);
}

/// ST-42: a non-empty cstring panic prints its message.
#[test]
fn st42_nonempty_cstring_panic_prints_message() {
    let source = r#"module spec.tpriii.paniccstringmsg;
import { toCString } from std.text;
func main(): i32 {
    panic(toCString("boom"));
    return 0;
}
"#;
    assert_panic_clean_message("paniccstringmsg", source, Some("boom"));
}

/// ST-43: a fixed-array for-step call assignment uses the flat store.
#[test]
fn st43_step_array_call_runs() {
    let source = r#"module spec.tpriii.steparraycall;
import { println } from std.io;
func mk3(): i32[3] { return [1, 2, 3]; }
func main(): i32 {
    let a: i32[3] = [0, 0, 0];
    for (let i: i32 = 0; i < 1; a = mk3(), i = i + 1) { }
    println($"{a[0]}");
    return 0;
}
"#;
    assert_leak_free("steparraycall", source, "1\n");
}

/// ST-44: a fixed-array for-step member assignment uses the flat store.
#[test]
fn st44_step_array_member_runs() {
    let source = r#"module spec.tpriii.steparraymember;
import { println } from std.io;
struct S { vals: i32[3]; }
func main(): i32 {
    let s: S = S { vals: [1, 2, 3] };
    let a: i32[3] = [0, 0, 0];
    for (let i: i32 = 0; i < 1; a = s.vals, i = i + 1) { }
    println($"{a[0]}");
    return 0;
}
"#;
    assert_leak_free("steparraymember", source, "1\n");
}

/// ST-45: an array-literal step keeps the flat store and runs.
#[test]
fn st45_step_array_literal_runs() {
    let source = r#"module spec.tpriii.steparrayliteral;
import { println } from std.io;
func main(): i32 {
    let a: i32[3] = [0, 0, 0];
    for (let i: i32 = 0; i < 1; a = [1, 2, 3], i = i + 1) { }
    println($"{a[1]}");
    return 0;
}
"#;
    assert_leak_free("steparrayliteral", source, "2\n");
}

/// ST-46: an unrelated loaded module's function named `t` does not capture the
/// alias import.
#[test]
fn st46_alias_is_not_captured_by_an_unrelated_function() {
    let holder = r#"module lib.coll2;
import { String, stringOf } from std.text;
export func t(take s: String): String {
    return stringOf("libwrong");
}
"#;
    let main = r#"module spec.tpriii.aliasuncaptured;
import { println } from std.io;
import { t as other } from lib.coll2;
import { text as t } from std.fmt;
func main(): i32 {
    println(t(7));
    return 0;
}
"#;
    assert_pair_output("aliasuncaptured", holder, main, "7\n");
}

/// ST-47: every intrinsic spelling works in a module initializer.
#[test]
fn st47_initializer_text_spellings_run() {
    let source = r#"module spec.tpriii.inittext;
import { println } from std.io;
import { text, text as t } from std.fmt;
let a: usize = len(text(7));
let b: usize = len(t(78));
let c: usize = len(std.fmt.text(789));
func main(): i32 {
    println($"{a}{b}{c}");
    return 0;
}
"#;
    assert_output("inittext", source, "123\n");
}

/// ST-48: an `ffi` block named `text` is not a callable signature, so the
/// intrinsic still fires.
#[test]
fn st48_ffi_block_name_does_not_shadow_text() {
    let source = r#"module spec.tpriii.ffiblockname;
import { println } from std.io;
ffi text {
    func foo(): i32;
}
func main(): i32 {
    println(text(7));
    return 0;
}
"#;
    assert_output("ffiblockname", source, "7\n");
}

/// ST-49: a generic logical condition reports once per site across two
/// non-bool instances, with the operand's null report.
#[test]
fn st49_logical_condition_reports_once_across_instances() {
    let source = r#"module spec.tpriii.gencondmulti;
import { String, stringOf } from std.text;
func show<T>(flag: bool, c: bool, x: T): bool {
    return flag && (c ? null : x);
}
func main(): i32 {
    let b: bool = show<i32>(true, false, 1);
    let a: bool = show<String>(true, false, stringOf("x"));
    return 0;
}
"#;
    let stderr = reject("gencondmulti", source);
    assert_eq!(stderr.matches("error E10218:").count(), 1, "stderr:\n{stderr}");
    assert_eq!(stderr.matches("error E10012:").count(), 1, "stderr:\n{stderr}");
}
