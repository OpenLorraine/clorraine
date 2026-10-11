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
