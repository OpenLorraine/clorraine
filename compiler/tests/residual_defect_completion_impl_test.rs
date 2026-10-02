//! Implementation tests for the symbol keys and argument decisions that carry
//! fixed-array and generic types across the C boundary.
//!
//! The specification suite proves *what* a program computes; these cases prove
//! *how* the emitter spells it. Distinct extent lists must produce distinct C
//! struct names, a substituted borrow argument must be passed by address, and
//! two emissions of one program must be byte-identical.

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
    let emit = |output_name: &str| -> String {
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
            "determinism: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(&output_path).unwrap_or_default()
    };
    let first = emit("first.c");
    let second = emit("second.c");
    assert!(
        !first.trim().is_empty(),
        "determinism: the emitted C is empty, so the emitter is not implemented yet"
    );
    assert_eq!(
        first, second,
        "determinism: the same program emitted two different translations"
    );
}
