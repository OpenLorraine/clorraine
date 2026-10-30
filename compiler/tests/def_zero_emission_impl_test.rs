//! Implementation pins for the zero-defect emission and ABI batch.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-zeroemission-impl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Compiles one program under a chosen profile and returns the output and C
/// path.
fn compile_profile(
    tag: &str,
    source: &str,
    profile: &str,
) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            profile,
            "-o",
            c_path.to_str().expect("C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn clc");
    (scratch, output, c_path)
}

/// Compiles one host program and returns the output and C path.
fn compile(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    compile_profile(tag, source, "host")
}

/// Compiles one accepted host program and returns its emitted C.
fn emit(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Counts non-overlapping occurrences of `needle`.
fn count(text: &str, needle: &str) -> usize {
    text.match_indices(needle).count()
}

/// the zero-high slice guard is gone from the emitted C.
#[test]
fn should_not_emit_the_zero_high_slice_guard() {
    let source = r#"module spec.zei.slice_guard;

import { slice } from std.mem;

func useView(v: u8[]): i32 {
    let w: u8[] = slice(v, 0, 0);
    return i32(len(w));
}

func main(): i32 {
    let xs: u8[3] = [1, 2, 3];
    return useView(slice(xs, 0, 3));
}
"#;
    let emitted = emit("slice_guard", source);
    assert!(
        !emitted.contains("(usize)(0) >"),
        "slice_guard: no always-false upper-bound comparison; C:\n{emitted}"
    );
}

/// the closure argument widens through the view constructor.
#[test]
fn should_widen_a_closure_string_argument_to_the_view() {
    let source = r#"module spec.zei.closure_widen;

import { stringOf } from std.text;

func main(): i32 {
    return ((s: str) => i32(len(s)))(stringOf("hello")) - 5;
}
"#;
    let emitted = emit("closure_widen", source);
    assert!(
        emitted.contains("cl_string_view("),
        "closure_widen: the argument must widen; C:\n{emitted}"
    );
}

/// the initializer fixed-array field emits one division guard.
#[test]
fn should_emit_one_initializer_division_guard() {
    let source = r#"module spec.zei.one_guard;

struct BoxA { arr: i32[2]; }

let d: i32 = 1;
let b: BoxA = BoxA { arr: [12 / d, 2] };

func main(): i32 {
    return b.arr[0] + b.arr[1] - 14;
}
"#;
    let emitted = emit("one_guard", source);
    assert_eq!(
        count(&emitted, "integer division by zero"),
        1,
        "one_guard: one guard per operand; C:\n{emitted}"
    );
}

/// the nested instance is defined before the struct that embeds it.
#[test]
fn should_order_the_nested_instance_before_its_user() {
    let source = r#"module spec.zei.instance_order;

struct Pair<T> { n: T; }
struct Box<T> { p: Pair<T>; }

func sizeOf<T>(): usize { return sizeof(T[]); }

func main(): i32 {
    let n: usize = sizeOf<Box<i32>>();
    let m: usize = sizeof(Box<i32>[]);
    return i32(n - m);
}
"#;
    let emitted = emit("instance_order", source);
    let pair = emitted
        .find("struct ClPair__i32 {")
        .expect("the Pair instance must be defined");
    let user = emitted
        .find("struct ClBox__i32 {")
        .expect("the Box instance must be defined");
    assert!(
        pair < user,
        "instance_order: Pair must precede Box; C:\n{emitted}"
    );
}

/// `offsetof` emits the real C macro, not zero.
#[test]
fn should_emit_the_offsetof_macro() {
    let source = r#"module spec.zei.offsetof_macro;

struct S { a: i32; b: i32; }

const k: usize = offsetof(S, b);

func main(): i32 {
    let m: usize = offsetof(S, b);
    return i32(k - 4) + i32(m - 4);
}
"#;
    let emitted = emit("offsetof_macro", source);
    assert!(
        emitted.contains("offsetof(ClS, b)"),
        "offsetof_macro: the value position must spell the C macro; C:\n{emitted}"
    );
}

/// the raw scalar member is rejected before any C is written.
#[test]
fn should_reject_the_raw_scalar_member_before_emission() {
    let source = r#"module spec.zei.raw_member;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let p: raw u8 = unsafe &a[0];
    let q: raw u8 = unsafe p.ptr;
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile("raw_member", source);
    assert_ne!(output.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("error E10212:"),
        "raw_member: expected E10212; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!c_path.exists(), "raw_member: a rejected program writes no C");
}

/// `offsetof` builds under the freestanding no-libc profile.
#[test]
fn should_build_offsetof_under_the_freestanding_profile() {
    let source = r#"module spec.zei.offsetof_freestanding;

struct S { a: i32; b: i32; }

func main(): i32 {
    let m: usize = offsetof(S, b);
    return i32(m) - 4;
}
"#;
    let (scratch, output, c_path) = compile_profile("offsetof_fs", source, "freestanding");
    assert_eq!(
        output.status.code(),
        Some(0),
        "offsetof_fs: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-ffreestanding",
            "-fno-builtin",
            "-nostdlib",
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
        "offsetof_fs: the freestanding C build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new(&program)
        .output()
        .expect("failed to spawn the freestanding program");
    assert_eq!(
        run.status.code(),
        Some(0),
        "offsetof_fs: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
