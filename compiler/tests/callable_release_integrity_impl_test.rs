//! Implementation tests for the generated identities of the callable release
//! integrity batch.
//!
//! The specification suite proves what a program computes; these cases prove
//! how the compiler spells and emits the identities: a generated owner keeps a
//! spelling apart from a colliding source local, two `func` signatures key
//! apart (a `borrow` parameter included, because it changes the C
//! representation), and a loop's capturing lambda is emitted exactly once.

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
            "clc-criimpl-{}-{tag}-{sequence}",
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

/// Emits one host-profile program from a holder module and its main.
///
/// The import binding and the qualified spelling need the declaring module on
/// the command line, exactly as a build does, so this helper passes both
/// files. Returns the emitted translation unit.
fn emit_pair(tag: &str, holder: &str, main: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Runs a two-file host program and asserts the front end rejected it.
///
/// The declaring module and the program are written beside each other, as a
/// build passes them. Returns the diagnostics' text; a rejected program must
/// not write a translation unit.
fn reject_pair(tag: &str, holder: &str, main: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program wrote a translation unit"
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Emits a two-file program twice from the same source paths.
///
/// The module and the program keep one path across both runs, so a translation
/// unit that differs only in its source locations still compares equal. Returns
/// both emitted texts.
fn emit_twice_pair(tag: &str, holder: &str, main: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
        let output = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            main_path.display().to_string(),
            holder_path.display().to_string(),
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

/// Emits a three-file module program and asserts the front end accepted it.
///
/// Two holder files declare the modules the program names, so two same-named
/// bindings in different modules both resolve and the emitter must keep them
/// apart. Returns the emitted translation unit.
fn emit_three(tag: &str, first: &str, second: &str, main: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let first_path = scratch.path().join("first.clo");
    let second_path = scratch.path().join("second.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&first_path, first)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the first holder: {err}"));
    std::fs::write(&second_path, second)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the second holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        first_path.display().to_string(),
        second_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
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
        "{tag}: the emitted C is empty"
    );
    emitted
}

/// Asserts the emitted C contains `needle`, reporting the full text otherwise.
fn assert_contains(tag: &str, emitted: &str, needle: &str) {
    assert!(
        emitted.contains(needle),
        "{tag}: expected the emitted C to contain {needle:?}; emitted C:\n{emitted}"
    );
}

/// Counts non-overlapping occurrences of `needle`.
fn count(emitted: &str, needle: &str) -> usize {
    emitted.matches(needle).count()
}

/// Asserts the first `s = NULL;` after `call_at` follows the call directly.
///
/// The emitted C may carry `#line` markers between the call and the next
/// statement, so the lines after the call's own line may consist of those
/// markers and blank lines; any other text means a statement intervenes before
/// the move-out.
fn assert_move_out_follows(tag: &str, emitted: &str, call_at: usize) {
    let after = &emitted[call_at..];
    let line_end = after.find('\n').map_or(after.len(), |index| index + 1);
    let rest = &after[line_end..];
    let null_at = rest
        .find("s = NULL;")
        .unwrap_or_else(|| panic!("{tag}: no move-out follows the call; emitted C:\n{emitted}"));
    assert!(
        rest[..null_at]
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("#line")),
        "{tag}: a statement intervenes before the move-out; emitted C:\n{emitted}"
    );
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

/// A colliding source local and the generated owner get distinct spellings.
///
/// The user's local keeps its written spelling; the generated owner declared
/// later is masked to a distinct C name, and each is released once.
#[test]
fn should_spell_a_generated_owner_apart_from_a_colliding_source_local() {
    let source = r#"module spec.cri.impl.idspell;

import { String, stringOf } from std.text;

func use(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let __cl_owned0: String = stringOf("q");
    let n: i32 = use(stringOf("w"));
    return n + i32(len(__cl_owned0)) - 2;
}
"#;
    let emitted = emit_host("idspell", source);
    assert_contains("idspell", &emitted, "ClString __cl_owned0 = NULL;");
    assert_contains("idspell", &emitted, "ClString __cl_owned0_1 = NULL;");
    // The pre-fix emission bound a spurious third owner (a copy of the user's
    // local) whose release double-freed at cleanup; the fix removes it.
    assert!(
        !emitted.contains("__cl_owned1"),
        "no spurious copy owner may be emitted"
    );
    // Each owner releases on its own paths — the release-before-store and the
    // scope cleanup — and never under the other's spelling.
    assert_eq!(
        count(&emitted, "FreeString(__cl_owned0)"),
        2,
        "the source local releases on its own paths"
    );
    assert_eq!(
        count(&emitted, "FreeString(__cl_owned0_1)"),
        2,
        "the generated owner releases on its own paths"
    );
}

/// Two `func` signatures key apart, so the unit defines two structs.
#[test]
fn should_key_two_func_signatures_apart() {
    let source = r#"module spec.cri.impl.fkkey;

struct Holder<T> {
    call: T;
}

func addOne(v: i32): i32 {
    return v + 1;
}

func addTwo(v: u32): u32 {
    return v + 2;
}

func main(): i32 {
    let a: Holder<func(i32): i32> = Holder<func(i32): i32> { call: addOne };
    let b: Holder<func(u32): u32> = Holder<func(u32): u32> { call: addTwo };
    return a.call(2) + i32(b.call(2)) - 7;
}
"#;
    let emitted = emit_host("fkkey", source);
    assert_contains("fkkey", &emitted, "ClHolder__fn_i32__i32");
    assert_contains("fkkey", &emitted, "ClHolder__fn_u32__u32");
    assert_eq!(
        count(&emitted, "struct ClHolder__fn_i32__i32 {"),
        1,
        "one struct per instance"
    );
    assert_eq!(
        count(&emitted, "struct ClHolder__fn_u32__u32 {"),
        1,
        "one struct per instance"
    );
}

/// A `borrow` parameter mode is part of the key: it changes the C type.
#[test]
fn should_key_a_borrow_parameter_mode_apart() {
    let source = r#"module spec.cri.impl.mkkey;

import { String, stringOf } from std.text;

struct Holder<T> {
    call: T;
}

func lenOf(s: String): i32 {
    return i32(len(s));
}

func lendOf(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let a: Holder<func(String): i32> = Holder<func(String): i32> { call: lenOf };
    let b: Holder<func(borrow String): i32> = Holder<func(borrow String): i32> { call: lendOf };
    return a.call(stringOf("ab")) + b.call(stringOf("cde")) - 5;
}
"#;
    let emitted = emit_host("mkkey", source);
    assert_contains("mkkey", &emitted, "ClHolder__fn_String__i32");
    assert_contains("mkkey", &emitted, "ClHolder__fn_borrow_String__i32");
}

/// A loop's capturing lambda emits one environment, one typedef, one function.
#[test]
fn should_emit_one_lifted_environment_per_site() {
    let source = r#"module spec.cri.impl.envonce;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("ab");
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        total = total + ([s] () => i32(len(s)))();
        i = i + 1;
    }
    return total - 4;
}
"#;
    let emitted = emit_host("envonce", source);
    assert_eq!(
        count(&emitted, "struct ClEnv_0 {"),
        1,
        "one environment definition"
    );
    assert_eq!(
        count(&emitted, "} ClClosure_0;"),
        1,
        "one closure typedef"
    );
    assert_eq!(
        count(&emitted, "cl_lambda_0("),
        3,
        "one prototype, one call, one definition"
    );
}

/// Two emissions of one program are byte-identical.
#[test]
fn should_re_emit_the_identity_programs_byte_identically() {
    let identity = r#"module spec.cri.impl.reemit;

import { String, stringOf } from std.text;

func use(borrow s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let __cl_owned0: String = stringOf("q");
    let n: i32 = use(stringOf("w"));
    return n + i32(len(__cl_owned0)) - 2;
}
"#;
    let (first, second) = emit_twice("reemit", identity);
    assert_eq!(first, second, "re-emission must be byte-identical");

    let looped = r#"module spec.cri.impl.reemitloop;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String = stringOf("ab");
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 2) {
        total = total + ([s] () => i32(len(s)))();
        i = i + 1;
    }
    return total - 4;
}
"#;
    let (first, second) = emit_twice("reemit-loop", looped);
    assert_eq!(first, second, "re-emission must be byte-identical");
}

/// A `take` field call moves its argument out through the field's signature.
///
/// The call goes through the field's own value (`(h.call)(s)`), and the
/// argument's owner is emptied right after the call because the field's
/// signature marks the parameter `take` — the same move a module twin emits.
#[test]
fn should_move_out_a_take_field_calls_argument() {
    let source = r#"module spec.cri.impl.fieldtake;

import { String, stringOf } from std.text;

struct Holder {
    call: func(take String): i32;
}

func sink(take s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let h: Holder = Holder { call: sink };
    let s: String = stringOf("x");
    return h.call(s) - 1;
}
"#;
    let emitted = emit_host("fieldtake", source);
    let call = emitted
        .find("__cl_tmp0 = (h.call)(s);")
        .expect("fieldtake: the call must go through the field's own value");
    assert_move_out_follows("fieldtake", &emitted, call);
}

/// A plain field call binds its owned temporary argument for release.
///
/// The field's signature is an unmarked (read-only lend) parameter, so a
/// fresh `String` argument binds to a scope owner the cleanup releases; the
/// field call receives the owner's spelling.
#[test]
fn should_release_an_owned_temporary_at_a_plain_field_call() {
    let source = r#"module spec.cri.impl.fieldborrow;

import { String, stringOf } from std.text;

struct Holder { call: func(String): i32; }

func lenOf(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let h: Holder = Holder { call: lenOf };
    return h.call(stringOf("x")) - 1;
}
"#;
    let emitted = emit_host("fieldborrow", source);
    assert_contains("fieldborrow", &emitted, "ClString __cl_owned0 = NULL;");
    assert_contains("fieldborrow", &emitted, "(h.call)(__cl_owned0)");
}

/// A `take` global call moves its argument out through the global's signature.
#[test]
fn should_move_out_a_take_global_calls_argument() {
    let source = r#"module spec.cri.impl.globaltake;

import { String, stringOf } from std.text;

func sink(take s: String): i32 {
    return i32(len(s));
}

let g: func(take String): i32 = sink;

func main(): i32 {
    let s: String = stringOf("x");
    return g(s) - 1;
}
"#;
    let emitted = emit_host("globaltake", source);
    let call = emitted
        .find("__cl_tmp0 = cl_spec_cri_impl_globaltake_g(s);")
        .expect("globaltake: the call must go through the global's own value");
    assert_move_out_follows("globaltake", &emitted, call);
}

/// A plain global call binds its owned temporary argument for release.
#[test]
fn should_release_an_owned_temporary_at_a_plain_global_call() {
    let source = r#"module spec.cri.impl.globalborrow;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

let g: func(String): i32 = lenOf;

func main(): i32 {
    return g(stringOf("x")) - 1;
}
"#;
    let emitted = emit_host("globalborrow", source);
    assert_contains("globalborrow", &emitted, "ClString __cl_owned0 = NULL;");
    assert_contains(
        "globalborrow",
        &emitted,
        "cl_spec_cri_impl_globalborrow_g(__cl_owned0)",
    );
}

/// A callable-bound parameter call uses the bound's parameters and modes.
///
/// The monomorphized instance carries the bound's signature as its instance
/// key, and the owned temporary argument binds to a scope owner the cleanup
/// releases before the value crosses as a lend.
#[test]
fn should_release_an_owned_temporary_at_a_callable_bound_parameter() {
    let source = r#"module spec.cri.impl.paramleak;

import { String, stringOf } from std.text;

func lenOf(s: String): i32 {
    return i32(len(s));
}

func wrap<F: func(String): i32>(f: F): i32 {
    return f(stringOf("x"));
}

func main(): i32 {
    return wrap<func(String): i32>(lenOf) - 1;
}
"#;
    let emitted = emit_host("paramleak", source);
    assert_contains(
        "paramleak",
        &emitted,
        "static i32 cl_wrap__fn_String__i32(i32 (*f)(ClString))",
    );
    assert_contains("paramleak", &emitted, "__cl_ret = f(__cl_owned0);");
}

/// A closure call binds its owned temporary argument for caller-side release.
///
/// The lambda's parameters are unmarked lends like a module function's, so a
/// fresh argument binds to a scope owner the cleanup releases; the lifted
/// function receives the owner's spelling.
#[test]
fn should_release_an_owned_temporary_at_a_closure_call() {
    let source = r#"module spec.cri.impl.lambdaarg;

import { String, stringOf } from std.text;

func main(): i32 {
    return ((p: String) => i32(len(p)))(stringOf("x")) - 1;
}
"#;
    let emitted = emit_host("lambdaarg", source);
    assert_contains("lambdaarg", &emitted, "ClString __cl_owned0 = NULL;");
    assert_contains("lambdaarg", &emitted, "cl_lambda_0(__cl_owned0)");
}

/// A temporary func-field receiver binds its root for sibling release.
///
/// The receiver aggregate is only read for its `call` field, so a fresh
/// literal or call result binds to an owned local; the call reads the field
/// through that local and the local's cleanup releases the other owning
/// fields. Each call site gets its own owner.
#[test]
fn should_bind_a_temporary_func_field_receivers_root() {
    let source = r#"module spec.cri.impl.temprec;

import { String, stringOf } from std.text;

func addOne(x: i32): i32 {
    return x + 1;
}

struct Holder { call: func(i32): i32; s: String; }

func makeH(): Holder {
    return Holder { call: addOne, s: stringOf("x") };
}

func main(): i32 {
    let a: i32 = Holder { call: addOne, s: stringOf("x") }.call(5);
    let b: i32 = makeH().call(5);
    return a + b - 12;
}
"#;
    let emitted = emit_host("temprec", source);
    assert_contains("temprec", &emitted, "ClHolder __cl_owned0 = {0};");
    assert_contains("temprec", &emitted, "ClHolder __cl_owned1 = {0};");
    assert_contains(
        "temprec",
        &emitted,
        "__cl_owned0 = (ClHolder){ cl_spec_cri_impl_temprec_addOne",
    );
    assert_contains(
        "temprec",
        &emitted,
        "__cl_owned1 = cl_spec_cri_impl_temprec_makeH();",
    );
    assert_contains("temprec", &emitted, "(__cl_owned0.call)(5)");
    assert_contains("temprec", &emitted, "(__cl_owned1.call)(5)");
    assert_contains(
        "temprec",
        &emitted,
        "if (__cl_owned1.s) { FreeString(__cl_owned1.s); }",
    );
}

/// A defer body's view owner stays in the block's own scope.
///
/// The body's view binding takes its backing from a call temporary inside the
/// deferred copy, so the owner binds and releases there — once per execution
/// of the body rather than once for the program.
#[test]
fn should_keep_a_defer_bodys_view_owner_in_the_block() {
    let source = r#"module spec.cri.impl.deferview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box {
    return Box { v: stringOf("ab") };
}

let total: i32 = 0;

func main(): i32 {
    defer {
        let v: str = mk().v;
        total = total + i32(len(v));
    }
    return 0;
}
"#;
    let emitted = emit_host("deferview", source);
    assert_contains("deferview", &emitted, "ClBox __cl_owned0 = {0};");
    assert_contains(
        "deferview",
        &emitted,
        "__cl_owned0 = cl_spec_cri_impl_deferview_mk();",
    );
    assert_contains("deferview", &emitted, "v = cl_string_view(__cl_owned0.v);");
    assert_contains(
        "deferview",
        &emitted,
        "if (__cl_owned0.v) { FreeString(__cl_owned0.v); }",
    );
}

/// A defer body's owned call temporary binds for caller-side release.
///
/// The deferred call passes a fresh owned temporary to a lend parameter, so
/// the argument binds to an owner the deferred copy releases once per
/// execution.
#[test]
fn should_bind_a_defer_bodys_owned_call_temporary() {
    let source = r#"module spec.cri.impl.defercall;

import { String, stringOf } from std.text;

let total: i32 = 0;

func use(s: String): i32 {
    return i32(len(s));
}

func main(): i32 {
    let f: func(String): i32 = use;
    defer {
        total = total + f(stringOf("x"));
    }
    return 0;
}
"#;
    let emitted = emit_host("defercall", source);
    assert_contains("defercall", &emitted, "ClString __cl_owned0 = NULL;");
    assert_contains(
        "defercall",
        &emitted,
        "__cl_owned0 = cl_std_text_stringOf(cl_str(\"x\"));",
    );
    assert_contains(
        "defercall",
        &emitted,
        "cl_spec_cri_impl_defercall_total = (i32)((u32)cl_spec_cri_impl_defercall_total + (u32)f(__cl_owned0));",
    );
    assert_contains(
        "defercall",
        &emitted,
        "if (__cl_owned0) { FreeString(__cl_owned0); }",
    );
}

/// A mixed conditional view's owning arm roots in its own branch.
///
/// The place arm stays the caller's borrow; the taken call arm binds to a
/// scope owner in the branch that evaluates it, and the view reads the select
/// between the place and the owner.
#[test]
fn should_own_a_conditional_views_owning_arm_in_its_branch() {
    let source = r#"module spec.cri.impl.condview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func makeBox(): Box {
    return Box { v: stringOf("ab") };
}

func main(): i32 {
    let b: Box = Box { v: stringOf("cd") };
    let c: bool = false;
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let v: str = (c ? b : makeBox()).v;
        n = n + i32(len(v));
        i = i + 1;
    }
    return n - 6;
}
"#;
    let emitted = emit_host("condview", source);
    assert_contains("condview", &emitted, "ClBox __cl_owned0 = {0};");
    assert_contains(
        "condview",
        &emitted,
        "__cl_owned0 = cl_spec_cri_impl_condview_makeBox();",
    );
    assert_contains(
        "condview",
        &emitted,
        "cl_string_view((__cl_tmp0 ? b : __cl_owned0).v)",
    );
    assert_contains(
        "condview",
        &emitted,
        "if (__cl_owned0.v) { FreeString(__cl_owned0.v); }",
    );
}

/// Both owning conditional view arms bind their own roots.
///
/// Each arm projects a fresh struct literal, so the taken arm's root gets its
/// own owner in its own branch and both roots' siblings release with the view
/// binding's scope.
#[test]
fn should_own_both_conditional_view_arms_roots() {
    let source = r#"module spec.cri.impl.condviewstruct;

import { String, stringOf } from std.text;

struct P { a: String; b: String; }
struct Q { a: String; b: String; }

func main(): i32 {
    let c: bool = true;
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let v: str = c ? (P { a: stringOf("ab"), b: stringOf("cd") }).a : (Q { a: stringOf("ef"), b: stringOf("gh") }).a;
        n = n + i32(len(v));
        i = i + 1;
    }
    return n - 6;
}
"#;
    let emitted = emit_host("condviewstruct", source);
    assert_contains("condviewstruct", &emitted, "ClP __cl_owned0 = {0};");
    assert_contains("condviewstruct", &emitted, "ClQ __cl_owned1 = {0};");
    assert_contains(
        "condviewstruct",
        &emitted,
        "__cl_owned0 = (ClP){ cl_std_text_stringOf(cl_str(\"ab\")), cl_std_text_stringOf(cl_str(\"cd\")) };",
    );
    assert_contains(
        "condviewstruct",
        &emitted,
        "__cl_owned1 = (ClQ){ cl_std_text_stringOf(cl_str(\"ef\")), cl_std_text_stringOf(cl_str(\"gh\")) };",
    );
    assert_contains(
        "condviewstruct",
        &emitted,
        "if (__cl_owned0.b) { FreeString(__cl_owned0.b); }",
    );
}

/// A discarded owned call result releases at the statement's end.
///
/// The bare expression statement stores its result in a statement temporary
/// and the statement's release frees it; a duplicable result stays a plain
/// call.
#[test]
fn should_release_a_discarded_owned_call_result_at_statement_end() {
    let source = r#"module spec.cri.impl.discard;

import { String, stringOf } from std.text;

func make(): String {
    return stringOf("ab");
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 3) {
        make();
        i = i + 1;
    }
    return 0;
}
"#;
    let emitted = emit_host("discard", source);
    assert_contains(
        "discard",
        &emitted,
        "__cl_tmp0 = cl_spec_cri_impl_discard_make();",
    );
    assert_contains(
        "discard",
        &emitted,
        "if (__cl_tmp0) { FreeString(__cl_tmp0); }",
    );
}

/// The branch-owned conditional shapes re-emit byte-identically.
#[test]
fn should_emit_the_conditional_view_owner_byte_identically() {
    let source = r#"module spec.cri.impl.condviewtwice;

import { String, stringOf } from std.text;

struct Box { v: String; }

func makeBox(): Box {
    return Box { v: stringOf("ab") };
}

func main(): i32 {
    let b: Box = Box { v: stringOf("cd") };
    let c: bool = false;
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let v: str = (c ? b : makeBox()).v;
        n = n + i32(len(v));
        i = i + 1;
    }
    return n - 6;
}
"#;
    let (first, second) = emit_twice("condviewtwice", source);
    assert_contains("condviewtwice", &first, "ClBox __cl_owned0 = {0};");
    assert_eq!(
        first, second,
        "condviewtwice: the same program emitted two different translations"
    );
}

/// The narrowed widening renders like any accepted operand.
///
/// The new check only reports; an accepted operand widens exactly as before,
/// so the string view of the narrowed handle is the ordinary accessor call.
#[test]
fn should_emit_the_narrowed_widening_unchanged() {
    let source = r#"module spec.cri.impl.narrowwiden;

import { String, stringOf } from std.text;

func main(): i32 {
    let s: String? = null;
    if (s != null) {
        return i32(len(s));
    }
    return 0;
}
"#;
    let emitted = emit_host("narrowwiden", source);
    assert_contains("narrowwiden", &emitted, "cl_string_view(s)");
}

// ===== The func-global callee =====

/// An imported or qualified func-global call renders the declaring symbol.
///
/// The import binding and the qualified spelling carry the global's identity,
/// so the call resolves the module-level binding and renders its mangled
/// symbol; the struct global's field call resolves through the same identity
/// (red at the phase base: the strict build failed with `implicit declaration
/// of function 'g'` and `'h' undeclared`).
#[test]
fn should_resolve_an_imported_or_qualified_func_global_call() {
    let holder = r#"module spec.criimpl.funcglobal;

func addOne(v: i32): i32 {
    return v + 1;
}

func addTwo(v: i32): i32 {
    return v + 2;
}

struct Holder {
    call: func(i32): i32;
}

export let g: func(i32): i32 = addOne;
export let h: Holder = Holder { call: addTwo };
"#;
    let main = r#"module spec.criimpl.funcglobalmain;

import { g, h } from spec.criimpl.funcglobal;

func main(): i32 {
    let a: i32 = g(5);
    let b: i32 = h.call(6);
    let c: i32 = spec.criimpl.funcglobal.g(5);
    return (a - 6) + (b - 8) + (c - 6);
}
"#;
    let emitted = emit_pair("funccall", holder, main);
    assert_contains("funccall", &emitted, "a = cl_spec_criimpl_funcglobal_g(5);");
    assert_contains(
        "funccall",
        &emitted,
        "b = (cl_spec_criimpl_funcglobal_h.call)(6);",
    );
    assert_contains("funccall", &emitted, "c = cl_spec_criimpl_funcglobal_g(5);");
}

/// A current-module func-global call keeps its resolved symbol.
///
/// The control reads the same shape without an import: the module-level
/// binding is initialized with the function's symbol and called through it.
#[test]
fn should_keep_a_current_module_func_global_call() {
    let source = r#"module spec.criimpl.funcglobalctl;

func triple(v: i32): i32 {
    return v * 3;
}

let local: func(i32): i32 = triple;

func main(): i32 {
    return local(2) - 6;
}
"#;
    let emitted = emit_host("funccallctl", source);
    assert_contains(
        "funccallctl",
        &emitted,
        "cl_spec_criimpl_funcglobalctl_local = cl_spec_criimpl_funcglobalctl_triple;",
    );
    assert_contains(
        "funccallctl",
        &emitted,
        "cl_spec_criimpl_funcglobalctl_local(2)",
    );
}

/// Two same-named func globals emit their own modules' symbols.
///
/// Each call carries the binding's identity, so the qualified spelling emits
/// the first module's symbol, and the import and the bare same-module call
/// emit the second module's symbol; a name-only resolution would emit the
/// first module's symbol three times. The counts pin each symbol to its own
/// call sites (red at the phase base: all three calls emitted the first
/// module's symbol).
#[test]
fn should_split_two_same_named_func_globals_by_module() {
    let first = r#"module spec.criimpl.globalsplitfirst;

func plusOne(v: i32): i32 {
    return v + 1;
}

export let pick: func(i32): i32 = plusOne;
"#;
    let second = r#"module spec.criimpl.globalsplitsecond;

func plusHundred(v: i32): i32 {
    return v + 100;
}

export let pick: func(i32): i32 = plusHundred;

export func ownCall(): i32 {
    return pick(5) - 105;
}
"#;
    let main = r#"module spec.criimpl.globalsplitmain;

import { pick, ownCall } from spec.criimpl.globalsplitsecond;

func main(): i32 {
    let imported: i32 = pick(5) - 105;
    let qualified: i32 = spec.criimpl.globalsplitfirst.pick(5) - 6;
    let own: i32 = ownCall();
    return imported + qualified + own;
}
"#;
    let emitted = emit_three("globalsplit", first, second, main);
    assert_eq!(
        count(&emitted, "cl_spec_criimpl_globalsplitfirst_pick(5)"),
        1,
        "globalsplit: only the qualified call must name the first module; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_spec_criimpl_globalsplitsecond_pick(5)"),
        2,
        "globalsplit: the import and the bare same-module call must both name the second module; emitted C:\n{emitted}"
    );
}

/// A same-module function's call emits the function's own symbol.
///
/// The callee resolves as a function declaration before any module-level
/// value, so the emitted call names the caller's function and the foreign
/// global's symbol never appears as a callee (red at the phase base: the call
/// named the foreign module's global).
#[test]
fn should_prefer_the_module_function_symbol_over_a_same_named_func_global() {
    let holder = r#"module spec.criimpl.globalmod;

export func g(x: i32): i32 {
    return x + 100;
}

export let f: func(i32): i32 = g;
"#;
    let main = r#"module spec.criimpl.globalmodmain;

import { g } from spec.criimpl.globalmod;

func f(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    return (f(5) - 6) + (g(5) - 105);
}
"#;
    let emitted = emit_pair("globalmod", holder, main);
    assert_contains(
        "globalmod",
        &emitted,
        "cl_spec_criimpl_globalmodmain_f(5)",
    );
    assert!(
        !emitted.contains("cl_spec_criimpl_globalmod_f(5)"),
        "globalmod: the foreign global must not be called; emitted C:\n{emitted}"
    );
}

/// Imported and qualified func-global calls emit the holder's symbol when a
/// same-named function exists elsewhere.
///
/// The import binding and the qualified spelling both resolve the holder's
/// module-level binding, so the emitted calls name its symbol and a foreign
/// function's symbol never appears as a callee — the unexported function of a
/// never-imported module least of all (red at the phase base: the calls named
/// the foreign function).
#[test]
fn should_keep_the_func_global_symbol_when_a_same_named_function_exists_elsewhere() {
    let holder = r#"module spec.criimpl.globalfnholder;

func hg(x: i32): i32 {
    return x + 200;
}

export let f: func(i32): i32 = hg;
"#;
    let other = r#"module spec.criimpl.globalfnother;

export func f(x: i32): i32 {
    return x + 3;
}
"#;
    let main = r#"module spec.criimpl.globalfnmain;

import { f } from spec.criimpl.globalfnholder;

func main(): i32 {
    return (f(5) - 205) + (spec.criimpl.globalfnholder.f(5) - 205);
}
"#;
    let emitted = emit_three("globalfn", holder, other, main);
    assert_eq!(
        count(&emitted, "cl_spec_criimpl_globalfnholder_f(5)"),
        2,
        "globalfn: both calls must name the holder's binding; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_spec_criimpl_globalfnother_f(5)"),
        0,
        "globalfn: the foreign function must not be called; emitted C:\n{emitted}"
    );

    let hidden = r#"module spec.criimpl.globalfnhidden;

func f(x: i32): i32 {
    return x + 230;
}
"#;
    let private_main = r#"module spec.criimpl.globalfnprivmain;

import { f } from spec.criimpl.globalfnholder;

func main(): i32 {
    return (f(5) - 205) + (spec.criimpl.globalfnholder.f(5) - 205);
}
"#;
    let emitted = emit_three("globalfnpriv", holder, hidden, private_main);
    assert_eq!(
        count(&emitted, "cl_spec_criimpl_globalfnholder_f(5)"),
        2,
        "globalfnpriv: both calls must name the holder's binding; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_spec_criimpl_globalfnhidden_f(5)"),
        0,
        "globalfnpriv: the unexported function must not be called; emitted C:\n{emitted}"
    );
}

// ===== Aliased imports and module-qualified value heads =====

/// An aliased import calls the imported function's own symbol.
///
/// The alias is a local spelling of the imported binding, so the emitted call
/// names the imported function; the alias spelling would leave the call
/// undeclared in the C.
#[test]
fn should_call_an_aliased_import_by_the_imported_symbol() {
    let holder = r#"module spec.criimpl.aliaspinmod;

export func g(x: i32): i32 {
    return x + 4;
}
"#;
    let main = r#"module spec.criimpl.aliaspin;

import { g as bg } from spec.criimpl.aliaspinmod;

func main(): i32 {
    return (bg(5) - 9) + (bg(7) - 11);
}
"#;
    let emitted = emit_pair("aliaspin", holder, main);
    assert_contains("aliaspin", &emitted, "cl_spec_criimpl_aliaspinmod_g(5)");
    assert_eq!(
        count(&emitted, "_bg("),
        0,
        "aliaspin: the alias must not spell the symbol; emitted C:\n{emitted}"
    );
}

/// A value-headed call through its declaring module names the binding.
///
/// `m.holder.g(5)` reads the module's binding and calls its field, so the
/// emitted call spells the mangled binding the module declares, exactly as
/// the same call through a plain import does.
#[test]
fn should_call_a_module_qualified_value_head_field() {
    let holder = r#"module spec.criimpl.vheadpinmod;

func qf(x: i32): i32 {
    return x + 30;
}

struct Holder {
    g: func(i32): i32;
}

export let holder: Holder = Holder { g: qf };
"#;
    let main = r#"module spec.criimpl.vheadpin;

func main(): i32 {
    return spec.criimpl.vheadpinmod.holder.g(5) - 35;
}
"#;
    let emitted = emit_pair("vheadpin", holder, main);
    assert_contains(
        "vheadpin",
        &emitted,
        "(cl_spec_criimpl_vheadpinmod_holder.g)(5)",
    );
}

/// A bare func-global call resolves through the scope's binding.
///
/// The imported `f` names the holder's func-typed global in both spellings,
/// so the emitted calls use the holder's symbol even though another loaded
/// module declares a same-named function (whose mismatched call the spec
/// suite pins as rejected).
#[test]
fn should_resolve_a_bare_func_global_call_through_the_scope() {
    let holder = r#"module spec.criimpl.scopepinholder;

func hg(x: i32): i32 {
    return x + 200;
}

export let f: func(i32): i32 = hg;
"#;
    let other = r#"module spec.criimpl.scopepinother;

export func f(s: str): i32 {
    return 0;
}
"#;
    let main = r#"module spec.criimpl.scopepin;

import { f } from spec.criimpl.scopepinholder;

func main(): i32 {
    return (f(5) - 205) + (spec.criimpl.scopepinholder.f(5) - 205);
}
"#;
    let emitted = emit_three("scopepin", holder, other, main);
    assert_eq!(
        count(&emitted, "cl_spec_criimpl_scopepinholder_f(5)"),
        2,
        "scopepin: both spellings call the holder's binding; emitted C:\n{emitted}"
    );
}

/// A module path binding wins over a colliding import in the emission.
///
/// The qualified read spells the module's own global, not the imported
/// binding of the same name.
#[test]
fn should_spell_a_module_path_binding_despite_an_import_collision() {
    let dep = r#"module pinmoddep;

export let thing: i32 = 3;
"#;
    let other = r#"module pinmodother;

export let pinmoddep: i32 = 4;
"#;
    let app = r#"module pinmodapp;

import { pinmoddep } from pinmodother;

func main(): i32 {
    return pinmoddep.thing - 3;
}
"#;
    let emitted = emit_three("pinmodapp", dep, other, app);
    assert_contains("pinmodapp", &emitted, "cl_pinmoddep_thing");
    assert_eq!(
        count(&emitted, "cl_pinmodother_pinmoddep.thing"),
        0,
        "pinmodapp: the read must not go through the imported binding; emitted C:\n{emitted}"
    );
}

/// A bare func-global call's mismatch reports once at the argument.
///
/// The imported `f` is the holder's `func(i32)` global (another loaded module
/// declares a same-named `func(str)`), so a string literal reports the frozen
/// conversion once, at the written argument — the record the scope-aware
/// typing produces. The emission pin above is the matching-call control.
#[test]
fn should_report_a_bare_func_global_call_mismatch_once() {
    let holder = r#"module spec.criimpl.scoperepholder;

func hg(x: i32): i32 {
    return x + 200;
}

export let f: func(i32): i32 = hg;
"#;
    let other = r#"module spec.criimpl.scoperepother;

export func f(s: str): i32 {
    return 0;
}
"#;
    let main = r#"module spec.criimpl.scoperep;

import { f } from spec.criimpl.scoperepholder;

func main(): i32 {
    return f("x") - 0;
}
"#;
    let scratch = ScratchDir::new("scoperep");
    let first_path = scratch.path().join("first.clo");
    let second_path = scratch.path().join("second.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&first_path, holder)
        .unwrap_or_else(|err| panic!("scoperep: cannot write the first holder: {err}"));
    std::fs::write(&second_path, other)
        .unwrap_or_else(|err| panic!("scoperep: cannot write the second holder: {err}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|err| panic!("scoperep: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        main_path.display().to_string(),
        first_path.display().to_string(),
        second_path.display().to_string(),
    ]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "scoperep: the front end accepted a mismatched call"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "scoperep: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'static str' to 'i32' implicitly"),
        "scoperep: the conversion message is missing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:6:14"),
        "scoperep: the argument's span is missing; stderr:\n{stderr}"
    );
}

/// An aliased `take` callee empties the caller's slot after the call.
///
/// The alias keeps the imported declaration's modes, so the call records the
/// same move-out a plain import records: the slot is emptied right after the
/// call and the callee owns the value.
#[test]
fn should_move_out_an_aliased_take_arguments_place() {
    let holder = r#"module spec.criimpl.aliastakepinholder;

export func sink(take s: String): i32 {
    return i32(len(s));
}
"#;
    let main = r#"module spec.criimpl.aliastakepin;

import { stringOf } from std.text;
import { sink as drop } from spec.criimpl.aliastakepinholder;

func main(): i32 {
    let s: String = stringOf("abcd");
    return drop(s) - 4;
}
"#;
    let emitted = emit_pair("aliastakepin", holder, main);
    // The name with the call's own argument: the prototype and the definition
    // spell the parameter's type there instead.
    let call_at = emitted
        .find("cl_spec_criimpl_aliastakepinholder_sink(s)")
        .unwrap_or_else(|| panic!("aliastakepin: the call is missing; emitted C:\n{emitted}"));
    assert_move_out_follows("aliastakepin", &emitted, call_at);
    assert_eq!(
        count(&emitted, "_drop("),
        0,
        "aliastakepin: the alias must not spell the symbol; emitted C:\n{emitted}"
    );
}

/// An aliased take callee moves its argument like the plain twin.
///
/// Both spellings report the same single move error at the same site, so the
/// aliased resolution cannot lose the argument's mode.
#[test]
fn should_report_an_aliased_take_use_like_the_plain_twin() {
    let holder = r#"module spec.criimpl.aliastakeuseholder;

export func sink(take s: String): i32 {
    return i32(len(s));
}
"#;
    let aliased = r#"module spec.criimpl.aliastakeusemain;

import { stringOf } from std.text;
import { sink as drop } from spec.criimpl.aliastakeuseholder;

func main(): i32 {
    let s: String = stringOf("abcd");
    let n: i32 = drop(s);
    return n + i32(len(s)) - 4;
}
"#;
    let plain = r#"module spec.criimpl.aliastakeuseplain;

import { stringOf } from std.text;
import { sink } from spec.criimpl.aliastakeuseholder;

func main(): i32 {
    let s: String = stringOf("abcd");
    let n: i32 = sink(s);
    return n + i32(len(s)) - 4;
}
"#;
    for (tag, main) in [("aliastakeuse", aliased), ("aliastakeuseplain", plain)] {
        let scratch = ScratchDir::new(tag);
        let holder_path = scratch.path().join("holder.clo");
        let main_path = scratch.path().join("main.clo");
        std::fs::write(&holder_path, holder)
            .unwrap_or_else(|err| panic!("{tag}: cannot write the holder: {err}"));
        std::fs::write(&main_path, main)
            .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
        let c_path = scratch.path().join("out.c");
        let output = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            main_path.display().to_string(),
            holder_path.display().to_string(),
        ]);
        assert_ne!(
            output.status.code(),
            Some(0),
            "{tag}: the front end accepted a moved value's use"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr.matches("error E10020:").count(),
            1,
            "{tag}: expected one E10020; stderr:\n{stderr}"
        );
        assert!(
            stderr.contains("'s' was moved and cannot be used") && stderr.contains("main.clo:9:24"),
            "{tag}: the report's message or site is missing; stderr:\n{stderr}"
        );
    }
}

/// A module-path receiver spells the binding the resolution reaches.
///
/// The call goes through the module's `holder` binding even though a local
/// binding of the same head name exists, matching the checker's order.
#[test]
fn should_spell_a_module_path_receiver_beside_a_colliding_binding() {
    let holder = r#"module collpin.a;

func qf(x: i32): i32 {
    return x + 30;
}

struct Holder {
    g: func(i32): i32;
}

export let holder: Holder = Holder { g: qf };
"#;
    let main = r#"module collpinmain;

struct H2 {
    g: func(i32): i32;
}

struct Outer {
    holder: H2;
}

func wrong(x: i32): i32 {
    return x + 7;
}

let collpin: Outer = Outer { holder: H2 { g: wrong } };

func main(): i32 {
    return collpin.a.holder.g(5) - 35;
}
"#;
    let emitted = emit_pair("collpin", holder, main);
    assert_contains("collpin", &emitted, "(cl_collpin_a_holder.g)(5)");
    assert_eq!(
        count(&emitted, "collpinmain_collpin.a"),
        0,
        "collpin: the binding's field chain must not spell the call; emitted C:\n{emitted}"
    );
}

/// A borrow deref argument is read in place; a take one empties the pointee.
///
/// The pointee emptying belongs to the owning position only: the borrow
/// spelling passes the read straight through, so the owner stays readable,
/// while the take spelling keeps the move-out.
#[test]
fn should_empty_a_take_deref_arguments_pointee_only() {
    let holder = r#"module spec.criimpl.ptrpinmod;

export func lend(borrow s: String): i32 {
    return i32(len(s));
}

export func sink(take s: String): i32 {
    return i32(len(s));
}
"#;
    let lend_main = r#"module spec.criimpl.ptrlendpin;

import { stringOf } from std.text;
import { lend } from spec.criimpl.ptrpinmod;

func main(): i32 {
    let a: String = stringOf("abcd");
    let q: raw String = unsafe &a;
    let n: i32 = lend(unsafe *q);
    return n + i32(len(a)) - 8;
}
"#;
    let emitted = emit_pair("ptrlendpin", holder, lend_main);
    assert_contains("ptrlendpin", &emitted, "cl_spec_criimpl_ptrpinmod_lend(*(q))");
    assert_eq!(
        count(&emitted, "*(q) = NULL;"),
        0,
        "ptrlendpin: a borrow argument must not empty its pointee; emitted C:\n{emitted}"
    );

    let take_main = r#"module spec.criimpl.ptrtakepin;

import { stringOf } from std.text;
import { sink } from spec.criimpl.ptrpinmod;

func main(): i32 {
    let a: String = stringOf("abcd");
    let q: raw String = unsafe &a;
    let n: i32 = sink(unsafe *q);
    return n - 4;
}
"#;
    let emitted = emit_pair("ptrtakepin", holder, take_main);
    assert_eq!(
        count(&emitted, "*(q) = NULL;"),
        1,
        "ptrtakepin: the take argument's move-out is missing; emitted C:\n{emitted}"
    );
}

// ===== Qualified call identity =====

/// A qualified call reaches the module's declaration in the emitted symbol.
///
/// The emitter spells the module's function symbol for a qualified call, not a
/// same-named function of the caller's module: the call declares, defines, and
/// calls the module's `f`, and the caller's same-named `f` has no call. A
/// second emission is byte-identical.
#[test]
fn should_emit_the_qualified_function_identity_at_the_symbol() {
    let lib = r#"module probe.m390.lib;

export func f(): i32 {
    return 1;
}
"#;
    let main = r#"module probe.m390.main;

func f(): i32 {
    return 2;
}

func main(): i32 {
    return probe.m390.lib.f() - 1;
}
"#;
    let emitted = emit_pair("qualidentity", lib, main);
    assert_eq!(
        count(&emitted, "i32 cl_probe_m390_lib_f(void);"),
        1,
        "qualidentity: the module's prototype is missing; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "i32 cl_probe_m390_lib_f(void)\n{"),
        1,
        "qualidentity: the module's definition is missing; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_probe_m390_lib_f()"),
        1,
        "qualidentity: the call must reach the module's function; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_probe_m390_main_f()"),
        0,
        "qualidentity: the caller's same-named function must not be \
         called; emitted C:\n{emitted}"
    );
    let (first, second) = emit_twice_pair("qualidentityreemit", lib, main);
    assert_eq!(
        first, second,
        "qualidentityreemit: the program re-emitted differently"
    );
}

/// A qualified call uses the module's signature, and a mismatch is rejected.
///
/// The signature-divergent caller passes an `i32` to the module's
/// `f(n: i32)` while its own same-named `f` takes a `str`; the call must use
/// the module's by-value parameter, not the caller's signature. The mismatched
/// argument reports one frozen conversion at the written value and writes no
/// translation unit.
#[test]
fn should_emit_the_qualified_signature_and_reject_the_mismatch() {
    let siglib = r#"module probe.m390.sig;

export func f(n: i32): i32 {
    return n + 1;
}
"#;
    let sigmain = r#"module probe.m390.sigmain;

func f(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    return probe.m390.sig.f(5) - 6;
}
"#;
    let emitted = emit_pair("qualidentitysig", siglib, sigmain);
    assert_eq!(
        count(&emitted, "i32 cl_probe_m390_sig_f(i32 n);"),
        1,
        "qualidentitysig: the module's prototype is missing; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "i32 cl_probe_m390_sig_f(i32 n)\n{"),
        1,
        "qualidentitysig: the module's definition is missing; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_probe_m390_sig_f(5)"),
        1,
        "qualidentitysig: the call must pass the module's by-value parameter; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_probe_m390_sigmain_f(5)"),
        0,
        "qualidentitysig: the call must not reach the caller's str function; emitted C:\n{emitted}"
    );

    let mismatch = r#"module probe.m390.mismatch;

func f(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    return probe.m390.sig.f("x") - 6;
}
"#;
    let stderr = reject_pair("qualidentitymismatch", siglib, mismatch);
    assert_eq!(
        stderr.matches("error E10200:").count(),
        1,
        "qualidentitymismatch: expected one E10200; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot convert 'static str' to 'i32' implicitly"),
        "qualidentitymismatch: expected the conversion message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:8:29"),
        "qualidentitymismatch: expected the report at the written argument; stderr:\n{stderr}"
    );
}

/// The qualified func-typed global keeps its own binding.
///
/// `m.hook(5)` names the module's exported global, so the call goes through the
/// module's function pointer rather than the caller's same-named function. The
/// shape already held before the identity change, so this is a control.
#[test]
fn should_keep_the_qualified_func_global_call() {
    let hooklib = r#"module probe.m390.hooklib;

export let hook: func(i32): i32 = (n: i32) => n + 1;
"#;
    let hookmain = r#"module probe.m390.hookmain;

func hook(n: i32): i32 { return n + 9; }

func main(): i32 {
    return probe.m390.hooklib.hook(5) - 6;
}
"#;
    let emitted = emit_pair("qualidentityhook", hooklib, hookmain);
    assert_eq!(
        count(&emitted, "cl_probe_m390_hooklib_hook(5)"),
        1,
        "qualidentityhook: the call must reach the module's global; emitted C:\n{emitted}"
    );
    assert_eq!(
        count(&emitted, "cl_probe_m390_hookmain_hook(5)"),
        0,
        "qualidentityhook: the call must not reach the caller's function; emitted C:\n{emitted}"
    );
}
