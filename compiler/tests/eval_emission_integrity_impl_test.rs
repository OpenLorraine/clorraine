//! Implementation tests for the evaluation and emission integrity batch.
//!
//! These cases pin the observable internals of the module-initializer fixes:
//! the destination-carrying emission of a fixed-array call in each accepted
//! position (whole value, element and member projection, user-function and
//! intrinsic arguments, conditional arm), the one shared operand for a
//! guarded read, the absence of init locals in unaffected initializers, and
//! the deterministic re-emission of the rewritten forms. They complement the
//! specification suite, which pins the accepted and rejected program
//! behavior.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-eeiimpl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Emits one host-profile program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            c_path.to_str().expect("emitted C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Emits the same program twice into one directory and returns both units.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_clc"))
            .args([
                "--profile",
                "host",
                "-o",
                c_path.to_str().expect("emitted C path"),
                source_path.to_str().expect("source path"),
            ])
            .output()
            .expect("failed to spawn the compiler binary");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{tag}: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        emitted.push(std::fs::read_to_string(&c_path).unwrap_or_default());
    }
    (emitted[0].clone(), emitted[1].clone())
}

/// Counts the occurrences of `needle` in `haystack`.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// Asserts the emitted unit contains every fragment.
fn assert_contains_all(tag: &str, emitted: &str, fragments: &[&str]) {
    for fragment in fragments {
        assert!(
            emitted.contains(fragment),
            "{tag}: expected the emitted C to contain {fragment:?}"
        );
    }
}

/// The whole-value position binds the call through its hidden destination and
/// fills the target element-wise from the bound local.
#[test]
fn should_emit_the_hidden_destination_for_the_whole_value_init() {
    let source = r#"module spec.eei.initwhole;

func mk(): i32[2] {
    return [4, 2];
}

let a: i32[2] = mk();

func main(): i32 {
    return a[1];
}
"#;
    let emitted = emit_host("pin-whole", source);
    assert_contains_all(
        "pin-whole",
        &emitted,
        &[
            "i32 __cl_itmp0[2] = {0};",
            "cl_spec_eei_initwhole_mk(__cl_itmp0);",
            "cl_spec_eei_initwhole_a[0] = __cl_itmp0[0u];",
            "cl_spec_eei_initwhole_a[1] = __cl_itmp0[1u];",
        ],
    );
    assert!(
        !emitted.contains("= cl_spec_eei_initwhole_mk();"),
        "pin-whole: the destination-less call spelling must be gone"
    );
}

/// The element-projection position binds the call and reads the local at the
/// constant index.
#[test]
fn should_emit_the_bound_local_for_the_element_projection() {
    let source = r#"module spec.eei.initelement;

func mk(): i32[2] {
    return [7, 2];
}

let b: i32 = mk()[1];

func main(): i32 {
    return b;
}
"#;
    let emitted = emit_host("pin-element", source);
    assert_contains_all(
        "pin-element",
        &emitted,
        &[
            "i32 __cl_itmp0[2] = {0};",
            "cl_spec_eei_initelement_mk(__cl_itmp0);",
            "cl_spec_eei_initelement_b = __cl_itmp0[1];",
        ],
    );
}

/// The member-projection position binds the struct-returning call once and
/// fills the target from the local's member.
#[test]
fn should_emit_the_bound_call_for_the_member_projection() {
    let source = r#"module spec.eei.initmember;

struct Box {
    arr: i32[2];
}

func mkBox(): Box {
    return Box { arr: [5, 2] };
}

let c: i32[2] = mkBox().arr;

func main(): i32 {
    return c[1];
}
"#;
    let emitted = emit_host("pin-member", source);
    assert_contains_all(
        "pin-member",
        &emitted,
        &[
            "ClBox __cl_itmp0 = cl_spec_eei_initmember_mkBox();",
            "cl_spec_eei_initmember_c[0] = __cl_itmp0.arr[0u];",
            "cl_spec_eei_initmember_c[1] = __cl_itmp0.arr[1u];",
        ],
    );
}

/// The argument positions bind each fixed-array call once, and the slice
/// typedef the new locals need is emitted.
#[test]
fn should_emit_bound_arguments_for_the_call_and_intrinsic_positions() {
    let source = r#"module spec.eei.initargs;

import { slice } from std.mem;

func mk(): i32[2] {
    return [3, 9];
}

func eat(arr: i32[2]): i32 {
    return arr[0] + arr[1];
}

let d: i32 = eat(mk());
let e: i32 = i32(len(slice(mk(), 0, 2)));

func main(): i32 {
    return d + e;
}
"#;
    let emitted = emit_host("pin-args", source);
    assert_contains_all(
        "pin-args",
        &emitted,
        &[
            "cl_spec_eei_initargs_eat(__cl_itmp0)",
            "ClSlice_i32",
            "cl_spec_eei_initargs_d = cl_spec_eei_initargs_eat(__cl_itmp0);",
        ],
    );
    assert_eq!(
        count_occurrences(&emitted, "cl_spec_eei_initargs_mk(__cl_itmp"),
        2,
        "pin-args: both call results must bind through their own destination"
    );
}

/// The conditional-arm position stores the condition once and binds the arm
/// call under it, so the guards and the read share one evaluation.
#[test]
fn should_emit_the_guarded_binding_for_the_conditional_arm() {
    let source = r#"module spec.eei.initcond;

func mk(): i32[2] {
    return [6, 1];
}

func pick(): bool {
    return true;
}

let y: i32[2] = [8, 8];
let g: i32[2] = pick() ? mk() : y;

func main(): i32 {
    return g[0];
}
"#;
    let emitted = emit_host("pin-cond", source);
    assert_contains_all(
        "pin-cond",
        &emitted,
        &[
            "bool __cl_itmp0 = cl_spec_eei_initcond_pick();",
            "if (__cl_itmp0) {",
            "cl_spec_eei_initcond_mk(__cl_itmp1);",
            "cl_spec_eei_initcond_g[0] = (__cl_itmp0 ? __cl_itmp1[0u] : cl_spec_eei_initcond_y[0u]);",
        ],
    );
    assert_eq!(
        count_occurrences(&emitted, "cl_spec_eei_initcond_pick()"),
        1,
        "pin-cond: the condition must be evaluated exactly once"
    );
}

/// A guarded read stores its impure operand once; the bounds guard and the
/// read share the stored value.
#[test]
fn should_share_one_operand_between_the_guard_and_the_store() {
    let source = r#"module spec.eei.guardonce;

let calls: i32 = 0;

func idx(): i32 {
    calls = calls + 1;
    if (calls == 1) {
        return 0;
    }
    return 5;
}

let s: u8[2] = [1, 2];
let v: u8 = s[idx()];

func main(): i32 {
    return calls;
}
"#;
    let emitted = emit_host("pin-guard", source);
    assert_eq!(
        count_occurrences(&emitted, "cl_spec_eei_guardonce_idx()"),
        1,
        "pin-guard: the impure index must be evaluated exactly once"
    );
    assert_contains_all(
        "pin-guard",
        &emitted,
        &[
            "i32 __cl_itmp0 = cl_spec_eei_guardonce_idx();",
            "cl_spec_eei_guardonce_v = cl_spec_eei_guardonce_s[__cl_itmp0];",
        ],
    );
}

/// An initializer with no fixed-array call and no guarded operand keeps its
/// emission: no init local appears.
#[test]
fn should_emit_no_init_local_for_an_unaffected_initializer() {
    let source = r#"module spec.eei.unaffected;

func pick(): bool {
    return true;
}

let a: i32[2] = [7, 8];
let b: i32 = a[1];
let c: bool = pick();
let d: bool = c ? true : false;

func main(): i32 {
    return b;
}
"#;
    let emitted = emit_host("pin-unaffected", source);
    assert_eq!(
        count_occurrences(&emitted, "__cl_itmp"),
        0,
        "pin-unaffected: an unchanged initializer must not grow an init local"
    );
}

/// The rewritten forms re-emit byte-identically.
#[test]
fn should_re_emit_the_rewritten_forms_byte_identically() {
    let source = r#"module spec.eei.initcond;

func mk(): i32[2] {
    return [6, 1];
}

func pick(): bool {
    return true;
}

let y: i32[2] = [8, 8];
let g: i32[2] = pick() ? mk() : y;

func main(): i32 {
    return g[0];
}
"#;
    let (first, second) = emit_twice("pin-ident", source);
    assert_eq!(first, second, "pin-ident: the emission must be deterministic");
}
