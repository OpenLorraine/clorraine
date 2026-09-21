//! Standard-library implementation tests.
//!
//! The specification suite pins the main paths of the standard library: a
//! buffer slice, a full file round-trip, array basics, and a span that reaches
//! past the source. These cases probe the edges those cases leave open: a
//! buffer built from a view and an array grown past its first allocation,
//! `get`/`set` at and past the element count, padding to a width of zero and
//! to a width below the input, a short file read ending in EOF, and a span
//! whose end lands exactly on the source length.
//!
//! Each case compiles a real program with the built `clc` binary, builds the
//! emitted C with the host C compiler under the fixed warning flags, links the
//! real host profile shim, and runs the program. There are no mocks.

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
            "clc-stdlib-{}-{tag}-{sequence}",
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

/// Compiles one source, builds the emitted C, and runs it.
fn build_and_run(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let program = compile_and_build(tag, source, scratch.path());
    run_program(tag, &program, None)
}

/// Like [`build_and_run`] but the program runs with the scratch directory as
/// its working directory.
///
/// A file case opens paths relative to the working directory, so it runs in a
/// private scratch directory that is removed, with every file the program
/// created in it, when the call returns.
fn build_and_run_in_scratch(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let program = compile_and_build(tag, source, scratch.path());
    run_program(tag, &program, Some(scratch.path()))
}

/// Compiles `source`, builds the emitted C, and returns the built program path.
///
/// Both the front end and the host C compiler must accept the input, so a
/// rejection panics instead of returning.
fn compile_and_build(tag: &str, source: &str, scratch: &Path) -> PathBuf {
    let source_path = scratch.join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let c_path = scratch.join("out.c");
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

    let program = scratch.join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99".to_owned(),
            "-O0".to_owned(),
            "-g".to_owned(),
            "-Wall".to_owned(),
            "-Wextra".to_owned(),
            "-Werror".to_owned(),
            "-o".to_owned(),
            program.display().to_string(),
            c_path.display().to_string(),
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

/// Runs a built program, optionally from a working directory, and returns its
/// output.
fn run_program(tag: &str, program: &Path, cwd: Option<&Path>) -> Output {
    let mut command = Command::new(program);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
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

/// `bufferFrom` copies its source and a growable array keeps every element it
/// gains across allocations.
///
/// The specification case writes a fixed buffer and slices it. This case adds
/// the edges: a buffer built from a byte view, an empty buffer whose view has
/// length zero, an `Array<i32>` pushed past the first growth boundary so its
/// backing buffer reallocates, and a `reserve` call before a push.
#[test]
fn buffer_from_and_array_growth_keep_their_contents() {
    let source = r#"module impl.buffer;

import { Buffer, bufferNew, bufferFrom, bufferBytes, Array, arrayNew } from std.mem;
import { String, stringOf, bytes } from std.text;
import { println } from std.io;

func copySum(): i32 {
    let s: String = stringOf("tag");
    let src: u8[] = bytes(s);
    let b: Buffer? = bufferFrom(src);
    if (b == null) {
        return -1;
    }
    let out: u8[] = bufferBytes(b);
    return i32(out[0]) + i32(out[1]) + i32(out[2]);
}

func main(): i32 {
    let sum: i32 = copySum();
    let empty: Buffer? = bufferNew(0);
    if (empty == null) {
        return 1;
    }
    let emptyLen: usize = len(bufferBytes(empty));
    let xs: Array<i32> = arrayNew<i32>();
    let i: i32 = 0;
    while (i < 20) {
        xs.push(i);
        i = i + 1;
    }
    let first: i32 = xs.get(0)[0];
    let last: i32 = xs.get(19)[0];
    let count: usize = xs.len();
    let ys: Array<i32> = arrayNew<i32>();
    ys.reserve(64);
    ys.push(7);
    let reserved: i32 = ys.get(0)[0];
    println($"impl-buffer: sum={sum} empty={i32(emptyLen)} first={first} last={last} count={i32(count)} reserved={reserved}");
    return 0;
}
"#;
    let output = build_and_run("buffer", source);
    assert_stdout(
        "buffer",
        &output,
        "impl-buffer: sum=316 empty=0 first=0 last=19 count=20 reserved=7\n",
    );
}

/// A `get` at the element count traps instead of reading past the array.
#[test]
fn array_get_at_the_count_traps() {
    let source = r#"module impl.array_get;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let xs: Array<i32> = arrayNew<i32>();
    xs.push(1);
    xs.push(2);
    let over: i32 = xs.get(2)[0];
    return over;
}
"#;
    let output = build_and_run("array-get", source);
    assert_traps("array-get", &output, "error E11211: index out of range");
}

/// A `set` at the element count traps instead of writing past the array.
#[test]
fn array_set_at_the_count_traps() {
    let source = r#"module impl.array_set;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let xs: Array<i32> = arrayNew<i32>();
    xs.push(1);
    xs.set(1, 9);
    return 0;
}
"#;
    let output = build_and_run("array-set", source);
    assert_traps("array-set", &output, "error E11211: index out of range");
}

/// A `get` on an empty array traps.
#[test]
fn array_get_on_an_empty_array_traps() {
    let source = r#"module impl.array_empty;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let xs: Array<i32> = arrayNew<i32>();
    let over: i32 = xs.get(0)[0];
    return over;
}
"#;
    let output = build_and_run("array-empty", source);
    assert_traps("array-empty", &output, "error E11211: index out of range");
}

/// Padding a string to a width of zero or below its length returns it
/// unchanged, including the empty string at a positive width.
///
/// The specification case does not exercise a width that cannot grow the input,
/// which is where an off-by-one in the pad count would show.
#[test]
fn padding_below_the_source_width_leaves_the_source_unchanged() {
    let source = r#"module impl.format;

import { padLeft, padRight, toHex } from std.fmt;
import { println } from std.io;

func main(): i32 {
    println(padLeft("ab", 0, 48));
    println(padLeft("abcd", 2, 46));
    println(padLeft("", 3, 48));
    println(padRight("ab", 0, 48));
    println(padRight("abcd", 2, 46));
    println(padRight("", 3, 46));
    println(toHex(0));
    println(padLeft("a", 1, 45));
    return 0;
}
"#;
    let output = build_and_run("format", source);
    assert_stdout(
        "format",
        &output,
        "ab\nabcd\n000\nab\nabcd\n...\n0x0\na\n",
    );
}

/// Repeated short reads drain a file and then report end of file as zero.
///
/// The specification case reads a whole file in one call. This case reads five
/// bytes through a two-byte buffer, so the count must fall to one and then to
/// zero, which is the short-read and EOF boundary `readFile` documents.
#[test]
fn repeated_short_reads_end_at_zero() {
    let source = r#"module impl.file_eof;

import { File, openFile, createFile, readFile, writeBytes, println, MODE_OLDFILE } from std.io;
import { Buffer, bufferNew, bufferBytes } from std.mem;
import { String, stringOf, bytes, toCString } from std.text;

const PATH: static str = "impl-eof.txt";
const PAYLOAD: static str = "abcde";

func writePayload(path: str, data: u8[]): i32 {
    let out: File? = createFile(toCString(stringOf(path)));
    if (out == null) {
        return -1;
    }
    return writeBytes(out, data);
}

func main(): i32 {
    let payloadString: String = stringOf(PAYLOAD);
    let payload: u8[] = bytes(payloadString);
    if (writePayload(PATH, payload) != i32(len(payload))) {
        return 1;
    }
    let input: File? = openFile(toCString(stringOf(PATH)), MODE_OLDFILE);
    if (input == null) {
        return 2;
    }
    let buf: Buffer? = bufferNew(2);
    if (buf == null) {
        return 3;
    }
    let g1: i32 = readFile(input, borrow bufferBytes(buf));
    let g2: i32 = readFile(input, borrow bufferBytes(buf));
    let g3: i32 = readFile(input, borrow bufferBytes(buf));
    let g4: i32 = readFile(input, borrow bufferBytes(buf));
    println($"impl-file: {g1},{g2},{g3},{g4}");
    return 0;
}
"#;
    let output = build_and_run_in_scratch("file-eof", source);
    assert_stdout("file-eof", &output, "impl-file: 2,2,1,0\n");
}

/// A span whose end lands exactly on the source length is legal.
///
/// The specification case only requests a span past the source, which traps.
/// The exact boundary must not trap, so the guard is `hi > len`, not `hi >=`.
#[test]
fn a_span_ending_on_the_source_length_is_legal() {
    let source = r#"module impl.span_edge;

import { Buffer, bufferNew, bufferBytes, Span, spanOf } from std.mem;
import { println } from std.io;

func main(): i32 {
    let buf: Buffer? = bufferNew(8);
    if (buf == null) {
        return 1;
    }
    let src: u8[] = bufferBytes(buf);
    let i: usize = 0;
    while (i < 8) {
        src[i] = u8(i);
        i = i + 1;
    }
    let atEnd: Span = spanOf(src, 3, 5);
    println($"impl-span: {i32(atEnd.start)} {i32(atEnd.len)} {i32(src[3])} {i32(src[7])}");
    return 0;
}
"#;
    let output = build_and_run("span-edge", source);
    assert_stdout("span-edge", &output, "impl-span: 3 5 3 7\n");
}

/// A zero-length span at the source end is legal and empty.
///
/// The offset equals the source length and the requested length is zero, which
/// the guard must accept while still producing an empty view.
#[test]
fn a_zero_length_span_at_the_source_end_is_empty() {
    let source = r#"module impl.span_empty;

import { Buffer, bufferNew, bufferBytes, Span, spanOf } from std.mem;
import { println } from std.io;

func main(): i32 {
    let buf: Buffer? = bufferNew(8);
    if (buf == null) {
        return 1;
    }
    let src: u8[] = bufferBytes(buf);
    let empty: Span = spanOf(src, 8, 0);
    println($"impl-span-empty: {i32(empty.start)} {i32(empty.len)}");
    return 0;
}
"#;
    let output = build_and_run("span-empty", source);
    assert_stdout("span-empty", &output, "impl-span-empty: 8 0\n");
}
