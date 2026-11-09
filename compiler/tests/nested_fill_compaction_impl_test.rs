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
fn should_keep_a_volatile_global_target() {
    let emitted = emit(
        "volatile",
        r#"
module impl.NestedVolatile;
volatile let g: i32[8][9] = [; [; 0]];
func main(): i32 {
    g = [; [; 0]];
    return g[0][0];
}
"#,
    );
    assert_absent("volatile", &emitted, "memset(&(g[0]), 0,");
    assert_contains("volatile", &emitted, "__cl_fillrow");
}

#[test]
fn should_keep_a_field_target() {
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
    assert_contains("field", &emitted, "__cl_fillrow");
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
