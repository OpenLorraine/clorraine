//! Specification tests for the zero-defect calls and resolution batch.
//!
//! Each case either checks a rejected call shape's frozen diagnostic, or emits
//! and strictly builds an accepted program. The accepted cases run where the
//! behavior is observable.

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
            "clc-zerocalls-{}-{tag}-{sequence}",
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

/// Compiles one program with optional extra flags.
fn compile_with(
    tag: &str,
    source: &str,
    extra: &[&str],
) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let mut command = Command::new(env!("CARGO_BIN_EXE_clc"));
    command.args(["--profile", "host"]);
    command.args(extra);
    command.args([
        "-o",
        c_path.to_str().expect("C path"),
        source_path.to_str().expect("source path"),
    ]);
    let output = command
        .output()
        .expect("failed to spawn the compiler binary");
    (scratch, output, c_path)
}

/// Compiles one host program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_with(tag, source, &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Compiles a set of files together and returns the emitted translation unit.
fn emit_files(tag: &str, sources: &[(&str, &str)]) -> String {
    let scratch = ScratchDir::new(tag);
    let mut paths = Vec::new();
    for (name, source) in sources {
        let path = scratch.path().join(name);
        std::fs::write(&path, source)
            .unwrap_or_else(|err| panic!("{tag}: cannot write {name}: {err}"));
        paths.push(path);
    }
    let c_path = scratch.path().join("out.c");
    let mut command = Command::new(env!("CARGO_BIN_EXE_clc"));
    command.args(["--profile", "host", "-o", c_path.to_str().expect("C path")]);
    for path in &paths {
        command.arg(path);
    }
    let output = command.output().expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Builds one emitted translation unit under the strict host warning set.
fn build_strict(tag: &str, emitted: &str) -> (ScratchDir, PathBuf) {
    let scratch = ScratchDir::new(&format!("{tag}-build"));
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
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
    (scratch, program)
}

/// Emits, strictly builds, and runs one accepted host program.
fn run_strict_host(tag: &str, source: &str) -> Output {
    let emitted = emit_host(tag, source);
    let (_scratch, program) = build_strict(tag, &emitted);
    Command::new(&program)
        .output()
        .expect("failed to spawn the built program")
}

/// Emits, strictly builds, and runs a set of files.
fn run_strict_files(tag: &str, sources: &[(&str, &str)]) -> Output {
    let emitted = emit_files(tag, sources);
    let (_scratch, program) = build_strict(tag, &emitted);
    Command::new(&program)
        .output()
        .expect("failed to spawn the built program")
}

/// Asserts one diagnostic count and optional message fragments.
fn assert_rejected(tag: &str, source: &str, code: &str, expected: usize, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_with(tag, source, &[]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected {code}, but the front end accepted the program"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let found = stderr.matches(&format!("error {code}:")).count();
    assert_eq!(
        found, expected,
        "{tag}: expected {expected} {code} report(s), found {found}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the report to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(!c_path.exists(), "{tag}: a rejected program must not write C");
}

fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: unexpected exit code; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// ST-32: an escaped user component cannot collide with a function-type key.
#[test]
fn should_escape_user_type_components_to_avoid_func_key_collision() {
    let source = r#"module spec.zc.keyescape;

struct Holder<T> { value: T; }
struct Holder__fn_borrow_i32__i32 { x: i32; }
func inc(borrow x: i32): i32 { return x + 1; }

func main(): i32 {
    let h: Holder<func(borrow i32): i32> =
        Holder<func(borrow i32): i32> { value: inc };
    let s: Holder__fn_borrow_i32__i32 = Holder__fn_borrow_i32__i32 { x: 1 };
    return h.value(1) + s.x - 3;
}
"#;
    let emitted = emit_host("keyescape", source);
    assert!(
        emitted.contains("struct Clu26_Holder__fn_borrow_i32__i32"),
        "keyescape: the colliding user component must be escaped; emitted C:\n{emitted}"
    );
    let output = run_strict_host("keyescape", source);
    assert_exit("keyescape", &output, 0);
}

/// ST-33: a func-typed global uses its own signature, not a foreign function's
/// modes.
#[test]
fn should_use_the_func_global_signature_over_a_foreign_same_name_function() {
    let foreign = r#"module spec.zc.foreignmod;
import { String } from std.text;
export func g(take s: String): i32 { return i32(len(s)); }
export func dummy(): i32 { return 0; }
"#;
    let holder = r#"module spec.zc.globalmod;
import { String, stringOf } from std.text;
func lenOf(s: String): i32 { return i32(len(s)); }
let g: func(String): i32 = lenOf;
export func run(): i32 {
    let temp: String = stringOf("ab");
    let n: i32 = g(temp);
    return n + i32(len(temp));
}
"#;
    let main = r#"module spec.zc.globalmain;
import { dummy } from spec.zc.foreignmod;
import { run } from spec.zc.globalmod;
func main(): i32 { return dummy() + run() - 4; }
"#;
    let output = run_strict_files(
        "globalmodes",
        &[("foreign.clo", foreign), ("holder.clo", holder), ("main.clo", main)],
    );
    assert_exit("globalmodes", &output, 0);
}

/// ST-34: a closure-call argument with a contradicting marker reports E10039.
#[test]
fn should_reject_a_closure_call_marker_that_contradicts_the_lambda() {
    let source = r#"module spec.zc.closuremarker;

import { String, stringOf } from std.text;

func main(): i32 {
    let f = (p: String) => 0;
    return f(take stringOf("x"));
}
"#;
    assert_rejected(
        "closuremarker",
        source,
        "E10039",
        1,
        &["call-site marker does not match"],
    );
}

/// ST-35: an initializer call marker faces the same declaration check.
#[test]
fn should_reject_an_initializer_call_marker() {
    let source = r#"module spec.zc.initmarker;

import { String, stringOf } from std.text;

func f(s: String): i32 { return i32(len(s)); }
let g: i32 = f(take stringOf("ab"));

func main(): i32 { return g - 2; }
"#;
    assert_rejected(
        "initmarker",
        source,
        "E10039",
        1,
        &["call-site marker does not match"],
    );
}

/// ST-36: an initializer dot call borrowing a global reports E10038.
#[test]
fn should_reject_an_initializer_receiver_that_borrows_a_global() {
    let source = r#"module spec.zc.initglobal;

struct N { v: i32; }
func peek(borrow n: N): i32 { return n.v; }
let n: N = N { v: 1 };
let g: i32 = n.peek();

func main(): i32 { return g - 1; }
"#;
    assert_rejected(
        "initglobal",
        source,
        "E10038",
        1,
        &["cannot borrow global 'n'"],
    );
}

/// ST-37: a body func-typed global named like a view builder calls its value.
#[test]
fn should_not_intercept_a_func_global_named_slice() {
    let source = r#"module spec.zc.globalslice;

func passthrough(s: str): str { return s; }
let slice: func(str): str = passthrough;

func main(): i32 {
    let s: str = "ab";
    let t: str = slice(s);
    return i32(len(t)) - 2;
}
"#;
    let emitted = emit_host("globalslice", source);
    assert!(
        !emitted.contains("ClSlice_unknown"),
        "globalslice: the callable global must not lower as the view builder"
    );
    let output = run_strict_host("globalslice", source);
    assert_exit("globalslice", &output, 0);
}

/// ST-38: a local used only as a store index is used, not unused.
#[test]
fn should_not_warn_for_a_local_used_only_as_a_store_index() {
    let source = r#"module spec.zc.storeindex;

func main(): i32 {
    let i: i32 = 0;
    let xs: i32[2] = [1, 2];
    xs[i + 1] = 9;
    return xs[0] - 1;
}
"#;
    let (_scratch, output, _c_path) = compile_with("storeindex", source, &[]);
    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("W10000"),
        "storeindex: the store index is a read; stderr:\n{stderr}"
    );
    let output = run_strict_host("storeindex", source);
    assert_exit("storeindex", &output, 0);
}

/// ST-39: a call of a non-callable field or value reports E10309.
#[test]
fn should_reject_a_call_of_a_non_callable_field_or_value() {
    let field = r#"module spec.zc.noncallablefield;

struct S { x: i32; }
func main(): i32 {
    let s: S = S { x: 1 };
    s.x(3);
    return 0;
}
"#;
    assert_rejected("noncallablefield", field, "E10309", 1, &["not callable"]);

    let value = r#"module spec.zc.noncallablevalue;

func main(): i32 {
    let n: i32 = 1;
    n();
    return 0;
}
"#;
    assert_rejected("noncallablevalue", value, "E10309", 1, &["not callable"]);
}

/// ST-40: a func-value call with the wrong argument count reports the arity.
#[test]
fn should_report_the_arity_of_a_func_value_call() {
    let source = r#"module spec.zc.funcarity;

func g(x: i32): i32 { return x; }
func main(): i32 {
    let f: func(i32): i32 = g;
    return f(1, 2);
}
"#;
    assert_rejected(
        "funcarity",
        source,
        "E10308",
        1,
        &["wrong number of arguments for 'f'", "expected 1, found 2"],
    );
}

/// ST-41: the Array dot arm requires a receiver-taking candidate.
#[test]
fn should_reject_a_zero_parameter_array_dot_call() {
    let source = r#"module spec.zc.arraydot;

import { Array, arrayNew } from std.mem;

func main(): i32 {
    let xs: Array<i32> = arrayNew<i32>();
    let ys: Array<i32> = xs.arrayNew<i32>();
    return 0;
}
"#;
    assert_rejected(
        "arraydot",
        source,
        "E10308",
        1,
        &["wrong number of arguments for 'arrayNew'"],
    );
}

/// ST-42: a module-qualified call keeps the arity rule.
#[test]
fn should_report_the_arity_of_a_module_qualified_call() {
    let source = r#"module spec.zc.qualifiedarity;

import { stringOf } from std.text;

func main(): i32 {
    let s: str = "ab";
    let o: String = stringOf(s, 1);
    return 0;
}
"#;
    assert_rejected(
        "qualifiedarity",
        source,
        "E10308",
        1,
        &["wrong number of arguments for 'stringOf'"],
    );
}

/// ST-43: a callable field through a raw receiver uses the implied arrow.
#[test]
fn should_render_a_raw_field_call_with_an_arrow() {
    let source = r#"module spec.zc.rawfieldcall;

struct S { foo: func(i32): i32; }
func inc(x: i32): i32 { return x + 1; }

func main(): i32 {
    let s: S = S { foo: inc };
    let p: raw S = unsafe &s;
    unsafe { return p.foo(1) - 2; }
}
"#;
    let emitted = emit_host("rawfieldcall", source);
    assert!(
        emitted.contains("p->foo"),
        "rawfieldcall: the raw receiver must use the arrow spelling; emitted C:\n{emitted}"
    );
    let output = run_strict_host("rawfieldcall", source);
    assert_exit("rawfieldcall", &output, 0);
}

/// ST-44: an unused ffi declaration validates its parameter types.
#[test]
fn should_validate_an_unused_ffi_parameter() {
    let source = r#"module spec.zc.ffiunused;

ffi probe {
    func F(x: i32[0]): i32;
}

func main(): i32 { return 0; }
"#;
    assert_rejected(
        "ffiunused",
        source,
        "E10226",
        1,
        &["array extent must be greater than zero"],
    );
}

/// ST-45: a zero error cap is rejected and writes no translation unit.
#[test]
fn should_reject_a_zero_error_cap() {
    let source = r#"module spec.zc.maxzero;

func main(): i32 {
    let n: i32 = "bad";
    return n;
}
"#;
    let (_scratch, output, c_path) = compile_with("maxzero", source, &["--max-errors", "0"]);
    assert_ne!(output.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("positive number"),
        "maxzero: stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!c_path.exists(), "maxzero: the rejected program must not write C");
}

/// ST-46: the sanctioned raw-to-integer cast is strict-build clean.
#[test]
fn should_emit_a_host_clean_raw_to_integer_cast() {
    let source = r#"module spec.zc.rawcast;

func main(): i32 {
    let s: str = "ab";
    let n: usize = usize(s.ptr);
    return i32(n) - i32(n);
}
"#;
    let emitted = emit_host("rawcast", source);
    assert!(
        emitted.contains("uintptr_t"),
        "rawcast: the cast must route through uintptr_t; emitted C:\n{emitted}"
    );
    let output = run_strict_host("rawcast", source);
    assert_exit("rawcast", &output, 0);
}

/// ST-47: an explicit generic literal takes the type argument's context.
#[test]
fn should_type_an_explicit_generic_literal_by_its_type_argument() {
    let source = r#"module spec.zc.genericliteral;

func identity<T>(x: T): T { return x; }

func main(): i32 {
    let a: u8 = identity<u8>(5);
    let b: i8 = identity<i8>(-3);
    return i32(a) + i32(b) - 2;
}
"#;
    let output = run_strict_host("genericliteral", source);
    assert_exit("genericliteral", &output, 0);
}

/// ST-48: a bare member read does not borrow a free function's spelling.
#[test]
fn should_reject_a_bare_member_read_that_names_a_free_function() {
    let source = r#"module spec.zc.membershadow;

struct S { x: i32; }
func foo(x: str): i32 { return 0; }

func main(): i32 {
    let s: S = S { x: 1 };
    let y: i32 = s.foo;
    return y;
}
"#;
    assert_rejected("membershadow", source, "E10212", 1, &["'foo' is not a member"]);
}

/// ST-49: a member read on a literal null base reports E10212.
#[test]
fn should_reject_a_member_read_on_a_null_base() {
    let source = r#"module spec.zc.nullmember;

func main(): i32 {
    let y: i32 = null.foo;
    return y;
}
"#;
    assert_rejected("nullmember", source, "E10212", 1, &["'foo' is not a member"]);
}

/// ST-50: a module-path method call is resolved and reports the ownership rule.
#[test]
fn should_resolve_a_module_path_method_receiver() {
    let holder = r#"module spec.zc.methodmod;

struct Origin { x: i32; }
export let origin: Origin = Origin { x: 1 };
export func getx(borrow o: Origin): i32 { return o.x; }
"#;
    let main = r#"module spec.zc.methodmain;

func main(): i32 {
    return spec.zc.methodmod.origin.getx() - 1;
}
"#;
    let scratch = ScratchDir::new("methodpath");
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder).expect("write holder");
    std::fs::write(&main_path, main).expect("write main");
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(["--profile", "host", "-o"])
        .arg(&c_path)
        .arg(&main_path)
        .arg(&holder_path)
        .output()
        .expect("failed to spawn the compiler binary");
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E10038") && stderr.contains("cannot borrow global 'origin'"),
        "methodpath: expected the resolved receiver's borrow rule; stderr:\n{stderr}"
    );
    assert!(!stderr.contains("E10212"), "methodpath: the member error must be gone");
}

/// ST-51: the dot spelling accepts a static string literal for cstring.
#[test]
fn should_accept_a_dot_call_cstring_literal() {
    let source = r#"module spec.zc.dotcstring;

func put(x: i32, s: cstring): i32 { return x; }

func main(): i32 {
    return 1.put("x") - 1;
}
"#;
    let output = run_strict_host("dotcstring", source);
    assert_exit("dotcstring", &output, 0);
}

/// ST-52: the inferred generic route types a fitting literal by the inferred
/// parameter, and an unfit literal reports the fit rule.
#[test]
fn should_type_an_inferred_generic_literal_by_the_inferred_parameter() {
    let accepted = r#"module spec.zc.inferredliteral;

func pair<T>(x: T, n: T): T { return x; }

func main(): i32 {
    let a: u8 = 3;
    let b: u8 = pair(a, 6);
    return i32(b) - 3;
}
"#;
    let output = run_strict_host("inferredliteral", accepted);
    assert_exit("inferredliteral", &output, 0);

    let unfit = r#"module spec.zc.inferredfit;

func identity<T>(x: T): T { return x; }

func main(): i32 {
    let x: i8 = identity<i8>(300);
    return i32(x);
}
"#;
    assert_rejected("inferredfit", unfit, "E10202", 1, &["literal does not fit 'i8'"]);
}

/// ST-53: an import-renamed type resolves to its declared instance.
#[test]
fn should_resolve_an_import_renamed_type() {
    let source = r#"module spec.zc.aliastype;

import { Array as A, arrayNew } from std.mem;

struct S { a: A<i32>; }

func main(): i32 {
    let s: S = S { a: arrayNew<i32>() };
    return i32(s.a.count);
}
"#;
    let emitted = emit_host("aliastype", source);
    assert!(
        emitted.contains("ClArray__i32"),
        "aliastype: the alias must resolve to the Array instance; emitted C:\n{emitted}"
    );
    let output = run_strict_host("aliastype", source);
    assert_exit("aliastype", &output, 0);
}

/// ST-53b: an import-renamed type resolves in globals, parameters, returns, and
/// arrays, not only in a struct field.
#[test]
fn should_resolve_an_import_renamed_type_in_every_type_position() {
    let holder = r#"module spec.zc.aliasallholder;

export struct W { v: i32; }
export func make(v: i32): W { return W { v: v }; }
"#;
    let main = r#"module spec.zc.aliasallmain;

import { W as V, make } from spec.zc.aliasallholder;

let g: V = make(5);

func use(v: V): i32 { return v.v; }
func give(): V { return make(4); }

func main(): i32 {
    let w: V = make(3);
    let a: V[2] = [make(1), make(2)];
    return use(w) + use(give()) + g.v + a[0].v - 13;
}
"#;
    let output = run_strict_files(
        "aliasall",
        &[("holder.clo", holder), ("main.clo", main)],
    );
    assert_exit("aliasall", &output, 0);
}

/// ST-46b: the raw-to-integer cast spelling builds on the freestanding profile.
#[test]
fn should_build_the_raw_to_integer_cast_on_freestanding() {
    let source = r#"module spec.zc.rawcastfree;

func main(): i32 {
    let s: str = "ab";
    let n: usize = usize(s.ptr);
    return i32(n) - i32(n);
}
"#;
    let scratch = ScratchDir::new("rawcastfree");
    let source_path = scratch.path().join("main.clo");
    let c_path = scratch.path().join("out.c");
    std::fs::write(&source_path, source).expect("write source");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(["--profile", "freestanding", "-o"])
        .arg(&c_path)
        .arg(&source_path)
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "rawcastfree: the freestanding emit rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let object = scratch.path().join("out.o");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-ffreestanding",
            "-fno-builtin",
            "-nostdlib",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
            "-o",
        ])
        .arg(&object)
        .arg(&c_path)
        .output()
        .expect("failed to spawn the freestanding host compiler");
    assert!(
        built.status.success(),
        "rawcastfree: the freestanding C rejected the cast; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}
