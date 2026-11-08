//! Emission pins for the compiler robustness closeout.
//!
//! The red pins fail on the starting compiler; the green pins are regression
//! guards for forms the change must leave alone. Guard counts are scoped to the
//! generated function or initializer body, so the runtime shim's own `cl_trap`
//! definition and helpers do not pollute them.

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
            "clc-robustness-impl-{}-{tag}-{sequence}",
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
/// pin; the built binary is the default.
fn clc_path() -> String {
    std::env::var("RDC_CLC").unwrap_or_else(|_| env!("CARGO_BIN_EXE_clc").to_owned())
}

fn run_clc(args: &[String]) -> Output {
    Command::new(clc_path())
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

fn emit(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write the source: {error}"));
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
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// Emits several source files and returns the C for the root file.
fn emit_files(tag: &str, files: &[(&str, &str)], root_name: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let mut paths = Vec::new();
    for (name, source) in files {
        let path = scratch.path().join(name);
        std::fs::write(&path, source)
            .unwrap_or_else(|error| panic!("{tag}: write {name}: {error}"));
        paths.push(path);
    }
    let output_path = scratch.path().join("out.c");
    let root = paths
        .iter()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some(root_name))
        .unwrap_or_else(|| panic!("{tag}: root file {root_name} is missing"));
    let mut args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
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
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// Returns the text of one generated function or initializer body.
///
/// The emitted C also carries a prototype for each function, so the search
/// matches the definition's header followed by an opening brace, never the
/// prototype's semicolon.
fn body<'a>(tag: &str, emitted: &'a str, header: &str) -> &'a str {
    let definition = format!("{header}\n{{");
    let start = emitted
        .find(&definition)
        .unwrap_or_else(|| panic!("{tag}: emitted C has no definition for {header:?}"));
    let rest = &emitted[start..];
    let end = rest.find("\n}").map(|index| index + 2).unwrap_or(rest.len());
    &rest[..end]
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn trap_count(body: &str) -> usize {
    count(body, "cl_trap(\"E1121")
}

fn assert_contains(tag: &str, haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

fn assert_absent(tag: &str, haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "{tag}: emitted C unexpectedly contains {needle:?}"
    );
}

// ---------------------------------------------------------------------------
// D1 — fill guard counts
// ---------------------------------------------------------------------------

#[test]
fn should_emit_one_guard_per_pure_statement_fill() {
    let emitted = emit(
        "fill-guard-count",
        r#"
module fuzz.FillGuardCount;
func main(): i32 {
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let a: u8[4] = [; src[i]];
    return i32(a[0]);
}
"#,
    );
    let main = body("fill-guard-count", &emitted, "static i32 cl_fuzz_FillGuardCount_main(void)");
    assert_eq!(trap_count(main), 1, "fill-guard-count: expected one bounds guard");
}

#[test]
fn should_emit_one_guard_for_a_module_array() {
    let emitted = emit(
        "module-fill-count",
        r#"
module fuzz.ModuleFillCount;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let plain: u8[4] = [; src[i]];
func main(): i32 {
    return i32(plain[0]);
}
"#,
    );
    let init = body("module-fill-count", &emitted, "static void cl_init_fuzz_ModuleFillCount(void)");
    assert_eq!(trap_count(init), 1, "module-fill-count: expected one fill guard");
}

#[test]
fn should_emit_one_guard_per_prefix_element_and_fill() {
    let emitted = emit(
        "module-prefix-count",
        r#"
module fuzz.ModulePrefixCount;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let j: i32 = 10;
let plain: u8[4] = [src[i], 7; src[j]];
func main(): i32 {
    return i32(plain[0]);
}
"#,
    );
    let init = body("module-prefix-count", &emitted, "static void cl_init_fuzz_ModulePrefixCount(void)");
    assert_eq!(
        trap_count(init),
        2,
        "module-prefix-count: one guard for the prefix element and one for the fill"
    );
}

#[test]
fn should_restore_the_fill_guard_flag_after_a_module_fill() {
    let emitted = emit(
        "module-flag-restore",
        r#"
module fuzz.ModuleFlagRestore;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let j: i32 = 10;
let first: u8[4] = [; src[i]];
let second: u8[4] = [; src[j]];
func main(): i32 {
    let a: u8[4] = [; src[i]];
    return i32(a[0]) + i32(first[0]) + i32(second[0]);
}
"#,
    );
    let init = body("module-flag-restore", &emitted, "static void cl_init_fuzz_ModuleFlagRestore(void)");
    let main = body("module-flag-restore", &emitted, "static i32 cl_fuzz_ModuleFlagRestore_main(void)");
    assert_eq!(
        trap_count(init),
        2,
        "module-flag-restore: each module fill keeps its own guard"
    );
    assert_eq!(trap_count(main), 1, "module-flag-restore: the statement fill keeps its guard");
}

#[test]
fn should_emit_one_guard_for_a_struct_literal_field() {
    let emitted = emit(
        "struct-fill-count",
        r#"
module fuzz.StructFillCount;
struct S { a: u8[4]; }
func main(): i32 {
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let s: S = S { a: [; src[i]] };
    return i32(s.a[0]);
}
"#,
    );
    let main = body("struct-fill-count", &emitted, "static i32 cl_fuzz_StructFillCount_main(void)");
    assert_eq!(trap_count(main), 1, "struct-fill-count: expected one field guard");
}

#[test]
fn should_emit_one_guard_for_a_module_struct_literal_field() {
    let emitted = emit(
        "module-struct-count",
        r#"
module fuzz.ModuleStructCount;
struct S { a: u8[4]; }
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let s: S = S { a: [; src[i]] };
func main(): i32 {
    return i32(s.a[0]);
}
"#,
    );
    let init = body("module-struct-count", &emitted, "static void cl_init_fuzz_ModuleStructCount(void)");
    assert_eq!(trap_count(init), 1, "module-struct-count: the walk guard is the only copy");
}

#[test]
fn should_emit_two_guards_for_a_module_struct_prefix_and_fill() {
    let emitted = emit(
        "module-struct-prefix-count",
        r#"
module fuzz.ModuleStructPrefixCount;
struct S { a: u8[4]; }
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let j: i32 = 10;
let s: S = S { a: [src[i], 7; src[j]] };
func main(): i32 {
    return i32(s.a[0]);
}
"#,
    );
    let init = body("module-struct-prefix-count", &emitted, "static void cl_init_fuzz_ModuleStructPrefixCount(void)");
    assert_eq!(
        trap_count(init),
        2,
        "module-struct-prefix-count: one prefix guard and one fill guard"
    );
}

#[test]
fn should_emit_one_guard_for_a_module_array_of_structs_conditional_field() {
    let emitted = emit(
        "module-struct-array-count",
        r#"
module fuzz.ModuleStructArrayCount;
struct S { a: u8[4]; }
let flag: i32 = 1;
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let s: S[2] = [; S { a: flag != 0 ? [; src[i]] : [; src[0]] }];
func main(): i32 {
    return i32(s[0].a[0]);
}
"#,
    );
    let init = body("module-struct-array-count", &emitted, "static void cl_init_fuzz_ModuleStructArrayCount(void)");
    assert_eq!(
        trap_count(init),
        1,
        "module-struct-array-count: the selected arm carries one guard"
    );
}

#[test]
fn should_emit_one_guard_for_a_guarded_fill_condition() {
    let emitted = emit(
        "fill-condition-count",
        r#"
module fuzz.FillConditionCount;
func main(): i32 {
    let src: u8[4] = [1, 2, 3, 4];
    let i: i32 = 10;
    let a: u8[4] = [; src[i] != 0 ? 1 : 2];
    return i32(a[0]);
}
"#,
    );
    let main = body("fill-condition-count", &emitted, "static i32 cl_fuzz_FillConditionCount_main(void)");
    assert_eq!(
        trap_count(main),
        1,
        "fill-condition-count: the condition guard runs once"
    );
}

// ---------------------------------------------------------------------------
// D2/D4 — narrow result wrapping
// ---------------------------------------------------------------------------

#[test]
fn should_emit_a_result_cast_for_unsigned_arithmetic() {
    let emitted = emit(
        "narrow-add-pin",
        r#"
module fuzz.NarrowAddPin;
func main(): i32 {
    let a: i32 = i32(u8(200) + u8(100));
    let b: i32 = i32(u8(200) * u8(200));
    return a + b;
}
"#,
    );
    assert_contains(
        "narrow-add-pin",
        &emitted,
        "(u8)((unsigned int)(u8)(200) + (unsigned int)(u8)(100))",
    );
    assert_contains(
        "narrow-add-pin",
        &emitted,
        "(u8)((unsigned int)(u8)(200) * (unsigned int)(u8)(200))",
    );
}

#[test]
fn should_emit_a_result_cast_for_signed_narrow_arithmetic() {
    let emitted = emit(
        "narrow-signed-pin",
        r#"
module fuzz.NarrowSignedPin;
func mul(a: i16, b: i16): i16 {
    return a * b;
}
func main(): i32 {
    let a: i16 = i16(-1);
    let b: i16 = i16(-1);
    return i32(mul(a, b));
}
"#,
    );
    assert_contains(
        "narrow-signed-pin",
        &emitted,
        "(i16)((unsigned int)a * (unsigned int)b)",
    );
}

#[test]
fn should_emit_a_result_cast_for_an_unsigned_shift() {
    let emitted = emit(
        "narrow-shift-pin",
        r#"
module fuzz.NarrowShiftPin;
func main(): i32 {
    let a: i32 = i32(u8(128) << 1);
    return a;
}
"#,
    );
    assert_contains("narrow-shift-pin", &emitted, "(u8)((u8)(128) << (1 & 7))");
}

#[test]
fn should_emit_a_result_cast_for_a_16_bit_multiply() {
    let emitted = emit(
        "narrow-mul-pin",
        r#"
module fuzz.NarrowMulPin;
func main(): i32 {
    let a: i32 = i32(u16(65535) * u16(65535));
    return a;
}
"#,
    );
    assert_contains(
        "narrow-mul-pin",
        &emitted,
        "(u16)((unsigned int)(u16)(65535) * (unsigned int)(u16)(65535))",
    );
}

// ---------------------------------------------------------------------------
// D6 — enum member values and conversion
// ---------------------------------------------------------------------------

#[test]
fn should_emit_folded_enum_members() {
    let emitted = emit(
        "enum-macro-pin",
        r#"
module fuzz.EnumMacroPin;
enum E: u8 { A = u8(2) + u8(3), B }
func main(): i32 {
    return i32(E.A) + i32(E.B);
}
"#,
    );
    assert_contains("enum-macro-pin", &emitted, "#define E_A 5");
    assert_contains("enum-macro-pin", &emitted, "#define E_B 6");
}

#[test]
fn should_emit_in_range_enum_macros() {
    let emitted = emit(
        "enum-in-range-pin",
        r#"
module fuzz.EnumInRangePin;
enum E: u8 { A = 254, B }
func main(): i32 {
    return i32(E.B);
}
"#,
    );
    assert_contains("enum-in-range-pin", &emitted, "#define E_A 254");
    assert_contains("enum-in-range-pin", &emitted, "#define E_B 255");
}

#[test]
fn should_keep_an_inexact_type_name_call_on_the_call_path() {
    let emitted = emit(
        "inexact-cast-pin",
        r#"
module fuzz.InexactCastPin;
enum E: u8 { A }
func main(): i32 {
    let x: u8 = 2;
    let a: E = E<i32>(2);
    let b: E = E(borrow x);
    return i32(a) + i32(b);
}
"#,
    );
    assert_contains("inexact-cast-pin", &emitted, "E(2)");
    assert_contains("inexact-cast-pin", &emitted, "E(x)");
    assert_absent("inexact-cast-pin", &emitted, "(ClE)");
}

#[test]
fn should_emit_an_enum_conversion_as_a_cast() {
    let emitted = emit(
        "enum-cast-pin",
        r#"
module fuzz.EnumCastPin;
enum Direction: u8 { UP, DOWN, LEFT = 10, RIGHT }
func main(): i32 {
    let d: Direction = Direction(2);
    return i32(d);
}
"#,
    );
    assert_contains("enum-cast-pin", &emitted, "(ClDirection)(2)");
}

#[test]
fn should_emit_a_runtime_enum_conversion_as_a_cast() {
    let emitted = emit(
        "enum-cast-runtime-pin",
        r#"
module fuzz.EnumCastRuntimePin;
enum Direction: u8 { UP, DOWN, LEFT = 10, RIGHT }
func main(): i32 {
    let n: u8 = 2;
    let d: Direction = Direction(n);
    return i32(d);
}
"#,
    );
    assert_contains("enum-cast-runtime-pin", &emitted, "(ClDirection)(n)");
}

#[test]
fn should_keep_an_ffi_function_call_before_an_enum_type() {
    let emitted = emit_files(
        "enum-ffi-shadow-pin",
        &[
            (
                "holder.clo",
                r#"
module fuzz.EnumFfiHolder;
export enum E: u8 { A }
"#,
            ),
            (
                "main.clo",
                r#"
module fuzz.EnumFfiShadowPin;
ffi probe { func E(x: i32): i32; }
func main(): i32 {
    return E(2);
}
"#,
            ),
        ],
        "main.clo",
    );
    assert_contains("enum-ffi-shadow-pin", &emitted, "return E(2);");
    assert_absent("enum-ffi-shadow-pin", &emitted, "(ClE)(2)");
}

#[test]
fn should_emit_a_cross_module_enum_member() {
    let emitted = emit_files(
        "enum-cross-module-pin",
        &[
            (
                "holder.clo",
                r#"
module fuzz.EnumHolderPin;
export enum E: u8 { A = u8(1) + u8(2), B }
"#,
            ),
            (
                "main.clo",
                r#"
module fuzz.EnumCrossModulePin;
import { E } from fuzz.EnumHolderPin;
func main(): i32 {
    return i32(E.A) + i32(E.B);
}
"#,
            ),
        ],
        "main.clo",
    );
    assert_contains("enum-cross-module-pin", &emitted, "#define E_A 3");
    assert_contains("enum-cross-module-pin", &emitted, "#define E_B 4");
}

// ---------------------------------------------------------------------------
// D3 — inline constant shift divisor
// ---------------------------------------------------------------------------

#[test]
fn should_emit_the_masked_count_for_an_inline_shift_divisor() {
    let emitted = emit(
        "shift-divisor-pin",
        r#"
module fuzz.ShiftDivisorPin;
func main(): i32 {
    let q: u8 = u8(10) / (u8(128) >> 8);
    return i32(q);
}
"#,
    );
    let main = body("shift-divisor-pin", &emitted, "static i32 cl_fuzz_ShiftDivisorPin_main(void)");
    assert_contains("shift-divisor-pin", main, ">> (8 & 7)");
    assert_absent("shift-divisor-pin", main, "E11210");
}

#[test]
fn should_emit_one_guard_for_a_module_struct_guarded_condition() {
    let emitted = emit(
        "module-struct-cond-count",
        r#"
module fuzz.ModuleStructCondCount;
struct S { a: u8[4]; }
let src: u8[4] = [1, 2, 3, 4];
let j: i32 = 10;
let s: S = S { a: src[j] != 0 ? [; 1] : [; 2] };
func main(): i32 {
    return i32(s.a[0]);
}
"#,
    );
    let init = body("module-struct-cond-count", &emitted, "static void cl_init_fuzz_ModuleStructCondCount(void)");
    assert_eq!(
        trap_count(init),
        1,
        "module-struct-cond-count: the walk condition guard is the only copy"
    );
}

#[test]
fn should_emit_no_guard_for_a_guard_free_fill() {
    let emitted = emit(
        "guard-free-count",
        r#"
module fuzz.GuardFreeCount;
func main(): i32 {
    let a: u8[4] = [; 7];
    return i32(a[0]);
}
"#,
    );
    let main = body("guard-free-count", &emitted, "static i32 cl_fuzz_GuardFreeCount_main(void)");
    assert_eq!(trap_count(main), 0, "guard-free-count: no guard is expected");
    assert_absent("guard-free-count", main, "E11211");
}
