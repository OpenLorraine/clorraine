//! Runnable specification tests for module initializer integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. The memory helper
//! runs a case as an ordinary strict host build and adds leak and
//! use-after-free detection when `RDC_ASAN` is set.
//!
//! The call-resolution group pins that an initializer reaches the same
//! callables a body does: a lifted lambda callee and value, a func-typed
//! global or field carrying its own signature (including the widenings its
//! parameters drive), a generic instance, a qualified function path, a
//! module-function dot receiver, and a field that shadows an intrinsic name.
//!
//! The aggregate group pins the store, wrap, and fold forms an initializer
//! emits; the release group pins that owning temporaries release at the
//! init function's end; the checks group pins that const and immutable-view
//! writes report exactly as the body spellings report.

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

/// Compiles a two-file module program and returns the compiler output.
///
/// The holder file declares the module the program imports; both paths reach
/// the compiler, so an imported or qualified name resolves and the ordinary
/// checks run on it.
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

/// A capture-less lambda callee runs from a module initializer.
///
/// The initializer's lambda lifts to a module-level function like a body
/// lambda, so the strict build resolves the callable and the value is ready
/// before `main` (red at the batch base: the emitted call renders an
/// unresolved callable and the strict build fails).
#[test]
fn should_run_a_lambda_callee_from_a_module_initializer() {
    let source = r#"module spec.mii.lambdacallee;

let g: i32 = (() => 3)();

func main(): i32 {
    return g - 3;
}
"#;
    let output = run_memory("lambdacallee", source);
    assert_no_asan("lambdacallee", &output);
    assert_exit("lambdacallee", &output, 0);
}

/// A func-typed module global is callable from an initializer.
///
/// The call spells the global's mangled C symbol, exactly as the body path
/// does (red at the batch base: the plain source name is emitted and the
/// strict build fails on the implicit declaration).
#[test]
fn should_call_a_func_typed_global_from_a_module_initializer() {
    let source = r#"module spec.mii.funcglobal;

func ready(): i32 {
    return 7;
}

let f: func(): i32 = ready;
let g: i32 = f();

func main(): i32 {
    return g - 7;
}
"#;
    let output = run_memory("funcglobal", source);
    assert_no_asan("funcglobal", &output);
    assert_exit("funcglobal", &output, 0);
}

/// A generic function call in an initializer declares its instance.
///
/// The instance machinery walks initializer expressions like function
/// bodies, so the emitted call reaches a declared instance (red at the batch
/// base: the plain module symbol is emitted with no instance and the strict
/// build fails).
#[test]
fn should_call_a_generic_function_from_a_module_initializer() {
    let source = r#"module spec.mii.genericcall;

func pick<T>(a: T, b: T): T {
    return a;
}

let g: i32 = pick<i32>(1, 2);

func main(): i32 {
    return g - 1;
}
"#;
    let output = run_memory("genericcall", source);
    assert_no_asan("genericcall", &output);
    assert_exit("genericcall", &output, 0);
}

/// A callable-field call in an initializer passes its receiver.
///
/// The field read names the callable through the receiver global, like the
/// body spelling (red at the batch base: a free call to the field name is
/// emitted and the strict build fails).
#[test]
fn should_pass_the_receiver_of_a_callable_field_from_a_module_initializer() {
    let source = r#"module spec.mii.fieldreceiver;

func ready(): i32 {
    return 7;
}

struct Box { get: func(): i32; }

let b: Box = Box { get: ready };
let g: i32 = b.get();

func main(): i32 {
    return g - 7;
}
"#;
    let output = run_memory("fieldreceiver", source);
    assert_no_asan("fieldreceiver", &output);
    assert_exit("fieldreceiver", &output, 0);
}

/// A qualified function path in an initializer resolves like a body call.
///
/// The path names a declared function, not a value projection, so the call
/// keeps the named-call lowering and the module path never reaches the C file
/// (red at the phase head: the qualified path was rendered as a C member
/// access; the batch base built the shape).
#[test]
fn should_call_a_qualified_function_from_a_module_initializer() {
    let source = r#"module spec.mii.qualifiedcall;

func helper(): i32 {
    return 7;
}

let g: i32 = spec.mii.qualifiedcall.helper();

func main(): i32 {
    return g - 7;
}
"#;
    let output = run_memory("qualifiedcall", source);
    assert_no_asan("qualifiedcall", &output);
    assert_exit("qualifiedcall", &output, 0);
}

/// A module-function dot call in an initializer passes its receiver.
///
/// The callee is not a func-typed field, so the receiver is the first
/// argument, exactly as the body spelling passes it (red at the phase head:
/// the receiver was dropped and the strict build failed).
#[test]
fn should_pass_the_receiver_of_a_module_function_call_from_a_module_initializer() {
    let source = r#"module spec.mii.dotmethod;

func add1(a: i32): i32 {
    return a + 1;
}

let a: i32 = 5;
let g: i32 = a.add1();

func main(): i32 {
    return g - 6;
}
"#;
    let output = run_memory("dotmethod", source);
    assert_no_asan("dotmethod", &output);
    assert_exit("dotmethod", &output, 0);
}

/// An argument widens through a func-typed global's own signature.
///
/// The callable carries the signature the global was declared with, so the
/// `String` argument reaches the `str` parameter widened (red at the phase
/// head: the raw handle was passed and the strict build failed).
#[test]
fn should_widen_an_argument_through_a_func_typed_global() {
    let source = r#"module spec.mii.globalwiden;

import { stringOf } from std.text;

func lenOf(s: str): i32 {
    return i32(len(s));
}

let f: func(str): i32 = lenOf;
let g: i32 = f(stringOf("hello"));

func main(): i32 {
    return g - 5;
}
"#;
    let output = run_memory("globalwiden", source);
    assert_no_asan("globalwiden", &output);
    assert_exit("globalwiden", &output, 0);
}

/// An argument widens through a callable field's own signature.
///
/// The receiver carries the field's signature, so the conversions run exactly
/// as a body field call's (red at the phase head: the raw handle was passed
/// and the strict build failed).
#[test]
fn should_widen_an_argument_through_a_callable_field() {
    let source = r#"module spec.mii.fieldwiden;

import { stringOf } from std.text;

struct Box { get: func(str): i32; }

func lenOf(s: str): i32 {
    return i32(len(s));
}

let b: Box = Box { get: lenOf };
let g: i32 = b.get(stringOf("hello"));

func main(): i32 {
    return g - 5;
}
"#;
    let output = run_memory("fieldwiden", source);
    assert_no_asan("fieldwiden", &output);
    assert_exit("fieldwiden", &output, 0);
}

/// A callable field named like an intrinsic is called through the field.
///
/// The field shadows the built-in name exactly as it does in a body, so the
/// call returns the field's value instead of the length intrinsic's silent
/// zero (red at the phase head: the field was intercepted and the run
/// produced zero).
#[test]
fn should_call_a_field_named_like_an_intrinsic() {
    let source = r#"module spec.mii.fieldlen;

func ten(): i32 {
    return 10;
}

struct Box { len: func(): i32; }

let b: Box = Box { len: ten };
let g: i32 = b.len();

func main(): i32 {
    return g - 10;
}
"#;
    let output = run_memory("fieldlen", source);
    assert_no_asan("fieldlen", &output);
    assert_exit("fieldlen", &output, 0);
}

/// A lambda-valued func-typed global is callable from an initializer.
///
/// The initializer lambda lifts like any other, and the later call reaches
/// the lifted function through the global (red at the batch base: the global
/// held NULL and the run crashed).
#[test]
fn should_call_a_lambda_valued_global_from_a_module_initializer() {
    let source = r#"module spec.mii.lambdavalue;

import { stringOf } from std.text;

let f: func(String): i32 = (s: String) => i32(len(s));
let g: i32 = f(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let output = run_memory("lambdavalue", &source.replace(
        "let f: func(String): i32 = (s: String) => i32(len(s));\nlet g: i32 = f(stringOf(\"abc\"));",
        "let f: func(str): i32 = (s: str) => i32(len(s));\nlet g: i32 = f(\"abc\");",
    ));
    assert_no_asan("lambdavalue", &output);
    assert_exit("lambdavalue", &output, 0);
}

/// A module path spelled like an exported global is not a receiver.
///
/// The qualified call's head is a module qualifier even though another module
/// exports a global spelled like the next segment, so the call stays a named
/// call and passes no receiver (red at the fix base: the receiver path
/// resolved the exported global and prepended it, and the strict build failed).
#[test]
fn should_keep_a_module_path_out_of_a_qualified_initializer_call() {
    let holder = r#"module spec.mii.guardedpath;

export let utils: i32 = 5;
"#;
    let main = r#"module spec.mii.guardedpath.utils;

export func last(a: i32, b: i32): i32 {
    return a + b;
}

let g: i32 = spec.mii.guardedpath.utils.last(7, 1);

func main(): i32 {
    return g - 8;
}
"#;
    let (scratch, output, c_path) = compile_pair("guardedpath", holder, main);
    assert_eq!(
        output.status.code(),
        Some(0),
        "guardedpath: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).expect("guardedpath: read the emitted C");
    let program = build_host_c("guardedpath", &scratch, &emitted);
    let output = run_with_timeout("guardedpath", &program, &[]);
    assert_exit("guardedpath", &output, 0);
}

/// A struct literal with an array field from a place stores statement-form.
///
/// The fixed-array field copies with `memcpy` exactly as the body's store
/// does, so the strict build accepts the translation unit (red at the phase
/// base: the compound literal carried an array expression and the strict
/// build failed).
#[test]
fn should_store_a_struct_literal_with_an_array_field_from_a_place() {
    let source = r#"module spec.mii.structstore;

struct BoxA { arr: i32[2]; tag: i32; }

let garr: i32[2] = [1, 2];
let boxed: BoxA = BoxA { arr: garr, tag: 3 };

func main(): i32 {
    return boxed.arr[0] + boxed.arr[1] + boxed.tag - 6;
}
"#;
    let output = run_memory("structstore", source);
    assert_no_asan("structstore", &output);
    assert_exit("structstore", &output, 0);
}

/// A struct literal with an array field from a call result stores the same way.
///
/// The call fills the hidden destination and the field copies with `memcpy`,
/// so the strict build accepts and the elements read back (red at the phase
/// base: the compound literal carried the call expression).
#[test]
fn should_store_a_struct_literal_with_an_array_field_from_a_call() {
    let source = r#"module spec.mii.structstorecall;

struct BoxA { arr: i32[2]; }

func dup(): i32[2] {
    return [5, 6];
}

let boxed: BoxA = BoxA { arr: dup() };

func main(): i32 {
    return boxed.arr[0] + boxed.arr[1] - 11;
}
"#;
    let output = run_memory("structstorecall", source);
    assert_no_asan("structstorecall", &output);
    assert_exit("structstorecall", &output, 0);
}

/// A template initializer at a `str` destination receives the view wrapper.
///
/// The concatenation result reaches the `str` binding through the view
/// constructor, exactly as the body's store does, so the strict build accepts
/// and the bytes read back (red at the phase base: the raw handle was
/// assigned and the strict build failed).
#[test]
fn should_wrap_a_template_initializer_at_a_str_destination() {
    let source = r#"module spec.mii.templateview;

let n: i32 = 4;
let label: str = $"n={n}";

func main(): i32 {
    return i32(len(label)) - 3;
}
"#;
    let output = run_memory("templateview", source);
    assert_no_asan("templateview", &output);
    assert_exit("templateview", &output, 0);
}

/// A qualified dot-`len` on a fixed-array field folds to the extent.
///
/// The initializer's receiver walks the field projections like the body's
/// structural fold, so the binding holds the declared extent instead of a
/// silent zero (red at the phase base: the fold yielded `0` and the run
/// failed).
#[test]
fn should_fold_a_qualified_dot_len_in_a_module_initializer() {
    let source = r#"module spec.mii.foldlen;

struct BoxB { arr: i32[2]; }

let b: BoxB = BoxB { arr: [1, 2] };
let n: usize = b.arr.len();

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let output = run_memory("foldlen", source);
    assert_no_asan("foldlen", &output);
    assert_exit("foldlen", &output, 0);
}

/// A String-valued call at a `str` destination receives the view wrapper.
///
/// The call's owned result reaches the `str` binding through the view
/// constructor, exactly as the body's store does, so the strict build accepts
/// and the bytes read back (red at the phase base: the raw handle was
/// assigned and the strict build failed).
#[test]
fn should_wrap_a_string_call_at_a_str_destination() {
    let source = r#"module spec.mii.strviewcall;

import { stringOf } from std.text;

let s: str = stringOf("abc");

func main(): i32 {
    return i32(len(s)) - 3;
}
"#;
    let output = run_host("strviewcall", source);
    assert_exit("strviewcall", &output, 0);
}

/// A nested struct literal with an array field stores through the outer local.
///
/// The inner literal's array field copies with `memcpy` and the outer local
/// carries both levels, so the strict build accepts (red before the phase's
/// store work: the nested compound literal carried the array expression).
#[test]
fn should_store_a_nested_struct_literal_with_an_array_field() {
    let source = r#"module spec.mii.nestedstore;

struct BoxA { arr: i32[2]; }
struct Outer { inner: BoxA; }

let garr: i32[2] = [1, 2];
let o: Outer = Outer { inner: BoxA { arr: garr } };

func main(): i32 {
    return o.inner.arr[1] - 2;
}
"#;
    let output = run_memory("nestedstore", source);
    assert_no_asan("nestedstore", &output);
    assert_exit("nestedstore", &output, 0);
}

/// A conditional arm's struct literal stores under its branch guard.
///
/// Only the chosen arm's fields store, exactly as the body's conditional
/// store keeps each arm lazy (red before the phase's store work: the arm's
/// compound literal carried the array expression).
#[test]
fn should_store_a_conditional_arm_literal_under_its_guard() {
    let source = r#"module spec.mii.guardstore;

struct BoxA { arr: i32[2]; }

let garr: i32[2] = [1, 2];
let c: bool = true;
let boxed: BoxA = c ? BoxA { arr: garr } : BoxA { arr: [3, 4] };

func main(): i32 {
    return boxed.arr[0] - 1;
}
"#;
    let output = run_memory("guardstore", source);
    assert_no_asan("guardstore", &output);
    assert_exit("guardstore", &output, 0);
}

/// A literal receiver's fold never materializes the receiver.
///
/// The extent is a structural property, so the initializer emits the folded
/// constant and no receiver stores, exactly as the body's fold does (red at
/// the phase head: the materialized receiver was set but unused and the
/// strict build failed; the fold discards it).
#[test]
fn should_fold_a_literal_receiver_len_without_materializing_it() {
    let source = r#"module spec.mii.literalfold;

struct BoxA { arr: i32[2]; tag: i32; }

let n: usize = BoxA { arr: [1, 2], tag: 3 }.arr.len();

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let output = run_memory("literalfold", source);
    assert_no_asan("literalfold", &output);
    assert_exit("literalfold", &output, 0);
}

/// A folded receiver's operands are never evaluated.
///
/// The body's structural fold reads no runtime value, so a discarded
/// receiver's division does not run and no trap fires; the initializer folds
/// the same way (red at the batch base and the phase head: the materialized
/// receiver evaluated the division and trapped).
#[test]
fn should_not_evaluate_a_folded_receiver() {
    let source = r#"module spec.mii.folddiscard;

struct BoxA { arr: i32[2]; tag: i32; }

let d: i32 = 0;
let n: usize = BoxA { arr: [4 / d, 2], tag: 1 }.arr.len();

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let output = run_memory("folddiscard", source);
    assert_no_asan("folddiscard", &output);
    assert_exit("folddiscard", &output, 0);
}

/// A free `len` of a literal receiver folds without materializing it.
///
/// Both `len` spellings share one structural fold, so the initializer emits
/// the folded constant and no receiver stores (red at the fix base: the
/// materialized receiver was set but unused and the strict build failed).
#[test]
fn should_fold_a_free_len_of_a_literal_receiver() {
    let source = r#"module spec.mii.freelenfold;

struct BoxA { arr: i32[2]; tag: i32; }

let n: usize = len(BoxA { arr: [1, 2], tag: 3 }.arr);

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let output = run_memory("freelenfold", source);
    assert_no_asan("freelenfold", &output);
    assert_exit("freelenfold", &output, 0);
}

/// A free `len` never evaluates its folded receiver's operands.
///
/// The shared structural fold reads no runtime value, so a discarded
/// receiver's division does not run and no trap fires, exactly as the dot
/// spelling and the body fold.
#[test]
fn should_not_evaluate_a_free_folded_receiver() {
    let source = r#"module spec.mii.freelendiscard;

struct BoxA { arr: i32[2]; tag: i32; }

let d: i32 = 0;
let n: usize = len(BoxA { arr: [4 / d, 2], tag: 1 }.arr);

func main(): i32 {
    return i32(n) - 2;
}
"#;
    let output = run_memory("freelendiscard", source);
    assert_no_asan("freelendiscard", &output);
    assert_exit("freelendiscard", &output, 0);
}

/// An owned member projection in an initializer releases its temporary.
///
/// A call result used only by a member projection binds into an init local
/// and releases at the init function's end, so the program start leaks
/// nothing (red at the phase base: host AddressSanitizer reports the
/// `Direct leak` the row filed).
#[test]
fn should_release_an_owned_member_projection_temporary() {
    let source = r#"module spec.mii.memberrelease;

import { String, stringOf } from std.text;

struct Box { count: i32; name: String; }

func mkBox(): Box {
    return Box { count: 3, name: stringOf("n") };
}

let g: i32 = mkBox().count;

func main(): i32 {
    return g - 3;
}
"#;
    let output = run_memory("memberrelease", source);
    assert_no_asan("memberrelease", &output);
    assert_exit("memberrelease", &output, 0);
}

/// An owned `String[2]` element projection releases the array temporary.
///
/// The call result binds into an init local and releases at the init
/// function's end, so both element owners free (red at the phase base: host
/// AddressSanitizer reports two `Direct leak` records).
#[test]
fn should_release_an_owned_element_projection_temporary() {
    let source = r#"module spec.mii.elementrelease;

import { String, stringOf } from std.text;

func two(): String[2] {
    return [stringOf("a"), stringOf("bb")];
}

let g: usize = len(two()[0]);

func main(): i32 {
    return i32(g) - 1;
}
"#;
    let output = run_memory("elementrelease", source);
    assert_no_asan("elementrelease", &output);
    assert_exit("elementrelease", &output, 0);
}

/// An owned conditional arm in an initializer releases the chosen value.
///
/// The conditional's chosen `String` binds into an init local and releases
/// at the init function's end (red at the phase base: host AddressSanitizer
/// reports the chosen arm's `Direct leak`, while the body twin is clean).
#[test]
fn should_release_an_owned_conditional_arm_temporary() {
    let source = r#"module spec.mii.armrelease;

import { String, stringOf } from std.text;

let c: bool = true;
let g: i32 = i32(len(c ? stringOf("a") : stringOf("bb")));

func main(): i32 {
    return g - 1;
}
"#;
    let output = run_memory("armrelease", source);
    assert_no_asan("armrelease", &output);
    assert_exit("armrelease", &output, 0);
}

/// An owned argument to a lend parameter releases after the call.
///
/// The argument temporary binds into an init local and releases at the init
/// function's end, exactly as the body's owned-argument materialization
/// does (red at the phase base: host AddressSanitizer reports a 12-byte
/// `Direct leak`).
#[test]
fn should_release_an_owned_argument_temporary() {
    let source = r#"module spec.mii.argrelease;

import { String, stringOf } from std.text;

func first(xs: String): i32 {
    return i32(len(xs));
}

let g: i32 = first(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let output = run_memory("argrelease", source);
    assert_no_asan("argrelease", &output);
    assert_exit("argrelease", &output, 0);
}

/// A write through a const global in an initializer reports like the body.
///
/// The initializer's write checks run the function path's rule: `G++` on a
/// `const` global reports one `E10023` (red at the phase base: the write was
/// accepted and mutated the const at startup).
#[test]
fn should_reject_a_const_global_write_in_an_initializer() {
    let source = r#"module spec.mii.constwrite;

const G: i32 = 0;
let H: i32 = i32(G++);

func main(): i32 {
    return H;
}
"#;
    assert_rejected_once(
        "constwrite",
        source,
        "E10023",
        &["cannot assign to immutable 'G'"],
        &["E10042"],
    );
}

/// A write through an immutable view in an initializer reports like the body.
///
/// `S[0]++` through the `str` global reports one `E10042`, exactly as the
/// body spelling (red at the phase base: the write was accepted and emitted
/// C the strict build rejects).
#[test]
fn should_reject_an_immutable_view_write_in_an_initializer() {
    let source = r#"module spec.mii.viewwrite;

let S: str = "ab";
let T: i32 = i32(S[0]++);

func main(): i32 {
    return T;
}
"#;
    assert_rejected_once(
        "viewwrite",
        source,
        "E10042",
        &["cannot write through immutable view 'S'"],
        &["E10023"],
    );
}

/// Read-only initializer expressions stay accepted.
///
/// The checks judge writes only: a const read, an element read, and a
/// narrowed view read in initializers keep compiling (the controls for the
/// write rejections).
#[test]
fn should_accept_read_only_initializer_expressions() {
    let source = r#"module spec.mii.readonly;

const G: i32 = 7;
let S: str = "ab";
let a: i32 = G + 1;
let b: i32 = i32(S[0]);

func main(): i32 {
    return a + b - 105;
}
"#;
    let output = run_host("readonly", source);
    assert_exit("readonly", &output, 0);
}

/// A consumed conditional argument is not released by the initializer.
///
/// The callee frees the chosen arm, so the initializer must not: the walk
/// binds nothing inside a consumed argument (red at the review base: the
/// release pass freed both arms and the chosen one double-freed).
#[test]
fn should_not_release_a_consumed_conditional_argument() {
    let source = r#"module spec.mii.takecond;

import { String, stringOf } from std.text;

func consume(take xs: String): i32 {
    return i32(len(xs));
}

let c: bool = true;
let g: i32 = consume(c ? stringOf("a") : stringOf("bb"));

func main(): i32 {
    return g - 1;
}
"#;
    let output = run_memory("takecond", source);
    assert_no_asan("takecond", &output);
    assert_exit("takecond", &output, 0);
}

/// A consumed projection argument is not released by the initializer.
///
/// The callee takes the projected member out of the call result, so the
/// initializer frees nothing (red at the review base: it freed the member the
/// callee already owned).
#[test]
fn should_not_release_a_consumed_projection_argument() {
    let source = r#"module spec.mii.takeproject;

import { String, stringOf } from std.text;

func consume(take xs: String): i32 {
    return i32(len(xs));
}

struct Box { name: String; count: i32; }

func mkBox(): Box {
    return Box { name: stringOf("abc"), count: 3 };
}

let g: i32 = consume(mkBox().name);

func main(): i32 {
    return g - 3;
}
"#;
    let output = run_memory("takeproject", source);
    assert_no_asan("takeproject", &output);
    assert_exit("takeproject", &output, 0);
}

/// A value callee's modes come from its own signature, not a same-named one.
///
/// The field's `take` parameter decides consumption even though a module
/// function shares the field's name with different modes (red at the review
/// base: the name lookup supplied the wrong modes and the argument
/// double-freed).
#[test]
fn should_use_the_callable_signature_modes_for_a_name_collision() {
    let source = r#"module spec.mii.modesshadow;

import { String, stringOf } from std.text;

func consume(take xs: String): i32 {
    return i32(len(xs));
}

func go(xs: String): i32 {
    return i32(len(xs)) + 100;
}

struct H { go: func(take String): i32; }

let h: H = H { go: consume };
let g: i32 = h.go(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let output = run_memory("modesshadow", source);
    assert_no_asan("modesshadow", &output);
    assert_exit("modesshadow", &output, 0);
}

/// A `cstring` destination retains the storage its view points into.
///
/// The view keeps the backing for the program's lifetime, so nothing frees
/// it at the init function's end (red at the review base: the backing was
/// freed and `main` read freed storage).
#[test]
fn should_retain_a_cstring_backing() {
    let source = r#"module spec.mii.cstringretain;

import { toCString, stringOf } from std.text;

let c: cstring = toCString(stringOf("x"));

func main(): i32 {
    return i32(c[0]) - 120;
}
"#;
    let output = run_memory("cstringretain", source);
    assert_no_asan("cstringretain", &output);
    assert_exit("cstringretain", &output, 0);
}

/// A view-array destination retains the storage its views point into.
#[test]
fn should_retain_a_view_array_backing() {
    let source = r#"module spec.mii.viewarrayretain;

import { String, stringOf } from std.text;

let a: str[2] = [stringOf("a"), stringOf("b")];

func main(): i32 {
    return i32(a[0][0]) - 97;
}
"#;
    let output = run_memory("viewarrayretain", source);
    assert_no_asan("viewarrayretain", &output);
    assert_exit("viewarrayretain", &output, 0);
}

/// A struct field that holds views retains the storage behind them.
#[test]
fn should_retain_a_struct_view_field_backing() {
    let source = r#"module spec.mii.structviewretain;

import { stringOf } from std.text;

struct S { cells: str[1]; }

let s: S = S { cells: [stringOf("abc")] };

func main(): i32 {
    return i32(s.cells[0][0]) - 97;
}
"#;
    let output = run_memory("structviewretain", source);
    assert_no_asan("structviewretain", &output);
    assert_exit("structviewretain", &output, 0);
}

/// A structural `.len` update reports like the body.
///
/// The extent is a compile-time property with no storage, so the update
/// reports the body's own rejection (red at the review base: the initializer
/// accepted the form and emitted an invalid increment).
#[test]
fn should_reject_a_structural_len_update_in_an_initializer() {
    let source = r#"module spec.mii.lenupdate;

let a: i32[2] = [1, 2];
let x: usize = a.len++;

func main(): i32 {
    return i32(x);
}
"#;
    assert_rejected_once(
        "lenupdate",
        source,
        "E10221",
        &["left side of an assignment must be a place"],
        &["E10023", "E10042"],
    );
}

/// A body `sizeof` of a written slice type defines its typedef.
///
/// The written type argument needs the same `ClSlice_...` typedef a value
/// position needs, so the strict build accepts the unit and the size is at
/// least the pointer-and-length pair every profile stores (red at the phase
/// base: the typedef was missing and the strict build failed).
#[test]
fn should_define_a_body_slice_type_argument_typedef() {
    let source = r#"module spec.mii.bodyslice;

func size(): usize {
    let n: usize = sizeof(i32[]);
    return n;
}

func main(): i32 {
    if (size() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("bodyslice", source);
    assert_no_asan("bodyslice", &output);
    assert_exit("bodyslice", &output, 0);
}

/// A written slice type argument in a store index defines its typedef.
///
/// The run-time store index (and its bounds guard) carries the written type's
/// `sizeof`, so the collector's pointer operand yields the typedef and the
/// strict build accepts (red before the pointer walk: the typedef was missing
/// and the strict build failed).
#[test]
fn should_define_a_store_index_slice_type_argument_typedef() {
    let source = r#"module spec.mii.storeslice;

let position: usize = 0;

func main(): i32 {
    let xs: i32[4] = [1, 2, 3, 4];
    xs[position + sizeof(i32[]) - sizeof(i32[])] = 9;
    return xs[0] - 9;
}
"#;
    let output = run_memory("storeslice", source);
    assert_no_asan("storeslice", &output);
    assert_exit("storeslice", &output, 0);
}

/// An expression callee in an initializer resolves to the call's callable.
///
/// A call-result callee carries its callable value into the call exactly as
/// the body spelling does, and a conditional callee selects its callable the
/// same way, so both initializer bindings store the call's result (6) beside
/// the func-typed control (red at the phase base: the emitted initializer
/// collapsed both calls to their argument, `h = (5);`, and the run exited
/// 254).
#[test]
fn should_call_an_expression_callee_from_a_module_initializer() {
    let source = r#"module spec.mii.initcallexpr;

func addOne(a: i32): i32 {
    return a + 1;
}

func mk(): func(i32): i32 {
    return addOne;
}

let c: bool = true;
let f: func(i32): i32 = addOne;
let h: i32 = mk()(5);
let j: i32 = (c ? addOne : addOne)(5);
let k: i32 = f(5);

func main(): i32 {
    return (h - 6) + (j - 6) + (k - 6);
}
"#;
    let output = run_memory("initcallexpr", source);
    assert_no_asan("initcallexpr", &output);
    assert_exit("initcallexpr", &output, 0);
}

/// An inferred generic call in an initializer declares its instance.
///
/// A call with no written type arguments derives them from the checked
/// argument types like the body path, so the instance declares, the keyed
/// callee emits, and the run reads the call's result back; the explicit
/// spelling beside it stays a working control (red at the phase base: the
/// plain module symbol emitted with no instance and the strict build failed
/// with `implicit declaration of function 'cl_..._pick'`).
#[test]
fn should_infer_a_generic_call_instance_in_a_module_initializer() {
    let source = r#"module spec.mii.initgenericinfer;

func pick<T>(a: T, b: T): T {
    return a;
}

let g: i32 = pick(1, 2);
let h: i32 = pick<i32>(3, 4);

func main(): i32 {
    return (g - 1) + (h - 3);
}
"#;
    let output = run_memory("initgenericinfer", source);
    assert_no_asan("initgenericinfer", &output);
    assert_exit("initgenericinfer", &output, 0);
}

/// A guarded right-side bind releases inside its declaration block.
///
/// The `&&` right side declares its owned temporary inside the guard block,
/// so the release must run there while the unguarded bind keeps the ordinary
/// end-of-init release; both readings prove true (red at the phase base: the
/// release was emitted after the block and the strict build failed with
/// `'__cl_itmp1' undeclared`).
#[test]
fn should_release_a_guarded_owned_bind_inside_its_block() {
    let source = r#"module spec.mii.initguardrelease;

import { String, stringOf, bytes } from std.text;

let yes: bool = true;

func firstByte(s: String): u8 {
    return bytes(s)[0];
}

let hit: bool = yes && (firstByte(stringOf("ab")) == 97);
let plain: bool = firstByte(stringOf("ab")) == 97;

func main(): i32 {
    if (hit && plain) {
        return 0;
    }
    return 1;
}
"#;
    let output = run_memory("initguardrelease", source);
    assert_no_asan("initguardrelease", &output);
    assert_exit("initguardrelease", &output, 0);
}

/// An inferred generic view-builder call in an initializer declares its
/// instance.
///
/// The view-builder spelling with no written type arguments records its
/// instance exactly like the explicit spelling, so the keyed callee emits and
/// the run reads the indexed elements back (red at the phase base: the plain
/// view-builder symbol emitted with no instance, the strict build failed with
/// `implicit declaration of function 'cl_..._viewOf'` and an invalid
/// initializer).
#[test]
fn should_infer_a_generic_view_builder_instance_in_a_module_initializer() {
    let source = r#"module spec.mii.initviewinfer;

import { slice } from std.mem;

let xs: i32[4] = [1, 2, 3, 4];

func viewOf<T>(v: T[]): T[] {
    return v;
}

let e: i32 = viewOf(slice(xs, 0, 4))[1];
let f: i32 = viewOf<i32>(slice(xs, 0, 4))[2];

func main(): i32 {
    return (e - 2) + (f - 3);
}
"#;
    let output = run_memory("initviewinfer", source);
    assert_no_asan("initviewinfer", &output);
    assert_exit("initviewinfer", &output, 0);
}

/// A nullable view conditional keeps each arm's own nullness.
///
/// Each arm constructs its own view: the owned arm wraps its handle and the
/// null arm carries the zero view, so the destination's null test answers
/// nine and the taken arm's length read answers three, at the module level
/// and in a body. The both-arms-owned conditional is the non-null control
/// (red at the phase base: the null selection wrapped `NULL` after the
/// conditional and the run segfaulted with exit 139 at `cl_string_ptr(NULL)`).
#[test]
fn should_keep_the_nullness_of_a_nullable_view_conditional() {
    let source = r#"module spec.mii.nullarmview;

import { stringOf } from std.text;

let c: bool = false;
let s: str? = c ? stringOf("abc") : null;

func main(): i32 {
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}
"#;
    let output = run_memory("nullarmview-null", source);
    assert_no_asan("nullarmview-null", &output);
    assert_exit("nullarmview-null", &output, 9);

    let taken = source.replace("let c: bool = false;", "let c: bool = true;");
    let output = run_memory("nullarmview-taken", &taken);
    assert_no_asan("nullarmview-taken", &output);
    assert_exit("nullarmview-taken", &output, 3);

    let body = r#"module spec.mii.nullarmbody;

import { stringOf } from std.text;

func code(c: bool): i32 {
    let s: str? = c ? stringOf("abc") : null;
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}

func main(): i32 {
    return code(false);
}
"#;
    let output = run_memory("nullarmbody-null", body);
    assert_no_asan("nullarmbody-null", &output);
    assert_exit("nullarmbody-null", &output, 9);

    let body_taken = body.replace("return code(false);", "return code(true);");
    let output = run_memory("nullarmbody-taken", &body_taken);
    assert_no_asan("nullarmbody-taken", &output);
    assert_exit("nullarmbody-taken", &output, 3);

    let control = r#"module spec.mii.nullarmviewctl;

import { stringOf } from std.text;

let c: bool = false;
let s: str? = c ? stringOf("ab") : stringOf("xyz");

func main(): i32 {
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}
"#;
    let output = run_memory("nullarmview-ctl", control);
    assert_no_asan("nullarmview-ctl", &output);
    assert_exit("nullarmview-ctl", &output, 3);
}

/// A member projection of an owned call result keeps its binding and release.
///
/// The projection's root call binds into an init local or a statement local —
/// in the let, return, argument, and module-initializer spellings — and the
/// owned member releases with it, so every projection reads back and the
/// program start releases everything (green control: verified strict-clean,
/// exit 0, and ASan-clean at the phase base).
#[test]
fn should_keep_a_member_projection_of_an_owned_call_result() {
    let source = r#"module spec.mii.memberprojcontrol;

import { String, stringOf } from std.text;

struct Box { count: i32; name: String; }

func mkBox(): Box {
    return Box { count: 3, name: stringOf("n") };
}

func byArg(x: i32): i32 {
    return x;
}

let g: i32 = mkBox().count;

func letSpelling(): i32 {
    let v: i32 = mkBox().count;
    return v;
}

func returnSpelling(): i32 {
    return mkBox().count;
}

func argSpelling(): i32 {
    return byArg(mkBox().count);
}

func main(): i32 {
    return (g - 3) + (letSpelling() - 3) + (returnSpelling() - 3) + (argSpelling() - 3);
}
"#;
    let output = run_memory("memberprojcontrol", source);
    assert_no_asan("memberprojcontrol", &output);
    assert_exit("memberprojcontrol", &output, 0);
}
