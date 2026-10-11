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

import { slice } from std.mem;

let calls: i32 = 0;
let xs: i32[4] = [1, 2, 3, 4];

func v(s: i32[]): i32[] {
    calls = calls + 1;
    return s;
}

let sl: i32[] = slice(xs, 0, 4);
let e: i32 = v(sl)[1];

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

/// An undeclared enum member on an instance reports the unknown member.
///
/// The audit mirrors the concrete enum rule, so a declared member stays
/// accepted while an undeclared one reports the registry message.
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

