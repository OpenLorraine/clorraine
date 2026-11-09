//! Behaviour oracle for module-level constant scalar array initializers.
//!
//! Every running case compiles a program, builds it on the host with a strict
//! C compiler, runs it, and compares the output. The cases cover the eligible
//! scalar types, the guard-free exclusions, the front-end-invalid shapes, and
//! the committed lookup benchmark. The emitted-shape pins live in the
//! implementation suite.
//!
//! The suite opts into both sanitizer legs. `RDC_ASAN` adds AddressSanitizer to
//! every child build; `RDC_UBSAN` adds UndefinedBehaviorSanitizer. `RDC_CLC`
//! lets the same cases run against a recorded pre-change compiler; a relative
//! value is resolved against the workspace root.

const _ASAN_SUITE: &str = "RDC_ASAN";
const _UBSAN_SUITE: &str = "RDC_UBSAN";

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
            "clc-constant-array-spec-{}-{tag}-{sequence}",
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
    let args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ];
    let output = run_clc(&args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&output_path).expect("read the emitted C");
    (scratch, emitted)
}

/// Emits a two-file program and returns its C and the scratch directory.
fn emit_pair(tag: &str, holder: &str, main: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|error| panic!("{tag}: write holder.clo: {error}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ];
    let output = run_clc(&args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&output_path).expect("read the emitted C");
    (scratch, emitted)
}

fn build_host(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut command = Command::new("cc");
    command.args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"]);
    if std::env::var("RDC_ASAN").is_ok() {
        command.args(["-fsanitize=address", "-fno-omit-frame-pointer"]);
    }
    if std::env::var("RDC_UBSAN").is_ok() {
        command.args(["-fsanitize=undefined", "-fno-sanitize-recover=all"]);
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

fn run_program(tag: &str, source: &str) -> (i32, String, String) {
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted);
    let output = Command::new(&program).output().expect("run the program");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn assert_output(tag: &str, source: &str, expected: &str) {
    let (code, stdout, stderr) = run_program(tag, source);
    assert_eq!(code, 0, "{tag}: exit {code}; stderr:\n{stderr}");
    assert_eq!(stdout, expected, "{tag}: unexpected stdout");
}

fn assert_output_pair(tag: &str, holder: &str, main: &str, expected: &str) {
    let (scratch, emitted) = emit_pair(tag, holder, main);
    let program = build_host(tag, &scratch, &emitted);
    let output = Command::new(&program).output().expect("run the program");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "{tag}: unexpected stdout"
    );
}

fn assert_fault(tag: &str, source: &str, code: &str) {
    let (exit, _stdout, stderr) = run_program(tag, source);
    assert_ne!(exit, 0, "{tag}: the program did not fault");
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
}

fn assert_compile_fail(tag: &str, source: &str, code: &str) {
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
    assert_ne!(output.status.code(), Some(0), "{tag}: the compiler accepted the shape");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    assert!(!output_path.is_file(), "{tag}: a rejected shape wrote C");
}

fn seq(start: i32, count: usize) -> String {
    (0..count)
        .map(|offset| (start + offset as i32).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn neg_seq(count: usize) -> String {
    (1..=count)
        .map(|value| format!("-{value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rep(value: &str, count: usize) -> String {
    std::iter::repeat(value)
        .take(count)
        .collect::<Vec<_>>()
        .join(", ")
}

fn bools(count: usize) -> String {
    (0..count)
        .map(|index| if index % 2 == 0 { "true" } else { "false" })
        .collect::<Vec<_>>()
        .join(", ")
}

fn first_then_zeros(first: &str, count: usize) -> String {
    format!("{first}, {}", rep("0", count - 1))
}

#[test]
fn should_emit_i32_values() {
    assert_output(
        "i32",
        &format!(
            r#"
module spec.ConstArrayI32;
import {{ println }} from std.io;
let t: i32[32] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_emit_negative_i32_values() {
    assert_output(
        "i32-neg",
        &format!(
            r#"
module spec.ConstArrayI32Neg;
import {{ println }} from std.io;
let t: i32[32] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = neg_seq(32)
        ),
        "sum: -528\n",
    );
}

#[test]
fn should_emit_u8_values() {
    assert_output(
        "u8",
        &format!(
            r#"
module spec.ConstArrayU8;
import {{ println }} from std.io;
let t: u8[64] = [{list}];
func main(): i32 {{
    let total: u32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ total = total + u32(t[i]); i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = seq(1, 64)
        ),
        "sum: 2080\n",
    );
}

#[test]
fn should_emit_u16_values() {
    assert_output(
        "u16",
        &format!(
            r#"
module spec.ConstArrayU16;
import {{ println }} from std.io;
let t: u16[32] = [{list}];
func main(): i32 {{
    let total: u32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + u32(t[i]); i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = rep("1000", 32)
        ),
        "sum: 32000\n",
    );
}

#[test]
fn should_emit_u32_max() {
    assert_output(
        "u32-max",
        &format!(
            r#"
module spec.ConstArrayU32Max;
import {{ println }} from std.io;
let t: u32[16] = [u32(4294967295), {zeros}];
func main(): i32 {{
    println($"value: {{t[0]}}");
    return 0;
}}
"#,
            zeros = rep("0", 15)
        ),
        "value: 4294967295\n",
    );
}

#[test]
fn should_emit_i8_values() {
    assert_output(
        "i8",
        &format!(
            r#"
module spec.ConstArrayI8;
import {{ println }} from std.io;
let t: i8[64] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ total = total + i32(t[i]); i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = rep("1", 64)
        ),
        "sum: 64\n",
    );
}

#[test]
fn should_emit_i16_values() {
    assert_output(
        "i16",
        &format!(
            r#"
module spec.ConstArrayI16;
import {{ println }} from std.io;
let t: i16[32] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + i32(t[i]); i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = rep("1", 32)
        ),
        "sum: 32\n",
    );
}

#[test]
fn should_emit_usize_values() {
    assert_output(
        "usize",
        &format!(
            r#"
module spec.ConstArrayUsize;
import {{ println }} from std.io;
let t: usize[32] = [{list}];
func main(): i32 {{
    let total: usize = 0;
    let i: usize = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = rep("1", 32)
        ),
        "sum: 32\n",
    );
}

#[test]
fn should_emit_isize_values() {
    assert_output(
        "isize",
        &format!(
            r#"
module spec.ConstArrayIsize;
import {{ println }} from std.io;
let t: isize[32] = [{list}];
func main(): i32 {{
    let total: isize = 0;
    let i: isize = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = rep("-1", 32)
        ),
        "sum: -32\n",
    );
}

#[test]
fn should_emit_bool_values() {
    assert_output(
        "bool",
        &format!(
            r#"
module spec.ConstArrayBool;
import {{ println }} from std.io;
let t: bool[64] = [{list}];
func main(): i32 {{
    let count: i32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ if (t[i]) {{ count = count + 1; }} i = i + 1; }}
    println($"true: {{count}}");
    return 0;
}}
"#,
            list = bools(64)
        ),
        "true: 32\n",
    );
}

#[test]
fn should_wrap_u8_casts() {
    assert_output(
        "u8-casts",
        &format!(
            r#"
module spec.ConstArrayU8Casts;
import {{ println }} from std.io;
let t: u8[64] = [u8(300), u8(255), u8(256), u8(0), {zeros}];
func main(): i32 {{
    let total: u32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ total = total + u32(t[i]); i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            zeros = rep("0", 60)
        ),
        "sum: 299\n",
    );
}

#[test]
fn should_wrap_i8_casts() {
    assert_output(
        "i8-casts",
        &format!(
            r#"
module spec.ConstArrayI8Casts;
import {{ println }} from std.io;
let t: i8[64] = [i8(200), i8(-128), i8(127), i8(0), {zeros}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ total = total + i32(t[i]); i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            zeros = rep("0", 60)
        ),
        "sum: -57\n",
    );
}

#[test]
fn should_keep_an_address_taken_table_writable() {
    assert_output(
        "address-taken",
        &format!(
            r#"
module spec.ConstArrayAddressTaken;
import {{ println }} from std.io;
let t: i32[32] = [{list}];
func main(): i32 {{
    let p: raw i32 = unsafe &t[0];
    unsafe {{ *p = 9; }}
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 536\n",
    );
}

#[test]
fn should_keep_cross_module_order_reading_the_table() {
    let holder = format!(
        r#"
module spec.ConstArrayHolder;
export let table: i32[32] = [{list}];
"#,
        list = seq(1, 32)
    );
    let main = r#"
module spec.ConstArrayMain;
import { println } from std.io;
import { table } from spec.ConstArrayHolder;
func main(): i32 {
    println($"pair: {table[0] + table[31]}");
    return 0;
}
"#;
    assert_output_pair("cross-module", &holder, main, "pair: 33\n");
}

#[test]
fn should_keep_a_later_initializer_after_the_table() {
    let holder = format!(
        r#"
module spec.ConstArrayLaterHolder;
export let table: i32[32] = [{list}];
export let later: i32 = table[2];
"#,
        list = seq(1, 32)
    );
    let main = r#"
module spec.ConstArrayLaterMain;
import { println } from std.io;
import { later } from spec.ConstArrayLaterHolder;
func main(): i32 {
    println($"later: {later}");
    return 0;
}
"#;
    assert_output_pair("later-init", &holder, main, "later: 3\n");
}

#[test]
fn should_keep_a_place_copy() {
    assert_output(
        "place-copy",
        &format!(
            r#"
module spec.ConstArrayPlaceCopy;
import {{ println }} from std.io;
let src: i32[32] = [{list}];
let copy: i32[32] = src;
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + copy[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_keep_a_fill() {
    assert_output(
        "fill",
        r#"
module spec.ConstArrayFill;
import { println } from std.io;
let t: i32[32] = [; 7];
func main(): i32 {
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) { total = total + t[i]; i = i + 1; }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 224\n",
    );
}

#[test]
fn should_keep_a_volatile_table() {
    assert_output(
        "volatile",
        &format!(
            r#"
module spec.ConstArrayVolatile;
import {{ println }} from std.io;
volatile let t: i32[32] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_keep_a_multi_extent_table() {
    assert_output(
        "multi-extent",
        r#"
module spec.ConstArrayMulti;
import { println } from std.io;
let t: i32[4][4] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]];
func main(): i32 {
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 4) {
        let j: i32 = 0;
        while (j < 4) { total = total + t[i][j]; j = j + 1; }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 136\n",
    );
}

#[test]
fn should_keep_a_struct_table() {
    assert_output(
        "struct",
        &format!(
            r#"
module spec.ConstArrayStruct;
import {{ println }} from std.io;
struct S {{ v: i32; }}
let t: S[32] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i].v; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = rep("S { v: 1 }", 32)
        ),
        "sum: 32\n",
    );
}

#[test]
fn should_keep_a_float_table() {
    assert_output(
        "float",
        &format!(
            r#"
module spec.ConstArrayFloat;
import {{ println }} from std.io;
let t: f32[32] = [{list}];
func main(): i32 {{
    let total: f32 = 0.0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{i32(total)}}");
    return 0;
}}
"#,
            list = rep("f32(1.5)", 32)
        ),
        "sum: 48\n",
    );
}

#[test]
fn should_keep_a_short_list() {
    assert_output(
        "short-list",
        r#"
module spec.ConstArrayShort;
import { println } from std.io;
let t: i32[32] = [1, 2];
func main(): i32 {
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) { total = total + t[i]; i = i + 1; }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 3\n",
    );
}

#[test]
fn should_keep_an_empty_list() {
    assert_output(
        "empty-list",
        r#"
module spec.ConstArrayEmpty;
import { println } from std.io;
let t: i32[32] = [];
func main(): i32 {
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) { total = total + t[i]; i = i + 1; }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_keep_a_division_guard() {
    assert_fault(
        "division-guard",
        r#"
module spec.ConstArrayDivisionGuard;
let min: i32 = i32(u32(0x80000000));
let neg: i32 = -1;
let t: i32[32] = [min / neg, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
func main(): i32 { return t[0]; }
"#,
        "E11210",
    );
}

#[test]
fn should_keep_an_in_range_float_cast() {
    assert_output(
        "float-cast-in-range",
        &format!(
            r#"
module spec.ConstArrayFloatCast;
import {{ println }} from std.io;
let t: i32[32] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = first_then_zeros("i32(f32(1.5))", 32)
        ),
        "sum: 1\n",
    );
}

#[test]
fn should_keep_an_out_of_range_float_cast() {
    assert_fault(
        "float-cast-out-of-range",
        &format!(
            r#"
module spec.ConstArrayFloatOut;
let t: i32[32] = [{list}];
func main(): i32 {{ return t[0]; }}
"#,
            list = first_then_zeros("i32(f32(3.0e9))", 32)
        ),
        "E11212",
    );
}

#[test]
fn should_not_evaluate_a_short_circuited_call() {
    assert_output(
        "short-circuit-call",
        &format!(
            r#"
module spec.ConstArrayShortCall;
import {{ println }} from std.io;
let calls: i32 = 0;
func f(): bool {{ calls = calls + 1; return true; }}
let t: bool[64] = [false && f(), {list}];
func main(): i32 {{
    let count: i32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ if (t[i]) {{ count = count + 1; }} i = i + 1; }}
    println($"calls: {{calls}} true: {{count}}");
    return 0;
}}
"#,
            list = rep("false", 63)
        ),
        "calls: 0 true: 0\n",
    );
}

#[test]
fn should_not_read_a_short_circuited_place() {
    assert_output(
        "short-circuit-place",
        &format!(
            r#"
module spec.ConstArrayShortPlace;
import {{ println }} from std.io;
let x: i32 = 1;
let t: bool[64] = [false && (x > 0), {list}];
func main(): i32 {{
    let count: i32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ if (t[i]) {{ count = count + 1; }} i = i + 1; }}
    println($"true: {{count}}");
    return 0;
}}
"#,
            list = rep("false", 63)
        ),
        "true: 0\n",
    );
}

#[test]
fn should_evaluate_a_call_element_once() {
    assert_output(
        "call-element",
        &format!(
            r#"
module spec.ConstArrayCall;
import {{ println }} from std.io;
let calls: i32 = 0;
func f(): i32 {{ calls = calls + 1; return 7; }}
let t: i32[32] = [f(), {list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}} calls: {{calls}}");
    return 0;
}}
"#,
            list = rep("0", 31)
        ),
        "sum: 7 calls: 1\n",
    );
}

#[test]
fn should_evaluate_a_conditional_element_once() {
    assert_output(
        "conditional-element",
        &format!(
            r#"
module spec.ConstArrayConditional;
import {{ println }} from std.io;
let c: bool = true;
let t: i32[32] = [c ? 1 : 2, {list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = rep("0", 31)
        ),
        "sum: 1\n",
    );
}

#[test]
fn should_keep_a_remainder_guard() {
    assert_fault(
        "remainder-guard",
        r#"
module spec.ConstArrayRemainderGuard;
let min: i32 = i32(u32(0x80000000));
let neg: i32 = -1;
let t: i32[32] = [min % neg, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
func main(): i32 { return t[0]; }
"#,
        "E11210",
    );
}

#[test]
fn should_keep_a_below_threshold_table() {
    assert_output(
        "below-threshold",
        &format!(
            r#"
module spec.ConstArraySmall;
import {{ println }} from std.io;
let t: i32[8] = [{list}];
func main(): i32 {{
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {{ total = total + t[i]; i = i + 1; }}
    println($"sum: {{total}}");
    return 0;
}}
"#,
            list = seq(1, 8)
        ),
        "sum: 36\n",
    );
}

#[test]
fn should_keep_a_view_table() {
    assert_output(
        "view-table",
        &format!(
            r#"
module spec.ConstArrayView;
import {{ println }} from std.io;
let t: str[16] = [{list}];
func main(): i32 {{
    let total: usize = 0;
    let i: usize = 0;
    while (i < 16) {{ total = total + len(t[i]); i = i + 1; }}
    println($"len: {{total}}");
    return 0;
}}
"#,
            list = rep("\"aa\"", 16)
        ),
        "len: 32\n",
    );
}

#[test]
fn should_emit_i32_extremes() {
    assert_output(
        "i32-extremes",
        &format!(
            r#"
module spec.ConstArrayExtremes;
import {{ println }} from std.io;
let t: i32[32] = [2147483647, -2147483647, {zeros}];
func main(): i32 {{
    println($"max: {{t[0]}} min: {{t[1]}}");
    return 0;
}}
"#,
            zeros = rep("0", 30)
        ),
        "max: 2147483647 min: -2147483647\n",
    );
}

#[test]
fn should_emit_usize_and_isize_casts() {
    assert_output(
        "usize-isize-casts",
        &format!(
            r#"
module spec.ConstArrayWidthCasts;
import {{ println }} from std.io;
let a: usize[32] = [usize(7), {zeros}];
let b: isize[32] = [isize(-7), {zeros}];
func main(): i32 {{
    println($"u: {{a[0]}} i: {{b[0]}}");
    return 0;
}}
"#,
            zeros = rep("0", 31)
        ),
        "u: 7 i: -7\n",
    );
}

#[test]
fn should_emit_a_bool_cast() {
    assert_output(
        "bool-cast",
        &format!(
            r#"
module spec.ConstArrayBoolCast;
import {{ println }} from std.io;
let t: bool[64] = [bool(1), {list}];
func main(): i32 {{
    let count: i32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ if (t[i]) {{ count = count + 1; }} i = i + 1; }}
    println($"true: {{count}}");
    return 0;
}}
"#,
            list = rep("false", 63)
        ),
        "true: 1\n",
    );
}

#[test]
fn should_reject_a_zero_extent() {
    assert_compile_fail(
        "zero-extent",
        r#"
module spec.ConstArrayZero;
let t: i32[0] = [];
func main(): i32 { return 0; }
"#,
        "E10226",
    );
}

#[test]
fn should_reject_a_slice_initializer() {
    assert_compile_fail(
        "slice",
        r#"
module spec.ConstArraySlice;
let t: i32[] = [1, 2, 3];
func main(): i32 { return 0; }
"#,
        "E10200",
    );
}

#[test]
fn should_reject_a_handle_array() {
    assert_compile_fail(
        "handle-array",
        r#"
module spec.ConstArrayHandle;
import { String, stringOf } from std.text;
let t: String[2] = [stringOf("a"), stringOf("b")];
func main(): i32 { return 0; }
"#,
        "E11106",
    );
}

#[test]
fn should_reject_a_const_array() {
    assert_compile_fail(
        "const-array",
        &format!(
            r#"
module spec.ConstArrayConst;
const T: i32[32] = [{list}];
func main(): i32 {{ return 0; }}
"#,
            list = seq(1, 32)
        ),
        "E10224",
    );
}

#[test]
fn should_reject_an_out_of_range_literal() {
    assert_compile_fail(
        "out-of-range",
        &format!(
            r#"
module spec.ConstArrayOutOfRange;
let t: i8[64] = [200, {zeros}];
func main(): i32 {{ return 0; }}
"#,
            zeros = rep("0", 63)
        ),
        "E10202",
    );
}

#[test]
fn should_print_the_lookup_benchmark() {
    assert_output(
        "lookup",
        include_str!("../../benchmarks/lookup/program.clo"),
        "lookup: 134217728\n",
    );
}
