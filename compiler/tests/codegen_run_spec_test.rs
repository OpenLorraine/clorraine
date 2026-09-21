//! Host codegen compile-and-run specification tests.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! compiles the emitted C with the host C compiler under the fixed warning
//! flags, links an optional C stub, runs it, and asserts the observable result.
//! The cases cover the cleanup, ownership, binding, loop, and arithmetic
//! behavior the unit-level oracle cannot see: a defect here only appears once
//! the emitted C is actually compiled and executed.
//!
//! The stub provides the `ffi` targets a case calls, so a case can observe an
//! open/close count through `printf` without a standard library.

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
            "clc-codegen-{}-{tag}-{sequence}",
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

/// Compiles one source, builds the emitted C (plus an optional stub), and runs it.
///
/// Panics when the front end rejects the program or the C compiler rejects the
/// emitted C, because either is a failure of this case rather than a result to
/// assert. Returns the finished program's output.
fn build_and_run(tag: &str, source: &str, stub: Option<&str>) -> Output {
    build_and_run_files(tag, &[("main.clo", source)], stub)
}

/// Like [`build_and_run`] but with several source files in one invocation.
fn build_and_run_files(tag: &str, files: &[(&str, &str)], stub: Option<&str>) -> Output {
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
    let mut cc_args = vec![
        "-std=c99".to_owned(),
        "-O0".to_owned(),
        "-g".to_owned(),
        "-Wall".to_owned(),
        "-Wextra".to_owned(),
        "-Werror".to_owned(),
        "-o".to_owned(),
        program.display().to_string(),
        c_path.display().to_string(),
    ];
    if let Some(stub) = stub {
        let stub_path = scratch.path().join("stub.c");
        std::fs::write(&stub_path, stub).expect("write the stub");
        cc_args.push(stub_path.display().to_string());
    }
    let built = Command::new("cc")
        .args(&cc_args)
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

/// Asserts the run wrote exactly `expected` to stdout and exited zero.
fn assert_stdout(tag: &str, output: &Output, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected a clean exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "{tag}: unexpected standard output"
    );
}

/// Asserts the run aborted and its first stderr line is `expected`.
fn assert_traps(tag: &str, output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "{tag}: a runtime fault must exit nonzero"
    );
    let first = String::from_utf8_lossy(&output.stderr)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(first, expected, "{tag}: unexpected first stderr line");
}

/// An `ffi` stub that reports every open and close to stdout.
const OPEN_CLOSE_STUB: &str = r#"#include <stdio.h>
#include <stdlib.h>
void *OpenFile(void) { puts("open"); return malloc(8); }
void CloseFile(void *f) { puts("close"); free(f); }
"#;

/// An `ffi` stub that prints its integer argument.
const EMIT_STUB: &str = r#"#include <stdio.h>
void emit(int n) { printf("%d\n", n); }
"#;

/// A scope's cleanup runs when control falls off its closing brace.
///
/// The `demo` body owns a handle and does not return early, so the drop must run
/// on the normal fall-through. A missing drop leaks, and a ladder label with no
/// jump fails `-Werror=unused-label`, so the stub's `close` proves the run.
#[test]
fn scope_cleanup_runs_on_normal_fall_through() {
    let source = r#"module codegen.c1;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func demo(): void {
    let f: File? = OpenFile();
    if (f == null) {
        return;
    }
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let output = build_and_run("c1", source, Some(OPEN_CLOSE_STUB));
    assert_stdout("c1", &output, "open\nclose\n");
}

/// A nested scope's cleanup runs on the inner block's normal end.
#[test]
fn nested_scope_cleanup_runs_on_normal_fall_through() {
    let source = r#"module codegen.c1_nested;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func demo(): void {
    let outer: File? = OpenFile();
    if (outer == null) {
        return;
    }
    {
        let inner: File? = OpenFile();
        if (inner == null) {
            return;
        }
    }
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let output = build_and_run("c1-nested", source, Some(OPEN_CLOSE_STUB));
    assert_stdout("c1-nested", &output, "open\nopen\nclose\nclose\n");
}

/// A `defer` in a scope that owns no drop still runs on `return`.
#[test]
fn a_defer_runs_on_return_without_an_owned_local() {
    let source = r#"module codegen.c2;

ffi probe {
    func emit(n: i32): void;
}

func demo(): i32 {
    defer {
        emit(7);
    }
    return 0;
}

func main(): i32 {
    return demo();
}
"#;
    let output = build_and_run("c2", source, Some(EMIT_STUB));
    assert_stdout("c2", &output, "7\n");
}

/// A `break` out of a `switch` arm runs the arm scope's cleanup first.
#[test]
fn a_break_runs_the_scope_cleanup_before_the_jump() {
    let source = r#"module codegen.c5;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func demo(n: i32): void {
    let f: File? = OpenFile();
    if (f == null) {
        return;
    }
    switch (n) {
        case 1: {
            let g: File? = OpenFile();
            if (g == null) {
                return;
            }
            break;
        }
        default: {}
    }
}

func main(): i32 {
    demo(1);
    return 0;
}
"#;
    let output = build_and_run("c5", source, Some(OPEN_CLOSE_STUB));
    assert_stdout("c5", &output, "open\nopen\nclose\nclose\n");
}

/// A `take` argument empties the caller's binding, so the scope drop is a no-op.
#[test]
fn a_take_argument_is_emptied_after_the_call() {
    let source = r#"module codegen.c6;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func demo(): void {
    let f: File? = OpenFile();
    if (f == null) {
        return;
    }
    CloseFile(f);
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let output = build_and_run("c6", source, Some(OPEN_CLOSE_STUB));
    assert_stdout("c6", &output, "open\nclose\n");
}

/// Returning an owned place empties the source, so the callee does not free it.
#[test]
fn returning_an_owned_place_empties_the_source() {
    let source = r#"module codegen.c7;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func acquire(): File? {
    let f: File? = OpenFile();
    if (f == null) {
        return null;
    }
    return f;
}

func main(): i32 {
    let g: File? = acquire();
    return 0;
}
"#;
    let output = build_and_run("c7", source, Some(OPEN_CLOSE_STUB));
    assert_stdout("c7", &output, "open\nclose\n");
}

/// A negative signed index fails the bounds check instead of reading out of range.
#[test]
fn a_negative_index_traps() {
    let source = r#"module codegen.c8;

func get(xs: i32[3], i: i32): i32 {
    return xs[i];
}

func main(): i32 {
    let xs: i32[3] = [1, 2, 3];
    let i: i32 = -1;
    return get(xs, i);
}
"#;
    let output = build_and_run("c8", source, None);
    assert_traps("c8", &output, "error E11211: index out of range");
}

/// A store through a run-time index writes the selected element.
///
/// Element zero stays zero while the selected element becomes nine, so the
/// hardcoded element-zero store of the old lowering would return ninety.
#[test]
fn a_store_through_a_run_time_index_writes_the_selected_element() {
    let source = r#"module codegen.c9;

func fill(borrow xs: u32[4], i: usize): u32 {
    xs[i] = 9;
    return xs[0] * 10 + xs[i];
}

func main(): i32 {
    let xs: u32[4] = [0, 0, 0, 0];
    let i: usize = 2;
    return i32(fill(borrow xs, i));
}
"#;
    let output = build_and_run("c9", source, None);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "",
        "c9: the program prints nothing"
    );
    assert_eq!(output.status.code(), Some(9), "c9: expected 9");
}

/// An element loop copies each element, and a `borrow` element writes through.
#[test]
fn element_loops_copy_and_write_through() {
    let source = r#"module codegen.c4;

func sum(xs: u32[4]): u32 {
    let total: u32 = 0;
    for (x in xs) {
        total = total + x;
    }
    return total;
}

func doubleAll(borrow xs: u32[4]): void {
    for (borrow e in xs) {
        e = e * 2;
    }
}

func main(): i32 {
    let xs: u32[4] = [1, 2, 3, 4];
    doubleAll(borrow xs);
    return i32(sum(xs));
}
"#;
    let output = build_and_run("c4", source, None);
    assert_eq!(output.status.code(), Some(20), "c4: expected 20");
}

/// Two modules with a same-named private function do not collide.
#[test]
fn same_named_functions_in_two_modules_do_not_collide() {
    let files = [
        (
            "alpha.clo",
            "module Codegen.Alpha;\n\nfunc helper(): i32 { return 1; }\n\nexport func alpha(): i32 { return helper(); }\n",
        ),
        (
            "beta.clo",
            "module Codegen.Beta;\n\nfunc helper(): i32 { return 2; }\n\nexport func beta(): i32 { return helper(); }\n",
        ),
        (
            "main.clo",
            "module Codegen.Main;\n\nimport { alpha } from Codegen.Alpha;\nimport { beta } from Codegen.Beta;\n\nfunc main(): i32 { return alpha() + beta(); }\n",
        ),
    ];
    let output = build_and_run_files("m1", &files, None);
    assert_eq!(output.status.code(), Some(3), "m1: expected 3");
}

/// Assigning to a live owned target releases the old value first.
#[test]
fn assignment_drops_the_old_value() {
    let source = r#"module codegen.m2;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func demo(): void {
    let f: File? = OpenFile();
    if (f == null) {
        return;
    }
    f = OpenFile();
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let output = build_and_run("m2", source, Some(OPEN_CLOSE_STUB));
    assert_stdout("m2", &output, "open\nclose\nopen\nclose\n");
}

/// A signed left shift is emitted through unsigned operands and is defined.
#[test]
fn a_signed_left_shift_is_well_defined() {
    let source = r#"module codegen.m3;

func shift(x: i32, n: u32): i32 {
    return x << n;
}

func main(): i32 {
    let a: i32 = -8;
    return shift(a, 1) + 16;
}
"#;
    let output = build_and_run("m3", source, None);
    assert_eq!(output.status.code(), Some(0), "m3: -16 + 16 is zero");
}

/// A switch with fallthrough compiles warning-free and falls through.
#[test]
fn switch_fallthrough_compiles_and_falls_through() {
    let source = r#"module codegen.m4;

func classify(v: i32): i32 {
    let r: i32 = 0;
    switch (v) {
        case 1: {
            r = 10;
        }
        case 2: {
            r = r + 1;
            break;
        }
        default: {
            r = 99;
        }
    }
    return r;
}

func main(): i32 {
    return classify(1);
}
"#;
    let output = build_and_run("m4", source, None);
    assert_eq!(output.status.code(), Some(11), "m4: case 1 falls into case 2");
}

/// Unused parameters and an uncalled private function compile warning-free.
#[test]
fn unused_parameters_and_functions_compile() {
    let source = r#"module codegen.m5;

func neverCalled(a: i32, b: i32): i32 {
    return a + b;
}

func main(): i32 {
    return 0;
}
"#;
    let output = build_and_run("m5", source, None);
    assert_stdout("m5", &output, "");
}

/// A guard-bearing loop test emits structured C with no stray `goto`.
#[test]
fn a_guard_bearing_loop_test_is_structured() {
    let source = r#"module codegen.m6;

func count(n: i32): i32 {
    let i: i32 = 0;
    while (i < n / 2) {
        i = i + 1;
    }
    return i;
}

func main(): i32 {
    return count(10);
}
"#;
    let output = build_and_run("m6", source, None);
    assert_eq!(output.status.code(), Some(5), "m6: expected 5");
}

/// A module-global initializer guards its arithmetic before the store.
#[test]
fn a_global_initializer_guards_its_arithmetic() {
    let source = r#"module codegen.m7;

let ZERO: i32 = 0;
let QUOTIENT: i32 = 10 / ZERO;

func main(): i32 {
    return QUOTIENT;
}
"#;
    let output = build_and_run("m7", source, None);
    assert_traps("m7", &output, "error E11210: integer division by zero");
}

/// Two defers run in reverse registration order.
#[test]
fn defers_run_in_reverse_registration_order() {
    let source = r#"module codegen.m8;

ffi probe {
    func emit(n: i32): void;
}

func demo(): i32 {
    defer {
        emit(1);
    }
    defer {
        emit(2);
    }
    return 0;
}

func main(): i32 {
    return demo();
}
"#;
    let output = build_and_run("m8", source, Some(EMIT_STUB));
    assert_stdout("m8", &output, "2\n1\n");
}

/// An owned local declared in a loop body is released once per iteration.
///
/// The local is hoisted to the function frame, so the per-iteration cleanup must
/// empty it; otherwise the next iteration releases the dangling handle a second
/// time.
#[test]
fn an_owned_local_in_a_loop_is_released_once_per_iteration() {
    let source = r#"module codegen.rv430;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func demo(): void {
    let n: i32 = 0;
    while (n < 2) {
        let f: File? = OpenFile();
        if (f == null) {
            return;
        }
        n = n + 1;
    }
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let output = build_and_run("rv430", source, Some(OPEN_CLOSE_STUB));
    assert_stdout("rv430", &output, "open\nclose\nopen\nclose\n");
}

/// A stub whose `Dup` reports before it reads, so the old-value drop order shows.
const DUP_STUB: &str = r#"#include <stdio.h>
#include <stdlib.h>
void *OpenFile(void) { puts("open"); return malloc(8); }
void CloseFile(void *f) { puts("close"); free(f); }
void *Dup(void *f) { puts("dup"); (void)f; return malloc(8); }
"#;

/// An assignment whose right-hand side reads its own target releases the old
/// value only after the right-hand side has been evaluated.
///
/// The stub prints `dup` before the old value is closed, so the test fails if
/// the emitter drops the target before the call.
#[test]
fn an_assignment_reading_its_target_drops_the_old_value_after_the_rhs() {
    let source = r#"module codegen.rv431;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
    func Dup(borrow f: File): File?;
}

func demo(): void {
    let f: File? = OpenFile();
    if (f == null) {
        return;
    }
    f = Dup(borrow f);
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let output = build_and_run("rv431", source, Some(DUP_STUB));
    assert_stdout("rv431", &output, "open\ndup\nclose\nclose\n");
}

/// An element loop writes its `borrow` element back even on `break`/`continue`.
///
/// `doubleUntil` breaks after the element exceeds four and `doubleAllContinue`
/// continues after it exceeds three; both must still copy the final element
/// value into the iterable, so the two sums are sixteen and twenty.
#[test]
fn element_loop_write_back_survives_break_and_continue() {
    let source = r#"module codegen.rv432;

func doubleUntil(borrow xs: u32[4]): void {
    for (borrow e in xs) {
        e = e * 2;
        if (e > 4) {
            break;
        }
    }
}

func doubleAllContinue(borrow xs: u32[4]): void {
    for (borrow e in xs) {
        e = e * 2;
        if (e > 3) {
            continue;
        }
    }
}

func sum(xs: u32[4]): u32 {
    let total: u32 = 0;
    for (x in xs) {
        total = total + x;
    }
    return total;
}

func main(): i32 {
    let xs: u32[4] = [1, 2, 3, 4];
    doubleUntil(borrow xs);
    let a: i32 = i32(sum(xs));
    let ys: u32[4] = [1, 2, 3, 4];
    doubleAllContinue(borrow ys);
    let b: i32 = i32(sum(ys));
    return a + b * 10;
}
"#;
    let output = build_and_run("rv432", source, None);
    assert_eq!(output.status.code(), Some(216), "rv432: sixteen plus ten times twenty");
}

/// An imported same-named function resolves to the imported module's symbol.
///
/// Two modules export `pick`; the caller imports `pick` from the first, so the
/// call must bind that module's symbol rather than emit an implicit declaration.
#[test]
fn an_imported_same_named_function_uses_the_imported_symbol() {
    let files = [
        (
            "alpha.clo",
            "module Codegen.Pick.Alpha;\n\nexport func pick(): i32 { return 1; }\n",
        ),
        (
            "beta.clo",
            "module Codegen.Pick.Beta;\n\nexport func pick(): i32 { return 2; }\n",
        ),
        (
            "main.clo",
            "module Codegen.Pick.Main;\n\nimport { pick } from Codegen.Pick.Alpha;\n\nfunc main(): i32 { return pick(); }\n",
        ),
    ];
    let output = build_and_run_files("rv433", &files, None);
    assert_eq!(output.status.code(), Some(1), "rv433: the imported pick wins");
}

/// An assigned-but-unread local and a never-used local compile warning-free.
#[test]
fn an_assigned_but_unread_local_compiles() {
    let source = r#"module codegen.rv436;

func demo(n: i32): i32 {
    let x: i32 = 0;
    x = n + 1;
    let unused: i32 = 5;
    return n;
}

func main(): i32 {
    return demo(1);
}
"#;
    let output = build_and_run("rv436", source, None);
    assert_eq!(
        output.status.code(),
        Some(1),
        "rv436: the program prints nothing and returns the argument"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "", "rv436: no output");
}

/// A combined stub that reports an open, a close, and an emitted integer.
const OPEN_CLOSE_EMIT_STUB: &str = r#"#include <stdio.h>
#include <stdlib.h>
void *OpenFile(void) { puts("open"); return malloc(8); }
void CloseFile(void *f) { puts("close"); free(f); }
void emit(int n) { printf("%d\n", n); }
"#;

/// Drops and `defer` entries run in one reverse registration order.
///
/// The `defer` is registered before the handle, so the handle is released first
/// and the deferred body runs last: `open`, `close`, then `1`.
#[test]
fn defers_and_drops_run_in_one_reverse_registration_order() {
    let source = r#"module codegen.rv438;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
    func emit(n: i32): void;
}

func demo(): void {
    defer {
        emit(1);
    }
    let f: File? = OpenFile();
    if (f == null) {
        return;
    }
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let output = build_and_run("rv438", source, Some(OPEN_CLOSE_EMIT_STUB));
    assert_stdout("rv438", &output, "open\nclose\n1\n");
}
