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

/// A struct-element array instance is rejected by the pinned element rule.
///
/// The opaque element binding cannot emit a copy of an aggregate element, so
/// the instance reports the loop's judgment instead of emitting invalid C (red
/// at the phase base: the program was accepted and failed the strict build).
#[test]
fn should_reject_a_struct_element_array_instance_of_an_opaque_iterable() {
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
    assert_rejected_once(
        "iterstruct",
        source,
        "E10043",
        &["type 'S[2]' cannot be iterated"],
        &[],
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
