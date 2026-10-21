//! Runnable specification tests for checker conformance.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. The memory helper
//! runs a case as an ordinary strict host build and adds leak and
//! use-after-free detection when `RDC_ASAN` is set.
//!
//! Phase 1 pins the initializer's single evaluation of an impure dividend and
//! an impure slice index base, the slice typedefs an inline argument, a
//! lifted specialization, and a generic struct-instance element need, and the
//! owning-index-base release that must stay unchanged.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one program may run before the case fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

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
            "clc-cri-{}-{tag}-{sequence}",
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

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Compiles `source` for the host profile and returns the scratch directory,
/// the compiler output, and the emitted-C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// Emits one host-profile program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// Compiles the emitted C with the host compiler under the strict flag set.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
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
    program
}

/// Asserts one compiler run rejected the program with exactly one `code`
/// diagnostic, that the message contains every fragment and none of the
/// absent markers, and that no translation unit is written.
fn assert_rejected_output(
    tag: &str,
    output: &Output,
    c_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = format!("error {code}:");
    assert_eq!(
        stderr.matches(&marker).count(),
        1,
        "{tag}: expected exactly one {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    for absent_marker in absent {
        assert!(
            !stderr.contains(absent_marker),
            "{tag}: the report {absent_marker:?} must be absent; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the rejection contract on the output.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str], absent: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
}


/// Runs a built program with a time limit and the given environment values.
fn run_with_timeout(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{tag}: the program did not finish within {RUN_TIMEOUT:?}");
            }
            Err(err) => panic!("{tag}: cannot wait for the program: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program output: {err}"))
}

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let program = build_host_c(tag, &scratch, &emitted);
    run_with_timeout(tag, &program, &[])
}

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
fn run_asan(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let emitted = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emitted.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-fsanitize=address",
            "-no-pie",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the sanitizer build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(
        tag,
        &program,
        &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")],
    )
}

/// Runs a memory-safety case: under AddressSanitizer when `RDC_ASAN` is set,
/// otherwise as an ordinary strict host build.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
}

/// Asserts the program exited with exactly `expected`, reporting its stderr.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit code {expected}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts the run produced no AddressSanitizer report.
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

/// Asserts the run aborted at a runtime trap for `code`.
fn assert_trap(tag: &str, output: &Output, code: &str) {
    assert!(
        !output.status.success(),
        "{tag}: a trapped run must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected the {code} trap; stderr:\n{stderr}"
    );
}

/// An impure dividend evaluates once in a module initializer.
///
/// The guard and the division read the same stored value, so a function with
/// a side effect runs exactly once (red at the phase base: the guard
/// re-rendered the call and the program observed two runs).
#[test]
fn should_evaluate_an_impure_dividend_once_in_an_initializer() {
    let source = r#"module spec.cc.initdividend;

let calls: i32 = 0;

func d(): i32 {
    calls = calls + 1;
    return 2;
}

let g: i32 = d() / 1;

func main(): i32 {
    return calls * 10 + g;
}
"#;
    let output = run_memory("initdividend", source);
    assert_no_asan("initdividend", &output);
    assert_exit("initdividend", &output, 12);
}

/// An impure slice index base evaluates once in a module initializer.
///
/// The bounds guard and the element access read the same stored view, so the
/// call behind the base runs exactly once (red at the phase base: the guard
/// re-rendered the call and the program observed two runs).
#[test]
fn should_evaluate_an_impure_index_base_once_in_an_initializer() {
    let source = r#"module spec.cc.initbase;

let calls: i32 = 0;
const s: str = "abcd";

func v(x: str): str {
    calls = calls + 1;
    return x;
}

let e: i32 = i32(v(s)[1]) - 96;

func main(): i32 {
    return calls * 10 + e;
}
"#;
    let output = run_memory("initbase", source);
    assert_no_asan("initbase", &output);
    assert_exit("initbase", &output, 12);
}

/// A slice built only as an inline argument defines its typedef.
///
/// The written view type reaches the program only as the argument value, so
/// the collector must take its element type from the value itself (red at the
/// phase base: the strict build failed with an undeclared `ClSlice_String`).
#[test]
fn should_reach_an_inline_argument_slice_typedef() {
    let source = r#"module spec.cc.inlinearg;

import { slice } from std.mem;
import { stringOf } from std.text;

func mkS(): String[2] {
    return [stringOf("ab"), stringOf("cd")];
}

func main(): i32 {
    return i32(len(slice(mkS(), 0, 2)));
}
"#;
    let output = run_memory("inlinearg", source);
    assert_no_asan("inlinearg", &output);
    assert_exit("inlinearg", &output, 2);
}

/// A lifted-lambda specialization defines its written slice typedef.
///
/// The specialization is emitted as its own function, so its written type
/// argument must join the typedef set (red at the phase base: the strict
/// build failed on the specialization's `sizeof(ClSlice_i32)`).
#[test]
fn should_define_a_lifted_specialization_slice_typedef() {
    let source = r#"module spec.cc.lambdaspec;

func makeSize<T>(): func(): usize {
    return () => sizeof(T[]);
}

func main(): i32 {
    let size: func(): usize = makeSize<i32>();
    if (size() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("lambdaspec", source);
    assert_no_asan("lambdaspec", &output);
    assert_exit("lambdaspec", &output, 0);
}

/// A generic struct-instance element defines its instance typedef.
///
/// The slice typedef names the struct instance, so the instance's own typedef
/// must be declared first (red at the phase base: the emitted typedef named
/// an undeclared instance type).
#[test]
fn should_declare_a_generic_struct_instance_slice_typedef() {
    let source = r#"module spec.cc.structelement;

struct Wrap<T> {
    value: T;
}

func sizeOf<T>(): usize {
    return sizeof(T[]);
}

func main(): i32 {
    if (sizeOf<Wrap<i32>>() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("structelement", source);
    assert_no_asan("structelement", &output);
    assert_exit("structelement", &output, 0);
}

/// An owning index base releases exactly once (the accepted reference).
///
/// A call-result fixed array behind a run-time index already stores once and
/// releases its elements at the init function's end; the fix must keep that
/// shape unchanged.
#[test]
fn should_release_an_owning_index_base_once() {
    let source = r#"module spec.cc.initownbase;

import { stringOf } from std.text;

let position: usize = 0;
let calls: i32 = 0;

func mk(): String[2] {
    calls = calls + 1;
    return [stringOf("ab"), stringOf("cd")];
}

let n: usize = len(mk()[position]);

func main(): i32 {
    return calls * 10 + i32(n);
}
"#;
    let output = run_memory("initownbase", source);
    assert_no_asan("initownbase", &output);
    assert_exit("initownbase", &output, 12);
}


/// A scalar instance of an opaque iterable is rejected at the instantiation.
///
/// The declaration accepts an opaque `T` because its instance may be an array;
/// this instance is not, so the loop reports the instance's judgment (red at
/// the phase base: the program was accepted and emitted a loop over a scalar).
#[test]
fn should_reject_a_scalar_instance_of_an_opaque_iterable() {
    let source = r#"module spec.cc.iterscalar;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    return each<i32>(0);
}
"#;
    assert_rejected_once(
        "iterscalar",
        source,
        "E10043",
        &["type 'i32' cannot be iterated"],
        &[],
    );
}

/// An enum instance of an opaque iterable is rejected at the instantiation.
#[test]
fn should_reject_an_enum_instance_of_an_opaque_iterable() {
    let source = r#"module spec.cc.iterenum;

enum E : u8 { A, B }

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    return each<E>(E.A);
}
"#;
    assert_rejected_once(
        "iterenum",
        source,
        "E10043",
        &["type 'E' cannot be iterated"],
        &[],
    );
}

/// A struct-element array instance binds the concrete element type.
///
/// The loop binding carries the substituted one-step element, so a duplicable
/// struct element copies exactly as the concrete spelling does (red at the
/// phase base: the binding was the unknown fallback and failed the strict
/// build).
#[test]
fn should_accept_a_struct_element_array_instance() {
    let source = r#"module spec.cc.iterstruct;

struct S {
    x: i32;
}

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: S[2] = [S { x: 1 }, S { x: 2 }];
    return each<S[2]>(a);
}
"#;
    let output = run_host("iterstruct", source);
    assert_exit("iterstruct", &output, 2);
}

/// A borrow element over a struct array instance binds the element type.
#[test]
fn should_accept_a_borrow_struct_element_array_instance() {
    let source = r#"module spec.cc.iterborrowstruct;

struct S {
    x: i32;
}

func eachB<T>(xs: T): i32 {
    let n: i32 = 0;
    for (borrow v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: S[2] = [S { x: 1 }, S { x: 2 }];
    return eachB<S[2]>(a);
}
"#;
    let output = run_host("iterborrowstruct", source);
    assert_exit("iterborrowstruct", &output, 2);
}

/// A multi-extent array instance binds the row element, not the innermost one.
///
/// The loop binds one row per step and the row carries the substituted element
/// type (red at the phase base: the binding was the innermost scalar's
/// fallback and failed the strict build).
#[test]
fn should_accept_a_multi_extent_array_instance() {
    let source = r#"module spec.cc.iterrow;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return each<i32[2][3]>(a);
}
"#;
    let output = run_host("iterrow", source);
    assert_exit("iterrow", &output, 2);
}

/// A float-element array instance binds the float element.
///
/// The read keeps the element's own type, so the accumulation is a float sum
/// (red at the phase base: the binding truncated each element to an integer
/// and the instance computed 3.0 instead of 4.0).
#[test]
fn should_accept_a_float_element_array_instance() {
    let source = r#"module spec.cc.iterf32;

func sumf<T>(xs: T): f32 {
    let s: f32 = 0.0;
    for (v in xs) {
        s = s + v;
    }
    return s;
}

func main(): i32 {
    let a: f32[2] = [1.5, 2.5];
    return i32(sumf<f32[2]>(a) * 10.0);
}
"#;
    let output = run_host("iterf32", source);
    assert_exit("iterf32", &output, 40);
}

/// A non-borrow copy of a non-duplicable element requires `borrow`.
///
/// The instance applies the concrete `E10040` judgment the declaration could
/// not make (red at the phase base: the instance was accepted and failed the
/// strict build).
#[test]
fn should_require_borrow_for_a_non_duplicable_element_instance() {
    let source = r#"module spec.cc.iterstring;

import { stringOf } from std.text;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cd")];
    return each<String[2]>(a);
}
"#;
    assert_rejected_once(
        "iterstring",
        source,
        "E10040",
        &["element loop requires 'borrow' for non-duplicable 'String'"],
        &["cannot be iterated"],
    );
}

/// A null-shaped iterable is rejected at the declaration.
///
/// The null literal's shape has no element sequence, so the loop reports the
/// null rule (red at the phase base: the program was accepted and failed the
/// strict build on an unsigned comparison).
#[test]
fn should_reject_a_null_shaped_iterable() {
    let source = r#"module spec.cc.iternull;

func main(): i32 {
    let x = null;
    for (v in x) {
    }
    return 0;
}
"#;
    assert_rejected_once(
        "iternull",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &["cannot be iterated"],
    );
}

/// A scalar-element array instance builds and runs (the reference).
#[test]
fn should_accept_a_scalar_element_array_instance() {
    let source = r#"module spec.cc.iterok;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let s: i32[4] = [1, 2, 3, 4];
    return each<i32[4]>(s);
}
"#;
    let output = run_host("iterok", source);
    assert_exit("iterok", &output, 4);
}

/// An enum-element array instance builds and runs (the reference).
#[test]
fn should_accept_an_enum_element_array_instance() {
    let source = r#"module spec.cc.iterenumelem;

enum Dir : u8 { N, S }

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: Dir[2] = [Dir.N, Dir.S];
    return each<Dir[2]>(a);
}
"#;
    let output = run_host("iterenumelem", source);
    assert_exit("iterenumelem", &output, 2);
}

/// A borrow element over a scalar array instance builds and runs.
#[test]
fn should_accept_a_borrow_element_array_instance() {
    let source = r#"module spec.cc.iterborrow;

func eachB<T>(xs: T): i32 {
    let n: i32 = 0;
    for (borrow v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let a: i32[4] = [1, 2, 3, 4];
    return eachB<i32[4]>(a);
}
"#;
    let output = run_host("iterborrow", source);
    assert_exit("iterborrow", &output, 4);
}

/// A value-form member read on a bare type parameter is judged per instance.
///
/// The instance's substituted type has no such field, so the read reports the
/// unknown member at the instance instead of emitting an unresolved member
/// access (red at the phase base: the program was accepted and failed the
/// strict build).
#[test]
fn should_reject_a_value_form_member_read_on_a_type_parameter() {
    let source = r#"module spec.cc.memval;

func get<T>(x: T): i32 {
    return x.foo;
}

func main(): i32 {
    return get<i32>(1);
}
"#;
    assert_rejected_once(
        "memval",
        source,
        "E10212",
        &["'foo' is not a member of 'i32'"],
        &[],
    );
}

/// The call form's unknown-member report is unchanged (one report).
#[test]
fn should_keep_one_report_for_a_call_form_member_on_a_type_parameter() {
    let source = r#"module spec.cc.memvalcall;

func callIt<T>(x: T): i32 {
    return x.foo();
}

func main(): i32 {
    return callIt<i32>(1);
}
"#;
    assert_rejected_once(
        "memvalcall",
        source,
        "E10212",
        &["'foo' is not a member of 'T'"],
        &[],
    );
}

/// A conditional-wrapped projection folds through the substituted base.
///
/// Both arms name the same field of the substituted struct, so the structural
/// fold reads the real extent (red at the phase base: the instance emitted
/// zero where the extent is seven).
#[test]
fn should_fold_a_conditional_wrapped_projection_through_the_substitution() {
    let source = r#"module spec.cc.condfold;

struct B {
    s: i32[7];
}

func cond<T>(b: T, flag: bool): usize {
    return len(flag ? b.s : b.s);
}

func main(): i32 {
    return i32(cond(B { s: [1, 2, 3, 4, 5, 6, 7] }, true));
}
"#;
    let output = run_host("condfold", source);
    assert_exit("condfold", &output, 7);
}

/// The conditional fold still evaluates nothing.
///
/// The condition behind the folded `len` must not run and the arms' projections
/// must not evaluate; the value is the extent and the side-effect counter stays
/// zero (red at the phase base: the instance emitted zero).
#[test]
fn should_keep_the_conditional_fold_evaluating_nothing() {
    let source = r#"module spec.cc.condfolddrop;

struct B {
    s: i32[7];
}

let calls: i32 = 0;

func callFlag(): bool {
    calls = calls + 1;
    return true;
}

func condD<T>(b: T): usize {
    return len(callFlag() ? b.s : b.s);
}

func main(): i32 {
    let n: usize = condD(B { s: [1, 2, 3, 4, 5, 6, 7] });
    return calls * 100 + i32(n);
}
"#;
    let output = run_host("condfolddrop", source);
    assert_exit("condfolddrop", &output, 7);
}

/// The concrete twin of the conditional fold is unchanged (the reference).
#[test]
fn should_fold_a_concrete_conditional_projection_unchanged() {
    let source = r#"module spec.cc.condfoldconcrete;

struct B {
    s: i32[7];
}

func condC(b: B, flag: bool): usize {
    return len(flag ? b.s : b.s);
}

func main(): i32 {
    return i32(condC(B { s: [1, 2, 3, 4, 5, 6, 7] }, true));
}
"#;
    let output = run_host("condfoldconcrete", source);
    assert_exit("condfoldconcrete", &output, 7);
}

/// A resolving projection at a struct instance keeps the read (the reference).
///
/// The value-form member read resolves through the instance's substituted
/// type, so the late resolution stays the designed path and runs.
#[test]
fn should_accept_a_resolving_projection_at_a_struct_instance() {
    let source = r#"module spec.cc.memok;

struct W {
    foo: i32;
}

func get<T>(x: T): i32 {
    return x.foo;
}

func main(): i32 {
    return get(W { foo: 7 });
}
"#;
    let output = run_host("memok", source);
    assert_exit("memok", &output, 7);
}

/// A projection through a raw pointer to a struct instance resolves.
///
/// The audit mirrors the value-form checker's raw arm, so the pointee's field
/// resolves and the accepted read runs (red at the fix-round base: the audit
/// reported the field missing).
#[test]
fn should_accept_a_raw_pointer_projection_at_a_struct_instance() {
    let source = r#"module spec.cc.memraw;

struct S {
    foo: i32;
}

func get<T>(x: T): i32 {
    return x.foo;
}

func main(): i32 {
    let s: S = S { foo: 7 };
    let p: raw S = unsafe &s;
    return get<raw S>(p);
}
"#;
    let output = run_host("memraw", source);
    assert_exit("memraw", &output, 7);
}

/// A value-form enum member on an instance reports the unknown member.
///
/// The audit mirrors the concrete rule: a value receiver has no member
/// surface, so every value-form read reports the registry message while the
/// scoped `Type.MEMBER` spelling stays accepted.
#[test]
fn should_reject_an_undeclared_enum_member_on_an_instance() {
    let source = r#"module spec.cc.memenum;

enum E : u8 { A, B }

func pick<T>(x: T): i32 {
    return i32(x.N);
}

func main(): i32 {
    return pick<E>(E.B);
}
"#;
    assert_rejected_once(
        "memenum",
        source,
        "E10212",
        &["'N' is not a member of 'E'"],
        &[],
    );
}

// ===== Phase 3: call, member, and cstring surfaces =====

/// A literal at an uncontracted `cstring` parameter builds and runs.
#[test]
fn should_accept_a_literal_at_a_cstring_parameter() {
    let source = r#"module spec.cc.cstringliteral;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    unsafe {
        let f: raw u8 = Open("/definitely/not/here", 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    let output = run_host("cstringliteral", source);
    assert_exit("cstringliteral", &output, 7);
}

/// A `str` view at a `cstring` parameter reports the conversion.
///
/// A `cstring` parameter is an ordinary value parameter and the conversion
/// table is exhaustive: only a `static str` literal converts implicitly, so
/// the view reports the conversion rather than handing the C bytes no
/// zero-terminator guarantee covers (red at the phase base: the view struct
/// was passed and the strict build failed).
#[test]
fn should_report_a_view_at_a_cstring_parameter() {
    let source = r#"module spec.cc.cstringview;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let s: str = "/definitely/not/here";
    unsafe {
        let f: raw u8 = Open(s, 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    assert_rejected_once(
        "cstringview",
        source,
        "E10200",
        &["cannot convert 'str' to 'cstring' implicitly"],
        &[],
    );
}

/// A slice view at a `cstring` parameter reports the conversion.
#[test]
fn should_report_a_slice_at_a_cstring_parameter() {
    let source = r#"module spec.cc.cstringslicearg;

import { stringOf, bytes } from std.text;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let owned: String = stringOf("/definitely/not/here");
    let v: u8[] = bytes(owned);
    unsafe {
        let f: raw u8 = Open(v, 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    assert_rejected_once(
        "cstringslicearg",
        source,
        "E10200",
        &["cannot convert 'u8[]' to 'cstring' implicitly"],
        &[],
    );
}

/// A `String` handle at a `cstring` parameter reports the conversion.
///
/// The conversion table does not list the handle, so the explicit route
/// (`toCString`) is the spelling (red at the phase base: the handle was
/// accepted and crossed as its block pointer).
#[test]
fn should_report_a_string_handle_at_a_cstring_parameter() {
    let source = r#"module spec.cc.cstringowned;

import { stringOf } from std.text;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let owned: String = stringOf("/definitely/not/here");
    unsafe {
        let f: raw u8 = Open(owned, 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    assert_rejected_once(
        "cstringowned",
        source,
        "E10200",
        &["cannot convert 'String' to 'cstring' implicitly"],
        &[],
    );
}

/// A `raw` pointer at a `cstring` parameter reports the conversion.
#[test]
fn should_report_a_raw_pointer_at_a_cstring_parameter() {
    let source = r#"module spec.cc.cstringraw;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let a: u8[4] = [65, 66, 67, 0];
    let p: raw u8 = unsafe &a[0];
    unsafe {
        let f: raw u8 = Open(p, 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    assert_rejected_once(
        "cstringraw",
        source,
        "E10200",
        &["cannot convert 'raw u8' to 'cstring' implicitly"],
        &[],
    );
}

/// A dot-call arity mismatch reports once with the remaining parameter count.
///
/// The receiver fills the first parameter, so no argument remains (red at the
/// phase base: the call was accepted and the strict build failed).
#[test]
fn should_report_a_dot_call_arity_mismatch() {
    let source = r#"module spec.cc.dotarity;

func addOne(a: i32): i32 {
    return a + 1;
}

func main(): i32 {
    let x: i32 = 1;
    return x.addOne(5);
}
"#;
    assert_rejected_once(
        "dotarity",
        source,
        "E10308",
        &["wrong number of arguments for 'addOne': expected 0, found 1"],
        &[],
    );
}

/// A direct-call arity mismatch reports once.
#[test]
fn should_report_a_direct_call_arity_mismatch() {
    let source = r#"module spec.cc.directarity;

func addOne(a: i32): i32 {
    return a + 1;
}

func main(): i32 {
    return addOne(1, 2);
}
"#;
    assert_rejected_once(
        "directarity",
        source,
        "E10308",
        &["wrong number of arguments for 'addOne': expected 1, found 2"],
        &[],
    );
}

/// A generic direct-call arity mismatch reports once.
#[test]
fn should_report_a_generic_call_arity_mismatch() {
    let source = r#"module spec.cc.genericarity;

func identity<T>(x: T): T {
    return x;
}

func main(): i32 {
    return identity<i32>(1, 2);
}
"#;
    assert_rejected_once(
        "genericarity",
        source,
        "E10308",
        &["wrong number of arguments for 'identity': expected 1, found 2"],
        &[],
    );
}

/// An `ffi` direct-call arity mismatch reports once.
#[test]
fn should_report_an_ffi_call_arity_mismatch() {
    let source = r#"module spec.cc.ffiarity;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    unsafe {
        let f: raw u8 = Open("/definitely/not/here", 0, 1);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    assert_rejected_once(
        "ffiarity",
        source,
        "E10308",
        &["wrong number of arguments for 'Open': expected 2, found 3"],
        &[],
    );
}

/// Exact counts in both call forms build and run.
#[test]
fn should_accept_exact_argument_counts_in_both_forms() {
    let source = r#"module spec.cc.arityok;

func addOne(a: i32): i32 {
    return a + 1;
}

func addTwo(a: i32, b: i32): i32 {
    return a + b;
}

func main(): i32 {
    let x: i32 = 10;
    let y: i32 = x.addOne();
    return addTwo(y, 5);
}
"#;
    let output = run_host("arityok", source);
    assert_exit("arityok", &output, 16);
}

/// An unknown member callee on a struct receiver reports once.
#[test]
fn should_report_an_unknown_member_callee_on_a_struct() {
    let source = r#"module spec.cc.memberunknown;

struct S {
    x: i32;
}

func main(): i32 {
    let s: S = S { x: 1 };
    return s.foo(5);
}
"#;
    assert_rejected_once(
        "memberunknown",
        source,
        "E10212",
        &["'foo' is not a member of 'S'"],
        &[],
    );
}

/// An unknown member callee through a `raw` receiver reports once.
#[test]
fn should_report_an_unknown_member_callee_through_a_raw_receiver() {
    let source = r#"module spec.cc.memberraw;

struct S {
    x: i32;
}

func main(): i32 {
    let s: S = S { x: 1 };
    let p: raw S = unsafe &s;
    return p.foo(5);
}
"#;
    assert_rejected_once(
        "memberraw",
        source,
        "E10212",
        &["'foo' is not a member of 'raw S'"],
        &[],
    );
}

/// A zero-parameter free function is not a dot-call candidate.
///
/// A candidate must accept the receiver as its first parameter, so a
/// zero-parameter function has zero matches and the member is unknown (red at
/// the phase base: the receiver was passed and the strict build failed).
#[test]
fn should_report_a_zero_parameter_free_function_as_an_unknown_member() {
    let source = r#"module spec.cc.memberzero;

struct S {
    x: i32;
}

func zero(): i32 {
    return 5;
}

func main(): i32 {
    let s: S = S { x: 1 };
    return s.zero();
}
"#;
    assert_rejected_once(
        "memberzero",
        source,
        "E10212",
        &["'zero' is not a member of 'S'"],
        &[],
    );
}

/// A callable field wins over a same-named free function.
#[test]
fn should_keep_the_callable_field_winning_over_a_free_function() {
    let source = r#"module spec.cc.memberfield;

struct S {
    foo: func(i32): i32;
}

func foo(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let s: S = S { foo: foo };
    return s.foo(1);
}
"#;
    let output = run_host("memberfield", source);
    assert_exit("memberfield", &output, 2);
}

/// `toCString` accepts a `str` view (the widened signature).
#[test]
fn should_accept_a_view_argument_for_to_c_string() {
    let source = r#"module spec.cc.cstringviewconv;

import { toCString } from std.text;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let s: str = "/definitely/not/here";
    let c: cstring = toCString(s);
    unsafe {
        let f: raw u8 = Open(c, 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    let output = run_host("cstringviewconv", source);
    assert_exit("cstringviewconv", &output, 7);
}

/// `toCString` keeps accepting an owned `String` caller (the widening).
#[test]
fn should_keep_an_owned_argument_for_to_c_string() {
    let source = r#"module spec.cc.cstringownedconv;

import { stringOf, toCString } from std.text;

ffi probe {
    func Open(path: cstring, mode: i32): raw u8;
}

func main(): i32 {
    let owned: String = stringOf("/definitely/not/here");
    let c: cstring = toCString(owned);
    unsafe {
        let f: raw u8 = Open(c, 0);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    let output = run_host("cstringownedconv", source);
    assert_exit("cstringownedconv", &output, 7);
}

/// An inferred generic call with a wrong count reports only the arity.
///
/// The inference cannot bind from arguments that are missing, so its
/// missing-type-parameter report would be a cascade of the arity mistake; the
/// suppression is pinned here (red at the fix-round base: two reports).
#[test]
fn should_report_only_the_arity_for_an_inferred_generic_call() {
    let source = r#"module spec.cc.genericaritycascade;

func first<T>(a: T, b: T): T {
    return a;
}

func main(): i32 {
    return i32(first());
}
"#;
    assert_rejected_once(
        "genericaritycascade",
        source,
        "E10308",
        &["wrong number of arguments for 'first': expected 2, found 0"],
        &["E10301"],
    );
}

/// A rejected `ffi` declaration type reports once per declaration.
///
/// The declared parameter types resolve at the first call site and are
/// cached, so a second call does not repeat the declaration's report (red at
/// the fix-round base: one report per call site).
#[test]
fn should_report_an_ffi_declaration_type_once() {
    let source = r#"module spec.cc.ffideclonce;

ffi probe {
    func cl_probe_zero(x: i32[0]): i32;
}

func main(): i32 {
    unsafe {
        return cl_probe_zero() + cl_probe_zero();
    }
}
"#;
    assert_rejected_once("ffideclonce", source, "E10226", &[], &[]);
}

// ===== Phase 4: null-shaped values =====

/// A concrete value stored into the null-shaped local reports the null rule.
///
/// The local inferred from `null` has no concrete type to hold the view, so
/// the store reports instead of emitting `void * = ClStr` (red at the phase
/// base: the program was accepted and failed the strict build).
#[test]
fn should_report_a_concrete_store_into_a_null_shaped_local() {
    let source = r#"module spec.cc.nullassignview;

func main(): i32 {
    let x = null;
    let s: str = "ab";
    x = s;
    return 0;
}
"#;
    assert_rejected_once(
        "nullassignview",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );
}

/// A null-shaped value returned where a concrete type is required reports.
///
/// The narrowed guard does not give the value a concrete type, so the return
/// reports the null rule (red at the phase base: the program was accepted and
/// emitted `void *` where `i32` is declared).
#[test]
fn should_report_a_null_shaped_return_value() {
    let source = r#"module spec.cc.nullreturn;

func pick(): i32 {
    let x = null;
    if (x != null) {
        return x;
    }
    return 0;
}

func main(): i32 {
    return pick();
}
"#;
    assert_rejected_once(
        "nullreturn",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );
}

/// A null-shaped value at a concrete parameter reports the null rule.
#[test]
fn should_report_a_null_shaped_argument() {
    let source = r#"module spec.cc.nullarg;

func consume(n: i32): i32 {
    return n;
}

func main(): i32 {
    let x = null;
    return consume(x);
}
"#;
    assert_rejected_once(
        "nullarg",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );
}

/// The null comparisons stay accepted and run.
#[test]
fn should_keep_the_null_comparisons_accepted() {
    let source = r#"module spec.cc.nullcompare;

func main(): i32 {
    let x = null;
    if (x == null) {
        return 1;
    }
    if (x != null) {
        return 2;
    }
    return 0;
}
"#;
    let output = run_host("nullcompare", source);
    assert_exit("nullcompare", &output, 1);
}

/// A nullable declaration and its null guards stay accepted.
#[test]
fn should_keep_the_nullable_declarations_accepted() {
    let source = r#"module spec.cc.nullnullable;

import { File } from std.io;

func main(): i32 {
    let f: File? = null;
    if (f != null) {
        return 1;
    }
    let p: raw u8? = null;
    if (p == null) {
        return 3;
    }
    return 0;
}
"#;
    let output = run_host("nullnullable", source);
    assert_exit("nullnullable", &output, 3);
}

/// A copy between two null-shaped locals stays accepted.
#[test]
fn should_keep_a_copy_between_null_shaped_locals_accepted() {
    let source = r#"module spec.cc.nullcopy;

func main(): i32 {
    let x = null;
    let y = x;
    if (y == null) {
        return 4;
    }
    return 0;
}
"#;
    let output = run_host("nullcopy", source);
    assert_exit("nullcopy", &output, 4);
}

// ===== Phase 5: diagnostic hygiene =====

/// A value update on a wholly moved local's field reports one state error.
///
/// The operand walk and the read/write checks all reach the state reporters,
/// so the site used to report three times (red at the phase base).
#[test]
fn should_report_a_moved_field_update_once() {
    let source = r#"module spec.cc.statededupe;

import { String, stringOf } from std.text;

struct P { s: String; n: i32; }

func main(): i32 {
    let p = P { s: stringOf("ab"), n: 1 };
    let q = p;
    let y = p.n++;
    return q.n + y;
}
"#;
    assert_rejected_once(
        "statededupe",
        source,
        "E10020",
        &["'p' was moved and cannot be used"],
        &[],
    );
}

/// The statement form of the same update reports once too.
#[test]
fn should_report_a_moved_field_statement_update_once() {
    let source = r#"module spec.cc.statededupestmt;

import { String, stringOf } from std.text;

struct P { s: String; n: i32; }

func main(): i32 {
    let p = P { s: stringOf("ab"), n: 1 };
    let q = p;
    p.n++;
    return q.n;
}
"#;
    assert_rejected_once(
        "statededupestmt",
        source,
        "E10020",
        &["'p' was moved and cannot be used"],
        &[],
    );
}

/// An uninitialized element update reports one state error.
#[test]
fn should_report_an_uninitialized_element_update_once() {
    let source = r#"module spec.cc.statededupeinit;

func main(): i32 {
    let a: i32[2];
    a[0]++;
    return 0;
}
"#;
    assert_rejected_once(
        "statededupeinit",
        source,
        "E10021",
        &["'a' may be used before it is initialized"],
        &[],
    );
}

/// An uninitialized field update reports one state error.
#[test]
fn should_report_an_uninitialized_field_update_once() {
    let source = r#"module spec.cc.statededupefield;

struct P { n: i32; m: i32; }

func main(): i32 {
    let p: P;
    p.n++;
    return 0;
}
"#;
    assert_rejected_once(
        "statededupefield",
        source,
        "E10021",
        &["'p.n' may be used before it is initialized"],
        &[],
    );
}

/// A legal update still builds and runs.
#[test]
fn should_keep_a_legal_update_accepted() {
    let source = r#"module spec.cc.statededupeok;

func main(): i32 {
    let a: i32[2] = [1, 2];
    a[0]++;
    let n: i32 = 3;
    n++;
    return a[0] + n;
}
"#;
    let output = run_host("statededupeok", source);
    assert_exit("statededupeok", &output, 6);
}

/// A generic capture moved into a local reports the capture-move rule at the
/// owning instance, not the lend-storage rule (red at the phase base: E10034).
#[test]
fn should_attribute_a_let_bound_capture_move_to_the_capture_rule() {
    let source = r#"module spec.cc.capturelet;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        let y: T = x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "capturelet",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// The assignment form of the same move reports the capture-move rule.
#[test]
fn should_attribute_an_assigned_capture_move_to_the_capture_rule() {
    let source = r#"module spec.cc.captureassign;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        let y: T;
        y = x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "captureassign",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// The same move at a duplicable instance stays accepted.
#[test]
fn should_keep_a_duplicable_capture_move_accepted() {
    let source = r#"module spec.cc.captureduplicable;

func make<T>(x: T): void {
    ([x] () => {
        let y: T = x;
    })();
}

func main(): i32 {
    make<i32>(5);
    return 0;
}
"#;
    let output = run_host("captureduplicable", source);
    assert_exit("captureduplicable", &output, 0);
}

/// A block-bodied composite capture return reports the capture rule alone
/// (red at the phase base: E10401 and E10034 together).
#[test]
fn should_report_a_block_composite_capture_return_once() {
    let source = r#"module spec.cc.captureblock;

import { String, stringOf } from std.text;

struct Holder<T> { v: T; }

func make<T>(x: T): Holder<T> {
    return ([x] () => {
        return Holder<T> { v: x };
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    let h: Holder<String> = make<String>(s);
    return i32(len(h.v));
}
"#;
    assert_rejected_once(
        "captureblock",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// The expression-bodied twin reports once after the fix; at the baseline it
/// paired the two reports exactly as the block body did.
#[test]
fn should_report_an_expression_composite_capture_return_once() {
    let source = r#"module spec.cc.captureexpr;

import { String, stringOf } from std.text;

struct Holder<T> { v: T; }

func make<T>(x: T): Holder<T> {
    return ([x] () => Holder<T> { v: x })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    let h: Holder<String> = make<String>(s);
    return i32(len(h.v));
}
"#;
    assert_rejected_once(
        "captureexpr",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// An escaped generated owner renders a display phrase, not its compiler
/// spelling.
#[test]
fn should_render_a_generated_owner_as_a_phrase() {
    let source = r#"module spec.cc.escapeowner;

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
    assert_rejected_once(
        "escapeowner",
        source,
        "E10030",
        &["a compiler-generated owner", "does not live long enough"],
        &["__cl_"],
    );
}

/// A by-value capture stored into a `borrow T?` out-slot reports the capture
/// rule at a nullable owning instance.
///
/// The capture's environment field is an aliasing copy of the enclosing
/// binding at an owning substitution, so storing it through the slot hands the
/// caller an unsound alias; the out-slot carve-out governs lend parameters,
/// not captures. The base reported the lend rule (`E10034`) at the store and
/// the capture rule reports instead, so acceptance is unchanged.
#[test]
fn should_reject_a_capture_stored_into_an_out_slot() {
    let source = r#"module spec.cc.captureoutslot;

import { String, stringOf } from std.text;

func make<T>(borrow out: T, x: T): void {
    ([x, borrow out] () => {
        out = x;
    })();
}

func main(): i32 {
    let out: String? = null;
    let s: String = stringOf("ab");
    make<String?>(borrow out, s);
    return 0;
}
"#;
    assert_rejected_once(
        "captureoutslot",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// One capture keeps one report even when a lambda moves it at two sites.
///
/// The second site's use of the moved capture is the ordinary moved-value
/// error, so the capture rule still reports once for the lambda.
#[test]
fn should_report_one_capture_move_for_two_sites() {
    let source = r#"module spec.cc.capturetwosite;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        let a: T = x;
        let b: T = x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "capturetwosite",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// A partially moved struct's remaining-field update stays accepted, so the
/// per-site dedupe never masks a legitimate update.
#[test]
fn should_keep_a_remaining_field_update_accepted() {
    let source = r#"module spec.cc.statededupepartial;

import { String, stringOf } from std.text;

struct P { s: String; n: i32; }

func main(): i32 {
    let p = P { s: stringOf("ab"), n: 1 };
    let q: String = p.s;
    p.n++;
    return p.n + i32(len(q));
}
"#;
    let output = run_host("statededupepartial", source);
    assert_exit("statededupepartial", &output, 4);
}

// ===== Phase 5 fix round =====

/// A capture moved into an array literal at a call argument reports the
/// capture rule.
///
/// The element store consumes the capture, and the literal sits in a position
/// no enclosing move site covers; without the element record the program was
/// accepted and double-freed.
#[test]
fn should_report_a_capture_moved_into_an_array_argument() {
    let source = r#"module spec.cc.capturearrayarg;

import { String, stringOf } from std.text;

func observe<T>(v: T[1]): void {}

func make<T>(x: T): void {
    ([x] () => {
        observe([x]);
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "capturearrayarg",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// A capture moved into an array literal in an iterable position reports the
/// capture rule.
#[test]
fn should_report_a_capture_moved_into_an_iterable_literal() {
    let source = r#"module spec.cc.captureiterable;

import { String, stringOf } from std.text;

func observe<T>(v: T[1]): void {}

func make<T>(x: T): void {
    ([x] () => {
        for (y in [x]) {
            observe([y]);
        }
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "captureiterable",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// A capture moved through an `unsafe` wrapper reports the capture rule.
#[test]
fn should_report_a_capture_moved_through_unsafe() {
    let source = r#"module spec.cc.captureunsafe;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        let y: T = unsafe x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "captureunsafe",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// A read of a duplicable projection of a capture stays accepted.
///
/// The move is judged at the place it consumes: the numeric field copies out
/// of the capture, and the base runs it clean.
#[test]
fn should_keep_a_duplicable_capture_projection_accepted() {
    let source = r#"module spec.cc.captureprojection;

import { String, stringOf } from std.text;

struct O { s: String; n: i32; }

func make<T>(x: T): void {
    ([x] () => {
        let n: i32 = x.n;
    })();
}

func main(): i32 {
    let o: O = O { s: stringOf("ab"), n: 7 };
    make<O>(o);
    return 0;
}
"#;
    let output = run_host("captureprojection", source);
    assert_exit("captureprojection", &output, 0);
}

/// A move of an owning projection of a capture reports the capture rule
/// alone.
#[test]
fn should_report_an_owning_capture_projection_once() {
    let source = r#"module spec.cc.captureprojectionowning;

import { String, stringOf } from std.text;

struct O { s: String; n: i32; }

func make<T>(x: T): void {
    ([x] () => {
        let s: String = x.s;
    })();
}

func main(): i32 {
    let o: O = O { s: stringOf("ab"), n: 7 };
    make<O>(o);
    return 0;
}
"#;
    assert_rejected_once(
        "captureprojectionowning",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034", "E10026"],
    );
}

/// A returned duplicable projection of a capture stays accepted, so a copy out
/// of a borrow is never mistaken for a move.
#[test]
fn should_keep_a_returned_duplicable_projection_accepted() {
    let source = r#"module spec.cc.returncaptureprojection;

import { String, stringOf } from std.text;

struct O { s: String; n: i32; }

func read<T>(x: T): i32 {
    return ([x] () => {
        return x.n;
    })();
}

func main(): i32 {
    let o: O = O { s: stringOf("ab"), n: 7 };
    return read<O>(o);
}
"#;
    let output = run_host("returncaptureprojection", source);
    assert_exit("returncaptureprojection", &output, 7);
}

/// A parenthesized member update keeps the field-specific state text at its
/// site.
#[test]
fn should_keep_the_field_specific_state_text_at_a_colliding_site() {
    let source = r#"module spec.cc.statededupeparen;

struct P { n: i32; m: i32; }

func main(): i32 {
    let p: P;
    let y = (p).n++;
    return y;
}
"#;
    assert_rejected_once(
        "statededupeparen",
        source,
        "E10021",
        &["'p.n' may be used before it is initialized"],
        &["'p' may be used"],
    );
}

/// A capture moved through a `const` initializer reports the capture rule.
///
/// The `const` initializer is a move site of its own; without the record the
/// program was accepted and double-freed.
#[test]
fn should_report_a_capture_moved_through_a_const_initializer() {
    let source = r#"module spec.cc.captureconst;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        const y: T = x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "captureconst",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// The same `const` initializer at a duplicable instance stays accepted.
#[test]
fn should_keep_a_duplicable_const_capture_move_accepted() {
    let source = r#"module spec.cc.captureconstdup;

func make<T>(x: T): void {
    ([x] () => {
        const y: T = x;
    })();
}

func main(): i32 {
    make<i32>(5);
    return 0;
}
"#;
    let output = run_host("captureconstdup", source);
    assert_exit("captureconstdup", &output, 0);
}

/// A capture consumed by a bare discarded array literal reports the capture
/// rule.
#[test]
fn should_report_a_capture_in_a_discarded_literal() {
    let source = r#"module spec.cc.capturebarediscard;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        [x];
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "capturebarediscard",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// A capture repeated by an array fill reports the capture rule.
#[test]
fn should_report_a_capture_repeated_by_a_fill() {
    let source = r#"module spec.cc.capturefill;

import { String, stringOf } from std.text;

func make<T>(x: T): void {
    ([x] () => {
        let ys: T[2] = [; x];
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    make<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "capturefill",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

// ===== Loop element placeholder, inference, and bounds =====

/// A member read of a loop element passed to a generic callee is keyed by the
/// substituted field type.
///
/// The type stage binds the loop element as the iterable's one-step element
/// placeholder, so `v.y` resolves to the substituted row's member (`i32`) and
/// the inferred `id` instance declares and emits; the run reads the summed
/// members back (red at the phase base: the argument stayed `unknown`, the
/// instance keyed `cl_id__unknown`, and the strict build failed with an
/// implicit declaration).
#[test]
fn should_key_a_loop_element_member_read_through_a_generic_callee() {
    let source = r#"module spec.cc.elementmembercall;

struct S {
    y: i32;
}

func id<T>(x: T): T {
    return x;
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + id(v.y);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;
    let output = run_memory("elementmembercall", source);
    assert_no_asan("elementmembercall", &output);
    assert_exit("elementmembercall", &output, 3);
}

/// A direct member read of a loop element stays accepted (the control).
///
/// The element binding carries the substituted row's type into the read, so
/// the loop sums the members without a generic callee in between; the run
/// reads the summed members back (green control at the phase base).
#[test]
fn should_accept_a_direct_member_read_of_a_loop_element() {
    let source = r#"module spec.cc.elementmembercallctl;

struct S {
    y: i32;
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + i32(v.y);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;
    let output = run_memory("elementmembercallctl", source);
    assert_no_asan("elementmembercallctl", &output);
    assert_exit("elementmembercallctl", &output, 3);
}

/// A closure capture of a loop element carries the element in its
/// environment field.
///
/// The captured binding's checked type is the element placeholder, so the
/// lifted lambda's environment declares the element struct and the member
/// read resolves at the instance; the run reads the summed members back (red
/// at the phase base: the environment declared `i32 v` for a `ClS` element,
/// `incompatible types when initializing type 'int' using type 'ClS'` and
/// `request for member 'y'`).
#[test]
fn should_type_a_loop_element_capture_environment_from_the_element() {
    let source = r#"module spec.cc.elementcapture;

struct S {
    y: i32;
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + ([v] () => v.y)();
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;
    let output = run_memory("elementcapture", source);
    assert_no_asan("elementcapture", &output);
    assert_exit("elementcapture", &output, 3);
}

/// A concrete loop element capture stays accepted (the control).
///
/// The same capture over a concrete struct array types its environment field
/// from the element struct and reads the member; the run reads the summed
/// members back (green control at the phase base).
#[test]
fn should_accept_a_concrete_loop_element_capture() {
    let source = r#"module spec.cc.elementcapturectl;

struct S {
    y: i32;
}

func eachc(xs: S[2]): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + ([v] () => v.y)();
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return eachc(a);
}
"#;
    let output = run_memory("elementcapturectl", source);
    assert_no_asan("elementcapturectl", &output);
    assert_exit("elementcapturectl", &output, 3);
}

/// An inferred generic call on the element placeholder keys its instance.
///
/// The argument is the loop element itself, so the inferred `inner` instance
/// maps the placeholder through the instance substitution, declares, and
/// emits; the run reads the summed elements back (red at the phase base: the
/// instance keyed `cl_inner__unknown` and the strict build failed with an
/// implicit declaration).
#[test]
fn should_key_an_inferred_generic_call_on_the_element_placeholder() {
    let source = r#"module spec.cc.elementinfer;

func inner<T>(x: T): T {
    return x;
}

func outer<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + inner(v);
    }
    return n;
}

func main(): i32 {
    let a: i32[2] = [1, 2];
    return outer<i32[2]>(a);
}
"#;
    let output = run_memory("elementinfer", source);
    assert_no_asan("elementinfer", &output);
    assert_exit("elementinfer", &output, 3);
}

/// A concrete element argument keys its inferred instance (the control).
///
/// The same inner call over a concrete array binds the argument to the
/// element type directly, so the instance declares and the run reads the
/// summed elements back (green control at the phase base).
#[test]
fn should_accept_a_concrete_element_inferred_argument() {
    let source = r#"module spec.cc.elementinferctl;

func inner(x: i32): i32 {
    return x;
}

func outer(xs: i32[2]): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + inner(v);
    }
    return n;
}

func main(): i32 {
    let a: i32[2] = [1, 2];
    return outer(a);
}
"#;
    let output = run_memory("elementinferctl", source);
    assert_no_asan("elementinferctl", &output);
    assert_exit("elementinferctl", &output, 3);
}

/// A null-shaped inferred argument reports the null rule before an instance
/// key derives.
///
/// The argument's shape is not an instance-able type, so the call reports the
/// frozen null judgment instead of inventing a null-shaped instance (red at
/// the phase base: the call was accepted and the strict build failed with an
/// implicit `cl_each__unknownQ` declaration).
#[test]
fn should_report_a_null_shaped_inferred_argument_with_the_null_rule() {
    let source = r#"module spec.cc.elementinfernull;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let x = null;
    return each(x);
}
"#;
    assert_rejected_once(
        "elementinfernull",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );
}

/// A null-shaped argument with a written type argument reports the null rule
/// (the control).
///
/// The parameter is concrete after the written type argument, so the call
/// already reaches the frozen null judgment; the inferred path must match it
/// (green control at the phase base).
#[test]
fn should_report_a_null_shaped_explicit_argument_with_the_null_rule() {
    let source = r#"module spec.cc.elementinfernullctl;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let x = null;
    return each<i32>(x);
}
"#;
    assert_rejected_once(
        "elementinfernullctl",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );
}

/// A constant index past the substituted row's extent reports out of range at
/// the instance.
///
/// The row element substitutes to `i32[3]`, so the constant index 5 reports
/// the frozen range error exactly as the concrete spelling does (red at the
/// phase base: the program was accepted and emitted the unguarded read).
#[test]
fn should_report_an_out_of_range_constant_row_index_at_the_instance() {
    let source = r#"module spec.cc.rowindexconst;

func rowsum<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[5];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return rowsum<i32[2][3]>(rows);
}
"#;
    assert_rejected_once(
        "rowindexconst",
        source,
        "E10216",
        &["index 5 is out of range for 'i32[3]'"],
        &[],
    );
}

/// A run-time index past the substituted row's extent traps at the bounds
/// guard.
///
/// The guard condition decides on the substituted row type, so the index 5
/// aborts at the frozen trap instead of reading past the row and the read
/// never completes (red at the phase base: no guard emitted and the run
/// returned a value).
#[test]
fn should_trap_when_a_runtime_row_index_is_out_of_range() {
    let source = r#"module spec.cc.rowindexrt;

func rowsum<T>(xs: T, i: usize): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[i];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let i: usize = 5;
    return rowsum<i32[2][3]>(rows, i);
}
"#;
    let output = run_memory("rowindexrt", source);
    assert_trap("rowindexrt", &output, "E11211");
}

/// The concrete out-of-range constant index stays rejected (the control).
///
/// The non-generic row sum's constant index 5 reports the frozen range error
/// before any emission (green control at the phase base).
#[test]
fn should_report_an_out_of_range_constant_row_index_concretely() {
    let source = r#"module spec.cc.rowindexconc;

func rowsumC(xs: i32[2][3]): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[5];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return rowsumC(rows);
}
"#;
    assert_rejected_once(
        "rowindexconc",
        source,
        "E10216",
        &["index 5 is out of range for 'i32[3]'"],
        &[],
    );
}

/// A concrete in-bounds row index stays accepted (the control).
///
/// Both constant indexes are inside the row extent 3, so the loop reads the
/// rows' members and the run reads their sum back (green control at the phase
/// base).
#[test]
fn should_accept_a_concrete_in_bounds_row_index() {
    let source = r#"module spec.cc.rowindexconcok;

func rowsumC(xs: i32[2][3]): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[0] + v[2];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return rowsumC(rows);
}
"#;
    let output = run_memory("rowindexconcok", source);
    assert_no_asan("rowindexconcok", &output);
    assert_exit("rowindexconcok", &output, 14);
}

/// An in-bounds run-time row index through the instance stays accepted (the
/// control).
///
/// The bounds guard on the substituted row passes for an in-bounds index, so
/// the loop reads each row's member and the run reads their sum back; the
/// guard must not fire where the concrete path does not (green control at the
/// phase base).
#[test]
fn should_accept_an_in_bounds_runtime_row_index() {
    let source = r#"module spec.cc.rowindexrtok;

func rowsum<T>(xs: T, i: usize): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[i];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let i: usize = 2;
    return rowsum<i32[2][3]>(rows, i);
}
"#;
    let output = run_memory("rowindexrtok", source);
    assert_no_asan("rowindexrtok", &output);
    assert_exit("rowindexrtok", &output, 9);
}

/// The concrete run-time out-of-range index still traps (the control).
///
/// The concrete row sum already carries the bounds guard, so index 5 aborts
/// at the frozen trap and the read never completes (green control at the
/// phase base).
#[test]
fn should_trap_when_a_concrete_runtime_row_index_is_out_of_range() {
    let source = r#"module spec.cc.rowindexconctrap;

func rowsumC(xs: i32[2][3], i: usize): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[i];
    }
    return n;
}

func main(): i32 {
    let rows: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let i: usize = 5;
    return rowsumC(rows, i);
}
"#;
    let output = run_memory("rowindexconctrap", source);
    assert_trap("rowindexconctrap", &output, "E11211");
}

// ===== Substituted index kinds, nested elements, and capture moves =====

/// A scalar instance indexed through its type parameter reports the frozen
/// non-indexable rule.
///
/// The substituted base is `i32`, which has no index surface: the instance
/// reports the concrete rule's code and message at the base, so the program
/// never reaches the strict build (red at the phase base: the read was
/// accepted and the emitted `xs[i]` failed `subscripted value`).
#[test]
fn should_reject_a_scalar_instance_indexed_through_a_type_parameter() {
    let source = r#"module spec.cc.corrscalarindex;

func at<T>(xs: T, i: usize): i32 {
    return xs[i];
}

func main(): i32 {
    let n: i32 = 7;
    let i: usize = 0;
    return at<i32>(n, i) - 7;
}
"#;
    assert_rejected_once(
        "corrscalarindex",
        source,
        "E10209",
        &["type 'i32' cannot be indexed"],
        &[],
    );
}

/// A string-view instance's element index is accepted and runs.
///
/// The substituted base is `str`, so the index yields its byte and the keyed
/// element type resolves; the run reads the byte back (red at the phase base:
/// the inferred element keyed `u8` with no instance declared, and the strict
/// build failed).
#[test]
fn should_accept_a_string_view_instance_element_index() {
    let source = r#"module spec.cc.corrstrindex;

func at<T>(xs: T, i: usize): u8 {
    return xs[i];
}

func main(): i32 {
    let s: str = "abc";
    let i: usize = 1;
    return i32(at<str>(s, i)) - 98;
}
"#;
    let output = run_memory("corrstrindex", source);
    assert_no_asan("corrstrindex", &output);
    assert_exit("corrstrindex", &output, 0);
}

/// A possibly-null substituted index base reports the null rule.
///
/// The parameter binds to `i32[]?`, so the base may be null exactly as a
/// concrete nullable spelling may be: the instance reports the frozen null
/// rule instead of emitting the read (red at the phase base: the program was
/// accepted and the emitted C read the null slice).
#[test]
fn should_reject_a_nullable_instance_index_base() {
    let source = r#"module spec.cc.corrnullindex;

func at<T>(xs: T, i: usize): i32 {
    return xs[i];
}

func main(): i32 {
    let s: i32[]? = null;
    let i: usize = 0;
    return at<i32[]?>(s, i);
}
"#;
    assert_rejected_once(
        "corrnullindex",
        source,
        "E10010",
        &["may be null here"],
        &[],
    );
}

/// A nested element loop in a generic body builds and runs.
///
/// Both element levels carry their own placeholder, so the inner loop's
/// element binds from the substituted row; the run reads the summed rows back
/// (red at the phase base: the doubly nested placeholder rendered into the C
/// declaration and the strict build failed).
#[test]
fn should_build_a_nested_element_loop_in_a_generic_body() {
    let source = r#"module spec.cc.corrnestedloop;

func total<T>(xs: T): i32 {
    let n: i32 = 0;
    for (row in xs) {
        for (x in row) {
            n = n + x;
        }
    }
    return n;
}

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return total<i32[2][3]>(m);
}
"#;
    let output = run_memory("corrnestedloop", source);
    assert_no_asan("corrnestedloop", &output);
    assert_exit("corrnestedloop", &output, 21);
}

/// A by-value capture of an owning element reports the concrete move rule.
///
/// The element's substituted type owns its field, so the capture's by-value
/// move aliases the array's storage; the instance reports the capture-move
/// rule once, exactly as the concrete twin reports its release rule (red at
/// the phase base: the duty skipped the element placeholder and the run
/// double-freed).
#[test]
fn should_reject_a_capture_move_of_an_owning_element() {
    let source = r#"module spec.cc.corrcapturemove;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        ([v] () => { let s: String = v.name; })();
        total = total + 1;
    }
    return total;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab"), y: 1 }];
    return each<S[1]>(a);
}
"#;
    assert_rejected_once(
        "corrcapturemove",
        source,
        "E10401",
        &["cannot move captured 'v' in a lambda"],
        &[],
    );

    let concrete = r#"module spec.cc.corrcapturemoveconcrete;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func eachc(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + ([v] () => { let s: String = v.name; return i32(len(bytes(s))); })();
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cd"), y: 2 }];
    return eachc(a);
}
"#;
    assert_rejected_once(
        "corrcapturemoveconcrete",
        concrete,
        "E10034",
        &["cannot release or store borrowed parameter 'v'"],
        &[],
    );
}

/// An unknown member of a substituted element reports the concrete rule.
///
/// The read stays accepted at the declaration; the instance resolves the
/// substituted struct and reports the frozen unknown-member rule once,
/// exactly as a concrete read would (red at the phase base: the audit left
/// the element placeholder unbound and the emitted C failed `'ClS' has no
/// member named 'z'`).
#[test]
fn should_reject_an_unknown_member_of_a_substituted_element() {
    let source = r#"module spec.cc.corrmember;

struct S { y: i32; }

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v.z;
    }
    return n;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;
    assert_rejected_once(
        "corrmember",
        source,
        "E10212",
        &["'z' is not a member of 'S'"],
        &[],
    );
}

// ===== Chained opaque indexes and conditional arguments =====

/// A chained opaque index's inner bounds judgment reaches the instance.
///
/// The inner base is the outer index's element placeholder, so the substituted
/// row supplies its own extent: the run-time index traps, the constant index
/// reports the frozen range error, and the in-bounds spelling reads the value
/// back (red at the phase base: only the outer guard was emitted and the run
/// read out of bounds).
#[test]
fn should_guard_a_chained_opaque_index_in_the_instance() {
    let runtime = r#"module spec.cc.corrchainidx;

func pick<T>(xs: T, i: usize, j: usize): i32 {
    return xs[i][j];
}

func main(): i32 {
    let g: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let i: usize = 1;
    let j: usize = 5;
    return pick<i32[2][3]>(g, i, j);
}
"#;
    let output = run_memory("corrchainidx", runtime);
    assert_no_asan("corrchainidx", &output);
    assert_trap("corrchainidx", &output, "E11211");

    let constant = r#"module spec.cc.corrchainidxconst;

func pick<T>(xs: T, i: usize): i32 {
    return xs[i][5];
}

func main(): i32 {
    let g: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let i: usize = 1;
    return pick<i32[2][3]>(g, i);
}
"#;
    assert_rejected_once(
        "corrchainidxconst",
        constant,
        "E10216",
        &["index 5 is out of range for 'i32[3]'"],
        &[],
    );

    let in_bounds = r#"module spec.cc.corrchainidxok;

func pick<T>(xs: T, i: usize, j: usize): i32 {
    return xs[i][j];
}

func main(): i32 {
    let g: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let i: usize = 1;
    let j: usize = 2;
    return pick<i32[2][3]>(g, i, j);
}
"#;
    let output = run_memory("corrchainidxok", in_bounds);
    assert_no_asan("corrchainidxok", &output);
    assert_exit("corrchainidxok", &output, 6);
}

/// A chained index of a loop element keeps the same inner judgment.
///
/// The element's own index reaches a row, so the inner index carries the row's
/// extent: the out-of-range index traps exactly as the concrete twin does (red
/// at the phase base: the run read past the row and returned a value).
#[test]
fn should_guard_a_chained_element_index_in_the_instance() {
    let source = r#"module spec.cc.corrchainelem;

func each<T>(xs: T, i: usize): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + v[0][i];
    }
    return n;
}

func main(): i32 {
    let g: i32[2][2][3] = [[[1, 2, 3], [4, 5, 6]], [[7, 8, 9], [10, 11, 12]]];
    let i: usize = 5;
    return each<i32[2][2][3]>(g, i);
}
"#;
    let output = run_memory("corrchainelem", source);
    assert_no_asan("corrchainelem", &output);
    assert_trap("corrchainelem", &output, "E11211");
}

/// A conditional argument of agreeing arms keys its instance.
///
/// Both arms resolve to the same substituted type, so the inferred argument
/// keys the concrete instance exactly as a direct argument would; the run
/// reads the selected member back (red at the phase base: no instance was
/// recorded and the strict build failed with an implicit keyed symbol).
#[test]
fn should_key_a_conditional_argument_in_the_instance() {
    let member = r#"module spec.cc.corrcondarg;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, c: bool): i32 {
    return id(c ? xs[0].y : xs[0].y);
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return pick<S[2]>(a, true) - 1;
}
"#;
    let output = run_memory("corrcondarg", member);
    assert_no_asan("corrcondarg", &output);
    assert_exit("corrcondarg", &output, 0);

    let place = r#"module spec.cc.corrcondplace;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, i: usize, c: bool): i32 {
    return id(c ? xs[i] : xs[i]).y;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i, true) - 2;
}
"#;
    let output = run_memory("corrcondplace", place);
    assert_no_asan("corrcondplace", &output);
    assert_exit("corrcondplace", &output, 0);
}

// ===== Member typing: enum value form and view members =====

/// A value-form enum member read reports the unknown member concretely.
///
/// The frozen member table spells an enum member `Type.MEMBER`, so a value
/// receiver has no member surface: the declared member reports the registered
/// unknown-member diagnostic instead of emitting the invalid C (red at the
/// phase base: the program was accepted and the strict build failed with
/// `request for member 'A' in something not a structure or union`).
#[test]
fn should_reject_a_concrete_value_form_enum_member_read() {
    let source = r#"module spec.cc.enumvaluemember;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.A;
    let n = i32(e.A);
    return n - 1;
}
"#;
    assert_rejected_once(
        "enumvaluemember",
        source,
        "E10212",
        &["'A' is not a member of 'E'"],
        &[],
    );
}

/// A value-form enum member read reports the unknown member at the instance.
///
/// The declaration accepts the opaque member read because the substituted
/// instance decides; a value-form read has no member surface at any
/// substitution, so the audit reports the registered unknown-member
/// diagnostic once, at the read's span (red at the phase base: the instance
/// was accepted and the strict build failed with `request for member 'A' in
/// something not a structure or union`).
#[test]
fn should_reject_an_instance_value_form_enum_member_read() {
    let source = r#"module spec.cc.enumvaluememberinst;

enum E : u8 { A, B }

func read<T>(x: T): i32 {
    return i32(x.A);
}

func main(): i32 {
    let e: E = E.B;
    return read<E>(e);
}
"#;
    assert_rejected_once(
        "enumvaluememberinst",
        source,
        "E10212",
        &["'A' is not a member of 'E'"],
        &[],
    );
}

/// The scoped enum member and the qualified name forms stay accepted.
///
/// `E.B` is the frozen `Type.MEMBER` spelling and a two-segment
/// module-qualified call (`A.B.f`) is an ordinary dotted name, so neither
/// passes through the value-form member rule; both runs read their values
/// back (green control at the phase base).
#[test]
fn should_keep_the_scoped_enum_member_forms_accepted() {
    let scoped = r#"module spec.cc.enumvaluememberctl;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.B;
    return i32(e);
}
"#;
    let output = run_memory("enumvaluememberctl", scoped);
    assert_no_asan("enumvaluememberctl", &output);
    assert_exit("enumvaluememberctl", &output, 1);

    let qualified = r#"module spec.cc.enumvaluememberqual;

func main(): i32 {
    let owned: String = std.text.stringOf("abc");
    let v: u8[] = std.text.bytes(owned);
    return i32(len(v));
}
"#;
    let output = run_memory("enumvaluememberqual", qualified);
    assert_no_asan("enumvaluememberqual", &output);
    assert_exit("enumvaluememberqual", &output, 3);
}

/// A view pointer member at an integer destination reports the conversion.
///
/// `.ptr` carries the view's raw pointer to its element (`raw u8` for `str`),
/// and the frozen table converts a raw pointer to an integer only through an
/// explicit cast, so the return reports the registered conversion (red at the
/// phase base: the program was accepted and the strict build failed with
/// `returning 'const u8 *' ... makes integer from pointer without a cast
/// [-Werror=int-conversion]`).
#[test]
fn should_report_a_view_pointer_member_at_an_integer_destination() {
    let source = r#"module spec.cc.viewmembers;

func f(s: str): usize {
    return s.ptr;
}

func main(): i32 {
    let s: str = "abc";
    return i32(f(s));
}
"#;
    assert_rejected_once(
        "viewmembers",
        source,
        "E10200",
        &["cannot convert 'raw u8' to 'usize' implicitly"],
        &[],
    );
}

/// The view length member stays accepted (the control).
///
/// `.len` is the view's element count typed `usize`, so the return needs no
/// conversion and the run reads the length back; the pinned `toCString`
/// cases keep the stdlib pointer-member conversion green alongside (green
/// control at the phase base).
#[test]
fn should_keep_the_view_length_member_accepted() {
    let source = r#"module spec.cc.viewmemberlen;

func span(s: str): usize {
    return s.len;
}

func main(): i32 {
    let s: str = "abcd";
    return i32(span(s));
}
"#;
    let output = run_memory("viewmemberlen", source);
    assert_no_asan("viewmemberlen", &output);
    assert_exit("viewmemberlen", &output, 4);
}

// ===== View pointer member conversions and the fixed-array member =====

/// A view pointer member read reaches a `cstring` store and a return and runs.
///
/// The member's ABI type is the view's byte pointer, and the frozen rules let
/// that read reach a `cstring` destination slot — here a local store — and a
/// return without a cast in the source; the run reads the non-null pointers
/// back. The same read at a call argument is not a destination slot: the
/// rejected half below reports the frozen conversion rule there.
#[test]
fn should_convert_a_view_pointer_member_at_a_c_string_destination() {
    let source = r#"module spec.cc.pointermembercstr;

func cstr(s: str): cstring {
    return s.ptr;
}

func main(): i32 {
    let s: str = "abc";
    let c: cstring = s.ptr;
    if (c == null) {
        return 1;
    }
    let d: cstring = cstr(s);
    if (d == null) {
        return 2;
    }
    return 0;
}
"#;
    let output = run_memory("pointermembercstr", source);
    assert_no_asan("pointermembercstr", &output);
    assert_exit("pointermembercstr", &output, 0);

    let argument = r#"module spec.cc.pointermembercstrarg;

func use(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let s: str = "abc";
    return use(s.ptr);
}
"#;
    assert_rejected_once(
        "pointermembercstrarg",
        argument,
        "E10200",
        &["cannot convert 'raw u8' to 'cstring' implicitly"],
        &[],
    );
}

/// A view pointer member read at an FFI argument reports the conversion rule.
///
/// An FFI argument is not a destination slot: the frozen conversion table has
/// no raw-pointer widening, so the read reports exactly one `E10200` and the
/// call never reaches the linker.
#[test]
fn should_reject_a_view_pointer_member_at_an_ffi_c_string_argument() {
    let source = r#"module spec.cc.pointermemberffi;

ffi probe {
    func cl_probe_take(c: cstring): i32;
}

func main(): i32 {
    let s: str = "abc";
    unsafe {
        return cl_probe_take(s.ptr);
    }
}
"#;
    assert_rejected_once(
        "pointermemberffi",
        source,
        "E10200",
        &["cannot convert 'raw u8' to 'cstring' implicitly"],
        &[],
    );
}

/// A view pointer member read at a struct-literal field reports the rule.
///
/// A struct-literal field is not a destination slot: the read reports exactly
/// one `E10200` (the emitter would carry the bare pointer and the strict build
/// would fail).
#[test]
fn should_reject_a_view_pointer_member_at_a_struct_literal_field() {
    let source = r#"module spec.cc.pointermemberfield;

struct Holder { c: cstring; }

func main(): i32 {
    let s: str = "abc";
    let h: Holder = Holder { c: s.ptr };
    if (h.c == null) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once(
        "pointermemberfield",
        source,
        "E10200",
        &["cannot convert 'raw u8' to 'cstring' implicitly"],
        &[],
    );
}

/// A byte pointer that is not a view member keeps the frozen conversion rule.
///
/// The frozen table has no raw-pointer-to-`cstring` widening, so a `raw u8`
/// local passed where a `cstring` is expected reports exactly one `E10200`,
/// the same report the base emits (the member read above is the sanctioned
/// exception).
#[test]
fn should_reject_a_non_member_byte_pointer_at_a_c_string_argument() {
    let source = r#"module spec.cc.rawcstrarg;

func use(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: u8[4] = [65, 66, 67, 0];
    let p: raw u8 = unsafe &a[0];
    return use(p);
}
"#;
    assert_rejected_once(
        "rawcstrarg",
        source,
        "E10200",
        &["cannot convert 'raw u8' to 'cstring' implicitly"],
        &[],
    );
}

/// A fixed-array instance has no pointer member; the slice twin does.
///
/// The member rule is the view's: a fixed array is not a view, so the
/// instance reports the frozen unknown-member rule exactly as the concrete
/// spelling would; the slice substitution carries the member and the run
/// reads the first byte back through it (red at the phase base: the
/// fixed-array instance was accepted and the emitted C failed `request for
/// member 'ptr'`).
#[test]
fn should_reject_a_fixed_array_pointer_member_at_the_instance() {
    let fixed = r#"module spec.cc.fixedptrmember;

func first<T>(x: T): raw u8 {
    return x.ptr;
}

func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let p: raw u8 = first<u8[4]>(a);
    unsafe {
        if (p == null) {
            return 1;
        }
    }
    return 0;
}
"#;
    assert_rejected_once(
        "fixedptrmember",
        fixed,
        "E10212",
        &["'ptr' is not a member of 'u8[4]'"],
        &[],
    );

    let slice_twin = r#"module spec.cc.sliceptrmember;

import { slice } from std.mem;

func first<T>(x: T): raw u8 {
    return x.ptr;
}

func main(): i32 {
    let a: u8[4] = [65, 66, 67, 68];
    let s: u8[] = slice(a, 0, 4);
    let p: raw u8 = first<u8[]>(s);
    unsafe {
        if (p == null) {
            return 1;
        }
        if (p[0] != 65) {
            return 2;
        }
    }
    return 0;
}
"#;
    let output = run_memory("sliceptrmember", slice_twin);
    assert_no_asan("sliceptrmember", &output);
    assert_exit("sliceptrmember", &output, 0);
}

/// A generic pointer member at an integer destination reports the conversion.
///
/// The instance resolves the member to the view's byte pointer, so the integer
/// return reports the frozen conversion rule exactly as the concrete spelling
/// does (red at the phase base: the instance was accepted and the emitted C
/// failed `int-conversion`).
#[test]
fn should_reject_a_generic_pointer_member_at_an_integer_destination() {
    let source = r#"module spec.cc.genptrint;

func span<T>(x: T): usize {
    return x.ptr;
}

func main(): i32 {
    let s: str = "abc";
    return i32(span<str>(s));
}
"#;
    assert_rejected_once(
        "genptrint",
        source,
        "E10200",
        &["cannot convert 'raw u8' to 'usize' implicitly"],
        &[],
    );
}

// ===== Call conversions and `ffi` result typing =====

/// A `str` instance at an `ffi` `cstring` parameter reports the conversion.
///
/// A parameter-typed argument's conversion is discharged at its instance: the
/// view's bytes carry no zero-terminator guarantee, so the frozen table
/// reports the conversion once instead of passing the view struct where the
/// `char *` parameter is declared (red at the phase base: the program was
/// accepted and the strict build failed with `incompatible type for argument
/// 1` for the view struct).
#[test]
fn should_report_a_view_instance_at_an_ffi_c_string_argument() {
    let source = r#"module spec.cc.genericconvert;

ffi probe {
    func cl_probe_convert(name: cstring, mode: i32): raw u8;
}

func convert<T>(x: T): raw u8 {
    unsafe {
        return cl_probe_convert(x, 0);
    }
}

func main(): i32 {
    let s: str = "abc";
    unsafe {
        let f: raw u8 = convert<str>(s);
        if (f == null) {
            return 1;
        }
    }
    return 0;
}
"#;
    assert_rejected_once(
        "genericconvert",
        source,
        "E10200",
        &["cannot convert 'str' to 'cstring' implicitly"],
        &[],
    );
}

/// The same conversion at an ordinary `cstring` parameter reports once.
///
/// The direct-call route defers a parameter-typed argument exactly as the
/// `ffi` route does, so the instance judgment is the same (red at the phase
/// base: the accepted program failed the strict build with `incompatible type
/// for argument 1` for the view struct).
#[test]
fn should_report_a_view_instance_at_an_ordinary_c_string_argument() {
    let source = r#"module spec.cc.genericconvertdirect;

func consume(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func convert<T>(x: T): i32 {
    return consume(x);
}

func main(): i32 {
    let s: str = "abc";
    return convert<str>(s);
}
"#;
    assert_rejected_once(
        "genericconvertdirect",
        source,
        "E10200",
        &["cannot convert 'str' to 'cstring' implicitly"],
        &[],
    );
}

/// A parameter-typed value at a concrete return reports the conversion.
///
/// The return type is concrete, so the instance can judge the substituted
/// value: a `str` instance cannot convert to `i32` and the return reports the
/// frozen conversion once (red at the phase base: the accepted program failed
/// the strict build with `incompatible types when returning type`).
#[test]
fn should_report_a_parameter_typed_value_at_a_concrete_return() {
    let source = r#"module spec.cc.genericconvertret;

func pick<T>(x: T): i32 {
    return x;
}

func main(): i32 {
    let s: str = "abc";
    return pick<str>(s);
}
"#;
    assert_rejected_once(
        "genericconvertret",
        source,
        "E10200",
        &["cannot convert 'str' to 'i32' implicitly"],
        &[],
    );
}

/// A parameter-typed value at a concrete declaration reports the conversion.
///
/// The declared type is concrete, so the instance can judge the substituted
/// value: a `str` instance cannot initialize an `i32` local and the
/// declaration reports the frozen conversion once (red at the phase base: the
/// accepted program failed the strict build with `incompatible types when
/// assigning`).
#[test]
fn should_report_a_parameter_typed_value_at_a_concrete_declaration() {
    let source = r#"module spec.cc.genericconvertdecl;

func keep<T>(x: T): i32 {
    let y: i32 = x;
    return y;
}

func main(): i32 {
    let s: str = "abc";
    return keep<str>(s);
}
"#;
    assert_rejected_once(
        "genericconvertdecl",
        source,
        "E10200",
        &["cannot convert 'str' to 'i32' implicitly"],
        &[],
    );
}

/// A null-shaped value at a parameter-typed return reports the null rule.
///
/// The return type resolves to a concrete `i32` at the instance, and a value
/// the compiler inferred from `null` has no type to convert: the frozen null
/// judgment reports once, not a raw `void *` conversion (red at the phase
/// base: the accepted program failed the strict build with `returning 'void
/// *' from a function with return type 'i32'`).
#[test]
fn should_report_a_null_shaped_source_at_a_parameter_typed_return() {
    let source = r#"module spec.cc.genericconvertnullret;

func make<T>(): T {
    let x = null;
    return x;
}

func main(): i32 {
    return i32(make<i32>()) + 1;
}
"#;
    assert_rejected_once(
        "genericconvertnullret",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &["E10200"],
    );
}

/// A null-shaped value at a parameter-typed declaration reports the null rule.
///
/// `let y: T = x` with the local inferred from `null` resolves to an `i32`
/// slot at the instance, and the frozen null judgment reports once, not a raw
/// `void *` conversion (red at the phase base: the accepted program failed
/// the strict build with `assignment to 'i32' from 'void *'`).
#[test]
fn should_report_a_null_shaped_source_at_a_parameter_typed_declaration() {
    let source = r#"module spec.cc.genericconvertnulldecl;

func store<T>(): i32 {
    let x = null;
    let y: T = x;
    return 0;
}

func main(): i32 {
    return store<i32>() + 1;
}
"#;
    assert_rejected_once(
        "genericconvertnulldecl",
        source,
        "E10012",
        &["'null' is not allowed here"],
        &["E10200"],
    );
}

/// The legally converting instances stay accepted.
///
/// The conversion duty reports only instances whose conversion is not
/// implicit: a `cstring` instance at an `ffi` `cstring` parameter, a `str`
/// instance at a `str` parameter, and a `String` instance widening to `str`
/// all build and run, and each run folds the call's result into its exit code
/// (the acceptance control for the duty's instance audit).
#[test]
fn should_keep_the_legal_conversion_instances_accepted() {
    let c_string = r#"module spec.cc.genericconvertffiok;

import { toCString } from std.text;

ffi probe {
    func Open(name: cstring, mode: i32): raw u8;
}

func convert<T>(x: T): raw u8 {
    unsafe {
        return Open(x, 0);
    }
}

func main(): i32 {
    let c: cstring = toCString("/definitely/not/here");
    unsafe {
        let f: raw u8 = convert<cstring>(c);
        if (f == null) {
            return 7;
        }
        return 1;
    }
}
"#;
    let output = run_memory("genericconvertffiok", c_string);
    assert_no_asan("genericconvertffiok", &output);
    assert_exit("genericconvertffiok", &output, 7);

    let view = r#"module spec.cc.genericconvertparam;

func span(s: str): i32 {
    return i32(len(s));
}

func sink<T>(x: T): i32 {
    return span(x);
}

func main(): i32 {
    let s: str = "abcd";
    return sink<str>(s);
}
"#;
    let output = run_memory("genericconvertparam", view);
    assert_no_asan("genericconvertparam", &output);
    assert_exit("genericconvertparam", &output, 4);

    let owned = r#"module spec.cc.genericconvertparamstr;

import { stringOf } from std.text;

func span(s: str): i32 {
    return i32(len(s));
}

func sink<T>(x: T): i32 {
    return span(x);
}

func main(): i32 {
    return sink<String>(stringOf("abc"));
}
"#;
    let output = run_memory("genericconvertparamstr", owned);
    assert_no_asan("genericconvertparamstr", &output);
    assert_exit("genericconvertparamstr", &output, 3);
}

/// The parameter-typed identity instances stay accepted.
///
/// A parameter-typed declaration and return are judged per instance, so the
/// legal `cstring`, `str`, and `String` instances of an identity round trip
/// must stay accepted: the run reads the view's and the handle's lengths and
/// the pointer's non-null state back.
#[test]
fn should_keep_the_identity_instances_accepted() {
    let source = r#"module spec.cc.genericconvertident;

import { stringOf, toCString } from std.text;

func keep<T>(take x: T): T {
    let y: T = x;
    return y;
}

func main(): i32 {
    let s: str = "abcd";
    let t: str = keep<str>(s);
    if (len(t) != 4) {
        return 1;
    }
    let u: String = keep<String>(stringOf("abc"));
    if (len(u) != 3) {
        return 2;
    }
    let c: cstring = toCString("ab");
    let d: cstring = keep<cstring>(c);
    if (d == null) {
        return 3;
    }
    return 0;
}
"#;
    let output = run_memory("genericconvertident", source);
    assert_no_asan("genericconvertident", &output);
    assert_exit("genericconvertident", &output, 0);
}

/// An `ffi` integer result at a `cstring` destination reports the conversion.
///
/// The call carries the declaration's resolved return type, so `Count()`
/// reaches a `cstring` destination as an `i32` and the frozen conversion
/// reports once instead of the C build failing (red at the phase base: the
/// accepted program failed the strict build with `passing argument 1 ... makes
/// pointer from integer`). The direct-call twin judges the same result at an
/// ordinary `cstring` parameter.
#[test]
fn should_report_an_ffi_integer_result_at_a_c_string_destination() {
    let source = r#"module spec.cc.ffiresult;

ffi probe {
    func Count(): i32;
    func Uses(s: cstring): i32;
}

func main(): i32 {
    unsafe {
        return Uses(Count());
    }
}
"#;
    assert_rejected_once(
        "ffiresult",
        source,
        "E10200",
        &["cannot convert 'i32' to 'cstring' implicitly"],
        &[],
    );

    let direct = r#"module spec.cc.ffiresultdirect;

ffi probe {
    func Count(): i32;
}

func consume(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    unsafe {
        return consume(Count());
    }
}
"#;
    assert_rejected_once(
        "ffiresultdirect",
        direct,
        "E10200",
        &["cannot convert 'i32' to 'cstring' implicitly"],
        &[],
    );
}

/// An `ffi` integer result at an integer destination builds and runs.
///
/// The result carries the declared `i32`, so an integer destination accepts
/// it and the run reads the call's value back; the handle is closed so the
/// run stays leak-free.
#[test]
fn should_keep_an_ffi_integer_result_at_an_integer_destination() {
    let source = r#"module spec.cc.ffiresultok;

ffi probe {
    func Open(name: cstring, mode: i32): raw u8;
    func Read(file: raw u8, buf: raw u8, count: i32): i32;
    func Close(file: raw u8): void;
}

func main(): i32 {
    unsafe {
        let f: raw u8 = Open("/dev/null", 0);
        if (f == null) {
            return 1;
        }
        let a: u8[4] = [0, 0, 0, 0];
        let n: i32 = Read(f, &a[0], 4);
        Close(f);
        if (n == 0) {
            return 7;
        }
        return 2;
    }
}
"#;
    let output = run_memory("ffiresultok", source);
    assert_no_asan("ffiresultok", &output);
    assert_exit("ffiresultok", &output, 7);
}

// ===== Instance-time receiver and null-shape judgments =====

/// A parameter-typed dot-call receiver is judged at its instance.
///
/// The receiver is a written argument at index zero, so its conversion is
/// deferred exactly as an argument's and the instance reports the frozen
/// conversion once, at the receiver; the out-slot receiver route defers the
/// same way. Before the judgment the accepted program passed the view struct
/// where the `char *` receiver parameter is declared and the strict build
/// failed with `incompatible type for argument 1`. The legal receiver
/// instances run: the `cstring` receiver and the `cstring?` out-slot receiver
/// (whose write-through is read back).
#[test]
fn should_report_a_generic_dot_call_receiver_at_its_instance() {
    let receiver = r#"module spec.cc.receiverinstance;

func sink(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func conv<T>(x: T): i32 {
    return x.sink();
}

func main(): i32 {
    let s: str = "abc";
    return conv<str>(s);
}
"#;
    assert_rejected_once(
        "receiverinstance",
        receiver,
        "E10200",
        &["cannot convert 'str' to 'cstring' implicitly"],
        &[],
    );

    let slot = r#"module spec.cc.receiveroutslot;

func put(borrow out: cstring?, v: cstring?): void {
    out = v;
}

func conv<T>(x: T, v: cstring?): i32 {
    let r: T = x;
    r.put(v);
    return 0;
}

func main(): i32 {
    let s: str = "abc";
    let v: cstring? = null;
    return conv<str>(s, v);
}
"#;
    assert_rejected_once(
        "receiveroutslot",
        slot,
        "E10200",
        &["cannot convert 'str' to 'cstring?' implicitly"],
        &[],
    );

    let legal = r#"module spec.cc.receiverok;

import { toCString } from std.text;

func sink(c: cstring): i32 {
    if (c == null) {
        return 1;
    }
    return 0;
}

func conv<T>(x: T): i32 {
    return x.sink();
}

func main(): i32 {
    let c: cstring = toCString("abc");
    return conv<cstring>(c);
}
"#;
    let output = run_memory("receiverok", legal);
    assert_no_asan("receiverok", &output);
    assert_exit("receiverok", &output, 0);

    let legal_slot = r#"module spec.cc.receiveroutslotok;

import { toCString } from std.text;

func put(borrow out: cstring?, v: cstring?): void {
    out = v;
}

func conv<T>(x: T, v: cstring?): i32 {
    let r: T = x;
    r.put(v);
    return 0;
}

func main(): i32 {
    let r: cstring? = toCString("xy");
    let v: cstring? = toCString("ab");
    if (conv<cstring?>(r, v) != 0) {
        return 1;
    }
    if (r == null) {
        return 2;
    }
    return 0;
}
"#;
    let output = run_memory("receiveroutslotok", legal_slot);
    assert_no_asan("receiveroutslotok", &output);
    assert_exit("receiveroutslotok", &output, 0);
}

/// A null shape at a concrete destination reports the null rule once.
///
/// A destination that is already concrete keeps the position's own rule, so
/// the instance conversion must not record the null shape again: the
/// declaration and the argument report exactly one `E10012` (without the
/// destination gate the same site reported twice, twice at the same span for
/// the argument).
#[test]
fn should_report_a_null_shape_at_a_concrete_destination_once() {
    let decl = r#"module spec.cc.nullconcretedecl;

func store<T>(x: T): i32 {
    let n = null;
    let y: i32 = n;
    return y;
}

func main(): i32 {
    let s: str = "abc";
    return store<str>(s);
}
"#;
    assert_rejected_once(
        "nullconcretedecl",
        decl,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );

    let argument = r#"module spec.cc.nullconcretearg;

func want(v: i32): i32 {
    return v;
}

func pass<T>(x: T): i32 {
    let n = null;
    return want(n);
}

func main(): i32 {
    let s: str = "abc";
    return pass<str>(s);
}
"#;
    assert_rejected_once(
        "nullconcretearg",
        argument,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );
}

/// A null shape keeps the destination position's own rule.
///
/// An element or field destination accepts the null shape exactly as its
/// concrete sibling does — both are accepted and leave the emitted conversion
/// to the host compiler, the pre-existing null-shape hole at those positions —
/// while a destination that is still a type parameter defers to its instance
/// and reports the null rule once.
#[test]
fn should_keep_a_null_shape_matching_its_destination_position() {
    for (tag, source) in [
        ("nullelement", r#"module spec.cc.nullelement;

func pass<T>(x: T): i32 {
    let n = null;
    let a: i32[2] = [n, 0];
    return a[1];
}

func main(): i32 {
    let s: str = "abc";
    return pass<str>(s);
}
"#),
        ("nullelementsibling", r#"module spec.cc.nullelementsibling;

func main(): i32 {
    let n = null;
    let a: i32[2] = [n, 0];
    return a[1];
}
"#),
        ("nullfield", r#"module spec.cc.nullfield;

struct Box { v: i32; }

func pass<T>(x: T): i32 {
    let n = null;
    let b: Box = Box { v: n };
    return b.v;
}

func main(): i32 {
    let s: str = "abc";
    return pass<str>(s);
}
"#),
        ("nullfieldsibling", r#"module spec.cc.nullfieldsibling;

struct Box { v: i32; }

func main(): i32 {
    let n = null;
    let b: Box = Box { v: n };
    return b.v;
}
"#),
    ] {
        let (_scratch, output, _c_path) = compile_only(tag, source);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{tag}: the destination position must accept the null shape; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let decl = r#"module spec.cc.nulltdecl;

func store<T>(x: T): i32 {
    let n = null;
    let y: T = n;
    return 0;
}

func main(): i32 {
    return store<i32>(5) + 1;
}
"#;
    assert_rejected_once(
        "nulltdecl",
        decl,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );

    let ret = r#"module spec.cc.nulltret;

func make<T>(): T {
    let n = null;
    return n;
}

func main(): i32 {
    return i32(make<i32>()) + 1;
}
"#;
    assert_rejected_once(
        "nulltret",
        ret,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );
}

// ===== The nullable parameter destination =====

/// A null shape at a nullable parameter destination keeps one report.
///
/// A `T?` destination is rejected where it is written (a type parameter
/// cannot be empty), so the instance judgment must not add a dependent null
/// report on the recovered substituted shape: the declaration reports exactly
/// one `E10011` and no `E10012` (before the suppression the same declaration
/// also reported the null rule at the value).
#[test]
fn should_report_a_nullable_parameter_destination_once() {
    let source = r#"module spec.cc.nullableparameter;

func store<T>(x: T): i32 {
    let y: T? = null;
    return 0;
}

func main(): i32 {
    return store<i32>(5) + 1;
}
"#;
    assert_rejected_once(
        "nullableparameter",
        source,
        "E10011",
        &["nullable type is not allowed on 'T'"],
        &["E10012"],
    );
}

// ===== The view-typed ffi result =====

/// A view-typed `ffi` result converts to a `str` destination under the rule.
///
/// A view crosses the FFI boundary as its pointer only (the ABI spells it
/// `APTR`, and the length is a separate parameter), so the result is the byte
/// pointer: storing it in a `str` and passing it to a `str` parameter both
/// report the frozen conversion once (before the pointer typing the accepted
/// programs read a struct member off the pointer and the strict build failed).
#[test]
fn should_report_a_view_typed_ffi_result_at_a_view_destination() {
    let store = r#"module spec.cc.ffiviewconvert;

ffi probe {
    func Make(): str;
}

func main(): i32 {
    let s: str = Make();
    if (s == null) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once(
        "ffiviewconvert",
        store,
        "E10200",
        &["cannot convert 'raw u8' to 'str' implicitly"],
        &[],
    );

    let argument = r#"module spec.cc.ffiviewarg;

ffi probe {
    func Make(): str;
}

func consume(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    unsafe {
        return consume(Make());
    }
}
"#;
    assert_rejected_once(
        "ffiviewarg",
        argument,
        "E10200",
        &["cannot convert 'raw u8' to 'str' implicitly"],
        &[],
    );
}

/// A view-typed `ffi` result in pointer positions builds and runs.
///
/// The `len` of the ABI pointer is not a view length: it takes the frozen
/// non-view path and folds to the empty length, exactly as the same use of a
/// `raw u8` local does. The slice twin types as its element pointer the same
/// way. Both runs read the folded length back (the exit is the length).
#[test]
fn should_keep_a_view_typed_ffi_result_in_pointer_positions() {
    let view = r#"module spec.cc.ffiviewlen;

ffi probe {
    func Make(): str;
}

func main(): i32 {
    unsafe {
        return i32(len(Make()));
    }
}
"#;
    let output = run_memory("ffiviewlen", view);
    assert_no_asan("ffiviewlen", &output);
    assert_exit("ffiviewlen", &output, 0);

    let slice = r#"module spec.cc.ffislice_len;

ffi probe {
    func Make(): u8[];
}

func main(): i32 {
    unsafe {
        return i32(len(Make()));
    }
}
"#;
    let output = run_memory("ffislice_len", slice);
    assert_no_asan("ffislice_len", &output);
    assert_exit("ffislice_len", &output, 0);
}

// ===== Null-shaped destinations and the capture slot store =====

/// True when `tool` can be spawned, so an m68k case skips on a host without
/// the pinned SDK toolchain.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// Emits one hosted-amiga program and asserts the front end accepted it.
fn emit_hosted_amiga(tag: &str, source: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    (scratch, emitted)
}

/// Links the emitted hosted-amiga C with the cross toolchain under the
/// profile's strict flags.
///
/// The toolchain lives only in the pinned SDK container, so a bare host
/// reports a skip rather than a false failure. The flags mirror the driver's
/// compile matrix line for the hosted profile. The m68k artifact cannot run
/// here, so the case ends at the link.
fn build_hosted_amiga_c(tag: &str, scratch: &ScratchDir, emitted: &str) {
    if !tool_available("m68k-amigaos-gcc") {
        println!("{tag}: skipped (m68k-amigaos-gcc is not installed)");
        return;
    }
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            "-mcrt=nix20",
            "-m68000",
            "-fwrapv",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
            "-lamiga",
            "-lm",
        ])
        .output()
        .expect("failed to spawn the hosted-amiga C compiler");
    assert!(
        built.status.success(),
        "{tag}: the hosted-amiga C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// A null shape at a nullable view destination carries the zero view.
///
/// The store and the argument destination lower the shape to the view's zero
/// value, so the store's run exits 0 on the null test and the argument's run
/// exits 0 when the parameter receives the same null view; a non-null empty
/// view would exit 1 on either path. The length read sits on the narrowed
/// branch because the frozen null rule keeps a read off the possibly-null
/// value itself (red at the phase base: the emitted store copied the shape's
/// own `void *` into the view struct and the strict build failed with
/// `incompatible types when assigning to type 'ClStr' from type 'void *'` and
/// `incompatible type for argument 1`). The pinned null comparisons, the
/// null-shaped copy, and the single-`null` declarations are the existing
/// controls.
#[test]
fn should_lower_a_null_shape_at_a_nullable_view_destination() {
    let store = r#"module spec.cc.nullshape;

func main(): i32 {
    let x = null;
    let s: str? = x;
    if (s == null) {
        return 0;
    }
    return i32(len(s)) + 1;
}
"#;
    let output = run_memory("nullshape", store);
    assert_no_asan("nullshape", &output);
    assert_exit("nullshape", &output, 0);

    let argument = r#"module spec.cc.nullshapearg;

func show(s: str?): i32 {
    if (s == null) {
        return 0;
    }
    return i32(len(s)) + 1;
}

func main(): i32 {
    let x = null;
    return show(x);
}
"#;
    let output = run_memory("nullshapearg", argument);
    assert_no_asan("nullshapearg", &output);
    assert_exit("nullshapearg", &output, 0);
}

/// A null shape at a nullable handle destination lowers on hosted-amiga.
///
/// The `File?` store and return carry the destination's own null value, so the
/// cross build links the handle instead of copying the shape's `void *` (red
/// at the phase base: the accepted program failed the hosted-amiga build with
/// `assignment to 'BPTR' {aka 'long int'} from 'void *'` and `returning 'void
/// *' from a function with return type 'BPTR'`).
#[test]
fn should_lower_a_null_shape_at_a_hosted_amiga_handle_destination() {
    let source = r#"module spec.cc.nullshapeamiga;

import { File } from std.io;

func pass(): File? {
    let x = null;
    return x;
}

func main(): i32 {
    let x = null;
    let fl: File? = x;
    if (fl != null) {
        return 1;
    }
    let f2: File? = pass();
    if (f2 != null) {
        return 2;
    }
    return 0;
}
"#;
    let (scratch, emitted) = emit_hosted_amiga("nullshapeamiga", source);
    build_hosted_amiga_c("nullshapeamiga", &scratch, &emitted);
}

/// A writable capture stored into a nullable owning out-slot reports once.
///
/// The out-slot store would move out of the capture environment while the
/// caller still owns the value, so the store judges the capture move and the
/// frozen capture rule reports one `E10401` at the store (red at the phase
/// base: the accepted program aborted with `free(): double free detected` at
/// exit 134 and AddressSanitizer reported `attempting double-free`). The
/// read-only capture's rejection stays pinned by
/// `should_reject_a_capture_stored_into_an_out_slot`.
#[test]
fn should_reject_a_writable_capture_stored_into_a_nullable_out_slot() {
    let source = r#"module spec.cc.captureslot;

import { String, stringOf } from std.text;

func wrap<T>(borrow out: T, x: T): void {
    ([borrow x, borrow out] () => {
        out = x;
    })();
}

func main(): i32 {
    let out: String? = null;
    let s: String = stringOf("ab");
    wrap<String?>(borrow out, s);
    return 0;
}
"#;
    assert_rejected_once(
        "captureslot",
        source,
        "E10401",
        &["cannot move captured 'x' in a lambda"],
        &["E10034"],
    );
}

/// The writable non-capture out-slot store stays accepted.
///
/// A `take` argument moves into the slot, so the store writes through without
/// a capture source and the caller reads the slot back: the run measures the
/// stored string and exits 3 (the acceptance control for the store judgment).
#[test]
fn should_keep_the_non_capture_nullable_out_slot_store_accepted() {
    let source = r#"module spec.cc.captureslotctl;

import { String, stringOf } from std.text;

func put<T>(borrow d: T, take s: T): void {
    d = s;
}

func main(): i32 {
    let d: String? = null;
    let s: String = stringOf("ab");
    put<String?>(borrow d, s);
    if (d == null) {
        return 1;
    }
    return i32(len(d)) + 1;
}
"#;
    let output = run_memory("captureslotctl", source);
    assert_no_asan("captureslotctl", &output);
    assert_exit("captureslotctl", &output, 3);
}

// ===== The null-shaped conditional arm and the capture view store =====

/// A null-shaped conditional arm lowers at an owning destination.
///
/// A conditional at a non-duplicable destination stores per arm, so the
/// arm's null shape takes the destination's own empty value and the run reads
/// the null back (before the per-arm destination typing the accepted programs
/// emitted the shape's `void *` into the handle and failed the strict build).
#[test]
fn should_lower_a_null_shaped_conditional_arm_at_an_owning_destination() {
    let handle = r#"module spec.cc.nullcondfile;

import { File } from std.io;

func main(): i32 {
    let c: bool = true;
    let x = null;
    let g: File? = c ? x : null;
    if (g == null) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("nullcondfile", handle);
    assert_no_asan("nullcondfile", &output);
    assert_exit("nullcondfile", &output, 0);

    let owned = r#"module spec.cc.nullcondarm;

import { String, stringOf } from std.text;

func main(): i32 {
    let c: bool = true;
    let x = null;
    let g: String? = c ? x : null;
    if (g == null) {
        return 0;
    }
    return i32(len(g)) + 1;
}
"#;
    let output = run_memory("nullcondarm", owned);
    assert_no_asan("nullcondarm", &output);
    assert_exit("nullcondarm", &output, 0);
}

/// A capture's view-widening store is not a move and stays accepted.
///
/// The store copies the borrowed capture into the view, so the instance
/// judgment leaves it accepted; the run measures the stored view and the
/// caller's own string (before the store's duty judged every assignment, the
/// accepted program was rejected with the capture-move rule).
#[test]
fn should_keep_a_capture_view_widening_store_accepted() {
    let source = r#"module spec.cc.captureviewstore;

import { String, stringOf } from std.text;

func make<T>(x: T): i32 {
    return ([borrow x] () => {
        let y: str = "q";
        y = x;
        return i32(len(y));
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    return make<String>(s) + i32(len(s)) + 1;
}
"#;
    let output = run_memory("captureviewstore", source);
    assert_no_asan("captureviewstore", &output);
    assert_exit("captureviewstore", &output, 5);
}

// ===== The computed null shape and the non-out-slot store =====

/// A computed null-shaped value runs its call before the destination's null.
///
/// A lambda whose inferred return is the null shape still computes: the call
/// executes (its side effect reaches the module counter) and the destination
/// takes its own empty value, so the run reads the counter back (before the
/// typed null replaced the call, the body never ran and the exit was 255; the
/// strict build failed on the shape's pointer at the view destination).
#[test]
fn should_run_a_computed_null_shape_before_the_destination_value() {
    let stored = r#"module spec.cc.nullcomputedstore;

let hits: i32 = 0;

func main(): i32 {
    let x = null;
    let s: str? = ([x] () => {
        hits = hits + 1;
        return x;
    })();
    if (s == null) {
        return hits - 1;
    }
    return 9;
}
"#;
    let output = run_memory("nullcomputedstore", stored);
    assert_no_asan("nullcomputedstore", &output);
    assert_exit("nullcomputedstore", &output, 0);

    let argument = r#"module spec.cc.nullcomputedarg;

let hits: i32 = 0;

func show(s: str?): i32 {
    if (s == null) {
        return 0;
    }
    return 1;
}

func main(): i32 {
    let x = null;
    let r: i32 = show(([x] () => {
        hits = hits + 1;
        return x;
    })());
    return hits - 1 + r;
}
"#;
    let output = run_memory("nullcomputedarg", argument);
    assert_no_asan("nullcomputedarg", &output);
    assert_exit("nullcomputedarg", &output, 0);
}

/// A store at a non-out-slot destination keeps the single lend report.
///
/// Only a writable lend destination carries the capture-store judgment, so a
/// local or member target keeps the frozen lend-store report and the capture
/// rule stays absent (before the gate the generic spelling added the capture
/// report on top).
#[test]
fn should_keep_a_single_lend_report_at_a_non_out_slot_store() {
    let local = r#"module spec.cc.capturelocalstore;

import { String, stringOf } from std.text;

func wrap<T>(x: T): void {
    ([borrow x] () => {
        let y: String? = null;
        y = x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    wrap<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "capturelocalstore",
        local,
        "E10034",
        &["cannot release or store borrowed parameter 'x'"],
        &["E10401"],
    );

    let member = r#"module spec.cc.capturememberstore;

import { String, stringOf } from std.text;

struct Holder { y: String?; }

func wrap<T>(x: T): void {
    ([borrow x] () => {
        let h: Holder = Holder { y: null };
        h.y = x;
    })();
}

func main(): i32 {
    let s: String = stringOf("ab");
    wrap<String>(s);
    return 0;
}
"#;
    assert_rejected_once(
        "capturememberstore",
        member,
        "E10034",
        &["cannot release or store borrowed parameter 'x'"],
        &["E10401"],
    );
}

// ===== The pointer-destination conditional =====

/// A pointer-destination conditional takes the destination's own null.
///
/// A `cstring?` or `raw` result conditional stores the destination's own null
/// per arm, from a place arm and from the literal arms; on the profiles whose
/// null literal is the integer constant the pointer cast keeps both arms a
/// pointer, so the cross build accepts what a bare `0` pair would fail with
/// `-Werror=int-conversion`.
#[test]
fn should_lower_a_pointer_destination_conditional_per_arm() {
    let place = r#"module spec.cc.condpointerplace;

func main(): i32 {
    let c: bool = true;
    let x = null;
    let cs: cstring? = c ? x : null;
    let p: raw u8? = c ? x : null;
    if (cs == null && p == null) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("condpointerplace", place);
    assert_no_asan("condpointerplace", &output);
    assert_exit("condpointerplace", &output, 0);

    let literal = r#"module spec.cc.condpointerlit;

func main(): i32 {
    let c: bool = true;
    let cs: cstring? = c ? null : null;
    let p: raw u8? = c ? null : null;
    if (cs == null && p == null) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("condpointerlit", literal);
    assert_no_asan("condpointerlit", &output);
    assert_exit("condpointerlit", &output, 0);

    let (scratch, emitted) = emit_hosted_amiga("condpointerplace", place);
    build_hosted_amiga_c("condpointerplace", &scratch, &emitted);
}

// ===== The instance-key walk's projected-argument resolver =====

/// Compiles a two-file module program for the host profile.
///
/// The holder file declares the module the program imports, so the imported
/// binding resolves and the ordinary checks run on it. Returns the scratch
/// directory, the compiler output, and the emitted-C path.
fn compile_pair(tag: &str, holder: &str, main: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// Emits a two-file module program and asserts the front end accepted it.
fn emit_pair(tag: &str, holder: &str, main: &str) -> (ScratchDir, String) {
    let (scratch, output, c_path) = compile_pair(tag, holder, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    (scratch, emitted)
}

/// Emits, builds, and runs one two-file host program, returning its output.
fn run_host_pair(tag: &str, holder: &str, main: &str) -> Output {
    let (scratch, emitted) = emit_pair(tag, holder, main);
    let program = build_host_c(tag, &scratch, &emitted);
    run_with_timeout(tag, &program, &[])
}

/// Emits a two-file program, builds it with AddressSanitizer, and runs it
/// with leak detection on, returning its output.
fn run_asan_pair(tag: &str, holder: &str, main: &str) -> Output {
    let (scratch, emitted) = emit_pair(tag, holder, main);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-fsanitize=address",
            "-no-pie",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the sanitizer build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(
        tag,
        &program,
        &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")],
    )
}

/// Runs a two-file memory-safety case: under AddressSanitizer when `RDC_ASAN`
/// is set, otherwise as an ordinary strict host build.
fn run_memory_pair(tag: &str, holder: &str, main: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan_pair(tag, holder, main)
    } else {
        run_host_pair(tag, holder, main)
    }
}

/// An aliased module receiver of an initializer dot call keys its instance.
///
/// `bb.pick(7)` passes the imported binding — the holder's `b: Box` — as the
/// call's argument position zero, so the inferred instance key must take the
/// receiver's type: the binding fixes `T` to `Box`, the emitted call reaches
/// the keyed instance, and the run reads the argument back (exit 0). The same
/// call in a body and the unaliased import are the unchanged controls (red at
/// the phase base: the initializer's instance walk resolved the receiver's
/// head against the program's module-level names, found no global named `bb`,
/// and recorded no instance, so the strict build failed on an implicit
/// declaration of `cl_pick__Box`).
#[test]
fn should_key_an_aliased_receiver_dot_call_in_an_initializer() {
    let holder = r#"module spec.cc.walkholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

    let aliased = r#"module spec.cc.walkdotreceiver;

import { b as bb } from spec.cc.walkholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let g: i32 = bb.pick(7);

func main(): i32 {
    return g - 7;
}
"#;
    let output = run_memory_pair("walkdotreceiver", holder, aliased);
    assert_no_asan("walkdotreceiver", &output);
    assert_exit("walkdotreceiver", &output, 0);

    let body = r#"module spec.cc.walkdotreceiverbody;

import { b as bb } from spec.cc.walkholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

func body(): i32 {
    return bb.pick(7);
}

let g: i32 = body();

func main(): i32 {
    return g - 7;
}
"#;
    let output = run_memory_pair("walkdotreceiverbody", holder, body);
    assert_no_asan("walkdotreceiverbody", &output);
    assert_exit("walkdotreceiverbody", &output, 0);

    let direct = r#"module spec.cc.walkdotreceiverdirect;

import { b } from spec.cc.walkholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let g: i32 = b.pick(7);

func main(): i32 {
    return g - 7;
}
"#;
    let output = run_memory_pair("walkdotreceiverdirect", holder, direct);
    assert_no_asan("walkdotreceiverdirect", &output);
    assert_exit("walkdotreceiverdirect", &output, 0);
}

/// A temp-rooted member argument in a borrow loop keys its instance.
///
/// `keep(v.name)` inside `each<T>`'s borrow loop passes the borrowed
/// element's owning `String` member, whose type the generic body does not
/// know until the instance is substituted: the keyed instance runs once per
/// element and the run reads the loop's sum and the call count back (exit 0).
/// `id(v[i].y)` in a by-value loop is the recorded control (exit 7) and the
/// concrete-parameter twin of the same source shape stays keyed (exit 2) (red
/// at the phase base: the instance walk could not resolve the borrowed
/// element's member and recorded no instance, so the strict build failed on
/// an implicit declaration of `cl_keep__String`).
#[test]
fn should_key_a_temp_rooted_member_argument_in_a_borrow_loop() {
    let temp_rooted = r#"module spec.cc.walktemprooted;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

let hits: i32 = 0;

func keep<T>(x: T): i32 {
    hits = hits + 1;
    return 1;
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + keep(v.name);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cd"), y: 2 }];
    return (each<S[2]>(a) - 2) + (hits - 2);
}
"#;
    let output = run_memory("walktemprooted", temp_rooted);
    assert_no_asan("walktemprooted", &output);
    assert_exit("walktemprooted", &output, 0);

    let control = r#"module spec.cc.walktempref;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func rowsum<T>(xs: T, i: usize): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + id(v[i].y);
    }
    return n;
}

func main(): i32 {
    let rows: S[2][3] = [
        [S { y: 1 }, S { y: 2 }, S { y: 3 }],
        [S { y: 4 }, S { y: 5 }, S { y: 6 }]
    ];
    let i: usize = 1;
    return rowsum<S[2][3]>(rows, i);
}
"#;
    let output = run_memory("walktempref", control);
    assert_no_asan("walktempref", &output);
    assert_exit("walktempref", &output, 7);

    let concrete = r#"module spec.cc.walktempconcrete;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func keep<T>(x: T): i32 {
    return 1;
}

func eachS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + keep(v.name);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cd"), y: 2 }];
    return eachS(a);
}
"#;
    let output = run_memory("walktempconcrete", concrete);
    assert_no_asan("walktempconcrete", &output);
    assert_exit("walktempconcrete", &output, 2);
}

/// A conditional argument in a generic instance keys its selected arm.
///
/// `id(c ? a.y : b.y)` reads both arms through the instance substitution, so
/// the conditional resolves only when the arms agree on one concrete type;
/// the keyed instance runs and the run folds both selections to the exit (0).
/// The nested double-placeholder loop (exit 21), the nested call result
/// (exit 5), the field chain through a `let` local (exit 3), and the concrete
/// parameter-rooted field chain (exit 1) are the controls (red at the phase
/// base: the instance walk declined the conditional's parameter-rooted arms
/// and recorded no instance, so the strict build failed on an implicit
/// declaration of `cl_id__i32`).
#[test]
fn should_key_a_conditional_argument_in_a_generic_instance() {
    let conditional = r#"module spec.cc.walkconditional;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(a: T, b: T, c: bool): i32 {
    return id(c ? a.y : b.y);
}

func main(): i32 {
    let a: S = S { y: 1 };
    let b: S = S { y: 2 };
    return (pick<S>(a, b, true) - 1) + (pick<S>(a, b, false) - 2);
}
"#;
    let output = run_memory("walkconditional", conditional);
    assert_no_asan("walkconditional", &output);
    assert_exit("walkconditional", &output, 0);

    let nested_loop = r#"module spec.cc.walknestedloop;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func rowsum<T>(xs: T): i32 {
    let n: i32 = 0;
    for (row in xs) {
        for (c in row) {
            n = n + id(c.y);
        }
    }
    return n;
}

func main(): i32 {
    let rows: S[2][3] = [
        [S { y: 1 }, S { y: 2 }, S { y: 3 }],
        [S { y: 4 }, S { y: 5 }, S { y: 6 }]
    ];
    return rowsum<S[2][3]>(rows);
}
"#;
    let output = run_memory("walknestedloop", nested_loop);
    assert_no_asan("walknestedloop", &output);
    assert_exit("walknestedloop", &output, 21);

    let call_result = r#"module spec.cc.walkcallresult;

struct S { y: i32; }

func mk(x: i32): i32 {
    return x + 1;
}

func id<T>(x: T): T { return x; }

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + id(mk(v.y));
    }
    return n;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;
    let output = run_memory("walkcallresult", call_result);
    assert_no_asan("walkcallresult", &output);
    assert_exit("walkcallresult", &output, 5);

    let local_place = r#"module spec.cc.walklocalplace;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow e in xs) {
        let s: S = e;
        total = total + id(s.y);
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return each<S[2]>(a);
}
"#;
    let output = run_memory("walklocalplace", local_place);
    assert_no_asan("walklocalplace", &output);
    assert_exit("walklocalplace", &output, 3);

    let projected = r#"module spec.cc.walkprojected;

struct S { y: i32; }
struct O { s: S; }

func id<T>(x: T): T { return x; }

func pick(o: O): i32 {
    return id(o.s.y);
}

func main(): i32 {
    let o: O = O { s: S { y: 1 } };
    return pick(o);
}
"#;
    let output = run_memory("walkprojected", projected);
    assert_no_asan("walkprojected", &output);
    assert_exit("walkprojected", &output, 1);
}

// ===== The receiver head's scope =====

/// A module initializer's dot-call receiver keys the module's own binding.
///
/// With distinct names — the import renamed to `other` and the module's own
/// `g` — the initializer's `g.pick(7)` keys the own `i32` instance and the run
/// reads the value back (exit 0). The member-dependent sibling resolves the
/// same way through the instance check: `g.drain()` iterates the own array and
/// the run counts the two elements (exit 0). Binding one name twice is the
/// declaration/import collision, which the collision case above pins as
/// `E11103`.
#[test]
fn should_key_an_own_binding_at_an_initializer_dot_receiver() {
    let holder = r#"module spec.cc.aliasreceiverholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

    let initializer = r#"module spec.cc.aliascollision;

import { b as other } from spec.cc.aliasreceiverholder;

func pick<T>(take v: T, w: i32): i32 {
    return w;
}

let g: i32 = 3;

let h: i32 = g.pick(7) + other.n;

func main(): i32 {
    return (h - 12) + (g - 3);
}
"#;
    let output = run_memory_pair("aliascollision", holder, initializer);
    assert_no_asan("aliascollision", &output);
    assert_exit("aliascollision", &output, 0);

    let member = r#"module spec.cc.aliascollisionmember;

import { b as other } from spec.cc.aliasreceiverholder;

func drain<T>(take xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

let g: i32[2] = [1, 2];

let h: i32 = g.drain() + other.n;

func main(): i32 {
    return h - 7;
}
"#;
    let output = run_memory_pair("aliascollisionmember", holder, member);
    assert_no_asan("aliascollisionmember", &output);
    assert_exit("aliascollisionmember", &output, 0);
}

/// A body binding that shadows an import alias keeps its own type at a dot
/// call.
///
/// A local `bb: i32[2]` and the parameter twin shadow the imported `bb`, so
/// `bb.drain()` resolves the body's own binding and the per-instance walk
/// keys the array instance: both runs count their two elements (exit 0).
/// Resolving the head through the module scope inside a body instead rejected
/// the generic body with an iteration error over the import's struct type.
#[test]
fn should_key_a_shadowed_alias_receiver_in_a_body() {
    let holder = r#"module spec.cc.aliasreceiverholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

    let local = r#"module spec.cc.shadowedalias;

import { b as bb } from spec.cc.aliasreceiverholder;

func drain<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func run(): i32 {
    let bb: i32[2] = [3, 4];
    return bb.drain();
}

func main(): i32 {
    return run() - 2;
}
"#;
    let output = run_memory_pair("shadowedalias", holder, local);
    assert_no_asan("shadowedalias", &output);
    assert_exit("shadowedalias", &output, 0);

    let parameter = r#"module spec.cc.shadowedparam;

import { b as bb } from spec.cc.aliasreceiverholder;

func drain<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func run(bb: i32[2]): i32 {
    return bb.drain();
}

func main(): i32 {
    let a: i32[2] = [3, 4];
    return run(a) - 2;
}
"#;
    let output = run_memory_pair("shadowedparam", holder, parameter);
    assert_no_asan("shadowedparam", &output);
    assert_exit("shadowedparam", &output, 0);
}

// ===== The lambda body's own scope =====

/// An expression-bodied initializer lambda resolves its own parameter, not an
/// import the parameter shadows.
///
/// `(bb: i32[2]) => bb.drain()` binds `bb` to the lambda's parameter, so the
/// body iterates the argument and the run counts its two elements (exit 0).
/// The renamed parameter is the control: nothing is shadowed there, and the
/// same shape keys the same instance. Reading the parameter's name as the
/// imported binding instead rejected the generic body with an iteration error
/// over the import's struct type.
#[test]
fn should_key_an_expression_lambda_parameter_that_shadows_an_import() {
    let holder = r#"module spec.cc.aliasreceiverholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

    let shadowed = r#"module spec.cc.exprlambdashadow;

import { b as bb } from spec.cc.aliasreceiverholder;

func drain<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

let f: func(i32[2]): i32 = (bb: i32[2]) => bb.drain();

func main(): i32 {
    let a: i32[2] = [3, 4];
    return f(a) - 2;
}
"#;
    let output = run_memory_pair("exprlambdashadow", holder, shadowed);
    assert_no_asan("exprlambdashadow", &output);
    assert_exit("exprlambdashadow", &output, 0);

    let control = r#"module spec.cc.exprlambdactl;

import { b as bb } from spec.cc.aliasreceiverholder;

func drain<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

let f: func(i32[2]): i32 = (q: i32[2]) => q.drain();

func main(): i32 {
    let a: i32[2] = [3, 4];
    return f(a) - 2;
}
"#;
    let output = run_memory_pair("exprlambdactl", holder, control);
    assert_no_asan("exprlambdactl", &output);
    assert_exit("exprlambdactl", &output, 0);
}

// ===== The instance surface's element stores, conversions, and out-slots =====

/// A store to an owned field through a borrowed generic element releases the
/// overwritten owner on every path.
///
/// `v.name = stringOf("zzzz")` inside `each<T>`'s borrow loop overwrites the
/// element's owning `String`, so the old owner must release once per store;
/// the run reads each stored size back (exit 0) and the leak checker is
/// clean. The early-exit spelling stores and returns from inside the loop, so
/// the same release must cover that exit path. The concrete twin is the
/// control: its store already releases and the run is clean. (red at the
/// phase base: the generic spellings leaked one overwritten owner per store
/// under LeakSanitizer; the runs otherwise read their sizes back.)
#[test]
fn should_release_the_overwritten_owner_of_a_borrowed_element_store() {
    let looping = r#"module spec.cc.elementstoredrop;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = stringOf("zzzz");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return each<S[2]>(a) - 8;
}
"#;
    let output = run_memory("elementstoredrop", looping);
    assert_no_asan("elementstoredrop", &output);
    assert_exit("elementstoredrop", &output, 0);

    let early_exit = r#"module spec.cc.elementstoredropexit;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func first<T>(xs: T): i32 {
    for (borrow v in xs) {
        v.name = stringOf("zzzz");
        return i32(len(bytes(v.name)));
    }
    return 0;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return first<S[2]>(a) - 4;
}
"#;
    let output = run_memory("elementstoredropexit", early_exit);
    assert_no_asan("elementstoredropexit", &output);
    assert_exit("elementstoredropexit", &output, 0);

    let concrete = r#"module spec.cc.elementstoredropctl;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func eachS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = stringOf("zzzz");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return eachS(a) - 8;
}
"#;
    let output = run_memory("elementstoredropctl", concrete);
    assert_no_asan("elementstoredropctl", &output);
    assert_exit("elementstoredropctl", &output, 0);
}

/// A substituted element at a concrete conversion position reports the
/// conversion exactly as its concrete twin does.
///
/// The seven element spellings — a place, conditional, and direct element
/// return, an element `let`, an element argument, and the loop element
/// return and `let` — each substitute an `S` element into a concrete `i32`
/// destination, so each instance must report the frozen `E10200` exactly
/// once and emit no translation unit, matching the same program with the
/// array type carried concretely. (red at the phase base: all seven were
/// accepted and the strict build failed with the C type mismatch the
/// instance judgment must report.)
#[test]
fn should_report_a_converted_element_at_each_concrete_position() {
    let place_return = r#"module spec.cc.elemconvplaceret;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, i: usize): i32 {
    return id(xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvplaceret",
        place_return,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let place_return_concrete = r#"module spec.cc.elemconvplaceretctl;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick(xs: S[3], i: usize): i32 {
    return id(xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return pick(a, 1);
}
"#;
    assert_rejected_once(
        "elemconvplaceretctl",
        place_return_concrete,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let conditional_return = r#"module spec.cc.elemconvcondret;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, i: usize, c: bool): i32 {
    return id(c ? xs[i] : xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i, true);
}
"#;
    assert_rejected_once(
        "elemconvcondret",
        conditional_return,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let conditional_return_concrete = r#"module spec.cc.elemconvcondretctl;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick(xs: S[3], i: usize, c: bool): i32 {
    return id(c ? xs[i] : xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return pick(a, 1, true);
}
"#;
    assert_rejected_once(
        "elemconvcondretctl",
        conditional_return_concrete,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let direct_return = r#"module spec.cc.elemconvdirectret;

struct S { y: i32; }

func pick<T>(xs: T, i: usize): i32 {
    return xs[i];
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvdirectret",
        direct_return,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let direct_return_concrete = r#"module spec.cc.elemconvdirectretctl;

struct S { y: i32; }

func pick(xs: S[3], i: usize): i32 {
    return xs[i];
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return pick(a, 1);
}
"#;
    assert_rejected_once(
        "elemconvdirectretctl",
        direct_return_concrete,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let element_let = r#"module spec.cc.elemconvlet;

struct S { y: i32; }

func pick<T>(xs: T, i: usize): i32 {
    let v: i32 = xs[i];
    return v;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvlet",
        element_let,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let element_let_concrete = r#"module spec.cc.elemconvletctl;

struct S { y: i32; }

func pick(xs: S[3], i: usize): i32 {
    let v: i32 = xs[i];
    return v;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return pick(a, 1);
}
"#;
    assert_rejected_once(
        "elemconvletctl",
        element_let_concrete,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let element_argument = r#"module spec.cc.elemconvarg;

struct S { y: i32; }

func use(x: i32): i32 { return x; }

func pick<T>(xs: T, i: usize): i32 {
    return use(xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick<S[3]>(a, i);
}
"#;
    assert_rejected_once(
        "elemconvarg",
        element_argument,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let element_argument_concrete = r#"module spec.cc.elemconvargctl;

struct S { y: i32; }

func use(x: i32): i32 { return x; }

func pick(xs: S[3], i: usize): i32 {
    return use(xs[i]);
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return pick(a, 1);
}
"#;
    assert_rejected_once(
        "elemconvargctl",
        element_argument_concrete,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let loop_return = r#"module spec.cc.elemconvloopret;

struct S { y: i32; }

func first<T>(xs: T): i32 {
    for (v in xs) {
        return v;
    }
    return 0;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return first<S[3]>(a);
}
"#;
    assert_rejected_once(
        "elemconvloopret",
        loop_return,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let loop_return_concrete = r#"module spec.cc.elemconvloopretctl;

struct S { y: i32; }

func first(xs: S[3]): i32 {
    for (v in xs) {
        return v;
    }
    return 0;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return first(a);
}
"#;
    assert_rejected_once(
        "elemconvloopretctl",
        loop_return_concrete,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let loop_let = r#"module spec.cc.elemconvlooplet;

struct S { y: i32; }

func first<T>(xs: T): i32 {
    for (v in xs) {
        let w: i32 = v;
        return w;
    }
    return 0;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return first<S[3]>(a);
}
"#;
    assert_rejected_once(
        "elemconvlooplet",
        loop_let,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );

    let loop_let_concrete = r#"module spec.cc.elemconvloopletctl;

struct S { y: i32; }

func first(xs: S[3]): i32 {
    for (v in xs) {
        let w: i32 = v;
        return w;
    }
    return 0;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    return first(a);
}
"#;
    assert_rejected_once(
        "elemconvloopletctl",
        loop_let_concrete,
        "E10200",
        &["cannot convert 'S' to 'i32' implicitly"],
        &[],
    );
}

/// The matching-type element positions stay accepted and read their values
/// back.
///
/// The loop element return and `let` over an `i32` slice and the conditional
/// element member positions are the accepted twins of the conversion
/// spellings: each instance builds strictly and folds its element value into
/// the exit code (exit 7, 24, 2, and 1). (green controls at the phase base.)
#[test]
fn should_keep_the_matching_element_positions_accepted() {
    let element_return = r#"module spec.cc.elementconvret;

import { slice } from std.mem;

func first<T>(xs: T[]): i32 {
    for (borrow e in xs) {
        return e;
    }
    return 0;
}

func main(): i32 {
    let a: i32[3] = [7, 8, 9];
    let s: i32[] = slice(a, 0, 3);
    return first<i32>(s);
}
"#;
    let output = run_memory("elementconvret", element_return);
    assert_no_asan("elementconvret", &output);
    assert_exit("elementconvret", &output, 7);

    let element_let = r#"module spec.cc.elementconvlet;

import { slice } from std.mem;

func sum<T>(xs: T[]): i32 {
    let total: i32 = 0;
    for (borrow e in xs) {
        let y: i32 = e;
        total = total + y;
    }
    return total;
}

func main(): i32 {
    let a: i32[3] = [7, 8, 9];
    let s: i32[] = slice(a, 0, 3);
    return sum<i32>(s);
}
"#;
    let output = run_memory("elementconvlet", element_let);
    assert_no_asan("elementconvlet", &output);
    assert_exit("elementconvlet", &output, 24);

    let conditional_member = r#"module spec.cc.elementconvcond;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick(xs: S[3], i: usize, c: bool): i32 {
    return id(c ? xs[i] : xs[i]).y;
}

func main(): i32 {
    let a: S[3] = [S { y: 1 }, S { y: 2 }, S { y: 3 }];
    let i: usize = 1;
    return pick(a, i, true);
}
"#;
    let output = run_memory("elementconvcond", conditional_member);
    assert_no_asan("elementconvcond", &output);
    assert_exit("elementconvcond", &output, 2);

    let conditional_argument = r#"module spec.cc.elementconvcondarg;

struct S { y: i32; }

func id<T>(x: T): T { return x; }

func pick<T>(xs: T, c: bool): i32 {
    return id(c ? xs[0].y : xs[0].y);
}

func main(): i32 {
    let a: S[2] = [S { y: 1 }, S { y: 2 }];
    return pick<S[2]>(a, true);
}
"#;
    let output = run_memory("elementconvcondarg", conditional_argument);
    assert_no_asan("elementconvcondarg", &output);
    assert_exit("elementconvcondarg", &output, 1);
}

/// A writable lend of a duplicable type writes the callee's copy only.
///
/// The deliberate by-value deviation for a duplicable lend binds the
/// `borrow` parameter to a copy, so a write through it affects that copy
/// while the caller's slot is unchanged, and the compiler warns the lend as
/// unnecessary (`W10001`), which is the documented behavior. The copy
/// read-back and the caller read-back fold both values: the callee's `wrap`
/// returns the copy it stored (7) and the caller's slot keeps its
/// initialized 0, so the direct caller read-back folds 249 (the unchanged
/// zero minus 7). The `String?` in-out slot is the control that does write
/// through: `put<String?>` stores the handle and the caller reads the stored
/// length back (exit 0). (green control at the phase base: the emitter
/// already writes the parameter copy and warns the lend; the spec's own
/// `bump` example keeps the caller's value.)
#[test]
fn should_keep_a_duplicable_lend_write_on_the_callee_copy() {
    let copy_readback = r#"module spec.cc.duplicableoutslotcopy;

func wrap<T>(borrow out: T, x: T): T {
    ([borrow x, borrow out] () => {
        out = x;
    })();
    return out;
}

func main(): i32 {
    let out: i32 = 0;
    let copy: i32 = wrap<i32>(borrow out, 7);
    return (out - 0) + (copy - 7);
}
"#;
    let output = run_memory("duplicableoutslotcopy", copy_readback);
    assert_no_asan("duplicableoutslotcopy", &output);
    assert_exit("duplicableoutslotcopy", &output, 0);

    let caller_readback = r#"module spec.cc.duplicableoutslot;

func wrap<T>(borrow out: T, x: T): void {
    ([borrow x, borrow out] () => {
        out = x;
    })();
}

func main(): i32 {
    let out: i32 = 0;
    wrap<i32>(borrow out, 7);
    return out - 7;
}
"#;
    let output = run_memory("duplicableoutslot", caller_readback);
    assert_no_asan("duplicableoutslot", &output);
    assert_exit("duplicableoutslot", &output, 249);

    let nullable = r#"module spec.cc.duplicableoutslotctl;

import { String, stringOf, bytes } from std.text;

func put<T>(borrow d: T, take s: T): void {
    d = s;
}

func main(): i32 {
    let d: String? = null;
    let s: String = stringOf("ab");
    put<String?>(borrow d, s);
    if (d == null) {
        return 1;
    }
    return i32(len(bytes(d))) - 2;
}
"#;
    let output = run_memory("duplicableoutslotctl", nullable);
    assert_no_asan("duplicableoutslotctl", &output);
    assert_exit("duplicableoutslotctl", &output, 0);
}

// ===== The element place's moves and stores =====

/// A named owner stored into a borrowed element moves, and the next store
/// releases what it replaces.
///
/// `v.name = s` inside `each<T>`'s borrow loop stores the `String` `s` into
/// the element, so the source empties exactly as the concrete twin empties
/// it, and the following store releases the owner it overwrites. The run
/// folds both stored sizes back (exit 0) and the leak checker is clean. The
/// concrete twin is the control. (The overwrite release used to leak one
/// stored owner per element, and without the matching source emptying the
/// release that followed freed the moved value twice.)
#[test]
fn should_move_a_named_owner_into_a_borrowed_element_and_release_the_replaced_value() {
    let looping = r#"module spec.cc.elementmovestore;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func eachM<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        let s: String = stringOf("qq");
        v.name = s;
        total = total + i32(len(bytes(v.name)));
        v.name = stringOf("r");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return eachM<S[2]>(a) - 6;
}
"#;
    let output = run_memory("elementmovestore", looping);
    assert_no_asan("elementmovestore", &output);
    assert_exit("elementmovestore", &output, 0);

    let concrete = r#"module spec.cc.elementmovestorectl;

import { String, stringOf, bytes } from std.text;

struct S { name: String; y: i32; }

func eachMS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        let s: String = stringOf("qq");
        v.name = s;
        total = total + i32(len(bytes(v.name)));
        v.name = stringOf("r");
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("a"), y: 1 }, S { name: stringOf("bb"), y: 2 }];
    return eachMS(a) - 6;
}
"#;
    let output = run_memory("elementmovestorectl", concrete);
    assert_no_asan("elementmovestorectl", &output);
    assert_exit("elementmovestorectl", &output, 0);
}

/// A take argument of a borrowed element reports the partial-move limit.
///
/// `eat(v.name)` consumes a member of a borrowed element: the element view
/// never owns its element and the member is not a top-level field of an owned
/// local or `take` parameter, so the move is the v1 element-move limit and the
/// generic instance reports the same `E10026` its concrete twin reports — no
/// translation unit is written, so the value the callee would own is never
/// aliased with the element. The top-level field spelling is the accepted
/// control: it moves the member to the callee and empties the field, so the
/// following store writes a fresh value and the run is leak-clean.
#[test]
fn should_report_a_borrowed_element_take_as_the_partial_move_limit() {
    let looping = r#"module spec.cc.elementtakestore;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func each<T>(xs: T): i32 {
    for (borrow v in xs) {
        eat(v.name);
        v.name = stringOf("zzzz");
        return i32(len(bytes(v.name)));
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    return each<S[1]>(a) - 4;
}
"#;
    assert_rejected_once(
        "elementtakestore",
        looping,
        "E10026",
        &["partial moves are limited to top-level fields in v1"],
        &[],
    );

    let top_level = r#"module spec.cc.elementtakestorectl;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func main(): i32 {
    let a: S = S { name: stringOf("a") };
    eat(a.name);
    a.name = stringOf("zzzz");
    return i32(len(bytes(a.name))) - 4;
}
"#;
    let output = run_memory("elementtakestorectl", top_level);
    assert_no_asan("elementtakestorectl", &output);
    assert_exit("elementtakestorectl", &output, 0);
}

// ===== The element store's aliasing right-hand side =====

/// A store to a borrowed element survives a right-hand side that reads the
/// same element through its container.
///
/// `v.name = dup(borrow xs[0].name)` reads the very storage the target names,
/// through the array root, so the release of the replaced owner must wait
/// until the right-hand side has been evaluated: both the generic spelling
/// and the concrete twin save the replaced value first, store the new one,
/// and release the saved value. Each run folds the cloned size back (exit 0)
/// and the leak checker is clean. (The generic spelling used to release the
/// target before the right-hand side read it and faulted on the emptied
/// storage; the concrete spelling faulted the same way.)
#[test]
fn should_keep_an_aliasing_right_hand_side_alive_across_the_element_store() {
    let looping = r#"module spec.cc.elementstorealias;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func each<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = dup(borrow xs[0].name);
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab"), y: 1 }];
    return each<S[1]>(a) - 2;
}
"#;
    let output = run_memory("elementstorealias", looping);
    assert_no_asan("elementstorealias", &output);
    assert_exit("elementstorealias", &output, 0);

    let concrete = r#"module spec.cc.elementstorealiasctl;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func eachS(xs: S[2]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        v.name = dup(borrow xs[0].name);
        total = total + i32(len(bytes(v.name)));
    }
    return total;
}

func main(): i32 {
    let a: S[2] = [S { name: stringOf("ab"), y: 1 }, S { name: stringOf("cdef"), y: 2 }];
    return eachS(a) - 4;
}
"#;
    let output = run_memory("elementstorealiasctl", concrete);
    assert_no_asan("elementstorealiasctl", &output);
    assert_exit("elementstorealiasctl", &output, 0);
}

/// A store to a borrowed element of a local container survives an aliasing
/// right-hand side.
///
/// The element belongs to the local `a` — an opaque local container in the
/// generic spelling and a concrete one in the twin — so a right-hand side
/// that reads `a` reads the very storage the target names; the release of
/// the replaced owner must wait until the right-hand side has been
/// evaluated. Each run folds the cloned size back (exit 0) and the leak
/// checker is clean. (The generic spelling used to release the target before
/// the right-hand side read it and faulted on the emptied storage; the
/// concrete twin faulted the same way.)
#[test]
fn should_keep_an_aliasing_right_hand_side_alive_across_a_local_element_store() {
    let looping = r#"module spec.cc.elementlocalalias;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func each<T>(take v: T): i32 {
    let a: T = v;
    for (borrow e in a) {
        e.name = dup(borrow a[0].name);
    }
    return i32(len(bytes(a[0].name))) - 2;
}

func main(): i32 {
    let s: S[1] = [S { name: stringOf("ab"), y: 1 }];
    return each<S[1]>(s);
}
"#;
    let output = run_memory("elementlocalalias", looping);
    assert_no_asan("elementlocalalias", &output);
    assert_exit("elementlocalalias", &output, 0);

    let concrete = r#"module spec.cc.elementlocalaliasctl;

import { String, stringOf, bytes, clone } from std.text;

struct S { name: String; y: i32; }

func dup(borrow s: String): String {
    return clone(s);
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab"), y: 1 }];
    for (borrow v in a) {
        v.name = dup(borrow a[0].name);
    }
    return i32(len(bytes(a[0].name))) - 2;
}
"#;
    let output = run_memory("elementlocalaliasctl", concrete);
    assert_no_asan("elementlocalaliasctl", &output);
    assert_exit("elementlocalaliasctl", &output, 0);
}

// ===== The sibling-fixed null argument at an inferred generic call =====

/// A null-shaped argument whose sibling fixes the parameter reports the null
/// rule at each use site.
///
/// `pair(1, x)` with `x` inferred from `null` resolves the parameter from the
/// first argument, so the second argument's null shape faces the frozen null
/// rule exactly as the explicit `pair<i32>(1, x)` does: one `E10012` at the
/// argument's own site and no translation unit. The reversed spelling
/// (`pair(x, 1)`) and the two-null spelling (`pair(x, x)`) keep the frozen
/// per-use counts — one report and two reports — and the explicit twin keeps
/// its one report. (red at the phase base: `pair(1, x)` was accepted and the
/// strict build failed with `passing argument 2 ... makes integer from
/// pointer`.)
#[test]
fn should_judge_the_sibling_fixed_null_argument_at_each_use_site() {
    let sibling_fixed = r#"module spec.cc.inferrednull;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair(1, x);
}
"#;
    assert_rejected_once(
        "inferrednull",
        sibling_fixed,
        "E10012",
        &["'null' is not allowed here", "main.clo:9:20"],
        &["E10200"],
    );

    let reversed = r#"module spec.cc.inferrednullrev;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair(x, 1);
}
"#;
    assert_rejected_once(
        "inferrednullrev",
        reversed,
        "E10012",
        &["'null' is not allowed here", "main.clo:9:17"],
        &["E10200"],
    );

    let two_uses = r#"module spec.cc.inferrednullxx;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair(x, x);
}
"#;
    let (_scratch, output, c_path) = compile_only("inferrednullxx", two_uses);
    assert_ne!(
        output.status.code(),
        Some(0),
        "inferrednullxx: the front end accepted the two-use spelling"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        2,
        "inferrednullxx: each null use reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:9:17") && stderr.contains("main.clo:9:20"),
        "inferrednullxx: the reports name both use sites; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "inferrednullxx: a rejected program must not produce a translation unit"
    );

    let explicit_twin = r#"module spec.cc.inferrednulltwin;

func pair<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let x = null;
    return pair<i32>(1, x);
}
"#;
    assert_rejected_once(
        "inferrednulltwin",
        explicit_twin,
        "E10012",
        &["'null' is not allowed here", "main.clo:9:25"],
        &["E10200"],
    );
}

/// The nullable sibling instances stay accepted and read the callee's null
/// test back.
///
/// A `T?` parameter is null-capable, so a null-shaped argument at a nullable
/// instance is a legal null use: the inferred `str?` instance and the explicit
/// `String?` instance both build under the strict flags, run under the memory
/// helper, and fold the callee's `b == null` test back into their exit codes
/// (exit 0). (green controls at the phase base: both spellings were accepted
/// and ran.)
#[test]
fn should_keep_the_nullable_sibling_instances_accepted() {
    let view = r#"module spec.cc.inferrednullok;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func main(): i32 {
    let s: str? = null;
    let x = null;
    return pair(s, x) - 7;
}
"#;
    let output = run_memory("inferrednullok", view);
    assert_no_asan("inferrednullok", &output);
    assert_exit("inferrednullok", &output, 0);

    let owned = r#"module spec.cc.inferrednulloke;

import { String, stringOf } from std.text;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    let x = null;
    return pair<String?>(s, x) - 7;
}
"#;
    let output = run_memory("inferrednulloke", owned);
    assert_no_asan("inferrednulloke", &output);
    assert_exit("inferrednulloke", &output, 0);
}

// ===== The inverted conversion routes at their instances =====

/// A conversion the declaration cannot judge reports the frozen conversion at
/// its instance.
///
/// Four routes invert the direction the deferred conversion duty records: a
/// concrete return value at a parameter-typed return (`pick<cstring>()`'s
/// `return 5`), a parameter value at a concrete destination reached through
/// an explicit type argument (`consume2<cstring>(x)` with `x: str`), the
/// caller's own parameter written as the callee's explicit type argument
/// (`sink<U>(a)` at `U = cstring`), and a concrete argument at the callee's
/// parameter instantiated with the caller's own type argument (`inner<U>(c)`
/// at `U = raw u8`). Each reports the frozen `E10200` once at the written
/// value's site and writes no translation unit, exactly as its concrete twin
/// does; the explicit generic call with a static literal keeps its one
/// `E10200` as the rejecting control. (red at the phase base: all four routes
/// were accepted and failed the strict build with the C type mismatch the
/// instance judgment must report.)
#[test]
fn should_report_the_inverted_conversion_routes_at_their_instances() {
    let return_route = r#"module spec.cc.instconvret;

func pick<T>(): T {
    return 5;
}

func main(): i32 {
    let c: cstring = pick<cstring>();
    if (c == null) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once(
        "instconvret",
        return_route,
        "E10200",
        &["cannot convert 'i32' to 'cstring' implicitly", "main.clo:4:12"],
        &[],
    );

    let concrete_return = r#"module spec.cc.instconvretctl;

func pickc(): cstring {
    return 5;
}

func main(): i32 {
    let c: cstring = pickc();
    if (c == null) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once(
        "instconvretctl",
        concrete_return,
        "E10200",
        &["cannot convert 'i32' to 'cstring' implicitly", "main.clo:4:12"],
        &[],
    );

    let argument_route = r#"module spec.cc.instconvcallee;

func consume2<U>(c: U): i32 {
    return 0;
}

func convert<T>(x: T): i32 {
    return consume2<cstring>(x);
}

func main(): i32 {
    let s: str = "abc";
    return convert<str>(s);
}
"#;
    assert_rejected_once(
        "instconvcallee",
        argument_route,
        "E10200",
        &["cannot convert 'str' to 'cstring' implicitly", "main.clo:8:30"],
        &[],
    );

    let concrete_argument = r#"module spec.cc.instconvcalleectl;

func consume2(c: cstring): i32 {
    return 0;
}

func convert(x: str): i32 {
    return consume2(x);
}

func main(): i32 {
    let s: str = "abc";
    return convert(s);
}
"#;
    assert_rejected_once(
        "instconvcalleectl",
        concrete_argument,
        "E10200",
        &["cannot convert 'str' to 'cstring' implicitly", "main.clo:8:21"],
        &[],
    );

    let caller_parameter = r#"module spec.cc.instconvmixed;

func sink<U>(x: U): i32 {
    return 0;
}

func outer<T, U>(a: T): i32 {
    return sink<U>(a);
}

func main(): i32 {
    return outer<i32, cstring>(1) + 1;
}
"#;
    assert_rejected_once(
        "instconvmixed",
        caller_parameter,
        "E10200",
        &["cannot convert 'i32' to 'cstring' implicitly", "main.clo:8:20"],
        &[],
    );

    let concrete_parameter = r#"module spec.cc.instconvmixedctl;

func sink(x: cstring): i32 {
    return 0;
}

func outer(a: i32): i32 {
    return sink(a);
}

func main(): i32 {
    return outer(1) + 1;
}
"#;
    assert_rejected_once(
        "instconvmixedctl",
        concrete_parameter,
        "E10200",
        &["cannot convert 'i32' to 'cstring' implicitly", "main.clo:8:17"],
        &[],
    );

    let concrete_caller = r#"module spec.cc.instconvparam;

import { toCString } from std.text;

func inner<T>(x: T): i32 {
    if (x == null) {
        return 1;
    }
    return 0;
}

func outer<U>(c: cstring): i32 {
    return inner<U>(c);
}

func main(): i32 {
    let c: cstring = toCString("x");
    return outer<raw u8>(c);
}
"#;
    assert_rejected_once(
        "instconvparam",
        concrete_caller,
        "E10200",
        &["cannot convert 'cstring' to 'raw u8' implicitly", "main.clo:13:21"],
        &[],
    );

    let concrete_parameter_twin = r#"module spec.cc.instconvparamctl;

import { toCString } from std.text;

func inner(x: raw u8): i32 {
    if (x == null) {
        return 1;
    }
    return 0;
}

func outer(c: cstring): i32 {
    return inner(c);
}

func main(): i32 {
    let c: cstring = toCString("x");
    return outer(c);
}
"#;
    assert_rejected_once(
        "instconvparamctl",
        concrete_parameter_twin,
        "E10200",
        &["cannot convert 'cstring' to 'raw u8' implicitly", "main.clo:13:18"],
        &[],
    );

    let literal_control = r#"module spec.cc.instconvlit;

func consume2<U>(c: U): i32 {
    return 0;
}

func main(): i32 {
    return consume2<cstring>("x") + 1;
}
"#;
    assert_rejected_once(
        "instconvlit",
        literal_control,
        "E10200",
        &["cannot convert 'static str' to 'cstring' implicitly", "main.clo:8:30"],
        &[],
    );
}

/// The legal instances of the inverted routes stay accepted and read their
/// values back.
///
/// The instance judgment reports only the incompatible pairs: the reverse
/// route's `i32` instance returns its value, the explicit argument route's
/// `cstring` instance is the identity, and the caller-parameter routes accept
/// the identity and widening instances (`i32`, `str`, the `String` widening
/// to `str`, and `cstring`). Every accepted call folds its result into the
/// exit code and the program observes the values under the memory helper
/// (exit 0).
#[test]
fn should_keep_the_inverted_route_legal_instances_accepted() {
    let source = r#"module spec.cc.instconvok;

import { String, stringOf, toCString } from std.text;

func pick<T>(): T {
    return 5;
}

func consume2<U>(c: U): i32 {
    return 1;
}

func convert<T>(x: T): i32 {
    return consume2<cstring>(x);
}

func sink<U>(x: U): i32 {
    return 1;
}

func outer<T, U>(a: T): i32 {
    return sink<U>(a);
}

func inner<T>(x: T): i32 {
    return 1;
}

func outer2<U>(c: cstring): i32 {
    return inner<U>(c);
}

func main(): i32 {
    let r: i32 = pick<i32>();
    if (r != 5) {
        return 1;
    }
    let c: cstring = toCString("ab");
    if (convert<cstring>(c) != 1) {
        return 2;
    }
    if (outer<i32, i32>(7) != 1) {
        return 3;
    }
    let s: str = "xy";
    if (outer<str, str>(s) != 1) {
        return 4;
    }
    let h: String = stringOf("abc");
    if (outer<String, str>(h) != 1) {
        return 5;
    }
    if (outer<cstring, cstring>(c) != 1) {
        return 6;
    }
    if (outer2<cstring>(c) != 1) {
        return 7;
    }
    return 0;
}
"#;
    let output = run_memory("instconvok", source);
    assert_no_asan("instconvok", &output);
    assert_exit("instconvok", &output, 0);
}

// ===== The view and slice `ffi` result spellings =====

/// A view or slice `ffi` result at a view destination reports the conversion.
///
/// A view or slice crosses the FFI boundary as its pointer only, so the result
/// types as the ABI pointer: storing it in a `str` or a slice and passing it
/// to a `str` parameter each report the frozen `E10200` once, including the
/// nullable `str?` destination whose pointer keeps its nullability. The
/// template placeholder reports its own frozen `E10500` for the pointer. Each
/// rejection names the written call's span and writes no translation unit.
/// (green controls at the phase base: the pointer typing made these spellings
/// report exactly as pinned.)
#[test]
fn should_report_the_view_and_slice_ffi_result_destinations() {
    let view = r#"module spec.cc.ffiresultview;

ffi probe {
    func GetStr(): str;
}

func main(): i32 {
    let s: str = GetStr();
    if (len(s) != 3) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once(
        "ffiresultview",
        view,
        "E10200",
        &["cannot convert 'raw u8' to 'str' implicitly", "main.clo:8:18"],
        &[],
    );

    let slice = r#"module spec.cc.ffiresultslice;

ffi probe {
    func GetSlice(): u8[];
}

func main(): i32 {
    let s: u8[] = GetSlice();
    if (len(s) != 3) {
        return 1;
    }
    return i32(s[0]);
}
"#;
    assert_rejected_once(
        "ffiresultslice",
        slice,
        "E10200",
        &["cannot convert 'raw u8' to 'u8[]' implicitly", "main.clo:8:19"],
        &[],
    );

    let argument = r#"module spec.cc.ffiresultarg;

ffi probe {
    func Make(): str;
}

func consume(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    unsafe {
        return consume(Make());
    }
}
"#;
    assert_rejected_once(
        "ffiresultarg",
        argument,
        "E10200",
        &["cannot convert 'raw u8' to 'str' implicitly", "main.clo:13:24"],
        &[],
    );

    let nullable = r#"module spec.cc.ffiresultnullable;

ffi probe {
    func Maybe(): str?;
}

func main(): i32 {
    unsafe {
        let s: str? = Maybe();
        if (s == null) {
            return 1;
        }
        return i32(len(s));
    }
}
"#;
    assert_rejected_once(
        "ffiresultnullable",
        nullable,
        "E10200",
        &["cannot convert 'raw u8?' to 'str?' implicitly", "main.clo:9:23"],
        &[],
    );

    let template = r#"module spec.cc.ffiresulttemplate;

ffi probe {
    func Make(): str;
}

func main(): i32 {
    unsafe {
        let s: String = $"{Make()}";
        return i32(len(s));
    }
}
"#;
    assert_rejected_once(
        "ffiresulttemplate",
        template,
        "E10500",
        &["type 'raw u8' is not formattable", "main.clo:9:28"],
        &[],
    );
}

/// The recorded `ffi` stub for the `cstring` return: the call answers a
/// non-null literal so the program's null check observes the value.
const CSTRING_RETURN_STUB: &str = r#"typedef void *APTR;
typedef char *STRPTR;
STRPTR Get(void) { return "x"; }
"#;

/// Emits a program, links the emitted C with an inline stub under the strict
/// flags, and runs it, returning its output.
///
/// An `ffi` target has no definition in the emitted translation unit, so a
/// case that asserts a run supplies the recorded stub and the C compiler
/// builds both files together.
fn run_host_with_stub(tag: &str, source: &str, stub: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let stub_path = scratch.path().join("stub.c");
    std::fs::write(&stub_path, stub).expect("write the stub");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
            stub_path.to_str().expect("stub path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(tag, &program, &[])
}

/// Emits a program and strictly compiles the emitted C without linking.
///
/// The pinned evidence for a result position whose extern has no recorded
/// stub ends at the translation unit: the same strict flags a linking build
/// uses must accept it while the extern stays unresolved.
fn compile_host_c_only(tag: &str, source: &str) {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let object_path = scratch.path().join("build.o");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
            "-o",
            object_path.to_str().expect("object path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// The pointer uses of a view or slice `ffi` result build and read back.
///
/// The folded `len` of the ABI pointer is the empty length, exactly as the
/// same use of a `raw u8` value; each run reads the folded length back (exit
/// 0). A `cstring` result is a value pointer: the strict build links the
/// recorded stub and the run reads its non-null answer back (exit 0). The
/// index result is a pointer read whose extern has no recorded stub, so its
/// pinned evidence is the strict compile of the translation unit; the
/// `.ptr`/`.len` member spellings stay outside this table.
#[test]
fn should_keep_the_view_and_slice_ffi_result_uses_building() {
    let view = r#"module spec.cc.ffiresultlen;

ffi probe {
    func Make(): str;
}

func main(): i32 {
    unsafe {
        return i32(len(Make()));
    }
}
"#;
    let output = run_memory("ffiresultlen", view);
    assert_no_asan("ffiresultlen", &output);
    assert_exit("ffiresultlen", &output, 0);

    let slice = r#"module spec.cc.ffiresultslielen;

ffi probe {
    func Make(): u8[];
}

func main(): i32 {
    unsafe {
        return i32(len(Make()));
    }
}
"#;
    let output = run_memory("ffiresultslielen", slice);
    assert_no_asan("ffiresultslielen", &output);
    assert_exit("ffiresultslielen", &output, 0);

    let c_string = r#"module spec.cc.ffiresultcstring;

ffi probe {
    func Get(): cstring;
}

func main(): i32 {
    unsafe {
        let c: cstring = Get();
        if (c == null) {
            return 1;
        }
        return 0;
    }
}
"#;
    let output = run_host_with_stub("ffiresultcstring", c_string, CSTRING_RETURN_STUB);
    assert_exit("ffiresultcstring", &output, 0);

    let index = r#"module spec.cc.ffiresultindex;

ffi probe {
    func Make(): str;
}

func main(): i32 {
    unsafe {
        return i32(Make()[0]);
    }
}
"#;
    compile_host_c_only("ffiresultindex", index);
}

// ===== The context-typed literals and the borrow-slot count =====

/// A numeric literal takes the scalar context's type at every destination.
///
/// A literal's type is the context's where the instance supplies a scalar —
/// the frozen numeric widenings judge it there — so the return, the `let`,
/// the assignment, the struct-literal field, the array element, the argument,
/// and the float destination all accept their literals exactly as the
/// concrete twins do. Each run folds its read-back (exit 0).
#[test]
fn should_accept_a_numeric_literal_at_every_scalar_destination() {
    let returned = r#"module spec.cc.litcontextret;

func pick<T>(): T {
    return 5;
}

func main(): i32 {
    let y: u8 = pick<u8>();
    return i32(y) - 5;
}
"#;
    let output = run_memory("litcontextret", returned);
    assert_no_asan("litcontextret", &output);
    assert_exit("litcontextret", &output, 0);

    let binding = r#"module spec.cc.litcontextlet;

func outer<T>(): T {
    let y: T = 5;
    return y;
}

func main(): i32 {
    let z: u8 = outer<u8>();
    return i32(z) - 5;
}
"#;
    let output = run_memory("litcontextlet", binding);
    assert_no_asan("litcontextlet", &output);
    assert_exit("litcontextlet", &output, 0);

    let assigned = r#"module spec.cc.litcontextassign;

func outer<T>(): u8 {
    let y: T = 1;
    y = 5;
    return u8(y);
}

func main(): i32 {
    return i32(outer<u8>()) - 5;
}
"#;
    let output = run_memory("litcontextassign", assigned);
    assert_no_asan("litcontextassign", &output);
    assert_exit("litcontextassign", &output, 0);

    let field = r#"module spec.cc.litcontextfield;

struct Box<T> { x: T; }

func outer<T>(): T {
    let b: Box<T> = Box<T> { x: 5 };
    return b.x;
}

func main(): i32 {
    let v: u8 = outer<u8>();
    return i32(v) - 5;
}
"#;
    let output = run_memory("litcontextfield", field);
    assert_no_asan("litcontextfield", &output);
    assert_exit("litcontextfield", &output, 0);

    let element = r#"module spec.cc.litcontextelem;

func outer<T>(): T {
    let xs: T[2] = [1, 2];
    return xs[0];
}

func main(): i32 {
    let v: u8 = outer<u8>();
    return i32(v) - 1;
}
"#;
    let output = run_memory("litcontextelem", element);
    assert_no_asan("litcontextelem", &output);
    assert_exit("litcontextelem", &output, 0);

    let argument = r#"module spec.cc.litcontextarg;

func sink<T>(x: T): i32 {
    return 0;
}

func outer<T>(): i32 {
    return sink<T>(5);
}

func main(): i32 {
    return outer<u8>();
}
"#;
    let output = run_memory("litcontextarg", argument);
    assert_no_asan("litcontextarg", &output);
    assert_exit("litcontextarg", &output, 0);

    let float = r#"module spec.cc.litcontextfloat;

func outer<T>(): T {
    return 1.5;
}

func main(): i32 {
    let v: f32 = outer<f32>();
    return i32(v) - 1;
}
"#;
    let output = run_memory("litcontextfloat", float);
    assert_no_asan("litcontextfloat", &output);
    assert_exit("litcontextfloat", &output, 0);
}

/// A numeric literal at a non-scalar destination keeps the frozen report.
///
/// A view destination is no scalar context, so `pick<str>()`'s `return 5`
/// reports the frozen conversion once, exactly as its concrete twin does.
#[test]
fn should_report_a_numeric_literal_at_a_non_scalar_destination() {
    let view = r#"module spec.cc.litcontextview;

func pick<T>(): T {
    return 5;
}

func main(): i32 {
    let s: str = pick<str>();
    return i32(len(s));
}
"#;
    assert_rejected_once(
        "litcontextview",
        view,
        "E10200",
        &["cannot convert 'i32' to 'str' implicitly", "main.clo:4:12"],
        &[],
    );
}

/// A borrow argument whose declared slot mentions a parameter reports once.
///
/// The out-slot place duty judges the borrow argument at its instance, so the
/// ordinary conversion record stays silent: the shape reports one `E10200`
/// exactly as its concrete twin does.
#[test]
fn should_report_a_borrow_slot_argument_once() {
    let slot = r#"module spec.cc.borrowslotcount;

export handle File freedBy Close;

ffi probe {
    func Open(name: cstring, mode: i32): File?;
    func Close(take f: File): void;
}

func fill<T>(borrow out: T, take x: T): void {
    out = x;
}

func outer<T, U>(a: T, b: U): i32 {
    fill<T>(borrow b, a);
    return 0;
}

func main(): i32 {
    unsafe {
        let f = Open("/definitely/not/here", 0);
        if (f == null) {
            return 7;
        }
        let s: str = "y";
        return outer<File?, str>(f, s);
    }
}
"#;
    let (_scratch, output, c_path) = compile_only("borrowslotcount", slot);
    assert_ne!(
        output.status.code(),
        Some(0),
        "borrowslotcount: the front end accepted the incompatible borrow slot"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "borrowslotcount: the borrow slot reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'str' to 'File?' implicitly"),
        "borrowslotcount: the report names the slot conversion; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "borrowslotcount: a rejected program must not produce a translation unit"
    );
}

/// A caller-parameter-resolved null sibling is judged at its instance.
///
/// `pair(x, n)` inside `outer<U>` resolves the parameter from the caller's own
/// parameter, so the null argument is judged where the instance is: at
/// `U = str?` the nullable slot keeps the legal null use (exit 0), and an
/// un-instantiated body is accepted too. The concrete `i32` instance reports
/// the frozen null rule exactly once, at the argument's site.
#[test]
fn should_keep_a_caller_parameter_resolved_nullable_instance_accepted() {
    let nullable = r#"module spec.cc.parsiblingnullable;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func outer<U>(x: U): i32 {
    let n = null;
    return pair(x, n) - 7;
}

func main(): i32 {
    let s: str? = null;
    return outer<str?>(s);
}
"#;
    let output = run_memory("parsiblingnullable", nullable);
    assert_no_asan("parsiblingnullable", &output);
    assert_exit("parsiblingnullable", &output, 0);

    let no_instantiation = r#"module spec.cc.parsiblingnoinst;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func outer<U>(x: U): i32 {
    let n = null;
    return pair(x, n) - 7;
}

func main(): i32 {
    return 0;
}
"#;
    let output = run_memory("parsiblingnoinst", no_instantiation);
    assert_no_asan("parsiblingnoinst", &output);
    assert_exit("parsiblingnoinst", &output, 0);

    let concrete = r#"module spec.cc.parsiblingi32;

func pair<T>(a: T, b: T): i32 {
    if (b == null) {
        return 7;
    }
    return 0;
}

func outer<U>(x: U): i32 {
    let n = null;
    return pair(x, n) - 7;
}

func main(): i32 {
    let s: i32 = 5;
    return outer<i32>(s);
}
"#;
    let (_scratch, output, c_path) = compile_only("parsiblingi32", concrete);
    assert_ne!(
        output.status.code(),
        Some(0),
        "parsiblingi32: the front end accepted the concrete null use"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        1,
        "parsiblingi32: the concrete instance reports once; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:12:20"),
        "parsiblingi32: the report lands on the argument's site; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "parsiblingi32: a rejected program must not produce a translation unit"
    );
}

/// A literal at an incompatible scalar destination reports like its twin.
///
/// A float at an integer, an integer at a float, and an integer at a bool each
/// report the frozen conversion at the written value's site, exactly as their
/// concrete twins do; an integer outside its destination's range reports the
/// literal-fit rule instead.
#[test]
fn should_report_the_literal_boundary_instances_like_their_twins() {
    let float_to_integer = r#"module spec.cc.litboundfloat;

func pickf<T>(): T {
    return 1.5;
}

func main(): i32 {
    let v: i32 = pickf<i32>();
    return v - 1;
}
"#;
    assert_rejected_once(
        "litboundfloat",
        float_to_integer,
        "E10200",
        &["cannot convert 'f64' to 'i32' implicitly", "main.clo:4:12"],
        &[],
    );
    let float_to_integer_twin = r#"module spec.cc.litboundfloattwin;

func pickfc(): i32 {
    return 1.5;
}

func main(): i32 {
    let v: i32 = pickfc();
    return v - 1;
}
"#;
    assert_rejected_once(
        "litboundfloattwin",
        float_to_integer_twin,
        "E10200",
        &["cannot convert 'f64' to 'i32' implicitly", "main.clo:4:12"],
        &[],
    );

    let integer_to_float = r#"module spec.cc.litboundintf32;

func picki<T>(): T {
    return 5;
}

func main(): i32 {
    let v: f32 = picki<f32>();
    return 0;
}
"#;
    assert_rejected_once(
        "litboundintf32",
        integer_to_float,
        "E10200",
        &["cannot convert 'i32' to 'f32' implicitly", "main.clo:4:12"],
        &[],
    );
    let integer_to_float_twin = r#"module spec.cc.litboundintf32twin;

func pickic(): f32 {
    return 5;
}

func main(): i32 {
    let v: f32 = pickic();
    return 0;
}
"#;
    assert_rejected_once(
        "litboundintf32twin",
        integer_to_float_twin,
        "E10200",
        &["cannot convert 'i32' to 'f32' implicitly", "main.clo:4:12"],
        &[],
    );

    let integer_to_bool = r#"module spec.cc.litboundintbool;

func pickb<T>(): T {
    return 5;
}

func main(): i32 {
    let v: bool = pickb<bool>();
    if (v) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once(
        "litboundintbool",
        integer_to_bool,
        "E10200",
        &["cannot convert 'i32' to 'bool' implicitly", "main.clo:4:12"],
        &[],
    );

    let out_of_range = r#"module spec.cc.litboundrange;

func picko<T>(): T {
    return 300;
}

func main(): i32 {
    let v: u8 = picko<u8>();
    return i32(v) - 44;
}
"#;
    assert_rejected_once(
        "litboundrange",
        out_of_range,
        "E10202",
        &["literal does not fit 'u8'", "main.clo:4:12"],
        &[],
    );
    let out_of_range_twin = r#"module spec.cc.litboundrangetwin;

func pickoc(): u8 {
    return 300;
}

func main(): i32 {
    let v: u8 = pickoc();
    return i32(v) - 44;
}
"#;
    assert_rejected_once(
        "litboundrangetwin",
        out_of_range_twin,
        "E10202",
        &["literal does not fit 'u8'", "main.clo:4:12"],
        &[],
    );
}

/// A non-nullable borrow slot reports like its concrete twin.
///
/// `store<T>(borrow b, a)` at `T = i32` receives the caller's `str` through a
/// borrow slot that resolves non-nullably, so the ordinary conversion reports
/// once — the same code and count the concrete twin reports.
#[test]
fn should_report_a_non_nullable_borrow_slot_argument() {
    let generic = r#"module spec.cc.borrowslotgeneric;

func store<T>(borrow out: T, x: T): void {
    out = x;
}

func outer<T, U>(a: T, b: U): i32 {
    store<T>(borrow b, a);
    return 0;
}

func main(): i32 {
    let n: i32 = 1;
    let s: str = "y";
    return outer<i32, str>(n, s);
}
"#;
    assert_rejected_once(
        "borrowslotgeneric",
        generic,
        "E10200",
        &["cannot convert 'str' to 'i32' implicitly", "main.clo:8:21"],
        &[],
    );

    let concrete = r#"module spec.cc.borrowslotconcrete;

func storec(borrow out: i32, x: i32): void {
    out = x;
}

func outerc(a: i32, b: str): i32 {
    storec(borrow b, a);
    return 0;
}

func main(): i32 {
    let n: i32 = 1;
    let s: str = "y";
    return outerc(n, s);
}
"#;
    assert_rejected_once(
        "borrowslotconcrete",
        concrete,
        "E10200",
        &["cannot convert 'str' to 'i32' implicitly", "main.clo:8:19"],
        &[],
    );
}

// ===== Member typing on concrete paths =====

/// An unknown member on a concrete base reports the unknown-member rule.
///
/// A fixed array has no pointer member, a view has no member outside its
/// pointer and its length, and a `cstring` has no member surface at all, so
/// each fallthrough spelling reports the registered `E10212` exactly once, at
/// the base expression's site, and writes no translation unit (red at the
/// phase base: all three were accepted and the strict build failed with
/// `request for member 'ptr'` / `'ClStr' has no member named 'foo'`). The
/// program below pins the typed members and the scoped and qualified forms as
/// the controls; the instance audit mirrors the same rule for the substituted
/// bases (`fixedptrmember`, `memenum`).
#[test]
fn should_reject_the_unknown_member_on_concrete_bases() {
    let fixed_array = r#"module spec.cc.fixedmember;

func main(): i32 {
    let a: u8[4] = [1, 2, 3, 4];
    let p = a.ptr;
    if (p == null) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once(
        "fixedmember",
        fixed_array,
        "E10212",
        &["'ptr' is not a member of 'u8[4]'", "main.clo:5:13"],
        &[],
    );

    let view = r#"module spec.cc.viewmemberunknown;

func main(): i32 {
    let s: str = "abc";
    let x = s.foo;
    return 0;
}
"#;
    assert_rejected_once(
        "viewmemberunknown",
        view,
        "E10212",
        &["'foo' is not a member of 'str'", "main.clo:5:13"],
        &[],
    );

    let c_string = r#"module spec.cc.cstringmember;

import { toCString } from std.text;

func main(): i32 {
    let c: cstring = toCString("abc");
    let p = c.ptr;
    return 0;
}
"#;
    assert_rejected_once(
        "cstringmember",
        c_string,
        "E10212",
        &["'ptr' is not a member of 'cstring'", "main.clo:7:13"],
        &[],
    );

    // The typed members and the scoped and qualified forms stay accepted: the
    // view's `.len`, the fixed array's structural `.len`, the slice's
    // `.ptr`/`.len`, the view pointer at a `cstring` slot, the scoped enum
    // member, and the module-qualified calls all build and read their values
    // back.
    let controls = r#"module spec.cc.membercontrols;

import { slice } from std.mem;

enum E : u8 { A, B }

func span(s: str): usize {
    return s.len;
}

func cstr(s: str): cstring {
    return s.ptr;
}

func main(): i32 {
    let s: str = "abcd";
    let a: u8[4] = [65, 66, 67, 68];
    let sl: u8[] = slice(a, 0, 4);
    if (i32(span(s)) != 4) {
        return 1;
    }
    if (i32(a.len) != 4) {
        return 2;
    }
    let p: raw u8 = sl.ptr;
    if (i32(sl.len) != 4) {
        return 3;
    }
    unsafe {
        if (p == null) {
            return 4;
        }
        if (p[0] != 65) {
            return 5;
        }
    }
    let c: cstring = s.ptr;
    if (c == null) {
        return 6;
    }
    let d: cstring = cstr(s);
    if (d == null) {
        return 7;
    }
    let e: E = E.B;
    let owned: String = std.text.stringOf("abc");
    let v: u8[] = std.text.bytes(owned);
    return i32(e) + i32(len(v)) - 4;
}
"#;
    let output = run_memory("membercontrols", controls);
    assert_no_asan("membercontrols", &output);
    assert_exit("membercontrols", &output, 0);
}

/// A view pointer member read at a mutable raw destination is cast.
///
/// The view's `.ptr` is a `const u8 *` in the emitted C while a `raw u8`
/// destination is a mutable byte pointer: the local store, the `raw u8`
/// return, and the raw struct-literal field each spell the sanctioned member
/// cast, so every destination builds strictly and the run reads the bytes
/// back (red at the phase base: the three spellings were accepted and the
/// strict build failed with `discarded-qualifiers`). The explicit cast and
/// the pre-existing `cstring` slots and read-only uses stay accepted in the
/// controls below.
#[test]
fn should_cast_the_view_pointer_member_at_a_mutable_raw_destination() {
    let source = r#"module spec.cc.constptrmutdest;

struct Holder {
    c: raw u8;
}

func get(s: str): raw u8 {
    return s.ptr;
}

func main(): i32 {
    let s: str = "abc";
    let p: raw u8 = s.ptr;
    let h: Holder = Holder { c: s.ptr };
    let q: raw u8 = get(s);
    unsafe {
        if (p == null) {
            return 1;
        }
        if (h.c == null) {
            return 2;
        }
        if (q == null) {
            return 3;
        }
        return i32(*p) + i32(*h.c) + i32(*q) - 291;
    }
}
"#;
    let output = run_memory("constptrmutdest", source);
    assert_no_asan("constptrmutdest", &output);
    assert_exit("constptrmutdest", &output, 0);

    // The sanctioned explicit cast (`raw u8(s.ptr)` inside `unsafe`), the
    // `cstring` store slot, and the read-only comparison stay accepted and
    // read their values back.
    let controls = r#"module spec.cc.constptrcontrols;

func main(): i32 {
    let s: str = "abc";
    let p: raw u8 = unsafe raw u8(s.ptr);
    let c: cstring = s.ptr;
    if (p == null) {
        return 1;
    }
    if (c == null) {
        return 2;
    }
    if (s.ptr == null) {
        return 3;
    }
    unsafe {
        return i32(*p) - 97;
    }
}
"#;
    let output = run_memory("constptrcontrols", controls);
    assert_no_asan("constptrcontrols", &output);
    assert_exit("constptrcontrols", &output, 0);
}

/// A module initializer's pointer member store casts like a store.
///
/// The module-scope direct store of a view's `.ptr` at a raw destination and
/// at a `cstring` destination spells the sanctioned member conversions a
/// function-path store spells, so the emitted initializer builds strictly and
/// the run reads the module view's bytes back (red at the phase base: the two
/// direct stores were accepted and the strict build failed with
/// `discarded-qualifiers`). The raw struct-literal field, the raw array
/// literal, the raw conditional, and the explicit cast stay accepted below.
#[test]
fn should_cast_the_module_initializer_pointer_member_store() {
    let source = r#"module spec.cc.modulestore;

struct Holder {
    c: raw u8;
}

let s: str = "abc";
let g: raw u8 = s.ptr;
let c: cstring = s.ptr;
let h: Holder = Holder { c: s.ptr };
let a: raw u8[2] = [s.ptr, null];
let p: raw u8 = unsafe raw u8(s.ptr);

func main(): i32 {
    if (c == null) {
        return 1;
    }
    if (h.c == null) {
        return 2;
    }
    if (a[0] == null) {
        return 3;
    }
    if (p == null) {
        return 4;
    }
    unsafe {
        if (g == null) {
            return 5;
        }
        return i32(*g) - 97;
    }
}
"#;
    let output = run_memory("modulestore", source);
    assert_no_asan("modulestore", &output);
    assert_exit("modulestore", &output, 0);
}

/// A raw-enum member read reports the value-form member rule at both paths.
///
/// A member reached through a raw pointer implies the dereference (Ch 04
/// section 7), so the receiver is the pointee value; an enum constant is
/// reached only through the scoped `Type.MEMBER` spelling (Ch 04 section 8),
/// and a value receiver has no member surface, so every raw-enum member read
/// — an existing constant and an unknown name alike — reports the
/// registered `E10212` once at the base's site and writes no translation
/// unit. The instance spelling reports the same rule for the substituted
/// base; the scoped and qualified forms stay accepted below (red at the
/// phase base: the concrete reads were accepted and the strict build failed
/// with `request for member`, while the instance twin already reported
/// `E10212`).
#[test]
fn should_reject_a_raw_enum_member_read_at_both_paths() {
    let existing = r#"module spec.cc.rawenum;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.A;
    let p: raw E = unsafe &e;
    unsafe {
        return i32(p.A);
    }
}
"#;
    assert_rejected_once(
        "rawenum",
        existing,
        "E10212",
        &["'A' is not a member of", "main.clo:9:20"],
        &[],
    );

    let missing = r#"module spec.cc.rawenummissing;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.B;
    let p: raw E = unsafe &e;
    unsafe {
        return i32(p.N);
    }
}
"#;
    assert_rejected_once(
        "rawenummissing",
        missing,
        "E10212",
        &["'N' is not a member of", "main.clo:9:20"],
        &[],
    );

    let instance = r#"module spec.cc.rawenuminstance;

enum E : u8 { A, B }

func read<T>(x: T): i32 {
    return i32(x.B);
}

func main(): i32 {
    let e: E = E.A;
    let p: raw E = unsafe &e;
    return read<raw E>(p) - 1;
}
"#;
    assert_rejected_once(
        "rawenuminstance",
        instance,
        "E10212",
        &["'B' is not a member of 'raw E'", "main.clo:6:16"],
        &[],
    );

    let instance_missing = r#"module spec.cc.rawenuminstmissing;

enum E : u8 { A, B }

func read<T>(x: T): i32 {
    return i32(x.N);
}

func main(): i32 {
    let e: E = E.B;
    let p: raw E = unsafe &e;
    return read<raw E>(p);
}
"#;
    assert_rejected_once(
        "rawenuminstmissing",
        instance_missing,
        "E10212",
        &["'N' is not a member of 'raw E'", "main.clo:6:16"],
        &[],
    );

    // The scoped member and the module-qualified calls are the controls: the
    // enum constant and the qualified spelling stay accepted and read their
    // values back.
    let controls = r#"module spec.cc.rawenumcontrols;

enum E : u8 { A, B }

func main(): i32 {
    let e: E = E.A;
    let owned: String = std.text.stringOf("abc");
    let v: u8[] = std.text.bytes(owned);
    return i32(e) + i32(len(v)) - 3;
}
"#;
    let output = run_memory("rawenumcontrols", controls);
    assert_no_asan("rawenumcontrols", &output);
    assert_exit("rawenumcontrols", &output, 0);
}

// ===== Call identity and written-argument checks =====

/// The holder module the declaration/import collision programs import from.
///
/// It exports `v` for the renamed-import shapes and `g` for the direct
/// spelling, so the colliding local name is always `g`.
const DECL_COLLISION_HOLDER: &str = r#"module spec.cc.declcollisionholder;

export let v: i32 = 5;
export let g: i32 = 5;
"#;

/// Builds a module that binds `g` through an import and through its own
/// declaration, in the requested order.
///
/// The import renames `v` to `g`, so the diagnostic's site is the imported
/// source name `v` while its message names the local binding `g`. Returns the
/// program text and the one-based line of the import statement.
fn declaration_collision_program(
    module: &str,
    declaration: &str,
    import_first: bool,
) -> (String, usize) {
    let import = "import { v as g } from spec.cc.declcollisionholder;";
    let main = "func main(): i32 {\n    return 0;\n}";
    if import_first {
        (
            format!("module {module};\n\n{import}\n\n{declaration}\n\n{main}\n"),
            3,
        )
    } else {
        let import_line = declaration.lines().count() + 4;
        (
            format!("module {module};\n\n{declaration}\n\n{import}\n\n{main}\n"),
            import_line,
        )
    }
}

/// A dot call's written argument is judged against the parameter the receiver
/// fixes.
///
/// The receiver `b: Box` fixes the callee's first type parameter to `Box`, so
/// every written argument must convert to `Box` under the usual implicit
/// widenings; the callee's type arguments are inferred at the call site. The
/// `take` and `borrow` receiver spellings each report one `E10200` at the
/// written argument, and the direct inferred and explicit twins report the
/// same code at their own written argument. The matching `take` dot call and
/// the generic caller `outer<i32>(b, c)` with `c: Box` run their values,
/// while the `c: i32` caller reports at the written argument.
///
/// Before the fix the dot calls were accepted and the strict C build failed on
/// `cl_pick__Box` and `cl_put__Box`; the valid generic caller was falsely
/// rejected with two `E10200` because the deferred record judged the callee's
/// parameter against the caller's own instance; and the `c: i32` caller
/// reported at the receiver instead of the written argument.
#[test]
fn should_report_a_dot_call_written_argument_at_its_resolved_parameter() {
    let take_dot = r#"module spec.cc.dotwrittenarg;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let g: i32 = b.pick(7);
    return g - 1;
}
"#;
    assert_rejected_once(
        "dotwrittenarg",
        take_dot,
        "E10200",
        &[
            "cannot convert 'i32' to 'Box' implicitly",
            "main.clo:11:25",
        ],
        &[],
    );

    let borrow_dot = r#"module spec.cc.dotwrittenborrow;

struct Box { x: i32; }

func put<T>(borrow out: T, v: T): void {
    out = v;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    b.put(7);
    return b.x - 1;
}
"#;
    assert_rejected_once(
        "dotwrittenborrow",
        borrow_dot,
        "E10200",
        &[
            "cannot convert 'i32' to 'Box' implicitly",
            "main.clo:11:11",
        ],
        &[],
    );

    let inferred = r#"module spec.cc.dotwritteninferred;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let g: i32 = pick(b, 7);
    return g - 1;
}
"#;
    assert_rejected_once(
        "dotwritteninferred",
        inferred,
        "E10200",
        &[
            "cannot convert 'i32' to 'Box' implicitly",
            "main.clo:11:26",
        ],
        &[],
    );

    let explicit = r#"module spec.cc.dotwrittenexplicit;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let g: i32 = pick<Box>(b, 7);
    return g - 1;
}
"#;
    assert_rejected_once(
        "dotwrittenexplicit",
        explicit,
        "E10200",
        &[
            "cannot convert 'i32' to 'Box' implicitly",
            "main.clo:11:31",
        ],
        &[],
    );

    let generic_i32 = r#"module spec.cc.dotgenerici32;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func outer<T>(b: Box, c: T): i32 {
    return b.pick(c);
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    return outer<i32>(b, 7) - 1;
}
"#;
    assert_rejected_once(
        "dotgenerici32",
        generic_i32,
        "E10200",
        &[
            "cannot convert 'i32' to 'Box' implicitly",
            "main.clo:10:19",
        ],
        &[],
    );

    let matching = r#"module spec.cc.dotcontrol;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 2;
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let c: Box = Box { x: 2 };
    let g: i32 = b.pick(c);
    return g - 2;
}
"#;
    let output = run_memory("dotcontrol", matching);
    assert_no_asan("dotcontrol", &output);
    assert_exit("dotcontrol", &output, 0);

    let generic_box = r#"module spec.cc.dotgenericbox;

struct Box { x: i32; }

func pick<T>(take v: T, take w: T): i32 {
    return 1;
}

func outer<T>(b: Box, c: Box): i32 {
    return b.pick(c);
}

func main(): i32 {
    let b: Box = Box { x: 1 };
    let c: Box = Box { x: 2 };
    return outer<i32>(b, c) - 1;
}
"#;
    let output = run_memory("dotgenericbox", generic_box);
    assert_no_asan("dotgenericbox", &output);
    assert_exit("dotgenericbox", &output, 0);
}

/// One name may not be bound by both an import and a same-module declaration.
///
/// Binding one name twice in a module is an error for every declaration kind,
/// in either order; the collision reports once, at the import item's name, and
/// names the local binding. The module declaration stays a valid single
/// declaration (no duplicate-declaration report), and a renamed import beside
/// an unrelated declaration stays accepted and runs. Before the fix only a
/// prior import was compared, so every declaration kind was accepted with an
/// unused-import warning.
#[test]
fn should_report_a_declaration_import_collision() {
    let direct = r#"module spec.cc.declcollisiondirect;

import { g } from spec.cc.declcollisionholder;

let g: i32 = 7;

func main(): i32 {
    return g - 7;
}
"#;
    let (_scratch, output, c_path) =
        compile_pair("declcollision-direct", DECL_COLLISION_HOLDER, direct);
    assert_rejected_output(
        "declcollision-direct",
        &output,
        &c_path,
        "E11103",
        &["duplicate import of 'g'", "main.clo:3:10"],
        &["E11101"],
    );

    // Every declaration kind the module loader registers, in both orders. A
    // `comptime` declaration registers like the others; the collision is a
    // load error, so the driver stops before the staged-feature report runs and
    // this row reports the same single `E11103`.
    let kinds: &[(&str, &str)] = &[
        ("let", "let g: i32 = 7;"),
        ("func", "func g(): i32 { return 7; }"),
        ("struct", "struct g { x: i32; }"),
        ("const", "const g: i32 = 7;"),
        ("enum", "enum g : u8 { A, B }"),
        (
            "handle",
            r#"ffi hlib {
    func Close(take p: g): void;
}
handle g freedBy Close;"#,
        ),
        ("ffi-block", "ffi g { func h(x: i32): i32; }"),
        ("ffi-func", "ffi hlib { func g(x: i32): i32; }"),
        ("intrinsic", "intrinsic func g(): i32;"),
        ("comptime", "comptime func g(): i32 { return 7; }"),
    ];
    for (index, (kind, declaration)) in kinds.iter().enumerate() {
        for import_first in [true, false] {
            let order = if import_first { "importfirst" } else { "declfirst" };
            let tag = format!("declcollision-{kind}-{order}");
            let module = format!(
                "spec.cc.declcollision{index}{}",
                if import_first { "i" } else { "d" }
            );
            let (main, import_line) =
                declaration_collision_program(&module, declaration, import_first);
            let (_scratch, output, c_path) = compile_pair(&tag, DECL_COLLISION_HOLDER, &main);
            let site = format!("main.clo:{import_line}:10");
            assert_rejected_output(
                &tag,
                &output,
                &c_path,
                "E11103",
                &["duplicate import of 'g'", site.as_str()],
                &["E11101"],
            );
        }
    }

    let control = r#"module spec.cc.declcollisioncontrol;

import { v as g } from spec.cc.declcollisionholder;

let h: i32 = 2;

func main(): i32 {
    return g + h - 7;
}
"#;
    let output = run_memory_pair("declcollision-control", DECL_COLLISION_HOLDER, control);
    assert_no_asan("declcollision-control", &output);
    assert_exit("declcollision-control", &output, 0);
}

/// The holder module the direct-name shadow programs import from.
///
/// The exported `b` is a `Box`, so a dot receiver that wrongly resolved to the
/// import would instantiate the loop over a non-iterable struct and fail.
const SHADOW_HOLDER: &str = r#"module spec.cc.shadowholder;

export struct Box { n: i32; }

export let b: Box = Box { n: 5 };
"#;

/// A direct-name shadow at a dot receiver keeps its own type.
///
/// A local, a parameter, a generic-body local, and an expression- or
/// block-bodied lambda parameter may shadow a module name, and the innermost
/// scope decides the dot-call receiver: each program builds strictly and runs
/// its expected value, so the imported binding is never the receiver. The
/// no-import baseline pins the same loop shape without a shadow. The
/// module-own/module-initializer shape is not part of this control; it is the
/// declaration/import collision above.
#[test]
fn should_keep_a_direct_name_shadow_at_a_dot_receiver() {
    let local = r#"module spec.cc.shadowlocal;

import { b } from spec.cc.shadowholder;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) { n = n + i32(v); }
    return n;
}

func body(): i32 {
    let b: i32[2] = [3, 4];
    return b.each();
}

let g: i32 = body();

func main(): i32 {
    return g - 7;
}
"#;
    let output = run_memory_pair("shadowlocal", SHADOW_HOLDER, local);
    assert_no_asan("shadowlocal", &output);
    assert_exit("shadowlocal", &output, 0);

    let parameter = r#"module spec.cc.shadowparam;

import { b } from spec.cc.shadowholder;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) { n = n + i32(v); }
    return n;
}

func body(b: i32[2]): i32 {
    return b.each();
}

func main(): i32 {
    let a: i32[2] = [3, 4];
    return body(a) - 7;
}
"#;
    let output = run_memory_pair("shadowparam", SHADOW_HOLDER, parameter);
    assert_no_asan("shadowparam", &output);
    assert_exit("shadowparam", &output, 0);

    let generic = r#"module spec.cc.shadowgeneric;

import { b } from spec.cc.shadowholder;

func drain<T>(xs: T): i32 { let n: i32 = 0; for (v in xs) { n = n + 1; } return n; }

func run<T>(x: T): i32 { let b: i32[2] = [3, 4]; return b.drain() + 0; }

func main(): i32 { let a: i32[2] = [3, 4]; return run<i32[2]>(a) - 2; }
"#;
    let output = run_memory_pair("shadowgeneric", SHADOW_HOLDER, generic);
    assert_no_asan("shadowgeneric", &output);
    assert_exit("shadowgeneric", &output, 0);

    let lambda_expression = r#"module spec.cc.shadowlambdaexpr;

import { b } from spec.cc.shadowholder;

func drain<T>(xs: T): i32 { let n: i32 = 0; for (v in xs) { n = n + 1; } return n; }

let f: func(i32[2]): i32 = (b: i32[2]) => b.drain();

func main(): i32 { let a: i32[2] = [3, 4]; return f(a) - 2; }
"#;
    let output = run_memory_pair("shadowlambdaexpr", SHADOW_HOLDER, lambda_expression);
    assert_no_asan("shadowlambdaexpr", &output);
    assert_exit("shadowlambdaexpr", &output, 0);

    let lambda_block = r#"module spec.cc.shadowlambdablock;

import { b } from spec.cc.shadowholder;

func drain<T>(xs: T): i32 { let n: i32 = 0; for (v in xs) { n = n + 1; } return n; }

let f: func(i32[2]): i32 = (b: i32[2]) => { return b.drain(); };

func main(): i32 { let a: i32[2] = [3, 4]; return f(a) - 2; }
"#;
    let output = run_memory_pair("shadowlambdablock", SHADOW_HOLDER, lambda_block);
    assert_no_asan("shadowlambdablock", &output);
    assert_exit("shadowlambdablock", &output, 0);

    let baseline = r#"module spec.cc.shadowcontrol;

func each<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) { n = n + i32(v); }
    return n;
}

func body(): i32 {
    let b: i32[2] = [3, 4];
    return b.each();
}

func main(): i32 {
    return body() - 7;
}
"#;
    let output = run_host("shadowcontrol", baseline);
    assert_exit("shadowcontrol", &output, 0);
}

// ===== Qualified call identity and the written-value span =====

/// A self-qualified user call keeps its take, borrow, and initializer modes.
///
/// The module spells its own name in front of the call; the declaration's
/// ownership modes still decide the call. The initializer's `take` value is
/// owned and released by the callee, the body's `take` value is moved once, and
/// the borrow leaves the caller's source alive. A user declaration's modes
/// already followed a qualified spelling, so this case is the control for the
/// `ffi` and generic cases below.
#[test]
fn should_keep_a_self_qualified_user_calls_modes() {
    let source = r#"module spec.cc.qualtake;

import { String, stringOf } from std.text;

func takeIt(take s: String): i32 {
    return i32(len(s));
}

func lendIt(borrow s: String): i32 {
    return i32(len(s));
}

let taken: i32 = spec.cc.qualtake.takeIt(stringOf("abc"));

func main(): i32 {
    let s: String = stringOf("abcd");
    let a: i32 = spec.cc.qualtake.lendIt(s);
    let b: i32 = spec.cc.qualtake.takeIt(stringOf("ab"));
    return (taken - 3) + (a - 4) + (b - 2) + i32(len(s)) - 4;
}
"#;
    let output = run_memory("qualtake", source);
    assert_no_asan("qualtake", &output);
    assert_exit("qualtake", &output, 0);
}

/// A self-qualified `ffi` call reaches its declaration and keeps its mode.
///
/// The `take` declaration owns and frees the string it receives, so the caller
/// must not release that value again; the `borrow` declaration reads the caller's
/// storage, which the caller releases at the end of the scope. The borrowed
/// initializer call reads the module's own declaration too. The module spells
/// its own name in front of both calls.
#[test]
fn should_reach_a_self_qualified_ffi_declaration() {
    let source = r#"module spec.cc.qualffi;

import { String, stringOf } from std.text;

ffi own {
    func FreeString(take s: String): void;
    func cl_string_len(borrow s: String): usize;
}

let g: usize = spec.cc.qualffi.cl_string_len(stringOf("abc"));

func main(): i32 {
    spec.cc.qualffi.FreeString(stringOf("ab"));
    let n: usize = spec.cc.qualffi.cl_string_len(stringOf("abcd"));
    return (i32(g) - 3) + (i32(n) - 4);
}
"#;
    let output = run_memory("qualffi", source);
    assert_no_asan("qualffi", &output);
    assert_exit("qualffi", &output, 0);
}

/// A self-qualified inferred generic call declares and uses its instance.
///
/// A generic function cannot be exported, so a module-qualified generic call is
/// legal only from the declaring module. The inferred argument list must key
/// the same instance a plain call would, in a body, through a module-level
/// argument, and inside another generic body.
#[test]
fn should_reach_a_self_qualified_inferred_generic() {
    let direct = r#"module spec.cc.selfgeneric;

let seed: i32 = 5;

func pick<T>(take a: T, take b: T): i32 {
    return 1;
}

func main(): i32 {
    let a: i32 = 5;
    let b: i32 = 6;
    return (spec.cc.selfgeneric.pick(a, b) - 1) + (spec.cc.selfgeneric.seed - 5);
}
"#;
    let output = run_memory("selfgeneric", direct);
    assert_no_asan("selfgeneric", &output);
    assert_exit("selfgeneric", &output, 0);

    let relayed = r#"module spec.cc.selfgenericbody;

func sink<T>(take v: T): i32 {
    return 1;
}

func relay<T>(take v: T): i32 {
    return spec.cc.selfgenericbody.sink(v);
}

func main(): i32 {
    return relay<i32>(5) - 1;
}
"#;
    let output = run_memory("selfgenericbody", relayed);
    assert_no_asan("selfgenericbody", &output);
    assert_exit("selfgenericbody", &output, 0);
}

/// A module-path binding receiver keeps its member call.
///
/// `m.x.f(...)` where `x` is the module's binding is a member call through the
/// binding, not a module-qualified function: the callee is `f`, and the
/// receiver is `x`. The body call and the initializer call both reach the
/// module's function, including a deep module path.
#[test]
fn should_reach_a_module_path_bindings_member_call() {
    let source = r#"module spec.cc.globaldot;

func peek(borrow n: i32, x: i32): i32 {
    return n + x;
}

const n: i32 = 1;

let g: i32 = spec.cc.globaldot.n.peek(5);

func main(): i32 {
    return (g - 6) + (spec.cc.globaldot.n.peek(7) - 8);
}
"#;
    let output = run_memory("globaldot", source);
    assert_no_asan("globaldot", &output);
    assert_exit("globaldot", &output, 0);

    let deep = r#"module spec.cc.deep.globaldot;

func peek(borrow n: i32, x: i32): i32 {
    return n + x;
}

const n: i32 = 1;

func main(): i32 {
    return spec.cc.deep.globaldot.n.peek(5) - 6;
}
"#;
    let output = run_memory("deepglobaldot", deep);
    assert_no_asan("deepglobaldot", &output);
    assert_exit("deepglobaldot", &output, 0);
}

/// A marked dot-call argument reports at the written value.
///
/// A pending written argument is judged after the call site fixes the type
/// arguments, and the report covers the value rather than the `take` marker in
/// front of it, exactly as the direct call's report does.
#[test]
fn should_report_a_marked_dot_call_argument_at_its_written_value() {
    let dot = r#"module spec.cc.markedpending;

struct S { x: i32; }

func g<T>(take s: S, take v: T, take w: T): i32 {
    return 1;
}

func outer<T>(s: S): i32 {
    return s.g(take 7, take "x");
}

func main(): i32 {
    let s: S = S { x: 0 };
    return outer<i32>(s) - 1;
}
"#;
    assert_rejected_once(
        "markedpending",
        dot,
        "E10200",
        &[
            "cannot convert 'static str' to 'i32' implicitly",
            "main.clo:10:29",
        ],
        &["main.clo:10:24"],
    );

    let direct = r#"module spec.cc.markeddirect;

struct S { x: i32; }

func g<T>(take s: S, take v: T, take w: T): i32 {
    return 1;
}

func call(s: S): i32 {
    return g(take s, take 7, take "x");
}

func main(): i32 {
    let s: S = S { x: 0 };
    return call(s) - 1;
}
"#;
    assert_rejected_once(
        "markeddirect",
        direct,
        "E10200",
        &[
            "cannot convert 'static str' to 'i32' implicitly",
            "main.clo:10:35",
        ],
        &["main.clo:10:30"],
    );
}

// ===== Substitution edges: conditionals, nested element keys, and element members =====

/// A conditional with opaque arms is judged with its instance's arm types.
///
/// The generic body's arms name type parameters, so the declaration alone
/// cannot decide whether they agree. At each instantiation the substituted arm
/// types decide: disagreement reports the conversion on the else arm, and a
/// null arm at a non-nullable substituted context reports the null rule at the
/// `null`. The direct conditional takes the destination's expected type in the
/// let, return, struct-field, and call-argument positions, so the same null
/// arm reports in each of them. The concrete disagreeing call is the already
/// correct twin of the first shape; the agreeing and non-null controls are
/// pinned separately.
///
/// At the base implementation the opaque and direct shapes are accepted and
/// the emitted C fails the strict build (a conditional type mismatch, or a
/// `void *` where an integer is declared).
#[test]
fn should_judge_an_opaque_conditional_at_its_instance() {
    // The concrete twin reports the disagreement at its else arm.
    let concrete = r#"module spec.cc.condconcrete;

func f(v: i32): i32 { return v; }

func main(): i32 {
    let c: bool = true;
    return f(c ? 5 : "x");
}
"#;
    assert_rejected_once(
        "condconcrete",
        concrete,
        "E10200",
        &[
            "cannot convert 'static str' to 'i32' implicitly",
            "main.clo:7:22",
        ],
        &["E10012"],
    );

    // The opaque arms disagree once the instance substitutes `i32` and `str`.
    let opaque = r#"module spec.cc.condopaque;

func id<T>(x: T): T { return x; }

func choose<T, U>(c: bool, a: T, b: U): i32 {
    return i32(id(c ? a : b));
}

func main(): i32 {
    return choose<i32, str>(true, 3, "x") - 3;
}
"#;
    assert_rejected_once(
        "condopaque",
        opaque,
        "E10200",
        &[
            "cannot convert 'str' to 'i32' implicitly",
            "main.clo:6:27",
        ],
        &["E10012"],
    );

    // The null arm at the substituted, non-nullable `i32` context reports at
    // the `null`.
    let null_arm = r#"module spec.cc.condnullarm;

func id<T>(x: T): T { return x; }

func pick(c: bool, x: i32): i32 {
    return i32(id(c ? x : null));
}

func main(): i32 { return pick(true, 5) - 5; }
"#;
    assert_rejected_once(
        "condnullarm",
        null_arm,
        "E10012",
        &["'null' is not allowed here", "main.clo:6:27"],
        &["E10200"],
    );

    // A direct let's destination reaches the conditional's null arm.
    let direct = r#"module spec.cc.conddirect;

func pick(c: bool, x: i32): i32 {
    let y: i32 = c ? x : null;
    return y;
}

func main(): i32 {
    return pick(true, 5) - 5;
}
"#;
    assert_rejected_once(
        "conddirect",
        direct,
        "E10012",
        &["'null' is not allowed here", "main.clo:4:26"],
        &["E10200"],
    );

    // The four direct contexts each report once at their own `null`.
    let contexts = r#"module spec.cc.condctx;

struct S { v: i32; }

func ret(c: bool, x: i32): i32 {
    return c ? x : null;
}

func arg(v: i32): i32 { return v; }

func main(): i32 {
    let c: bool = true;
    let x: i32 = 5;
    let a: i32 = c ? x : null;
    let s: S = S { v: c ? x : null };
    let b: i32 = arg(c ? x : null);
    return a + s.v + b + ret(c, x) - 20;
}
"#;
    let (_scratch, output, c_path) = compile_only("condctx", contexts);
    assert_ne!(
        output.status.code(),
        Some(0),
        "condctx: the front end accepted direct conditionals with a null arm at \
         four non-nullable destinations"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        4,
        "condctx: each direct context reports once; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("'null' is not allowed here").count(),
        4,
        "condctx: each report names the null rule; stderr:\n{stderr}"
    );
    for site in [
        "main.clo:6:20",
        "main.clo:14:26",
        "main.clo:15:31",
        "main.clo:16:30",
    ] {
        assert!(
            stderr.contains(site),
            "condctx: the report for the null at {site} must be present; stderr:\n{stderr}"
        );
    }
    assert!(
        !stderr.contains("error E10200:"),
        "condctx: the null rule owns these reports; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "condctx: a rejected program must not produce a translation unit"
    );
}

/// A nested element key comes from the substituted element type.
///
/// The row type `str[2]` substitutes `T`; indexing its `str` element yields a
/// byte, so the callee instance is keyed `u8` rather than leaving the
/// generated element placeholder in the symbol. The first row's first byte is
/// 97, which the program subtracts, so the run exits 0. The `len` call is the
/// control that does not index the element.
#[test]
fn should_key_a_nested_element_from_the_substituted_element() {
    let len_control = r#"module spec.cc.elemkeylen;

func id<T>(x: T): T { return x; }

func f<T>(xs: T): i32 {
    for (v in xs) {
        return i32(id(v.len));
    }
    return 0;
}

func main(): i32 {
    let a: str[2] = ["ab", "c"];
    return f<str[2]>(a) - 2;
}
"#;
    let output = run_host("elemkeylen", len_control);
    assert_exit("elemkeylen", &output, 0);

    let source = r#"module spec.cc.elemkey;

func id<T>(x: T): T { return x; }

func f<T>(xs: T): i32 {
    for (v in xs) {
        return i32(id(v[0]));
    }
    return 0;
}

func main(): i32 {
    let a: str[2] = ["ab", "c"];
    return f<str[2]>(a) - 97;
}
"#;
    let scratch = ScratchDir::new("elemkey");
    let emitted = emit_host("elemkey", source);
    assert!(
        emitted.contains("cl_id__u8"),
        "elemkey: the nested element instance is keyed from the substituted \
         `str` element; emitted C:\n{emitted}"
    );
    assert!(
        !emitted.contains("#element"),
        "elemkey: the translation unit must not carry an unbound element \
         placeholder; emitted C:\n{emitted}"
    );
    let program = build_host_c("elemkey", &scratch, &emitted);
    let output = run_with_timeout("elemkey", &program, &[]);
    assert_exit("elemkey", &output, 0);

    let (amiga_scratch, amiga) = emit_hosted_amiga("elemkeyamiga", source);
    assert!(
        amiga.contains("cl_id__u8"),
        "elemkeyamiga: the hosted-amiga instance carries the same key; \
         emitted C:\n{amiga}"
    );
    assert!(
        !amiga.contains("#element"),
        "elemkeyamiga: the hosted-amiga translation unit must not carry an \
         unbound element placeholder; emitted C:\n{amiga}"
    );
    build_hosted_amiga_c("elemkeyamiga", &amiga_scratch, &amiga);

    // The freestanding key and the absent placeholder are pinned by
    // `should_key_a_nested_element_instance_from_the_substituted_element` in
    // the implementation suite.
}

/// A substituted element member widens at a view destination.
///
/// The generic body assigns a `String` member read from a substituted element
/// to a `str` view. The instance must emit the same `cl_string_view` widening
/// as the concrete twin, so both strict builds pass and both runs read the
/// same length (2). The concrete spelling is the byte-identity control.
#[test]
fn should_widen_a_substituted_element_member_at_a_view_destination() {
    let concrete = r#"module spec.cc.elemviewconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func first(xs: S[1]): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab") }];
    return first(a) - 2;
}
"#;
    let scratch = ScratchDir::new("elemviewconcrete");
    let emitted = emit_host("elemviewconcrete", concrete);
    assert!(
        emitted.contains("s = cl_string_view(xs[0].name);"),
        "elemviewconcrete: the concrete member read widens into the view; \
         emitted C:\n{emitted}"
    );
    let program = build_host_c("elemviewconcrete", &scratch, &emitted);
    let output = run_with_timeout("elemviewconcrete", &program, &[]);
    assert_exit("elemviewconcrete", &output, 0);

    let generic = r#"module spec.cc.elemview;

import { String, stringOf } from std.text;

struct S { name: String; }

func first<T>(xs: T): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab") }];
    return first<S[1]>(a) - 2;
}
"#;
    let scratch = ScratchDir::new("elemview");
    let emitted = emit_host("elemview", generic);
    assert!(
        emitted.contains("s = cl_string_view(xs[0].name);"),
        "elemview: the substituted member read emits the concrete twin's \
         widening; emitted C:\n{emitted}"
    );
    let program = build_host_c("elemview", &scratch, &emitted);
    let output = run_with_timeout("elemview", &program, &[]);
    assert_exit("elemview", &output, 0);
}

/// A legal nullable destination keeps a direct conditional's null arm.
///
/// The expected type must reach a direct conditional's arms without rejecting
/// a null arm at a destination that can hold null. The let, return, and call
/// argument positions use `str?`, and the identical-arm and non-null-sibling
/// conditionals from the frozen control stay accepted.
#[test]
fn should_keep_a_direct_conditional_control_with_a_legal_nullable() {
    let nullable = r#"module spec.cc.condnullable;

func pass(x: str?): i32 {
    if (x == null) {
        return 0;
    }
    return 9;
}

func forward(c: bool, s: str): str? {
    return c ? s : null;
}

func pick(c: bool, s: str): i32 {
    let a: str? = c ? s : null;
    let r: i32 = pass(c ? s : null);
    let f: str? = forward(c, s);
    if (a == null) {
        return 1;
    }
    if (r != 9) {
        return 2;
    }
    if (f == null) {
        return 3;
    }
    return pass(a) + pass(f) - 18;
}

func main(): i32 {
    return pick(true, "ab");
}
"#;
    let output = run_host("condnullable", nullable);
    assert_exit("condnullable", &output, 0);

    let control = r#"module spec.cc.condcontrol;

func id<T>(x: T): T { return x; }

func choose<T, U>(c: bool, a: T, b: U): i32 {
    return i32(id<T>(c ? a : a));
}

func pick(c: bool, x: i32): i32 {
    return i32(id<i32>(c ? x : 5));
}

func main(): i32 {
    return choose<i32, str>(true, 3, "x") + pick(true, 5) - 8;
}
"#;
    let output = run_host("condcontrol", control);
    assert_exit("condcontrol", &output, 0);
}

// ===== Substitution edges: nullable members and conditional residue =====

/// A nullable substituted member guards its view widening.
///
/// The generic body reads a `String?` member from the opaque element. At the
/// `str?` destination the instance must select the destination's empty view
/// for a null member instead of constructing a view over the null handle, so
/// the null-selected run exits 0 and stays clean under AddressSanitizer. The
/// non-null selection is the control that reads the member's bytes.
///
/// At the base implementation the null-selected run dereferences the null
/// handle in `cl_string_view`.
#[test]
fn should_guard_a_nullable_substituted_member_at_a_view_destination() {
    let null_selected = r#"module spec.cc.corr.nulmemberview;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str? = xs[0].name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first<S[1]>(a) - 3;
}
"#;
    let output = run_memory("nulmemberview", null_selected);
    assert_exit("nulmemberview", &output, 0);
    assert_no_asan("nulmemberview", &output);

    let non_null = r#"module spec.cc.corr.nulmemberviewok;

import { String, stringOf } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str? = xs[0].name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("ab") }];
    return first<S[1]>(a);
}
"#;
    let output = run_host("nulmemberviewok", non_null);
    assert_exit("nulmemberviewok", &output, 0);
}

/// A nullable substituted member at a non-nullable view reports the null rule.
///
/// The member `String?` widens to `str`, so the conversion itself is legal; the
/// nullability is the failure. The instance audit reports the concrete twin's
/// possible-null rule once at the read, naming the root binding, and writes no
/// translation unit. The concrete spelling is the control.
///
/// At the base implementation the generic spelling is accepted.
#[test]
fn should_reject_a_nullable_substituted_member_at_a_nonnullable_destination() {
    let generic = r#"module spec.cc.corr.nulmemberreject;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first<S[1]>(a);
}
"#;
    assert_rejected_once(
        "nulmemberreject",
        generic,
        "E10010",
        &["value 'xs' may be null here", "main.clo:8:18"],
        &["E10200"],
    );

    let concrete = r#"module spec.cc.corr.nulmemberrejectconcrete;

import { String } from std.text;

struct S { name: String?; }

func first(xs: S[1]): i32 {
    let s: str = xs[0].name;
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first(a);
}
"#;
    assert_rejected_once(
        "nulmemberrejectconcrete",
        concrete,
        "E10010",
        &["value 'xs' may be null here", "main.clo:8:5"],
        &["E10200"],
    );
}

/// A conditional null arm reports once across its concrete instances.
///
/// One site inside a generic function is judged once per instantiation. Two
/// instances (`i32` and `u8`) both reach the same non-nullable destination, so
/// the site must not print the same report twice; one `E10012` at the `null`
/// is the result.
///
/// At the base implementation each instance prints an identical report at the
/// same span.
#[test]
fn should_report_a_two_instance_conditional_null_once() {
    let source = r#"module spec.cc.corr.twonull;

func choose<T>(c: bool, a: T): i32 {
    let y: T = c ? a : null;
    return 0;
}

func main(): i32 {
    let a: i32 = choose<i32>(true, 5);
    let u: u8 = 6;
    let b: i32 = choose<u8>(false, u);
    return a + b;
}
"#;
    assert_rejected_once(
        "twonull",
        source,
        "E10012",
        &["'null' is not allowed here", "main.clo:4:24"],
        &["E10200"],
    );
}

/// A nested conditional's null arm reports at the one `null`.
///
/// The inner `c ? x : null` takes its non-null sibling's type, so the enclosing
/// conditional does not see a null-shaped arm: only the inner `null` reports.
/// The enclosing conditional's span must not carry a second report.
///
/// At the base implementation the inner conditional types as the null shape
/// and the enclosing site reports it as a null arm.
#[test]
fn should_report_a_nested_conditional_null_once() {
    let source = r#"module spec.cc.corr.nestednull;

func id<T>(x: T): T { return x; }

func f<T>(c1: bool, c2: bool, x: T): i32 {
    return i32(id(c2 ? (c1 ? x : null) : x));
}

func main(): i32 {
    return f<i32>(true, true, 5) - 5;
}
"#;
    assert_rejected_once(
        "nestednull",
        source,
        "E10012",
        &["'null' is not allowed here", "main.clo:6:34"],
        &["E10200", "main.clo:6:24"],
    );
}

/// An unannotated conditional local keeps its sibling's type.
///
/// `let v = c ? x : null;` takes the non-null sibling's type, so the later
/// `return v` does not treat the local as the null shape. Only the `null` arm
/// reports once.
///
/// At the base implementation the local carries the null shape and the return
/// reports it a second time.
#[test]
fn should_report_an_unannotated_conditional_null_once() {
    let source = r#"module spec.cc.corr.unannotnull;

func f<T>(c: bool, x: T): T {
    let v = c ? x : null;
    return v;
}

func main(): i32 {
    return f<i32>(true, 5) - 5;
}
"#;
    assert_rejected_once(
        "unannotnull",
        source,
        "E10012",
        &["'null' is not allowed here", "main.clo:4:21"],
        &["E10200", "main.clo:5:12"],
    );
}

/// Literal and array-literal arms take the destination's type.
///
/// The destination reaches both arm checks, so an integer literal arm takes the
/// scalar destination's type, a float literal arm takes `f32`, and an array
/// literal arm takes the destination's fixed-array element type. The program
/// reads the selected values back. These shapes already held before this
/// correction, so they are controls.
#[test]
fn should_keep_the_literal_arm_controls() {
    let source = r#"module spec.cc.corr.literalarms;

func main(): i32 {
    let c: bool = true;
    let y: u8 = c ? 1 : 2;
    let f: f32 = c ? 1.5 : 2.5;
    let a: u8[2] = c ? [1, 2] : [3, 4];
    if (f != 1.5) {
        return 9;
    }
    return i32(y) + i32(a[0]) + i32(a[1]) - 4;
}
"#;
    let output = run_host("literalarms", source);
    assert_exit("literalarms", &output, 0);
}

/// A literal arm that does not fit its destination reports the fit rule.
///
/// The arm is typed by the scalar destination, so the out-of-range literal
/// reports `E10202` at the literal rather than a conversion at the conditional.
/// The shape already held before this correction, so it is a control.
#[test]
fn should_report_a_literal_arm_fit_at_its_site() {
    let source = r#"module spec.cc.corr.literalfit;

func main(): i32 {
    let c: bool = true;
    let y: u8 = c ? 1 : 300;
    return i32(y);
}
"#;
    assert_rejected_once(
        "literalfit",
        source,
        "E10202",
        &["literal does not fit 'u8'", "main.clo:5:25"],
        &["E10200"],
    );
}

// ===== Dotted member roots, FFI view crossings, and conditional-arm residue =====

/// A dotted nullable member reports the possible-null rule like its postfix twin.
///
/// The parser spells `xs.name` as a dotted read and `xs[0].name` as a postfix
/// read. Both root at the binding `xs`, so both report one `E10010` at a
/// non-nullable `str` destination when the member is `String?`. The concrete
/// twin reports the same rule at its own statement, and no translation unit is
/// written.
///
/// At the base implementation the dotted spelling is accepted because its root
/// name is not recorded.
#[test]
fn should_report_a_dotted_nullable_member_at_a_nonnullable_destination() {
    let generic = r#"module spec.cc.f2.dotreject;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str = xs.name;
    return i32(len(s));
}

func main(): i32 {
    let a: S = S { name: null };
    return first<S>(a);
}
"#;
    assert_rejected_once(
        "dotreject",
        generic,
        "E10010",
        &["value 'xs' may be null here", "main.clo:8:18"],
        &["E10200"],
    );

    let concrete = r#"module spec.cc.f2.dotrejectconcrete;

import { String } from std.text;

struct S { name: String?; }

func first(xs: S): i32 {
    let s: str = xs.name;
    return i32(len(s));
}

func main(): i32 {
    let a: S = S { name: null };
    return first(a);
}
"#;
    assert_rejected_once(
        "dotrejectconcrete",
        concrete,
        "E10010",
        &["value 'xs' may be null here", "main.clo:8:5"],
        &["E10200"],
    );
}

/// A dotted nullable member at a view destination guards its widening.
///
/// The dotted read resolves through the instance to `String?`, so the `str?`
/// store selects the destination's empty view for a null member. The
/// null-selected run exits 0 and stays clean under AddressSanitizer; the
/// non-null selection reads the member's bytes. The postfix spelling is
/// unchanged.
#[test]
fn should_guard_a_dotted_nullable_member_at_a_view_destination() {
    let null_selected = r#"module spec.cc.f2.dotview;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str? = xs.name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S = S { name: null };
    return first<S>(a) - 3;
}
"#;
    let output = run_memory("dotview", null_selected);
    assert_exit("dotview", &output, 0);
    assert_no_asan("dotview", &output);

    let non_null = r#"module spec.cc.f2.dotviewok;

import { String, stringOf } from std.text;

struct S { name: String?; }

func first<T>(xs: T): i32 {
    let s: str? = xs.name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S = S { name: stringOf("ab") };
    return first<S>(a);
}
"#;
    let output = run_host("dotviewok", non_null);
    assert_exit("dotviewok", &output, 0);
}

/// A substituted member argument crosses an uncontracted FFI view as a pointer.
///
/// The `str` parameter's ABI type is `APTR`, so the argument crosses as the
/// view's pointer member. A dotted member read and a postfix member read both
/// widen first, then cross as `(APTR)(...).ptr`. The raw-pointer and `cstring`
/// parameters keep their own crossings.
///
/// At the correction head the crossing test reads the unresolved argument type,
/// so the widened view struct reaches the stub and the C build fails.
#[test]
fn should_cross_a_substituted_member_argument_to_an_ffi_view() {
    let source = r#"module spec.cc.f2.ffiview;

import { String, stringOf } from std.text;

ffi lib {
    func RStr(s: str): i32;
    func Raw(p: raw u8): i32;
    func CText(s: cstring): i32;
}

struct S { name: String; }

func dotted<T>(xs: T): i32 {
    let n: i32 = 0;
    unsafe {
        n = RStr(xs.name);
    }
    return n;
}

func indexed<T>(xs: T): i32 {
    let n: i32 = 0;
    unsafe {
        n = RStr(xs[0].name);
    }
    return n;
}

func controls(t: str): i32 {
    let n: i32 = 0;
    let p: raw u8 = t.ptr;
    unsafe {
        n = CText("x") + Raw(p) + RStr(t);
    }
    return n;
}

func main(): i32 {
    let a: S = S { name: stringOf("hi") };
    let b: S[1] = [S { name: stringOf("hi") }];
    let t: str = "hi";
    return dotted<S>(a) + indexed<S[1]>(b) + controls(t) - 5;
}
"#;
    let stub = "typedef void *APTR;\ntypedef int LONG;\nLONG RStr(APTR s) { return s != 0; }\nLONG Raw(APTR p) { return p != 0; }\nLONG CText(char *s) { return s != 0; }\n";
    let output = run_host_with_stub("ffiview", source, stub);
    assert_exit("ffiview", &output, 0);
}

/// A nullable mixed conditional arm reports the concrete twin's conversion.
///
/// The then arm is a `String?` member and the else arm a `static str`; the two
/// do not convert to each other, so the concrete spelling reports one `E10200`
/// at the else arm. The generic instance resolves the arm and reports the same
/// conversion at the same arm, with no translation unit.
///
/// At the base implementation the unresolved arm takes the sibling's type and
/// the instance builds a conditional instead of rejecting.
#[test]
fn should_report_a_nullable_mixed_conditional_arm() {
    let generic = r#"module spec.cc.f2.condmixed;

import { String, stringOf } from std.text;

struct S { name: String?; }

func first<T>(xs: T, c: bool): i32 {
    let s: str = c ? xs[0].name : "z";
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first<S[1]>(a, false) - 1;
}
"#;
    assert_rejected_once(
        "condmixed",
        generic,
        "E10200",
        &[
            "cannot convert 'static str' to 'String?' implicitly",
            "main.clo:8:35",
        ],
        &["E10010"],
    );

    let concrete = r#"module spec.cc.f2.condmixedconcrete;

import { String, stringOf } from std.text;

struct S { name: String?; }

func first(xs: S[1], c: bool): i32 {
    let s: str = c ? xs[0].name : "z";
    return i32(len(s));
}

func main(): i32 {
    let a: S[1] = [S { name: null }];
    return first(a, false) - 1;
}
"#;
    assert_rejected_once(
        "condmixedconcrete",
        concrete,
        "E10200",
        &[
            "cannot convert 'static str' to 'String?' implicitly",
            "main.clo:8:35",
        ],
        &["E10010"],
    );
}

/// The accepted conditional-arm controls keep building.
///
/// Two `String?` members at a `str?` destination agree on their type, and a
/// non-nullable `String` member beside a `str` arm settles on `str`; both
/// selections widen their owned arms and run.
#[test]
fn should_keep_the_conditional_arm_controls() {
    let same_arm = r#"module spec.cc.f2.condsamearm;

import { String } from std.text;

struct S { name: String?; }

func first<T>(xs: T, c: bool): i32 {
    let s: str? = c ? xs[0].name : xs[1].name;
    if (s == null) {
        return 3;
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: S[2] = [S { name: null }, S { name: null }];
    return first<S[2]>(a, true) - 3;
}
"#;
    let output = run_host("condsamearm", same_arm);
    assert_exit("condsamearm", &output, 0);

    let non_null_arm = r#"module spec.cc.f2.condnonnullarm;

import { String, stringOf } from std.text;

struct S { name: String; }

func first<T>(xs: T, t: str, c: bool): i32 {
    let s: str = c ? xs.name : t;
    return i32(len(s));
}

func main(): i32 {
    let a: S = S { name: stringOf("hi") };
    return first<S>(a, "z", true) - 2;
}
"#;
    let output = run_host("condnonnullarm", non_null_arm);
    assert_exit("condnonnullarm", &output, 0);
}

/// A site whose two arms are null reports each arm once for the first instance.
///
/// Two instances reach the same site; both arms are the null literal at a
/// non-nullable substituted destination. The first instance's two arms report
/// at their own nulls and the site ends, so the site carries two reports, not
/// four.
///
/// At the base implementation each instance reports both arms.
#[test]
fn should_report_both_null_arms_once_per_instance() {
    let source = r#"module spec.cc.f2.bothnull;

func choose<T>(c: bool): i32 {
    let y: T = c ? null : null;
    return 0;
}

func main(): i32 {
    let a: i32 = choose<i32>(true);
    let u: u8 = 6;
    let b: i32 = choose<u8>(false);
    return a + b;
}
"#;
    let (_scratch, output, c_path) = compile_only("bothnull", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bothnull: the front end accepted null arms at a concrete destination"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10012:").count(),
        2,
        "bothnull: each null arm of the first instance reports once; stderr:\n{stderr}"
    );
    for site in ["main.clo:4:20", "main.clo:4:27"] {
        assert!(
            stderr.contains(site),
            "bothnull: the null at {site} must report; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "bothnull: a rejected program must not write a translation unit"
    );
}

// ===== Context typing at substitution edges =====

/// An integer literal at a generic callee's concrete parameter takes that
/// type.
///
/// The callee's type parameter is bound by the explicit argument list, while
/// its `n: usize` parameter is already concrete. The literal `0` is typed by
/// that parameter, so the call builds strictly and runs. The concrete callee
/// is the unchanged control, and a literal whose value does not fit `usize`
/// reports the fit rule once at the literal.
#[test]
fn should_context_type_a_literal_at_a_generic_concrete_parameter() {
    let concrete = r#"module spec.cc.p3litconcrete;

struct S { x: i32; }

func setS(borrow a: S[1], n: usize): void {
    a[n] = S { x: 9 };
}

func main(): i32 {
    let a: S[1] = [S { x: 1 }];
    setS(borrow a, 0);
    return a[0].x - 9;
}
"#;
    let output = run_host("p3litconcrete", concrete);
    assert_exit("p3litconcrete", &output, 0);

    let generic = r#"module spec.cc.p3litgeneric;

struct S { x: i32; }

func set<T>(borrow a: T, n: usize): void {
    a[n] = S { x: 9 };
}

func main(): i32 {
    let a: S[1] = [S { x: 1 }];
    set<S[1]>(borrow a, 0);
    return a[0].x - 9;
}
"#;
    let output = run_host("p3litgeneric", generic);
    assert_exit("p3litgeneric", &output, 0);

    let out_of_range = r#"module spec.cc.p3litrange;

struct S { x: i32; }

func set<T>(borrow a: T, n: usize): void {
    a[n] = S { x: 9 };
}

func main(): i32 {
    let a: S[1] = [S { x: 1 }];
    set<S[1]>(borrow a, 1000000000000);
    return a[0].x - 9;
}
"#;
    assert_rejected_once(
        "p3litrange",
        out_of_range,
        "E10202",
        &["literal does not fit 'usize'", "main.clo:11:25"],
        &["E10200"],
    );
}

/// A float or static string literal at a concrete generic parameter takes
/// that parameter's type.
///
/// The callee's `x: f32` and `p: cstring` parameters are concrete even though
/// `T` is generic, so the float literal takes `f32` and the static string
/// literal crosses as a C string exactly as the concrete twins' arguments do.
/// Both generic spellings build and run. An integer literal at `f32` or `bool`
/// supplies no literal context and keeps the ordinary conversion report.
#[test]
fn should_context_type_a_float_and_a_cstring_at_a_generic_concrete_parameter() {
    let float_concrete = r#"module spec.cc.p3floatcon;

func keepS(x: f32, y: bool): f32 { return x; }

func main(): i32 {
    let v: f32 = keepS(1.5, true);
    if (v != 1.5) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("p3floatcon", float_concrete);
    assert_exit("p3floatcon", &output, 0);

    let float_generic = r#"module spec.cc.p3floatgen;

func keep<T>(x: f32, y: T): f32 { return x; }

func main(): i32 {
    let v: f32 = keep(1.5, true);
    if (v != 1.5) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("p3floatgen", float_generic);
    assert_exit("p3floatgen", &output, 0);

    let cstring_concrete = r#"module spec.cc.p3cstrcon;

func keepS(p: cstring, y: bool): cstring { return p; }

func main(): i32 {
    let p: cstring = keepS("hi", true);
    if (p == null) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("p3cstrcon", cstring_concrete);
    assert_exit("p3cstrcon", &output, 0);

    let cstring_generic = r#"module spec.cc.p3cstrgen;

func keep<T>(p: cstring, y: T): cstring { return p; }

func main(): i32 {
    let p: cstring = keep("hi", true);
    if (p == null) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("p3cstrgen", cstring_generic);
    assert_exit("p3cstrgen", &output, 0);

    let integer_float = r#"module spec.cc.p3intf32;

func keep<T>(x: f32, y: T): f32 { return x; }

func main(): i32 {
    let v: f32 = keep(5, true);
    return i32(v) - 5;
}
"#;
    assert_rejected_once(
        "p3intf32",
        integer_float,
        "E10200",
        &["cannot convert 'i32' to 'f32' implicitly", "main.clo:6:23"],
        &["E10202"],
    );

    let integer_bool = r#"module spec.cc.p3intbool;

func keep<T>(x: bool, y: T): i32 { return 0; }

func main(): i32 {
    let v: i32 = keep(5, true);
    return v;
}
"#;
    assert_rejected_once(
        "p3intbool",
        integer_bool,
        "E10200",
        &["cannot convert 'i32' to 'bool' implicitly", "main.clo:6:23"],
        &["E10202"],
    );
}

/// A nullable scalar type argument is rejected at every nested position.
///
/// `?` applies only to handles, `raw`, `cstring`, and views, so an `i32?`
/// written inside a type argument or as an explicit call type argument
/// reports one `E10011` at the offending type. A declared parameter, return,
/// or binding keeps its existing report, so a program that carries every form
/// reports each of its five sites once. The nullable handle `Array<String?>`
/// is the control: it keys its own instance, builds strictly, and runs.
#[test]
fn should_report_a_nullable_type_argument_on_a_non_capable_type() {
    let control = r#"module spec.cc.p3nullablehandle;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func main(): i32 {
    let g: Array<String?> = arrayNew<String?>();
    let s: String? = stringOf("aa");
    g.push(s);
    if (len(g) == 1) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("p3nullablehandle", control);
    assert!(
        emitted.contains("ClArray__StringQ"),
        "p3nullablehandle: the nullable handle type argument keys its own \
         instance; emitted C:\n{emitted}"
    );
    let output = run_memory("p3nullablehandle", control);
    assert_no_asan("p3nullablehandle", &output);
    assert_exit("p3nullablehandle", &output, 0);

    let annotation = r#"module spec.cc.p3nullableannot;

import { Array } from std.mem;

struct H { a: Array<i32?>; }

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "p3nullableannot",
        annotation,
        "E10011",
        &["nullable type is not allowed on 'i32'", "main.clo:5:21"],
        &[],
    );

    let call_argument = r#"module spec.cc.p3nullablecallarg;

import { arrayNew } from std.mem;

func main(): i32 {
    let g = arrayNew<i32?>();
    return 0;
}
"#;
    assert_rejected_once(
        "p3nullablecallarg",
        call_argument,
        "E10011",
        &["nullable type is not allowed on 'i32'", "main.clo:6:22"],
        &[],
    );

    let nested_array = r#"module spec.cc.p3nullablenestedarray;

import { Array } from std.mem;

struct H {
    a: Array<Array<i32?>>;
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "p3nullablenestedarray",
        nested_array,
        "E10011",
        &["nullable type is not allowed on 'i32'", "main.clo:6:20"],
        &[],
    );

    let nested_wrap = r#"module spec.cc.p3nullablenestedwrap;

import { Array } from std.mem;

struct Wrap<T> { v: T; }

struct H {
    a: Array<Wrap<i32?>>;
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "p3nullablenestedwrap",
        nested_wrap,
        "E10011",
        &["nullable type is not allowed on 'i32'", "main.clo:8:19"],
        &[],
    );

    let count = r#"module spec.cc.p3nullablecount;

import { Array, arrayNew } from std.mem;

func f(x: i32?): i32? {
    return x;
}

func main(): i32 {
    let x: i32? = null;
    let g: Array<i32?> = arrayNew<i32?>();
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("p3nullablecount", count);
    assert_ne!(
        output.status.code(),
        Some(0),
        "p3nullablecount: the front end accepted nullable scalar type arguments"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10011:").count(),
        5,
        "p3nullablecount: the parameter, return, binding, annotation, and call \
         argument each report once; stderr:\n{stderr}"
    );
    for site in [
        "main.clo:5:6",
        "main.clo:5:8",
        "main.clo:10:5",
        "main.clo:11:18",
        "main.clo:11:35",
    ] {
        assert_eq!(
            stderr.matches(site).count(),
            1,
            "p3nullablecount: the site at {site} reports exactly once; \
             stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "p3nullablecount: a rejected program must not write a translation unit"
    );
}

/// Emits one host program from a fixed source path.
///
/// Two spellings that differ only inside the program then compare without
/// source-location noise from different scratch directories.
fn emit_host_at(tag: &str, scratch: &ScratchDir, source: &str, stem: &str) -> String {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join(format!("{stem}.c"));
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// A qualified type path resolves to the same instance as its last segment.
///
/// A type written as `std.mem.Array<i32>` names the same generic instance as
/// the imported `Array<i32>`: the declaration carries the instance, its
/// method resolves to that instance, and the run reads the same value. The
/// unqualified spelling is the control, and both emissions are byte for byte
/// identical.
#[test]
fn should_resolve_a_qualified_type_path_to_its_instance() {
    let unqualified = r#"module spec.cc.p3qualified;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let g: Array<i32> = arrayNew<i32>();
    g.push(1);
    return i32(len(g)) - 1;
}
"#;
    let source_scratch = ScratchDir::new("p3qualified");
    let unqualified_emitted = emit_host_at(
        "p3qualifiedunq",
        &source_scratch,
        unqualified,
        "unqualified",
    );
    assert!(
        unqualified_emitted.contains("ClArray__i32 g"),
        "p3qualifiedunq: the unqualified declaration carries the generic \
         instance; emitted C:\n{unqualified_emitted}"
    );
    let build_scratch = ScratchDir::new("p3qualifiedbuild");
    let program = build_host_c("p3qualifiedunq", &build_scratch, &unqualified_emitted);
    let output = run_with_timeout("p3qualifiedunq", &program, &[]);
    assert_exit("p3qualifiedunq", &output, 0);

    let qualified = r#"module spec.cc.p3qualified;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let g: std.mem.Array<i32> = std.mem.arrayNew<i32>();
    g.push(1);
    return i32(len(g)) - 1;
}
"#;
    let qualified_emitted = emit_host_at(
        "p3qualifiedqual",
        &source_scratch,
        qualified,
        "qualified",
    );
    let program = build_host_c("p3qualifiedqual", &build_scratch, &qualified_emitted);
    let output = run_with_timeout("p3qualifiedqual", &program, &[]);
    assert_exit("p3qualifiedqual", &output, 0);
    assert!(
        qualified_emitted.contains("ClArray__i32 g"),
        "p3qualifiedqual: the qualified declaration resolves to the same \
         generic instance; emitted C:\n{qualified_emitted}"
    );
    assert_eq!(
        qualified_emitted, unqualified_emitted,
        "p3qualifiedqual: the qualified spelling must emit the unqualified \
         control byte for byte"
    );
}

/// A qualified type path names a module declaration, not a value member.
///
/// `mem.Array` in a type position has a value-headed prefix, so it reports the
/// undeclared-name error once at the type and writes no translation unit. A
/// real module path (`std.mem.Array`) and this module's own full path stay
/// accepted; the current module's qualified spelling emits the same instance as
/// its unqualified control.
#[test]
fn should_reject_a_value_headed_qualified_type_path() {
    let value_headed = r#"module spec.cc.p3qualvalue;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let mem: i32 = 1;
    let g: mem.Array<i32> = arrayNew<i32>();
    g.push(7);
    return i32(len(g)) + mem - 8;
}
"#;
    assert_rejected_once(
        "p3qualvalue",
        value_headed,
        "E10219",
        &["'mem.Array' is not declared", "main.clo:7:12"],
        &["E10011"],
    );

    let module_path = r#"module spec.cc.p3qualmodule;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let g: std.mem.Array<i32> = std.mem.arrayNew<i32>();
    g.push(1);
    return i32(len(g)) - 1;
}
"#;
    let output = run_host("p3qualmodule", module_path);
    assert_exit("p3qualmodule", &output, 0);

    let unqualified = r#"module spec.cc.p3qualcurrent;

struct Local<T> { v: T; }

func main(): i32 {
    let g: Local<i32> = Local<i32> { v: 1 };
    return g.v - 1;
}
"#;
    let qualified = r#"module spec.cc.p3qualcurrent;

struct Local<T> { v: T; }

func main(): i32 {
    let g: spec.cc.p3qualcurrent.Local<i32> = Local<i32> { v: 1 };
    return g.v - 1;
}
"#;
    let scratch = ScratchDir::new("p3qualcurrent");
    let unqualified_emitted = emit_host_at(
        "p3qualcurrentunq",
        &scratch,
        unqualified,
        "unqualified",
    );
    let qualified_emitted = emit_host_at("p3qualcurrentqual", &scratch, qualified, "qualified");
    assert!(
        qualified_emitted.contains("ClLocal__i32 g"),
        "p3qualcurrentqual: the current module's qualified type must carry \
         the instance; emitted C:\n{qualified_emitted}"
    );
    assert_eq!(
        qualified_emitted, unqualified_emitted,
        "p3qualcurrentqual: the qualified spelling must emit the unqualified \
         control byte for byte"
    );
    let program = build_host_c("p3qualcurrent", &scratch, &qualified_emitted);
    let output = run_with_timeout("p3qualcurrent", &program, &[]);
    assert_exit("p3qualcurrent", &output, 0);
}

/// A negated integer literal is folded and fit-checked once.
///
/// The unary minus and its integer literal fold to one value, so `-5` takes
/// the destination's integer type when the folded value fits. The eight
/// destination positions (let initializer, module value, call argument,
/// return, struct field, array element, conditional arm, and assignment)
/// accept `-5` at `i16`, and `i8` accepts `-5` and `-128`. A folded value
/// outside the destination's range reports the fit rule at the literal, while
/// a non-integer destination keeps the ordinary conversion report. Positive
/// literals are unchanged.
#[test]
fn should_context_type_a_negated_literal() {
    let positive = r#"module spec.cc.p3negpositive;

func main(): i32 {
    let a: i16 = 5;
    let b: i8 = 5;
    let c: u8 = 5;
    return i32(a) + i32(b) + i32(c) - 15;
}
"#;
    let output = run_host("p3negpositive", positive);
    assert_exit("p3negpositive", &output, 0);

    let float_destination = r#"module spec.cc.p3negf32;

func main(): i32 {
    let a: f32 = -5;
    return i32(a) + 5;
}
"#;
    assert_rejected_once(
        "p3negf32",
        float_destination,
        "E10200",
        &["cannot convert 'i32' to 'f32' implicitly", "main.clo:4:18"],
        &["E10202"],
    );

    let bool_destination = r#"module spec.cc.p3negbool;

func main(): i32 {
    let a: bool = -5;
    return 0;
}
"#;
    assert_rejected_once(
        "p3negbool",
        bool_destination,
        "E10200",
        &["cannot convert 'i32' to 'bool' implicitly", "main.clo:4:19"],
        &["E10202"],
    );

    let matrix = r#"module spec.cc.p3negmatrix;

struct S { v: i16; }

let g: i16 = -5;

func took(v: i16): i32 { return i32(v); }

func ret(): i16 { return -5; }

func main(): i32 {
    let a: i16 = -5;
    let s: S = S { v: -5 };
    let arr: i16[1] = [-5];
    let c: i16 = true ? -5 : -5;
    let b: i16 = 0;
    b = -5;
    return i32(a) + i32(g) + took(-5) + i32(ret()) + i32(s.v) + i32(arr[0]) + i32(c) + i32(b) + 40;
}
"#;
    let output = run_host("p3negmatrix", matrix);
    assert_exit("p3negmatrix", &output, 0);

    let i8_five = r#"module spec.cc.p3negi8five;

func main(): i32 {
    let a: i8 = -5;
    return i32(a) + 5;
}
"#;
    let output = run_host("p3negi8five", i8_five);
    assert_exit("p3negi8five", &output, 0);

    let i8_min = r#"module spec.cc.p3negi8min;

func main(): i32 {
    let a: i8 = -128;
    return i32(a) + 128;
}
"#;
    let output = run_host("p3negi8min", i8_min);
    assert_exit("p3negi8min", &output, 0);

    let i8_below_min = r#"module spec.cc.p3negi8below;

func main(): i32 {
    let a: i8 = -129;
    return i32(a);
}
"#;
    assert_rejected_once(
        "p3negi8below",
        i8_below_min,
        "E10202",
        &["literal does not fit 'i8'", "main.clo:4:17"],
        &["E10200"],
    );

    let unsigned = r#"module spec.cc.p3negu8;

func main(): i32 {
    let a: u8 = -5;
    return i32(a);
}
"#;
    assert_rejected_once(
        "p3negu8",
        unsigned,
        "E10202",
        &["literal does not fit 'u8'", "main.clo:4:17"],
        &["E10200"],
    );
}

/// A self-qualified generic call reaches the direct-call checker.
///
/// A generic function is legal only from its declaring module, so the
/// self-qualified spelling is the qualified form. Its written type arguments,
/// arity, argument conversions, and substituted return type must match the
/// direct spelling; a fixed-array result used as a loop iterable names the
/// concrete element.
#[test]
fn should_check_a_self_qualified_generic_call() {
    let looped = r#"module spec.cc.qgenericloop;

func id<T>(x: T): T { return x; }

func main(): i32 {
    let a: i32[2] = [20, 21];
    let s: i32 = 0;
    for (v in spec.cc.qgenericloop.id<i32[2]>(a)) {
        s = s + v;
    }
    return s - 41;
}
"#;
    let output = run_memory("qgenericloop", looped);
    assert_no_asan("qgenericloop", &output);
    assert_exit("qgenericloop", &output, 0);

    let mismatch = r#"module spec.cc.qgenericmismatch;

func id<T>(x: T): T { return x; }

func main(): i32 {
    return spec.cc.qgenericmismatch.id<i32>("x");
}
"#;
    assert_rejected_once(
        "qgenericmismatch",
        mismatch,
        "E10200",
        &["cannot convert"],
        &[],
    );

    let arity = r#"module spec.cc.qgenericarity;

func id<T>(x: T): T { return x; }

func main(): i32 {
    return spec.cc.qgenericarity.id<i32>() - 5;
}
"#;
    assert_rejected_once(
        "qgenericarity",
        arity,
        "E10308",
        &["wrong number of arguments"],
        &[],
    );

    let type_count = r#"module spec.cc.qgenericcount;

func id<T>(x: T): T { return x; }

func main(): i32 {
    return spec.cc.qgenericcount.id<i32, i32>(5) - 5;
}
"#;
    assert_rejected_once(
        "qgenericcount",
        type_count,
        "E10300",
        &["wrong number of type arguments"],
        &[],
    );
}

/// A self-qualified generic call in a module initializer declares its instance.
///
/// The initializer collector resolves the module-path receiver like the body
/// collector, so `spec...id<i32>(20)` names the `i32` instance and the strict
/// build and run succeed.
#[test]
fn should_reach_a_self_qualified_generic_in_an_initializer() {
    let source = r#"module spec.cc.qgenericinit;

func id<T>(x: T): T { return x; }

let v: i32 = spec.cc.qgenericinit.id<i32>(20);

func main(): i32 { return v - 20; }
"#;
    let output = run_memory("qgenericinit", source);
    assert_no_asan("qgenericinit", &output);
    assert_exit("qgenericinit", &output, 0);
}

/// A qualified function value resolves to its emitted module symbol.
///
/// The type stage already accepts `spec...helper` as a `func(): i32`; the
/// emitter must name `cl_spec_..._helper`, not the raw dotted path. The same
/// program exercises a self-qualified private function, which the resolver
/// already accepts inside its own module.
#[test]
fn should_emit_a_qualified_function_value() {
    let source = r#"module spec.cc.qfuncvalue;

func helper(): i32 { return 5; }

func main(): i32 {
    let g: func(): i32 = spec.cc.qfuncvalue.helper;
    return g() - 5;
}
"#;
    let output = run_memory("qfuncvalue", source);
    assert_no_asan("qfuncvalue", &output);
    assert_exit("qfuncvalue", &output, 0);
}

/// A qualified compiler intrinsic or `ffi` declaration is available.
///
/// `std.mem.len`, `std.mem.rawSlice`, and `std.mem.rawStr` are compiler-known
/// declarations of the standard module; qualified access reaches them exactly
/// as the bare spellings. A cross-module `ffi` call reaches the target module's
/// declaration and runs.
#[test]
fn should_resolve_a_qualified_intrinsic_and_ffi_declaration() {
    let length = r#"module spec.cc.qintrlen;

func main(): i32 {
    let a: i32[2] = [20, 21];
    let n: usize = std.mem.len(a);
    return i32(n) + i32(a[0]) - 22;
}
"#;
    let output = run_memory("qintrlen", length);
    assert_no_asan("qintrlen", &output);
    assert_exit("qintrlen", &output, 0);

    let raw_slice = r#"module spec.cc.qintrrawslice;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let v: u8[] = unsafe std.mem.rawSlice<u8>(raw u8(&a[0]), 2);
    return i32(v[1]) - 2;
}
"#;
    let output = run_memory("qintrrawslice", raw_slice);
    assert_no_asan("qintrrawslice", &output);
    assert_exit("qintrrawslice", &output, 0);

    let raw_str = r#"module spec.cc.qintrrawstr;

func main(): i32 {
    let a: u8[2] = [65, 66];
    let s: str = unsafe std.mem.rawStr(raw u8(&a[0]), 2);
    return i32(s[1]) - 66;
}
"#;
    let output = run_memory("qintrrawstr", raw_str);
    assert_no_asan("qintrrawstr", &output);
    assert_exit("qintrrawstr", &output, 0);

    let holder = r#"module spec.cc.qffi.lib;

import { String } from std.text;

ffi api {
    func cl_string_len(borrow s: String): usize;
}
"#;
    let ffi = r#"module spec.cc.qffi.user;

import { String, stringOf } from std.text;

func main(): i32 {
    let n: usize = spec.cc.qffi.lib.cl_string_len(stringOf("abcd"));
    return i32(n) - 4;
}
"#;
    let output = run_memory_pair("qffi", holder, ffi);
    assert_no_asan("qffi", &output);
    assert_exit("qffi", &output, 0);
}

/// A module path wins over an imported value with the same head.
///
/// The holder exports `f` and a value named `spec`; the caller imports the
/// value and calls the module function. The path resolves the function, and a
/// lexical local named like the head still takes the value route.
#[test]
fn should_prefer_a_module_path_over_an_imported_value_head() {
    let holder = r#"module spec.cc.qcoll.lib;

export func f(): i32 { return 5; }
export let spec: i32 = 0;
"#;
    let caller = r#"module spec.cc.qcoll.user;

import { spec } from spec.cc.qcoll.lib;

func main(): i32 {
    return spec.cc.qcoll.lib.f() - 5 + spec;
}
"#;
    let output = run_memory_pair("qcollpath", holder, caller);
    assert_no_asan("qcollpath", &output);
    assert_exit("qcollpath", &output, 0);

    let shadow = r#"module spec.cc.qcollshadow;

struct Wrap {
    f: func(): i32;
}

func helper(): i32 { return 5; }

func main(): i32 {
    let spec: Wrap = Wrap { f: helper };
    return spec.f() - 5;
}
"#;
    let output = run_memory("qcollshadow", shadow);
    assert_no_asan("qcollshadow", &output);
    assert_exit("qcollshadow", &output, 0);
}

/// A module path and a value-headed dot call stay distinct.
///
/// A module binding followed by `.len` is a dot call on that binding, even
/// when the binding is a fixed array whose extent folds. A module-level or
/// imported value named like a standard module does not displace the standard
/// module path, and a single-segment module path wins over a same-named value.
/// A module initializer reaches the free `len` spelling too.
#[test]
fn should_keep_a_module_path_binding_dot_call_and_free_intrinsic_apart() {
    let dot = r#"module spec.cc.qdotlen;

let text: str = "abc";
let arr: i32[2] = [20, 21];

func main(): i32 {
    return i32(spec.cc.qdotlen.text.len())
        + i32(spec.cc.qdotlen.arr.len())
        + i32(spec.cc.qdotlen.arr[0]) - 25;
}
"#;
    let output = run_memory("qdotlen", dot);
    assert_no_asan("qdotlen", &output);
    assert_exit("qdotlen", &output, 0);

    let head = r#"module spec.cc.qhead;

let std: i32 = 7;

func main(): i32 {
    let a: i32[2] = [20, 21];
    let n: usize = std.mem.len(a);
    return i32(n) + i32(a[0]) - 22 + std - 7;
}
"#;
    let output = run_memory("qhead", head);
    assert_no_asan("qhead", &output);
    assert_exit("qhead", &output, 0);

    let holder = r#"module spec.cc.qheadlib;

export let std: i32 = 7;
"#;
    let imported_head = r#"module spec.cc.qheaduser;

import { std } from spec.cc.qheadlib;

func main(): i32 {
    let a: i32[2] = [20, 21];
    let n: usize = std.mem.len(a);
    return i32(n) + i32(a[0]) - 22 + std - 7;
}
"#;
    let output = run_memory_pair("qheadpair", holder, imported_head);
    assert_no_asan("qheadpair", &output);
    assert_exit("qheadpair", &output, 0);

    let single_holder = r#"module libm;

export func f(): i32 { return 5; }
export let libm: i32 = 3;
"#;
    let single_imported = r#"module spec.cc.qsingle;

import { libm } from libm;

func main(): i32 {
    return libm.f() - 5 + libm - 3;
}
"#;
    let output = run_memory_pair("qsingle", single_holder, single_imported);
    assert_no_asan("qsingle", &output);
    assert_exit("qsingle", &output, 0);

    let single_local = r#"module spec.cc.qsinglelocal;

let libm: i32 = 3;

func main(): i32 {
    return libm.f() - 5 + libm - 3;
}
"#;
    let output = run_memory_pair("qsinglelocal", single_holder, single_local);
    assert_no_asan("qsinglelocal", &output);
    assert_exit("qsinglelocal", &output, 0);

    let initializer = r#"module spec.cc.qinitlen;

let a: i32[2] = [20, 21];
let n: usize = std.mem.len(a);

func mk(): i32[2] { return [20, 21]; }

let m: usize = std.mem.len(mk());

func main(): i32 {
    return i32(n) + i32(m) + i32(a[0]) - 24;
}
"#;
    let output = run_memory("qinitlen", initializer);
    assert_no_asan("qinitlen", &output);
    assert_exit("qinitlen", &output, 0);

    let unread = r#"module spec.cc.qunread;

func main(): i32 {
    let a: i32[2] = [20, 21];
    let n: usize = std.mem.len(a);
    return i32(n) - 2;
}
"#;
    let (_scratch, output, c_path) = compile_only("qunread", unread);
    assert_eq!(
        output.status.code(),
        Some(0),
        "qunread: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("warning W10000"),
        "qunread: the folded operand must still count as a use; stderr:\n{stderr}"
    );
    assert!(c_path.exists());
    let output = run_memory("qunread", unread);
    assert_no_asan("qunread", &output);
    assert_exit("qunread", &output, 0);

    let side_effect = r#"module spec.cc.qsidefree;

let hits: i32 = 0;

func mk(): i32[2] {
    hits = hits + 1;
    return [20, 21];
}

func main(): i32 {
    let n: usize = std.mem.len(mk());
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("qsidefree", side_effect);
    assert_no_asan("qsidefree", &output);
    assert_exit("qsidefree", &output, 0);

    let row = r#"module spec.cc.qrowlen;

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [20, 21]];
    let n: usize = std.mem.len(a[0]);
    return i32(n) - 2;
}
"#;
    let output = run_memory("qrowlen", row);
    assert_no_asan("qrowlen", &output);
    assert_exit("qrowlen", &output, 0);
}

/// A bare global receiver is still a global borrow.
///
/// The module-qualified receiver shape keeps the existing member-call rule,
/// but a bare module-level receiver reports `E10038` exactly as the free twin
/// does.
#[test]
fn should_reject_a_bare_global_dot_call_receiver() {
    let source = r#"module spec.cc.qglobalborrow;

struct S { x: i32; }

let G: S = S { x: 1 };

func bump(borrow s: S): void { s.x = s.x + 1; }

func main(): i32 {
    G.bump();
    return G.x - 1;
}
"#;
    assert_rejected_once(
        "qglobalborrow",
        source,
        "E10038",
        &["cannot borrow global 'G'"],
        &[],
    );
}

/// A qualified intrinsic and `ffi` declaration are call-only.
///
/// A value position reports `E10219`; a nullable source keeps the bare
/// spelling's `E10010`; a non-generic intrinsic's type-argument count is
/// `E10300`, its argument count is `E10308`, and an intrinsic import is not
/// exported.
#[test]
fn should_judge_a_qualified_intrinsics_arguments_and_value_position() {
    let value = r#"module spec.cc.qintrvalue;

func main(): i32 {
    let g: usize = std.mem.len;
    return 0;
}
"#;
    assert_rejected_once(
        "qintrvalue",
        value,
        "E10219",
        &["'std.mem.len' is not declared", "call the function directly"],
        &[],
    );

    let ffi_holder = r#"module spec.cc.qffivalue.lib;

import { String } from std.text;

ffi api {
    func cl_string_len(borrow s: String): usize;
}
"#;
    let ffi_value = r#"module spec.cc.qffivalue.main;

func main(): i32 {
    let g = spec.cc.qffivalue.lib.cl_string_len;
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_pair("qffivalue", ffi_holder, ffi_value);
    assert_ne!(
        output.status.code(),
        Some(0),
        "qffivalue: a qualified ffi value must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10219:").count(),
        1,
        "qffivalue: expected one value-position report; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("call the function directly"),
        "qffivalue: expected the call-only help; stderr:\n{stderr}"
    );
    assert!(!c_path.exists());

    let nullable_str = r#"module spec.cc.qnullstr;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String? = stringOf("ab");
    return i32(std.mem.len(s)) - 2;
}
"#;
    assert_rejected_once(
        "qnullstr",
        nullable_str,
        "E10010",
        &["may be null here"],
        &[],
    );

    let nullable_raw = r#"module spec.cc.qnullraw;

func main(): i32 {
    let p: raw u8? = null;
    let v: u8[] = unsafe std.mem.rawSlice(p, 2);
    return 0;
}
"#;
    assert_rejected_once(
        "qnullraw",
        nullable_raw,
        "E10010",
        &["may be null here"],
        &[],
    );

    let nullable_raw_str = r#"module spec.cc.qnullrawstr;

func main(): i32 {
    let p: raw u8? = null;
    let v: str = unsafe std.mem.rawStr(p, 2);
    return 0;
}
"#;
    assert_rejected_once(
        "qnullrawstr",
        nullable_raw_str,
        "E10010",
        &["may be null here"],
        &[],
    );

    let type_args = r#"module spec.cc.qlenargs;

func main(): i32 {
    let a: i32[2] = [1, 2];
    return i32(std.mem.len<usize>(a)) - 2;
}
"#;
    assert_rejected_once(
        "qlenargs",
        type_args,
        "E10300",
        &["wrong number of type arguments for 'len'"],
        &[],
    );

    let arity = r#"module spec.cc.qlenarity;

func bump(): i32 { return 1; }

func main(): i32 {
    let a: i32[2] = [1, 2];
    return i32(std.mem.len(a, bump())) - 2;
}
"#;
    assert_rejected_once(
        "qlenarity",
        arity,
        "E10308",
        &["wrong number of arguments for 'len'"],
        &[],
    );

    let raw_str_arity = r#"module spec.cc.qrawstrarity;

func main(): i32 {
    let a: u8[1] = [65];
    let s: str = unsafe std.mem.rawStr(raw u8(&a[0]));
    return 0;
}
"#;
    assert_rejected_once(
        "qrawstrarity",
        raw_str_arity,
        "E10308",
        &["wrong number of arguments for 'rawStr'"],
        &[],
    );

    let raw_slice_types = r#"module spec.cc.qrawslicetypes;

func main(): i32 {
    let a: u8[1] = [65];
    let v: u8[] = unsafe std.mem.rawSlice<u8, i32>(raw u8(&a[0]), 1);
    return 0;
}
"#;
    assert_rejected_once(
        "qrawslicetypes",
        raw_slice_types,
        "E10300",
        &["wrong number of type arguments for 'rawSlice'"],
        &[],
    );

    let imported = r#"module spec.cc.qrawimport;

import { rawSlice } from std.mem;

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "qrawimport",
        imported,
        "E11102",
        &["not exported from module 'std.mem'"],
        &[],
    );
}

/// A module function value wins over an imported value with the same head.
///
/// The holder exports both `helper` and a value named `spec`; the caller
/// imports the value and reads the function as a value. The emitted symbol is
/// the holder's function, not a projection of the imported value. A
/// module-qualified func-typed global still calls through its binding.
#[test]
fn should_emit_a_module_function_value_over_an_imported_value_head() {
    let holder = r#"module spec.cc.qvaluable.lib;

export func helper(): i32 { return 5; }
export let spec: i32 = 0;
"#;
    let caller = r#"module spec.cc.qvaluable.user;

import { spec } from spec.cc.qvaluable.lib;

func main(): i32 {
    let g: func(): i32 = spec.cc.qvaluable.lib.helper;
    return g() - 5 + spec;
}
"#;
    let output = run_memory_pair("qvaluable", holder, caller);
    assert_no_asan("qvaluable", &output);
    assert_exit("qvaluable", &output, 0);

    let init_holder = r#"module spec.cc.qinitvalue.lib;

export func helper(): i32 { return 5; }
export let probe: i32 = 0;
"#;
    let init_caller = r#"module spec.cc.qinitvalue.user;

import { probe } from spec.cc.qinitvalue.lib;

let g: func(): i32 = spec.cc.qinitvalue.lib.helper;

func main(): i32 {
    return g() - 5 + probe;
}
"#;
    let output = run_memory_pair("qinitvalue", init_holder, init_caller);
    assert_no_asan("qinitvalue", &output);
    assert_exit("qinitvalue", &output, 0);

    let global_holder = r#"module spec.cc.qglobalfunc.lib;

func five(): i32 { return 5; }

export let g: func(): i32 = five;
"#;
    let global_caller = r#"module spec.cc.qglobalfunc.user;

func main(): i32 {
    return spec.cc.qglobalfunc.lib.g() - 5;
}
"#;
    let output = run_memory_pair("qglobalfunc", global_holder, global_caller);
    assert_no_asan("qglobalfunc", &output);
    assert_exit("qglobalfunc", &output, 0);
}

/// A folded `len` operand is evaluated only on the path that reaches it.
///
/// The structural fold reads no storage, but an effectful operand (a call, an
/// owned call, or a run-time index) still runs. A conditional, short-circuit,
/// or loop-condition operand must run only where the expression is evaluated.
#[test]
fn should_evaluate_a_folded_len_operand_on_the_reached_path() {
    let conditional_false = r#"module spec.cc.qcondfalse;

let hits: i32 = 0;

func mk(): i32[2] {
    hits = hits + 1;
    return [20, 21];
}

func main(): i32 {
    let run: bool = false;
    let n: usize = run ? std.mem.len(mk()) : 4;
    return i32(n) - 4 + hits;
}
"#;
    let output = run_memory("qcondfalse", conditional_false);
    assert_no_asan("qcondfalse", &output);
    assert_exit("qcondfalse", &output, 0);

    let conditional_true = r#"module spec.cc.qcondtrue;

let hits: i32 = 0;

func mk(): i32[2] {
    hits = hits + 1;
    return [20, 21];
}

func main(): i32 {
    let run: bool = true;
    let n: usize = run ? std.mem.len(mk()) : 4;
    return i32(n) - 2 + hits - 1;
}
"#;
    let output = run_memory("qcondtrue", conditional_true);
    assert_no_asan("qcondtrue", &output);
    assert_exit("qcondtrue", &output, 0);

    let short_circuit = r#"module spec.cc.qshort;

let hits: i32 = 0;

func mk(): i32[2] {
    hits = hits + 1;
    return [20, 21];
}

func main(): i32 {
    let run: bool = false;
    if (run && i32(std.mem.len(mk())) == 2) {
        return 9;
    }
    return hits;
}
"#;
    let output = run_memory("qshort", short_circuit);
    assert_no_asan("qshort", &output);
    assert_exit("qshort", &output, 0);

    let loop_condition = r#"module spec.cc.qloopcond;

let hits: i32 = 0;

func mk(): i32[2] {
    hits = hits + 1;
    return [20, 21];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && i32(std.mem.len(mk())) == 2) {
        i = i + 1;
    }
    return i32(i) - 2 + hits - 2;
}
"#;
    let output = run_memory("qloopcond", loop_condition);
    assert_no_asan("qloopcond", &output);
    assert_exit("qloopcond", &output, 0);

    let statement = r#"module spec.cc.qstmtlen;

let hits: i32 = 0;

func mk(): i32[2] {
    hits = hits + 1;
    return [20, 21];
}

func main(): i32 {
    std.mem.len(mk());
    return hits - 1;
}
"#;
    let output = run_memory("qstmtlen", statement);
    assert_no_asan("qstmtlen", &output);
    assert_exit("qstmtlen", &output, 0);
}

/// An `ffi` declaration may not take a compiler intrinsic's reserved name.
///
/// The declaration would emit the written C symbol and collide with the
/// compiler's own lowering and the standard-library shim, so the loader reports
/// `E20006` exactly as it does for a user function.
#[test]
fn should_reject_an_ffi_declaration_of_a_reserved_intrinsic() {
    let slice = r#"module spec.cc.qffislice;

ffi api {
    func slice(v: i32[], lo: usize, hi: usize): i32[];
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "qffislice",
        slice,
        "E20006",
        &["'slice' is a reserved built-in and cannot be redeclared"],
        &[],
    );

    let raw_str = r#"module spec.cc.qffirawstr;

ffi api {
    func rawStr(x: i32): i32;
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once(
        "qffirawstr",
        raw_str,
        "E20006",
        &["'rawStr' is a reserved built-in and cannot be redeclared"],
        &[],
    );
}

/// A module initializer applies the global-borrow call rule.
///
/// A direct call and a bare global dot receiver report `E10038` exactly as
/// their function-body twins do. A `const` global argument, a literal
/// argument, and the module-qualified receiver shape the existing
/// member-call oracle accepts stay accepted.
#[test]
fn should_apply_the_global_borrow_rule_in_a_module_initializer() {
    let direct = r#"module spec.cc.qinitdirect;

struct N { v: i32; }

func peek(borrow n: N, x: i32): i32 {
    return n.v + x;
}

let n: N = N { v: 1 };
let g: i32 = peek(n, 5);

func main(): i32 {
    return g - 6;
}
"#;
    assert_rejected_once(
        "qinitdirect",
        direct,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let dot = r#"module spec.cc.qinitdot;

struct N { v: i32; }

func peek(borrow n: N, x: i32): i32 {
    return n.v + x;
}

let n: N = N { v: 1 };
let g: i32 = n.peek(5);

func main(): i32 {
    return g - 6;
}
"#;
    assert_rejected_once(
        "qinitdot",
        dot,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let safe = r#"module spec.cc.qinitsafe;

const n: i32 = 1;

func add(a: i32, b: i32): i32 {
    return a + b;
}

let g: i32 = add(n, 2);

func main(): i32 {
    return g - 3;
}
"#;
    let output = run_memory("qinitsafe", safe);
    assert_no_asan("qinitsafe", &output);
    assert_exit("qinitsafe", &output, 0);

    let qualified = r#"module spec.cc.qinitqualified;

func peek(borrow n: i32): i32 {
    return n;
}

const n: i32 = 1;
let g: i32 = spec.cc.qinitqualified.n.peek();

func main(): i32 {
    return g - 1;
}
"#;
    let output = run_memory("qinitqualified", qualified);
    assert_no_asan("qinitqualified", &output);
    assert_exit("qinitqualified", &output, 0);

    let generic = r#"module spec.cc.qinitgeneric;

struct N { v: i32; }

func id<T>(borrow x: T): i32 {
    return 0;
}

let n: N = N { v: 1 };
let g: i32 = id(n);

func main(): i32 {
    return g;
}
"#;
    assert_rejected_once(
        "qinitgeneric",
        generic,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let field = r#"module spec.cc.qinitfield;

struct Holder { call: func(i32): i32; }

func call(x: i32): i32 { return x; }

let h: Holder = Holder { call: (x: i32) => x + 1 };
let g: i32 = h.call(5);

func main(): i32 {
    return g - 6;
}
"#;
    let output = run_memory("qinitfield", field);
    assert_no_asan("qinitfield", &output);
    assert_exit("qinitfield", &output, 0);

    let same_type_field = r#"module spec.cc.qinitfield2;

struct Holder { call: func(i32): i32; }

func call(x: Holder): i32 { return 0; }

let h: Holder = Holder { call: (x: i32) => x + 1 };
let g: i32 = h.call(5);

func main(): i32 {
    return g - 6;
}
"#;
    let output = run_memory("qinitfield2", same_type_field);
    assert_no_asan("qinitfield2", &output);
    assert_exit("qinitfield2", &output, 0);
}

/// A field call and a same-named free function stay apart in an initializer.
///
/// The receiver's projected type owns the field, so the call is a field call,
/// not a borrow of the root global: `o.inner.call(5)` accepts even when a free
/// `call` declares a borrow of `Inner`. A same-named struct loaded from another
/// module must not hide the field either: the module's own `Holder` declares
/// `call`, and the initializer accepts.
#[test]
fn should_keep_a_field_call_and_a_same_named_struct_apart_in_an_initializer() {
    let projected = r#"module spec.cc.qinitprojfield;

struct Inner { call: func(i32): i32; }
struct Outer { inner: Inner; }

func call(borrow h: Inner, x: i32): i32 { return 0; }

let o: Outer = Outer { inner: Inner { call: (x: i32) => x + 1 } };
let g: i32 = o.inner.call(5);

func main(): i32 {
    return g - 6;
}
"#;
    let output = run_memory("qinitprojfield", projected);
    assert_no_asan("qinitprojfield", &output);
    assert_exit("qinitprojfield", &output, 0);

    let holder = r#"module spec.cc.qinitstructname.holder;

export struct Holder { other: i32; }
"#;
    let main = r#"module spec.cc.qinitstructname.main;

import { Holder as OtherHolder } from spec.cc.qinitstructname.holder;

struct Holder { call: func(i32): i32; }

func call(borrow h: Holder, x: i32): i32 { return 0; }

let h: Holder = Holder { call: (x: i32) => x + 1 };
let other: OtherHolder = OtherHolder { other: 1 };
let g: i32 = h.call(5);

func main(): i32 {
    return (g - 6) + (other.other - 1);
}
"#;
    // The two loaded modules declare `Holder` with different fields. The
    // semantic type table keeps the last declaration, which is this module's,
    // so the field check accepts. The shared simple type name is the
    // pre-existing DEF-416 collision, so this case stops at the front end.
    let (_scratch, _emitted) = emit_pair("qinitstructname", holder, main);

    let order_holder = r#"module spec.cc.qinitorder;

export struct N { v: i32; }

export func peek(borrow n: N): i32 {
    return n.v;
}
"#;
    let order_main = r#"module spec.cc.qinitordermain;

import { N } from spec.cc.qinitorder;

func peek(take n: N): i32 {
    return n.v;
}

let n: N = N { v: 1 };
let g: i32 = peek(n);

func main(): i32 {
    return g - 1;
}
"#;
    let output = run_memory_pair("qinitorder", order_holder, order_main);
    assert_no_asan("qinitorder", &output);
    assert_exit("qinitorder", &output, 0);
}

/// The initializer walk resolves an aliased, fallback, generic-dot, or
/// index-projected callee before judging a global borrow.
///
/// Each program reports `E10038` once, exactly as the function-body twin does:
/// an import alias, the program-map fallback with a type alias, a generic dot
/// receiver with an explicit `borrow`, and a receiver indexed out of a global
/// array.
#[test]
fn should_apply_the_initializer_borrow_rule_through_the_resolved_callee() {
    let alias_holder = r#"module spec.cc.qinitalias;

export struct N { v: i32; }

export func peek(borrow n: N, x: i32): i32 {
    return n.v + x;
}
"#;
    let alias_main = r#"module spec.cc.qinitaliasmain;

import { N, peek as p } from spec.cc.qinitalias;

let n: N = N { v: 1 };
let g: i32 = p(n, 5);

func main(): i32 {
    return g - 6;
}
"#;
    let (_scratch, output, c_path) = compile_pair("qinitalias", alias_holder, alias_main);
    assert_rejected_output(
        "qinitalias",
        &output,
        &c_path,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let fallback_holder = r#"module spec.cc.qinitfallback;

export struct N { v: i32; }

export func peek(borrow n: N): i32 {
    return n.v;
}
"#;
    let fallback_main = r#"module spec.cc.qinitfallbackmain;

import { N as M } from spec.cc.qinitfallback;

let n: M = M { v: 1 };
let g: i32 = n.peek();

func main(): i32 {
    return g - 1;
}
"#;
    let (_scratch, output, c_path) =
        compile_pair("qinitfallback", fallback_holder, fallback_main);
    assert_rejected_output(
        "qinitfallback",
        &output,
        &c_path,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let generic_dot = r#"module spec.cc.qinitgenericdot;

struct N { v: i32; }

func id<T>(borrow x: T): i32 { return 0; }

let n: N = N { v: 1 };
let g: i32 = n.id();

func main(): i32 {
    return g;
}
"#;
    assert_rejected_once(
        "qinitgenericdot",
        generic_dot,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let index_projection = r#"module spec.cc.qinitindexproj;

struct E { v: i32; }

func peek(borrow e: E): i32 { return e.v; }

let es: E[1] = [E { v: 1 }];
let g: i32 = es[0].peek();

func main(): i32 {
    return g - 1;
}
"#;
    assert_rejected_once(
        "qinitindexproj",
        index_projection,
        "E10038",
        &["cannot borrow global 'es'"],
        &[],
    );
}

/// The initializer borrow rule judges a widened receiver, not only an exact
/// type match.
///
/// A `static str` global passed to `borrow s: str`, a projected `static str`
/// field, and a `u8` global passed to `borrow x: u16` all report `E10038`
/// exactly as the function-body twin does. The call has already been
/// type-checked, so the receiver names the free function or its widened form;
/// the walk must not drop the report when the written and declared types differ.
#[test]
fn should_apply_the_borrow_rule_to_a_widened_initializer_receiver() {
    let text = r#"module spec.cc.qinitwiden;

func sizeOf(borrow s: str): i32 { return i32(len(s)); }

let s: static str = "abc";
let g: i32 = s.sizeOf();

func main(): i32 {
    return g - 3;
}
"#;
    assert_rejected_once(
        "qinitwiden",
        text,
        "E10038",
        &["cannot borrow global 's'"],
        &[],
    );

    let projected = r#"module spec.cc.qinitwidenproj;

struct H { tag: static str; }

func sizeOf(borrow s: str): i32 { return i32(len(s)); }

let h: H = H { tag: "abc" };
let g: i32 = h.tag.sizeOf();

func main(): i32 {
    return g - 3;
}
"#;
    assert_rejected_once(
        "qinitwidenproj",
        projected,
        "E10038",
        &["cannot borrow global 'h'"],
        &[],
    );

    let scalar = r#"module spec.cc.qinitwidenscalar;

func takeW(borrow x: u16): i32 { return i32(x); }

let a: u8 = 1;
let g: i32 = a.takeW();

func main(): i32 {
    return g - 1;
}
"#;
    assert_rejected_once(
        "qinitwidenscalar",
        scalar,
        "E10038",
        &["cannot borrow global 'a'"],
        &[],
    );
}

/// BR-9 applies to a module-qualified receiver and an initializer view builder.
///
/// The frozen rule rejects every borrow of a non-`const` global. A
/// module-qualified receiver is not an exception, in a body or in an
/// initializer; a view builder's non-`const` source is a borrow; and an
/// unmarked scalar or view global argument reports. The `const` controls stay
/// accepted.
#[test]
fn should_apply_br9_to_a_qualified_receiver_and_an_initializer_view_builder() {
    let qualified_body = r#"module spec.cc.qqualbody;

struct N { v: i32; }

func peek(borrow n: N): i32 {
    return n.v;
}

let n: N = N { v: 1 };

func main(): i32 {
    return spec.cc.qqualbody.n.peek() - 1;
}
"#;
    assert_rejected_once(
        "qqualbody",
        qualified_body,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let qualified_init = r#"module spec.cc.qqualinit;

struct N { v: i32; }

func peek(borrow n: N): i32 {
    return n.v;
}

let n: N = N { v: 1 };
let g: i32 = spec.cc.qqualinit.n.peek();

func main(): i32 {
    return g - 1;
}
"#;
    assert_rejected_once(
        "qqualinit",
        qualified_init,
        "E10038",
        &["cannot borrow global 'n'"],
        &[],
    );

    let slice_source = r#"module spec.cc.qinitslice;

import { slice } from std.mem;

let xs: i32[4] = [1, 2, 3, 4];
let sl: i32[] = slice(xs, 0, 4);

func main(): i32 {
    return i32(sl[1]) - 2;
}
"#;
    assert_rejected_once(
        "qinitslice",
        slice_source,
        "E10038",
        &["cannot borrow global 'xs'"],
        &[],
    );

    let scalar = r#"module spec.cc.qinitscalarborrow;

func add1(a: i32): i32 {
    return a + 1;
}

let a: i32 = 5;
let g: i32 = a.add1();

func main(): i32 {
    return g - 6;
}
"#;
    assert_rejected_once(
        "qinitscalarborrow",
        scalar,
        "E10038",
        &["cannot borrow global 'a'"],
        &[],
    );

    let view = r#"module spec.cc.qinitviewborrow;

func first(s: str): i32 {
    return i32(len(s));
}

let s: str = "abcd";
let g: i32 = first(s);

func main(): i32 {
    return g - 4;
}
"#;
    assert_rejected_once(
        "qinitviewborrow",
        view,
        "E10038",
        &["cannot borrow global 's'"],
        &[],
    );
}
