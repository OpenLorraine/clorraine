//! Host specification tests for the integer `String` formatters.
//!
//! The cases cover every boundary of the signed and unsigned 32-bit decimal
//! output, the template and explicit-conversion paths that reach the
//! converters, the benchmark's own loop shape, and the runtime contract that
//! integer formatting no longer calls the C library formatter.
//!
//! The suite opts into both sanitizer legs. `RDC_ASAN` adds AddressSanitizer
//! to every child build and runs with leak detection; `RDC_UBSAN` adds
//! UndefinedBehaviorSanitizer with recovery disabled. `RDC_CLC` lets the same
//! cases run against a recorded pre-change compiler; a relative value is
//! resolved against the workspace root.

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
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-string-format-spec-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The compiler under test. A relative `RDC_CLC` is resolved against the
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

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(clc_path())
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Emits one host-profile program and returns its C and scratch directory.
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

/// Builds one emitted program with the strict host flags and the active
/// sanitizer legs.
///
/// `stub` adds a C source to the link; `wrap_snprintf` adds the GNU linker's
/// `--wrap` mapping so a test can count formatter calls. The sanitizer flags
/// match the repository's `make asan` and `make ubsan` runners.
fn build_host(
    tag: &str,
    scratch: &ScratchDir,
    emitted: &str,
    stub: Option<&str>,
    wrap_snprintf: bool,
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
    if wrap_snprintf {
        command.arg("-Wl,--wrap=snprintf");
    }
    let built = command.output().expect("spawn cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Emits, builds, and runs one host program, returning its output.
fn run(tag: &str, source: &str) -> Output {
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted, None, false);
    let mut command = Command::new(&program);
    if std::env::var("RDC_ASAN").is_ok() {
        command.env("ASAN_OPTIONS", "detect_leaks=1:symbolize=0");
    }
    command.output().expect("run the program")
}

/// Asserts one program prints exactly `expected` and exits 0.
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

/// Asserts a run reports no AddressSanitizer error. The check is meaningful
/// only when `RDC_ASAN` is set; a leak or use-after-free aborts the exit first.
fn assert_no_sanitizer_report(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer") && !stderr.contains("runtime error:"),
        "{tag}: the sanitizer reported an error; stderr:\n{stderr}"
    );
}

/// The signed and unsigned decimal boundaries, plus the narrow integer
/// widening and the explicit converters.
#[test]
fn boundaries_print_the_decimal_text() {
    let source = r#"module spec.sif.boundaries;

import { println } from std.io;
import { toDecimal, toHex } from std.fmt;

func main(): i32 {
    let zero: i32 = 0;
    let one: i32 = 1;
    let minusOne: i32 = -1;
    let max: i32 = 2147483647;
    let min: i32 = -2147483647 - 1;
    let minusMax: i32 = -2147483647;
    let uzero: u32 = 0;
    let umax: u32 = 0xFFFFFFFF;
    let uhigh: u32 = 2147483648;
    let usizeMax: usize = 0xFFFFFFFF;
    let narrow: i8 = -128;
    let byte: u8 = 255;
    println($"{zero}|{one}|{minusOne}|{max}|{min}|{minusMax}");
    println($"{uzero}|{umax}|{uhigh}|{usizeMax}");
    println($"{narrow}|{byte}");
    println(toDecimal(-42));
    println(toHex(0x3F9A6));
    return 0;
}
"#;
    assert_output(
        "boundaries",
        source,
        "0|1|-1|2147483647|-2147483648|-2147483647\n\
         0|4294967295|2147483648|4294967295\n\
         -128|255\n\
         -42\n\
         0x3F9A6\n",
    );
}

/// Adjacent placeholders print digits only; no separator is inserted.
#[test]
fn adjacent_placeholders_insert_no_separators() {
    let source = r#"module spec.sif.adjacent;

import { println } from std.io;

func main(): i32 {
    let a: i32 = 1;
    let b: u32 = 2;
    let c: i32 = 3;
    println($"{a}{b}{c}");
    return 0;
}
"#;
    assert_output("adjacent", source, "123\n");
}

/// Literal text around and between placeholders is preserved.
#[test]
fn literals_between_placeholders_are_preserved() {
    let source = r#"module spec.sif.literals;

import { println } from std.io;

func main(): i32 {
    let a: i32 = -7;
    let b: i32 = 9;
    println($"x={a},y={b}!");
    return 0;
}
"#;
    assert_output("literals", source, "x=-7,y=9!\n");
}

/// An empty `str` placeholder contributes no bytes, and an owned `String`
/// placeholder is borrowed rather than moved.
#[test]
fn string_placeholders_keep_their_behavior() {
    let source = r#"module spec.sif.strings;

import { println } from std.io;
import { stringOf } from std.text;

func main(): i32 {
    let empty: str = "";
    let owned: String = stringOf("abc");
    let wrapped: String = $"[{owned}]";
    println($"[{empty}]");
    println($"{owned}{wrapped}");
    return 0;
}
"#;
    assert_output("strings", source, "[]\nabc[abc]\n");
}

/// Boolean and float placeholders keep their documented text.
#[test]
fn bool_and_float_placeholders_are_unchanged() {
    let source = r#"module spec.sif.other;

import { println } from std.io;

func main(): i32 {
    let yes: bool = true;
    let no: bool = false;
    let half: f64 = 1.5;
    let small: f64 = 0.000123456789;
    let large: f64 = 1234567.0;
    println($"{yes} {no}");
    println($"{half} {small} {large}");
    return 0;
}
"#;
    assert_output(
        "other",
        source,
        "true false\n1.5 0.000123457 1.23457e+06\n",
    );
}

/// The scored benchmark's loop shape keeps its exact checksum.
#[test]
fn the_benchmark_loop_checksum_is_unchanged() {
    let source = r#"module spec.sif.benchloop;

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

/// The wrapped libc formatter counts every `snprintf` call. Integer
/// formatting through templates and explicit conversions must make none.
#[test]
fn integer_formatting_makes_no_snprintf_calls() {
    let source = r#"module spec.sif.calls;

import { println } from std.io;
import { toDecimal, toHex } from std.fmt;

ffi testlib {
    func cl_test_snprintf_calls(): i32;
}

func main(): i32 {
    let a: i32 = -2147483647 - 1;
    let b: u32 = 0xFFFFFFFF;
    let s: String = $"a={a} b={b}";
    let t: String = toDecimal(a);
    let u: String = toDecimal(7);
    let used: usize = len(s) + len(t) + len(u);
    if (used == 0) {
        return 1;
    }
    println($"hex={toHex(u32(cl_test_snprintf_calls()))}");
    return 0;
}
"#;
    let stub = r#"#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>

static int cl_test_calls;

int cl_test_snprintf_calls(void)
{
    return cl_test_calls;
}

int __wrap_snprintf(char *buffer, size_t size, const char *format, ...)
{
    va_list args;
    va_start(args, format);
    int written = vsnprintf(buffer, size, format, args);
    va_end(args);
    cl_test_calls++;
    return written;
}
"#;
    let (scratch, emitted) = emit("calls", source);
    let program = build_host("calls", &scratch, &emitted, Some(stub), true);
    let output = Command::new(&program).output().expect("run the program");
    assert_eq!(
        output.status.code(),
        Some(0),
        "calls: expected exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hex=0x0\n",
        "calls: integer formatting called the C library formatter"
    );
}

/// A boundary program is clean under the active sanitizers.
#[test]
fn the_integer_boundaries_are_sanitizer_clean() {
    let source = r#"module spec.sif.sanitize;

import { println } from std.io;

func main(): i32 {
    let min: i32 = -2147483647 - 1;
    let max: u32 = 0xFFFFFFFF;
    let i: i32 = 0;
    while (i < 100) {
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().next().unwrap_or_default();
    assert_eq!(first, "v=-2147483648 u=4294967295 i=0", "sanitize: first line");
}

/// A loop with an owned placeholder that is reused after the template does
/// not leak or double free the source.
#[test]
fn owned_placeholders_in_a_loop_are_leak_free() {
    let source = r#"module spec.sif.ownedloop;

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
