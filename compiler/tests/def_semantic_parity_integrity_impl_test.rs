//! Emitted-shape and internal-boundary tests for the semantic parity batch.
//!
//! The specification suite states the observable behavior; these cases pin the
//! one-report counts, the absence of an element placeholder, the concrete and
//! generic view-member shapes, the read/return translation unit, and the
//! byte-identical re-emission the accepted fixes rely on.

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
            "clc-semparimpl-{}-{tag}-{sequence}",
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
/// Returns the emitted translation unit. These cases are about the text the
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

/// Emits one program twice from the same source path, returning both texts.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
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
        emitted.push(std::fs::read_to_string(&c_path).unwrap_or_default());
    }
    (emitted[0].clone(), emitted[1].clone())
}

/// Compiles a program and returns the compiler output and the C path.
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

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
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
    run_program(tag, &program, &[])
}

/// Runs a built program with a time limit and the given environment values.
///
/// A program that is still running when the limit passes is killed and the case
/// fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_program(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
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

/// Asserts the first fragment appears before the second one in the text.
fn assert_before(tag: &str, text: &str, first: &str, second: &str) {
    let first_at = text
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {first:?}"));
    let second_at = text
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {second:?}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}"
    );
}

/// Asserts the front end rejects the program with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
    assert_rejected_with(tag, source, code, &[]);
}

/// Asserts the front end rejects the program with `code`, that the message
/// contains every fragment, and that no translation unit is written.
fn assert_rejected_with(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: the diagnostic does not contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}


/// The rejected sites report exactly once per site or per nullable arm.
#[test]
fn should_pin_the_one_report_counts() {
    fn count(tag: &str, source: &str, code: &str) -> usize {
        let (_scratch, output, c_path) = compile_only(tag, source);
        assert_ne!(
            output.status.code(),
            Some(0),
            "{tag}: the front end accepted a program that must be rejected"
        );
        assert!(
            !c_path.exists(),
            "{tag}: a rejected program must not produce a translation unit"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        stderr.matches(&format!("error {code}:")).count()
    }

    let nullable_arms = r#"module impl.sempar.st26;

import { String, stringOf } from std.text;

struct H { name: String?; }

func main(): i32 {
    let xs: H[2] = [H { name: stringOf("aa") }, H { name: null }];
    let c: bool = true;
    let s: str = c ? xs[0].name : xs[0].name;
    return i32(len(s)) - 2;
}
"#;
    assert_eq!(count("st26", nullable_arms, "E10010"), 2);

    let owning_projection = r#"module impl.sempar.st27b;

import { String, stringOf } from std.text;

struct Inner { n: String; }
struct H { inner: Inner; }

func g<T, U>(c: bool, take h: T, take d: U): U {
    return c ? h.inner.n : d;
}

func main(): i32 {
    let h: H = H { inner: Inner { n: stringOf("aa") } };
    let d: String = stringOf("bb");
    let r: String = g<H, String>(true, take h, take d);
    return i32(len(r)) - 2;
}
"#;
    assert_eq!(count("st27b", owning_projection, "E10026"), 1);

    let lend_projection = r#"module impl.sempar.st27c;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct H { inner: Inner; }

func g<T, U>(c: bool, borrow h: T, take d: U): U {
    return c ? h.inner.name : d;
}

func main(): i32 {
    let h: H = H { inner: Inner { name: stringOf("aa") } };
    let d: String = stringOf("bb");
    let r: String = g<H, String>(true, h, take d);
    return i32(len(r)) - 2;
}
"#;
    assert_eq!(count("st27c", lend_projection, "E10034"), 1);

    let owned_view = r#"module impl.sempar.st28name;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func nameView(borrow h: H): str {
    return h.name;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: "aa" };
    let s: str = nameView(h);
    return i32(len(s)) - 2;
}
"#;
    assert_eq!(count("st28name", owned_view, "E10036"), 1);

    let inline_alias = r#"module impl.sempar.st30;

import { String, stringOf } from std.text;

struct B { v: String; }

func make(borrow s: String): str {
    return s;
}

func go(borrow b: B): str {
    for (borrow e in [make(b.v)]) {
        return e;
    }
    return "";
}

func main(): i32 {
    let b: B = B { v: stringOf("aa") };
    let s: str = go(b);
    return i32(len(s)) - 2;
}
"#;
    assert_eq!(count("st30", inline_alias, "E10030"), 1);

    let local_owned = r#"module impl.sempar.st31localowned;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func pack<T>(d: bool, borrow h1: T, borrow h2: T): str {
    let l: H = H { name: stringOf("ll"), tag: "ll" };
    return (d ? h1 : l).name;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    let s: str = pack<H>(true, h1, h2);
    return i32(len(s)) - 2;
}
"#;
    assert_eq!(count("st31localowned", local_owned, "E10035"), 1);
}

/// A nested element substitution never reaches the emitter as a placeholder.
#[test]
fn should_not_emit_an_element_placeholder() {
    let source = r#"module impl.sempar.st25;

func f<T>(xs: T[2]): i32 {
    let n: i32 = 0;
    for (v in xs) {
        for (w in v) {
            n = n + i32(w);
        }
    }
    return n;
}

func main(): i32 {
    let m: u8[2][2] = [[1, 2], [3, 4]];
    return i32(f<u8[2]>(m)) - 10;
}
"#;
    let emitted = emit_host("st25", source);
    assert_absent("st25", &emitted, "#element");
    assert_contains("st25", &emitted, "cl_f__arr2_u8");
}

/// The generic view-member move emits the same static tag pointer access as
/// its concrete twin.
#[test]
fn should_match_the_concrete_and_generic_view_member_shape() {
    let generic = r#"module impl.sempar.st31;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { p: raw u8; }

func pack<T>(d: bool, borrow h1: T, borrow h2: T): S {
    return S { p: (d ? h1 : h2).tag.ptr };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let s: S = pack<H>(true, h1, h2);
        if (s.p == null) { return 9; }
    }
    return 0;
}
"#;
    let emitted = emit_host("st31", generic);
    assert_contains("st31", &emitted, "cl_pack__H");
    assert_contains("st31", &emitted, ".tag.ptr");

    let concrete = r#"module impl.sempar.st31cb;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { p: raw u8; }

func packcb(d: bool, borrow h1: H, borrow h2: H): S {
    return S { p: (d ? h1 : h2).tag.ptr };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let s: S = packcb(true, h1, h2);
        if (s.p == null) { return 9; }
    }
    return 0;
}
"#;
    let emitted_concrete = emit_host("st31cb", concrete);
    assert_contains("st31cb", &emitted_concrete, ".tag.ptr");
    assert_contains("st31cb", &emitted_concrete, "(u8 *)");
}

/// The read/return view-member form emits the instance's selected tag pointer.
#[test]
fn should_emit_the_read_return_view_member() {
    let source = r#"module impl.sempar.st31read;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func readTag<T>(d: bool, borrow h1: T, borrow h2: T): raw u8 {
    return (d ? h1 : h2).tag.ptr;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let p: raw u8 = readTag<H>(true, h1, h2);
        if (p == null) { return 9; }
    }
    return 0;
}
"#;
    let emitted = emit_host("st31read", source);
    assert_contains("st31read", &emitted, "cl_readTag__H");
    assert_contains("st31read", &emitted, "static u8 *");
    assert_contains("st31read", &emitted, ".tag.ptr");
}

/// Re-emitting the accepted generic program is byte-identical.
#[test]
fn should_reemit_the_semantic_parity_program_identically() {
    let source = r#"module impl.sempar.determinism;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }
struct S { p: raw u8; }

func pack<T>(d: bool, borrow h1: T, borrow h2: T): S {
    return S { p: (d ? h1 : h2).tag.ptr };
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa"), tag: "aa" };
    let h2: H = H { name: stringOf("bb"), tag: "bb" };
    unsafe {
        let s: S = pack<H>(true, h1, h2);
        if (s.p == null) { return 9; }
    }
    return 0;
}
"#;
    let (first, second) = emit_twice("determinism", source);
    assert_eq!(first, second, "the emitted translation unit is not stable");
}
