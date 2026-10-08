//! Implementation tests for the symbol keys and argument decisions that carry
//! fixed-array and generic types across the C boundary.
//!
//! The specification suite proves *what* a program computes; these cases prove
//! *how* the emitter spells it. Distinct extent lists must produce distinct C
//! struct names, a substituted borrow argument must be passed by address, and
//! two emissions of one program must be byte-identical.

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
            "clc-rdcimpl-{}-{tag}-{sequence}",
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

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit. These cases are about what the
/// emitter produces for accepted programs, so a rejection is a failure.
fn emit_host(tag: &str, source: &str) -> String {
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

/// Asserts the text contains a fragment.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: the emitted C does not contain {needle:?}"
    );
}

/// Asserts the text does not contain a superseded fragment.
fn assert_absent(tag: &str, text: &str, needle: &str) {
    assert!(
        !text.contains(needle),
        "{tag}: the emitted C still contains {needle:?}"
    );
}

/// Emits the scratch directory's `main.clo` with a relative source path.
///
/// Both runs of a determinism case then name the same logical source, so the
/// emitted `#line` directives stay comparable.
fn emit_relative(scratch: &ScratchDir, tag: &str, output_name: &str) -> String {
    let output_path = scratch.path().join(output_name);
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .current_dir(scratch.path())
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            "main.clo".to_owned(),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&output_path).unwrap_or_default()
}

/// Emits, builds, and runs one host program, returning its output.
///
/// The emitted C is built with the same strict flag set the specification
/// suite uses, so a specialized emission that only builds without `-Werror`
/// still fails here.
fn run_host(tag: &str, source: &str) -> Output {
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
    run_with_timeout(tag, &program, &[])
}

/// Runs a built program with a time limit and the given environment values.
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
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

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
///
/// A leaked allocation fails the run and a double free aborts, so a clean exit
/// pins release-once for the case.
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
///
/// The pinned container's sanitizer runtime randomly fails to start, so the
/// sanitizer run is opt-in; the plain build still aborts in the C library on a
/// double release and pins the program's exit code, and the recorded
/// leak-clean probes run on the host at close-out.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
}

/// Asserts the front end rejects the program with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
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
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
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

/// Distinct extent lists must name distinct C structs.
///
/// `Grid<i32[2]>` and `Grid<i32[3]>` must not share a struct name, and the
/// folded name that dropped the extents must not appear.
#[test]
fn should_name_distinct_structs_when_fixed_array_extents_differ() {
    let source = r#"module impl.rdc.keys;

struct Grid<T> {
    value: T;
}

func main(): i32 {
    let a: Grid<i32[2]> = Grid<i32[2]> { value: [1, 2] };
    let b: Grid<i32[3]> = Grid<i32[3]> { value: [1, 2, 3] };
    return a.value[0] + b.value[0];
}
"#;
    let emitted = emit_host("keys", source);
    assert_contains("keys", &emitted, "typedef struct ClGrid__arr2_i32 ClGrid__arr2_i32;");
    assert_contains("keys", &emitted, "typedef struct ClGrid__arr3_i32 ClGrid__arr3_i32;");
    assert_absent("keys", &emitted, "ClGrid__i32");
}

/// A substituted borrow argument must be passed by address.
///
/// Under the `String` instance the callee's parameter is a pointer, so the
/// projection argument must render as the address of the caller's place, not as
/// a value copy.
#[test]
fn should_pass_the_projection_address_when_a_substituted_borrow_argument_is_used() {
    let source = r#"module impl.rdc.subst;

import { String, stringOf } from std.text;

struct Inner<T> {
    s: T;
}

struct Outer<T> {
    v: Inner<T>;
}

func sink<T>(borrow i: Inner<T>): i32 {
    return 1;
}

func probe<T>(borrow p: Outer<T>): i32 {
    return sink<T>(borrow p.v);
}

func main(): i32 {
    let p: Outer<String> = Outer<String> { v: Inner<String> { s: stringOf("x") } };
    return probe<String>(borrow p);
}
"#;
    let emitted = emit_host("subst", source);
    assert_contains("subst", &emitted, "cl_sink__String(&((*p).v))");
}

/// A view built over a fixed grid must carry the written row-slice name.
///
/// `slice<i32[3]>(grid, …)` and the parameter `xs: i32[][3]` describe the same
/// view of `i32[3]` rows. Both must spell `ClSlice_arr3_i32`, whose pointer
/// field keeps the row extent, and the typedef the parameter declares must
/// cover the view the call builds.
#[test]
fn should_share_the_slice_name_when_a_fixed_grid_is_sliced_by_rows() {
    let source = r#"module impl.rdc.rows;

import { slice } from std.mem;

func sink(xs: i32[][3]): i32 {
    return i32(len(xs));
}

func main(): i32 {
    let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    return sink(slice<i32[3]>(grid, 0, 2));
}
"#;
    let emitted = emit_host("rows", source);
    assert_contains(
        "rows",
        &emitted,
        "typedef struct { i32 (*ptr)[3]; usize len; } ClSlice_arr3_i32;",
    );
    assert_contains("rows", &emitted, "((ClSlice_arr3_i32){");
    assert_absent("rows", &emitted, "ClSlice_i32");
}

/// Two emissions of one program must be byte-identical.
///
/// Both runs name the source with the same relative path so the emitted
/// `#line` directives stay comparable.
#[test]
fn should_emit_identical_translations_when_the_same_program_is_emitted_twice() {
    let source = r#"module impl.rdc.determinism;

struct Grid<T> {
    value: T;
}

func main(): i32 {
    let a: Grid<i32[2]> = Grid<i32[2]> { value: [1, 2] };
    let b: Grid<i32[3]> = Grid<i32[3]> { value: [1, 2, 3] };
    return a.value[0] + b.value[0];
}
"#;
    let scratch = ScratchDir::new("determinism");
    std::fs::write(scratch.path().join("main.clo"), source).expect("write the program");
    let first = emit_relative(&scratch, "determinism", "first.c");
    let second = emit_relative(&scratch, "determinism", "second.c");
    assert!(
        !first.trim().is_empty(),
        "determinism: the emitted C is empty, so the emitter is not implemented yet"
    );
    assert_eq!(
        first, second,
        "determinism: the same program emitted two different translations"
    );
}

/// A generic call inside a lifted lambda must name a specialized symbol.
///
/// The lifted function's symbol and its nested call carry the enclosing
/// instance, so the emitted C names `cl_lambda_0__i32` and `cl_apply__i32` and
/// never spells the unresolved `ClT`.
#[test]
fn should_specialize_the_lifted_symbol_when_a_generic_lambda_calls_a_generic() {
    let source = r#"module impl.rdc.specialize;

func apply<T>(x: T): T {
    return x;
}

func outer<T>(x: T): T {
    let f: func(T): T = (v: T) => apply<T>(v);
    return f(x);
}

func main(): i32 {
    return outer<i32>(3);
}
"#;
    let emitted = emit_host("specialize", source);
    assert_contains("specialize", &emitted, "static i32 cl_lambda_0__i32(i32 v);");
    assert_contains("specialize", &emitted, "cl_apply__i32(v)");
    assert_absent("specialize", &emitted, "ClT");
    assert_absent("specialize", &emitted, "cl_lambda_0(i32 v)");
}

/// A capturing lambda in a generic body must name its instance environment.
///
/// The environment struct, the callable struct, the lifted symbol, and the
/// inline call all carry the enclosing instance key, and no site-only spelling
/// remains.
#[test]
fn should_name_the_environment_when_a_capturing_lambda_is_specialized() {
    let source = r#"module impl.rdc.capture;

import { String, stringOf } from std.text;

func outer<T>(x: T): T {
    return ([x] () => x)();
}

func main(): i32 {
    return i32(len(outer<String>(stringOf("abc"))));
}
"#;
    let emitted = emit_host("capture", source);
    assert_contains("capture", &emitted, "struct ClEnv_0__String {");
    assert_contains("capture", &emitted, "typedef struct ClClosure_0__String {");
    assert_contains(
        "capture",
        &emitted,
        "static ClString cl_lambda_0__String(struct ClEnv_0__String *__env);",
    );
    assert_contains(
        "capture",
        &emitted,
        "cl_lambda_0__String(&(((struct ClEnv_0__String){ x })))",
    );
    assert_absent("capture", &emitted, "struct ClEnv_0 {");
    assert_absent("capture", &emitted, "ClClosure_0 {");
}

/// A lambda in a non-generic function must keep the shared symbol.
///
/// Its body mentions no type parameter, so it stays a single shared emission
/// with the site-only name and no instance suffix.
#[test]
fn should_keep_the_shared_symbol_when_a_lambda_lives_in_a_non_generic_function() {
    let source = r#"module impl.rdc.shared;

func demo(): i32 {
    let f: func(i32): i32 = (v: i32) => v + 1;
    return f(2);
}

func main(): i32 {
    return demo();
}
"#;
    let emitted = emit_host("shared", source);
    assert_contains("shared", &emitted, "static i32 cl_lambda_0(i32 v);");
    assert_absent("shared", &emitted, "cl_lambda_0__");
}

/// Two emissions of a specialized-lambda program must be byte-identical.
///
/// The specialization order and every instance name must be deterministic.
#[test]
fn should_emit_identical_translations_when_a_generic_lambda_is_specialized() {
    let source = r#"module impl.rdc.liftdeterminism;

import { String, stringOf } from std.text;

func apply<T>(take x: T): T {
    return x;
}

func outer<T>(x: T): T {
    let f: func(T): T = (v: T) => apply<T>(v);
    return f(x);
}

func main(): i32 {
    let n: i32 = outer<i32>(3);
    return i32(len(outer<String>(stringOf("abc")))) + (n - 3);
}
"#;
    let scratch = ScratchDir::new("liftdeterminism");
    std::fs::write(scratch.path().join("main.clo"), source).expect("write the program");
    let first = emit_relative(&scratch, "liftdeterminism", "first.c");
    let second = emit_relative(&scratch, "liftdeterminism", "second.c");
    assert_contains("liftdeterminism", &first, "cl_lambda_0__i32");
    assert_contains("liftdeterminism", &first, "cl_lambda_0__String");
    assert_eq!(
        first, second,
        "liftdeterminism: the same program emitted two different translations"
    );
}

/// A specialized lifted lambda returning a fixed array must use the hidden
/// destination.
///
/// Under `T = i32[3]` the lifted function returns by value through the trailing
/// out-parameter; the body must write it, so the program builds under `-Werror`
/// and reads the first element back.
#[test]
fn should_return_the_array_when_a_fixed_array_instance_specializes_a_lambda() {
    let source = r#"module impl.rdc.arrayreturn;

func outer<T>(x: T): T {
    let f: func(T): T = (v: T) => v;
    return f(x);
}

func main(): i32 {
    let a: i32[3] = [1, 2, 3];
    let b: i32[3] = outer<i32[3]>(a);
    return b[0] - 1;
}
"#;
    let output = run_host("arrayreturn", source);
    assert_exit("arrayreturn", &output, 0);
}

/// A capture that only lives in the environment still needs specialization.
///
/// `x` is captured but never read, so the type parameter appears only in the
/// environment field. The lifted function and its `ClEnv_0__String` struct must
/// still be specialized, and the program must build and run.
#[test]
fn should_specialize_when_a_capture_is_only_in_the_environment() {
    let source = r#"module impl.rdc.unusedcapture;

import { String, stringOf } from std.text;

func outer<T>(x: T): i32 {
    return ([x] () => 3)();
}

func main(): i32 {
    return outer<String>(stringOf("abc"));
}
"#;
    let emitted = emit_host("unusedcapture", source);
    assert_contains("unusedcapture", &emitted, "struct ClEnv_0__String {");
    assert_absent("unusedcapture", &emitted, "ClT");
    let output = run_host("unusedcapture-run", source);
    assert_exit("unusedcapture-run", &output, 3);
}

/// A nested capture chain must specialize every lifted lambda it reaches.
///
/// The outer lambda captures `x` only for its inner lambda; both environments
/// and both lifted functions must carry the instance key, and the program must
/// build and run.
#[test]
fn should_specialize_when_a_nested_capture_mentions_the_parameter() {
    let source = r#"module impl.rdc.nestedcapture;

import { String, stringOf } from std.text;

func outer<T>(x: T): i32 {
    return ([x] () => ([x] () => 3)())();
}

func main(): i32 {
    return outer<String>(stringOf("abc"));
}
"#;
    let emitted = emit_host("nestedcapture", source);
    assert_contains("nestedcapture", &emitted, "struct ClEnv_0__String {");
    assert_contains("nestedcapture", &emitted, "struct ClEnv_1__String {");
    assert_absent("nestedcapture", &emitted, "ClT");
    let output = run_host("nestedcapture-run", source);
    assert_exit("nestedcapture-run", &output, 3);
}

/// A receiver call that names the parameter must specialize the lambda.
///
/// The lambda's only use of `T` is the receiver call `pick<T>()`; the lifted
/// body must carry the enclosing substitution, so the emitted C calls
/// `cl_pick__i32` and builds.
#[test]
fn should_specialize_when_a_receiver_call_mentions_the_parameter() {
    let source = r#"module impl.rdc.receiver;

func double(x: i32): i32 {
    return x * 2;
}

func pick<T>(): func(i32): i32 {
    return double;
}

func outer<T>(x: T): i32 {
    let f = () => {
        pick<T>()(2);
        return 4;
    };
    return f();
}

func main(): i32 {
    return outer<i32>(0);
}
"#;
    let emitted = emit_host("receiver", source);
    assert_contains("receiver", &emitted, "cl_pick__i32()");
    assert_absent("receiver", &emitted, "cl_pick__T");
    let output = run_host("receiver-run", source);
    assert_exit("receiver-run", &output, 4);
}

/// A returned projection must be moved out of the bound call result.
///
/// The emitted C binds `mkv()` to `__cl_owned0`, reads `.a` into the return
/// register, and empties the field, so the scope release frees only `b`.
#[test]
fn should_bind_and_move_when_a_returned_projection_is_taken() {
    let source = r#"module impl.rdc.returnproj;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func takea(): String {
    return mkv().a;
}

func main(): i32 {
    let s: String = takea();
    return i32(len(s));
}
"#;
    let emitted = emit_host("returnproj", source);
    assert_contains("returnproj", &emitted, "__cl_owned0 = cl_impl_rdc_returnproj_mkv();");
    assert_contains("returnproj", &emitted, "__cl_ret = __cl_owned0.a;");
    assert_contains("returnproj", &emitted, "__cl_owned0.a = NULL;");
    let output = run_memory("returnproj-run", source);
    assert_exit("returnproj-run", &output, 1);
}

/// A bound projection must be moved out of the call-result temporary.
///
/// `let s: String = mkv().a` binds `mkv()` to `__cl_owned0`, moves `.a` into
/// `s`, and empties the field; the sanitizer run proves `b` is released once and
/// `a` is not double-freed.
#[test]
fn should_bind_and_move_when_a_projection_is_bound() {
    let source = r#"module impl.rdc.boundproj;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func main(): i32 {
    let s: String = mkv().a;
    return i32(len(s));
}
"#;
    let emitted = emit_host("boundproj", source);
    assert_contains("boundproj", &emitted, "s = __cl_owned0.a;");
    assert_contains("boundproj", &emitted, "__cl_owned0.a = NULL;");
    let output = run_memory("boundproj-run", source);
    assert_exit("boundproj-run", &output, 1);
}

/// A projected `take` argument must be moved out of the bound temporary.
///
/// The call receives `__cl_owned0.a` and the caller empties that field after
/// the call, so the temporary releases only `b`.
#[test]
fn should_bind_and_move_when_a_projection_is_passed_to_take() {
    let source = r#"module impl.rdc.takeproj;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    return sink(mkv().a);
}
"#;
    let emitted = emit_host("takeproj", source);
    assert_contains("takeproj", &emitted, "_sink(__cl_owned0.a)");
    assert_contains("takeproj", &emitted, "__cl_owned0.a = NULL;");
    let output = run_memory("takeproj-run", source);
    assert_exit("takeproj-run", &output, 1);
}

/// An ownerless element take must empty the taken element exactly once.
///
/// The lifted call receives `__cl_tmp0[i]` and the caller empties that element
/// after the call; the temporary then releases only the other element, so the
/// sanitizer run is clean.
#[test]
fn should_move_the_element_once_when_an_ownerless_take_is_substituted() {
    let source = r#"module impl.rdc.elementtake;

import { String, stringOf } from std.text;

func sink<T>(take s: T): i32 {
    return 0;
}

func at<T>(mk: func(): T[2], i: i32): i32 {
    return sink<T>(mk()[i]);
}

func mkString(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return at<String>(mkString, 0);
}
"#;
    let emitted = emit_host("elementtake", source);
    assert_contains("elementtake", &emitted, "__cl_tmp1 = i;");
    assert_contains("elementtake", &emitted, "_sink__String(__cl_tmp0[__cl_tmp1])");
    assert_contains("elementtake", &emitted, "__cl_tmp0[__cl_tmp1] = NULL;");
    let output = run_memory("elementtake-run", source);
    assert_exit("elementtake-run", &output, 0);
}

/// A concrete ownerless element take must keep its rejection.
///
/// Without a substitution there is no caller place to empty, so the front end
/// still reports `E10026` and writes no translation unit.
#[test]
fn should_reject_when_a_concrete_ownerless_element_is_taken() {
    let source = r#"module impl.rdc.elementreject;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return 0;
}

func mkv(): String[2] {
    return [stringOf("ab"), stringOf("cde")];
}

func main(): i32 {
    return sink(mkv()[0]);
}
"#;
    assert_rejected("elementreject", source, "E10026");
}

/// A guarded ownerless take binds its call root through a deferred temporary.
///
/// The root call runs only when the guard evaluates the operand, so the binding
/// cannot be an eagerly assigned scope local: the guard emits the call into a
/// statement temporary, passes the selected member to the callee, and empties
/// the member on that path. The scope cleanup then releases the remaining
/// fields.
#[test]
fn should_bind_a_guarded_ownerless_take_through_a_deferred_temp() {
    let source = r#"module impl.rdc.guardt;

import { String, stringOf } from std.text;

struct Pair {
    a: String;
    b: String;
}

func mkv(): Pair {
    return Pair { a: stringOf("x"), b: stringOf("yz") };
}

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let flag: bool = false;
    let hit: bool = flag && sink(mkv().a) == 1;
    return hit ? 1 : 0;
}
"#;
    let emitted = emit_host("guardt", source);
    let bind_at = emitted
        .find("__cl_tmp0 = cl_impl_rdc_guardt_mkv()")
        .expect("guardt: the root call is bound to a deferred temporary");
    let call_at = emitted
        .find("_sink(__cl_tmp0.a)")
        .expect("guardt: the callee reads the member of the bound temporary");
    assert!(
        bind_at < call_at,
        "guardt: expected the root bind before the callee call"
    );
    assert_contains("guardt", &emitted, "__cl_tmp0.a = NULL");
    assert_absent("guardt", &emitted, "__cl_owned0 = ");
}

/// A guarded element with no expression drop is released before the re-store.
///
/// The hoisted statement-form element loop runs before the guarded store, and
/// the scope cleanup releases the last value, so the release site appears twice
/// and the first one precedes the store.
#[test]
fn should_hoist_the_statement_drop_when_a_guarded_element_needs_a_drop() {
    let source = r#"module impl.rdc.hoist;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("hoist", source);
    let release_at = emitted
        .find("__cl_tmp0[0].count = 0;")
        .expect("hoist: the element release site is emitted");
    let store_at = emitted
        .find("_grow(__cl_tmp0)")
        .expect("hoist: the guarded store is emitted");
    assert!(
        release_at < store_at,
        "hoist: the previous element must be released before the re-store"
    );
    assert!(
        emitted.matches("__cl_tmp0[0].count = 0;").count() >= 2,
        "hoist: the hoisted release and the scope cleanup must both run"
    );
    let output = run_memory("hoist-run", source);
    assert_exit("hoist-run", &output, 2);
}

/// A guarded struct field with no expression drop is released the same way.
///
/// The `Bag`'s `Array<String>` field drops through the same element loop, so
/// the release site must appear before the guarded store.
#[test]
fn should_hoist_the_statement_drop_when_a_guarded_struct_field_needs_a_drop() {
    let source = r#"module impl.rdc.hoistbag;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

struct Bag {
    items: Array<String>;
}

func grow(): Bag {
    let b: Bag = Bag { items: arrayNew<String>() };
    b.items.push(stringOf("ab"));
    return b;
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow().items.count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("hoistbag", source);
    let release_at = emitted
        .find("__cl_tmp0.items.count = 0;")
        .expect("hoistbag: the field release site is emitted");
    let store_at = emitted
        .find("_grow()")
        .expect("hoistbag: the guarded store is emitted");
    assert!(
        release_at < store_at,
        "hoistbag: the previous field must be released before the re-store"
    );
    let output = run_memory("hoistbag-run", source);
    assert_exit("hoistbag-run", &output, 2);
}

/// A guarded element that needs no drop keeps its expression-form drop.
///
/// `Array<i32>` has an expression-form drop, so the guarded store still carries
/// it and no statement hoist is emitted.
#[test]
fn should_keep_the_expression_drop_when_a_guarded_element_needs_no_drop() {
    let source = r#"module impl.rdc.noopdrop;

import { Array, arrayNew } from std.mem;

func grow(): Array<i32>[2] {
    let a: Array<i32> = arrayNew<i32>();
    a.push(1);
    let b: Array<i32> = arrayNew<i32>();
    b.push(2);
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let emitted = emit_host("noopdrop", source);
    assert_contains(
        "noopdrop",
        &emitted,
        "(__cl_tmp0[0].data ? (FreeBuffer(__cl_tmp0[0].data), __cl_tmp0[0].data = NULL, __cl_tmp0[0].count = 0) : 0)",
    );
    assert!(
        !emitted.contains("__cl_tmp0[0].count = 0;"),
        "noopdrop: an expression-form drop must not be hoisted"
    );
    let output = run_memory("noopdrop-run", source);
    assert_exit("noopdrop-run", &output, 2);
}

/// A materialized operand in a `?:` arm is released before the store.
///
/// The conditional arm is conditionally evaluated, so the statement drop is
/// hoisted before the assignment that holds the conditional; the release site
/// precedes it and the scope cleanup follows.
#[test]
fn should_hoist_the_statement_drop_in_a_conditional_arm() {
    let source = r#"module impl.rdc.hoistcond;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    let flag: bool = true;
    while (i < 2) {
        let x: i32 = flag ? i32(grow()[0].count) : 0;
        i = i + x;
    }
    return i;
}
"#;
    let emitted = emit_host("hoistcond", source);
    let release_at = emitted
        .find("__cl_tmp0[0].count = 0;")
        .expect("hoistcond: the element release site is emitted");
    let store_at = emitted
        .find("x = (flag ?")
        .expect("hoistcond: the conditional store is emitted");
    assert!(
        release_at < store_at,
        "hoistcond: the previous element must be released before the conditional store"
    );
    let output = run_memory("hoistcond-run", source);
    assert_exit("hoistcond-run", &output, 2);
}

/// Two emissions of a hoisted-guard program must be byte-identical.
#[test]
fn should_emit_identical_translations_when_a_guarded_element_is_hoisted() {
    let source = r#"module impl.rdc.hoistdet;

import { Array, arrayNew } from std.mem;
import { String, stringOf } from std.text;

func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow()[0].count > 0) {
        i = i + 1;
    }
    return i;
}
"#;
    let scratch = ScratchDir::new("hoistdet");
    std::fs::write(scratch.path().join("main.clo"), source).expect("write the program");
    let first = emit_relative(&scratch, "hoistdet", "first.c");
    let second = emit_relative(&scratch, "hoistdet", "second.c");
    assert_contains("hoistdet", &first, "__cl_tmp0[0].count = 0;");
    assert_eq!(
        first, second,
        "hoistdet: the same program emitted two different translations"
    );
}
