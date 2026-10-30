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

#[test]
fn should_keep_a_loan_on_another_member_accepted() {
    let accepted = r#"module spec.zl.memberloan;

import { String, stringOf } from std.text;

struct Box { v: i32; w: String; }
struct Holder { names: str[1]; }

func main(): i32 {
    let b: Box = Box { v: 1, w: stringOf("AA") };
    let h: Holder = Holder { names: [b.w] };
    b.v = 2;
    return i32(h.names[0][0]) - 65;
}
"#;
    let output = run_source("memberloan", accepted);
    assert_eq!(output.status.code(), Some(0));
    assert_no_sanitizer("memberloan", &output);

    let rejected = r#"module spec.zl.memberloanwrite;

import { String, stringOf } from std.text;

struct Box { v: i32; w: String; }
struct Holder { names: str[1]; }

func main(): i32 {
    let b: Box = Box { v: 1, w: stringOf("AA") };
    let h: Holder = Holder { names: [b.w] };
    b.w = stringOf("ZZ");
    return i32(h.names[0][0]) - 65;
}
"#;
    let (_scratch, output, c_path) = compile("memberloanwrite", rejected, false);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10031:").count(), 1, "{stderr}");
    assert!(!c_path.exists());
}

#[test]
fn should_reject_a_loop_head_take_through_a_raw_deref() {
    let source = r#"module spec.zl.looptake;

import { String, stringOf } from std.text;

func eat(take s: String): i32 { return i32(len(s)); }

func loopPlaceTake(p: raw String): i32 {
    let i: i32 = 0;
    while (i < 2 && eat(take unsafe *p) > 0) {
        i = i + 1;
    }
    return i - 2;
}

func main(): i32 {
    let s: String = stringOf("aa");
    let p: raw String = unsafe &s;
    return loopPlaceTake(p);
}
"#;
    let (_scratch, output, c_path) = compile("looptake", source, false);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10020:").count(), 1, "{stderr}");
    assert!(stderr.contains("loop-head take"), "{stderr}");
    assert!(!c_path.exists());
}

#[test]
fn should_not_report_an_escape_for_a_local_conditional_literal_arm() {
    let source = r#"module spec.zl.condliteral;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Holder { names: str[2]; }

func use(c: bool): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let h3: Holder = Holder { names: ["CC", "DD"] };
    let h: Holder = c ? h3 : Holder { names: [b.v, "EE"] };
    b.v = stringOf("zz");
    return i32(h.names[0][0]);
}

func main(): i32 { return use(true) - 99; }
"#;
    let (_scratch, output, c_path) = compile("condliteral", source, false);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10031:").count(), 1, "{stderr}");
    assert!(!stderr.contains("E10033"), "{stderr}");
    assert!(!c_path.exists());
}

#[test]
fn should_accept_a_proven_static_take_carrier() {
    let inline = r#"module spec.zl.takestaticinline;

struct Holder { names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    store(Holder { names: ["AA", "BB"] }, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    let output = run_source("takestaticinline", inline);
    assert_eq!(output.status.code(), Some(0));

    let alias = r#"module spec.zl.takealias;

struct Holder { names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    let hs: Holder[1] = [Holder { names: ["AA", "BB"] }];
    for (borrow e in hs) { store(take e, borrow out); }
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    let output = run_source("takealias", alias);
    assert_eq!(output.status.code(), Some(0));

    let factory = r#"module spec.zl.takefactory;

struct Holder { names: str[2]; }
func makeStatic(): Holder { return Holder { names: ["AA", "BB"] }; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    let h: Holder = makeStatic();
    store(take h, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    let output = run_source("takefactory", factory);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn should_fail_closed_for_unproven_take_carriers() {
    let call_result = r#"module spec.zl.takecallresult;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    let a: Box = Box { v: stringOf("aa") };
    store(make(a.v), borrow out);
    return 0;
}
"#;
    let temporary = r#"module spec.zl.taketemporary;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }
func fill(borrow out: str?): void {
    store(Holder { names: [stringOf("AA"), "BB"] }, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let dot_call = r#"module spec.zl.takedot;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = Holder { names: [a.v, "EE"] };
    h.store(borrow out);
    return 0;
}
"#;
    let value_call = r#"module spec.zl.takevalue;

struct Holder { names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    let f: func(take Holder, borrow str?): void = store;
    let h: Holder = Holder { names: ["AA", "BB"] };
    f(take h, borrow out);
    return 0;
}
"#;
    for (tag, source) in [
        ("takecallresult", call_result),
        ("taketemporary", temporary),
        ("takedot", dot_call),
        ("takevalue", value_call),
    ] {
        let (_scratch, output, c_path) = compile(tag, source, false);
        assert_ne!(output.status.code(), Some(0), "{tag}: accepted");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr.matches("error E10030:").count(),
            1,
            "{tag}:\n{stderr}"
        );
        assert!(!c_path.exists(), "{tag}: emitted a translation unit");
    }
}

#[test]
fn should_fail_closed_for_laundered_take_carriers() {
    let global_value = r#"module spec.zl.takeglobalvalue;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }
let g: func(take Holder, borrow str?): void = store;

func fill(borrow out: str?): void {
    g(Holder { names: [stringOf("QQ"), "BB"] }, borrow out);
}

func main(): i32 {
    let staticOut: str? = null;
    let out: str? = null;
    store(Holder { names: ["AA", "BB"] }, borrow staticOut);
    fill(borrow out);
    return 0;
}
"#;
    let call_result_local = r#"module spec.zl.takecallresultlocal;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
func id(v: str): Holder { return Holder { names: [v, "BB"] }; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let h: Holder = id(stringOf("QQ"));
    store(take h, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let raw_write = r#"module spec.zl.takerawwrite;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = Holder { names: ["AA", "BB"] };
    unsafe { let hp: raw Holder = &h; (*hp).names[0] = a.v; }
    store(take h, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let raw_view = r#"module spec.zl.takerawview;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let v: str = "AA";
    let s: String = stringOf("QQ");
    let p: raw str = unsafe &v;
    unsafe { *p = s; }
    store(Holder { names: [v, "BB"] }, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let ffi_view = r#"module spec.zl.takeffiview;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
ffi probe { func Poke(p: raw str, v: str): void; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let v: str = "AA";
    let s: String = stringOf("QQ");
    let p: raw str = unsafe &v;
    unsafe { Poke(p, s); }
    store(Holder { names: [v, "BB"] }, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let borrow_write = r#"module spec.zl.takeborrowwrite;

import { String, stringOf } from std.text;

struct Holder { label: String; names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }
func corrupt(borrow h: Holder, borrow v: str): void {
    unsafe { let p: raw Holder = &h; (*p).names[0] = v; }
}

func fill(borrow out: str?): void {
    let s: String = stringOf("QQ");
    let h: Holder = Holder { label: stringOf("x"), names: ["AA", "BB"] };
    corrupt(borrow h, borrow s);
    store(take h, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let capture_write = r#"module spec.zl.takecapturewrite;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = Holder { names: ["AA", "BB"] };
    ([borrow h, borrow a] () => { h.names[0] = a.v; })();
    store(take h, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let factory_shadow = r#"module spec.zl.takefactoryshadow;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
func make(): Holder {
    let h: Holder = Holder { names: ["AA", "BB"] };
    let v: String = stringOf("aa");
    unsafe { let hp: raw Holder = &h; (*hp).names[0] = v; }
    return h;
}
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let h: Holder = Holder { names: ["CC", "DD"] };
    let x: Holder = make();
    store(take x, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let capture_address = r#"module spec.zl.takecaptureaddress;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = Holder { names: ["AA", "BB"] };
    ([borrow h, borrow a] () => {
        unsafe { let p: raw Holder = &h; (*p).names[0] = a.v; }
    })();
    store(take h, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    for (tag, source) in [
        ("takeglobalvalue", global_value),
        ("takecallresultlocal", call_result_local),
        ("takerawwrite", raw_write),
        ("takerawview", raw_view),
        ("takeffiview", ffi_view),
        ("takeborrowwrite", borrow_write),
        ("takecapturewrite", capture_write),
        ("takefactoryshadow", factory_shadow),
        ("takecaptureaddress", capture_address),
    ] {
        let (_scratch, output, c_path) = compile(tag, source, false);
        assert_ne!(output.status.code(), Some(0), "{tag}: accepted");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr.matches("error E10030:").count(),
            1,
            "{tag}:\n{stderr}"
        );
        assert!(!c_path.exists(), "{tag}: emitted a translation unit");
    }
}
