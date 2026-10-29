//! Specification tests for the zero-defect borrow and loan batch.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-zeroloans-{}-{tag}-{sequence}",
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

fn compile(tag: &str, source: &str, sanitize: bool) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write source: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            c_path.to_str().expect("C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn clc");
    if output.status.code() != Some(0) || !sanitize {
        return (scratch, output, c_path);
    }
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("C path"),
        ])
        .output()
        .expect("failed to spawn cc");
    assert!(
        built.status.success(),
        "{tag}: ASan build failed:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    (scratch, output, program)
}

fn run_source(tag: &str, source: &str) -> Output {
    let (scratch, output, c_path) = compile(tag, source, false);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: front end rejected:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("C path"),
        ])
        .output()
        .expect("failed to spawn cc");
    assert!(
        built.status.success(),
        "{tag}: build failed:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new("timeout")
        .arg("10")
        .arg(&program)
        .output()
        .expect("failed to run program")
}

fn assert_no_sanitizer(tag: &str, output: &Output) {
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(
        !text.contains("ERROR: AddressSanitizer") && !text.contains("LeakSanitizer"),
        "{tag}: sanitizer report:\n{text}"
    );
}

#[test]
fn should_report_a_concrete_capture_move_as_e10401() {
    let source = r#"module spec.zl.capture;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("ab");
    ([s] () => { let y: String = s; })();
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile("capture", source, false);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10401:").count(), 1, "{stderr}");
    assert!(stderr.contains("cannot move captured 's'"), "{stderr}");
    assert!(!stderr.contains("E10034"), "{stderr}");
    assert!(!c_path.exists());
}

#[test]
fn should_reject_a_move_out_of_a_const_binding() {
    let source = r#"module spec.zl.constmove;

import { String, stringOf } from std.text;

func main(): i32 {
    const s: String = stringOf("ab");
    let t: String = s;
    return i32(len(t)) - 2;
}
"#;
    let (_scratch, output, c_path) = compile("constmove", source, false);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10023:").count(), 1, "{stderr}");
    assert!(stderr.contains("cannot move out of immutable 's'"), "{stderr}");
    assert!(!c_path.exists());
}

#[test]
fn should_release_the_operand_projection_root() {
    let operand = r#"module spec.zl.operand;

import { String, stringOf } from std.text;

struct Box { n: i32; s: String; }
func mkBox(v: i32): Box { return Box { n: v, s: stringOf("0123456789") }; }
func f(c: bool, b: Box): i32 { return 1 + (c ? b.n : mkBox(7).n); }

func main(): i32 {
    let b: Box = Box { n: 2, s: stringOf("AA") };
    return f(false, b) - 8;
}
"#;
    let output = run_source("operand", operand);
    assert_eq!(output.status.code(), Some(0));
    assert_no_sanitizer("operand", &output);

    let guarded = r#"module spec.zl.guarded;

import { String, stringOf } from std.text;

struct Box { ys: i32[2]; s: String; }
func mk(): Box { return Box { ys: [1, 2], s: stringOf("0123456789") }; }
func f(g: bool, c: bool, b: Box): i32 {
    if (g && i32((c ? mk() : b).ys[0]) > 0) { return 0; }
    return 0;
}

func main(): i32 {
    let b: Box = Box { ys: [2, 3], s: stringOf("BB") };
    return f(true, true, b);
}
"#;
    let output = run_source("guarded", guarded);
    assert_eq!(output.status.code(), Some(0));
    assert_no_sanitizer("guarded", &output);
}

#[test]
fn should_accept_an_unsafe_prefixed_raw_store() {
    let source = r#"module spec.zl.rawprefix;

struct Inner { x: i32; }
struct S { f: Inner; }
let g: S = S { f: Inner { x: 0 } };
func getp(): raw S { return unsafe &g; }

func main(): i32 {
    let p: raw S = getp();
    unsafe p.f = Inner { x: 7 };
    return g.f.x - 7;
}
"#;
    let output = run_source("rawprefix", source);
    assert_eq!(output.status.code(), Some(0));
    assert_no_sanitizer("rawprefix", &output);
}

#[test]
fn should_not_warn_for_a_raw_store_rhs_pointer() {
    let source = r#"module spec.zl.rawrhs;

import { String, stringOf } from std.text;

struct S { items: String; }

func main(): i32 {
    let a: S = S { items: stringOf("aa") };
    let b: S = S { items: stringOf("bb") };
    let p: raw S = unsafe &a;
    let q: raw S = unsafe &b;
    unsafe { (*p).items = (*q).items; }
    return i32(len(a.items)) - 2;
}
"#;
    let (_scratch, output, _c_path) = compile("rawrhs", source, false);
    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("W10000"), "{stderr}");
}

#[test]
fn should_report_each_later_use_statement_in_a_generic_body() {
    let source = r#"module spec.zl.lateruse;

import { String, stringOf } from std.text;

func sink(take s: String): i32 { return 0; }

func generic<T>(h: T): i32 {
    let i: i32 = 0;
    while (i < 1) {
        sink(take h);
        sink(take h);
        i = i + 1;
    }
    return 0;
}

func main(): i32 {
    let s: String = stringOf("ab");
    return generic<String>(s);
}
"#;
    let (_scratch, output, c_path) = compile("lateruse", source, false);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10020:").count(), 2, "{stderr}");
    assert!(!c_path.exists());
}

#[test]
fn should_not_warn_for_a_move_only_array_lend() {
    let source = r#"module spec.zl.arraylend;

import { String, stringOf } from std.text;

struct S { y: String; }
func w2(borrow xs: S[2]): i32 { xs[0].y = stringOf("aa"); return 0; }

func main(): i32 {
    let xs: S[2] = [S { y: stringOf("aa") }, S { y: stringOf("bb") }];
    w2(borrow xs);
    return 0;
}
"#;
    let (_scratch, output, _c_path) = compile("arraylend", source, false);
    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("W10001"), "{stderr}");
}

#[test]
fn should_keep_the_overwritten_element_loan_cleared() {
    let source = r#"module spec.zl.overwrite;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Holder { names: str[2]; }

func main(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = Holder { names: [a.v, "EE"] };
    h.names[0] = "FF";
    return i32(h.names[1][0]) - 69;
}
"#;
    let output = run_source("overwrite", source);
    assert_eq!(output.status.code(), Some(0));
    assert_no_sanitizer("overwrite", &output);
}
