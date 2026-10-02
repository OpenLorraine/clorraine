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
    let mut child = Command::new(&program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

/// A view built over a fixed grid must share the written row-slice name.
///
/// `slice<i32[3]>(grid, …)` and the parameter `xs: i32[][3]` describe the same
/// view. Both must spell `ClSlice_i32`, and the typedef the parameter declares
/// must cover the view the call builds.
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
        "typedef struct { i32 *ptr; usize len; } ClSlice_i32;",
    );
    assert_contains("rows", &emitted, "((ClSlice_i32){");
    assert_absent("rows", &emitted, "ClSlice_arr3_i32");
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

func apply<T>(x: T): T {
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
