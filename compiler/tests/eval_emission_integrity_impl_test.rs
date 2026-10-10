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

/// A slice built inside a skipped arm neither runs its guard nor faults.
#[test]
fn should_keep_a_slice_guard_lazy_under_an_enclosing_choice() {
    let source = r#"module spec.eei.slicelenarm;

import { slice } from std.mem;

func pick(): bool {
    return false;
}

let arr: i32[4] = [1, 2, 3, 4];
let n: i32 = 5;
let m: usize = pick() ? len(slice(arr, 0, n)) : usize(0);

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
let n: i32 = 5;
let ok: bool = yes() || (i32(len(slice(arr, 0, n))) > 0);

func main(): i32 {
    if (ok) {
        return 1;
    }
    return 0;
}
"#;
    assert_run_exit("pin-slice-len-or-skip", or_skip, 1);
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
