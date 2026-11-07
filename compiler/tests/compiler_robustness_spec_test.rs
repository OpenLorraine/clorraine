//! Behavior oracle for the compiler robustness closeout.
//!
//! The cases derive from the requirements: fill operand guards, narrow
//! arithmetic wrapping, the inline shift divisor, `f32` folding, and enum
//! member values and conversions. The emitted shapes are pinned separately in
//! the implementation suite.
//!
//! The suite opts into both sanitizer legs. `RDC_ASAN` adds AddressSanitizer to
//! every child build; `RDC_UBSAN` adds UndefinedBehaviorSanitizer. The
//! `-no-pie` link is supplied by the sanitizer runner's `cc` wrapper.

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
            "clc-robustness-{}-{tag}-{sequence}",
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

/// The compiler under test. `RDC_CLC` lets a recorded base binary run the same
/// case; the built binary is the default.
fn clc_path() -> String {
    std::env::var("RDC_CLC").unwrap_or_else(|_| env!("CARGO_BIN_EXE_clc").to_owned())
}

fn run_clc(args: &[String]) -> Output {
    Command::new(clc_path())
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Emits a host program and returns its C and the scratch directory.
fn emit_host(tag: &str, source: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write the source: {error}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    (scratch, emitted)
}

/// Emits a program for one profile and returns its C.
fn emit_profile(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write the source: {error}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).expect("read the emitted C")
}

/// Adds the active sanitizer flags to a host C build command.
fn sanitizer_flags() -> Vec<&'static str> {
    let mut flags = Vec::new();
    if std::env::var_os("RDC_ASAN").is_some() {
        flags.extend(["-fsanitize=address", "-fno-omit-frame-pointer"]);
    }
    if std::env::var_os("RDC_UBSAN").is_some() {
        flags.extend(["-fsanitize=undefined", "-fno-sanitize-recover=all"]);
    }
    flags
}

/// Builds emitted C with the strict host flags and the active sanitizers.
fn build_host(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut command = Command::new("cc");
    command.args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"]);
    command.args(sanitizer_flags());
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

/// Runs a program and returns its exit code, stdout, and stderr.
fn run_program(tag: &str, source: &str) -> (i32, String, String) {
    let (scratch, emitted) = emit_host(tag, source);
    let program = build_host(tag, &scratch, &emitted);
    let output = Command::new(&program).output().expect("run the program");
    let code = output
        .status
        .code()
        .unwrap_or_else(|| signal_code(output.status));
    (
        code,
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A signal death is reported as a negative number, matching the shell's
/// negative-signal convention without depending on the platform's signal
/// values.
fn signal_code(status: std::process::ExitStatus) -> i32 {
    if status.success() {
        0
    } else {
        -1
    }
}

fn assert_output(tag: &str, source: &str, expected: &str) {
    let (code, stdout, stderr) = run_program(tag, source);
    assert_eq!(code, 0, "{tag}: exit {code}; stderr:\n{stderr}");
    assert_eq!(stdout, expected, "{tag}: unexpected stdout");
}

/// Emits several source files and returns the C for the root file.
fn emit_files(tag: &str, files: &[(&str, &str)], root_name: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let mut paths = Vec::new();
    for (name, source) in files {
        let path = scratch.path().join(name);
        std::fs::write(&path, source).unwrap_or_else(|error| panic!("{tag}: write {name}: {error}"));
        paths.push(path);
    }
    let c_path = scratch.path().join("out.c");
    let root = paths
        .iter()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some(root_name))
        .unwrap_or_else(|| panic!("{tag}: root file {root_name} is missing"));
    let mut args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        root.display().to_string(),
    ];
    for path in &paths {
        if path != root {
            args.push(path.display().to_string());
        }
    }
    let output = run_clc(&args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    (scratch, emitted)
}

fn assert_output_files(tag: &str, files: &[(&str, &str)], root: &str, expected: &str) {
    let (scratch, emitted) = emit_files(tag, files, root);
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

/// Compiles a program that must be rejected and returns its stderr.
fn compile_error(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write the source: {error}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert!(
        !output.status.success(),
        "{tag}: the program was accepted but must be rejected"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Asserts a compile error carries the registered E10202 text.
fn assert_e10202(tag: &str, stderr: &str, target: &str) {
    assert!(
        stderr.contains(&format!("error E10202: literal does not fit '{target}'")),
        "{tag}: expected the registered E10202 message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("help: use a wider type or a hex/binary form"),
        "{tag}: expected the registered E10202 help; stderr:\n{stderr}"
    );
}

fn assert_trap(tag: &str, source: &str, first_line: &str) {
    let (code, _stdout, stderr) = run_program(tag, source);
    assert_ne!(code, 0, "{tag}: a trap must exit nonzero");
    let first = stderr.lines().next().unwrap_or_default();
    assert_eq!(first, first_line, "{tag}: unexpected first stderr line");
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Sanitizer activation controls
// ---------------------------------------------------------------------------

#[test]
fn should_activate_the_sanitizer_suite_builds() {
    let asan = std::env::var_os("RDC_ASAN").is_some();
    let ubsan = std::env::var_os("RDC_UBSAN").is_some();
    if !asan && !ubsan {
        return;
    }
    let scratch = ScratchDir::new("sanitizer-activation");
    let mut flags: Vec<&str> = vec!["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"];
    flags.extend(sanitizer_flags());
    flags.push("-no-pie");

    let mut check = |name: &str, source: &str, needle: &str| {
        let c_path = scratch.path().join(format!("{name}.c"));
        std::fs::write(&c_path, source).expect("write the activation probe");
        let program = scratch.path().join(name);
        let mut build = Command::new("cc");
        build.args(&flags);
        build.args([
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("C path"),
        ]);
        let built = build.output().expect("spawn cc");
        assert!(
            built.status.success(),
            "{name}: the activation probe did not build; stderr:\n{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let run = Command::new(&program)
            .env("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")
            .env("UBSAN_OPTIONS", "print_stacktrace=0")
            .output()
            .expect("run the activation probe");
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert!(
            stderr.contains(needle),
            "{name}: the sanitizer did not report {needle:?}; stderr:\n{stderr}"
        );
    };

    if asan {
        check(
            "asan-activation",
            r#"int main(void) { int a[2] = {0, 0}; volatile int i = 3; return a[i]; }"#,
            "AddressSanitizer",
        );
    }
    if ubsan {
        check(
            "ubsan-activation",
            r#"#include <limits.h>
int main(void) { volatile int value = INT_MAX; value = value + 1; return value == 0; }"#,
            "runtime error:",
        );
    }
}

// ---------------------------------------------------------------------------
// D1 — fill operand guards
// ---------------------------------------------------------------------------

#[test]
fn should_guard_a_runtime_index_in_a_pure_fill() {
    assert_trap(
        "fill-bounds",
        r#"
module fuzz.FillBounds;
import { println } from std.io;
func main(): i32 {
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let a: u8[8] = [; src[i]];
    println($"a: {i32(a[0])}");
    return 0;
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_trap_on_a_runtime_zero_divisor_in_a_pure_fill() {
    assert_trap(
        "fill-divisor",
        r#"
module fuzz.FillDivisor;
func main(): i32 {
    let d: u8 = 0;
    let a: u8[4] = [; u8(10) / d];
    return i32(a[0]);
}
"#,
        "error E11210: integer division by zero",
    );
}

#[test]
fn should_trap_on_an_out_of_range_float_fill() {
    assert_trap(
        "fill-float",
        r#"
module fuzz.FillFloat;
func main(): i32 {
    let f: f32 = f32(300.0);
    let a: i8[4] = [; i8(f)];
    return i32(a[0]);
}
"#,
        "error E11212: float-to-integer conversion out of range",
    );
}

#[test]
fn should_guard_a_struct_literal_array_field() {
    assert_trap(
        "struct-fill-bounds",
        r#"
module fuzz.StructFillBounds;
struct S { a: u8[4]; }
func main(): i32 {
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let s: S = S { a: [; src[i]] };
    return i32(s.a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_guard_a_module_conditional_struct_array_field() {
    assert_trap(
        "module-cond-struct",
        r#"
module fuzz.ModuleCondStruct;
struct S { a: u8[4]; }
let flag: i32 = 1;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let s: S = S { a: flag != 0 ? [; src[i]] : [; src[0]] };
func main(): i32 {
    return i32(s.a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_keep_the_scalar_conditional_fill_guard() {
    // The scalar conditional arm is already guarded by the value renderer; this
    // is a control that the fill lowering does not move the guard across the
    // condition.
    assert_trap(
        "scalar-cond-fill",
        r#"
module fuzz.ScalarCondFill;
let flag: i32 = 1;
func main(): i32 {
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let a: u8[4] = [; flag != 0 ? src[i] : src[0]];
    return i32(a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_guard_a_conditional_struct_array_field() {
    assert_trap(
        "local-cond-struct",
        r#"
module fuzz.LocalCondStruct;
struct S { a: u8[4]; }
func main(): i32 {
    let flag: i32 = 1;
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let s: S = S { a: flag != 0 ? [; src[i]] : [; src[0]] };
    return i32(s.a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_guard_a_module_array_fill() {
    assert_trap(
        "module-array-bounds",
        r#"
module fuzz.ModuleArrayBounds;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let a: u8[8] = [; src[i]];
func main(): i32 {
    return i32(a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_guard_a_module_array_of_structs_conditional_field() {
    assert_trap(
        "module-struct-array",
        r#"
module fuzz.ModuleStructArray;
struct S { a: u8[4]; }
let flag: i32 = 1;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let s: S[2] = [; S { a: flag != 0 ? [; src[i]] : [; src[0]] }];
func main(): i32 {
    return i32(s[0].a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_guard_a_fill_condition() {
    assert_trap(
        "fill-condition",
        r#"
module fuzz.FillCondition;
func main(): i32 {
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let a: u8[4] = [; src[i] != 0 ? 1 : 2];
    return i32(a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_guard_a_module_struct_literal_array_field() {
    assert_trap(
        "module-struct-field",
        r#"
module fuzz.ModuleStructField;
struct S { a: u8[4]; }
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let s: S = S { a: [; src[i]] };
func main(): i32 {
    return i32(s.a[0]);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_keep_a_module_function_named_like_a_type() {
    assert_output(
        "enum-func-shadow",
        r#"
module fuzz.EnumFuncShadow;
import { println } from std.io;
func String(x: i32): i32 {
    return x + 5;
}
func main(): i32 {
    let n: i32 = String(2);
    println($"v: {n}");
    return 0;
}
"#,
        "v: 7\n",
    );
}

#[test]
fn should_keep_an_imported_function_call_before_an_enum_type() {
    assert_output_files(
        "enum-import-shadow",
        &[
            (
                "type_holder.clo",
                r#"
module fuzz.EnumTypeHolder;
export enum E: u8 { A }
"#,
            ),
            (
                "fn_holder.clo",
                r#"
module fuzz.EnumFnHolder;
export func E(x: i32): i32 { return x + 40; }
"#,
            ),
            (
                "main.clo",
                r#"
module fuzz.EnumImportShadow;
import { println } from std.io;
import { E } from fuzz.EnumFnHolder;
func main(): i32 {
    println($"v: {E(2)}");
    return 0;
}
"#,
            ),
        ],
        "main.clo",
        "v: 42\n",
    );
}

#[test]
fn should_accept_a_backward_enum_member_reference() {
    assert_output(
        "enum-backward",
        r#"
module fuzz.EnumBackward;
import { println } from std.io;
enum Flags: u8 { A = 1, B = 2, C = Flags.A | Flags.B }
func main(): i32 {
    println($"v: {i32(Flags.C)}");
    return 0;
}
"#,
        "v: 3\n",
    );
}

#[test]
fn should_fold_a_cross_module_enum_member_with_a_declaring_const() {
    assert_output_files(
        "enum-cross-const",
        &[
            (
                "holder.clo",
                r#"
module fuzz.EnumConstHolder;
export const BASE: u8 = 4;
export enum E: u8 { A = BASE + 1, B }
"#,
            ),
            (
                "main.clo",
                r#"
module fuzz.EnumCrossConst;
import { println } from std.io;
import { E } from fuzz.EnumConstHolder;
const N: usize = usize(i32(E.A)) + 1;
func main(): i32 {
    let a: u8[N] = [; 0];
    println($"n: {i32(E.A)} {i32(E.B)} {sizeof(u8[N])} {i32(N)}");
    return 0;
}
"#,
            ),
        ],
        "main.clo",
        "n: 5 6 6 6\n",
    );
}

#[test]
fn should_keep_the_abi_sizeof_for_an_enum_member() {
    assert_output(
        "enum-sizeof",
        r#"
module fuzz.EnumSizeof;
import { println } from std.io;
struct S { a: u8; b: u16; }
enum E: u8 { A = u8(sizeof(S)) }
const N: usize = usize(i32(E.A));
func main(): i32 {
    let a: u8[N] = [; 0];
    println($"n: {i32(E.A)} {sizeof(u8[N])} {i32(N)} {sizeof(S)} {i32(a[0])}");
    return 0;
}
"#,
        "n: 4 4 4 4 0\n",
    );
}

#[test]
fn should_reject_an_enum_member_that_overflows_through_sizeof() {
    let stderr = compile_error(
        "enum-sizeof-overflow",
        r#"
module fuzz.EnumSizeofOverflow;
struct S { a: u8; b: u16; }
enum E: u8 { A = u8(251 + sizeof(S)), B }
func main(): i32 {
    return i32(E.B);
}
"#,
    );
    assert_e10202("enum-sizeof-overflow", &stderr, "u8");
}

#[test]
fn should_fold_a_cross_module_const_chain() {
    assert_output_files(
        "enum-cross-chain",
        &[
            (
                "holder.clo",
                r#"
module fuzz.EnumChainHolder;
export const BASE: u8 = 4;
export const NEXT: u8 = BASE + 1;
export enum E: u8 { A = NEXT + 1, B }
"#,
            ),
            (
                "main.clo",
                r#"
module fuzz.EnumChain;
import { println } from std.io;
import { E } from fuzz.EnumChainHolder;
const N: usize = usize(i32(E.A)) + 1;
func main(): i32 {
    let a: u8[N] = [; 0];
    println($"n: {i32(E.A)} {i32(E.B)} {sizeof(u8[N])} {i32(N)}");
    return 0;
}
"#,
            ),
        ],
        "main.clo",
        "n: 6 7 7 7\n",
    );
}

#[test]
fn should_keep_a_module_global_named_like_a_type() {
    assert_output_files(
        "enum-global-shadow",
        &[
            (
                "holder.clo",
                r#"
module fuzz.EnumGlobalHolder;
export enum E: u8 { A }
"#,
            ),
            (
                "main.clo",
                r#"
module fuzz.EnumGlobalShadow;
import { println } from std.io;
func twice(x: i32): i32 {
    return x * 2;
}
let E: func(i32): i32 = twice;
func main(): i32 {
    println($"v: {E(2)}");
    return 0;
}
"#,
            ),
        ],
        "main.clo",
        "v: 4\n",
    );
}

#[test]
fn should_convert_an_enum_in_a_module_initializer() {
    assert_output(
        "enum-module-init-cast",
        r#"
module fuzz.EnumModuleInitCast;
import { println } from std.io;
enum Direction: u8 { UP, DOWN, LEFT = 10, RIGHT }
let V: Direction = Direction(2);
func main(): i32 {
    println($"v: {i32(V)}");
    return 0;
}
"#,
        "v: 2\n",
    );
}

#[test]
fn should_reject_a_constant_enum_conversion() {
    let stderr = compile_error(
        "enum-constant-cast",
        r#"
module fuzz.EnumConstantCast;
enum E: u8 { A = 2 }
const N: usize = usize(E(2)) + 1;
func main(): i32 {
    let a: u8[N] = [; 0];
    return i32(a[0]);
}
"#,
    );
    assert!(
        stderr.contains("error E10224: expression is not constant"),
        "enum-constant-cast: expected E10224; stderr:\n{stderr}"
    );
}

#[test]
fn should_reject_a_forward_enum_reference() {
    let stderr = compile_error(
        "enum-forward",
        r#"
module fuzz.EnumForward;
enum E: u8 { A = E.B, B = 2 }
func main(): i32 {
    return i32(E.A);
}
"#,
    );
    assert!(
        stderr.contains("error E10224: expression is not constant"),
        "enum-forward: expected E10224; stderr:\n{stderr}"
    );
}

#[test]
fn should_reject_a_const_mediated_sizeof_enum_overflow() {
    let stderr = compile_error(
        "enum-sizeof-const",
        r#"
module fuzz.EnumSizeofConst;
struct S { a: u8; b: u16; }
const K: usize = sizeof(S);
enum E: u8 { A = 252 + K }
func main(): i32 {
    return i32(E.A);
}
"#,
    );
    assert_e10202("enum-sizeof-const", &stderr, "u8");
}

#[test]
fn should_check_an_explicit_member_before_a_later_non_integer_member() {
    let stderr = compile_error(
        "enum-explicit-before-bool",
        r#"
module fuzz.EnumExplicitBeforeBool;
enum E: u8 { A = 300, B = true }
func main(): i32 {
    return i32(E.A);
}
"#,
    );
    assert_e10202("enum-explicit-before-bool", &stderr, "u8");
}

#[test]
fn should_report_an_out_of_range_backward_enum_reference() {
    let stderr = compile_error(
        "enum-backward-overflow",
        r#"
module fuzz.EnumBackwardOverflow;
enum E: u8 { A = 300, B = E.A + 1 }
func main(): i32 {
    return i32(E.B);
}
"#,
    );
    assert_e10202("enum-backward-overflow", &stderr, "u8");
    assert!(
        !stderr.contains("E10224"),
        "enum-backward-overflow: an out-of-range prefix must not cascade as not-constant; stderr:\n{stderr}"
    );
}

#[test]
fn should_keep_an_unselected_conditional_arm_lazy() {
    assert_output(
        "module-struct-lazy",
        r#"
module fuzz.ModuleStructLazy;
import { println } from std.io;
struct S { a: u8[4]; }
let flag: i32 = 0;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let s: S = S { a: flag != 0 ? [; src[i]] : [; src[0]] };
func main(): i32 {
    println($"v: {i32(s.a[0])}");
    return 0;
}
"#,
        "v: 1\n",
    );
}

// ---------------------------------------------------------------------------
// D2/D4 — narrow arithmetic wrapping
// ---------------------------------------------------------------------------

#[test]
fn should_wrap_signed_narrow_arithmetic_without_int_overflow() {
    assert_output(
        "narrow-signed",
        r#"
module fuzz.NarrowSigned;
import { println } from std.io;
const SC1: i32 = i32(i16(-1) * i16(-1));
const SC2: i32 = i32(i16(-24297) * i16(-13051));
func mul(a: i16, b: i16): i16 {
    return a * b;
}
func main(): i32 {
    let a: i16 = i16(-1);
    let b: i16 = i16(-1);
    let c: i16 = i16(-24297);
    let d: i16 = i16(-13051);
    println($"const: {SC1} {SC2}");
    println($"v: {i32(a * b)} {i32(c * d)} {i32(mul(c, d))} {i32(i16(32767) * i16(2))}");
    return 0;
}
"#,
        "const: 1 -28557\nv: 1 -28557 -28557 -2\n",
    );
}

#[test]
fn should_wrap_narrow_arithmetic_in_inline_and_const_positions() {
    assert_output(
        "narrow-wrap",
        r#"
module fuzz.NarrowWrap;
import { println } from std.io;
const A: i32 = i32(u8(200) + u8(100));
const M: i32 = i32(u8(200) * u8(2));
const Q: i32 = i32(u8(200) * u8(200));
const S: i32 = i32(u8(1) - u8(2));
const L: i32 = i32(u8(128) << 1);
const W: i32 = i32(u16(65535) + u16(2));
const H: i32 = i32(u16(65535) << 15);
const I: i32 = i32(i8(127) + i8(1));
func main(): i32 {
    println($"const: {A} {M} {Q} {S} {L} {W} {H} {I}");
    println($"inline: {i32(u8(200) + u8(100))} {i32(u8(200) * u8(2))} {i32(u8(200) * u8(200))} {i32(u8(1) - u8(2))} {i32(u8(128) << 1)} {i32(u16(65535) + u16(2))} {i32(u16(65535) << 15)} {i32(i8(127) + i8(1))}");
    return 0;
}
"#,
        "const: 44 144 64 255 0 1 32768 -128\ninline: 44 144 64 255 0 1 32768 -128\n",
    );
}

#[test]
fn should_wrap_a_16_bit_multiply_and_shift() {
    assert_output(
        "narrow-16",
        r#"
module fuzz.Narrow16;
import { println } from std.io;
func main(): i32 {
    println($"v: {i32(u16(65535) * u16(65535))} {i32(u16(65535) << 15)}");
    return 0;
}
"#,
        "v: 1 32768\n",
    );
}

#[test]
fn should_build_a_narrow_const_under_the_strict_flags() {
    assert_output(
        "narrow-const-build",
        r#"
module fuzz.NarrowConstBuild;
import { println } from std.io;
const U8W: u8 = u8(200) + u8(100);
const U16W: u16 = u16(65535) * u16(65535);
func main(): i32 {
    println($"v: {i32(U8W)} {i32(U16W)}");
    return 0;
}
"#,
        "v: 44 1\n",
    );
}

#[test]
fn should_build_a_narrow_local_a_fill_and_a_field() {
    assert_output(
        "narrow-build-positions",
        r#"
module fuzz.NarrowBuildPositions;
import { println } from std.io;
struct S { w: u8; }
func main(): i32 {
    let local: u8 = u8(200) + u8(100);
    let fill: u8[64] = [; u8(200) + u8(100)];
    let s: S = S { w: u8(200) + u8(100) };
    println($"v: {i32(local)} {i32(fill[0])} {i32(fill[63])} {i32(s.w)}");
    return 0;
}
"#,
        "v: 44 44 44 44\n",
    );
}

#[test]
fn should_build_under_the_freestanding_profile() {
    let source = r#"
module fuzz.NarrowFreestanding;
func main(): i32 {
    let a: u8 = u8(200) + u8(100);
    let b: u16 = u16(65535) * u16(65535);
    let c: u8 = u8(128) << 1;
    return i32(a) + i32(b) + i32(c);
}
"#;
    let emitted = emit_profile("narrow-freestanding", "freestanding", source);
    assert!(
        !emitted.contains("uint32_t") && !emitted.contains("uint16_t"),
        "narrow-freestanding: the freestanding C must use its own scalar typedefs"
    );
    let scratch = ScratchDir::new("narrow-freestanding-build");
    let c_path = scratch.path().join("out.c");
    std::fs::write(&c_path, emitted).expect("write the freestanding C");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-ffreestanding",
            "-fno-builtin",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsyntax-only",
            c_path.to_str().expect("C path"),
        ])
        .output()
        .expect("spawn cc");
    assert!(
        built.status.success(),
        "narrow-freestanding: the freestanding C did not compile; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

#[test]
fn should_agree_with_the_folder_for_the_other_operators() {
    assert_output(
        "narrow-operators",
        r#"
module fuzz.NarrowOperators;
import { println } from std.io;
func main(): i32 {
    println($"folder: {i32(sizeof(u8[usize(u8(200) >> 3)]))} {i32(sizeof(u8[usize(u8(200) / u8(3))]))} {i32(sizeof(u8[usize(u8(200) % u8(7))]))} {i32(sizeof(u8[usize(u8(200) & u8(15))]))} {i32(sizeof(u8[usize(u8(200) | u8(1))]))} {i32(sizeof(u8[usize(u8(200) ^ u8(255))]))} {i32(sizeof(u8[usize(~u8(0))]))}");
    println($"inline: {i32(u8(200) >> 3)} {i32(u8(200) / u8(3))} {i32(u8(200) % u8(7))} {i32(u8(200) & u8(15))} {i32(u8(200) | u8(1))} {i32(u8(200) ^ u8(255))} {i32(~u8(0))}");
    return 0;
}
"#,
        "folder: 25 66 4 8 201 55 255\ninline: 25 66 4 8 201 55 255\n",
    );
}

#[test]
fn should_keep_the_32_bit_controls() {
    assert_output(
        "narrow-32-controls",
        r#"
module fuzz.Narrow32Controls;
import { println } from std.io;
func main(): i32 {
    println($"v: {i32(i32(2147483647) + i32(1))} {i32(u32(4294967295) + u32(1))} {i32(i32(1) << 31)}");
    return 0;
}
"#,
        "v: -2147483648 0 -2147483648\n",
    );
}

// ---------------------------------------------------------------------------
// D6 — enum member values and conversion
// ---------------------------------------------------------------------------

#[test]
fn should_fold_an_explicit_enum_member_expression() {
    assert_output(
        "enum-explicit",
        r#"
module fuzz.EnumExplicit;
import { println } from std.io;
enum E: u8 { A = u8(2) + u8(3), B }
func main(): i32 {
    println($"e: {i32(E.A)} {i32(E.B)}");
    return 0;
}
"#,
        "e: 5 6\n",
    );
}

#[test]
fn should_reject_an_enum_auto_increment_that_does_not_fit() {
    let stderr = compile_error(
        "enum-overflow",
        r#"
module fuzz.EnumOverflow;
enum E: u8 { A = 255, B }
func main(): i32 {
    return i32(E.B);
}
"#,
    );
    assert_e10202("enum-overflow", &stderr, "u8");
}

#[test]
fn should_keep_an_in_range_enum_auto_increment() {
    assert_output(
        "enum-in-range",
        r#"
module fuzz.EnumInRange;
import { println } from std.io;
enum E: u8 { A = 254, B }
func main(): i32 {
    println($"e: {i32(E.A)} {i32(E.B)}");
    return 0;
}
"#,
        "e: 254 255\n",
    );
}

#[test]
fn should_store_an_enum_member_in_its_enum() {
    assert_output(
        "enum-store",
        r#"
module fuzz.EnumStore;
import { println } from std.io;
enum E: u8 { A = 254, B }
func main(): i32 {
    let x: E = E.B;
    let y: u8 = u8(x);
    println($"v: {i32(x)} {i32(y)}");
    return 0;
}
"#,
        "v: 255 255\n",
    );
}

#[test]
fn should_emit_an_enum_conversion_as_a_cast() {
    assert_output(
        "enum-cast",
        r#"
module fuzz.EnumCast;
import { println } from std.io;
enum Direction: u8 { UP, DOWN, LEFT = 10, RIGHT }
func main(): i32 {
    let d: Direction = Direction(2);
    println($"d: {i32(d)}");
    return 0;
}
"#,
        "d: 2\n",
    );
}

#[test]
fn should_convert_a_runtime_value_to_an_enum() {
    assert_output(
        "enum-cast-runtime",
        r#"
module fuzz.EnumCastRuntime;
import { println } from std.io;
enum Direction: u8 { UP, DOWN, LEFT = 10, RIGHT }
func main(): i32 {
    let n: u8 = 2;
    let d: Direction = Direction(n);
    println($"d: {i32(d)}");
    return 0;
}
"#,
        "d: 2\n",
    );
}

#[test]
fn should_keep_a_shadowing_binding_a_call() {
    assert_output(
        "enum-shadow",
        r#"
module fuzz.EnumShadow;
import { println } from std.io;
enum E: u8 { A }
func apply(E: func(i32): i32): i32 {
    return E(2);
}
func twice(x: i32): i32 {
    return x * 2;
}
func main(): i32 {
    println($"v: {apply(twice)}");
    return 0;
}
"#,
        "v: 4\n",
    );
}

#[test]
fn should_agree_on_a_cross_module_enum_member() {
    assert_output_files(
        "enum-cross-module",
        &[
            (
                "holder.clo",
                r#"
module fuzz.EnumHolder;
export enum E: u8 { A = u8(1) + u8(2), B }
"#,
            ),
            (
                "main.clo",
                r#"
module fuzz.EnumCrossModule;
import { println } from std.io;
import { E } from fuzz.EnumHolder;
const N: usize = usize(i32(E.B)) + 1;
func main(): i32 {
    let a: u8[N] = [; 0];
    println($"n: {i32(E.A)} {i32(E.B)} {sizeof(u8[N])} {i32(N)}");
    return 0;
}
"#,
            ),
        ],
        "main.clo",
        "n: 3 4 5 5\n",
    );
}

// ---------------------------------------------------------------------------
// D5 — f32 rounding and constant float-to-integer overflow
// ---------------------------------------------------------------------------

#[test]
fn should_round_an_f32_cast_in_a_constant_context() {
    assert_output(
        "f32-cast-round",
        r#"
module fuzz.F32CastRound;
import { println } from std.io;
const N: usize = usize(f32(16777217)) - 16777100;
func main(): i32 {
    println($"n: {sizeof(u8[N])} {i32(N)}");
    return 0;
}
"#,
        "n: 116 116\n",
    );
}

#[test]
fn should_round_a_declared_f32_const_reference() {
    assert_output(
        "f32-declared-round",
        r#"
module fuzz.F32DeclaredRound;
import { println } from std.io;
const A: f32 = 16777217.0;
const N: usize = usize(f64(A)) - 16777100;
func main(): i32 {
    println($"n: {sizeof(u8[N])} {i32(N)}");
    return 0;
}
"#,
        "n: 116 116\n",
    );
}

#[test]
fn should_keep_a_finite_f32_conversion() {
    assert_output(
        "f32-finite",
        r#"
module fuzz.F32Finite;
import { println } from std.io;
const N: usize = usize(f32(42.9)) + 1;
func main(): i32 {
    println($"n: {sizeof(u8[N])} {i32(N)}");
    return 0;
}
"#,
        "n: 43 43\n",
    );
}

#[test]
fn should_keep_a_negative_zero_float_conversion() {
    assert_output(
        "f32-negative-zero",
        r#"
module fuzz.F32NegativeZero;
import { println } from std.io;
const Z: i32 = i32(f32(-0.0));
func main(): i32 {
    println($"z: {Z}");
    return 0;
}
"#,
        "z: 0\n",
    );
}

#[test]
fn should_reject_a_fractional_boundary_float_conversion() {
    for (tag, source, target) in [
        (
            "f32-frac-i8-high",
            "const N: i8 = i8(f32(127.5));\nfunc main(): i32 { return i32(N); }\n",
            "i8",
        ),
        (
            "f32-frac-u8-high",
            "const N: u8 = u8(f32(255.9));\nfunc main(): i32 { return i32(N); }\n",
            "u8",
        ),
        (
            "f32-frac-u8-low",
            "const N: u8 = u8(f32(-0.5));\nfunc main(): i32 { return i32(N); }\n",
            "u8",
        ),
        (
            "f32-frac-i8-low",
            "const N: i8 = i8(f32(-128.5));\nfunc main(): i32 { return i32(N); }\n",
            "i8",
        ),
    ] {
        let stderr = compile_error(
            tag,
            &format!("module fuzz.F32Frac;\n{source}"),
        );
        assert_e10202(tag, &stderr, target);
    }
}

#[test]
fn should_keep_a_fractional_in_range_float_conversion() {
    assert_output(
        "f32-frac-in-range",
        r#"
module fuzz.F32FracInRange;
import { println } from std.io;
const A: i8 = i8(f32(127.0));
const B: u8 = u8(f32(255.0));
const C: i8 = i8(f32(-128.0));
const D: i8 = i8(f32(-1.9));
func main(): i32 {
    println($"n: {i32(A)} {i32(B)} {i32(C)} {i32(D)}");
    return 0;
}
"#,
        "n: 127 255 -128 -1\n",
    );
}

#[test]
fn should_reject_a_constant_nan_float_conversion() {
    let stderr = compile_error(
        "f32-nan",
        r#"
module fuzz.F32Nan;
const B: f32 = f32(0.0) / f32(0.0);
enum E: i32 { A = i32(B) }
func main(): i32 {
    return i32(E.A);
}
"#,
    );
    assert_e10202("f32-nan", &stderr, "i32");
}

#[test]
fn should_reject_a_constant_out_of_range_float_conversion() {
    let stderr = compile_error(
        "f32-range",
        r#"
module fuzz.F32Range;
const N: i8 = i8(f32(127.9999999));
func main(): i32 {
    return i32(N);
}
"#,
    );
    assert_e10202("f32-range", &stderr, "i8");
}

#[test]
fn should_reject_a_constant_infinity_float_conversion() {
    let stderr = compile_error(
        "f32-infinity",
        r#"
module fuzz.F32Infinity;
const P: f32 = f32(1.0) / f32(0.0);
enum E: i32 { A = i32(P) }
func main(): i32 {
    return i32(E.A);
}
"#,
    );
    assert_e10202("f32-infinity", &stderr, "i32");
}

// ---------------------------------------------------------------------------
// D3 — inline constant shift divisor
// ---------------------------------------------------------------------------

#[test]
fn should_keep_an_inline_shift_divisor_out_of_the_zero_trap() {
    assert_output(
        "shift-divisor",
        r#"
module fuzz.ShiftDivisor;
import { println } from std.io;
const V: u8 = 128;
const Q: u8 = u8(10) / (u8(128) >> 8);
func iden(x: u8): u8 {
    return x;
}
func div(x: u8): u8 {
    return u8(10) / (x >> 8);
}
func main(): i32 {
    let a: u8 = u8(10) / (u8(128) >> 8);
    let b: u8[2] = [; u8(10) / (u8(128) >> 8)];
    let c: u8 = u8(10) / (V >> 8);
    let d: u8 = div(u8(128));
    let e: u8 = iden(u8(10) / (u8(128) >> 8));
    println($"q: {i32(a)} {i32(b[0])} {i32(c)} {i32(d)} {i32(e)} {i32(Q)}");
    return 0;
}
"#,
        "q: 0 0 0 0 0 0\n",
    );
}

#[test]
fn should_trap_on_a_genuinely_zero_inline_shift_divisor() {
    assert_trap(
        "shift-divisor-zero",
        r#"
module fuzz.ShiftDivisorZero;
func main(): i32 {
    let z: u8 = 0;
    let q: u8 = u8(10) / (z >> 8);
    return i32(q);
}
"#,
        "error E11210: integer division by zero",
    );
}

#[test]
fn should_keep_a_guard_free_fill_unchanged() {
    assert_output(
        "fill-guard-free",
        r#"
module fuzz.FillGuardFree;
import { println } from std.io;
func main(): i32 {
    let a: u8[4] = [; 7];
    println($"v: {i32(a[0])} {i32(a[3])}");
    return 0;
}
"#,
        "v: 7 7\n",
    );
}
