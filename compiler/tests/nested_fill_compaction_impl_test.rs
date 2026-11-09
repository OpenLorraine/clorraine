//! Emission pins for nested constant-zero fixed-array fills.
//!
//! The positive pins fail on the base compiler (which emits a row temporary
//! and one `memcpy` per row). The exclusion guards pass on both and fail if
//! the implementation wrongly compacts the excluded shape. Every exclusion
//! uses at least 72 slots so the 64-slot boundary cannot explain the guard.

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
            "clc-nested-fill-impl-{}-{tag}-{sequence}",
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

fn emit(tag: &str, source: &str) -> String {
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
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
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

fn assert_whole_array_memset_count(
    tag: &str,
    emitted: &str,
    target: &str,
    count_expression: &str,
    expected: usize,
) {
    let statement = format!("memset({target}, 0, {count_expression});");
    assert_eq!(
        count(emitted, &statement),
        expected,
        "{tag}: unexpected whole-array memset count"
    );
    assert_absent(tag, emitted, "__cl_fillrow");
}

fn assert_whole_array_memset(tag: &str, emitted: &str, target: &str, count_expression: &str) {
    assert_whole_array_memset_count(tag, emitted, target, count_expression, 1);
}

#[test]
fn should_compact_a_two_dimensional_zero_fill() {
    let emitted = emit(
        "2d",
        r#"
module impl.Nested2D;
func main(): i32 {
    let a: i32[8][8] = [; [; 0]];
    return a[0][0];
}
"#,
    );
    assert_whole_array_memset("2d", &emitted, "&(a[0])", "8 * sizeof(a[0])");
}

#[test]
fn should_compact_a_three_dimensional_zero_fill() {
    let emitted = emit(
        "3d",
        r#"
module impl.Nested3D;
func main(): i32 {
    let a: i32[4][4][4] = [; [; [; 0]]];
    return a[0][0][0];
}
"#,
    );
    assert_whole_array_memset("3d", &emitted, "&(a[0])", "4 * sizeof(a[0])");
}

#[test]
fn should_compact_only_the_covered_rows_after_a_prefix() {
    let emitted = emit(
        "prefix",
        r#"
module impl.NestedPrefix;
func main(): i32 {
    let a: i32[16][8] = [
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1];
        [; 0]
    ];
    return a[0][0];
}
"#,
    );
    assert_whole_array_memset("prefix", &emitted, "&(a[8])", "8 * sizeof(a[8])");
}

#[test]
fn should_keep_the_63_slot_boundary() {
    let emitted = emit(
        "boundary-63",
        r#"
module impl.Nested63;
func main(): i32 {
    let a: i32[7][9] = [; [; 0]];
    return a[0][0];
}
"#,
    );
    assert_absent("boundary-63", &emitted, "memset(&(a[0]), 0, 7 * sizeof(a[0]))");
    assert_contains("boundary-63", &emitted, "__cl_fillrow");
}

#[test]
fn should_compact_the_64_slot_boundary() {
    let emitted = emit(
        "boundary-64",
        r#"
module impl.Nested64;
func main(): i32 {
    let a: i32[8][8] = [; [; 0]];
    return a[0][0];
}
"#,
    );
    assert_whole_array_memset("boundary-64", &emitted, "&(a[0])", "8 * sizeof(a[0])");
}

#[test]
fn should_keep_a_non_zero_nested_fill() {
    let emitted = emit(
        "non-zero",
        r#"
module impl.NestedNonZero;
func main(): i32 {
    let a: i32[8][9] = [; [; 7]];
    return a[0][0];
}
"#,
    );
    assert_absent("non-zero", &emitted, "memset(&(a[0]), 0,");
    assert_contains("non-zero", &emitted, "__cl_fillrow");
}

#[test]
fn should_keep_a_float_leaf() {
    let emitted = emit(
        "float",
        r#"
module impl.NestedFloat;
func main(): i32 {
    let a: f32[8][9] = [; [; 0.0]];
    return 0;
}
"#,
    );
    assert_absent("float", &emitted, "memset(&(a[0]), 0,");
    assert_contains("float", &emitted, "__cl_fillrow");
}

#[test]
fn should_keep_a_struct_row() {
    let emitted = emit(
        "struct",
        r#"
module impl.NestedStruct;
struct R { v: i32[4]; }
func main(): i32 {
    let a: R[8][9] = [; [; R { v: [1, 2, 3, 4] }]];
    return a[0][0].v[0];
}
"#,
    );
    assert_absent("struct", &emitted, "memset(&(a[0]), 0,");
    assert_contains("struct", &emitted, "__cl_fillrow");
}

#[test]
fn should_keep_a_row_literal_with_explicit_elements() {
    let emitted = emit(
        "row-prefix",
        r#"
module impl.NestedRowPrefix;
func main(): i32 {
    let a: i32[16][8] = [; [1, 2, 3, 4, 5, 6, 7, 8]];
    return a[0][0];
}
"#,
    );
    assert_absent("row-prefix", &emitted, "memset(&(a[0]), 0,");
    assert_contains("row-prefix", &emitted, "__cl_fillrow");
}

#[test]
fn should_write_a_volatile_global_nested_fill_element_by_element() {
    let emitted = emit(
        "volatile-2d",
        r#"
module impl.NestedVolatile;
volatile let g: i32[8][9] = [; [; 0]];
func main(): i32 {
    g = [; [; 0]];
    return g[0][0];
}
"#,
    );
    assert_absent("volatile-2d", &emitted, "memcpy(&(cl_impl_NestedVolatile_g[");
    assert_absent("volatile-2d", &emitted, "memset(&(cl_impl_NestedVolatile_g[");
    assert_contains(
        "volatile-2d",
        &emitted,
        "cl_impl_NestedVolatile_g[0][0] = __cl_fillrow_1[0];",
    );
    assert_contains(
        "volatile-2d",
        &emitted,
        "cl_impl_NestedVolatile_g[7][8] = __cl_fillrow_1[8];",
    );
}

#[test]
fn should_keep_a_non_volatile_field_copy() {
    let emitted = emit(
        "field",
        r#"
module impl.NestedField;
struct S { a: i32[8][9]; }
func main(): i32 {
    let s: S = S { a: [; [; 2]] };
    s.a = [; [; 0]];
    return s.a[0][0];
}
"#,
    );
    assert_absent("field", &emitted, "memset(&(s.a[0]), 0,");
    assert_contains("field", &emitted, "memcpy(&(s.a[0]), &(__cl_fillrow_");
    assert_absent("field", &emitted, "s.a[0][0] = __cl_fillrow_");
}

#[test]
fn should_keep_the_one_extent_byte_fill() {
    let emitted = emit(
        "one-byte",
        r#"
module impl.OneByte;
func main(): i32 {
    let a: u8[8192] = [; 1];
    return i32(a[0]);
}
"#,
    );
    assert_contains("one-byte", &emitted, "memset(&(a[0]), 1, 8192);");
}

#[test]
fn should_keep_the_one_extent_integer_loop() {
    let emitted = emit(
        "one-integer",
        r#"
module impl.OneInteger;
func main(): i32 {
    let a: i32[100] = [; 0];
    return a[0];
}
"#,
    );
    assert_absent("one-integer", &emitted, "memset(&(a[0]), 0,");
    assert_contains("one-integer", &emitted, "for (");
}

#[test]
fn should_compact_an_array_parameter_with_the_row_size() {
    let emitted = emit(
        "parameter",
        r#"
module impl.NestedParameter;
func fill(borrow a: i32[8][9]): void {
    a = [; [; 0]];
}
func main(): i32 {
    let a: i32[8][9] = [; [; 0]];
    fill(a);
    return a[0][0];
}
"#,
    );
    assert_whole_array_memset_count(
        "parameter",
        &emitted,
        "&(a[0])",
        "8 * sizeof(a[0])",
        2,
    );
    assert_absent("parameter", &emitted, "8 * sizeof(a)");
}

#[test]
fn should_compact_a_hidden_return_slot() {
    let emitted = emit(
        "return",
        r#"
module impl.NestedReturn;
func make(): i32[8][9] {
    return [; [; 0]];
}
func main(): i32 {
    let a: i32[8][9] = make();
    return a[0][0];
}
"#,
    );
    assert_whole_array_memset(
        "return",
        &emitted,
        "&(__cl_ret[0])",
        "8 * sizeof(__cl_ret[0])",
    );
}

#[test]
fn should_compact_a_full_prefix_without_a_temporary() {
    let emitted = emit(
        "full-prefix",
        r#"
module impl.NestedFullPrefix;
func main(): i32 {
    let row: i32[8] = [; 1];
    let a: i32[8][8] = [
        row, row, row, row, row, row, row, row;
        [; 0]
    ];
    return a[0][0];
}
"#,
    );
    assert_absent("full-prefix", &emitted, "__cl_fillrow");
    assert_absent("full-prefix", &emitted, "if (sizeof(__cl_fillrow))");
}

#[test]
fn should_compact_a_conditional_return_arm() {
    let emitted = emit(
        "conditional-return",
        r#"
module impl.NestedConditional;
func maybe(ok: bool): i32[8][8] {
    if (ok) {
        return [; [; 0]];
    }
    return [; [; 1]];
}
func main(): i32 {
    let a: i32[8][8] = maybe(true);
    return a[0][0];
}
"#,
    );
    assert_contains("conditional-return", &emitted, "memset(&(__cl_ret[0]), 0,");
}

#[test]
fn should_compact_an_integer_cast_zero() {
    let emitted = emit(
        "cast-zero",
        r#"
module impl.NestedCastZero;
func main(): i32 {
    let a: i32[8][8] = [; [; i32(0)]];
    return a[0][0];
}
"#,
    );
    assert_whole_array_memset("cast-zero", &emitted, "&(a[0])", "8 * sizeof(a[0])");
}

#[test]
fn should_compact_the_72_slot_control() {
    let emitted = emit(
        "control-72",
        r#"
module impl.Nested72;
func main(): i32 {
    let a: i32[8][9] = [; [; 0]];
    return a[0][0];
}
"#,
    );
    assert_whole_array_memset("control-72", &emitted, "&(a[0])", "8 * sizeof(a[0])");
}

#[test]
fn should_keep_a_float_cast_zero_leaf() {
    let emitted = emit(
        "float-cast",
        r#"
module impl.NestedFloatCast;
func main(): i32 {
    let a: i32[8][9] = [; [; i32(f32(0.0))]];
    return a[0][0];
}
"#,
    );
    assert_absent("float-cast", &emitted, "memset(&(a[0]), 0,");
    assert_contains("float-cast", &emitted, "__cl_fillrow");
}

#[test]
fn should_emit_nothing_for_a_non_compact_full_prefix() {
    let emitted = emit(
        "field-full-prefix",
        r#"
module impl.NestedFieldFullPrefix;
struct S { a: i32[8][8]; }
func main(): i32 {
    let row: i32[8] = [; 1];
    let s: S = S { a: [
        [2, 2, 2, 2, 2, 2, 2, 2],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [2, 2, 2, 2, 2, 2, 2, 2]
    ] };
    s.a = [row, row, row, row, row, row, row, row; [; 0]];
    return s.a[0][0];
}
"#,
    );
    assert_absent("field-full-prefix", &emitted, "__cl_fillrow");
    assert_absent("field-full-prefix", &emitted, "memset(&(s.a[0]), 0,");
}

#[test]
fn should_write_a_volatile_three_dimensional_fill_element_by_element() {
    let emitted = emit(
        "volatile-3d",
        r#"
module impl.NestedVolatile3;
volatile let g: i32[2][3][4] = [; [; [; 0]]];
func main(): i32 {
    g = [; [; [; 7]]];
    return g[0][0][0];
}
"#,
    );
    assert_absent("volatile-3d", &emitted, "memcpy(&(cl_impl_NestedVolatile3_g[");
    assert_contains(
        "volatile-3d",
        &emitted,
        "cl_impl_NestedVolatile3_g[0][0][0] = __cl_fillrow_",
    );
    assert_contains(
        "volatile-3d",
        &emitted,
        "cl_impl_NestedVolatile3_g[1][2][3] = __cl_fillrow_",
    );
}

#[test]
fn should_write_a_volatile_overlay_field_element_by_element() {
    let emitted = emit(
        "volatile-field",
        r#"
module impl.NestedVolatileField;
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
    assert_absent("volatile-field", &emitted, "memcpy(&(regs->a[");
    assert_contains(
        "volatile-field",
        &emitted,
        "regs->a[0][0] = __cl_fillrow_1[0];",
    );
    assert_contains(
        "volatile-field",
        &emitted,
        "regs->a[1][2] = __cl_fillrow_1[2];",
    );
}

#[test]
fn should_write_a_volatile_struct_fill_element_by_element() {
    let emitted = emit(
        "volatile-struct",
        r#"
module impl.NestedVolatileStruct;
struct R { v: i32; }
volatile let g: R[2][3] = [; [; R { v: 0 }]];
func main(): i32 {
    g = [; [; R { v: 7 }]];
    return g[0][0].v;
}
"#,
    );
    assert_absent("volatile-struct", &emitted, "memcpy(&(cl_impl_NestedVolatileStruct_g[");
    assert_contains(
        "volatile-struct",
        &emitted,
        "cl_impl_NestedVolatileStruct_g[0][0] = __cl_fillrow_1[0];",
    );
}

#[test]
fn should_write_a_volatile_place_prefix_element_by_element() {
    let emitted = emit(
        "volatile-prefix",
        r#"
module impl.NestedPrefixPlace;
volatile let g: i32[4][4] = [; [; 0]];
func main(): i32 {
    let row: i32[4] = [; 1];
    g = [row, row; [; 0]];
    return g[0][0];
}
"#,
    );
    assert_absent("volatile-prefix", &emitted, "memcpy(&(cl_impl_NestedPrefixPlace_g[");
    assert_contains(
        "volatile-prefix",
        &emitted,
        "cl_impl_NestedPrefixPlace_g[0][0] = row[0];",
    );
    assert_contains(
        "volatile-prefix",
        &emitted,
        "cl_impl_NestedPrefixPlace_g[2][0] = __cl_fillrow_1[0];",
    );
}

#[test]
fn should_write_a_volatile_field_named_len_element_by_element() {
    let emitted = emit(
        "volatile-len",
        r#"
module impl.NestedVolatileLen;
struct S { len: volatile i32[2][3]; }
func main(): i32 {
    let regs: raw S = raw S(0x00DFF000);
    unsafe {
        regs.len = [; [; 0]];
    }
    return 0;
}
"#,
    );
    assert_absent("volatile-len", &emitted, "memcpy(&(regs->len[");
    assert_contains(
        "volatile-len",
        &emitted,
        "regs->len[0][0] = __cl_fillrow_1[0];",
    );
}

#[test]
fn should_keep_a_volatile_one_extent_fill() {
    let emitted = emit(
        "volatile-one",
        r#"
module impl.NestedVolatile1;
volatile let g: i32[8] = [; 0];
func main(): i32 {
    g = [; 0];
    return g[0];
}
"#,
    );
    assert_absent("volatile-one", &emitted, "memset(&(cl_impl_NestedVolatile1_g[0])");
    assert_contains("volatile-one", &emitted, "cl_impl_NestedVolatile1_g[0] = 0;");
}

#[test]
fn should_keep_a_volatile_full_prefix_empty() {
    let emitted = emit(
        "volatile-full-prefix",
        r#"
module impl.NestedVolatileFullPrefix;
volatile let g: i32[8][8] = [; [; 0]];
func main(): i32 {
    g = [
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1];
        [; 0]
    ];
    return g[0][0];
}
"#,
    );
    assert_absent("volatile-full-prefix", &emitted, "__cl_fillrow");
    assert_absent(
        "volatile-full-prefix",
        &emitted,
        "memset(&(cl_impl_NestedVolatileFullPrefix_g[",
    );
}

#[test]
fn should_keep_volatile_explicit_rows_direct() {
    let emitted = emit(
        "volatile-rows",
        r#"
module impl.NestedVolatileRows;
volatile let g: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
func main(): i32 {
    g = [[7, 8, 9], [10, 11, 12]];
    return g[0][0];
}
"#,
    );
    assert_absent("volatile-rows", &emitted, "__cl_fillrow");
    assert_contains("volatile-rows", &emitted, "cl_impl_NestedVolatileRows_g[0][0] = 7;");
}

#[test]
fn should_keep_the_whole_array_copy_unchanged() {
    let emitted = emit(
        "volatile-whole-copy",
        r#"
module impl.NestedWholeCopy;
volatile let g: i32[2][2] = [; [; 0]];
func main(): i32 {
    let other: i32[2][2] = [; [; 1]];
    g = other;
    return g[0][0];
}
"#,
    );
    assert_contains(
        "volatile-whole-copy",
        &emitted,
        "memcpy(&(cl_impl_NestedWholeCopy_g), &(other)",
    );
    assert_absent(
        "volatile-whole-copy",
        &emitted,
        "cl_impl_NestedWholeCopy_g[0][0] = other[0][0];",
    );
}

#[test]
fn should_write_a_runtime_indexed_volatile_element_by_element() {
    let emitted = emit(
        "volatile-index",
        r#"
module impl.NestedVolatileIndex;
volatile let g: i32[2][3][4] = [; [; [; 0]]];
func main(): i32 {
    let i: i32 = 1;
    g[i] = [; [; 9]];
    return g[1][0][0];
}
"#,
    );
    assert_absent(
        "volatile-index",
        &emitted,
        "memcpy(&((*(&(cl_impl_NestedVolatileIndex_g[",
    );
    assert_contains("volatile-index", &emitted, "(*(&(cl_impl_NestedVolatileIndex_g[");
    assert_contains("volatile-index", &emitted, ")))[0][0] = __cl_fillrow_");
}

#[test]
fn should_evaluate_an_impure_prefix_row_once_into_a_temporary() {
    let emitted = emit(
        "volatile-impure-prefix",
        r#"
module impl.NestedImpurePrefix;
volatile let g: i32[4][4] = [; [; 0]];
func makeRow(counter: raw i32): i32[4] {
    unsafe { *counter = *counter + 1; }
    return [; 7];
}
func main(): i32 {
    let calls: i32 = 0;
    let p: raw i32 = unsafe &calls;
    g = [makeRow(p); [; 0]];
    return g[0][0];
}
"#,
    );
    assert_absent(
        "volatile-impure-prefix",
        &emitted,
        "memcpy(&(cl_impl_NestedImpurePrefix_g[",
    );
    assert_eq!(
        count(&emitted, "_makeRow(p,"),
        1,
        "volatile-impure-prefix: the prefix value must be evaluated once"
    );
    assert_contains(
        "volatile-impure-prefix",
        &emitted,
        "cl_impl_NestedImpurePrefix_g[0][0] = __cl_fillrow_",
    );
}

#[test]
fn should_write_a_runtime_indexed_volatile_field_by_element() {
    let emitted = emit(
        "volatile-field-index",
        r#"
module impl.NestedFieldIndex;
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
    assert_absent("volatile-field-index", &emitted, "memcpy(&((*(&(regs->a[");
    assert_contains("volatile-field-index", &emitted, "(*(&(regs->a[");
    assert_contains("volatile-field-index", &emitted, ")))[0][0] = __cl_fillrow_");
}

#[test]
fn should_write_a_volatile_pointer_field_by_element() {
    let emitted = emit(
        "volatile-pointer-field",
        r#"
module impl.NestedPointerField;
struct Inner { v: i32[2][3]; }
struct S { p: volatile raw Inner; }
func main(): i32 {
    let regs: raw S = raw S(0x00DFF000);
    unsafe { regs.p.v = [; [; 0]]; }
    return 0;
}
"#,
    );
    assert_absent("volatile-pointer-field", &emitted, "memcpy(&(regs->p->v[");
    assert_contains(
        "volatile-pointer-field",
        &emitted,
        "regs->p->v[0][0] = __cl_fillrow_1[0];",
    );
}

#[test]
fn should_copy_a_constant_indexed_volatile_source_row_directly() {
    let emitted = emit(
        "volatile-source-const",
        r#"
module impl.VolatileSourceConst;
volatile let h: i32[2][4] = [[1, 2, 3, 4], [5, 6, 7, 8]];
volatile let g: i32[4][4] = [; [; 0]];
func main(): i32 {
    g = [h[0], h[1]; [; 0]];
    return g[0][0];
}
"#,
    );
    assert_absent(
        "volatile-source-const",
        &emitted,
        "memcpy(&(cl_impl_VolatileSourceConst_g[",
    );
    assert_contains(
        "volatile-source-const",
        &emitted,
        "cl_impl_VolatileSourceConst_g[0][0] = cl_impl_VolatileSourceConst_h[0][0];",
    );
}

#[test]
fn should_copy_an_arithmetic_indexed_volatile_source_row_directly() {
    let emitted = emit(
        "volatile-source-index",
        r#"
module impl.VolatileSourceIndex;
volatile let h: i32[2][4] = [[1, 2, 3, 4], [5, 6, 7, 8]];
volatile let g: i32[4][4] = [; [; 0]];
func main(): i32 {
    let k: i32 = 2;
    g = [h[k - 1]; [; 0]];
    return g[0][0];
}
"#,
    );
    assert_absent(
        "volatile-source-index",
        &emitted,
        "memcpy(&(cl_impl_VolatileSourceIndex_g[",
    );
    assert_contains(
        "volatile-source-index",
        &emitted,
        "cl_impl_VolatileSourceIndex_g[0][0] = cl_impl_VolatileSourceIndex_h[(i32)((u32)k - (u32)1)][0];",
    );
}
