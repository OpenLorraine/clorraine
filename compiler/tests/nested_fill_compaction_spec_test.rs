//! Behaviour oracle for nested constant-zero fixed-array fills.
//!
//! Every running case compiles a program, builds it on the host with a strict
//! C compiler, runs it, and compares the output. The original cases pass on
//! the base compiler and must keep passing. The volatile-storage cases are
//! deliberately red on the base: the base emits `memcpy` into a volatile
//! object, which strict C rejects. The emitted-shape pins live in the
//! implementation suite.

// This suite participates in the host sanitizer legs. The constants are the
// discovery markers; the builder below adds the flags when the runner sets the
// environment variable.
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
            "clc-nested-fill-spec-{}-{tag}-{sequence}",
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

/// Builds a program but does not run it.
///
/// The volatile overlay case writes through a raw register address that is not
/// a real host object, so only the strict C validity can be checked.
fn assert_builds(tag: &str, source: &str) {
    let (scratch, emitted) = emit(tag, source);
    let _program = build_host(tag, &scratch, &emitted);
}

#[test]
fn should_zero_a_two_dimensional_fill() {
    assert_output(
        "2d-zero",
        r#"
module spec.Nested2D;
import { println } from std.io;
func main(): i32 {
    let a: i32[8][8] = [; [; 0]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 8) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_zero_a_three_dimensional_fill() {
    assert_output(
        "3d-zero",
        r#"
module spec.Nested3D;
import { println } from std.io;
func main(): i32 {
    let a: i32[4][4][4] = [; [; [; 0]]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 4) {
        let j: i32 = 0;
        while (j < 4) {
            let k: i32 = 0;
            while (k < 4) {
                total = total + a[i][j][k];
                k = k + 1;
            }
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_keep_a_prefix_and_zero_the_tail() {
    assert_output(
        "prefix",
        r#"
module spec.NestedPrefix;
import { println } from std.io;
func main(): i32 {
    let row: i32[8] = [; 1];
    let a: i32[16][8] = [
        row, row, row, row, row, row, row, row;
        [; 0]
    ];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 16) {
        let j: i32 = 0;
        while (j < 8) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 64\n",
    );
}

#[test]
fn should_keep_a_non_zero_nested_fill() {
    assert_output(
        "non-zero",
        r#"
module spec.NestedNonZero;
import { println } from std.io;
func main(): i32 {
    let a: i32[8][9] = [; [; 7]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 9) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 504\n",
    );
}

#[test]
fn should_zero_u8_and_bool_nested_fills() {
    assert_output(
        "u8-bool",
        r#"
module spec.NestedBytes;
import { println } from std.io;
func main(): i32 {
    let bytes: u8[16][16] = [; [; 0]];
    bytes[3][4] = 9;
    let flags: bool[8][8] = [; [; false]];
    let trues: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 8) {
            if (flags[i][j]) { trues = trues + 1; }
            j = j + 1;
        }
        i = i + 1;
    }
    println($"byte: {i32(bytes[3][4])} zero: {i32(bytes[0][0])} trues: {trues}");
    return 0;
}
"#,
        "byte: 9 zero: 0 trues: 0\n",
    );
}

#[test]
fn should_zero_an_array_parameter() {
    assert_output(
        "parameter",
        r#"
module spec.NestedParameter;
import { println } from std.io;
func fill(borrow a: i32[8][9]): void {
    a = [; [; 0]];
}
func main(): i32 {
    let a: i32[8][9] = [; [; 1]];
    fill(a);
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 9) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_zero_a_hidden_return_slot() {
    assert_output(
        "return-slot",
        r#"
module spec.NestedReturn;
import { println } from std.io;
func make(): i32[8][9] {
    return [; [; 0]];
}
func main(): i32 {
    let a: i32[8][9] = make();
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 9) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_zero_an_address_taken_array() {
    assert_output(
        "address-taken",
        r#"
module spec.NestedAddress;
import { println } from std.io;
func main(): i32 {
    let a: i32[8][9] = [; [; 0]];
    let p: raw i32 = unsafe &a[0][0];
    if (p == null) { return 1; }
    println($"first: {a[0][0]}");
    return 0;
}
"#,
        "first: 0\n",
    );
}

#[test]
fn should_keep_a_struct_row_non_compact() {
    assert_output(
        "struct-row",
        r#"
module spec.NestedStruct;
import { println } from std.io;
struct R { v: i32[4]; }
func main(): i32 {
    let a: R[8][9] = [; [; R { v: [1, 2, 3, 4] }]];
    println($"first: {a[0][0].v[0]} last: {a[7][8].v[3]}");
    return 0;
}
"#,
        "first: 1 last: 4\n",
    );
}

#[test]
fn should_keep_the_below_threshold_control() {
    assert_output(
        "below-threshold",
        r#"
module spec.NestedSmall;
import { println } from std.io;
func main(): i32 {
    let a: i32[7][9] = [; [; 0]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 7) {
        let j: i32 = 0;
        while (j < 9) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_handle_a_full_prefix() {
    assert_output(
        "full-prefix",
        r#"
module spec.NestedFullPrefix;
import { println } from std.io;
func main(): i32 {
    let row: i32[8] = [; 1];
    let a: i32[8][8] = [
        row, row, row, row, row, row, row, row;
        [; 0]
    ];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 8) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 64\n",
    );
}

#[test]
fn should_accept_an_integer_cast_zero() {
    assert_output(
        "cast-zero",
        r#"
module spec.NestedCastZero;
import { println } from std.io;
func main(): i32 {
    let a: i32[8][8] = [; [; i32(0)]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 8) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_zero_a_conditional_return_arm() {
    assert_output(
        "conditional-return",
        r#"
module spec.NestedConditional;
import { println } from std.io;
func maybe(ok: bool): i32[8][8] {
    if (ok) {
        return [; [; 0]];
    }
    return [; [; 1]];
}
func main(): i32 {
    let a: i32[8][8] = maybe(true);
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 8) {
            total = total + a[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_zero_a_volatile_global_nested_fill() {
    assert_output(
        "volatile-2d",
        r#"
module spec.NestedFillVolatile;
import { println } from std.io;
volatile let g: i32[8][9] = [; [; 0]];
func main(): i32 {
    g = [; [; 0]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 8) {
        let j: i32 = 0;
        while (j < 9) {
            total = total + g[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 0\n",
    );
}

#[test]
fn should_fill_a_volatile_global_three_dimensional_array() {
    assert_output(
        "volatile-3d",
        r#"
module spec.NestedFillVolatile3;
import { println } from std.io;
volatile let g: i32[2][3][4] = [; [; [; 0]]];
func main(): i32 {
    g = [; [; [; 7]]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        let j: i32 = 0;
        while (j < 3) {
            let k: i32 = 0;
            while (k < 4) {
                total = total + g[i][j][k];
                k = k + 1;
            }
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 168\n",
    );
}

#[test]
fn should_fill_a_volatile_global_struct_array() {
    assert_output(
        "volatile-struct",
        r#"
module spec.NestedFillVolatileStruct;
import { println } from std.io;
struct R { v: i32; }
volatile let g: R[2][3] = [; [; R { v: 0 }]];
func main(): i32 {
    g = [; [; R { v: 9 }]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        let j: i32 = 0;
        while (j < 3) {
            total = total + g[i][j].v;
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 54\n",
    );
}

#[test]
fn should_keep_a_volatile_literal_prefix_and_zero_the_tail() {
    assert_output(
        "volatile-literal-prefix",
        r#"
module spec.NestedFillVolatilePrefix;
import { println } from std.io;
volatile let g: i32[4][4] = [; [; 0]];
func main(): i32 {
    g = [[1, 2, 3, 4], [5, 6, 7, 8]; [; 0]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 4) {
        let j: i32 = 0;
        while (j < 4) {
            total = total + g[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 36\n",
    );
}

#[test]
fn should_fill_a_volatile_place_prefix_and_zero_the_tail() {
    assert_output(
        "volatile-place-prefix",
        r#"
module spec.NestedFillVolatilePlacePrefix;
import { println } from std.io;
volatile let g: i32[4][4] = [; [; 0]];
func main(): i32 {
    let row: i32[4] = [; 1];
    g = [row, row; [; 0]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 4) {
        let j: i32 = 0;
        while (j < 4) {
            total = total + g[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 8\n",
    );
}

#[test]
fn should_build_a_volatile_overlay_field_fill() {
    assert_builds(
        "volatile-overlay",
        r#"
module spec.NestedFillVolatileOverlay;
struct S { a: volatile i32[2][3]; }
func main(): i32 {
    let regs: raw S = raw S(0x00DFF000);
    unsafe {
        regs.a = [; [; 0]];
    }
    return 0;
}
"#,
    );
}

#[test]
fn should_fill_a_runtime_indexed_volatile_element() {
    assert_output(
        "volatile-index",
        r#"
module spec.NestedFillVolatileIndex;
import { println } from std.io;
volatile let g: i32[2][3][4] = [; [; [; 0]]];
func main(): i32 {
    let i: i32 = 1;
    g[i] = [; [; 9]];
    let total: i32 = 0;
    let j: i32 = 0;
    while (j < 3) {
        let k: i32 = 0;
        while (k < 4) {
            total = total + g[1][j][k];
            k = k + 1;
        }
        j = j + 1;
    }
    println($"sum: {total}");
    return 0;
}
"#,
        "sum: 108\n",
    );
}

#[test]
fn should_evaluate_an_impure_prefix_row_once() {
    assert_output(
        "volatile-impure-prefix",
        r#"
module spec.NestedFillImpurePrefix;
import { println } from std.io;
volatile let g: i32[4][4] = [; [; 0]];
func makeRow(counter: raw i32): i32[4] {
    unsafe { *counter = *counter + 1; }
    return [; 7];
}
func main(): i32 {
    let calls: i32 = 0;
    let p: raw i32 = unsafe &calls;
    g = [makeRow(p); [; 0]];
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 4) {
        let j: i32 = 0;
        while (j < 4) {
            total = total + g[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    println($"calls: {calls} sum: {total}");
    return 0;
}
"#,
        "calls: 1 sum: 28\n",
    );
}

#[test]
fn should_build_a_runtime_indexed_volatile_field_fill() {
    assert_builds(
        "volatile-field-index",
        r#"
module spec.NestedFillVolatileFieldIndex;
struct S { a: volatile i32[2][3][4]; }
func fillField(i: i32): void {
    let regs: raw S = raw S(0x00DFF000);
    unsafe { regs.a[i] = [; [; 9]]; }
}
func main(): i32 {
    fillField(1);
    return 0;
}
"#,
    );
}

#[test]
fn should_build_a_volatile_pointer_field_fill() {
    assert_builds(
        "volatile-pointer-field",
        r#"
module spec.NestedFillVolatilePointerField;
struct Inner { v: i32[2][3]; }
struct S { p: volatile raw Inner; }
func main(): i32 {
    let regs: raw S = raw S(0x00DFF000);
    unsafe { regs.p.v = [; [; 0]]; }
    return 0;
}
"#,
    );
}
