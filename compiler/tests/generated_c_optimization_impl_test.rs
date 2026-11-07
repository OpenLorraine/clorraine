//! Emission pins for the generated-C optimization feature.
//!
//! The red pins fail on the base compiler; the green pins are regression
//! guards for forms the change must leave alone. Every pin names the emitted
//! line it protects.

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
            "clc-genopt-impl-{}-{tag}-{sequence}",
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

fn emit(tag: &str, files: &[(&str, &str)], root_name: &str) -> String {
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
        "host".to_owned(),
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

#[test]
fn should_fold_a_const_loop_bound_and_keep_its_storage() {
    let emitted = emit(
        "const-fold",
        &[(
            "main.clo",
            r#"
module bench.ImplConst;
const N: i32 = 8192;
func main(): i32 {
    let i: i32 = 0;
    while (i < N) { i = i + 1; }
    return i;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("const-fold", &emitted, "while ((i < 8192))");
    assert_contains("const-fold", &emitted, "static i32 cl_bench_ImplConst_N = 0;");
    assert_contains("const-fold", &emitted, "cl_bench_ImplConst_N = 8192;");
    assert_eq!(
        count(&emitted, "cl_bench_ImplConst_N"),
        2,
        "const-fold: the storage declaration and initializer remain, the loop reads the literal"
    );
}

#[test]
fn should_compact_a_byte_fill_with_memset() {
    let emitted = emit(
        "fill-byte",
        &[(
            "main.clo",
            r#"
module bench.ImplFillByte;
func main(): i32 {
    let a: u8[4096] = [; 1];
    return i32(a[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_contains("fill-byte", &emitted, "memset(&(a[0]), 1, 4096);");
    assert_eq!(
        count(&emitted, "] = 1u;"),
        0,
        "fill-byte: the fill is one memset, not per-element stores"
    );
}

#[test]
fn should_compact_an_integer_fill_with_one_loop() {
    let emitted = emit(
        "fill-int",
        &[(
            "main.clo",
            r#"
module bench.ImplFillInt;
func main(): i32 {
    let a: i32[4096] = [; 7];
    return a[0];
}
"#,
        )],
        "main.clo",
    );
    assert_contains(
        "fill-int",
        &emitted,
        "for (__cl_fill_1 = 0; __cl_fill_1 < 4096; __cl_fill_1++)",
    );
    assert_contains("fill-int", &emitted, "a[__cl_fill_1] = 7;");
    assert_absent("fill-int", &emitted, "a[4095] = 7;");
}

#[test]
fn should_keep_a_small_fill_unrolled_and_compact_the_threshold() {
    let small = emit(
        "fill-small",
        &[(
            "main.clo",
            r#"
module bench.ImplFillSmall;
func main(): i32 {
    let a: u8[15] = [; 1];
    return i32(a[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-small", &small, "memset(&(a[0])");
    assert_eq!(
        count(&small, "] = 1u;"),
        15,
        "fill-small: fifteen direct stores"
    );
    let threshold = emit(
        "fill-threshold",
        &[(
            "main.clo",
            r#"
module bench.ImplFillThreshold;
func main(): i32 {
    let a: u8[16] = [; 1];
    return i32(a[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_contains("fill-threshold", &threshold, "memset(&(a[0]), 1, 16);");
}

#[test]
fn should_fill_an_adjusted_parameter_and_a_return_slot() {
    let parameter = emit(
        "fill-param",
        &[(
            "main.clo",
            r#"
module bench.ImplFillParam;
func fill(borrow a: u8[4096]): void {
    a = [; 1];
}
func main(): i32 { return 0; }
"#,
        )],
        "main.clo",
    );
    assert_contains("fill-param", &parameter, "memset(&(a[0]), 1, 4096);");
    assert_absent("fill-param", &parameter, "memset(&(a[0]), 1, sizeof");

    let returns = emit(
        "fill-return",
        &[(
            "main.clo",
            r#"
module bench.ImplFillReturn;
func make(): u8[4096] {
    return [; 1];
}
func main(): i32 { return i32(make()[0]); }
"#,
        )],
        "main.clo",
    );
    assert_contains(
        "fill-return",
        &returns,
        "memset(&(__cl_ret[0]), 1, 4096);",
    );
}

#[test]
fn should_omit_the_lower_guard_for_a_non_negative_constant_index() {
    let emitted = emit(
        "guard-slice",
        &[(
            "main.clo",
            r#"
module bench.ImplGuardSlice;
import { slice } from std.mem;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    return i32(s[1]);
}
"#,
        )],
        "main.clo",
    );
    assert_contains(
        "guard-slice",
        &emitted,
        "if ((usize)(1) >= (usize)((s).len)) cl_trap(\"E11211\", \"index out of range\");",
    );
    assert_absent("guard-slice", &emitted, "(1 < 0)");
}

#[test]
fn should_keep_the_lower_guard_for_a_negative_constant_index() {
    let emitted = emit(
        "guard-negative",
        &[(
            "main.clo",
            r#"
module bench.ImplGuardNegative;
import { slice } from std.mem;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    return i32(s[-1]);
}
"#,
        )],
        "main.clo",
    );
    assert_contains(
        "guard-negative",
        &emitted,
        "((i32)(-(u32)1) < 0)",
    );
}

#[test]
fn should_omit_a_constant_division_guard() {
    let emitted = emit(
        "guard-div",
        &[(
            "main.clo",
            r#"
module bench.ImplGuardDiv;
func main(): i32 {
    let x: i32 = 10;
    return x % 2;
}
"#,
        )],
        "main.clo",
    );
    assert_absent("guard-div", &emitted, "2 == 0");
    assert_absent("guard-div", &emitted, "2 == -1");
}

#[test]
fn should_keep_only_the_min_check_for_a_minus_one_divisor() {
    let emitted = emit(
        "guard-min",
        &[(
            "main.clo",
            r#"
module bench.ImplGuardMin;
func main(): i32 {
    let min: i32 = -2147483648;
    let r: i32 = min / -1;
    return r;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("guard-min", &emitted, "min == -2147483648");
    assert_absent("guard-min", &emitted, "-1 == 0");
    assert_absent("guard-min", &emitted, "== -1)");
}

#[test]
fn should_keep_an_address_taken_const_as_a_place() {
    let emitted = emit(
        "address-taken",
        &[(
            "main.clo",
            r#"
module bench.ImplAddress;
const N: i32 = 5;
func main(): i32 {
    let p: raw i32 = unsafe &N;
    unsafe { *p = 6; }
    return N - 5;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("address-taken", &emitted, "&(cl_bench_ImplAddress_N)");
    assert_absent("address-taken", &emitted, "&(5)");
    assert_eq!(
        count(&emitted, "cl_bench_ImplAddress_N"),
        4,
        "address-taken: declaration, initializer, address, and the read stay visible"
    );
}

#[test]
fn should_keep_a_shadowing_local_as_a_local() {
    let emitted = emit(
        "shadow",
        &[(
            "main.clo",
            r#"
module bench.ImplShadow;
const N: i32 = 5;
func main(): i32 {
    let N: i32 = 3;
    return N;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("shadow", &emitted, "i32 N = 0;");
    assert_contains("shadow", &emitted, "N = 3;");
    assert_contains("shadow", &emitted, "return N;");
    assert_eq!(
        count(&emitted, "cl_bench_ImplShadow_N"),
        2,
        "shadow: the module const is only declared and initialized"
    );
}

#[test]
fn should_emit_byte_identical_c_twice() {
    let source = r#"
module bench.ImplDeterminism;
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
    let first = scratch.path().join("first.c");
    let second = scratch.path().join("second.c");
    for output in [&first, &second] {
        let run = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
            source_path.display().to_string(),
        ]);
        assert_eq!(run.status.code(), Some(0));
    }
    assert_eq!(
        std::fs::read_to_string(&first).expect("read first"),
        std::fs::read_to_string(&second).expect("read second"),
    );
}

#[test]
fn should_keep_a_volatile_place_read_in_the_compact_loop() {
    let emitted = emit(
        "fill-volatile",
        &[(
            "main.clo",
            r#"
module bench.ImplVolatile;
volatile let seed: u8 = 3;
func main(): i32 {
    let a: u8[4096] = [; seed];
    return i32(a[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-volatile", &emitted, "for (__cl_fill_");
    assert_contains("fill-volatile", &emitted, "a[4095] = cl_bench_ImplVolatile_seed;");
    assert_absent("fill-volatile", &emitted, "memset(&(a[0])");
}

#[test]
fn should_leave_a_materialized_fill_unrolled() {
    let emitted = emit(
        "fill-materialized",
        &[(
            "main.clo",
            r#"
module bench.ImplMaterialized;
func eat(a: u8[4096]): i32 { return i32(a[0]); }
func main(): i32 {
    let ok: bool = true;
    if (ok && eat([; 1]) == 1) { return 0; }
    return 1;
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-materialized", &emitted, "memset(&(__cl_owned");
    assert_contains("fill-materialized", &emitted, "[4095] = 1u");
}

#[test]
fn should_fold_a_shift_at_the_declared_width() {
    let emitted = emit(
        "shift",
        &[(
            "main.clo",
            r#"
module bench.ImplShift;
const S: i32 = 1 << 31;
const T: i32 = 1 << 32;
func main(): i32 {
    return S + T;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("shift", &emitted, "(i32)(-(u32)2147483648u)");
    assert_absent("shift", &emitted, "2147483648u < 0");
}

#[test]
fn should_leave_a_non_finite_float_const_as_a_place() {
    let emitted = emit(
        "inf",
        &[(
            "main.clo",
            r#"
module bench.ImplInf;
const INF: f64 = 1.0 / 0.0;
func main(): i32 {
    let x: f64 = INF;
    return 0;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("inf", &emitted, "cl_bench_ImplInf_INF");
    assert_absent("inf", &emitted, "inf.0");
    assert_absent("inf", &emitted, "NaN.0");
}

#[test]
fn should_fold_a_bare_qualified_const_and_keep_a_member() {
    let holder = r#"
module bench.ImplHolder;
export const N: i32 = 7;
export struct Pair { n: i32; }
export let G: Pair = Pair { n: 9 };
"#;
    let main = r#"
module bench.ImplQualified;
import { N, G } from bench.ImplHolder;
func main(): i32 {
    return N + G.n;
}
"#;
    let emitted = emit(
        "qualified",
        &[("holder.clo", holder), ("main.clo", main)],
        "main.clo",
    );
    assert_contains("qualified", &emitted, "return (i32)((u32)7 + (u32)cl_bench_ImplHolder_G.n);");
    assert_absent("qualified", &emitted, "cl_bench_ImplHolder_N +");
}

#[test]
fn should_evaluate_an_impure_intrinsic_fill_once() {
    let emitted = emit(
        "fill-intrinsic",
        &[(
            "main.clo",
            r#"
module bench.ImplIntrinsic;
volatile let v: u8 = 3;
func main(): i32 {
    let a: u8[4096] = [; unsafe readVolatile8(unsafe &v)];
    return i32(a[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_eq!(
        count(&emitted, "= (*(volatile u8 *)"),
        1,
        "fill-intrinsic: the volatile read happens once before the stores"
    );
    assert_contains("fill-intrinsic", &emitted, "a[4095] = __cl_fill_1;");
}

#[test]
fn should_keep_a_nested_address_taken_const_as_a_place() {
    let emitted = emit(
        "address-nested",
        &[(
            "main.clo",
            r#"
module bench.ImplAddressNested;
const N: i32 = 5;
func touch(p: raw i32): void { unsafe { *p = 6; } }
func main(): i32 {
    touch(unsafe &N);
    defer { let x: i32 = N; }
    return N - 5;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("address-nested", &emitted, "&(cl_bench_ImplAddressNested_N)");
    assert_absent("address-nested", &emitted, "&(5)");
    assert!(count(&emitted, "cl_bench_ImplAddressNested_N") >= 3);
}

#[test]
fn should_keep_a_shifting_fill_on_the_unrolled_path() {
    let emitted = emit(
        "fill-shift",
        &[(
            "main.clo",
            r#"
module bench.ImplFillShift;
func main(): i32 {
    let a: u8[32] = [; u8(1) << 8];
    return i32(a[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-shift", &emitted, "memset(&(a[0])");
    assert_absent("fill-shift", &emitted, "for (__cl_fill_");
    assert_contains("fill-shift", &emitted, "a[31] = ((u8)(1) << (8 & 7));");
}

#[test]
fn should_keep_a_volatile_target_on_the_unrolled_path() {
    let emitted = emit(
        "fill-volatile-target",
        &[(
            "main.clo",
            r#"
module bench.ImplVolatileTarget;
volatile let G: u8[16] = [; 0];
func main(): i32 {
    G = [; 1];
    return i32(G[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-volatile-target", &emitted, "memset(&(cl_bench_ImplVolatileTarget_G");
    assert_contains("fill-volatile-target", &emitted, "cl_bench_ImplVolatileTarget_G[0] = 1u;");
}

#[test]
fn should_keep_a_narrow_arithmetic_const_as_a_place() {
    let emitted = emit(
        "const-narrow",
        &[(
            "main.clo",
            r#"
module bench.ImplNarrow;
const G: i32 = i32(u8(200) + u8(100));
func main(): i32 { return G; }
"#,
        )],
        "main.clo",
    );
    assert_contains("const-narrow", &emitted, "return cl_bench_ImplNarrow_G;");
    assert_absent("const-narrow", &emitted, "return 44;");
}

#[test]
fn should_keep_an_enum_member_const_as_a_place() {
    let emitted = emit(
        "const-enum",
        &[(
            "main.clo",
            r#"
module bench.ImplEnum;
enum E: u8 { A = 255, B }
const X: i32 = i32(E.B);
func main(): i32 { return X; }
"#,
        )],
        "main.clo",
    );
    assert_contains("const-enum", &emitted, "return cl_bench_ImplEnum_X;");
}

#[test]
fn should_keep_the_lower_guard_for_a_huge_negative_slice_index() {
    let emitted = emit(
        "guard-huge",
        &[(
            "main.clo",
            r#"
module bench.ImplGuardHuge;
import { slice } from std.mem;
func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let s: u8[] = slice(a, 0, 4);
    return i32(s[-i32(2147483648)]);
}
"#,
        )],
        "main.clo",
    );
    assert_contains("guard-huge", &emitted, "< 0");
}

#[test]
fn should_keep_the_zero_trap_for_a_bitnot_divisor() {
    let emitted = emit(
        "div-bitnot",
        &[(
            "main.clo",
            r#"
module bench.ImplBitNotDiv;
func main(): i32 {
    let x: u32 = 7;
    let q: u32 = x / ~u32(4294967295);
    return i32(q);
}
"#,
        )],
        "main.clo",
    );
    assert_contains("div-bitnot", &emitted, "cl_trap(\"E11210\", \"integer division by zero\")");
}


#[test]
fn should_leave_a_while_materialized_fill_unrolled() {
    let emitted = emit(
        "fill-materialized-while",
        &[(
            "main.clo",
            r#"
module bench.ImplMaterializedWhile;
func eat(a: u8[4096]): i32 { return i32(a[0]); }
func main(): i32 {
    let ok: bool = true;
    while (ok && eat([; 1]) == 1) { return 0; }
    return 1;
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-materialized-while", &emitted, "memset(&(__cl_owned");
    assert_contains("fill-materialized-while", &emitted, "[4095] = 1u");
}

#[test]
fn should_leave_a_struct_literal_fill_unrolled() {
    let emitted = emit(
        "fill-struct",
        &[(
            "main.clo",
            r#"
module bench.ImplStructFill;
struct Pair { a: i32; b: i32; }
func main(): i32 {
    let p: Pair[16] = [; Pair { a: 2, b: 3 }];
    return p[0].a;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("fill-struct", &emitted, "p[15] = (ClPair){ 2, 3 };");
    assert_absent("fill-struct", &emitted, "for (__cl_fill");
}

#[test]
fn should_not_memset_a_volatile_target_on_the_reads_or_conditional_path() {
    let emitted = emit(
        "fill-volatile-paths",
        &[(
            "main.clo",
            r#"
module bench.ImplVolatilePaths;
volatile let G: u8[20] = [; 0];
func main(): i32 {
    let x: u8 = 7;
    G = [x; 1];
    let flag: bool = true;
    G = flag ? [x; 1] : [; 2];
    return i32(G[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-volatile-paths", &emitted, "memset(&(cl_bench_ImplVolatilePaths_G");
}

#[test]
fn should_not_memset_a_volatile_struct_field_fill() {
    let emitted = emit(
        "fill-volatile-field",
        &[(
            "main.clo",
            r#"
module bench.ImplVolatileField;
struct S { tag: i32; xs: u8[16]; }
volatile let H: S = S { tag: 0, xs: [; 0] };
func main(): i32 {
    H = S { xs: [; 1], tag: 2 };
    return i32(H.xs[0]);
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-volatile-field", &emitted, "memset(&(cl_bench_ImplVolatileField_H");
}

#[test]
fn should_not_compact_a_materialized_conditional_fill() {
    let emitted = emit(
        "fill-materialized-conditional",
        &[(
            "main.clo",
            r#"
module bench.ImplMaterializedConditional;
func eat(a: u8[4096]): i32 { return i32(a[0]); }
func main(): i32 {
    let c: bool = true;
    let ok: bool = true;
    while (ok && eat(c ? [; 1] : [; 2]) == 1) { return 0; }
    return 1;
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-materialized-conditional", &emitted, "memset(&(__cl_owned");
}

#[test]
fn should_keep_an_integer_const_from_an_f32_cast_as_a_place() {
    let emitted = emit(
        "const-f32-cast",
        &[(
            "main.clo",
            r#"
module bench.ImplF32Cast;
const A: i32 = i32(f32(16777217.0));
func main(): i32 { return A; }
"#,
        )],
        "main.clo",
    );
    assert_contains("const-f32-cast", &emitted, "return cl_bench_ImplF32Cast_A;");
}

#[test]
fn should_render_unsigned_unary_at_its_type() {
    let emitted = emit(
        "unsigned-unary",
        &[(
            "main.clo",
            r#"
module bench.ImplUnsignedUnary;
func main(): i32 {
    let m: i32 = -2147483648;
    let a: i32 = m / i32(-u8(1));
    let b: i32 = m / i32(~u8(0));
    return a + b;
}
"#,
        )],
        "main.clo",
    );
    assert_contains("unsigned-unary", &emitted, "(u8)(-(u8)((u8)(1)))");
    assert_contains("unsigned-unary", &emitted, "(u8)(~(u8)((u8)(0)))");
}

#[test]
fn should_trap_a_binary_zero_divisor_without_spelling_the_division() {
    let emitted = emit(
        "div-binary-zero",
        &[(
            "main.clo",
            r#"
module bench.ImplBinaryZero;
func main(): i32 {
    let x: u32 = 7;
    let q: u32 = x % (u32(0) + u32(0));
    return i32(q);
}
"#,
        )],
        "main.clo",
    );
    assert_contains("div-binary-zero", &emitted, "cl_trap(\"E11210\", \"integer division by zero\")");
    assert_absent("div-binary-zero", &emitted, "q = (x %");
}

#[test]
fn should_keep_a_non_byte_fill_unrolled_below_64_and_compact_at_64() {
    let small = emit(
        "fill-int-small",
        &[(
            "main.clo",
            r#"
module bench.ImplIntSmall;
func main(): i32 {
    let a: i32[63] = [; 7];
    return a[0];
}
"#,
        )],
        "main.clo",
    );
    assert_absent("fill-int-small", &small, "for (__cl_fill_");
    assert_eq!(count(&small, "] = 7;"), 63);
    let threshold = emit(
        "fill-int-threshold",
        &[(
            "main.clo",
            r#"
module bench.ImplIntThreshold;
func main(): i32 {
    let a: i32[64] = [; 7];
    return a[0];
}
"#,
        )],
        "main.clo",
    );
    assert_contains("fill-int-threshold", &threshold, "for (__cl_fill_");
}
