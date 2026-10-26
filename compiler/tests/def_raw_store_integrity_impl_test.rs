//! Implementation tests for the raw and conditional store/alias batch.
//!
//! The cases pin the emitted save, capture, containment, self-alias, and
//! element-state shapes that the specification suite reaches only through
//! behavior. Each case emits a small Clorraine program with the built `clc`
//! binary, reads the generated C, and builds it with the strict host flags so
//! an accepted program cannot emit invalid C. The determinism case re-emits one
//! accepted program and compares the two units byte for byte.
//!
//! All temporary files live in a self-removing scratch directory.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one emitted program may run before the case fails as a hang.
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
            "clc-rawstoreimpl-{}-{tag}-{sequence}",
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

/// Writes `source` into the scratch directory and returns its path.
fn write_source(tag: &str, scratch: &ScratchDir, source: &str) -> PathBuf {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    source_path
}

/// Compiles `source_path` into `c_path` and returns the emitted text.
fn emit_at(tag: &str, source_path: &Path, c_path: &Path) -> String {
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
    let emitted = std::fs::read_to_string(c_path).unwrap_or_default();
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    emitted
}

/// Emits one host-profile program and asserts the front end accepted it.
///
/// The scratch directory is returned so it outlives the emitted text and can
/// host the strict C build.
fn emit_host(tag: &str, source: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let emitted = emit_at(tag, &source_path, &scratch.path().join("out.c"));
    (scratch, emitted)
}

/// Emits the same program twice from one source path, returning both units.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let first = emit_at(tag, &source_path, &scratch.path().join("out0.c"));
    let second = emit_at(tag, &source_path, &scratch.path().join("out1.c"));
    (first, second)
}

/// Asserts the emitted C contains `needle`.
fn assert_contains(tag: &str, emitted: &str, needle: &str) {
    assert!(
        emitted.contains(needle),
        "{tag}: expected the emitted C to contain {needle:?}; emitted C:\n{emitted}"
    );
}

/// Asserts the emitted C does not contain `needle`.
fn assert_absent(tag: &str, emitted: &str, needle: &str) {
    assert!(
        !emitted.contains(needle),
        "{tag}: expected the emitted C to lack {needle:?}; emitted C:\n{emitted}"
    );
}

/// Asserts the emitted line with `anchor` also contains `needle`.
fn assert_on_line_of(tag: &str, emitted: &str, anchor: &str, needle: &str) {
    let line = emitted
        .lines()
        .find(|line| line.contains(anchor))
        .unwrap_or_else(|| panic!("{tag}: no emitted line contains {anchor:?}"));
    assert!(
        line.contains(needle),
        "{tag}: expected the line with {anchor:?} to contain {needle:?}; line:\n{line}"
    );
}

/// Asserts `second` follows `first` with only `#line` markers between them;
/// any other text means a statement intervenes.
fn assert_follows(tag: &str, emitted: &str, first: &str, second: &str) {
    let start = emitted
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: {first:?} is missing; emitted C:\n{emitted}"));
    let after = &emitted[start + first.len()..];
    let second_at = after.find(second).unwrap_or_else(|| {
        panic!("{tag}: {second:?} does not follow {first:?}; emitted C:\n{emitted}")
    });
    assert!(
        after[..second_at]
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("#line")),
        "{tag}: a statement intervenes between {first:?} and {second:?}; emitted C:\n{emitted}"
    );
}

/// The body of the function whose emitted signature is `signature`.
///
/// Nested blocks close with an indented brace, so the first bare `}` closes
/// the function itself.
fn function_body<'a>(tag: &str, emitted: &'a str, signature: &str) -> &'a str {
    let start = emitted
        .find(signature)
        .unwrap_or_else(|| panic!("{tag}: the emitted C does not contain {signature:?}"));
    let text = &emitted[start + signature.len()..];
    let end = text
        .find("\n}")
        .unwrap_or_else(|| panic!("{tag}: the body after {signature:?} has no closing brace"));
    &text[..end]
}

/// Builds the emitted C with the strict host flags and returns the program.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).unwrap_or_else(|err| {
        panic!("{tag}: cannot write the emitted C: {err}");
    });
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

/// Runs a built program with a time limit, returning its exit code.
fn run_with_timeout(tag: &str, program: &Path) -> i32 {
    let mut child = Command::new(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code().unwrap_or(-1),
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
}

/// Builds and runs one emitted program under the strict host flags.
fn run_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> i32 {
    let program = build_host_c(tag, scratch, emitted);
    run_with_timeout(tag, &program)
}

/// A guarded literal argument moves the raw-deref field and empties the
/// pointee on the guard's evaluated path.
#[test]
fn should_emit_the_guarded_literal_field_move() {
    let tag = "guardedfield";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.guardedfield;

import { String, stringOf } from std.text;

struct H { s: String; }

func consume(take h: H): i32 {
    return i32(len(h.s));
}

func main(): i32 {
    let h: H = H { s: stringOf("aa") };
    let g: bool = true;
    unsafe {
        let p: raw H = &h;
        if (g && consume(H { s: (*p).s }) == 2) {
            return 0;
        }
    }
    return 9;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_guardedfield_main(void)\n{",
    );
    assert_on_line_of(tag, body, "if ((g &&", ".s = (*(p)).s");
    assert_on_line_of(tag, body, "if ((g &&", "(*(p)).s = NULL)");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A fixed-array conditional store carries the take capture and clears the
/// saved subrange after the condition.
#[test]
fn should_emit_the_fixed_array_take_capture() {
    let tag = "arraycapture";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.arraycapture;

import { String, stringOf } from std.text;

struct A { items: String[2]; }

func eatRows(take rows: String[2]): i32 {
    return i32(len(rows[0])) - 2;
}

func main(): i32 {
    let a: A = A { items: [stringOf("aa"), stringOf("bb")] };
    let x: String[2] = [stringOf("xx"), stringOf("yy")];
    let y: String[2] = [stringOf("mm"), stringOf("nn")];
    let g: bool = true;
    unsafe {
        let p: raw A = &a;
        a.items = (g && eatRows(take (*p).items) == 0) ? x : y;
    }
    if (i32(len(a.items[0])) != 2) {
        return 9;
    }
    return 0;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_arraycapture_main(void)\n{",
    );
    assert_contains(tag, body, "ClString __cl_save_1[2] = {0};");
    assert_contains(tag, body, "bool __cl_take_alias_1 = 0;");
    assert_contains(tag, body, "bool __cl_cond_1 =");
    assert_contains(
        tag,
        body,
        "memset((char *)(&(__cl_save_1)) + (__cl_take_off_1), 0, (__cl_take_len_1));",
    );
    assert_contains(tag, body, "eatRows((*(p)).items)");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// The containment test compares integer addresses in both directions and
/// clears only the taken part of the saved value.
#[test]
fn should_emit_both_containment_directions() {
    let super_object = r#"module spec.rawstore.containsuper;

import { String, stringOf } from std.text;

struct Elem { s: String; }
struct Pair { head: i32; tail: Elem; }

func eatPair(take p: Pair): i32 { return 1; }

func main(): i32 {
    let pair: Pair = Pair { head: 7, tail: Elem { s: stringOf("aa") } };
    let x: Elem = Elem { s: stringOf("xx") };
    let y: Elem = Elem { s: stringOf("yy") };
    let g: bool = true;
    unsafe {
        let p: raw Pair = &pair;
        pair.tail = (g && eatPair(take (*p)) == 1) ? x : y;
    }
    return i32(len(pair.tail.s)) - 2;
}
"#;
    let tag = "containsuper";
    let (scratch, emitted) = emit_host(tag, super_object);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_containsuper_main(void)\n{",
    );
    assert_contains(tag, body, "unsigned long __cl_take_off_1 = 0;");
    assert_contains(tag, body, "unsigned long __cl_take_len_1 = 0;");
    assert_contains(tag, body, "sizeof(");
    assert!(
        body.matches("((unsigned long)(").count() > 2,
        "{tag}: the containment test must use integer addresses; body:\n{body}"
    );
    assert_contains(tag, body, "memset((char *)(&(__cl_save_1))");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");

    let first_member = r#"module spec.rawstore.containmember;

import { String, stringOf } from std.text;

struct Pair { first: String; sibling: String; }

func eat(take s: String): i32 { return 1; }

func main(): i32 {
    let pair: Pair = Pair { first: stringOf("aa"), sibling: stringOf("77") };
    let x: Pair = Pair { first: stringOf("xx"), sibling: stringOf("88") };
    let y: Pair = Pair { first: stringOf("yy"), sibling: stringOf("99") };
    let g: bool = true;
    unsafe {
        let p: raw Pair = &pair;
        pair = (g && eat(take (*p).first) == 1) ? x : y;
    }
    return i32(len(pair.sibling)) - 2;
}
"#;
    let tag = "containmember";
    let (scratch, emitted) = emit_host(tag, first_member);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_containmember_main(void)\n{",
    );
    assert_contains(tag, body, "unsigned long __cl_take_off_1 = 0;");
    assert_contains(tag, body, "memset((char *)(&(__cl_save_1))");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A nested materialization keeps the outer capture live and clears the outer
/// save when the inner take consumes it.
#[test]
fn should_keep_the_outer_capture_in_the_stack() {
    let tag = "nestedcapture";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.nestedcapture;

import { String, stringOf } from std.text;

func inner(take s: String): i32 { return i32(len(s)); }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let z: String = stringOf("zzzzzz");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = (g && inner(take (g ? (*p) : z)) == 2) ? x : y;
    }
    return i32(len(v)) - 2;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_nestedcapture_main(void)\n{",
    );
    assert_contains(tag, body, "ClString __cl_save_1 = v;");
    assert_contains(tag, body, "bool __cl_take_alias_1 = 0;");
    assert_contains(tag, body, "memset((char *)(&(__cl_save_1))");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A raw self-alias restores the save instead of emptying the destination.
#[test]
fn should_restore_the_save_on_a_raw_self_alias() {
    let tag = "rawselfalias";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.rawselfalias;

import { String, stringOf } from std.text;

func main(): i32 {
    let v: String = stringOf("seven77");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = g ? (*p) : y;
        return i32(len(v));
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_rawselfalias_main(void)\n{",
    );
    assert_contains(tag, body, "if (((void *)(p)) == ((void *)(&(v)))) {");
    assert_contains(tag, body, "__cl_save_1 = NULL;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 7, "{tag}: the value must survive");
}

/// The fixed-array parameter alias guard uses the adjusted source address and
/// clears the duplicate save.
#[test]
fn should_guard_the_adjusted_fixed_array_parameter_address() {
    let tag = "arrayaddress";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.arrayaddress;

import { String, stringOf } from std.text;

struct S { items: String[2]; }

func copyInto<T>(p: raw T, take items: String[2]): i32 {
    unsafe {
        (*p).items = items;
        return i32(len((*p).items[0])) - 2;
    }
}

func main(): i32 {
    let a: S = S { items: [stringOf("aa"), stringOf("bb")] };
    unsafe {
        return copyInto<S>(&a, a.items);
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_copyInto__S(ClS *p, ClString items[2])\n{",
    );
    assert_contains(
        tag,
        body,
        "if (((void *)(items)) == ((void *)(&((*(&((*(p)).items))))))) {",
    );
    assert_contains(tag, body, "cl_zero(&(__cl_save_1));");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// An intermediate raw field is pinned before the side-effecting RHS, and the
/// store writes through the pinned address.
#[test]
fn should_pin_the_intermediate_raw_field() {
    let tag = "intermediatepin";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.intermediatepin;

struct Inner { x: i32; }
struct Outer { next: Inner; }

let calls: i32 = 0;

func change(p: raw Outer): i32 {
    unsafe {
        (*p).next = Inner { x: 9 };
    }
    calls = calls + 1;
    return 3;
}

func main(): i32 {
    let a: Outer = Outer { next: Inner { x: 0 } };
    unsafe {
        let p: raw Outer = &a;
        (*p).next.x = change(p);
    }
    if (a.next.x != 3) { return 9; }
    if (calls != 1) { return 8; }
    return a.next.x;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_intermediatepin_main(void)\n{",
    );
    assert_contains(tag, body, "__cl_tmp0 = &((*(p)).next);");
    assert_contains(
        tag,
        body,
        "*(&((*(__cl_tmp0)).x)) = cl_spec_rawstore_intermediatepin_change(p);",
    );
    assert_follows(
        tag,
        body,
        "__cl_tmp0 = &((*(p)).next);",
        "*(&((*(__cl_tmp0)).x)) = cl_spec_rawstore_intermediatepin_change(p);",
    );
    assert_eq!(
        body.matches("cl_spec_rawstore_intermediatepin_change(p);").count(),
        1,
        "{tag}: the side-effecting RHS must evaluate once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 3, "{tag}: the program must run");
}

/// A duplicable fixed-array literal that reads its destination fills a
/// temporary before the array is copied over the destination.
#[test]
fn should_evaluate_the_duplicable_self_read_into_a_temporary() {
    let tag = "duplicableselfread";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.duplicableselfread;

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [3, 4]];
    a[0] = [a[0][1], a[0][0]];
    if (a[0][0] != 2 || a[0][1] != 1) { return 9; }
    a[0] = [; a[0][0] + a[0][1]];
    if (a[0][0] != 3 || a[0][1] != 3) { return 8; }
    return 0;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_duplicableselfread_main(void)\n{",
    );
    assert_contains(tag, body, "i32 __cl_rhs_");
    assert_contains(tag, body, "[2] = {0};");
    assert_contains(tag, body, "[0] = a[0][1];");
    assert_contains(tag, body, "[1] = a[0][0];");
    assert_contains(tag, body, "memcpy(&(a[0]), &(__cl_rhs_");
    assert_eq!(
        body.matches("(i32)((u32)a[0][0] + (u32)a[0][1]);").count(),
        2,
        "{tag}: the fill reads the old destination once per element; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A raw element source that aliases the destination keeps the value; the
/// source is emptied only on the distinct-storage path.
#[test]
fn should_test_the_raw_element_source_address() {
    let tag = "elementaddress";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.elementaddress;

import { String, stringOf } from std.text;

let choose: bool = true;

func alt(s: raw String, t: raw String): raw String {
    if (choose) { return s; }
    return t;
}

func main(): i32 {
    let xs: String[2] = [stringOf("aa"), stringOf("bb")];
    let other: String[2] = [stringOf("cc"), stringOf("dd")];
    unsafe {
        let p: raw String = &xs[0];
        let q: raw String = &other[0];
        xs[0] = *alt(p, q);
    }
    return i32(len(xs[0])) - 2;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_elementaddress_main(void)\n{",
    );
    assert_contains(tag, body, "ClString __cl_save_1 = xs[0];");
    assert_contains(
        tag,
        body,
        "if (((void *)(__cl_tmp0)) == ((void *)(&(xs[0])))) {",
    );
    assert_contains(tag, body, "*(__cl_tmp0) = NULL;");
    assert_contains(tag, body, "if (__cl_save_1) { FreeString(__cl_save_1); }");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A named-place take records the save flag only where the take actually runs,
/// so a skipped short-circuit right leaves the old value owned.
#[test]
fn should_record_the_place_take_on_the_evaluated_path() {
    let tag = "placetake";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.placetake;

import { String, stringOf } from std.text;

func eat(take v: String): i32 { return 2; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    v = (g || eat(take v) == 2) ? x : y;
    return i32(len(v)) - 2;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_placetake_main(void)\n{",
    );
    assert_contains(tag, body, "bool __cl_take_alias_1 = 0;");
    assert_contains(tag, body, "__cl_take_alias_1 = 1");
    assert_contains(tag, body, "(v = NULL)");
    assert_contains(tag, body, "memset((char *)(&(__cl_save_1))");
    let condition = body
        .find("bool __cl_cond_1")
        .unwrap_or_else(|| panic!("{tag}: the condition local is missing; body:\n{body}"));
    assert_absent(tag, &body[..condition], "__cl_save_1 = NULL;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A fixed-array parameter take measures the whole array extent.
///
/// The parameter name is the array address, so the taken range must use the
/// element size times the first extent; one element would leave the second
/// element in the save and double-free it.
#[test]
fn should_measure_a_fixed_array_parameter_take_by_its_whole_extent() {
    let tag = "arrayextent";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.arrayextent;

import { String, stringOf } from std.text;

struct A { flag: i32; items: String[2]; }

func eatRows(take rows: String[2]): i32 { return i32(len(rows[0])) - 2; }

func go(p: raw A, take rows: String[2]): i32 {
    let x: String[2] = [stringOf("xx"), stringOf("yy")];
    let y: String[2] = [stringOf("mm"), stringOf("nn")];
    let g: bool = true;
    unsafe {
        (*p).items = (g && (*p).flag == 7 && eatRows(take rows) == 0) ? x : y;
    }
    return 0;
}

func main(): i32 {
    let a: A = A { flag: 7, items: [stringOf("aa"), stringOf("bb")] };
    unsafe { return go(&a, a.items); }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_arrayextent_go(ClA *p, ClString rows[2])\n{",
    );
    assert_contains(tag, body, "(sizeof(rows[0]) * 2)");
    assert_contains(tag, body, "memset((char *)(&(__cl_save_1)) + (__cl_take_off_1), 0, (__cl_take_len_1));");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A call nested in the RHS index still pins the intermediate raw field before
/// the call.
#[test]
fn should_pin_the_intermediate_field_before_an_indexed_call() {
    let tag = "indexedpin";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.indexedpin;

struct Inner { x: i32; }
struct Outer { next: Inner; }

let arr: i32[2] = [5, 3];

func change(p: raw Outer): i32 {
    unsafe { (*p).next = Inner { x: 9 }; }
    return 1;
}

func main(): i32 {
    let a: Outer = Outer { next: Inner { x: 0 } };
    unsafe {
        let p: raw Outer = &a;
        (*p).next.x = arr[change(p)];
    }
    if (a.next.x != 3) { return 9; }
    return 0;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_rawstore_indexedpin_main(void)\n{",
    );
    assert_contains(tag, body, "__cl_tmp0 = &((*(p)).next);");
    let pin = body.find("__cl_tmp0 = &((*(p)).next);").expect("pin");
    let call = body
        .find("cl_spec_rawstore_indexedpin_change(p)")
        .expect("indexed call");
    assert!(pin < call, "{tag}: the pin must precede the indexed call; body:\n{body}");
    assert_eq!(
        body.matches("cl_spec_rawstore_indexedpin_change(p)").count(),
        1,
        "{tag}: the index call must evaluate once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// The accepted shapes re-emit byte for byte.
#[test]
fn should_re_emit_the_batch_byte_identically() {
    let source = r#"module spec.rawstore.deterministic;

import { String, stringOf } from std.text;

struct Elem { s: String; }
struct Pair { head: i32; tail: Elem; }

func eatPair(take p: Pair): i32 { return 1; }

func main(): i32 {
    let pair: Pair = Pair { head: 7, tail: Elem { s: stringOf("aa") } };
    let x: Elem = Elem { s: stringOf("xx") };
    let y: Elem = Elem { s: stringOf("yy") };
    let g: bool = true;
    unsafe {
        let p: raw Pair = &pair;
        pair.tail = (g && eatPair(take (*p)) == 1) ? x : y;
    }
    return i32(len(pair.tail.s)) - 2;
}
"#;
    let (first, second) = emit_twice("deterministic", source);
    assert!(
        first == second,
        "deterministic: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// A conditional arm whose literal reads the destination fills the temporary
/// inside the chosen branch, before the bytes are copied over.
#[test]
fn should_fill_a_conditional_arm_self_read_into_the_temporary() {
    let tag = "condrhs";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.condrhs;

func main(): i32 {
    let a: i32[2] = [1, 2];
    let y: i32[2] = [7, 8];
    let c: bool = true;
    a = c ? [a[1], a[0]] : y;
    if (a[0] != 2) { return 9; }
    if (a[1] != 1) { return 8; }
    return 0;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_rawstore_condrhs_main(void)\n{");
    assert_contains(tag, body, "i32 __cl_rhs_");
    assert_contains(tag, body, "[0] = a[1];");
    assert_contains(tag, body, "[1] = a[0];");
    assert_contains(tag, body, "memcpy(&(a), &(__cl_rhs_");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A raw-deref destination whose literal reads through the same pointer fills
/// the temporary first, so every element reads the old bytes.
#[test]
fn should_fill_a_raw_destination_self_read_into_the_temporary() {
    let tag = "rawrhs";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.rawrhs;

struct Box { data: i32[2]; }

func main(): i32 {
    let b: Box = Box { data: [1, 2] };
    unsafe {
        let p: raw Box = &b;
        (*p).data = [(*p).data[1], (*p).data[0]];
    }
    if (b.data[0] != 2) { return 9; }
    if (b.data[1] != 1) { return 8; }
    return 0;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_rawstore_rawrhs_main(void)\n{");
    assert_contains(tag, body, "i32 __cl_rhs_");
    assert_contains(tag, body, "[0] = (*(p)).data[1];");
    assert_contains(tag, body, "[1] = (*(p)).data[0];");
    assert_contains(tag, body, "memcpy(&((*(&((*(p)).data)))), &(__cl_rhs_");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A call nested under a binary index still pins the intermediate raw field
/// before the call.
#[test]
fn should_pin_the_intermediate_field_before_a_binary_indexed_call() {
    let tag = "binarypin";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.binarypin;

struct Inner { x: i32; }
struct Outer { next: Inner; }

let gp: raw Outer = null;
let arr: i32[2] = [5, 3];
let other: Outer = Outer { next: Inner { x: 0 } };

func change(): i32 {
    unsafe { gp = &other; }
    return 1;
}

func main(): i32 {
    let a: Outer = Outer { next: Inner { x: 0 } };
    unsafe { gp = &a; (*gp).next.x = arr[change() + 0]; }
    if (a.next.x != 3) { return 9; }
    if (other.next.x != 0) { return 8; }
    return 0;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_rawstore_binarypin_main(void)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp0 = &((*(cl_spec_rawstore_binarypin_gp)).next);",
    );
    let pin = body
        .find("__cl_tmp0 = &((*(cl_spec_rawstore_binarypin_gp)).next);")
        .expect("pin");
    let call = body
        .find("cl_spec_rawstore_binarypin_change()")
        .expect("indexed call");
    assert!(pin < call, "{tag}: the pin must precede the indexed call; body:\n{body}");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}

/// A raw-deref destination whose literal reads through an aliasing pointer
/// still fills the temporary first.
#[test]
fn should_fill_an_aliasing_raw_destination_self_read_into_the_temporary() {
    let tag = "rawaliasrhs";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.rawstore.rawaliasrhs;

struct Box { data: i32[2]; }

func main(): i32 {
    let b: Box = Box { data: [1, 2] };
    unsafe {
        let p: raw Box = &b;
        let q: raw Box = p;
        (*p).data = [(*q).data[1], (*q).data[0]];
    }
    if (b.data[0] != 2) { return 9; }
    if (b.data[1] != 1) { return 8; }
    return 0;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_rawstore_rawaliasrhs_main(void)\n{");
    assert_contains(tag, body, "i32 __cl_rhs_");
    assert_contains(tag, body, "[0] = (*(q)).data[1];");
    assert_contains(tag, body, "[1] = (*(q)).data[0];");
    assert_contains(tag, body, "memcpy(&((*(&((*(p)).data)))), &(__cl_rhs_");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the program must run");
}
