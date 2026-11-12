//! Host specification tests for the generated-C robustness bundle.
//!
//! The cases pin the module-path precedence (MOD-13/MOD-16), the owned text
//! argument release, and the module-initializer parity. The oracle is the
//! frozen module rules and the reported #16/#30/#31 programs.

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
            "clc-generated-robustness-{}-{tag}-{sequence}",
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


// -- Module-path precedence -------------------------------------------------

/// ST-1: a lexical `std` value keeps the member-call behavior.
#[test]
fn st1_lexical_std_member_call_runs() {
    let source = r#"module spec.gcr.lexical;
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
    assert_output("lexical", source, "u\n");
}

/// ST-2: a module-level `std` value does not hide the `std.fmt.text` module
/// path (MOD-13); a local alone may shadow it (MOD-16).
#[test]
fn st2_module_level_std_uses_the_module_path() {
    let source = r#"module spec.gcr.modulehead;
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
    assert_output("modulehead", source, "7\n");
}

/// ST-3: an imported `std` value does not hide the module path either.
#[test]
fn st3_imported_std_uses_the_module_path() {
    let holder = r#"module lib.head;
import { String, stringOf } from std.text;
export struct Fmt { text: func(i32): String; }
export struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
export let std: Std = Std { fmt: Fmt { text: userText } };
"#;
    let main = r#"module spec.gcr.imported;
import { println } from std.io;
import { std } from lib.head;
func main(): i32 {
    println(std.fmt.text(7));
    return 0;
}
"#;
    assert_pair_output("imported", holder, main, "7\n");
}

/// ST-4: the module-path intrinsic keeps the formattability rule.
#[test]
fn st4_module_level_std_text_null_is_rejected() {
    let source = r#"module spec.gcr.moduleheadnull;
import { String } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return "u"; }
let std: Std = Std { fmt: Fmt { text: userText } };
func main(): i32 {
    let s: String = std.fmt.text(null);
    return i32(len(s));
}
"#;
    assert_rejected("moduleheadnull", source, "E10500", &["type 'null' is not formattable"]);
}

/// ST-5: a real user `std.fmt.text` declaration wins over the intrinsic.
#[test]
fn st5_user_std_fmt_text_wins() {
    let holder = r#"module std.fmt;
import { String, stringOf } from std.text;
export func text(n: i32): String {
    return stringOf("user");
}
"#;
    let main = r#"module spec.gcr.userfmt;
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
    assert_pair_output("userfmt", holder, main, "user\n");
}

/// ST-6: an owned temporary argument to bare `text` is released.
#[test]
fn st6_owned_text_argument_is_released() {
    let source = r#"module spec.gcr.textarg;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func main(): i32 {
    println(text(stringOf("x")));
    return 0;
}
"#;
    assert_leak_free("textarg", source, "x\n");
}

/// ST-7: an owned template argument to `text` is released.
#[test]
fn st7_owned_template_argument_is_released() {
    let source = r#"module spec.gcr.texttpl;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func main(): i32 {
    let s: String = stringOf("abc");
    println(text($"{s}"));
    return 0;
}
"#;
    assert_leak_free("texttpl", source, "abc\n");
}

/// ST-8: a named-place argument stays a copy and remains readable.
#[test]
fn st8_named_text_argument_stays_readable() {
    let source = r#"module spec.gcr.textplace;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func main(): i32 {
    let s: String = stringOf("x");
    println(text(s));
    println(s);
    return 0;
}
"#;
    assert_output("textplace", source, "x\nx\n");
}

/// ST-9: a discarded `text` call releases its owned argument.
#[test]
fn st9_discarded_text_releases_its_argument() {
    let source = r#"module spec.gcr.textdiscard;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func main(): i32 {
    text(stringOf("x"));
    return 0;
}
"#;
    assert_leak_free("textdiscard", source, "");
}

/// ST-10: the alias and qualified spellings release their owned arguments.
#[test]
fn st10_alias_and_qualified_arguments_are_released() {
    let source = r#"module spec.gcr.textspellings;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text as t } from std.fmt;
func main(): i32 {
    println(t(stringOf("x")));
    println(std.fmt.text(stringOf("y")));
    return 0;
}
"#;
    assert_leak_free("textspellings", source, "x\ny\n");
}

/// ST-11: a module initializer uses the module path, not a module-level
/// `std` field. The member returns `"u"` (length 1); the intrinsic gives 4.
#[test]
fn st11_initializer_module_path_wins() {
    let source = r#"module spec.gcr.inithead;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
let n: usize = len(std.fmt.text(1234));
func main(): i32 {
    println($"{n}");
    return 0;
}
"#;
    assert_output("inithead", source, "4\n");
}

/// ST-12: a module-level global whose head is not a module path keeps its
/// member call.
#[test]
fn st12_non_module_global_field_call_runs() {
    let source = r#"module spec.gcr.globalfield;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
func userText(n: i32): String { return stringOf("u"); }
let box: Fmt = Fmt { text: userText };
func main(): i32 {
    println(box.text(7));
    return 0;
}
"#;
    assert_output("globalfield", source, "u\n");
}

/// ST-13: an imported `std` value in an initializer still uses the module
/// path (intrinsic length 4, not the member's 1).
#[test]
fn st13_imported_std_initializer_uses_the_module_path() {
    let holder = r#"module lib.head;
import { String, stringOf } from std.text;
export struct Fmt { text: func(i32): String; }
export struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
export let std: Std = Std { fmt: Fmt { text: userText } };
"#;
    let main = r#"module spec.gcr.importedinit;
import { println } from std.io;
import { std } from lib.head;
let n: usize = len(std.fmt.text(1234));
func main(): i32 {
    println($"{n}");
    return 0;
}
"#;
    assert_pair_output("importedinit", holder, main, "4\n");
}

/// ST-14: the initializer text argument is already released; the bundle keeps
/// it leak-free.
#[test]
fn st14_initializer_text_argument_is_leak_free() {
    let source = r#"module spec.gcr.initarg;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
let n: usize = len(text(stringOf("ab")));
func main(): i32 {
    println($"{n}");
    return 0;
}
"#;
    assert_leak_free("initarg", source, "2\n");
}

/// ST-15: a user `std.fmt` func-typed global `text` wins over the intrinsic,
/// including in an initializer.
#[test]
fn st15_user_global_text_declaration_wins() {
    let holder = r#"module std.fmt;
import { String, stringOf } from std.text;
func userText(n: i32): String { return stringOf("gg"); }
export let text: func(i32): String = userText;
"#;
    let main = r#"module spec.gcr.userglobal;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
let n: usize = len(std.fmt.text(1234));
func main(): i32 {
    println($"{n}");
    return 0;
}
"#;
    assert_pair_output("userglobal", holder, main, "2\n");
}
