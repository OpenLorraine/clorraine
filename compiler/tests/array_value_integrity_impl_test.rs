//! Implementation tests for the emitted shape of fixed-array values and `len`.
//!
//! The specification suite proves *what* a fixed-array value program computes;
//! these cases prove *how* the emitter spells it. C cannot assign an array, so
//! a copy or a move must be a whole-array byte copy through the runtime
//! `memcpy`; a move then zeroes the source; a same-place assignment emits no
//! copy at all. A call on the right of a reassignment writes through a hidden
//! out-parameter into a fresh temporary before the old target is released, so
//! a callee that reads the target (`s = dup(borrow s)`) never reads storage
//! mid-release. `len` renders the outermost extent of a fixed multi-extent
//! array, the byte view's length for an owned `String`, the literal byte count
//! for a string literal, and the count field of a growable `Array<T>`.
//!
//! The release cases count emitted deleter call sites: the old elements of a
//! reassigned `String` array must have exactly one release site each, so a
//! shape that releases a saved copy while the target still owns it would be
//! caught here.
//!
//! Every run goes through a helper with a time limit, so a regression that
//! loops forever fails the case instead of freezing the suite.

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
            "clc-avimpl-{}-{tag}-{sequence}",
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
/// Returns the emitted translation unit. A rejection is a failure of the case,
/// not a result to assert, because these cases are about what the emitter
/// produces for accepted programs.
fn emit_host(tag: &str, scratch: &ScratchDir, source: &str) -> String {
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

/// Compiles the emitted C with the host compiler under the strict flag set.
///
/// Panics with the compiler's stderr when the build fails. Returns the built
/// program path.
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

/// Runs a built program with a time limit and returns its output.
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_with_timeout(tag: &str, program: &Path) -> Output {
    let mut child = Command::new(program)
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

/// Emits, builds, and runs one program, returning its output and emitted C.
fn emit_build_run(tag: &str, source: &str) -> (Output, String) {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, &scratch, source);
    let program = build_host_c(tag, &scratch, &emitted);
    let output = run_with_timeout(tag, &program);
    (output, emitted)
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

/// Asserts the run aborted at the runtime guard for `code`.
fn assert_trap(tag: &str, output: &Output, code: &str) {
    assert!(
        !output.status.success(),
        "{tag}: a trapped run must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected the {code} trap; stderr:\n{stderr}"
    );
}

/// Binds a duplicable array to a second local with a whole-array byte copy.
///
/// C has no array assignment, so the emitter must copy the bytes and leave the
/// source intact; the program then sums the copy and reads the source.
#[test]
fn should_copy_the_bytes_when_a_fixed_array_initializes_from_a_place() {
    let source = r#"module codegen.av.impl1;

func main(): i32 {
    let a: i32[2] = [1, 2];
    let b: i32[2] = a;
    return b[0] + b[1] + a[0];
}
"#;
    let (output, emitted) = emit_build_run("ai1", source);
    assert!(
        emitted.contains("memcpy(&(b), &(a), sizeof(b));"),
        "ai1: the copy should be one whole-array memcpy from the source"
    );
    assert_exit("ai1", &output, 4);
}

/// Moving a `String` array copies the bytes and then zeroes the source.
///
/// The zeroed source proves the move emptied the original, and the clean exit
/// proves the two handles were released once each.
#[test]
fn should_copy_the_bytes_and_zero_the_source_when_a_string_array_moves() {
    let source = r#"module codegen.av.impl2;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2] = [stringOf("ab"), stringOf("cde")];
    let b: String[2] = a;
    return i32(len(b[1]));
}
"#;
    let (output, emitted) = emit_build_run("ai2", source);
    assert!(
        emitted.contains("memcpy(&(b), &(a), sizeof(b));"),
        "ai2: the move should copy the array bytes"
    );
    assert!(
        emitted.contains("cl_zero(&a);"),
        "ai2: the move should zero the moved-out source"
    );
    assert_exit("ai2", &output, 3);
}

/// Assigning an array to itself emits no copy and leaves the value intact.
///
/// The right-hand side renders to the target, so the bytes are already in
/// place; a self-`memcpy` would be redundant and undefined, and the program
/// still sums to three.
#[test]
fn should_emit_no_copy_when_a_fixed_array_is_assigned_to_itself() {
    let source = r#"module codegen.av.impl3;

func main(): i32 {
    let a: i32[2] = [1, 2];
    a = a;
    return a[0] + a[1];
}
"#;
    let (output, emitted) = emit_build_run("ai3", source);
    assert!(
        !emitted.contains("memcpy(&(a)"),
        "ai3: a same-place assignment must not copy the array onto itself"
    );
    assert_exit("ai3", &output, 3);
}

/// A call on the right of an array reassignment writes into a temporary first.
///
/// The hidden out-parameter names the temporary, not the target, so a callee
/// that reads the target never writes into storage it is reading. The old
/// target is released once per element before the byte copy, and the temporary
/// is zeroed after it.
#[test]
fn should_write_the_call_result_into_a_temporary_when_a_call_reassigns_an_array() {
    let source = r#"module codegen.av.impl4;

import { String, stringOf } from std.text;

func mk(): String[2] {
    return [stringOf("pq"), stringOf("rs")];
}

func main(): i32 {
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    s = mk();
    return i32(len(s[1]));
}
"#;
    let (output, emitted) = emit_build_run("ai4", source);
    assert!(
        emitted.contains("cl_codegen_av_impl4_mk(__cl_call"),
        "ai4: the call should write into a fresh temporary"
    );
    assert!(
        emitted.contains("memcpy(&(s), &(__cl_call"),
        "ai4: the temporary should be copied into the target"
    );
    assert!(
        emitted.contains("cl_zero(&__cl_call"),
        "ai4: the temporary should be zeroed after the copy"
    );
    // The first pair of sites guards the fresh initializer target, the second
    // pair releases the old elements at the reassignment, and the third pair
    // is the scope cleanup: one release site per element at each point.
    for element in ["s[0]", "s[1]"] {
        assert_eq!(
            emitted.matches(&format!("FreeString({element})")).count(),
            3,
            "ai4: {element} should have exactly three emitted release sites"
        );
    }
    assert_exit("ai4", &output, 2);
}

/// A call that reads the target releases the old elements exactly once.
///
/// The callee builds its result from the old elements, so the old value must
/// outlive the call and then be released once per element. The counts pin the
/// release sites: the initializer guard, the reassignment release, and the
/// scope cleanup, one site per element at each point.
#[test]
fn should_release_each_element_once_when_a_call_reassigns_from_itself() {
    let source = r#"module codegen.av.impl5;

import { String, stringOf } from std.text;

func dup(borrow source: String[2]): String[2] {
    return [stringOf(source[1]), stringOf(source[0])];
}

func main(): i32 {
    let s: String[2] = [stringOf("ab"), stringOf("cde")];
    s = dup(borrow s);
    return i32(len(s[0]));
}
"#;
    let (output, emitted) = emit_build_run("ai5", source);
    for element in ["s[0]", "s[1]"] {
        assert_eq!(
            emitted.matches(&format!("FreeString({element})")).count(),
            3,
            "ai5: {element} should have exactly three emitted release sites"
        );
    }
    assert_exit("ai5", &output, 3);
}

/// `len` of a fixed multi-extent array renders the outermost extent.
///
/// The array holds two rows, so the rendered constant is two; the element read
/// keeps the array a used value in the emitted C.
#[test]
fn should_render_the_outermost_extent_when_len_reads_a_multi_extent_array() {
    let source = r#"module codegen.av.impl6;

func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    if (m[0][0] != 1) {
        return 100;
    }
    return i32(len(m));
}
"#;
    let (output, emitted) = emit_build_run("ai6", source);
    assert!(
        emitted.contains("return (i32)(2);"),
        "ai6: len of a two-row array should render the constant 2"
    );
    assert_exit("ai6", &output, 2);
}

/// `len` renders the view length for a `String`, the literal count for a
/// string literal, and the count field for a growable array.
///
/// The three shapes must differ: an owned `String` reads its bytes through the
/// implicit view, a `static str` literal keeps its static length, and a
/// growable `Array<T>` reads its element count.
#[test]
fn should_render_the_view_length_and_count_when_len_reads_string_and_growable() {
    let source = r#"module codegen.av.impl7;

import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;

func main(): i32 {
    let s: String = stringOf("hello");
    let g: Array<i32> = arrayNew<i32>();
    g.push(1);
    if (len(g) != 1) {
        return 100;
    }
    return i32(len(s)) + i32(len("hi")) + i32(len(g));
}
"#;
    let (output, emitted) = emit_build_run("ai7", source);
    assert!(
        emitted.contains("(cl_string_view(s)).len"),
        "ai7: an owned String should read its byte count through the view"
    );
    assert!(
        emitted.contains("(cl_str(\"hi\")).len"),
        "ai7: a static string literal should keep its static length"
    );
    assert!(
        emitted.contains("(g).count"),
        "ai7: a growable array should read its count field"
    );
    assert_exit("ai7", &output, 8);
}

/// A guarded call argument traps before the call runs.
///
/// A fixed-array call on the right of an assignment must still run the
/// argument's bounds guard first; without it the program would read past the
/// array and pass the garbage into the callee instead of trapping.
#[test]
fn should_trap_when_a_call_argument_index_is_out_of_bounds() {
    let source = r#"module codegen.av.impl8;

func mk(n: i32): i32[2] {
    return [n, n];
}

func main(): i32 {
    let a: i32[2] = [1, 2];
    let i: i32 = 5;
    let b: i32[2] = mk(a[i]);
    return b[0];
}
"#;
    let (output, _emitted) = emit_build_run("ai8", source);
    assert_trap("ai8", &output, "E11211");
}

/// A slice with a constant zero low bound and a variable high bound builds
/// cleanly.
///
/// The lower-bound clause `(usize)(0) > (usize)(hi)` is always false for an
/// unsigned high bound, so it must not be emitted; otherwise the build fails
/// `-Wtype-limits`. The program then reads the first element of the slice.
#[test]
fn should_build_when_a_slice_low_bound_is_the_constant_zero() {
    let source = r#"module codegen.av.impl9;

import { slice } from std.mem;

func n(): usize {
    return 4;
}

func main(): i32 {
    let xs: u8[4] = [1, 2, 3, 4];
    let v: u8[] = slice(xs, 0, n());
    return i32(v[0]);
}
"#;
    let (output, emitted) = emit_build_run("ai9", source);
    assert!(
        !emitted.contains("(usize)(0) > (usize)"),
        "ai9: the always-false lower-bound comparison must not be emitted"
    );
    assert_exit("ai9", &output, 1);
}
