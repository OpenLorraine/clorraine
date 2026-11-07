//! Behavior oracle for the generated-C optimization feature.
//!
//! The original cases pass on the base compiler and must keep passing. The
//! fix-pass trap cases were added with the fix and are red on the pre-fix
//! compiler; they pin the intended behavior, not the old emission. The cases
//! derive from the requirements: folded constants, compact fills,
//! slice/string index guards, and constant division guards. The emission
//! shapes are pinned separately in the implementation suite.

// This suite participates in the host AddressSanitizer leg. The constant is
// the discovery marker; the builder below adds the sanitizer flags when the
// runner sets the environment variable.
const _ASAN_SUITE: &str = "RDC_ASAN";

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
            "clc-genopt-spec-{}-{tag}-{sequence}",
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

fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

fn emit_files(tag: &str, profile: &str, files: &[(&str, &str)], root_name: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let mut paths = Vec::new();
    for (name, source) in files {
        let path = scratch.path().join(name);
        std::fs::write(&path, source).unwrap_or_else(|error| panic!("write {name}: {error}"));
        paths.push(path);
    }
    let output_path = scratch.path().join("out.c");
    let mut args = vec![
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
    ];
    let root = paths
        .iter()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some(root_name))
        .unwrap_or_else(|| panic!("{tag}: root file {root_name} is missing"));
    args.push(root.display().to_string());
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
    let emitted = std::fs::read_to_string(&output_path).expect("read the emitted C");
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty"
    );
    (scratch, emitted)
}

fn build_host(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut command = Command::new("cc");
    command.args([
        "-std=c99",
        "-O0",
        "-g",
        "-Wall",
        "-Wextra",
        "-Werror",
    ]);
    if std::env::var("RDC_ASAN").is_ok() {
        command.args(["-fsanitize=address", "-fno-omit-frame-pointer"]);
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
    let (_scratch, emitted) = emit_files(tag, "host", &[("main.clo", source)], "main.clo");
    let scratch = ScratchDir::new(tag);
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

fn assert_trap(tag: &str, source: &str, first_line: &str) {
    let (code, _stdout, stderr) = run_program(tag, source);
    assert_ne!(code, 0, "{tag}: a trap must exit nonzero");
    let first = stderr.lines().next().unwrap_or_default();
    assert_eq!(first, first_line, "{tag}: unexpected first stderr line");
}

#[test]
fn should_fold_a_scalar_const_in_a_loop_bound() {
    assert_output(
        "const-loop",
        r#"
module bench.ConstLoop;
import { println } from std.io;
const N: i32 = 8;
func main(): i32 {
    let i: i32 = 0;
    let sum: i32 = 0;
    while (i < N) {
        sum = sum + i;
        i = i + 1;
    }
    println($"const: {sum}");
    return 0;
}
"#,
        "const: 28\n",
    );
}

#[test]
fn should_fold_negative_shift_and_wrapping_consts() {
    assert_output(
        "const-negative",
        r#"
module bench.ConstNegative;
import { println } from std.io;
const N: i32 = -3;
const S: i32 = 1 << 31;
const T: i32 = 1 << 32;
const A: i32 = 2147483647 + 1;
func main(): i32 {
    println($"neg: {N} {i32(N < 0)} {10 / N}");
    println($"shift: {i32(S < 0)} {T}");
    println($"wrap: {A}");
    return 0;
}
"#,
        "neg: -3 1 -3\nshift: 1 1\nwrap: -2147483648\n",
    );
}

#[test]
fn should_keep_finite_float_consts_as_places() {
    assert_output(
        "const-float",
        r#"
module bench.ConstFloat;
import { println } from std.io;
const A: f32 = f32(16777217.0);
const B: f64 = 0.5;
func main(): i32 {
    let b: f32 = f32(16777217.0);
    let c: f64 = 0.5;
    if (A == b) { println("f32: equal"); } else { println("f32: different"); }
    if (B == c) { println("f64: equal"); } else { println("f64: different"); }
    return 0;
}
"#,
        "f32: equal\nf64: equal\n",
    );
}

#[test]
fn should_fold_expression_and_imported_consts() {
    let holder = r#"
module bench.Holder;
export const A: i32 = 5;
export const B: i32 = A * 2;
"#;
    let main = r#"
module bench.ConstImport;
import { B } from bench.Holder;
import { println } from std.io;
const C: i32 = 7 * 2 + 1;
func main(): i32 {
    println($"values: {B} {C}");
    return 0;
}
"#;
    let (_scratch, emitted) = emit_files(
        "const-import",
        "host",
        &[("holder.clo", holder), ("main.clo", main)],
        "main.clo",
    );
    let scratch = ScratchDir::new("const-import-run");
    let program = build_host("const-import", &scratch, &emitted);
    let output = Command::new(&program).output().expect("run the program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "values: 10 15\n"
    );
}

#[test]
fn should_keep_a_shadowing_local_as_a_local() {
    assert_output(
        "const-shadow",
        r#"
module bench.ConstShadow;
import { println } from std.io;
const N: i32 = 5;
func main(): i32 {
    let N: i32 = 3;
    println($"shadow: {N}");
    return 0;
}
"#,
        "shadow: 3\n",
    );
}

#[test]
fn should_not_fold_an_address_taken_const() {
    let (code, _stdout, stderr) = run_program(
        "const-address",
        r#"
module bench.ConstAddress;
const N: i32 = 5;
func main(): i32 {
    let p: raw i32 = unsafe &N;
    unsafe { *p = 6; }
    return N - 5;
}
"#,
    );
    assert_eq!(code, 1, "the raw write must be visible through the read; stderr:\n{stderr}");
}

#[test]
fn should_not_fold_an_address_taken_const_in_a_call_or_defer() {
    let (code, stdout, stderr) = run_program(
        "const-address-nested",
        r#"
module bench.ConstAddressNested;
import { println } from std.io;
const N: i32 = 5;
func touch(p: raw i32): void {
    unsafe { *p = 6; }
}
func main(): i32 {
    let f: func(): i32 = () => N;
    touch(unsafe &N);
    let v: i32 = f();
    defer { println($"defer: {N}"); }
    return v - 5;
}
"#,
    );
    assert_eq!(code, 1, "the raw write must be visible; stderr:\n{stderr}");
    assert_eq!(stdout, "defer: 6\n");
}

#[test]
fn should_keep_a_static_str_const_as_a_place() {
    assert_output(
        "const-str",
        r#"
module bench.ConstStr;
import { println } from std.io;
const S: static str = "abc";
func main(): i32 {
    println($"str: {S}");
    return 0;
}
"#,
        "str: abc\n",
    );
}

#[test]
fn should_leave_a_module_initializer_read_unchanged() {
    assert_output(
        "const-init",
        r#"
module bench.ConstInit;
import { println } from std.io;
const N: i32 = 5;
let X: i32 = N;
func main(): i32 {
    println($"init: {X}");
    return 0;
}
"#,
        "init: 5\n",
    );
}

#[test]
fn should_fold_a_bare_qualified_const_but_not_a_member() {
    let holder = r#"
module bench.QualifiedHolder;
export const N: i32 = 7;
export struct Pair { n: i32; }
export let G: Pair = Pair { n: 9 };
"#;
    let main = r#"
module bench.QualifiedMain;
import { println } from std.io;
func main(): i32 {
    println($"qualified: {bench.QualifiedHolder.N} {bench.QualifiedHolder.G.n}");
    return 0;
}
"#;
    let (_scratch, emitted) = emit_files(
        "const-qualified",
        "host",
        &[("holder.clo", holder), ("main.clo", main)],
        "main.clo",
    );
    let scratch = ScratchDir::new("const-qualified-run");
    let program = build_host("const-qualified", &scratch, &emitted);
    let output = Command::new(&program).output().expect("run the program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "qualified: 7 9\n"
    );
}

#[test]
fn should_fill_byte_and_integer_arrays() {
    assert_output(
        "fill-scalars",
        r#"
module bench.FillScalars;
import { println } from std.io;
func main(): i32 {
    let a: u8[4096] = [; 1];
    let b: i32[4096] = [; 7];
    let sa: u32 = 0;
    let sb: i32 = 0;
    let i: usize = 0;
    while (i < 4096) {
        sa = sa + u32(a[i]);
        sb = sb + b[i];
        i = i + 1;
    }
    println($"fill: {sa} {sb}");
    return 0;
}
"#,
        "fill: 4096 28672\n",
    );
}

#[test]
fn should_fill_bool_and_prefix_arrays() {
    assert_output(
        "fill-bool-prefix",
        r#"
module bench.FillBoolPrefix;
import { println } from std.io;
func main(): i32 {
    let a: bool[4096] = [; true];
    let b: u8[20] = [2, 3; 4];
    let count: u32 = 0;
    let i: usize = 0;
    while (i < 4096) {
        if (a[i]) { count = count + 1; }
        i = i + 1;
    }
    let sum: u32 = 0;
    i = 0;
    while (i < 20) {
        sum = sum + u32(b[i]);
        i = i + 1;
    }
    println($"bool: {count} prefix: {sum}");
    return 0;
}
"#,
        "bool: 4096 prefix: 77\n",
    );
}

#[test]
fn should_evaluate_an_impure_fill_once() {
    assert_output(
        "fill-impure",
        r#"
module bench.FillImpure;
import { println } from std.io;
let calls: i32 = 0;
func next(): u8 {
    calls = calls + 1;
    return 1;
}
func main(): i32 {
    let a: u8[4096] = [; next()];
    let sum: u32 = 0;
    let i: usize = 0;
    while (i < 4096) {
        sum = sum + u32(a[i]);
        i = i + 1;
    }
    println($"impure: calls={calls} sum={sum}");
    return 0;
}
"#,
        "impure: calls=1 sum=4096\n",
    );
}

#[test]
fn should_fill_a_volatile_place_per_element() {
    assert_output(
        "fill-volatile",
        r#"
module bench.FillVolatile;
import { println } from std.io;
volatile let seed: u8 = 3;
func main(): i32 {
    let a: u8[4096] = [; seed];
    let sum: u32 = 0;
    let i: usize = 0;
    while (i < 4096) {
        sum = sum + u32(a[i]);
        i = i + 1;
    }
    println($"volatile: {sum}");
    return 0;
}
"#,
        "volatile: 12288\n",
    );
}

#[test]
fn should_fill_an_adjusted_parameter_and_a_return_slot() {
    assert_output(
        "fill-targets",
        r#"
module bench.FillTargets;
import { println } from std.io;
func fill(borrow a: u8[4096]): void {
    a = [; 1];
}
func make(): u8[4096] {
    return [; 1];
}
func main(): i32 {
    let a: u8[4096] = [; 0];
    fill(a);
    let b: u8[4096] = make();
    let sum: u32 = 0;
    let i: usize = 0;
    while (i < 4096) {
        sum = sum + u32(a[i]) + u32(b[i]);
        i = i + 1;
    }
    println($"targets: {sum}");
    return 0;
}
"#,
        "targets: 8192\n",
    );
}

#[test]
fn should_keep_a_nested_fill_behavior_unchanged() {
    assert_output(
        "fill-nested",
        r#"
module bench.FillNested;
import { println } from std.io;
const N: i32 = 8;
func main(): i32 {
    let a: i32[N][N] = [; [; 1]];
    let sum: i32 = 0;
    let i: i32 = 0;
    while (i < N) {
        let j: i32 = 0;
        while (j < N) {
            sum = sum + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"nested: {sum}");
    return 0;
}
"#,
        "nested: 64\n",
    );
}

#[test]
fn should_accept_a_non_negative_slice_and_string_constant_index() {
    assert_output(
        "guard-constant",
        r#"
module bench.GuardConstant;
import { slice } from std.mem;
import { println } from std.io;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    let t: str = "abcd";
    let x: u8 = s[1];
    let y: u8 = t[1];
    println($"index: {x} {y}");
    return 0;
}
"#,
        "index: 2 98\n",
    );
}

#[test]
fn should_trap_on_a_negative_or_upper_slice_index() {
    assert_trap(
        "guard-slice-negative",
        r#"
module bench.SliceNegative;
import { slice } from std.mem;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    let x: u8 = s[-1];
    return i32(x);
}
"#,
        "error E11211: index out of range",
    );
    assert_trap(
        "guard-string-negative",
        r#"
module bench.StringNegative;
func main(): i32 {
    let t: str = "abcd";
    let x: u8 = t[-1];
    return i32(x);
}
"#,
        "error E11211: index out of range",
    );
    assert_trap(
        "guard-slice-upper",
        r#"
module bench.SliceUpper;
import { slice } from std.mem;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    let x: u8 = s[4];
    return i32(x);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_reject_a_negative_fixed_array_index_at_compile_time() {
    let scratch = ScratchDir::new("guard-fixed-negative");
    let source = scratch.path().join("main.clo");
    let output = scratch.path().join("out.c");
    std::fs::write(
        &source,
        r#"
module bench.FixedNegative;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let x: u8 = a[-1];
    return i32(x);
}
"#,
    )
    .expect("write the source");
    let run = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output.display().to_string(),
        source.display().to_string(),
    ]);
    assert_ne!(run.status.code(), Some(0), "a fixed-array negative index must be rejected");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("E10216"),
        "expected E10216; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn should_keep_a_usize_and_a_runtime_signed_index_working() {
    assert_output(
        "guard-runtime",
        r#"
module bench.GuardRuntime;
import { slice } from std.mem;
import { println } from std.io;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    let i: usize = 1;
    let j: i32 = 2;
    println($"runtime: {s[i]} {s[j]}");
    return 0;
}
"#,
        "runtime: 2 3\n",
    );
}

#[test]
fn should_fold_a_constant_division_and_keep_the_traps() {
    assert_output(
        "div-constant",
        r#"
module bench.DivConstant;
import { println } from std.io;
func main(): i32 {
    let r: i32 = 10 % 2;
    let d: i32 = -1;
    let q: i32 = 10 / d;
    println($"div: {r} {q}");
    return 0;
}
"#,
        "div: 0 -10\n",
    );
    assert_trap(
        "div-min-neg",
        r#"
module bench.DivMinNeg;
func main(): i32 {
    let min: i32 = -2147483648;
    let r: i32 = min / -1;
    return r;
}
"#,
        "error E11210: integer division by zero",
    );
    assert_trap(
        "div-runtime-zero",
        r#"
module bench.DivRuntimeZero;
func main(): i32 {
    let z: i32 = 0;
    let r: i32 = 10 / z;
    return r;
}
"#,
        "error E11210: integer division by zero",
    );
    assert_trap(
        "div-literal-zero",
        r#"
module bench.DivLiteralZero;
func main(): i32 {
    let r: i32 = 10 / 0;
    return r;
}
"#,
        "error E11210: integer division by zero",
    );
    assert_trap(
        "div-wrapping",
        r#"
module bench.DivWrapping;
func main(): i32 {
    let min: i32 = -2147483648;
    let d: i32 = i32(2147483647) + i32(2147483647) + i32(1);
    let r: i32 = min / d;
    return r;
}
"#,
        "error E11210: integer division by zero",
    );
}

#[test]
fn should_emit_deterministic_c_for_the_same_source() {
    let source = r#"
module bench.Determinism;
const N: i32 = 8;
func main(): i32 {
    let a: u8[32] = [; 1];
    let i: i32 = 0;
    let sum: i32 = 0;
    while (i < N) {
        sum = sum + i32(a[i]);
        i = i + 1;
    }
    return sum;
}
"#;
    let scratch = ScratchDir::new("determinism");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the source");
    let first_path = scratch.path().join("first.c");
    let second_path = scratch.path().join("second.c");
    for output in [&first_path, &second_path] {
        let run = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
            source_path.display().to_string(),
        ]);
        assert_eq!(
            run.status.code(),
            Some(0),
            "the front end rejected the determinism source; stderr:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    let first = std::fs::read_to_string(&first_path).expect("read the first C");
    let second = std::fs::read_to_string(&second_path).expect("read the second C");
    assert_eq!(first, second, "the same source must produce byte-identical C");
}

#[test]
fn should_accept_the_new_shapes_on_every_profile() {
    let source = r#"
module bench.Profiles;
const N: i32 = 32;
func main(): i32 {
    let a: u8[N] = [; 1];
    let i: i32 = 0;
    let sum: i32 = 0;
    while (i < N) {
        sum = sum + i32(a[i]);
        i = i + 1;
    }
    return sum;
}
"#;
    for profile in ["host", "hosted-amiga", "freestanding"] {
        let (_scratch, emitted) = emit_files(
            &format!("profile-{profile}"),
            profile,
            &[("main.clo", source)],
            "main.clo",
        );
        assert!(
            !emitted.trim().is_empty(),
            "{profile}: the emitter produced no C"
        );
    }
}

#[test]
fn should_mask_a_fill_shift_at_the_element_width() {
    assert_output(
        "fill-shift",
        r#"
module bench.FillShift;
import { println } from std.io;
func main(): i32 {
    let a: u8[32] = [; u8(1) << 8];
    let b: u8[32] = [; u8(1) << 32];
    let sa: u32 = 0;
    let sb: u32 = 0;
    let i: usize = 0;
    while (i < 32) {
        sa = sa + u32(a[i]);
        sb = sb + u32(b[i]);
        i = i + 1;
    }
    println($"shift-fill: {sa} {sb}");
    return 0;
}
"#,
        "shift-fill: 32 32\n",
    );
}

#[test]
fn should_fill_a_volatile_target_without_memset() {
    assert_output(
        "fill-volatile-target",
        r#"
module bench.FillVolatileTarget;
import { println } from std.io;
volatile let G: u8[16] = [; 0];
func main(): i32 {
    G = [; 1];
    println($"volatile-target: {i32(G[0])}");
    return 0;
}
"#,
        "volatile-target: 1\n",
    );
}

#[test]
fn should_keep_a_float_const_as_a_place() {
    assert_output(
        "const-float-place",
        r#"
module bench.FloatPlace;
import { println } from std.io;
const A: f32 = f32(1.0e39);
const B: f64 = 0.1;
func main(): i32 {
    println($"float-place: {i32(A > f32(0.0))} {i32(B > 0.0)}");
    return 0;
}
"#,
        "float-place: 1 1\n",
    );
}

#[test]
fn should_keep_a_narrow_arithmetic_const_and_its_storage_consistent() {
    assert_output(
        "const-narrow",
        r#"
module bench.NarrowConst;
import { println } from std.io;
const G: i32 = i32(u8(200) + u8(100));
let W: i32 = G;
func main(): i32 {
    println($"narrow: {G} {W}");
    return 0;
}
"#,
        "narrow: 44 44\n",
    );
}

#[test]
fn should_keep_an_enum_member_const_consistent_with_the_emitter() {
    assert_output(
        "const-enum",
        r#"
module bench.EnumConst;
import { println } from std.io;
enum E: u8 { A = 254, B }
const X: i32 = i32(E.B);
func main(): i32 {
    println($"enum: {X} {i32(E.B)}");
    return 0;
}
"#,
        "enum: 255 255\n",
    );
}

#[test]
fn should_not_fold_an_address_taken_const_through_an_alias() {
    let (code, stdout, stderr) = run_program(
        "const-chain-address",
        r#"
module bench.ChainAddress;
import { println } from std.io;
const A: i32 = 5;
let Y: i32 = poke();
const B: i32 = A;
func poke(): i32 {
    unsafe { let p: raw i32 = &A; *p = 6; }
    return 0;
}
func main(): i32 {
    println($"chain: {B} {A} {Y}");
    return 0;
}
"#,
    );
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert_eq!(stdout, "chain: 6 6 0\n");
}

#[test]
fn should_keep_the_lower_guard_for_a_huge_negative_constant_index() {
    assert_trap(
        "guard-huge-negative",
        r#"
module bench.HugeNegative;
import { slice } from std.mem;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    let x: u8 = s[-i32(2147483648)];
    return i32(x);
}
"#,
        "error E11211: index out of range",
    );
}

#[test]
fn should_keep_the_zero_trap_for_a_bitnot_divisor() {
    assert_trap(
        "div-bitnot",
        r#"
module bench.BitNotDiv;
func main(): i32 {
    let x: u32 = 7;
    let q: u32 = x / ~u32(4294967295);
    return i32(q);
}
"#,
        "error E11210: integer division by zero",
    );
}

#[test]
fn should_keep_an_integer_const_from_an_f32_cast_as_a_place() {
    assert_output(
        "const-f32-cast",
        r#"
module bench.F32CastConst;
import { println } from std.io;
const A: i32 = i32(f32(16777217.0));
const B: i32 = i32(f32(0.99999999));
func main(): i32 {
    let a: i32 = i32(f32(16777217.0));
    let b: i32 = i32(f32(0.99999999));
    println($"f32-cast: {A} {a} {B} {b}");
    return 0;
}
"#,
        "f32-cast: 16777216 16777216 1 1\n",
    );
}

#[test]
fn should_wrap_an_unsigned_unary_divisor_at_its_type() {
    assert_output(
        "unsigned-unary-div",
        r#"
module bench.UnsignedUnaryDiv;
import { println } from std.io;
func main(): i32 {
    let m: i32 = -2147483648;
    let a: i32 = m / i32(-u8(1));
    let b: i32 = m / i32(~u8(0));
    println($"unsigned-div: {a} {b}");
    return 0;
}
"#,
        "unsigned-div: -8421504 -8421504\n",
    );
}

#[test]
fn should_trap_on_a_constant_binary_or_float_zero_divisor() {
    assert_trap(
        "div-binary-zero",
        r#"
module bench.BinaryZero;
func main(): i32 {
    let x: u32 = 7;
    let q: u32 = x % (u32(0) + u32(0));
    return i32(q);
}
"#,
        "error E11210: integer division by zero",
    );
    for (tag, divisor) in [
        ("div-float-zero", "i32(0.0)"),
        ("div-float-half", "i32(0.5)"),
        ("div-float-nested", "i32(f32(0.0))"),
        ("div-float-negative-zero", "i32(-0.0)"),
        ("div-float-sum", "i32(0.0 + 0.0)"),
        ("div-float-int-expr", "(i32(f32(2)) - 2)"),
        ("div-float-extra-cast", "i32(u8(0.5))"),
        ("div-float-bool-expr", "i32(f32(false))"),
        ("div-float-char-expr", "i32(f32('\0'))"),
        ("div-float-bitand", "i32(f32(1 & 0))"),
        ("div-float-int-arith", "i32(f32(1 / 2 + 1 / 2))"),
        ("div-float-not", "i32(f32(!true))"),
        ("div-float-bool-cast", "i32(f32(bool(i32(0))))"),
        ("div-float-conditional", "i32(f32(true ? false : false))"),
        ("div-float-comparison", "i32(f32(0.5 < 0.25))"),
    ] {
        assert_trap(
            tag,
            &format!(
                r#"
module bench.FloatZero;
func main(): i32 {{
    let x: i32 = 7;
    let q: i32 = x / {divisor};
    return q;
}}
"#
            ),
            "error E11210: integer division by zero",
        );
    }
    assert_trap(
        "div-float-unsigned-negative",
        r#"
module bench.FloatUnsignedNegative;
func main(): i32 {
    let x: u32 = 7;
    let q: u32 = x / u32(-0.5);
    return i32(q);
}
"#,
        "error E11212: float-to-integer conversion out of range",
    );
    assert_trap(
        "div-float-unsigned-nested",
        r#"
module bench.FloatUnsignedNested;
func main(): i32 {
    let x: u32 = 7;
    let q: u32 = x / (u32(f32(-0.5)) + u32(0));
    return i32(q);
}
"#,
        "error E11212: float-to-integer conversion out of range",
    );
    assert_output(
        "div-int-cast",
        r#"
module bench.IntCastDiv;
import { println } from std.io;
func main(): i32 {
    let x: i32 = 7;
    let q: i32 = x / i8(200);
    println($"int-cast: {q}");
    return 0;
}
"#,
        "int-cast: 0\n",
    );
    assert_output(
        "div-float-comparison-one",
        r#"
module bench.FloatComparisonOne;
import { println } from std.io;
func main(): i32 {
    let x: i32 = 7;
    let q: i32 = x / i32(f32(0.5 > 0.25));
    println($"comparison-one: {q}");
    return 0;
}
"#,
        "comparison-one: 7\n",
    );
    assert_output(
        "div-float-one",
        r#"
module bench.FloatOne;
import { println } from std.io;
func main(): i32 {
    let x: i32 = 7;
    let a: i32 = x / i32(f32(0.5) + f32(0.5));
    let b: i32 = x / i32(f32(0.99999999));
    println($"float-one: {a} {b}");
    return 0;
}
"#,
        "float-one: 7 7
",
    );
}

#[test]
fn should_keep_a_float_fill_on_the_store_path() {
    assert_output(
        "fill-float",
        r#"
module bench.FillFloat;
import { println } from std.io;
func main(): i32 {
    let a: i8[64] = [; i8(f32(44.0))];
    println($"fill-float: {i32(a[63])}");
    return 0;
}
"#,
        "fill-float: 44\n",
    );
}

#[test]
fn should_keep_a_volatile_target_unrolled_on_every_fill_path() {
    assert_output(
        "volatile-paths",
        r#"
module bench.VolatilePaths;
import { println } from std.io;
struct S { tag: i32; xs: u8[16]; }
volatile let G: u8[20] = [; 0];
volatile let H: S = S { tag: 0, xs: [; 0] };
func main(): i32 {
    let x: u8 = 7;
    G = [x; 1];
    let flag: bool = true;
    G = flag ? [x; 1] : [; 2];
    H = S { xs: [; 1], tag: 2 };
    println($"volatile-paths: {i32(G[0])} {i32(H.xs[0])}");
    return 0;
}
"#,
        "volatile-paths: 7 1\n",
    );
}

#[test]
fn should_keep_a_materialized_conditional_fill_unrolled() {
    assert_output(
        "materialized-conditional",
        r#"
module bench.MaterializedConditional;
import { println } from std.io;
func eat(a: u8[4096]): i32 { return i32(a[0]); }
func main(): i32 {
    let c: bool = true;
    let ok: bool = true;
    while (ok && eat(c ? [; 1] : [; 2]) == 1) {
        println("materialized-conditional: ok");
        return 0;
    }
    return 1;
}
"#,
        "materialized-conditional: ok\n",
    );
}

#[test]
fn should_keep_a_dead_conditional_arm_out_of_the_zero_trap() {
    assert_output(
        "dead-conditional-arm",
        r#"
module bench.DeadConditionalArm;
import { println } from std.io;
func main(): i32 {
    let x: i32 = 7;
    let a: i32 = x / (false ? i8(f32(300)) : i8(1));
    let flag: bool = false;
    let b: i32 = x / (flag ? i8(f32(300)) : i8(1));
    println($"dead-arm: {a} {b}");
    return 0;
}
"#,
        "dead-arm: 7 7\n",
    );
}

#[test]
fn should_keep_a_short_circuit_operand_out_of_the_zero_trap() {
    assert_output(
        "short-circuit-divisor",
        r#"
module bench.ShortCircuitDivisor;
import { println } from std.io;
func main(): i32 {
    let x: i32 = 7;
    let a: i32 = x / i32(true || (i8(f32(300)) > i8(0)));
    println($"short-circuit: {a}");
    return 0;
}
"#,
        "short-circuit: 7\n",
    );
    assert_trap(
        "short-circuit-zero-divisor",
        r#"
module bench.ShortCircuitZero;
func main(): i32 {
    let x: i32 = 7;
    let q: i32 = x / i32(false && (i8(f32(300)) > i8(0)));
    return q;
}
"#,
        "error E11210: integer division by zero",
    );
}

#[test]
fn should_trap_a_comparison_of_an_out_of_range_float_cast() {
    assert_trap(
        "float-comparison-out-of-range",
        r#"
module bench.FloatComparisonOutOfRange;
func main(): i32 {
    let x: i32 = 7;
    let q: i32 = x / i32(i8(f32(300)) > i8(0));
    return q;
}
"#,
        "error E11212: float-to-integer conversion out of range",
    );
}

#[test]
fn should_report_e11212_for_a_live_out_of_range_conditional_arm() {
    assert_trap(
        "live-conditional-arm",
        r#"
module bench.LiveConditionalArm;
func main(): i32 {
    let x: i32 = 7;
    let q: i32 = x / (true ? i8(f32(300)) : i8(1));
    return q;
}
"#,
        "error E11212: float-to-integer conversion out of range",
    );
}

#[test]
fn should_report_e11212_for_a_live_short_circuit_operand() {
    assert_trap(
        "live-and-operand",
        r#"
module bench.LiveAndOperand;
func main(): i32 {
    let x: i32 = 7;
    let q: i32 = x / i32(true && (i8(f32(300)) > i8(0)));
    return q;
}
"#,
        "error E11212: float-to-integer conversion out of range",
    );
    assert_trap(
        "live-or-operand",
        r#"
module bench.LiveOrOperand;
func main(): i32 {
    let x: i32 = 7;
    let q: i32 = x / i32(false || (i8(f32(300)) > i8(0)));
    return q;
}
"#,
        "error E11212: float-to-integer conversion out of range",
    );
}

#[test]
fn should_build_a_runtime_boolean_divisor() {
    assert_output(
        "runtime-bool-divisor",
        r#"
module bench.RuntimeBoolDivisor;
import { println } from std.io;
func main(): i32 {
    let x: i32 = 7;
    let flag: bool = true;
    let a: i32 = x / i32(flag);
    let a8: i8 = 3;
    let b8: i8 = 1;
    let b: i32 = x / i32(a8 > b8);
    println($"runtime-bool: {a} {b}");
    return 0;
}
"#,
        "runtime-bool: 7 7\n",
    );
}
