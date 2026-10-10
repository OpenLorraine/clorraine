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

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> std::process::Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new(&program)
        .output()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"))
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

/// A short-circuit operand's bind is guarded by the left operand, and it runs
/// only on the decided path.
#[test]
fn should_guard_a_short_circuit_bind_by_the_left_operand() {
    let skipped = r#"module spec.eei.shortcircuit;

let calls: i32 = 0;

func f(): bool {
    return false;
}

func mk(): i32[2] {
    calls = calls + 1;
    return [1, 0];
}

let x: bool = f() && (mk()[0] == 1);

func main(): i32 {
    return calls;
}
"#;
    let emitted = emit_host("pin-shortcircuit", skipped);
    assert_contains_all(
        "pin-shortcircuit",
        &emitted,
        &[
            "bool __cl_itmp0 = 0;",
            "if (cl_spec_eei_shortcircuit_f()) {",
            "cl_spec_eei_shortcircuit_mk(__cl_itmp1);",
            "__cl_itmp0 = (__cl_itmp1[0] == 1);",
            "cl_spec_eei_shortcircuit_x = __cl_itmp0;",
        ],
    );
    let output = run_host("pin-shortcircuit-run", skipped);
    assert_eq!(
        output.status.code(),
        Some(0),
        "pin-shortcircuit-run: a skipped operand must not run its bind"
    );

    let taken = skipped.replace("return false;", "return true;");
    let output = run_host("pin-shortcircuit-run-taken", &taken);
    assert_eq!(
        output.status.code(),
        Some(1),
        "pin-shortcircuit-run-taken: the decided operand runs its bind once"
    );
}

/// Emits, builds, and runs a program, asserting its exit code.
fn assert_run_exit(tag: &str, source: &str, expected: i32) {
    let output = run_host(tag, source);
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit code {expected}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A short-circuit result keeps the left-decided value when the left decides.
#[test]
fn should_keep_the_short_circuit_result_when_the_left_decides() {
    let source = r#"module spec.eei.orresult;

func f(): bool {
    return true;
}

func mk(): i32[2] {
    return [1, 0];
}

let x: bool = f() || (mk()[0] == 1);

func main(): i32 {
    if (x) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit_host("pin-or-result", source);
    assert!(
        emitted.contains("bool __cl_itmp0 = 1;"),
        "pin-or-result: a true left operand needs the left-decided `true` result"
    );
    assert_run_exit("pin-or-result-run", source, 1);
}

/// A lazily evaluated position never runs its operand on the untaken path.
#[test]
fn should_keep_lazy_positions_lazy() {
    let arm = r#"module spec.eei.lazyarm;

let calls: i32 = 0;

func t(): bool {
    return false;
}

func idx(): i32 {
    calls = calls + 1;
    return 0;
}

let s: u8[2] = [1, 2];
let x: i32 = t() ? i32(s[idx()]) : 0;

func main(): i32 {
    return calls;
}
"#;
    assert_run_exit("pin-lazy-arm-skip", arm, 0);
    let arm_taken = arm.replace("return false;", "return true;");
    assert_run_exit("pin-lazy-arm-taken", &arm_taken, 1);

    let right = r#"module spec.eei.lazyright;

let calls: i32 = 0;

func t(): bool {
    return false;
}

func idx(): i32 {
    calls = calls + 1;
    return 0;
}

let s: u8[2] = [1, 2];
let x: bool = t() && (i32(s[idx()]) > 0);

func main(): i32 {
    return calls;
}
"#;
    assert_run_exit("pin-lazy-right-skip", right, 0);
    let right_taken = right.replace("return false;", "return true;");
    assert_run_exit("pin-lazy-right-taken", &right_taken, 1);
}

/// A binding on the right side of an operator evaluates after an earlier
/// operand's stored value, in source order.
#[test]
fn should_evaluate_initializer_operands_left_to_right() {
    let source = r#"module spec.eei.order;

let log: i32 = 0;

func idx(): i32 {
    log = log * 10 + 1;
    return 1;
}

func mk(): i32[2] {
    log = log * 10 + 2;
    return [3, 0];
}

let s: u8[2] = [1, 2];
let x: i32 = i32(s[idx()]) + mk()[0];

func main(): i32 {
    return log;
}
"#;
    assert_run_exit("pin-order", source, 12);
}

/// A short-circuit operator inside a conditional arm keeps the enclosing
/// choice: the arm's left operand does not run on the skipped path.
#[test]
fn should_guard_a_short_circuit_bind_by_the_enclosing_choice() {
    let source = r#"module spec.eei.armguard;

let fcalls: i32 = 0;

func pick(): bool {
    return false;
}

func f(): bool {
    fcalls = fcalls + 1;
    return true;
}

func mk(): i32[2] {
    return [1, 0];
}

let x: bool = pick() ? (f() && (mk()[0] == 1)) : false;

func main(): i32 {
    return fcalls;
}
"#;
    assert_run_exit("pin-arm-guard-skip", source, 0);
}

/// The left operand of a short-circuit operator evaluates once, and its
/// result decides the right side.
#[test]
fn should_single_evaluate_a_short_circuit_left_operand() {
    let source = r#"module spec.eei.leftonce;

let calls: i32 = 0;

func idx(): i32 {
    calls = calls + 1;
    return 1;
}

func mk(): i32[2] {
    return [2, 0];
}

let s: u8[2] = [1, 2];
let x: bool = (i32(s[idx()]) == 2) && (mk()[0] == 2);

func main(): i32 {
    if (x) {
        return calls + 10;
    }
    return calls;
}
"#;
    assert_run_exit("pin-left-once", source, 11);
}

/// Calls inside literal elements, a template placeholder, and a fill
/// condition bind and build.
#[test]
fn should_bind_calls_inside_literals_and_fill_conditions() {
    let element = r#"module spec.eei.litelement;

func mk(): i32[2] {
    return [3, 9];
}

let a: i32[2] = [mk()[0], 2];

func main(): i32 {
    return a[0];
}
"#;
    assert_run_exit("pin-literal-element", element, 3);

    let row = r#"module spec.eei.rowliteral;

func mk(): i32[2] {
    return [3, 9];
}

let m: i32[2][2] = [[1, 2], mk()];

func main(): i32 {
    return m[1][0];
}
"#;
    assert_run_exit("pin-row-literal", row, 3);

    let condition = r#"module spec.eei.fillcond;

func mk(): i32[2] {
    return [3, 9];
}

let a: i32[2] = (mk()[0] == 3) ? [4, 5] : [0, 0];

func main(): i32 {
    return a[0];
}
"#;
    assert_run_exit("pin-fill-condition", condition, 4);

    let template = r#"module spec.eei.templatearg;

func mk(): i32[2] {
    return [3, 9];
}

func use(s: str): i32 {
    return i32(len(s));
}

let n: i32 = use($"{i32(mk()[0])}");

func main(): i32 {
    return n;
}
"#;
    assert_run_exit("pin-template-arg", template, 1);
}

/// An impure operand under a call argument stores once; the guard and the
/// read share the stored value.
#[test]
fn should_single_evaluate_operands_under_call_arguments() {
    let source = r#"module spec.eei.argoperand;

let calls: i32 = 0;

func idx(): i32 {
    calls = calls + 1;
    return 0;
}

func g(v: i32): i32 {
    return v;
}

let s: u8[2] = [7, 2];
let x: i32 = g(i32(s[idx()]));

func main(): i32 {
    return calls * 10 + x;
}
"#;
    assert_run_exit("pin-arg-operand", source, 17);
}

/// A slice view checks its offsets where the view is built, so an eager
/// out-of-range slice faults; a slice in a skipped arm does not.
#[test]
fn should_guard_an_eager_slice_view() {
    let out_of_range = r#"module spec.eei.sliceguard;

import { slice } from std.mem;

let arr: i32[4] = [1, 2, 3, 4];
let n: i32 = 5;
let v: i32[] = slice(arr, 0, n);

func main(): i32 {
    return i32(v[0]);
}
"#;
    let output = run_host("pin-slice-guard", out_of_range);
    assert!(
        !output.status.success(),
        "pin-slice-guard: an out-of-range view must fault"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E11211"),
        "pin-slice-guard: expected the bounds diagnostic; stderr:\n{stderr}"
    );

    let skipped_arm = r#"module spec.eei.slicearmskip;

import { slice } from std.mem;

func pick(): bool {
    return false;
}

let arr: i32[4] = [1, 2, 3, 4];
let n: i32 = 5;
let w: i32[] = pick() ? slice(arr, 0, n) : slice(arr, 0, 1);

func main(): i32 {
    return i32(w[0]);
}
"#;
    assert_run_exit("pin-slice-arm-skip", skipped_arm, 1);
}

/// A `||` under an enclosing choice keeps the guard: a skipped guard never
/// reaches the right side, and the taken path runs it once.
#[test]
fn should_keep_or_lazy_under_an_enclosing_choice() {
    let source = r#"module spec.eei.orguard;

let calls: i32 = 0;

func p(): bool {
    return false;
}

func f(): bool {
    calls = calls + 10;
    return false;
}

func idx(): i32 {
    calls = calls + 1;
    return 0;
}

let s: u8[2] = [1, 2];
let b: bool = p() ? (f() || (s[idx()] == 1)) : true;

func main(): i32 {
    return calls;
}
"#;
    assert_run_exit("pin-or-guard-skip", source, 0);
    let taken = source.replace(
        "func p(): bool {\n    return false;",
        "func p(): bool {\n    return true;",
    );
    assert_run_exit("pin-or-guard-taken", &taken, 11);
}

/// A written slice type argument in an initializer needs its typedef.
#[test]
fn should_emit_the_typedef_for_a_written_slice_type_argument() {
    let source = r#"module spec.eei.sizetypedef;

let n: usize = sizeof(i32[]);

func main(): i32 {
    if (n > 0) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit_host("pin-size-typedef", source);
    assert!(
        emitted.contains("ClSlice_i32"),
        "pin-size-typedef: the written slice type needs its emitted typedef"
    );
    assert_run_exit("pin-size-typedef-run", source, 1);
}

/// A condition inside a row element runs once, not once per element store.
#[test]
fn should_evaluate_a_row_element_condition_once() {
    let source = r#"module spec.eei.rowcond;

let calls: i32 = 0;

func pick(): bool {
    calls = calls + 1;
    return true;
}

let a: i32[2][2] = [pick() ? [1, 2] : [3, 4], [5, 6]];

func main(): i32 {
    return calls * 10 + a[0][0];
}
"#;
    assert_run_exit("pin-row-condition", source, 11);
}

/// A slice built inside a skipped arm neither runs its guard nor faults; a
/// taken arm faults on the out-of-range view, and the offset runs once.
#[test]
fn should_keep_a_slice_guard_lazy_under_an_enclosing_choice() {
    let source = r#"module spec.eei.slicelenarm;

import { slice } from std.mem;

func pick(): bool {
    return false;
}

let arr: i32[4] = [1, 2, 3, 4];
let n: i32 = 5;
let m: usize = pick() ? slice(arr, 0, n).len() : usize(0);

func main(): i32 {
    return i32(m);
}
"#;
    assert_run_exit("pin-slice-len-arm-skip", source, 0);
    let taken = source.replace(
        "func pick(): bool {\n    return false;",
        "func pick(): bool {\n    return true;",
    );
    let output = run_host("pin-slice-len-arm-taken", &taken);
    assert!(
        !output.status.success(),
        "pin-slice-len-arm-taken: the taken arm must fault on the out-of-range view"
    );

    let or_skip = r#"module spec.eei.slicelenor;

import { slice } from std.mem;

func yes(): bool {
    return true;
}

let arr: i32[4] = [1, 2, 3, 4];
let ok: bool = yes() || (i32(len(slice(slice(arr, 0, 5), 0, 1))) > 0);

func main(): i32 {
    if (ok) {
        return 1;
    }
    return 0;
}
"#;
    assert_run_exit("pin-slice-len-or-skip", or_skip, 1);

    let or_offset = r#"module spec.eei.slicelenoroffset;

import { slice } from std.mem;

let calls: i32 = 0;

func yes(): bool {
    return true;
}

func f(): i32 {
    calls = calls + 1;
    return 0;
}

let arr: i32[4] = [1, 2, 3, 4];
let ok: bool = yes() || (i32(len(slice(arr, f(), 2))) > 0);

func main(): i32 {
    return calls;
}
"#;
    assert_run_exit("pin-slice-len-or-offset", or_offset, 0);
}

/// A view-typed conditional arm with an impure offset stores the offset once,
/// on the taken path, so the guard and the construction share it.
#[test]
fn should_store_a_lazy_slice_offset_once() {
    let source = r#"module spec.eei.slicearmoffset;

import { slice } from std.mem;

let calls: i32 = 0;

func pick(): bool {
    return false;
}

func f(): i32 {
    calls = calls + 1;
    return 0;
}

let arr: i32[4] = [1, 2, 3, 4];
let w: i32[] = pick() ? slice(arr, f(), 2) : slice(arr, 0, 1);

func main(): i32 {
    return calls * 10 + i32(w[0]);
}
"#;
    assert_run_exit("pin-slice-arm-offset-skip", source, 0 * 10 + 1);

    let taken = source.replace(
        "func pick(): bool {\n    return false;",
        "func pick(): bool {\n    return true;",
    );
    let emitted = emit_host("pin-slice-arm-offset-taken", &taken);
    assert_eq!(
        count_occurrences(&emitted, "spec_eei_slicearmoffset_f()"),
        1,
        "pin-slice-arm-offset-taken: the offset must run once"
    );
    assert_run_exit("pin-slice-arm-offset-taken-run", &taken, 1 * 10 + 1);
}

/// A guarded nested-slice operand stores through its empty state and builds.
#[test]
fn should_build_a_guarded_nested_slice_operand() {
    let source = r#"module spec.eei.nestedslice;

import { slice } from std.mem;

func pick(): bool {
    return true;
}

func idx(): i32 {
    return 0;
}

let arr: i32[4] = [1, 2, 3, 4];
let x: i32[2] = [5, 6];
let v: i32 = pick() ? slice(slice(arr, 0, 2), 0, 1)[0] : x[idx()];

func main(): i32 {
    return v;
}
"#;
    assert_run_exit("pin-nested-slice", source, 1);
}

/// A fill-literal element's impure operand stores once, and the element
/// stores do not multiply its guard with the literal's depth.
#[test]
fn should_single_evaluate_a_fill_element_operand() {
    let source = r#"module spec.eei.fillelem;

let calls: i32 = 0;

func idx(): i32 {
    calls = calls + 1;
    return 0;
}

let s: u8[2] = [7, 2];
let a: i32[2] = [i32(s[idx()]), 9];

func main(): i32 {
    return calls * 10 + a[0];
}
"#;
    let emitted = emit_host("pin-fill-element", source);
    assert_eq!(
        count_occurrences(&emitted, "if ((__cl_itmp0 < 0)"),
        2,
        "pin-fill-element: the walk's guard and the element store's guard are the only copies"
    );
    assert_run_exit("pin-fill-element-run", source, 17);
}

/// A structural `len` binds nothing and evaluates nothing: the extent is the
/// folded constant and the base's instance is not emitted.
#[test]
fn should_not_bind_a_structural_len_argument() {
    let source = r#"module spec.eeiimpl.lenbinding;

func mkArr<T>(x: T): T[2] {
    return [x, x];
}

func g<T>(x: T): i32 {
    return i32(mkArr(x).len);
}

func main(): i32 {
    return g<i32>(5) * 10 + 2;
}
"#;
    let emitted = emit_host("pin-len-binding", source);
    assert!(
        !emitted.contains("__cl_owned"),
        "pin-len-binding: a structural length needs no owned temporary"
    );
    assert!(
        !emitted.contains("lenbinding_mkArr"),
        "pin-len-binding: the folded length does not run the base"
    );
    assert_contains_all(
        "pin-len-binding",
        &emitted,
        &["return (i32)(2u);"],
    );
    assert_run_exit("pin-len-binding-run", source, 22);
}

/// An owned struct's empty return takes the emitted zero helper, never a
/// compound literal.
#[test]
fn should_emit_the_struct_zero_helper() {
    let source = r#"module spec.eeiimpl.emptyret;

struct P {
    x: i32;
    y: i32;
}

func maybe(flag: bool): P {
    if (flag) {
        return P { x: 1, y: 2 };
    }
}

func main(): i32 {
    let p: P = maybe(false);
    return p.x + p.y + maybe(true).x;
}
"#;
    let emitted = emit_host("pin-zero-helper", source);
    assert_contains_all(
        "pin-zero-helper",
        &emitted,
        &[
            "static inline ClP cl_zero_ClP(void)",
            "return cl_zero_ClP();",
        ],
    );
    assert!(
        !emitted.contains("(ClP){0}"),
        "pin-zero-helper: the compound literal is gone"
    );
    assert_run_exit("pin-zero-helper-run", source, 1);
}

/// Source bindings named like the generated spellings re-emit
/// byte-identically with the de-duplicated names.
#[test]
fn should_reserve_the_generated_spellings() {
    let source = r#"module spec.eeiimpl.gennames;

import { String, stringOf } from std.text;

func demo(__cl_ret: i32): i32 {
    let s: String = stringOf("x");
    if (__cl_ret > 0) {
        return __cl_ret + i32(len(s));
    }
    return 0;
}

func outer(x: i32): i32 {
    let base: i32 = x;
    return ([base] () => {
        let __env: i32 = base + 1;
        return __env + 1;
    })();
}

func main(): i32 {
    return demo(4) + outer(3);
}
"#;
    let (first, second) = emit_twice("pin-generated-names", source);
    assert_eq!(
        first, second,
        "pin-generated-names: the emission must be deterministic"
    );
    assert_contains_all(
        "pin-generated-names",
        &first,
        &[
            "cl_spec_eeiimpl_gennames_demo(i32 __cl_ret_1)",
            "i32 __cl_ret = 0;",
            "i32 __env_1 = 0;",
        ],
    );
    assert!(
        !first.contains("(i32 __cl_ret)"),
        "pin-generated-names: the parameter must not keep the reserved spelling"
    );
    assert_run_exit("pin-generated-names-run", source, 10);

    let temp = r#"module spec.eeiimpl.tmpparam;

func rem(): i32 {
    return 0;
}

func guarded(__cl_tmp0: i32): i32 {
    let arr: u8[2] = [1, 2];
    return __cl_tmp0 + i32(arr[rem()]);
}

func main(): i32 {
    return guarded(5) + 1;
}
"#;
    let emitted = emit_host("pin-temp-param", temp);
    assert_contains_all(
        "pin-temp-param",
        &emitted,
        &[
            "cl_spec_eeiimpl_tmpparam_guarded(i32 __cl_tmp0_1)",
            "i32 __cl_tmp0 = 0;",
        ],
    );
    assert_run_exit("pin-temp-param-run", temp, 7);
}

/// A row-element slice typedef follows the struct definition it needs, while
/// the pointer-element control keeps its place before it.
#[test]
fn should_emit_row_slice_typedefs_after_the_struct_definitions() {
    let row_slice = r#"module spec.eeiimpl.rowsplit;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func total(rows: P[][2]): i32 {
    return rows[0][0].x + rows[1][1].y;
}

func main(): i32 {
    let back: P[3][2] = [[P { x: 1, y: 2 }, P { x: 3, y: 4 }], [P { x: 5, y: 6 }, P { x: 7, y: 8 }], [P { x: 9, y: 10 }, P { x: 11, y: 12 }]];
    let rows: P[][2] = slice(back, 0, 3);
    return total(rows);
}
"#;
    let emitted = emit_host("pin-row-split", row_slice);
    let definition = emitted
        .find("struct ClP {")
        .expect("pin-row-split: the struct definition is emitted");
    let row_typedef = emitted
        .find("} ClSlice_arr2_P;")
        .expect("pin-row-split: the row typedef is emitted");
    assert!(
        definition < row_typedef,
        "pin-row-split: the row typedef needs the complete struct"
    );

    let pointer_slice = r#"module spec.eeiimpl.ptrplace;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func main(): i32 {
    let ps: P[3] = [P { x: 1, y: 2 }, P { x: 3, y: 4 }, P { x: 5, y: 6 }];
    let s: P[] = slice(ps, 0, 3);
    return s[2].x + s[0].y;
}
"#;
    let emitted = emit_host("pin-ptr-place", pointer_slice);
    let definition = emitted
        .find("struct ClP {")
        .expect("pin-ptr-place: the struct definition is emitted");
    let pointer_typedef = emitted
        .find("} ClSlice_P;")
        .expect("pin-ptr-place: the pointer typedef is emitted");
    assert!(
        pointer_typedef < definition,
        "pin-ptr-place: a pointer-element typedef keeps its place"
    );
    assert_run_exit("pin-row-split-run", row_slice, 9);
}

/// A callable-bound parameter's field call goes through the field value.
#[test]
fn should_emit_the_bound_field_call_through_the_field() {
    let source = r#"module spec.eeiimpl.boundfield;

struct Holder<T> {
    call: T;
}

func use<T: func(i32): i32>(h: Holder<T>): i32 {
    return h.call(5);
}

func addOne(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    return use<func(i32): i32>(Holder<func(i32): i32> { call: addOne });
}
"#;
    let emitted = emit_host("pin-bound-field", source);
    assert_contains_all("pin-bound-field", &emitted, &["(h.call)(5)"]);
    assert_run_exit("pin-bound-field-run", source, 6);
}

/// An uncontracted extern receives a view's pointer with the ABI cast, while
/// an argument that is already a pointer (a `cstring` literal) never wraps.
#[test]
fn should_emit_the_view_pointer_coercions() {
    let source = r#"module spec.eeiimpl.coerce;

import { slice } from std.mem;
import { String, stringOf } from std.text;

ffi lib {
    func CText(s: cstring): i32;
    func RStr(s: str): i32;
    func RSlice(data: u8[]): i32;
}

func main(): i32 {
    let xs: u8[4] = [7, 2, 3, 4];
    let s: u8[] = slice(xs, 0, 4);
    let t: str = "hi";
    let owned: String = stringOf("z");
    let n: i32 = 0;
    unsafe {
        n = CText("x") + RStr(t) + RStr(owned) + RSlice(s);
    }
    return n;
}
"#;
    let emitted = emit_host("pin-coerce", source);
    assert_contains_all(
        "pin-coerce",
        &emitted,
        &[
            "CText(\"x\")",
            "RStr((APTR)(t).ptr)",
            "RSlice((APTR)(s).ptr)",
        ],
    );
    assert!(
        !emitted.contains("(\"x\").ptr"),
        "pin-coerce: a cstring literal is already a pointer"
    );
}

/// A callable field named `len` is called through the field value, not the
/// builtin length intrinsic.
#[test]
fn should_call_a_func_field_named_len_through_the_field() {
    let source = r#"module spec.eeiimpl.funcfieldlen;

struct Holder {
    len: func(i32): i32;
    tag: i32;
}

func addOne(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let h: Holder = Holder { len: addOne, tag: 7 };
    return h.len(5) + h.tag;
}
"#;
    let emitted = emit_host("pin-func-field-len", source);
    assert_contains_all("pin-func-field-len", &emitted, &["(h.len)(5)"]);
    assert_run_exit("pin-func-field-len-run", source, 13);
}

/// Compiles one host-profile program that must be rejected and returns the
/// compiler's stderr.
fn compile_stderr(tag: &str, source: &str) -> String {
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
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A projected read resolves through the substituted base: the per-instance
/// spelling reads the real field, a growable array reads its count, and the
/// concrete projection re-emits byte-identically.
#[test]
fn should_resolve_the_substituted_projection() {
    let borrowed = r#"module spec.eeiimpl.fieldlen;

import { String, stringOf } from std.text;

struct Box {
    s: String;
}

func takeBox<T>(borrow b: T, fallback: i32): i32 {
    if (i32(len(b.s)) > 0) {
        return i32(len(b.s));
    }
    return fallback;
}

func main(): i32 {
    let b: Box = Box { s: stringOf("abcdefg") };
    return takeBox<Box>(b, 3);
}
"#;
    let emitted = emit_host("pin-projection", borrowed);
    assert_contains_all(
        "pin-projection",
        &emitted,
        &["(cl_string_view((*b).s)).len"],
    );
    assert_run_exit("pin-projection-run", borrowed, 7);

    let growable = r#"module spec.eeiimpl.growlen;

import { Array, arrayNew } from std.mem;

struct Basket {
    items: Array<i32>;
}

func countOf<T>(borrow b: T): i32 {
    return i32(len(b.items));
}

func main(): i32 {
    let b: Basket = Basket { items: arrayNew<i32>() };
    b.items.push(5);
    b.items.push(7);
    return countOf<Basket>(b);
}
"#;
    let emitted = emit_host("pin-grow-projection", growable);
    assert_contains_all("pin-grow-projection", &emitted, &["((*b).items).count"]);
    assert_run_exit("pin-grow-projection-run", growable, 2);

    let concrete = r#"module spec.eeiimpl.concretefield;

import { String, stringOf } from std.text;

struct Plain {
    s: String;
}

func lenOf(b: Plain): i32 {
    return i32(len(b.s));
}

func main(): i32 {
    let b: Plain = Plain { s: stringOf("abc") };
    return lenOf(b);
}
"#;
    let (first, second) = emit_twice("pin-concrete-field", concrete);
    assert_eq!(
        first, second,
        "pin-concrete-field: the emission must be deterministic"
    );
    assert_contains_all(
        "pin-concrete-field",
        &first,
        &["(cl_string_view(b.s)).len"],
    );
    assert_run_exit("pin-concrete-field-run", concrete, 3);
}

/// The null literal's shape lowers to the generic pointer its value inhabits,
/// and the null pairing reads the local against the view's pointer.
#[test]
fn should_emit_the_null_local_declaration_and_its_pairing() {
    let inferred = r#"module spec.eeiimpl.nullinfer;

func main(): i32 {
    let x = null;
    if (x == null) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit_host("pin-null-infer", inferred);
    assert_contains_all(
        "pin-null-infer",
        &emitted,
        &["void * x = NULL;", "(x == NULL)"],
    );
    assert_run_exit("pin-null-infer-run", inferred, 1);

    let paired = r#"module spec.eeiimpl.nullpair;

func main(): i32 {
    let x = null;
    let s: str = "ab";
    if (x == s) {
        return 1;
    }
    return 2;
}
"#;
    let emitted = emit_host("pin-null-pair", paired);
    assert_contains_all(
        "pin-null-pair",
        &emitted,
        &["void * x = NULL;", "(x == (s).ptr)"],
    );
    assert_run_exit("pin-null-pair-run", paired, 2);
}

/// Each phase diagnostic reports once at its site: the view pair at the left
/// operand, the element loop at its header, the member callee at the callee.
#[test]
fn should_report_the_phase_diagnostics_once_at_the_site() {
    let view_eq = r#"module spec.eeiimpl.vieweq;

func main(): i32 {
    let a: str = "ab";
    let b: str = "ab";
    if (a == b) {
        return 1;
    }
    return 0;
}
"#;
    let stderr = compile_stderr("pin-view-eq", view_eq);
    assert_eq!(
        stderr.matches("error E10222:").count(),
        1,
        "pin-view-eq: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:6:9"),
        "pin-view-eq: the report lands on the left operand; stderr:\n{stderr}"
    );

    let str_loop = r#"module spec.eeiimpl.strloop;

func sumStr(s: str): i32 {
    let total: i32 = 0;
    for (borrow c in s) {
        total = total + i32(c);
    }
    return total;
}

func main(): i32 {
    return sumStr("ab");
}
"#;
    let stderr = compile_stderr("pin-str-loop", str_loop);
    assert_eq!(
        stderr.matches("error E10043:").count(),
        1,
        "pin-str-loop: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:5:5"),
        "pin-str-loop: the report lands on the loop header; stderr:\n{stderr}"
    );

    let bare = r#"module spec.eeiimpl.bareparam;

struct S {
    x: i32;
}

func g<T>(x: T): i32 {
    return x.foo(5);
}

func main(): i32 {
    return g<i32>(5);
}
"#;
    let stderr = compile_stderr("pin-bare-param", bare);
    assert_eq!(
        stderr.matches("error E10212:").count(),
        1,
        "pin-bare-param: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:8:12"),
        "pin-bare-param: the report lands on the callee; stderr:\n{stderr}"
    );
}

/// Each write-mutability rejection reports its exact code once at its site.
#[test]
fn should_report_the_write_mutability_codes_once_at_the_site() {
    let const_update = r#"module spec.eeiimpl.constupdate;

func main(): i32 {
    const a: u8[2] = [1, 2];
    let y: i32 = i32(a[0]++);
    return y;
}
"#;
    let stderr = compile_stderr("pin-const-update", const_update);
    assert_eq!(
        stderr.matches("error E10023:").count(),
        1,
        "pin-const-update: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:5:22"),
        "pin-const-update: the report lands on the updated place; stderr:\n{stderr}"
    );

    let const_slice = r#"module spec.eeiimpl.constsliceupdate;

import { slice } from std.mem;

func main(): i32 {
    let base: u8[2] = [1, 2];
    const s: u8[] = slice(base, 0, 2);
    let y: i32 = i32(s[0]++);
    return y;
}
"#;
    let stderr = compile_stderr("pin-const-slice", const_slice);
    assert_eq!(
        stderr.matches("error E10023:").count(),
        1,
        "pin-const-slice: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:8:22"),
        "pin-const-slice: the report lands on the updated place; stderr:\n{stderr}"
    );

    let readonly = r#"module spec.eeiimpl.paramupdate;

import { slice } from std.mem;

func bump(xs: u8[]): i32 {
    let z: i32 = i32(xs[0]++);
    return z;
}

func main(): i32 {
    let a: u8[2] = [1, 2];
    return bump(slice(a, 0, 2));
}
"#;
    let stderr = compile_stderr("pin-readonly-update", readonly);
    assert_eq!(
        stderr.matches("error E10024:").count(),
        1,
        "pin-readonly-update: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:6:22"),
        "pin-readonly-update: the report lands on the updated place; stderr:\n{stderr}"
    );

    let cstring_write = r#"module spec.eeiimpl.cstringwrite;

func fill(borrow c: cstring): i32 {
    c[0] = 65;
    return 0;
}

func main(): i32 {
    return fill("ab");
}
"#;
    let stderr = compile_stderr("pin-cstring-write", cstring_write);
    assert_eq!(
        stderr.matches("error E10042:").count(),
        1,
        "pin-cstring-write: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:4:5"),
        "pin-cstring-write: the report lands on the write; stderr:\n{stderr}"
    );

    let str_update = r#"module spec.eeiimpl.strvalueupdate;

func bump(borrow s: str): i32 {
    let w: i32 = i32(s[0]++);
    return w;
}

func main(): i32 {
    return bump("ab");
}
"#;
    let stderr = compile_stderr("pin-str-value-update", str_update);
    assert_eq!(
        stderr.matches("error E10042:").count(),
        1,
        "pin-str-value-update: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:4:22"),
        "pin-str-value-update: the report lands on the updated place; stderr:\n{stderr}"
    );
}

/// The immutable-view registry row names all three view types, and the
/// message and help stay unchanged.
#[test]
fn should_pin_the_immutable_view_registry_wording() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/14-diagnostics.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    let row = text
        .lines()
        .find(|line| line.starts_with("| E10042 |"))
        .expect("the E10042 row exists");
    assert_contains_all(
        "registry-row",
        row,
        &[
            "Write to an element of an immutable view (`str`, `static str`, or `cstring`)",
            "`cannot write through immutable view '{name}'`",
            "copy the bytes into an owned `String` or a `borrow`-derived `u8[]` view",
        ],
    );
}

/// The accepted write controls keep their emission and run: a writable slice
/// statement write, the value-position update on writable storage, and the
/// whole-view rebinds.
#[test]
fn should_keep_the_accepted_write_controls_byte_identical() {
    let writable = r#"module spec.eeiimpl.slicewrite;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let s: u8[] = slice(a, 0, 2);
    s[0] = 7;
    return i32(s[0]) + i32(a[0]);
}
"#;
    let (first, second) = emit_twice("pin-write-slice", writable);
    assert_eq!(
        first, second,
        "pin-write-slice: the emission must be deterministic"
    );
    assert_run_exit("pin-write-slice-run", writable, 14);

    let value_update = r#"module spec.eeiimpl.valueupdate;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let s: u8[] = slice(a, 0, 2);
    let y: i32 = i32(s[0]++);
    return y + i32(a[0]);
}
"#;
    let (first, second) = emit_twice("pin-write-value-update", value_update);
    assert_eq!(
        first, second,
        "pin-write-value-update: the emission must be deterministic"
    );
    assert_run_exit("pin-write-value-update-run", value_update, 3);

    let control = r#"module spec.eeiimpl.writecontrol;

import { slice } from std.mem;
import { stringOf, toCString } from std.text;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let s: u8[] = slice(a, 0, 2);
    s[0] = 7;
    let t: str = "ab";
    t = "cd";
    let c: cstring = toCString(stringOf("ef"));
    c = toCString(stringOf("gh"));
    if (i32(t[0]) != 99) {
        return 1;
    }
    if (i32(c[0]) != 103) {
        return 2;
    }
    return i32(s[0]) + i32(a[0]);
}
"#;
    let (first, second) = emit_twice("pin-write-control", control);
    assert_eq!(
        first, second,
        "pin-write-control: the emission must be deterministic"
    );
    assert_run_exit("pin-write-control-run", control, 14);
}

/// A view bound to an owned projection temporary emits an owner local in the
/// binding's scope, reads through it, and releases it on the scope cleanup,
/// with the re-assignment prologue releasing any value a previous evaluation
/// left when the scope re-opens.
#[test]
fn should_bind_the_view_owner_and_release_it_once_per_evaluation() {
    let source = r#"module spec.eeiimpl.viewtemp;

import { String, stringOf } from std.text;

struct Box {
    v: String;
}

func measure(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 3) {
        let s: str = (Box { v: stringOf("ab") }).v;
        total = total + measure(s);
        i = i + 1;
    }
    return total;
}
"#;
    let (first, second) = emit_twice("pin-view-owner", source);
    assert_eq!(
        first, second,
        "pin-view-owner: the emission must be deterministic"
    );
    assert_contains_all(
        "pin-view-owner",
        &first,
        &[
            "ClBox __cl_owned0 = {0};",
            "cl_string_view(__cl_owned0.v)",
        ],
    );
    assert_eq!(
        count_occurrences(&first, "FreeString(__cl_owned0.v)"),
        2,
        "pin-view-owner: the owner releases on the re-assignment and the cleanup"
    );
    assert_run_exit("pin-view-owner-run", source, 6);
}

/// A view copied out of its owner's scope reports once against the owner.
#[test]
fn should_report_the_escaped_view_once() {
    let source = r#"module spec.eeiimpl.escapedowner;

import { String, stringOf } from std.text;

struct Box {
    v: String;
}

func use(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: str = "zz";
    let i: i32 = 0;
    while (i < 2) {
        let t: str = (Box { v: stringOf("ab") }).v;
        if (i == 0) { s = t; }
        i = i + 1;
    }
    return use(s);
}
"#;
    let stderr = compile_stderr("pin-escaped-owner", source);
    assert_eq!(
        stderr.matches("error E10030:").count(),
        1,
        "pin-escaped-owner: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("does not live long enough"),
        "pin-escaped-owner: the report names the outlives rule; stderr:\n{stderr}"
    );
}

/// A capture carried inside a composite value reports the capture-move rule
/// once.
#[test]
fn should_reject_the_composite_capture_return_once() {
    let source = r#"module spec.eeiimpl.composite;

import { String, stringOf } from std.text;

struct Holder<T> {
    v: T;
}

func outer<T>(x: T): Holder<T> {
    return ([x] () => Holder<T> { v: x })();
}

func main(): i32 {
    let r: Holder<String> = outer<String>(stringOf("ab"));
    return i32(len(r.v));
}
"#;
    let stderr = compile_stderr("pin-composite-capture", source);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-composite-capture: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot move captured 'x'"),
        "pin-composite-capture: the report names the capture; stderr:\n{stderr}"
    );
}

/// The preempted ordinary return emits the release before the superseding
/// store, and the non-preempting control keeps its direct return.
#[test]
fn should_release_the_abandoned_result_before_the_superseding_store() {
    let source = r#"module spec.eeiimpl.preempt;

import { String, stringOf } from std.text;

func f(): String {
    switch (1) {
        case 1: {
            defer { break; }
            return stringOf("ab");
        }
        default: {
        }
    }
    return stringOf("cd");
}

func main(): i32 {
    let r: String = f();
    return i32(len(r));
}
"#;
    let (first, second) = emit_twice("pin-preempt", source);
    assert_eq!(
        first, second,
        "pin-preempt: the emission must be deterministic"
    );
    assert_contains_all(
        "pin-preempt",
        &first,
        &[
            "if (__cl_ret) { FreeString(__cl_ret); }",
            "__cl_ret = NULL;",
            "__cl_ret = cl_std_text_stringOf(cl_str(\"cd\"));",
        ],
    );
    assert_run_exit("pin-preempt-run", source, 2);

    let control = r#"module spec.eeiimpl.plainreturn;

import { String, stringOf } from std.text;

func f(): String {
    let i: i32 = 0;
    while (i < 5) {
        if (i == 1) { return stringOf("ab"); }
        i = i + 1;
    }
    return stringOf("cd");
}

func main(): i32 {
    let r: String = f();
    return i32(len(r));
}
"#;
    let (first, second) = emit_twice("pin-plain-return", control);
    assert_eq!(
        first, second,
        "pin-plain-return: the emission must be deterministic"
    );
    assert!(
        !first.contains("if (__cl_ret) { FreeString(__cl_ret); }"),
        "pin-plain-return: a return with no reachable preemption keeps its direct shape"
    );
    assert_run_exit("pin-plain-return-run", control, 2);
}

/// The func-value call binds and releases its owned temporary argument.
#[test]
fn should_materialize_the_func_value_temporary_argument() {
    let source = r#"module spec.eeiimpl.funcvalue;

import { String, stringOf } from std.text;

func copier(borrow v: String): String {
    return stringOf("y");
}

func wrap(f: func(borrow String): String): i32 {
    return i32(len(f(stringOf("x"))));
}

func main(): i32 {
    return wrap(copier);
}
"#;
    let (first, second) = emit_twice("pin-func-value", source);
    assert_eq!(
        first, second,
        "pin-func-value: the emission must be deterministic"
    );
    assert_contains_all(
        "pin-func-value",
        &first,
        &[
            "__cl_owned0 = cl_std_text_stringOf(cl_str(\"x\"));",
            "f(__cl_owned0)",
            "FreeString(__cl_owned0)",
        ],
    );
    assert_run_exit("pin-func-value-run", source, 1);
}

/// Every capture-move rejection reports its exact code once at its site.
#[test]
fn should_report_the_capture_move_codes_once_at_the_site() {
    let block_return = r#"module spec.eeiimpl.blockreturn;

import { String, stringOf } from std.text;

struct Wrap {
    v: String;
}

func outer<T>(x: T): T {
    return ([x] () => { return x; })();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    let r: Wrap = outer<Wrap>(w);
    return i32(len(r.v));
}
"#;
    let stderr = compile_stderr("pin-block-return", block_return);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-block-return: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:10:32"),
        "pin-block-return: the report lands on the moved capture; stderr:\n{stderr}"
    );

    let block_take = r#"module spec.eeiimpl.blocktake;

import { String, stringOf } from std.text;

struct Wrap {
    v: String;
}

func sink(take w: Wrap): i32 {
    return 0;
}

func outer<T>(x: T): i32 {
    return ([x] () => { sink(take x); return 0; })();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    return outer<Wrap>(w);
}
"#;
    let stderr = compile_stderr("pin-block-take", block_take);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-block-take: expected one report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:14:35"),
        "pin-block-take: the report lands on the moved capture; stderr:\n{stderr}"
    );

    let conditional = r#"module spec.eeiimpl.conditional;

import { String, stringOf } from std.text;

const C: bool = true;

struct Wrap {
    v: String;
}

func outer<T>(x: T): T {
    return ([x] () => C ? x : x)();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    let r: Wrap = outer<Wrap>(w);
    return i32(len(r.v));
}
"#;
    let stderr = compile_stderr("pin-conditional", conditional);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-conditional: expected one report for the two-arm site; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:12:23"),
        "pin-conditional: the report lands on the conditional; stderr:\n{stderr}"
    );

    let concrete = r#"module spec.eeiimpl.conditionalconcrete;

import { String, stringOf } from std.text;

const C: bool = true;

func outer(x: String): String {
    return ([x] () => C ? x : x)();
}

func main(): i32 {
    let r: String = outer(stringOf("ab"));
    return i32(len(r));
}
"#;
    let stderr = compile_stderr("pin-conditional-concrete", concrete);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-conditional-concrete: expected one report; stderr:\n{stderr}"
    );

    let take_argument = r#"module spec.eeiimpl.conditionaltake;

import { String, stringOf } from std.text;

const C: bool = true;

struct Wrap {
    v: String;
}

func sink(take w: Wrap): i32 {
    return 0;
}

func outer<T>(x: T): i32 {
    return ([x] () => sink(take (C ? x : x)))();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    return outer<Wrap>(w);
}
"#;
    let stderr = compile_stderr("pin-conditional-take", take_argument);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-conditional-take: expected one report; stderr:\n{stderr}"
    );
}

/// The duplicable conditional capture stays accepted and re-emits
/// byte-identically.
#[test]
fn should_keep_the_duplicable_conditional_capture_byte_identical() {
    let source = r#"module spec.eeiimpl.copycontrol;

const C: bool = true;

func outer<T>(x: T): T {
    return ([x] () => C ? x : x)();
}

func main(): i32 {
    return outer<i32>(42);
}
"#;
    let (first, second) = emit_twice("pin-conditional-copy", source);
    assert_eq!(
        first, second,
        "pin-conditional-copy: the emission must be deterministic"
    );
    assert_run_exit("pin-conditional-copy-run", source, 42);
}

/// The back-edge store releases the abandoned result before its store, and the
/// indexed and member projections of a composite capture report once.
#[test]
fn should_release_before_the_back_edge_store_and_reject_composite_projections() {
    let source = r#"module spec.eeiimpl.backedge;

import { String, stringOf } from std.text;

func f(): String {
    let i: i32 = 0;
    while (i < 4) {
        i = i + 1;
        if (i == 2) { return stringOf("aa"); }
        if (i == 1) { defer { continue; } return stringOf("bb"); }
    }
    return stringOf("cc");
}

func main(): i32 {
    let r: String = f();
    return i32(len(r));
}
"#;
    let (first, second) = emit_twice("pin-back-edge", source);
    assert_eq!(
        first, second,
        "pin-back-edge: the emission must be deterministic"
    );
    assert!(
        first.contains(
            "if (__cl_ret) { FreeString(__cl_ret); }\n    __cl_ret = NULL;\n    __cl_ret = cl_std_text_stringOf(cl_str(\"aa\"));"
        ),
        "pin-back-edge: the back-edge store does not release the abandoned result"
    );
    assert_run_exit("pin-back-edge-run", source, 2);

    let indexed = r#"module spec.eeiimpl.compositeindex;

import { String, stringOf } from std.text;

func outer(x: String): String {
    return ([x] () => [x][0])();
}

func main(): i32 {
    let r: String = outer(stringOf("ab"));
    return i32(len(r));
}
"#;
    let stderr = compile_stderr("pin-composite-index", indexed);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-composite-index: expected one report; stderr:\n{stderr}"
    );

    let member = r#"module spec.eeiimpl.compositemember;

import { String, stringOf } from std.text;

struct Holder<T> {
    v: T;
}

func outer<T>(x: T): T {
    return ([x] () => Holder<T> { v: x }.v)();
}

func main(): i32 {
    let r: String = outer<String>(stringOf("ab"));
    return i32(len(r));
}
"#;
    let stderr = compile_stderr("pin-composite-member", member);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-composite-member: expected one report; stderr:\n{stderr}"
    );

    let generic_indexed = r#"module spec.eeiimpl.compositeindexgeneric;

import { String, stringOf } from std.text;

func outer<T>(x: T): T {
    return ([x] () => [x][0])();
}

func main(): i32 {
    let r: String = outer<String>(stringOf("ab"));
    return i32(len(r));
}
"#;
    let stderr = compile_stderr("pin-composite-index-generic", generic_indexed);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-composite-index-generic: expected one report; stderr:\n{stderr}"
    );

    let take_argument = r#"module spec.eeiimpl.compositetake;

import { String, stringOf } from std.text;

struct Holder<T> {
    v: T;
}

func sink<T>(take h: Holder<T>): i32 {
    return 0;
}

func outer<T>(x: T): i32 {
    return ([x] () => sink(take Holder<T> { v: x }))();
}

func main(): i32 {
    return outer<String>(stringOf("ab"));
}
"#;
    let stderr = compile_stderr("pin-composite-take", take_argument);
    assert_eq!(
        stderr.matches("error E10401:").count(),
        1,
        "pin-composite-take: expected one report; stderr:\n{stderr}"
    );
}
