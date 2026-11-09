//! Emission pins for module-level constant scalar array initializers.
//!
//! The positive pins fail on the base compiler (which emits one store per
//! element). The exclusion guards pass on both and fail if the implementation
//! wrongly emits a template for an ineligible shape. Every positive and
//! exclusion table is at or above the 16-element/64-byte floor so the
//! threshold cannot explain a guard.

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
            "clc-constant-array-impl-{}-{tag}-{sequence}",
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

fn emit(tag: &str, source: &str) -> String {
    emit_profile(tag, "host", source)
}

fn emit_profile(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
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

fn seq(start: i32, count: usize) -> String {
    (0..count)
        .map(|offset| (start + offset as i32).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn rep(value: &str, count: usize) -> String {
    std::iter::repeat(value)
        .take(count)
        .collect::<Vec<_>>()
        .join(", ")
}

fn header(module: &str, table: &str) -> String {
    format!(
        r#"
module {module};
"#,
        module = module
    ) + table
        + r#"
func main(): i32 {
    return 0;
}
"#
}

fn assert_template(tag: &str, emitted: &str) {
    assert_contains(tag, emitted, "static const ");
    assert_contains(tag, emitted, "__cl_ctmpl_0[");
    assert_contains(tag, emitted, "memcpy(");
    assert_contains(tag, emitted, "__cl_ctmpl_0");
}

fn assert_no_template(tag: &str, emitted: &str) {
    assert_absent(tag, emitted, "__cl_ctmpl_");
}

fn assert_old_store(tag: &str, emitted: &str, target: &str) {
    assert_contains(tag, emitted, &format!("{target}[0] = "));
}

// ===== Positive pins =====

#[test]
fn should_emit_one_template_and_memcpy_for_i32() {
    let source = header(
        "spec.ConstArrayImplI32",
        &format!("let t: i32[32] = [{}];\n", seq(1, 32)),
    );
    let emitted = emit("impl-i32", &source);
    assert_template("impl-i32", &emitted);
    assert_contains("impl-i32", &emitted, "static const i32 __cl_ctmpl_0[32] = {");
    assert_contains(
        "impl-i32",
        &emitted,
        "memcpy(cl_spec_ConstArrayImplI32_t, __cl_ctmpl_0, sizeof(cl_spec_ConstArrayImplI32_t));",
    );
    assert_absent("impl-i32", &emitted, "cl_spec_ConstArrayImplI32_t[0] = ");
}

#[test]
fn should_emit_negative_and_typed_values() {
    let source = header(
        "spec.ConstArrayImplTyped",
        &format!(
            "let t: i32[32] = [-1, -2, i32(3), {}];\n",
            rep("0", 29)
        ),
    );
    let emitted = emit("impl-typed", &source);
    assert_template("impl-typed", &emitted);
    assert_contains("impl-typed", &emitted, "(i32)(-(u32)1)");
    assert_contains("impl-typed", &emitted, "(i32)(-(u32)2)");
    assert_contains("impl-typed", &emitted, "(i32)(3)");
    assert_absent("impl-typed", &emitted, "cl_spec_ConstArrayImplTyped_t[0] = ");
}

#[test]
fn should_keep_u8_and_bool_stores() {
    let source = format!(
        r#"
module spec.ConstArrayImplSmall;
let a: u8[64] = [{u8s}];
let b: bool[64] = [{bools}];
func main(): i32 {{ return i32(a[0]) + i32(b[0]); }}
"#,
        u8s = rep("1", 64),
        bools = rep("true", 64)
    );
    let emitted = emit("impl-small", &source);
    assert_no_template("impl-small", &emitted);
    assert_old_store("impl-small", &emitted, "cl_spec_ConstArrayImplSmall_a");
    assert_old_store("impl-small", &emitted, "cl_spec_ConstArrayImplSmall_b");
}

#[test]
fn should_emit_u16_and_i16_templates() {
    let source = format!(
        r#"
module spec.ConstArrayImplWide;
let a: u16[32] = [{list}];
let b: i16[32] = [{list}];
func main(): i32 {{ return i32(a[0]) + i32(b[0]); }}
"#,
        list = rep("1", 32)
    );
    let emitted = emit("impl-wide", &source);
    assert_contains("impl-wide", &emitted, "static const u16 __cl_ctmpl_0[32] = {");
    assert_contains("impl-wide", &emitted, "static const i16 __cl_ctmpl_1[32] = {");
    assert_contains("impl-wide", &emitted, "memcpy(cl_spec_ConstArrayImplWide_a, __cl_ctmpl_0");
    assert_contains("impl-wide", &emitted, "memcpy(cl_spec_ConstArrayImplWide_b, __cl_ctmpl_1");
}

#[test]
fn should_emit_usize_and_isize_templates() {
    let source = format!(
        r#"
module spec.ConstArrayImplSized;
let a: usize[32] = [{list}];
let b: isize[32] = [{list}];
func main(): i32 {{ return i32(a[0]) + i32(b[0]); }}
"#,
        list = rep("1", 32)
    );
    let emitted = emit("impl-sized", &source);
    assert_contains("impl-sized", &emitted, "static const usize __cl_ctmpl_0[32] = {");
    assert_contains("impl-sized", &emitted, "static const isize __cl_ctmpl_1[32] = {");
    assert_contains("impl-sized", &emitted, "memcpy(cl_spec_ConstArrayImplSized_a, __cl_ctmpl_0");
    assert_contains("impl-sized", &emitted, "memcpy(cl_spec_ConstArrayImplSized_b, __cl_ctmpl_1");
}

#[test]
fn should_use_distinct_templates_and_keep_the_target_declaration() {
    let source = format!(
        r#"
module spec.ConstArrayImplTwo;
let a: i32[32] = [{list}];
let b: i32[32] = [{list}];
func main(): i32 {{ return a[0] + b[0]; }}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-two", &source);
    assert_contains("impl-two", &emitted, "static i32 cl_spec_ConstArrayImplTwo_a[32] = {0};");
    assert_contains("impl-two", &emitted, "static i32 cl_spec_ConstArrayImplTwo_b[32] = {0};");
    assert_contains("impl-two", &emitted, "__cl_ctmpl_0[32]");
    assert_contains("impl-two", &emitted, "__cl_ctmpl_1[32]");
    assert_contains("impl-two", &emitted, "memcpy(cl_spec_ConstArrayImplTwo_a, __cl_ctmpl_0");
    assert_contains("impl-two", &emitted, "memcpy(cl_spec_ConstArrayImplTwo_b, __cl_ctmpl_1");
}

#[test]
fn should_keep_the_memcpy_at_the_initializer_position() {
    let source = format!(
        r#"
module spec.ConstArrayImplPosition;
let t: i32[32] = [{list}];
let after: i32 = 5;
func main(): i32 {{ return t[0] + after; }}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-position", &source);
    let init = emitted
        .split("static void cl_init_spec_ConstArrayImplPosition(void)")
        .nth(1)
        .expect("the init function");
    let memcpy = init.find("memcpy(cl_spec_ConstArrayImplPosition_t").expect("the memcpy");
    let after = init.find("cl_spec_ConstArrayImplPosition_after = 5").expect("the later init");
    assert!(memcpy < after, "the template copy must precede the later global");
}

#[test]
fn should_emit_the_template_for_each_hosted_profile() {
    let source = header(
        "spec.ConstArrayImplProfiles",
        &format!("let t: i32[32] = [{}];\n", seq(1, 32)),
    );
    let host = emit_profile("impl-host", "host", &source);
    let amiga = emit_profile("impl-amiga", "hosted-amiga", &source);
    let amiga13 = emit_profile("impl-amiga13", "hosted-amiga-13", &source);
    assert_template("impl-host", &host);
    assert_template("impl-amiga", &amiga);
    assert_template("impl-amiga13", &amiga13);
}

#[test]
fn should_wrap_the_template_at_eight_values_per_line() {
    let source = format!(
        r#"
module spec.ConstArrayImplWrap;
let t: i32[64] = [{list}];
func main(): i32 {{ return t[0]; }}
"#,
        list = seq(1, 64)
    );
    let emitted = emit("impl-wrap", &source);
    let start = emitted.find("static const i32 __cl_ctmpl_0[64] = {").expect("the template");
    let end = emitted[start..].find("};").expect("the template close") + start;
    let block = &emitted[start..end];
    let rows: Vec<&str> = block.lines().filter(|line| line.trim().starts_with(|c: char| c.is_ascii_digit() || c == '-')).collect();
    assert_eq!(rows.len(), 8, "a 64-entry template must use eight rows");
    for row in rows {
        let values = row.trim().trim_end_matches(',').split(',').count();
        assert_eq!(values, 8, "each template row must hold eight values: {row}");
    }
}

// ===== Exclusion guards =====

#[test]
fn should_keep_a_short_list() {
    let source = header("spec.ConstArrayImplShort", "let t: i32[32] = [1, 2];\n");
    let emitted = emit("impl-short", &source);
    assert_no_template("impl-short", &emitted);
    assert_old_store("impl-short", &emitted, "cl_spec_ConstArrayImplShort_t");
}

#[test]
fn should_keep_an_empty_list() {
    let source = header("spec.ConstArrayImplEmpty", "let t: i32[32] = [];\n");
    let emitted = emit("impl-empty", &source);
    assert_no_template("impl-empty", &emitted);
    assert_absent("impl-empty", &emitted, "cl_spec_ConstArrayImplEmpty_t[0] = ");
}

#[test]
fn should_keep_a_fill() {
    let source = header("spec.ConstArrayImplFill", "let t: i32[32] = [; 7];\n");
    let emitted = emit("impl-fill", &source);
    assert_no_template("impl-fill", &emitted);
}

#[test]
fn should_keep_a_multi_extent_table() {
    let source = header(
        "spec.ConstArrayImplMulti",
        "let t: i32[4][4] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]];\n",
    );
    let emitted = emit("impl-multi", &source);
    assert_no_template("impl-multi", &emitted);
}

#[test]
fn should_keep_a_volatile_table() {
    let source = header(
        "spec.ConstArrayImplVolatile",
        &format!("volatile let t: i32[32] = [{}];\n", seq(1, 32)),
    );
    let emitted = emit("impl-volatile", &source);
    assert_no_template("impl-volatile", &emitted);
}

#[test]
fn should_keep_a_place_copy() {
    let source = header(
        "spec.ConstArrayImplPlace",
        "let src: i32[32] = [; 7];\nlet t: i32[32] = src;\n",
    );
    let emitted = emit("impl-place", &source);
    assert_no_template("impl-place", &emitted);
}

#[test]
fn should_keep_a_call_element() {
    let source = header(
        "spec.ConstArrayImplCall",
        &format!(
            "func f(): i32 {{ return 7; }}\nlet t: i32[32] = [f(), {}];\n",
            rep("0", 31)
        ),
    );
    let emitted = emit("impl-call", &source);
    assert_no_template("impl-call", &emitted);
}

#[test]
fn should_keep_a_conditional_element() {
    let source = header(
        "spec.ConstArrayImplConditional",
        &format!(
            "let c: bool = true;\nlet t: i32[32] = [c ? 1 : 2, {}];\n",
            rep("0", 31)
        ),
    );
    let emitted = emit("impl-conditional", &source);
    assert_no_template("impl-conditional", &emitted);
}

#[test]
fn should_keep_a_division_guard() {
    let source = header(
        "spec.ConstArrayImplGuard",
        &format!(
            "let min: i32 = i32(u32(0x80000000));\nlet neg: i32 = -1;\nlet t: i32[32] = [min / neg, {}];\n",
            rep("0", 31)
        ),
    );
    let emitted = emit("impl-guard", &source);
    assert_no_template("impl-guard", &emitted);
    assert_contains("impl-guard", &emitted, "E11210");
}

#[test]
fn should_keep_a_float_cast() {
    let source = header(
        "spec.ConstArrayImplFloat",
        &format!("let t: i32[32] = [i32(f32(1.5)), {}];\n", rep("0", 31)),
    );
    let emitted = emit("impl-float", &source);
    assert_no_template("impl-float", &emitted);
}

#[test]
fn should_keep_a_below_threshold_table() {
    let source = header(
        "spec.ConstArrayImplSmallTable",
        &format!("let t: i32[8] = [{}];\n", seq(1, 8)),
    );
    let emitted = emit("impl-below", &source);
    assert_no_template("impl-below", &emitted);
}

#[test]
fn should_keep_a_view_table() {
    let source = header(
        "spec.ConstArrayImplView",
        &format!("let t: str[16] = [{}];\n", rep("\"aa\"", 16)),
    );
    let emitted = emit("impl-view", &source);
    assert_no_template("impl-view", &emitted);
}

#[test]
fn should_keep_a_struct_table() {
    let source = header(
        "spec.ConstArrayImplStruct",
        &format!(
            "struct S {{ v: i32; }}\nlet t: S[32] = [{}];\n",
            rep("S { v: 1 }", 32)
        ),
    );
    let emitted = emit("impl-struct", &source);
    assert_no_template("impl-struct", &emitted);
}

#[test]
fn should_keep_a_float_table() {
    let source = header(
        "spec.ConstArrayImplFloatTable",
        &format!("let t: f32[32] = [{}];\n", rep("f32(1.5)", 32)),
    );
    let emitted = emit("impl-float-table", &source);
    assert_no_template("impl-float-table", &emitted);
}

#[test]
fn should_keep_the_stores_on_freestanding() {
    let source = header(
        "spec.ConstArrayImplFree",
        &format!("let t: i32[32] = [{}];\n", seq(1, 32)),
    );
    let emitted = emit_profile("impl-free", "freestanding", &source);
    assert_no_template("impl-free", &emitted);
}

#[test]
fn should_not_shadow_a_user_ffi_template_name() {
    let source = format!(
        r#"
module spec.ConstArrayImplCollision;
ffi probe {{
    func __cl_ctmpl_0(): i32;
}}
let t: i32[32] = [{list}];
let y: i32 = __cl_ctmpl_0();
func main(): i32 {{ return t[0] + y; }}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-collision", &source);
    assert_contains("impl-collision", &emitted, "static const i32 __cl_ctmpl_1[32] = {");
    assert_contains("impl-collision", &emitted, "__cl_ctmpl_0()");
    assert_absent("impl-collision", &emitted, "static const i32 __cl_ctmpl_0[32] = {");
}

#[test]
fn should_keep_a_full_prefix_fill() {
    let source = header(
        "spec.ConstArrayImplPrefixFill",
        &format!("let t: i32[32] = [{}; 7];\n", seq(1, 32)),
    );
    let emitted = emit("impl-prefix-fill", &source);
    assert_no_template("impl-prefix-fill", &emitted);
}

#[test]
fn should_keep_a_constant_binary_element() {
    let source = header(
        "spec.ConstArrayImplBinary",
        &format!("let t: i32[32] = [1 / 2, {}];\n", rep("0", 31)),
    );
    let emitted = emit("impl-binary", &source);
    assert_no_template("impl-binary", &emitted);
}

#[test]
fn should_keep_a_constant_conditional_element() {
    let source = header(
        "spec.ConstArrayImplConstantConditional",
        &format!("let t: i32[32] = [true ? 1 : 2, {}];\n", rep("0", 31)),
    );
    let emitted = emit("impl-constant-conditional", &source);
    assert_no_template("impl-constant-conditional", &emitted);
}

#[test]
fn should_keep_a_short_byte_table() {
    let source = header(
        "spec.ConstArrayImplShortBytes",
        &format!("let t: u16[16] = [{}];\n", rep("1", 16)),
    );
    let emitted = emit("impl-short-bytes", &source);
    assert_no_template("impl-short-bytes", &emitted);
}

#[test]
fn should_emit_the_exact_typed_extremes() {
    let source = header(
        "spec.ConstArrayImplExtremes",
        &format!(
            "let t: i32[32] = [2147483647, -2147483647, {}];\n",
            rep("0", 30)
        ),
    );
    let emitted = emit("impl-extremes", &source);
    assert_template("impl-extremes", &emitted);
    assert_contains("impl-extremes", &emitted, "2147483647");
    assert_contains("impl-extremes", &emitted, "(i32)(-(u32)2147483647)");
}

#[test]
fn should_keep_the_init_local_and_the_template_distinct() {
    let source = format!(
        r#"
module spec.ConstArrayImplInitLocal;
func mk(): i32[32] {{ return [; 7]; }}
let call: i32[32] = mk();
let t: i32[32] = [{list}];
func main(): i32 {{ return call[0] + t[0]; }}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-init-local", &source);
    assert_contains("impl-init-local", &emitted, "__cl_itmp0");
    assert_contains("impl-init-local", &emitted, "static const i32 __cl_ctmpl_0[32] = {");
    assert_contains("impl-init-local", &emitted, "__cl_ctmpl_0, sizeof(cl_spec_ConstArrayImplInitLocal_t)");
}

#[test]
fn should_emit_the_exact_typed_cast_values() {
    let source = format!(
        r#"
module spec.ConstArrayImplTypedExact;
let a: i32[32] = [i32(2147483647), i32(-2147483647), {zeros}];
let b: u16[32] = [u16(255), {zeros}];
let c: u32[16] = [u32(4294967295), {zeros15}];
let d: usize[32] = [usize(7), {zeros}];
let e: isize[32] = [isize(-7), {zeros}];
func main(): i32 {{ return 0; }}
"#,
        zeros = rep("0", 30),
        zeros15 = rep("0", 15)
    );
    let emitted = emit("impl-typed-exact", &source);
    assert_contains("impl-typed-exact", &emitted, "(i32)(2147483647)");
    assert_contains("impl-typed-exact", &emitted, "(i32)((i32)(-(u32)2147483647))");
    assert_contains("impl-typed-exact", &emitted, "(u16)(255)");
    assert_contains("impl-typed-exact", &emitted, "(u32)(4294967295u)");
    assert_contains("impl-typed-exact", &emitted, "(usize)(7)");
    assert_contains("impl-typed-exact", &emitted, "(isize)((i32)(-(u32)7))");
}
