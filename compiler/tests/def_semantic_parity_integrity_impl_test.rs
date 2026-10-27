//! Emitted-shape and internal-boundary tests for the semantic parity batch.
//!
//! The specification suite states the observable behavior; these cases pin the
//! one-report counts, the absence of an element placeholder, the concrete and
//! generic view-member shapes, the read/return translation unit, and the
//! byte-identical re-emission the accepted fixes rely on.

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

    let generic_nullable = r#"module impl.sempar.st26generic;

import { String, stringOf } from std.text;

struct H { name: String?; }

func pick<T>(c: bool, xs: H[2]): i32 {
    let r: T = c ? xs[0].name : xs[0].name;
    return i32(len(r)) - 2;
}

func main(): i32 {
    let xs: H[2] = [H { name: stringOf("aa") }, H { name: null }];
    return pick<str>(true, xs);
}
"#;
    assert_eq!(count("st26generic", generic_nullable, "E10010"), 2);

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

    let owned_borrow = r#"module impl.sempar.st412owned;

import { String, stringOf } from std.text;

struct H { name: String; }

func getName<T>(borrow h: T): str { return h.name; }

func main(): i32 {
    let h: H = H { name: stringOf("ab") };
    let s: str = getName<H>(h);
    return i32(len(s)) - 2;
}
"#;
    assert_eq!(count("st412owned", owned_borrow, "E10036"), 1);

    let mixed_static = r#"module impl.sempar.st28mixed;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func pick(c: bool, borrow b: H): str {
    if (c) { return b.tag; }
    return b.name;
}

func main(): i32 {
    let b: H = H { name: stringOf("bb"), tag: "bb" };
    let s: str = pick(false, b);
    return i32(len(s)) - 2;
}
"#;
    assert_eq!(count("st28mixed", mixed_static, "E10036"), 1);

    let unnarrowed_loop = r#"module impl.sempar.st411unnarrow;

import { slice } from std.mem;

func f(a: u8[]?): i32 {
    let n: i32 = 0;
    for (x in a) { n = n + i32(x); }
    return n;
}

func main(): i32 {
    let b: u8[2] = [1, 2];
    let v: u8[] = slice(b, 0, 2);
    return f(v);
}
"#;
    assert_eq!(count("st411unnarrow", unnarrowed_loop, "E10010"), 1);

    let plain_second = r#"module impl.sempar.st412plain;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(borrow a: T, b: T): str[1] { return [a[0].name]; }

func main(): i32 {
    let hs: H[2] = [H { name: stringOf("aa") }, H { name: stringOf("bb") }];
    let s: str[1] = f<H[2]>(hs, hs);
    return i32(len(s[0])) - 2;
}
"#;
    assert_eq!(count("st412plain", plain_second, "E10036"), 1);
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
