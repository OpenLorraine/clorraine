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
    assert!(
        emitted[call..].contains("s = NULL;"),
        "fieldtake: the take argument must be emptied after the call; emitted C:\n{emitted}"
    );
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
    assert!(
        emitted[call..].contains("s = NULL;"),
        "globaltake: the take argument must be emptied after the call; emitted C:\n{emitted}"
    );
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
