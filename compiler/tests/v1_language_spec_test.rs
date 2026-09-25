//! Runnable specification tests for the v1 language completion surface.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs it, and asserts the observable result. The cases pin the behavior the
//! frozen specification requires for generic structs and callable-bound
//! generics: a defect here only appears once the emitted C is compiled and
//! executed, so the unit-level oracle cannot see it.
//!
//! The program's `main` return value is the process exit code, so a case can
//! observe a computed integer without an `ffi` stub.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
            "clc-v1lang-{}-{tag}-{sequence}",
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

/// Compiles `files` for the host profile, builds the emitted C, runs it, and
/// returns the run output together with the emitted translation unit.
///
/// The front end and the host C compiler must both accept the input, so a
/// rejection panics instead of returning: the case is about runtime behavior,
/// not about a rejection.
fn run_and_emit(tag: &str, files: &[(&str, &str)]) -> (Output, String) {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    let mut args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
    ];
    for (name, source) in files {
        let source_path = scratch.path().join(name);
        std::fs::write(&source_path, source)
            .unwrap_or_else(|err| panic!("cannot write snippet {name}: {err}"));
        args.push(source_path.display().to_string());
    }
    let compile = run_clc(&args);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
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

    let output = Command::new(&program)
        .output()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    (output, emitted)
}

/// The number of times `needle` appears in `haystack`.
fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// A generic struct that stores a type parameter monomorphizes once per
/// distinct instantiation and the emitted C compiles and runs.
///
/// `Pair<i32>` is written twice and `Pair<u8>` once, so the emitted C must
/// declare `ClPair__i32` and `ClPair__u8` exactly once each, and the run must
/// return the summed fields (1 + 4 + 1 = 6).
#[test]
fn a_generic_struct_monomorphizes_per_instantiation() {
    let source = r#"module tests.v1.generics_pair;

struct Pair<T> {
    first: T;
    second: T;
}

func demo(): i32 {
    let a = Pair<i32> { first: 1, second: 2 };
    let b = Pair<i32> { first: 3, second: 4 };
    let c = Pair<u8> { first: 1, second: 2 };
    return a.first + b.second + i32(c.first);
}

func main(): i32 {
    return demo();
}
"#;
    let (output, emitted) = run_and_emit("g1", &[("main.clo", source)]);
    assert_eq!(
        output.status.code(),
        Some(6),
        "g1: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        count(&emitted, "struct ClPair__i32 {"),
        1,
        "g1: the emitted C must declare ClPair__i32 exactly once"
    );
    assert_eq!(
        count(&emitted, "struct ClPair__u8 {"),
        1,
        "g1: the emitted C must declare ClPair__u8 exactly once"
    );
    assert!(
        !emitted.contains("ClT "),
        "g1: an unresolved type-parameter field reached the emitted C"
    );
}

/// Nested generic arguments compose: `Pair<Pair<i32>>` resolves its inner key
/// and the emitted C compiles and runs.
#[test]
fn a_nested_generic_struct_argument_composes() {
    let source = r#"module tests.v1.generics_nested;

struct Pair<T> {
    first: T;
    second: T;
}

func demo(): i32 {
    let inner = Pair<i32> { first: 1, second: 2 };
    let outer = Pair<Pair<i32>> { first: inner, second: inner };
    return outer.first.first + outer.second.second;
}

func main(): i32 {
    return demo();
}
"#;
    let output = run_and_emit("g2", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(3),
        "g2: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A struct declared in one module and instantiated in another compiles and
/// runs.
#[test]
fn a_generic_struct_crosses_module_boundaries() {
    let declaration = r#"module tests.v1.generics_lib;

export struct Pair<T> {
    first: T;
    second: T;
}
"#;
    let use_site = r#"module tests.v1.generics_use;

import { Pair } from tests.v1.generics_lib;

func main(): i32 {
    let p = Pair<i32> { first: 7, second: 0 };
    return p.first;
}
"#;
    let output = run_and_emit("g3", &[("lib.clo", declaration), ("main.clo", use_site)]).0;
    assert_eq!(
        output.status.code(),
        Some(7),
        "g3: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A callable-bound generic instantiated with a matching function compiles and
/// runs.
#[test]
fn a_callable_bound_generic_accepts_a_matching_function() {
    let source = r#"module tests.v1.generics_callable;

func addOne(x: i32): i32 {
    return x + 1;
}

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    return apply<i32, func(i32): i32>(5, addOne);
}
"#;
    let output = run_and_emit("g4", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(6),
        "g4: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A generic struct may embed another generic struct: the embedded
/// instantiation is defined once and before the struct that uses it, and the
/// emitted C compiles and runs.
#[test]
fn a_generic_struct_embeds_another_generic_struct() {
    let source = r#"module tests.v1.generics_embed;

struct Pair<T> {
    first: T;
    second: T;
}

struct Box<T> {
    value: Pair<T>;
}

func demo(): i32 {
    let b = Box<i32> { value: Pair<i32> { first: 1, second: 2 } };
    return b.value.first;
}

func main(): i32 {
    return demo();
}
"#;
    let output = run_and_emit("g5", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(1),
        "g5: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A capture-less lambda is a plain function pointer: it binds to a concrete
/// `func(...)` type and calls through it.
#[test]
fn a_captureless_lambda_is_a_function_pointer() {
    let source = r#"module tests.v1.lambda_ptr;

func demo(): i32 {
    let inc: func(i32): i32 = (x: i32) => x + 1;
    return inc(4);
}

func main(): i32 {
    return demo();
}
"#;
    let output = run_and_emit("l10", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(5),
        "l10: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A capture-less lambda fills a concrete `func(...)` parameter of a
/// non-generic function.
#[test]
fn a_captureless_lambda_fills_a_func_parameter() {
    let source = r#"module tests.v1.lambda_param;

func apply2(x: i32, f: func(i32): i32): i32 {
    return f(f(x));
}

func demo(): i32 {
    return apply2(3, (x: i32) => x + 1);
}

func main(): i32 {
    return demo();
}
"#;
    let output = run_and_emit("l11", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(5),
        "l11: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A capturing closure fills a callable-bound parameter with the bound left to
/// inference; a duplicable capture is copied, so changing the captured binding
/// later does not change a closure that captured it earlier.
#[test]
fn a_capturing_closure_fills_a_callable_bound_by_copy() {
    let source = r#"module tests.v1.lambda_copy;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func demo(): i32 {
    let base: i32 = 10;
    let first = apply(1, [base] (x: i32) => x + base);
    base = base + 5;
    let second = apply(1, [base] (x: i32) => x + base);
    return first + second;
}

func main(): i32 {
    return demo();
}
"#;
    let output = run_and_emit("l12", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(27),
        "l12: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A non-duplicable capture is a read-only borrow: the captured owner is not
/// moved, so it is still usable after the inline call.
#[test]
fn a_non_duplicable_capture_is_a_read_only_borrow() {
    let source = r#"module tests.v1.lambda_borrow;

import { String, stringOf, bytes } from std.text;

func apply<T, F: func(T): i32>(x: T, f: F): i32 {
    return f(x);
}

func demo(): i32 {
    let s: String = stringOf("hello");
    let n = apply(0, [s] (_x: i32) => i32(bytes(s)[0]));
    return n + i32(bytes(s)[0]);
}

func main(): i32 {
    return demo();
}
"#;
    let output = run_and_emit("l13", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(208),
        "l13: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A `[borrow x]` capture is a writable borrow, so a directly invoked closure
/// mutates the captured binding in the enclosing frame.
#[test]
fn a_borrow_capture_writes_through_to_the_owner() {
    let source = r#"module tests.v1.lambda_write;

func demo(): i32 {
    let counter: i32 = 0;
    ([borrow counter] (n: i32) => { counter = counter + n; })(5);
    return counter;
}

func main(): i32 {
    return demo();
}
"#;
    let output = run_and_emit("l14", &[("main.clo", source)]).0;
    assert_eq!(
        output.status.code(),
        Some(5),
        "l14: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
