//! Emission pins for function-local constant scalar array initializers.
//!
//! A complete explicit list of guard-free constants in a local fixed array
//! may copy from one read-only template instead of one store per element. The
//! positive pins fail on the base compiler (which emits one store per
//! element). The exclusion guards pass on both and fail if the implementation
//! wrongly emits a template for an ineligible shape.

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
            "clc-constant-array-residual-impl-{}-{tag}-{sequence}",
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

fn emit(tag: &str, source: &str) -> String {
    emit_profile(tag, "host", source)
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

fn assert_no_template(tag: &str, emitted: &str) {
    assert_absent(tag, emitted, "__cl_ctmpl_");
}

fn seq(start: i32, count: usize) -> String {
    (0..count)
        .map(|offset| (start + offset as i32).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn local_main(module: &str, ty: &str, count: usize, list: &str) -> String {
    format!(
        r#"
module spec.{module};
func main(): i32 {{
    let t: {ty}[{count}] = [{list}];
    return i32(t[0]) - 1;
}}
"#
    )
}

#[test]
fn should_emit_a_template_for_a_function_local_i32_table() {
    let source = local_main("ConstArrayLocalImplI32", "i32", 32, &seq(1, 32));
    let emitted = emit("impl-local-i32", &source);
    assert_contains("impl-local-i32", &emitted, "static const i32 __cl_ctmpl_");
    assert_contains("impl-local-i32", &emitted, "memcpy(&(t), __cl_ctmpl_");
    assert_contains("impl-local-i32", &emitted, "sizeof(t)");
}

#[test]
fn should_emit_a_template_for_a_function_local_u16_table() {
    let source = local_main("ConstArrayLocalImplU16", "u16", 32, &seq(1, 32));
    let emitted = emit("impl-local-u16", &source);
    assert_contains("impl-local-u16", &emitted, "static const u16 __cl_ctmpl_");
    assert_contains("impl-local-u16", &emitted, "memcpy(&(t), __cl_ctmpl_");
}

#[test]
fn should_emit_a_template_for_a_function_local_usize_table() {
    let source = local_main("ConstArrayLocalImplUsize", "usize", 32, &seq(1, 32));
    let emitted = emit("impl-local-usize", &source);
    assert_contains("impl-local-usize", &emitted, "static const usize __cl_ctmpl_");
    assert_contains("impl-local-usize", &emitted, "memcpy(&(t), __cl_ctmpl_");
}

#[test]
fn should_emit_the_template_for_each_hosted_profile() {
    let source = local_main("ConstArrayLocalImplProfiles", "i32", 32, &seq(1, 32));
    for profile in ["host", "hosted-amiga", "hosted-amiga-13"] {
        let tag = format!("impl-local-profile-{profile}");
        let emitted = emit_profile(&tag, profile, &source);
        assert_contains(&tag, &emitted, "static const i32 __cl_ctmpl_");
        assert_contains(&tag, &emitted, "memcpy(&(t), __cl_ctmpl_");
    }
}

#[test]
fn should_keep_the_stores_on_freestanding() {
    let source = local_main("ConstArrayLocalImplFree", "i32", 32, &seq(1, 32));
    let emitted = emit_profile("impl-local-free", "freestanding", &source);
    assert_no_template("impl-local-free", &emitted);
}

#[test]
fn should_keep_the_stores_for_a_short_local_list() {
    let source = local_main("ConstArrayLocalImplShort", "i32", 32, &seq(1, 16));
    let emitted = emit("impl-local-short", &source);
    assert_no_template("impl-local-short", &emitted);
    assert_contains("impl-local-short", &emitted, "t[0] = 1;");
}

#[test]
fn should_keep_the_stores_for_a_function_local_u8_table() {
    let list = (1..=64)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let source = local_main("ConstArrayLocalImplU8", "u8", 64, &list);
    let emitted = emit("impl-local-u8", &source);
    assert_no_template("impl-local-u8", &emitted);
}

#[test]
fn should_keep_the_stores_for_a_local_fill() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplFill;
func main(): i32 {{
    let t: i32[32] = [{prefix}; 7];
    return i32(t[0]) - 1;
}}
"#,
        prefix = seq(1, 16)
    );
    let emitted = emit("impl-local-fill", &source);
    assert_no_template("impl-local-fill", &emitted);
}

#[test]
fn should_keep_the_stores_for_a_local_guard() {
    let source = r#"
module spec.ConstArrayLocalImplGuard;
func main(): i32 {
    let d: i32 = 1;
    let t: i32[32] = [1 / d, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32];
    return i32(t[0]) - 1;
}
"#;
    let emitted = emit("impl-local-guard", source);
    assert_no_template("impl-local-guard", &emitted);
}

#[test]
fn should_keep_the_module_level_template() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplGlobal;
let t: i32[32] = [{list}];
func main(): i32 {{ return t[0] - 1; }}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-global", &source);
    assert_contains("impl-local-global", &emitted, "static const i32 __cl_ctmpl_");
    assert_contains(
        "impl-local-global",
        &emitted,
        "memcpy(cl_spec_ConstArrayLocalImplGlobal_t, __cl_ctmpl_",
    );
}

#[test]
fn should_not_shadow_a_user_ffi_template_name_from_a_local_table() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplCollision;
ffi probe {{
    func __cl_ctmpl_1(): i32;
}}
func main(): i32 {{
    let t: i32[32] = [{list}];
    return t[0] - 1 + __cl_ctmpl_1();
}}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-collision", &source);
    assert_contains("impl-local-collision", &emitted, "__cl_ctmpl_1()");
    assert_absent(
        "impl-local-collision",
        &emitted,
        "static const i32 __cl_ctmpl_1[32] = {",
    );
    assert_contains("impl-local-collision", &emitted, "static const i32 __cl_ctmpl_");
}

#[test]
fn should_not_shadow_a_user_local_template_name() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplShadow;
func main(): i32 {{
    let __cl_ctmpl_1: i32 = 5;
    let t: i32[32] = [{list}];
    return t[0] + __cl_ctmpl_1;
}}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-shadow", &source);
    assert_contains("impl-local-shadow", &emitted, "i32 __cl_ctmpl_1 = 0;");
    assert_absent(
        "impl-local-shadow",
        &emitted,
        "static const i32 __cl_ctmpl_1[32] = {",
    );
    assert_contains("impl-local-shadow", &emitted, "static const i32 __cl_ctmpl_");
}

#[test]
fn should_emit_a_template_for_a_struct_field_local_assignment() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplField;
struct Holder {{ values: i32[32]; }}
func main(): i32 {{
    let h: Holder = Holder {{ values: [; 0] }};
    h.values = [{list}];
    return h.values[0] - 1;
}}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-field", &source);
    assert_contains("impl-local-field", &emitted, "static const i32 __cl_ctmpl_");
    assert_contains("impl-local-field", &emitted, "memcpy(&(h.values), __cl_ctmpl_");
}

#[test]
fn should_keep_the_stores_for_a_runtime_indexed_member_target() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplMember;
struct Row {{ values: i32[32]; tag: i32; }}
struct Grid {{ rows: Row[2]; }}
func main(): i32 {{
    let g: Grid = Grid {{ rows: [Row {{ values: [; 0], tag: 10 }}, Row {{ values: [; 0], tag: 20 }}] }};
    let i: i32 = 1;
    g.rows[i].values = [{list}];
    return g.rows[1].values[0] - 1;
}}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-member", &source);
    assert_no_template("impl-local-member", &emitted);
}

#[test]
fn should_emit_a_template_for_a_constant_indexed_member_target() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplConstMember;
struct Row {{ values: i32[32]; tag: i32; }}
struct Grid {{ rows: Row[2]; }}
func main(): i32 {{
    let g: Grid = Grid {{ rows: [Row {{ values: [; 0], tag: 10 }}, Row {{ values: [; 0], tag: 20 }}] }};
    g.rows[0].values = [{list}];
    return g.rows[0].values[0] - 1;
}}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-const-member", &source);
    assert_contains("impl-local-const-member", &emitted, "static const i32 __cl_ctmpl_");
    assert_contains(
        "impl-local-const-member",
        &emitted,
        "memcpy(&(g.rows[0].values), __cl_ctmpl_",
    );
}

#[test]
fn should_emit_a_template_for_an_adjusted_array_parameter() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplParam;
func fill(borrow a: i32[32]): void {{
    a = [{list}];
}}
func main(): i32 {{ return 0; }}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-param", &source);
    assert_contains("impl-local-param", &emitted, "static const i32 __cl_ctmpl_");
    assert_contains("impl-local-param", &emitted, "memcpy(a, __cl_ctmpl_");
}

#[test]
fn should_keep_the_stores_for_a_volatile_global_assignment() {
    let source = format!(
        r#"
module spec.ConstArrayLocalImplVolatile;
volatile let g: i32[32] = [{list}];
func main(): i32 {{
    g = [{list}];
    return 0;
}}
"#,
        list = seq(1, 32)
    );
    let emitted = emit("impl-local-volatile", &source);
    assert_no_template("impl-local-volatile", &emitted);
}

#[test]
fn should_place_the_template_at_the_local_declaration() {
    let source = local_main("ConstArrayLocalImplPlace", "i32", 32, &seq(1, 32));
    let emitted = emit("impl-local-place", &source);
    let main = emitted
        .find("cl_spec_ConstArrayLocalImplPlace_main(void)")
        .expect("the main function");
    let template = emitted
        .find("static const i32 __cl_ctmpl_")
        .expect("the template");
    let copy = emitted
        .find("memcpy(&(t), __cl_ctmpl_")
        .expect("the local memcpy");
    assert!(main < template, "the template must be inside main");
    assert!(template < copy, "the template must precede its copy");
}
