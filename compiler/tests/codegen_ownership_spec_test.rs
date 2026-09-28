//! Runnable specification tests for writes through a `borrow` element view.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning flags,
//! runs the binary, and asserts the process exit code. Every program walks a
//! fixed array with `for (borrow p in xs)`, assigns to fields of an element
//! whose type owns a `String`, and then observes a duplicated integer field
//! through the array. `main` returns the observed sum, so the exit code carries
//! the result.
//!
//! The behavior under test is that an element view aliases its element, so
//! every write reaches the array, and that the old value of an assigned owned
//! field is released exactly once. A defect in either half only appears after
//! the emitted C is compiled and executed, so the unit-level oracle cannot see
//! it.

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
            "clc-owncodegen-{}-{tag}-{sequence}",
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

/// Compiles one program for the host profile, builds the emitted C under the
/// fixed warning flags, runs the binary, and returns its output.
///
/// The front end and the host C compiler must both accept the input, so a
/// rejection panics instead of returning: each case is about runtime behavior,
/// not about a rejection. An empty emitted translation unit would make both the
/// build and the run meaningless, so it is a panic too.
fn compile_build_and_run(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
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

    Command::new(&program)
        .output()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"))
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

/// A write through a `borrow` element view reaches the owning array.
///
/// `fill` assigns both fields of every element; `main` observes only the
/// duplicated `n` field, so a write that lands on a loop-local copy shows up as
/// `1 + 2` instead of `4 + 4`.
#[test]
fn should_alias_the_element_when_a_borrow_element_view_is_assigned() {
    let source = r#"module codegen.own.st1;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

func fill(borrow ps: P[2]): void {
    for (borrow p in ps) {
        p.name = stringOf("zz");
        p.n = 4;
    }
}

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    fill(borrow ps);
    return ps[0].n + ps[1].n;
}
"#;
    let output = compile_build_and_run("st1", source);
    assert_exit("st1", &output, 8);
}

/// A `return` from the loop body writes back the element written so far.
///
/// The first element's write must reach the array before `fill` returns, so the
/// observed sum is `4 + 2`. A view that is only a loop-local copy leaves the
/// array at `1 + 2`.
#[test]
fn should_write_back_the_written_element_when_the_borrow_loop_returns_early() {
    let source = r#"module codegen.own.st2;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

func fill(borrow ps: P[2]): void {
    for (borrow p in ps) {
        p.name = stringOf("zz");
        p.n = 4;
        return;
    }
}

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    fill(borrow ps);
    return ps[0].n + ps[1].n;
}
"#;
    let output = compile_build_and_run("st2", source);
    assert_exit("st2", &output, 6);
}

/// A `break` after the second write keeps both writes in the array.
///
/// The loop walks exactly two elements; the second assignment is immediately
/// followed by `break`, so both elements must read back as four.
#[test]
fn should_keep_both_writes_when_the_borrow_loop_breaks_after_the_second_write() {
    let source = r#"module codegen.own.st3;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

func fill(borrow ps: P[2]): void {
    let written: i32 = 0;
    for (borrow p in ps) {
        p.name = stringOf("zz");
        p.n = 4;
        written = written + 1;
        if (written == 2) {
            break;
        }
    }
}

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    fill(borrow ps);
    return ps[0].n + ps[1].n;
}
"#;
    let output = compile_build_and_run("st3", source);
    assert_exit("st3", &output, 8);
}

/// A `continue` before an assignment leaves the skipped element unchanged.
///
/// The first iteration jumps to the next element without writing, so only the
/// second element becomes four; the observed sum is `1 + 4`. The skipped
/// iteration must not release or corrupt the untouched element.
#[test]
fn should_write_only_the_second_element_when_the_first_iteration_continues() {
    let source = r#"module codegen.own.st4;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

func fill(borrow ps: P[2]): void {
    let visits: i32 = 0;
    for (borrow p in ps) {
        visits = visits + 1;
        if (visits == 1) {
            continue;
        }
        p.name = stringOf("zz");
        p.n = 4;
    }
}

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    fill(borrow ps);
    return ps[0].n + ps[1].n;
}
"#;
    let output = compile_build_and_run("st4", source);
    assert_exit("st4", &output, 5);
}

/// Replacing the same owned field twice keeps the last value and releases the
/// replaced value exactly once.
///
/// Each iteration installs two `String` values in a row on the element's `name`
/// field and then sets `n = 8`; the observed sum is `8 + 8`, and a run that
/// releases the first temporary twice aborts instead of returning.
#[test]
fn should_keep_the_last_value_when_a_borrow_element_field_is_replaced_twice() {
    let source = r#"module codegen.own.st5;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

func fill(borrow ps: P[2]): void {
    for (borrow p in ps) {
        p.name = stringOf("zz");
        p.name = stringOf("qqqq");
        p.n = 8;
    }
}

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    fill(borrow ps);
    return ps[0].n + ps[1].n;
}
"#;
    let output = compile_build_and_run("st5", source);
    assert_exit("st5", &output, 16);
}

/// Two borrow loops over two arrays write each array independently.
///
/// The first fill writes `n = 3` into every `xs` element and the second writes
/// `n = 4` into every `ys` element, so the observed sum across all four fields
/// is `3 + 3 + 4 + 4`.
#[test]
fn should_write_both_arrays_when_two_borrow_element_loops_run_in_sequence() {
    let source = r#"module codegen.own.st6;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

func fillThrees(borrow ps: P[2]): void {
    for (borrow p in ps) {
        p.name = stringOf("t");
        p.n = 3;
    }
}

func fillFours(borrow ps: P[2]): void {
    for (borrow p in ps) {
        p.name = stringOf("f");
        p.n = 4;
    }
}

func main(): i32 {
    let xs: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    let ys: P[2] = [P { name: stringOf("c"), n: 1 }, P { name: stringOf("d"), n: 2 }];
    fillThrees(borrow xs);
    fillFours(borrow ys);
    return xs[0].n + xs[1].n + ys[0].n + ys[1].n;
}
"#;
    let output = compile_build_and_run("st6", source);
    assert_exit("st6", &output, 14);
}

/// A borrow loop over a struct field's array writes the nested elements.
///
/// `fill` walks `bag.items` and sets `n = 5` on each element; the caller reads
/// the two item fields back and observes `5 + 5`. The write must reach the
/// struct's storage rather than a temporary copy of the field.
#[test]
fn should_write_nested_elements_when_a_borrow_loop_walks_a_struct_field() {
    let source = r#"module codegen.own.st7;

import { String, stringOf } from std.text;

struct P {
    name: String;
    n: i32;
}

struct Bag {
    items: P[2];
}

func fill(borrow bag: Bag): void {
    for (borrow p in bag.items) {
        p.n = 5;
    }
}

func main(): i32 {
    let bag: Bag = Bag { items: [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }] };
    fill(borrow bag);
    return bag.items[0].n + bag.items[1].n;
}
"#;
    let output = compile_build_and_run("st7", source);
    assert_exit("st7", &output, 10);
}

/// A local in the function body shadows the parameter of the same name.
///
/// `doubled` reads the parameter `LIMIT`, then the local `LIMIT` shadows it;
/// the returned value is the local, so `pick(3)` exits with `6`. A resolver
/// that keys locals only by source name loses the parameter's value here.
#[test]
fn should_return_the_local_when_a_local_shadows_a_parameter() {
    let source = r#"module codegen.own.st8;

func pick(LIMIT: i32): i32 {
    let doubled: i32 = LIMIT * 2;
    let LIMIT: i32 = doubled;
    return LIMIT;
}

func main(): i32 {
    return pick(i32(3));
}
"#;
    let output = compile_build_and_run("st8", source);
    assert_exit("st8", &output, 6);
}

/// A local declared inside an `if` block shadows the outer local of the same
/// name only inside that block.
///
/// The block's `x` is the one-character string; after the block the outer `x`
/// is visible again, so `main` exits with the outer length `4`.
#[test]
fn should_read_the_outer_string_when_an_if_block_declares_a_shadowing_local() {
    let source = r#"module codegen.own.st9;

import { String, stringOf } from std.text;

func vlen(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let x: String = stringOf("aaaa");
    if (vlen(x) == 4) {
        let x: String = stringOf("b");
        if (vlen(x) == 1) {
        }
    }
    return vlen(x);
}
"#;
    let output = compile_build_and_run("st9", source);
    assert_exit("st9", &output, 4);
}

/// Two sibling blocks may each declare a local with the same name and a
/// different type.
///
/// The first block adds the integer `x = 7` to `total`; the second adds the
/// length of the string `x`, so `main` exits with `8`. A code generator that
/// keys locals by name alone collides the two variables.
#[test]
fn should_keep_both_locals_when_sibling_blocks_shadow_the_same_name() {
    let source = r#"module codegen.own.st10;

import { String, stringOf } from std.text;

func vlen(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let total: i32 = 0;
    if (total == 0) {
        let x: i32 = 7;
        total = total + x;
    }
    if (total == 7) {
        let x: String = stringOf("q");
        total = total + vlen(x);
    }
    return total;
}
"#;
    let output = compile_build_and_run("st10", source);
    assert_exit("st10", &output, 8);
}

/// The parameter `n` keeps its value across a loop that declares a local `n`.
///
/// The loop runs for `i = 0` and `i = 1`; each iteration adds the shadowing
/// local (`n + i`) to `acc`, so `acc` becomes `5` then `11`. After the loop
/// the parameter `n` is visible again and `acc + n` exits with `16`.
#[test]
fn should_read_the_parameter_after_the_loop_when_a_loop_body_shadows_it() {
    let source = r#"module codegen.own.st11;

func f(n: i32): i32 {
    let acc: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        let n: i32 = n + i;
        acc = acc + n;
        i = i + 1;
    }
    return acc + n;
}

func main(): i32 {
    return f(i32(5));
}
"#;
    let output = compile_build_and_run("st11", source);
    assert_exit("st11", &output, 16);
}

/// A local in a `defer` block shadows the outer local only inside the block.
///
/// The deferred block declares its own string `x`; the function returns the
/// outer integer `x` after the defer body, so `main` exits with `5`.
#[test]
fn should_return_the_outer_value_when_a_defer_block_shadows_it() {
    let source = r#"module codegen.own.st12;

import { String, stringOf } from std.text;

func vlen(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let x: i32 = 5;
    defer {
        let x: String = stringOf("inner");
        if (vlen(x) > 0) {
        }
    }
    return x;
}
"#;
    let output = compile_build_and_run("st12", source);
    assert_exit("st12", &output, 5);
}

/// A `const` local shadows the parameter after its own initializer.
///
/// The initializer `n + 1` reads the parameter `n`, then the const `n`
/// shadows it; `f(3)` exits with `4`.
#[test]
fn should_read_the_parameter_in_the_initializer_when_a_const_shadows_it() {
    let source = r#"module codegen.own.st13;

func f(n: i32): i32 {
    const n: i32 = n + 1;
    return n;
}

func main(): i32 {
    return f(i32(3));
}
"#;
    let output = compile_build_and_run("st13", source);
    assert_exit("st13", &output, 4);
}

/// A local in a generic function shadows the parameter of the same name.
///
/// `pick` instantiates `T = i32`; the local `value` takes the fallback `3`
/// and is returned, so `main` exits with `3`.
#[test]
fn should_return_the_local_when_a_generic_local_shadows_a_parameter() {
    let source = r#"module codegen.own.st14;

func pick<T>(value: T, fallback: T): T {
    let value: T = fallback;
    return value;
}

func main(): i32 {
    return pick(i32(1), i32(3));
}
"#;
    let output = compile_build_and_run("st14", source);
    assert_exit("st14", &output, 3);
}

/// A local in a lambda body shadows the lambda's parameter of the same name.
///
/// The captured `base` is `2`, the argument is `3`; the body's local `x` is
/// `x + base = 5`, so `main` exits with `5`.
#[test]
fn should_return_the_local_sum_when_a_lambda_body_shadows_its_parameter() {
    let source = r#"module codegen.own.st15;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    let base: i32 = 2;
    return apply(i32(3), [base] (x: i32) => {
        let x: i32 = x + base;
        return x;
    });
}
"#;
    let output = compile_build_and_run("st15", source);
    assert_exit("st15", &output, 5);
}

/// A shadowing local may consume the outer binding in its own initializer.
///
/// The initializer's `x` resolves to the outer binding, which is declared
/// before use; the value moves into the new local, so the outer binding is
/// emptied and the function reads the moved four-byte string. A resolver that
/// attaches the move to the new binding instead rejects the valid program.
#[test]
fn should_move_the_outer_value_when_the_shadowing_initializer_names_it() {
    let source = r#"module codegen.own.st16;

import { String, stringOf } from std.text;

func vlen(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let x: String = stringOf("aaaa");
    let x: String = x;
    return vlen(x);
}
"#;
    let output = compile_build_and_run("st16", source);
    assert_exit("st16", &output, 4);
}
