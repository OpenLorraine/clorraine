//! Host specification tests for direct template formatting.
//!
//! The cases pin the allocation contract (one result for an integer/bool
//! template, a copy plus the result for `String`/`str`, two for a float), the
//! unchanged output for every formattable type, the source-copy safety, and
//! the sanitizer behavior. A wrapped `malloc` counts calls around one template
//! expression; the allocation cases are skipped under the sanitizer legs
//! because the wrapper would bypass the sanitizer's allocator.

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
            "clc-template-direct-{}-{tag}-{sequence}",
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

fn build_host(
    tag: &str,
    scratch: &ScratchDir,
    emitted: &str,
    stub: Option<&str>,
    wrap_malloc: bool,
) -> PathBuf {
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
    if let Some(stub) = stub {
        let stub_path = scratch.path().join("stub.c");
        std::fs::write(&stub_path, stub).expect("write the stub");
        command.arg(stub_path.to_str().expect("stub path"));
    }
    if wrap_malloc {
        command.arg("-Wl,--wrap=malloc");
    }
    let built = command.output().expect("spawn cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

fn run(tag: &str, source: &str) -> Output {
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted, None, false);
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
}

fn sanitizer_active() -> bool {
    std::env::var("RDC_ASAN").is_ok() || std::env::var("RDC_UBSAN").is_ok()
}

/// Runs a program with a wrapped `malloc` and returns the printed delta.
fn allocation_delta(tag: &str, source: &str) -> Option<String> {
    if sanitizer_active() {
        return None;
    }
    let stub = r#"#include <stddef.h>
#include <stdlib.h>

extern void *__real_malloc(size_t size);

static int cl_test_malloc_count;

int cl_test_malloc_calls(void)
{
    return cl_test_malloc_count;
}

void *__wrap_malloc(size_t size)
{
    cl_test_malloc_count++;
    return __real_malloc(size);
}
"#;
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted, Some(stub), true);
    let output = Command::new(&program).output().expect("run the program");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn assert_allocation_delta(tag: &str, source: &str, expected: &str) {
    if let Some(delta) = allocation_delta(tag, source) {
        assert_eq!(delta, expected, "{tag}: unexpected allocation delta");
    }
}

fn assert_no_sanitizer_report(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer") && !stderr.contains("runtime error:"),
        "{tag}: the sanitizer reported an error; stderr:\n{stderr}"
    );
}

/// Two integer placeholders allocate only the result.
#[test]
fn two_integer_placeholders_allocate_one_result() {
    let source = r#"module spec.tdf.allocint;
import { println } from std.io;
ffi testlib { func cl_test_malloc_calls(): i32; }
func main(): i32 {
    let before: i32 = cl_test_malloc_calls();
    let a: i32 = -7;
    let b: u32 = 9;
    let s: String = $"{a}-{b}";
    let after: i32 = cl_test_malloc_calls();
    let used: usize = len(s);
    if (used == 0) { return 1; }
    println($"{after - before}");
    return 0;
}
"#;
    assert_allocation_delta("allocint", source, "1");
}

/// A `String` placeholder copies its bytes at part construction for safety;
/// the copy and the result allocate.
#[test]
fn a_string_placeholder_copies_for_safety() {
    let source = r#"module spec.tdf.allocstring;
import { println } from std.io;
import { stringOf } from std.text;
ffi testlib { func cl_test_malloc_calls(): i32; }
func main(): i32 {
    let src: String = stringOf("abc");
    let before: i32 = cl_test_malloc_calls();
    let t: String = $"[{src}]";
    let after: i32 = cl_test_malloc_calls();
    let used: usize = len(src) + len(t);
    if (used == 0) { return 1; }
    println($"{after - before}");
    return 0;
}
"#;
    assert_allocation_delta("allocstring", source, "2");
}

/// A `str` placeholder copies its view at part construction for safety;
/// the copy and the result allocate.
#[test]
fn a_str_placeholder_copies_for_safety() {
    let source = r#"module spec.tdf.allocstr;
import { println } from std.io;
ffi testlib { func cl_test_malloc_calls(): i32; }
func main(): i32 {
    let v: str = "abc";
    let before: i32 = cl_test_malloc_calls();
    let t: String = $"[{v}]";
    let after: i32 = cl_test_malloc_calls();
    let used: usize = len(t);
    if (used == 0) { return 1; }
    println($"{after - before}");
    return 0;
}
"#;
    assert_allocation_delta("allocstr", source, "2");
}

/// A bool placeholder is formatted into the result; only the result allocates.
#[test]
fn a_bool_placeholder_allocates_the_result() {
    let source = r#"module spec.tdf.allocbool;
import { println } from std.io;
ffi testlib { func cl_test_malloc_calls(): i32; }
func main(): i32 {
    let before: i32 = cl_test_malloc_calls();
    let t: String = $"{true}";
    let after: i32 = cl_test_malloc_calls();
    let used: usize = len(t);
    if (used == 0) { return 1; }
    println($"{after - before}");
    return 0;
}
"#;
    assert_allocation_delta("allocbool", source, "1");
}

/// A float placeholder keeps the documented `%g` temporary.
#[test]
fn a_float_placeholder_keeps_its_temporary() {
    let source = r#"module spec.tdf.allocfloat;
import { println } from std.io;
ffi testlib { func cl_test_malloc_calls(): i32; }
func main(): i32 {
    let before: i32 = cl_test_malloc_calls();
    let t: String = $"{1.5}";
    let after: i32 = cl_test_malloc_calls();
    let used: usize = len(t);
    if (used == 0) { return 1; }
    println($"{after - before}");
    return 0;
}
"#;
    assert_allocation_delta("allocfloat", source, "2");
}

/// A literal-only template is a view and allocates nothing.
#[test]
fn a_literal_only_template_allocates_nothing() {
    let source = r#"module spec.tdf.allocplain;
import { println } from std.io;
ffi testlib { func cl_test_malloc_calls(): i32; }
func main(): i32 {
    let before: i32 = cl_test_malloc_calls();
    let v: str = $"plain";
    let after: i32 = cl_test_malloc_calls();
    let used: usize = len(v);
    if (used == 0) { return 1; }
    println($"{after - before}");
    return 0;
}
"#;
    assert_allocation_delta("allocplain", source, "0");
}

/// An explicit `text` call still allocates its one converter block.
#[test]
fn an_explicit_text_call_keeps_its_allocation() {
    let source = r#"module spec.tdf.alloctext;
import { println } from std.io;
import { text } from std.fmt;
ffi testlib { func cl_test_malloc_calls(): i32; }
func main(): i32 {
    let before: i32 = cl_test_malloc_calls();
    let t: String = text(7);
    let after: i32 = cl_test_malloc_calls();
    let used: usize = len(t);
    if (used == 0) { return 1; }
    println($"{after - before}");
    return 0;
}
"#;
    assert_allocation_delta("alloctext", source, "1");
}

/// The result is independent of a source that is reassigned after the
/// template expression.
#[test]
fn a_template_result_is_independent_of_the_source() {
    let source = r#"module spec.tdf.independent;
import { println } from std.io;
import { stringOf } from std.text;
func main(): i32 {
    let s: String = stringOf("a");
    let t: String = $"{s}";
    s = stringOf("b");
    println(t);
    return 0;
}
"#;
    assert_output("independent", source, "a\n");
}

/// Each placeholder call evaluates exactly once.
#[test]
fn placeholder_calls_evaluate_once() {
    let source = r#"module spec.tdf.once;
import { println } from std.io;
let calls: i32 = 0;
func touch(): i32 {
    calls = calls + 1;
    return calls;
}
func main(): i32 {
    let t: String = $"{touch()}-{touch()}";
    println($"{t} {calls}");
    return 0;
}
"#;
    assert_output("once", source, "1-2 2\n");
}

/// The scored benchmark's loop keeps its exact checksum.
#[test]
fn the_benchmark_loop_checksum_is_unchanged() {
    let source = r#"module spec.tdf.benchloop;
import { println } from std.io;
import { toHex } from std.fmt;
func main(): i32 {
    let total: u32 = 0;
    let i: u32 = 0;
    while (i < 2000) {
        let s: String = $"token-{i}-{i * u32(3)}";
        let v: str = s;
        total = total + u32(len(v)) + u32(v[0]);
        i = i + 1;
    }
    println($"strings: {toHex(total)}");
    return 0;
}
"#;
    assert_output("benchloop", source, "strings: 0x3F9A6\n");
}

/// A boundary loop is sanitizer-clean.
#[test]
fn the_boundaries_are_sanitizer_clean() {
    let source = r#"module spec.tdf.sanitize;
import { println } from std.io;
func main(): i32 {
    let min: i32 = -2147483647 - 1;
    let max: u32 = 0xFFFFFFFF;
    let i: i32 = 0;
    while (i < 50) {
        let s: String = $"v={min} u={max} i={i}";
        println(s);
        i = i + 1;
    }
    return 0;
}
"#;
    let output = run("sanitize", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "sanitize: expected exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_sanitizer_report("sanitize", &output);
}

/// A reused owned source in a loop does not leak or double free.
#[test]
fn owned_placeholders_in_a_loop_are_leak_free() {
    let source = r#"module spec.tdf.ownedloop;
import { println } from std.io;
import { stringOf } from std.text;
func main(): i32 {
    let source: String = stringOf("xy");
    let i: i32 = 0;
    while (i < 50) {
        let line: String = $"[{source}]-{i}";
        println(line);
        i = i + 1;
    }
    println(source);
    return 0;
}
"#;
    let output = run("ownedloop", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "ownedloop: expected exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_sanitizer_report("ownedloop", &output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.lines().last().unwrap_or_default(),
        "xy",
        "ownedloop: the source was not reusable"
    );
}

/// A sibling placeholder that reassigns the source through a borrow must not
/// free the bytes the template copies. The copy is built before the sibling
/// can release the source.
#[test]
fn an_alias_sibling_does_not_free_a_placeholder_source() {
    let source = r#"module spec.tdf.alias;
import { println } from std.io;
import { stringOf } from std.text;
func reset(borrow s: String?): i32 {
    s = stringOf("z");
    return 1;
}
func main(): i32 {
    let s: String? = stringOf("abcdef");
    if (s != null) {
        println($"{s}{reset(s)}");
    }
    return 0;
}
"#;
    assert_output("alias", source, "abcdef1\n");
}

/// A null view is an empty byte piece, never the scalar zero.
#[test]
fn a_null_view_prints_empty() {
    let source = r#"module spec.tdf.nullview;
import { println } from std.io;
func main(): i32 {
    let s: String? = null;
    let v: str? = s;
    if (v == null) {
        println($"[{v}]");
    }
    return 0;
}
"#;
    assert_output("nullview", source, "[]\n");
}

/// The explicit converters are unchanged.
#[test]
fn explicit_conversions_keep_their_text() {
    let source = r#"module spec.tdf.explicit;
import { println } from std.io;
import { toDecimal, toHex } from std.fmt;
func main(): i32 {
    println(toDecimal(-42));
    println(toHex(0x3F9A6));
    return 0;
}
"#;
    assert_output("explicit", source, "-42\n0x3F9A6\n");
}
